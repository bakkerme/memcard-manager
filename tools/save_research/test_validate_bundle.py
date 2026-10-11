"""Bundle handoff checks on disposable synthetic inputs and the shipped tools."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import build_bundle
import test_build_bundle
from test_disc_id import iso
from validate_bundle import validate_bundle


class ValidationTests(unittest.TestCase):
    def setUp(self):
        test_build_bundle.BundleTests.setUp(self)
        copier = build_bundle.copy_code
        with patch.object(build_bundle, 'copy_code', side_effect=lambda repo, output: copier(build_bundle.REPO, output)):
            build_bundle.build_bundle(self.repo, self.backlog, self.output, rom_roots=[])

    def test_portable_bundle_passes_and_missing_rom_gate_is_explicit(self):
        report = validate_bundle(self.output)
        self.assertEqual(report['status'], 'passed', report['errors'])
        self.assertEqual(report['counts']['samples'], 1)
        self.assertEqual(report['missingRoms'], ['SLUS-00603'])
        self.assertEqual(validate_bundle(self.output, require_roms=True)['status'], 'failed')

    def test_save_mutation_cannot_be_blessed_by_refresh(self):
        manifest = json.loads((self.output / 'jobs.json').read_text())
        sample = manifest['jobs'][0]['releases'][0]['samples'][0]
        path = self.output / sample['path']; path.write_bytes(b'tampered')
        previous = (self.output / 'checksums.sha256').read_bytes()
        with self.assertRaisesRegex(ValueError, 'Save input changed'):
            build_bundle.refresh_roms(self.output, [self.output / 'inputs/roms'])
        self.assertEqual((self.output / 'checksums.sha256').read_bytes(), previous)
        self.assertEqual(validate_bundle(self.output)['status'], 'failed')

    def test_suite_metadata_mismatch_and_escaping_paths_fail(self):
        path = self.output / 'suite-SLUS-00603.json'
        suite = json.loads(path.read_text())
        suite['cases'][0]['metadata']['productCode'] = 'SLES-00603'
        path.write_text(json.dumps(suite))
        self.assertEqual(validate_bundle(self.output)['status'], 'failed')
        suite['cases'][0]['metadata']['productCode'] = 'SLUS-00603'
        suite['cases'][0]['input'] = str(self.source)
        path.write_text(json.dumps(suite))
        self.assertEqual(validate_bundle(self.output)['status'], 'failed')

    def test_embedded_disc_id_is_checked_beyond_names_and_checksums(self):
        roms = self.output / 'inputs/roms'; folder = roms / 'SLUS-00603'
        disc = folder / 'SLUS-00603.iso'; disc.write_bytes(iso())
        build_bundle.refresh_roms(self.output, [roms])
        self.assertEqual(validate_bundle(self.output, True)['status'], 'passed')
        disc.write_bytes(iso('BOOT = cdrom:\\SLES_006.03;1\n', executable='SLES_006.03;1'))
        manifest_path = self.output / 'jobs.json'
        manifest = json.loads(manifest_path.read_text())
        manifest['jobs'][0]['releases'][0]['discs'][0]['files'][0]['sha256'] = build_bundle.sha(disc)
        manifest_path.write_text(json.dumps(manifest))
        build_bundle.write_checksums(self.output)
        report = validate_bundle(self.output)
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(any('Embedded disc ID' in error for error in report['errors']))

    def test_tooling_changes_and_unlisted_inputs_are_reported(self):
        (self.output / 'tools/save_research/parser_template.py').write_text('changed')
        (self.output / 'inputs/saves/untracked.mcs').write_bytes(b'new')
        report = validate_bundle(self.output)
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(any('SHA-256 mismatch' in error for error in report['errors']))
        self.assertTrue(any('exactly the current input' in error for error in report['errors']))


if __name__ == '__main__':
    unittest.main()
