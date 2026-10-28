import hashlib
import json
import tempfile
import unittest
from pathlib import Path, PureWindowsPath

from prepare_runner import checked, relative_name, sha, suite_files


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'recovery/parsers').mkdir(parents=True)
        (self.root / 'recovery/inputs').mkdir()
        (self.root / 'recovery/parsers/reader.py').write_text('actual recovered reader')
        (self.root / 'recovery/parsers/helper.py').write_text('declared helper')
        (self.root / 'recovery/inputs/save.bin').write_bytes(b'original')
        self.suite = self.root / 'recovery/suite.json'
        self.data = {'parser': 'parsers/reader.py', 'parserDependencies': ['parsers/helper.py'],
                     'cases': [{'input': 'inputs/save.bin', 'sha256': hashlib.sha256(b'original').hexdigest()}]}
        self.save()

    def save(self):
        self.suite.write_text(json.dumps(self.data))

    def test_preserves_recovered_reader_and_declared_helper_paths(self):
        (self.root / 'parser_template.py').write_text('must not select')
        files = suite_files(self.root, Path('recovery/suite.json'))
        self.assertEqual({str(p.relative_to(self.root)) for p in files}, {
            'recovery/suite.json', 'recovery/parsers/reader.py',
            'recovery/parsers/helper.py', 'recovery/inputs/save.bin'})

    def test_changed_original_fails_without_rebaselining(self):
        (self.root / 'recovery/inputs/save.bin').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'Fixture hash mismatch'):
            suite_files(self.root, Path('recovery/suite.json'))
        self.assertEqual(self.data['cases'][0]['sha256'], hashlib.sha256(b'original').hexdigest())

    def test_missing_helper_is_not_silently_dropped(self):
        (self.root / 'recovery/parsers/helper.py').unlink()
        with self.assertRaisesRegex(ValueError, 'Unsafe/missing'):
            suite_files(self.root, Path('recovery/suite.json'))

    def test_path_escape_is_rejected(self):
        with self.assertRaises(ValueError):
            checked(self.root, '../outside')

    def test_links_are_not_payloads(self):
        (self.root / 'linked').symlink_to(self.suite)
        with self.assertRaises(ValueError):
            checked(self.root, 'linked')

    def test_windows_wire_paths_use_forward_slashes(self):
        self.assertEqual(relative_name(PureWindowsPath('C:/payload/nested/brief.md'),
                                       PureWindowsPath('C:/payload')), 'nested/brief.md')

    def test_hash_matches_standard_digest(self):
        self.assertEqual(sha(self.root / 'recovery/inputs/save.bin'), hashlib.sha256(b'original').hexdigest())


def _import_runner():
    """Import research-runner even when tools/ps1_replay is on sys.path.

    That directory's runner.py is the emulator coordinator and would shadow the package.
    """
    import sys
    replay = Path(__file__).resolve().parents[1] / 'ps1_replay'
    saved = sys.path[:]
    sys.path[:] = [entry for entry in saved if Path(entry).resolve() != replay.resolve()]
    for name in list(sys.modules):
        module = sys.modules.get(name)
        file = str(getattr(module, '__file__', '') or '')
        if (name == 'runner' or name.startswith('runner.')) and 'ps1_replay' in file:
            del sys.modules[name]
    try:
        from runner import ResearchRepo
        from runner.package import assemble, verify
        return ResearchRepo, assemble, verify
    finally:
        sys.path[:] = saved


try:
    ResearchRepo, assemble, verify = _import_runner()
except ImportError:
    ResearchRepo = None


