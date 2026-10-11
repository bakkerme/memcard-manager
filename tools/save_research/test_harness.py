"""Exercise the runner with disposable fixtures and real child processes."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import harness


PARSER = '''
def inspect_save(source, metadata):
    if metadata['productCode'] != 'TEST-00001':
        raise ValueError('unsupported release')
    if len(source) != 4:
        raise ValueError('truncated layout')
    return {'fields': {'value': source[0]}, 'checksumOk': None}
'''


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'parser.py').write_text(PARSER)

    def case(self, name='known', source=b'\x07abc', **extra):
        path = self.root / (name + '.mcs'); path.write_bytes(source)
        return {'id': name, 'input': path.name, 'sha256': hashlib.sha256(source).hexdigest(),
                'metadata': {'productCode': 'TEST-00001', 'region': 'BA', 'identifier': 'A'},
                **extra}

    def run_cases(self, cases, timeout=3):
        path = self.root / 'suite.json'
        path.write_text(json.dumps({'schemaVersion': 1, 'parser': 'parser.py', 'cases': cases}))
        return harness.run_suite(path, timeout)

    def test_known_values_negative_cases_and_contract_only_remain_distinct(self):
        known = self.case(expect={'/fields/value': 7, '/checksumOk': None})
        short = self.case('short', b'a', expectErrorContains='truncated layout')
        wrong = self.case('wrong', expectErrorContains='unsupported release')
        wrong['metadata']['productCode'] = 'OTHER'
        plain = self.case('plain')
        report = self.run_cases([known, short, wrong, plain])
        self.assertEqual(report['status'], 'passed')
        self.assertEqual([r['status'] for r in report['cases']], ['expectations_passed'] * 3 + ['contract_passed'])
        self.assertTrue(all('input-unchanged' in r['checks'] for r in report['cases']))

    def test_wrong_expectations_fail_and_later_cases_still_run(self):
        bad = self.case(expect={'/fields/value': True})  # bool is not an integer expectation.
        good = self.case('later', expect={'/fields/value': 7})
        report = self.run_cases([bad, good])
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['cases'][1]['status'], 'expectations_passed')

    def test_bad_hash_missing_file_and_escaping_symlink_are_not_executed(self):
        (self.root / 'parser.py').write_text("raise RuntimeError('must not execute')")
        bad = self.case(); bad['sha256'] = '0' * 64
        missing = self.case('missing'); (self.root / missing['input']).unlink()
        with tempfile.TemporaryDirectory() as outside:
            dest = Path(outside) / 'fixture'; dest.write_bytes(b'a')
            (self.root / 'escape').symlink_to(dest)
            escape = self.case('escape-case'); escape['input'] = 'escape'
            report = self.run_cases([bad, missing, escape])
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(all('response' not in r for r in report['cases']))

    def test_timeout_and_crash_do_not_count_as_expected_rejection(self):
        (self.root / 'parser.py').write_text("def inspect_save(source, metadata):\n    while True: pass\n")
        row = self.run_cases([self.case(expectErrorContains='anything')], timeout=0.1)['cases'][0]
        self.assertIn('TimeoutExpired', row['error'])
        (self.root / 'parser.py').write_text("def inspect_save(source, metadata):\n    raise TypeError('crash')\n")
        row = self.run_cases([self.case(expectErrorContains='crash')])['cases'][0]
        self.assertEqual(row['status'], 'failed')
        self.assertIn('exited', row['error'])

    def test_checksum_boolean_requires_real_boolean_and_fields_are_required(self):
        for result in ["{'fields': {'value': 7}, 'checksumOk': 1}",
                       "{'fields': {}, 'checksumOk': None}", "{'fields': {'value': 7}}"]:
            (self.root / 'parser.py').write_text('def inspect_save(source, metadata):\n    return ' + result + '\n')
            self.assertEqual(self.run_cases([self.case()])['status'], 'failed')

    def test_input_mutation_is_reported(self):
        fixture = self.root / 'known.mcs'
        (self.root / 'parser.py').write_text(PARSER + f"\nfrom pathlib import Path\nPath({str(fixture)!r}).write_bytes(b'changed')\n")
        row = self.run_cases([self.case()])['cases'][0]
        self.assertEqual(row['status'], 'failed')
        self.assertIn('preservationError', row)

    def test_existing_report_is_never_overwritten(self):
        import subprocess
        import sys
        self.run_cases([self.case()])
        report = self.root / 'report.json'; report.write_text('retain')
        result = subprocess.run([sys.executable, str(Path(harness.__file__)), str(self.root / 'suite.json'), '--output', str(report)], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(report.read_text(), 'retain')

    def test_declared_helper_change_retains_case_results_but_fails_suite(self):
        helper = self.root / 'helper.py'; helper.write_text('original')
        (self.root / 'parser.py').write_text(PARSER + f"\nfrom pathlib import Path\nPath({str(helper)!r}).write_text('changed')\n")
        case = self.case()
        suite = self.root / 'suite.json'
        suite.write_text(json.dumps({'schemaVersion': 1, 'parser': 'parser.py',
                                    'parserDependencies': ['helper.py'], 'cases': [case]}))
        report = harness.run_suite(suite)
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(len(report['cases']), 1)
        self.assertIn('helper.py', report['parserSourceHashes'])
        self.assertIn('helper.py', report['preservationErrors'][0])

    def test_sibling_helper_import_works_in_fresh_worker(self):
        folder = self.root / 'parsers'; folder.mkdir()
        (folder / 'helper.py').write_text('VALUE = 7\n')
        (folder / 'parser.py').write_text('from helper import VALUE\ndef inspect_save(source, metadata):\n    return {"fields": {"value": VALUE}, "checksumOk": None}\n')
        suite = self.root / 'suite.json'
        suite.write_text(json.dumps({'schemaVersion': 1, 'parser': 'parsers/parser.py',
                                    'parserDependencies': ['parsers/helper.py'],
                                    'cases': [self.case(expect={'/fields/value': 7})]}))
        self.assertEqual(harness.run_suite(suite)['status'], 'passed')

    def test_later_case_mutating_previous_fixture_fails_suite(self):
        earlier = self.case('earlier')
        later = self.case('later', b'\x08abc')
        mutation = ('\noriginal = inspect_save\ndef inspect_save(source, metadata):\n'
                    '    if source[0] == 8:\n        from pathlib import Path\n'
                    f'        Path({str(self.root / earlier["input"])!r}).write_bytes(b"changed")\n'
                    '    return original(source, metadata)\n')
        (self.root / 'parser.py').write_text(PARSER + mutation)
        report = self.run_cases([earlier, later])
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['cases'][0]['status'], 'failed')
        self.assertIn('preservationError', report['cases'][0])

    def test_nested_types_and_invalid_array_pointers_cannot_pass(self):
        (self.root / 'parser.py').write_text('def inspect_save(source, metadata):\n    return {"fields": {"nested": {"value": 1}, "items": [7]}, "checksumOk": None}\n')
        for expected in [{'/fields/nested': {'value': True}}, {'/fields/items/-1': 7}, {'/fields/items/00': 7}]:
            self.assertEqual(self.run_cases([self.case(expect=expected)])['status'], 'failed')

    def test_expected_rejection_cannot_silently_ignore_value_assertions(self):
        case = self.case(source=b'a', expectErrorContains='truncated layout', expect={'/fields/value': 7})
        row = self.run_cases([case])['cases'][0]
        self.assertEqual(row['status'], 'failed')
        self.assertNotIn('response', row)


if __name__ == '__main__':
    unittest.main()
