"""Portable bundle preparation with disposable inputs and tiny disc fixtures."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import build_bundle
from test_disc_id import iso


class BundleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name); self.repo = self.root / 'repo'; self.repo.mkdir()
        (self.repo / 'src-tauri/src/card').mkdir(parents=True)
        (self.repo / 'src-tauri/src/card/game_details.rs').write_text('"SCUS-94423" => ()')
        (self.repo / 'tools/save_research').mkdir(parents=True)
        (self.repo / 'tools/save_research/parser_template.py').write_text('template')
        data = bytearray(8320); data[0] = 0x51; data[4:8] = (8192).to_bytes(4, 'little')
        name = b'BASLUS-00603-DASH00'; data[10:10 + len(name)] = name; data[128:132] = b'SC\x11\x01'
        self.source = self.root / 'save.mcs'; self.source.write_bytes(data)
        self.sample = {'blocks': 1, 'deleted': False, 'filename': name.decode(), 'identifier': '-DASH00',
                       'payloadSha256': hashlib.sha256(data[128:]).hexdigest(), 'region': 'BA', 'slot': 1,
                       'title': 'MEGAMAN LEGENDS', 'path': str(self.source), 'fileSha256': build_bundle.sha(self.source),
                       'activeCopyAvailable': True}
        self.backlog = self.root / 'parser-jobs.json'
        self.backlog.write_text(json.dumps({'schemaVersion': 1, 'jobs': [{'game': 'Mega Man Legends',
            'deletedOnly': False, 'releases': [{'productCode': 'SLUS-00603', 'region': 'NTSC-U',
            'identifiers': ['-DASH00'], 'blockCounts': [1], 'samples': [self.sample]}]}]}))
        self.output = self.root / 'bundle'

    def build(self, **kwargs):
        with patch.object(build_bundle, 'copy_code'):
            return build_bundle.build_bundle(self.repo, self.backlog, self.output, rom_roots=[], **kwargs)

    def test_portable_paths_hashes_templates_and_no_overwrite(self):
        before = self.source.read_bytes()
        manifest = self.build()
        sample = manifest['jobs'][0]['releases'][0]['samples'][0]
        self.assertFalse(Path(sample['path']).is_absolute())
        self.assertEqual((self.output / sample['path']).read_bytes(), before)
        self.assertEqual(self.source.read_bytes(), before)
        self.assertTrue((self.output / 'inputs/roms/SLUS-00603').is_dir())
        self.assertEqual(manifest['jobs'][0]['releases'][0]['romStatus'], 'missing')
        self.assertNotIn(str(self.root), (self.output / 'jobs.json').read_text())
        with self.assertRaisesRegex(ValueError, 'already exists'): self.build()

    def test_source_tampering_retains_inspectable_failure(self):
        self.source.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'Source changed'): self.build()
        self.assertTrue((self.output / 'build-error.json').exists())

    def test_unusual_sc_header_is_retained_and_reported(self):
        data = bytearray(self.source.read_bytes()); data[128] = 0
        self.source.write_bytes(data)
        self.sample['fileSha256'] = build_bundle.sha(self.source)
        self.sample['payloadSha256'] = hashlib.sha256(data[128:]).hexdigest()
        manifest = json.loads(self.backlog.read_text())
        manifest['jobs'][0]['releases'][0]['samples'][0] = self.sample
        self.backlog.write_text(json.dumps(manifest))
        result = self.build()
        sample = result['jobs'][0]['releases'][0]['samples'][0]
        self.assertIn('SC signature', sample['inputWarnings'][0])
        self.assertEqual((self.output / sample['path']).read_bytes(), data)

    def test_newly_covered_release_is_excluded(self):
        (self.repo / 'src-tauri/src/card/game_details.rs').write_text('"SLUS-00603" => ()')
        self.assertEqual(self.build()['jobs'], [])

    def test_refresh_uses_contents_reports_ambiguity_and_rejects_wrong_region(self):
        self.build()
        folder = self.output / 'inputs/roms/SLUS-00603'
        (folder / 'misleading-name.iso').write_bytes(iso())
        manifest = build_bundle.refresh_roms(self.output, [self.output / 'inputs/roms'])
        self.assertEqual(manifest['jobs'][0]['releases'][0]['romStatus'], 'matched-serial')
        (folder / 'second.iso').write_bytes(iso())
        manifest = build_bundle.refresh_roms(self.output, [self.output / 'inputs/roms'])
        self.assertEqual(manifest['jobs'][0]['releases'][0]['romStatus'], 'ambiguous')
        (folder / 'second.iso').unlink()
        (folder / 'misleading-name.iso').write_bytes(iso('BOOT = cdrom:\\SLES_006.03;1\n', executable='SLES_006.03;1'))
        manifest = build_bundle.refresh_roms(self.output, [self.output / 'inputs/roms'])
        self.assertEqual(manifest['jobs'][0]['releases'][0]['romStatus'], 'missing')
        self.assertEqual(manifest['romScan'][0]['serial'], 'SLES-00603')
        bios = self.output / 'inputs/bios/new.bin'; bios.write_bytes(b'bios')
        manifest = build_bundle.refresh_roms(self.output, [self.output / 'inputs/roms'])
        self.assertEqual(manifest['bios'][0]['path'], 'inputs/bios/new.bin')


if __name__ == '__main__':
    unittest.main()