@unittest.skipIf(ResearchRepo is None, 'Install requirements-research.txt for Runner integration tests')
class MigrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = ResearchRepo.init(self.root / 'consumer', name='ps1-summary', skill=False)
        self.source = self.root / 'legacy'
        self.source.mkdir()
        self.manifest = json.loads((self.repo.root / 'research/ps1-summary/manifest.json').read_text())
        self.manifest['inputs'] = ['inputs/save.bin', 'inputs/disc.cue', 'inputs/disc.bin', 'nested/manifest.json']
        self.manifest['required_outputs'] = ['outputs/result.json', 'outputs/handoff.md', 'outputs/final/reader.py']
        (self.source / 'inputs').mkdir()
        (self.source / 'nested').mkdir()
        (self.source / 'nested/manifest.json').write_text('{}')
        (self.source / 'inputs/save.bin').write_bytes(b'private-original')
        (self.source / 'inputs/disc.cue').write_text('FILE "disc.bin" BINARY')
        (self.source / 'inputs/disc.bin').write_bytes(b'disc-original')
        (self.source / 'brief.md').write_text('Original scientific question')
        (self.source / 'AGENTS.md').write_text('Original scientific constraints')
        (self.source / 'manifest.json').write_text(json.dumps(self.manifest))
        self.archive = self.root / 'legacy.rrpkg'
        self.expected = assemble(self.source, self.archive)

    def test_migration_preserves_dependencies_and_adds_structured_closeout(self):
        from prepare_runner import import_package
        package = import_package(self.repo, 'legacy-us', self.archive, self.expected)
        manifest, inventory = verify(package['archive'])
        target = self.repo.root / 'research/legacy-us'
        for name in self.manifest['inputs']:
            self.assertEqual((target / name).read_bytes(), (self.source / name).read_bytes())
            self.assertIn(name, inventory)
        self.assertIn('outputs/research.json', manifest['required_outputs'])
        self.assertIn('outputs/final/reader.py', manifest['required_outputs'])
        self.assertEqual(manifest['policy'], self.manifest['policy'])
        self.assertIn('inputs/contracts.json', inventory)
        self.assertEqual(sha(self.archive), self.expected)
        self.assertFalse((self.repo.state / 'runs').exists())
        self.assertEqual((target / 'preparation/runner-migration-original/brief.md').read_text(),
                         'Original scientific question')

    def test_rewritten_root_manifest_is_not_claimed_as_preserved(self):
        from prepare_runner import import_package
        self.manifest['inputs'].append('manifest.json')
        (self.source / 'manifest.json').write_text(json.dumps(self.manifest))
        self.expected = assemble(self.source, self.archive)
        import_package(self.repo, 'root-manifest', self.archive, self.expected)
        target = self.repo.root / 'research/root-manifest'
        ledger = json.loads((target / 'preparation/runner-migration.json').read_text())
        self.assertNotIn('manifest.json', ledger['preserved_files'])
        self.assertIn('nested/manifest.json', ledger['preserved_files'])
        for name, expected in ledger['preserved_files'].items():
            self.assertEqual(sha(target / name), expected, name)
        self.assertEqual(ledger['original_manifest'], self.manifest)

    def test_nested_control_files_remain_declared_once(self):
        from prepare_runner import import_package
        for key in ('brief', 'agents'):
            old = self.source / self.manifest[key]
            name = 'nested/' + old.name
            old.rename(self.source / name)
            self.manifest[key] = name
        skill = 'nested/SKILL.md'
        (self.source / skill).write_text('Research skill')
        self.manifest['skills'] = [skill]
        (self.source / 'manifest.json').write_text(json.dumps(self.manifest))
        self.expected = assemble(self.source, self.archive)
        package = import_package(self.repo, 'nested-control', self.archive, self.expected)
        manifest, inventory = verify(package['archive'])
        for name in (manifest['brief'], manifest['agents'], *manifest['skills']):
            self.assertIn(name, inventory)
            self.assertNotIn(name, manifest['inputs'])
        self.assertTrue(all('\\' not in name for name in manifest['inputs']))

    def test_mismatch_creates_no_definition(self):
        from prepare_runner import import_package
        with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
            import_package(self.repo, 'bad', self.archive, '0' * 64)
        self.assertFalse((self.repo.root / 'research/bad').exists())

    def test_existing_definition_is_never_overwritten(self):
        from prepare_runner import import_package
        with self.assertRaisesRegex(ValueError, 'overwrite'):
            import_package(self.repo, 'ps1-summary', self.archive, self.expected)

    def test_unsafe_name_is_rejected(self):
        from prepare_runner import import_package
        with self.assertRaisesRegex(ValueError, 'name'):
            import_package(self.repo, '../escape', self.archive, self.expected)


if __name__ == '__main__':
    unittest.main()
