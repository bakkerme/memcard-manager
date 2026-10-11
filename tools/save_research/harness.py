#!/usr/bin/env python3
"""Standard-library runner for portable, read-only PS1 parser research suites."""
import argparse
import base64
import contextlib
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import time


def digest(data):
    return hashlib.sha256(data).hexdigest()


def local_file(root, relative):
    path = Path(relative)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError('Suite paths must be relative and stay within the suite directory')
    resolved = (root / path).resolve()
    if not resolved.is_relative_to(root.resolve()) or not resolved.is_file():
        raise ValueError(f'Missing file or escaping path: {relative}')
    return resolved


def pointer(value, path):
    if not path.startswith('/'):
        raise ValueError('Expected-value keys must be JSON pointers beginning with /')
    for part in path[1:].split('/'):
        if re.search(r'~(?![01])', part):
            raise ValueError('Invalid JSON pointer escape')
        key = part.replace('~1', '/').replace('~0', '~')
        if isinstance(value, list) and not re.fullmatch(r'0|[1-9][0-9]*', key):
            raise ValueError('JSON pointer array indices must be nonnegative canonical integers')
        value = value[int(key)] if isinstance(value, list) else value[key]
    return value


def same_json(actual, expected):
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(same_json(actual[key], value) for key, value in expected.items())
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(same_json(a, b) for a, b in zip(actual, expected))
    return actual == expected


