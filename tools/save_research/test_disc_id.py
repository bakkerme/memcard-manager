"""Constructed ISO/CUE tests: no copyrighted disc fixtures are needed."""
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from disc_id import cue_layout, inspect_disc, rename_discs


def record(name, lba, length, directory=False):
    name = name.encode('ascii')
    data = bytearray(33 + len(name) + (len(name) % 2 == 0))
    data[0] = len(data)
    struct.pack_into('<I', data, 2, lba); struct.pack_into('>I', data, 6, lba)
    struct.pack_into('<I', data, 10, length); struct.pack_into('>I', data, 14, length)
    data[25] = 2 if directory else 0
    data[28:32] = b'\x01\0\0\x01'
    data[32] = len(name); data[33:33 + len(name)] = name
    return data


def iso(boot='BOOT = cdrom:\\SLUS_006.03;1\r\n', with_cnf=True, executable='SLUS_006.03;1'):
    sectors = [bytearray(2048) for _ in range(25)]
    pvd = sectors[16]; pvd[:7] = b'\x01CD001\x01'
    pvd[40:72] = b'TEST'.ljust(32, b' ')
    struct.pack_into('<H', pvd, 128, 2048); struct.pack_into('>H', pvd, 130, 2048)
    root = record('\0', 20, 2048, True); pvd[156:156 + len(root)] = root
    entries = [record('\0', 20, 2048, True), record('\1', 20, 2048, True)]
    if with_cnf:
        entries.append(record('SYSTEM.CNF;1', 21, len(boot)))
    entries.append(record(executable, 22, 2048))
    content = b''.join(entries); sectors[20][:len(content)] = content
    sectors[21][:len(boot)] = boot.encode()
    sectors[22][:8] = b'PS-X EXE'
    return b''.join(sectors)


def raw_image(cooked, mode):
    output = bytearray()
    for index in range(0, len(cooked), 2048):
        sector = bytearray(2352)
        sector[:12] = b'\0' + b'\xff' * 10 + b'\0'
        sector[15] = mode
        offset = 16 if mode == 1 else 24
        sector[offset:offset + 2048] = cooked[index:index + 2048]
        output.extend(sector)
    return output


class DiscIdTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_cooked_and_both_raw_modes_identify_from_boot_contents(self):
        for name, data in [('unrelated.iso', iso()), ('one.bin', raw_image(iso(), 1)), ('two.bin', raw_image(iso(), 2))]:
            path = self.root / name; path.write_bytes(data)
            self.assertEqual(inspect_disc(path)['serial'], 'SLUS-00603')
            self.assertEqual(path.read_bytes(), data)

    def test_cue_respects_data_index_and_preserves_audio_dependency(self):
        binary = self.root / 'data.bin'
        binary.write_bytes(bytes(150 * 2352) + raw_image(iso(), 2))
        audio = self.root / 'audio.bin'; audio.write_bytes(b'audio')
        cue = self.root / 'game.cue'
        cue.write_text('FILE "data.bin" BINARY\n TRACK 01 MODE2/2352\n INDEX 01 00:02:00\nFILE "audio.bin" BINARY\n TRACK 02 AUDIO\n INDEX 01 00:00:00\n')
        result = inspect_disc(cue)
        self.assertEqual(result['serial'], 'SLUS-00603')
        self.assertEqual(result['startSector'], 150)
        self.assertIn(str(audio.resolve()), result['files'])

    def test_2336_mode_and_nested_boot_directory(self):
        cooked = bytearray(iso('BOOT = cdrom:\\SUBDIR\\SLUS_006.03;1\n'))
        root = b''.join([record('\0', 20, 2048, True), record('\1', 20, 2048, True),
                         record('SYSTEM.CNF;1', 21, 38), record('SUBDIR', 23, 2048, True)])
        cooked[20 * 2048:21 * 2048] = root.ljust(2048, b'\0')
        nested = record('SLUS_006.03;1', 22, 2048)
        cooked[23 * 2048:24 * 2048] = nested.ljust(2048, b'\0')
        raw = raw_image(cooked, 2)
        compact = b''.join(raw[i + 16:i + 2352] for i in range(0, len(raw), 2352))
        path = self.root / 'compact.bin'; path.write_bytes(compact)
        self.assertEqual(inspect_disc(path)['serial'], 'SLUS-00603')

    def test_missing_cnf_boot2_nonserial_and_nonexe_do_not_guess(self):
        data_cases = [iso(with_cnf=False), iso('BOOT2 = cdrom:\\SLUS_006.03;1\n'),
                      iso('BOOT = cdrom:\\PSX.EXE;1\n'), iso()[:-2048 * 3]]
        nonexe = bytearray(iso()); nonexe[22 * 2048:22 * 2048 + 8] = b'NOT EXE!'
        data_cases.append(nonexe)
        for data in data_cases:
            path = self.root / 'SLUS-00603.iso'; path.write_bytes(data)
            with self.assertRaises(ValueError): inspect_disc(path)

    def test_malformed_extent_and_form2_are_rejected(self):
        cooked = bytearray(iso()); cooked[16 * 2048 + 156 + 6] ^= 1
        path = self.root / 'bad.iso'; path.write_bytes(cooked)
        with self.assertRaises(ValueError): inspect_disc(path)
        raw = raw_image(iso(), 2); raw[16 * 2352 + 18] = 0x20; raw[16 * 2352 + 22] = 0x20
        path = self.root / 'bad.bin'; path.write_bytes(raw)
        with self.assertRaises(ValueError): inspect_disc(path)

    def test_unsupported_container_and_unsafe_or_missing_cue_paths(self):
        with self.assertRaisesRegex(ValueError, 'extract/convert'): inspect_disc(self.root / 'game.chd')
        cue = self.root / 'bad.cue'
        for name in ['../escape.bin', '/absolute.bin', 'missing.bin']:
            cue.write_text(f'FILE "{name}" BINARY\nTRACK 01 MODE2/2352\nINDEX 01 00:00:00\n')
            with self.assertRaises(ValueError): cue_layout(cue)


class RenameTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()

    def cue(self, folder=None):
        folder = folder or self.root
        folder.mkdir(parents=True, exist_ok=True)
        data = folder / 'old data.bin'; data.write_bytes(raw_image(iso(), 2))
        audio = folder / 'old audio.bin'; audio.write_bytes(b'audio bytes')
        cue = folder / 'old game.cue'
        cue.write_bytes(b'\xef\xbb\xbfREM original comment\r\nFILE "old data.bin" BINARY\r\n TRACK 01 MODE2/2352\r\n INDEX 01 00:00:00\r\nFILE "old audio.bin" BINARY\r\n TRACK 02 AUDIO\r\n INDEX 01 00:00:00\r\n')
        return cue, data, audio

    def test_recursive_preview_then_rename_preserves_images_and_cue_format(self):
        cue, data, audio = self.cue(self.root / 'Game title' / 'nested')
        before = {path: path.read_bytes() for path in (cue, data, audio)}
        archive = self.root / 'download.7z'; archive.write_bytes(b'archive')
        preview = rename_discs([self.root], dry_run=True)
        self.assertEqual(len(preview), 1)
        self.assertEqual(preview[0]['status'], 'planned')
        self.assertEqual(len(preview[0]['renames']), 3)
        self.assertEqual(set(self.root.rglob('*.jsonl')), set())
        for path, content in before.items(): self.assertEqual(path.read_bytes(), content)
        report = rename_discs([self.root])[0]
        self.assertEqual(report['status'], 'renamed')
        renamed = cue.parent / 'SLUS-00603.cue'
        self.assertEqual(inspect_disc(renamed)['serial'], 'SLUS-00603')
        for old, new in [(data, 'SLUS-00603 (Track 01).bin'), (audio, 'SLUS-00603 (Track 02).bin')]:
            self.assertFalse(old.exists())
            self.assertEqual((old.parent / new).read_bytes(), before[old])
        expected = before[cue].replace(b'old data.bin', b'SLUS-00603 (Track 01).bin').replace(b'old audio.bin', b'SLUS-00603 (Track 02).bin')
        self.assertEqual(renamed.read_bytes(), expected)
        self.assertEqual(archive.read_bytes(), b'archive')
        import json
        records = [json.loads(line) for line in Path(report['log']).read_text().splitlines()]
        self.assertEqual([item['status'] for item in records], ['planned', 'renamed'])
        self.assertEqual(records[0]['originalCue'].encode(), before[cue])
        self.assertEqual(rename_discs([self.root])[0]['status'], 'unchanged')

    def test_iso_and_standalone_bin_in_separate_directories(self):
        for suffix, content in [('.iso', iso()), ('.bin', raw_image(iso(), 1))]:
            directory = self.root / suffix[1:]; directory.mkdir()
            (directory / ('unrelated' + suffix)).write_bytes(content)
        reports = rename_discs([self.root])
        self.assertEqual([item['status'] for item in reports], ['renamed', 'renamed'])
        self.assertEqual((self.root / 'iso' / 'SLUS-00603.iso').read_bytes(), iso())

    def test_existing_destinations_and_batch_collisions_are_never_overwritten(self):
        one = self.root / 'one.iso'; one.write_bytes(iso())
        target = self.root / 'SLUS-00603.iso'; target.write_bytes(b'keep me')
        report = rename_discs([one])[0]
        self.assertEqual(report['status'], 'skipped')
        self.assertEqual(target.read_bytes(), b'keep me')
        target.unlink()
        two = self.root / 'two.iso'; two.write_bytes(iso())
        reports = rename_discs([self.root])
        self.assertTrue(all(item['status'] == 'skipped' for item in reports))
        self.assertTrue(one.exists() and two.exists())
        self.assertFalse(target.exists())

    def test_failed_cue_still_protects_its_audio_and_data_tracks(self):
        cue, data, audio = self.cue()
        data.write_bytes(raw_image(iso(with_cnf=False), 2))
        reports = rename_discs([self.root])
        self.assertEqual(len(reports), 1)
        self.assertEqual(reports[0]['status'], 'skipped')
        self.assertTrue(cue.exists() and data.exists() and audio.exists())

    def test_shared_cue_dependencies_are_skipped(self):
        cue, data, audio = self.cue()
        other = self.root / 'other.cue'; other.write_bytes(cue.read_bytes())
        reports = rename_discs([self.root])
        self.assertEqual(len(reports), 2)
        self.assertTrue(all('shared' in item['error'] for item in reports))
        self.assertTrue(cue.exists() and other.exists() and data.exists() and audio.exists())
        # Selecting only one CUE must still notice the neighbouring owner.
        self.assertIn('shared', rename_discs([cue])[0]['error'])

    def test_explicit_track_cannot_break_neighbouring_cue(self):
        cue, data, audio = self.cue()
        report = rename_discs([data])[0]
        self.assertEqual(report['status'], 'skipped')
        self.assertIn('belongs to a CUE', report['error'])
        self.assertTrue(data.exists())
        self.assertEqual(inspect_disc(cue)['serial'], 'SLUS-00603')

    def test_unreadable_cue_protects_nearby_tracks_without_blocking_other_folders(self):
        cue, data, audio = self.cue(self.root / 'bad')
        cue.write_bytes(b'\xffinvalid encoding')
        good = self.root / 'good'; good.mkdir()
        (good / 'old.iso').write_bytes(iso())
        reports = rename_discs([self.root])
        self.assertEqual(len(reports), 2)
        self.assertEqual([item['status'] for item in reports], ['skipped', 'renamed'])
        self.assertTrue(data.exists() and audio.exists())
        self.assertTrue((good / 'SLUS-00603.iso').exists())

    def test_rewrite_failure_rolls_back_file_names(self):
        cue, data, audio = self.cue()
        original = cue.read_bytes()
        with patch('disc_id.replace_cue', side_effect=OSError('injected write failure')):
            report = rename_discs([self.root])[0]
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['rollbackErrors'], [])
        self.assertEqual(cue.read_bytes(), original)
        self.assertTrue(data.exists() and audio.exists())
        self.assertFalse((self.root / 'SLUS-00603.cue').exists())

    def test_target_created_after_plan_is_not_overwritten(self):
        import disc_id
        source = self.root / 'old.iso'; source.write_bytes(iso())
        original_move = disc_id.move_no_replace
        def racing_move(old, new):
            Path(new).write_bytes(b'concurrent file')
            original_move(old, new)
        with patch('disc_id.move_no_replace', side_effect=racing_move):
            report = rename_discs([source])[0]
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(source.exists())
        self.assertEqual((self.root / 'SLUS-00603.iso').read_bytes(), b'concurrent file')

    def test_bad_log_destination_fails_before_mutation(self):
        source = self.root / 'old.iso'; source.write_bytes(iso())
        log = self.root / 'existing.jsonl'; log.write_text('keep log')
        with self.assertRaises(FileExistsError): rename_discs([source], log=log)
        self.assertTrue(source.exists())
        self.assertEqual(log.read_text(), 'keep log')

    def test_postrename_validation_failure_restores_original_cue(self):
        cue, data, audio = self.cue()
        original = cue.read_bytes()
        real_inspect = inspect_disc
        def fail_after_rename(path):
            if Path(path).name == 'SLUS-00603.cue': raise ValueError('injected validation failure')
            return real_inspect(path)
        with patch('disc_id.inspect_disc', side_effect=fail_after_rename):
            report = rename_discs([self.root])[0]
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['rollbackErrors'], [])
        self.assertEqual(cue.read_bytes(), original)
        self.assertTrue(data.exists() and audio.exists())


if __name__ == '__main__':
    unittest.main()