def worker(module_path):
    request = json.load(sys.stdin)
    sys.path.insert(0, str(Path(module_path).resolve().parent))
    # Keep parser diagnostics separate from the JSON protocol, including imports.
    with contextlib.redirect_stdout(sys.stderr):
        spec = importlib.util.spec_from_file_location('research_parser', module_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        try:
            result = module.inspect_save(base64.b64decode(request['source']), request['metadata'])
            response = {'outcome': 'decoded', 'result': result}
        except ValueError as error:
            response = {'outcome': 'rejected', 'error': str(error)}
    print(json.dumps(response, allow_nan=False))


def invoke(parser, source, metadata, timeout):
    request = json.dumps({'source': base64.b64encode(source).decode(), 'metadata': metadata})
    result = subprocess.run([sys.executable, '-B', str(Path(__file__).resolve()),
                             '--worker', str(parser)], input=request, text=True,
                            capture_output=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'Parser process exited {result.returncode}: {result.stderr[-4000:]}')
    response = json.loads(result.stdout)
    response['diagnostics'] = result.stderr[-4000:]
    return response


def run_suite(manifest_path, timeout=10):
    manifest_path = Path(manifest_path).resolve()
    manifest_bytes = manifest_path.read_bytes()
    suite = json.loads(manifest_bytes)
    if suite.get('schemaVersion') != 1 or not suite.get('cases'):
        raise ValueError('Expected schemaVersion 1 and at least one case')
    if not math.isfinite(timeout) or timeout <= 0:
        raise ValueError('Timeout must be positive and finite')
    root = manifest_path.parent
    harness_path = Path(__file__).resolve()
    harness_hash = digest(harness_path.read_bytes())
    parser = local_file(root, suite['parser'])
    parser_hash = digest(parser.read_bytes())
    code_files = {suite['parser']: parser}
    for dependency in suite.get('parserDependencies', []):
        code_files[dependency] = local_file(root, dependency)
    code_hashes = {name: digest(path.read_bytes()) for name, path in code_files.items()}
    results = []
    observed_inputs = []
    ids = set()
    for case in suite['cases']:
        if not isinstance(case.get('id'), str) or not case['id'] or case['id'] in ids:
            raise ValueError('Cases require unique, nonempty string IDs')
        ids.add(case['id'])
        row = {'id': case['id'], 'status': 'failed', 'checks': [],
               'evidence': case.get('evidence', {}), 'durationSeconds': None}
        started = time.monotonic()
        source_path = None
        before = None
        try:
            source_path = local_file(root, case['input'])
            with source_path.open('rb') as file:
                candidate = file.read(128 + 15 * 8192 + 1)
            if len(candidate) > 128 + 15 * 8192:
                raise ValueError('Supply an individual MCS or normalized SC payload, not a whole card')
            before = candidate
            observed_inputs.append((source_path, digest(before), row))
            row['sourceSha256'] = digest(before)
            if row['sourceSha256'] != case['sha256']:
                raise ValueError('Fixture SHA-256 mismatch; parser was not executed')
            metadata = case['metadata']
            if not isinstance(metadata, dict) or not all(isinstance(metadata.get(k), str)
                    for k in ('productCode', 'region', 'identifier')):
                raise ValueError('Metadata requires productCode, region and identifier strings')
            if 'expectErrorContains' in case and case.get('expect'):
                raise ValueError('A case cannot combine decoded-value expectations with expected rejection')
            row['checks'].append('fixture-hash')
            response = invoke(parser, before, metadata, timeout)
            row['response'] = response
            if 'expectErrorContains' in case:
                if not isinstance(case['expectErrorContains'], str) or not case['expectErrorContains']:
                    raise ValueError('Negative cases require a nonempty expected rejection message')
                if response['outcome'] != 'rejected' or case['expectErrorContains'] not in response['error']:
                    raise ValueError('Parser did not reject input with the expected ValueError')
                row['checks'].append('expected-rejection')
                row['status'] = 'expectations_passed'
            else:
                if response['outcome'] != 'decoded':
                    raise ValueError('Parser rejected a positive case')
                value = response['result']
                if not isinstance(value, dict) or not isinstance(value.get('fields'), dict) or not value['fields']:
                    raise ValueError('Parser must return a nonempty fields object')
                if 'checksumOk' not in value or (value['checksumOk'] is not None and type(value['checksumOk']) is not bool):
                    raise ValueError('checksumOk must be true, false, or null (unverified)')
                row['checks'].append('output-contract')
                expected = case.get('expect', {})
                if not isinstance(expected, dict):
                    raise ValueError('expect must be an object of JSON pointers and values')
                for path, wanted in expected.items():
                    actual = pointer(value, path)
                    if not same_json(actual, wanted):
                        raise ValueError(f'Expected {path}={wanted!r}; got {actual!r}')
                    row['checks'].append('expected-value:' + path)
                row['status'] = 'expectations_passed' if expected else 'contract_passed'
        except Exception as error:
            row['error'] = f'{type(error).__name__}: {error}'
        finally:
            if source_path is not None and before is not None:
                try:
                    unchanged = source_path.read_bytes() == before
                except OSError:
                    unchanged = False
                if unchanged:
                    row['checks'].append('input-unchanged')
                else:
                    row['status'] = 'failed'
                    row['preservationError'] = 'Input changed or disappeared; original is not repaired'
            row['durationSeconds'] = round(time.monotonic() - started, 3)
            results.append(row)
    report = {'schemaVersion': 1, 'manifestSha256': digest(manifest_bytes),
            'harnessSha256': harness_hash,
            'pythonVersion': sys.version,
            'parserSourceHashes': code_hashes,
            'parserSha256': parser_hash, 'status': 'failed' if any(r['status'] == 'failed' for r in results) else 'passed',
            'interpretation': 'Passing checks validate the declared contract and expectations, not independent field meanings or save/load persistence.',
            'cases': results}
    monitored = [(path, code_hashes[name]) for name, path in code_files.items()]
    monitored.append((manifest_path, digest(manifest_bytes)))
    monitored.append((harness_path, harness_hash))
    for path, original in monitored:
        try:
            unchanged = digest(path.read_bytes()) == original
        except OSError:
            unchanged = False
        if not unchanged:
            report['status'] = 'failed'
            report.setdefault('preservationErrors', []).append(f'{path.name} changed or disappeared during the suite')
    for path, expected_hash, row in observed_inputs:
        try:
            unchanged = digest(path.read_bytes()) == expected_hash
        except OSError:
            unchanged = False
        if not unchanged:
            report['status'] = row['status'] = 'failed'
            row['preservationError'] = 'Input changed or disappeared during the suite; original is not repaired'
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('suite', nargs='?', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--timeout', type=float, default=10)
    parser.add_argument('--worker', type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        worker(args.worker)
        return
    if not args.suite or not args.output:
        parser.error('Supply a suite JSON and --output pointing to a new report file')
    # Exclusive creation preserves previous reports, even on configuration errors.
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as destination:
        try:
            report = run_suite(args.suite, args.timeout)
        except Exception as error:
            report = {'status': 'failed', 'configurationError': f'{type(error).__name__}: {error}'}
        json.dump(report, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps({'status': report['status'], 'report': str(args.output.resolve())}))
    raise SystemExit(report['status'] == 'failed')


if __name__ == '__main__':
    main()
