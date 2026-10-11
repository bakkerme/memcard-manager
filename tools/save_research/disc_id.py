#!/usr/bin/env python3
"""Identify PS1 discs, or recursively rename discs and CUE tracks by boot serial.

Format sources: Sony's Third Party Technical Check Guide, SYSTEM.CNF section,
and PSX-SPX CDROM format/disk-image notes. Image bytes are never modified;
--rename changes filenames and rewrites CUE FILE references, retaining a log.
"""
import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import tempfile

MODES = {'MODE1/2048': (2048, 0), 'MODE1/2352': (2352, 16),
         'MODE2/2352': (2352, 24), 'MODE2/2336': (2336, 8)}
FILE_LINE = re.compile(r'^(\s*FILE\s+)(?:"([^"\r\n]+)"|([^\s]+))(\s+\S+[^\r\n]*)(\r?\n|\r|)$', re.I)


def cue_layout(path):
    """Resolve all FILE entries and the single supported data track's INDEX 01."""
    path = Path(path).resolve()
    files, tracks = [], []
    current_file = current_track = None
    for line in path.read_text(encoding='utf-8-sig').splitlines():
        match = re.match(r'\s*FILE\s+(?:"([^"]+)"|(\S+))\s+(\S+)\s*$', line, re.I)
        if match:
            relative = Path(match[1] or match[2])
            if relative.is_absolute() or '..' in relative.parts or match[3].upper() != 'BINARY':
                raise ValueError('CUE requires relative BINARY files within its directory')
            literal = path.parent / relative
            current_file = literal.resolve()
            if literal != current_file:
                raise ValueError('CUE dependencies through symlinks are unsupported; supply regular files')
            if not current_file.is_relative_to(path.parent) or not current_file.is_file():
                raise ValueError(f'Missing or escaping CUE dependency: {relative}')
            if current_file not in files:
                files.append(current_file)
            current_track = None
            continue
        match = re.match(r'\s*TRACK\s+(\d+)\s+(\S+)\s*$', line, re.I)
        if match:
            if current_file is None:
                raise ValueError('TRACK precedes FILE')
            current_track = {'file': current_file, 'mode': match[2].upper(), 'number': int(match[1])}
            tracks.append(current_track)
            continue
        match = re.match(r'\s*INDEX\s+01\s+(\d+):(\d+):(\d+)\s*$', line, re.I)
        if match and current_track is not None:
            minutes, seconds, frames = map(int, match.groups())
            if seconds >= 60 or frames >= 75 or 'index' in current_track:
                raise ValueError('Invalid or duplicate CUE INDEX 01')
            current_track['index'] = (minutes * 60 + seconds) * 75 + frames
    data = [track for track in tracks if track['mode'] != 'AUDIO']
    if len(data) != 1 or data[0]['mode'] not in MODES or 'index' not in data[0]:
        raise ValueError('Expected one supported data track with INDEX 01')
    track = data[0]
    size, offset = MODES[track['mode']]
    for other in tracks:
        if other['file'] == track['file'] and other['mode'] != track['mode']:
            if size != 2352 or (other['mode'] != 'AUDIO' and MODES.get(other['mode'], (0,))[0] != 2352):
                raise ValueError('Shared CUE file has incompatible sector sizes')
    return {'file': track['file'], 'sectorSize': size, 'dataOffset': offset,
            'startSector': track['index'], 'files': files, 'mode': track['mode'], 'tracks': tracks}


class IsoReader:
    def __init__(self, file, size, offset, start=0):
        self.file, self.size, self.offset, self.start = file, size, offset, start

    def sector(self, lba):
        if lba < 0:
            raise ValueError('Negative ISO sector')
        self.file.seek((self.start + lba) * self.size)
        raw = self.file.read(self.size)
        if len(raw) != self.size:
            raise ValueError('Truncated disc extent')
        if self.size == 2352:
            if raw[:12] != b'\0' + b'\xff' * 10 + b'\0' or raw[15] != (1 if self.offset == 16 else 2):
                raise ValueError('Unexpected raw data-sector header')
        if self.offset in (8, 24):
            base = self.offset - 8
            if raw[base:base + 4] != raw[base + 4:base + 8] or raw[base + 2] & 0x20:
                raise ValueError('Expected duplicated Mode 2 Form 1 subheader')
        return raw[self.offset:self.offset + 2048]

    def extent(self, lba, length, limit=1024 * 1024):
        if length < 0 or length > limit:
            raise ValueError('ISO metadata extent exceeds bounded reader limit')
        return b''.join(self.sector(lba + i) for i in range((length + 2047) // 2048))[:length]

    @staticmethod
    def record(data):
        if len(data) < 34 or data[0] != len(data) or 33 + data[32] > len(data):
            raise ValueError('Malformed ISO directory record')
        lba = struct.unpack_from('<I', data, 2)[0]
        length = struct.unpack_from('<I', data, 10)[0]
        if lba != struct.unpack_from('>I', data, 6)[0] or length != struct.unpack_from('>I', data, 14)[0]:
            raise ValueError('ISO extent endian copies disagree')
        if data[1] or data[25] & 0x80 or data[26] or data[27]:
            raise ValueError('Extended, multi-extent or interleaved ISO files are unsupported')
        return {'lba': lba, 'length': length, 'directory': bool(data[25] & 2),
                'name': data[33:33 + data[32]].decode('ascii', errors='replace')}

    def entries(self, directory):
        data = self.extent(directory['lba'], directory['length'])
        entries, pos = {}, 0
        while pos < len(data):
            length = data[pos]
            if not length:
                pos = (pos // 2048 + 1) * 2048
                continue
            if pos % 2048 + length > 2048:
                raise ValueError('ISO directory record crosses a sector')
            record = self.record(data[pos:pos + length])
            name = record['name'].upper().split(';')[0]
            if name in entries:
                raise ValueError('Ambiguous ISO directory entry')
            entries[name] = record
            pos += length
        return entries

    def identify(self):
        descriptor = None
        for lba in range(16, 48):
            data = self.sector(lba)
            if data[1:6] != b'CD001' or data[6] != 1:
                raise ValueError('No supported ISO9660 volume descriptor')
            if data[0] == 1:
                descriptor = data
                break
            if data[0] == 255:
                break
        if descriptor is None or struct.unpack_from('<H', descriptor, 128)[0] != 2048 or struct.unpack_from('>H', descriptor, 130)[0] != 2048:
            raise ValueError('No 2048-byte ISO9660 primary volume descriptor')
        root = self.record(descriptor[156:156 + descriptor[156]])
        if not root['directory']:
            raise ValueError('ISO root is not a directory')
        cnf = self.entries(root).get('SYSTEM.CNF')
        if cnf is None or cnf['directory']:
            raise ValueError('SYSTEM.CNF absent; no serial inferred from filenames')
        text = self.extent(cnf['lba'], cnf['length'], limit=65536).decode('ascii', errors='strict')
        boot = re.findall(r'^\s*BOOT\s*=\s*"?cdrom:\\?([^\r\n"]+)', text, re.I | re.M)
        if len(boot) != 1:
            raise ValueError('Missing or ambiguous PS1 BOOT entry (BOOT2 is not PS1)')
        boot_path = boot[0].strip().replace('/', '\\').strip('\\')
        parts = boot_path.split('\\')
        serial = re.fullmatch(r'([A-Z]{4})[_-](\d{3})\.(\d{2})(?:;\d+)?', parts[-1], re.I)
        if serial is None:
            raise ValueError('BOOT executable has no conventional PS1 serial')
        directory = root
        for index, part in enumerate(parts):
            if not part or part in ('.', '..'):
                raise ValueError('Unexpected BOOT path component')
            entry = self.entries(directory).get(part.upper().split(';')[0])
            if entry is None or entry['directory'] != (index < len(parts) - 1):
                raise ValueError('BOOT executable or directory is absent')
            directory = entry
        if entry['length'] < 2048 or self.sector(entry['lba'])[:8] != b'PS-X EXE':
            raise ValueError('BOOT target does not have a PS-X EXE header')
        return {'serial': serial[1].upper() + '-' + serial[2] + serial[3],
                'bootPath': boot_path, 'volumeId': descriptor[40:72].decode('ascii', errors='replace').strip(),
                'method': 'ISO9660 SYSTEM.CNF BOOT + PS-X EXE header',
                'revisionVerified': False}


def inspect_disc(path):
    path = Path(path).resolve()
    if path.suffix.lower() == '.cue':
        layout = cue_layout(path)
        with layout['file'].open('rb') as file:
            result = IsoReader(file, layout['sectorSize'], layout['dataOffset'], layout['startSector']).identify()
        return result | {'path': str(path), 'sectorSize': layout['sectorSize'],
                         'startSector': layout['startSector'], 'files': [str(path), *map(str, layout['files'])]}
    if path.suffix.lower() not in ('.iso', '.bin'):
        raise ValueError('Supported inputs are .cue, .iso and standalone .bin; extract/convert other formats separately')
    errors = []
    with path.open('rb') as file:
        for size, offset in ((2048, 0), (2352, 24), (2352, 16), (2336, 8)):
            try:
                result = IsoReader(file, size, offset).identify()
                return result | {'path': str(path), 'sectorSize': size, 'startSector': 0, 'files': [str(path)]}
            except (ValueError, UnicodeError) as error:
                errors.append(str(error))
    raise ValueError('No supported disc layout: ' + '; '.join(errors))


def discover_images(inputs):
    """Recurse without following symlinks; a CUE owns its referenced tracks."""
    candidates = set()
    for supplied in inputs:
        supplied = Path(supplied).absolute()
        if supplied.is_symlink():
            raise ValueError(f'Symlink inputs are not supported: {supplied}')
        if supplied.is_dir():
            candidates.update(path.resolve() for path in supplied.rglob('*')
                              if path.is_file() and not path.is_symlink()
                              and path.suffix.lower() in ('.cue', '.iso', '.bin'))
        else:
            candidates.add(supplied.resolve())
    owners = defaultdict(set)
    cues = {path for path in candidates if path.suffix.lower() == '.cue'}
    # Also notice neighbouring CUE files for explicit-file invocations. A BIN
    # should not be renamed independently when an unselected CUE refers to it.
    for path in candidates:
        cues.update(cue.resolve() for cue in path.parent.glob('*')
                    if cue.suffix.lower() == '.cue' and cue.is_file() and not cue.is_symlink())
    for cue in sorted(cues):
        # Protect tracks even when the CUE or SYSTEM.CNF later fails validation.
        try:
            lines = cue.read_text(encoding='utf-8-sig').splitlines(keepends=True)
        except (OSError, UnicodeError):
            # An unreadable CUE cannot safely tell us which nearby images it
            # owns. Keep those images untouched while processing other folders.
            for path in candidates:
                if path != cue and path.is_relative_to(cue.parent) and path.suffix.lower() in ('.bin', '.iso'):
                    owners[path].add(cue)
            continue
        for line in lines:
            match = FILE_LINE.match(line)
            if match:
                dependency = (cue.parent / (match[2] or match[3])).resolve()
                owners[dependency].add(cue)
    return sorted(path for path in candidates if not (owners[path] & candidates)), owners


def identity(path):
    stat = path.stat()
    return stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns


def rename_plan(inputs):
    candidates, owners = discover_images(inputs)
    plans = []
    for path in candidates:
        plan = {'path': str(path)}
        try:
            if owners[path]:
                raise ValueError('This image belongs to a CUE; provide the CUE file or its entire folder')
            plan.update(inspect_disc(path))
            serial = plan['serial']
            mapping = {path: path.with_name(serial + path.suffix.lower())}
            if path.suffix.lower() == '.cue':
                layout = cue_layout(path)
                for dependency in layout['files']:
                    if owners[dependency] != {path}:
                        raise ValueError(f'Track is shared by multiple CUE files: {dependency}')
                    track = next((item['number'] for item in layout['tracks']
                                  if item['file'] == dependency), None)
                    if track is None:
                        raise ValueError(f'CUE FILE has no TRACK: {dependency}')
                    suffix = '' if len(layout['files']) == 1 else f' (Track {track:02d})'
                    mapping[dependency] = dependency.with_name(serial + suffix + dependency.suffix.lower())
                original = path.read_bytes()
                updated = []
                for line in original.decode('utf-8-sig').splitlines(keepends=True):
                    match = FILE_LINE.match(line)
                    if match:
                        literal = path.parent / (match[2] or match[3])
                        dependency = literal.resolve()
                        if literal != dependency:
                            raise ValueError('Cannot rename CUE dependencies through symlinks or noncanonical paths')
                        name = mapping[dependency].relative_to(path.parent).as_posix()
                        line = f'{match[1]}"{name}"{match[4]}{match[5]}'
                    updated.append(line)
                encoding = 'utf-8-sig' if original.startswith(b'\xef\xbb\xbf') else 'utf-8'
                plan['_cueOriginal'] = original
                plan['_cueUpdated'] = ''.join(updated).encode(encoding)
                # Original text is retained in the log for manual recovery.
                plan['originalCue'] = original.decode('utf-8')
                plan['originalCueSha256'] = hashlib.sha256(original).hexdigest()
            targets = list(mapping.values())
            if len(set(targets)) != len(targets):
                raise ValueError('Multiple disc files would receive the same name')
            for source, target in mapping.items():
                if source != target and os.path.lexists(target):
                    raise ValueError(f'Rename destination already exists: {target}')
            plan['renames'] = [{'from': str(source), 'to': str(target)}
                               for source, target in mapping.items() if source != target]
            plan['target'] = str(mapping[path])
            plan['_identities'] = {source: identity(source) for source in mapping}
            changed = bool(plan['renames']) or plan.get('_cueOriginal') != plan.get('_cueUpdated')
            plan['status'] = 'planned' if changed else 'unchanged'
        except (ValueError, OSError, UnicodeError) as error:
            plan.update(status='skipped', error=str(error))
        plans.append(plan)
    destinations = defaultdict(list)
    for plan in plans:
        if plan['status'] == 'planned':
            for move in plan['renames']:
                destinations[move['to']].append(plan)
    for target, claimants in destinations.items():
        if len(claimants) > 1:
            for plan in claimants:
                plan.update(status='skipped', error=f'Multiple discs would use destination: {target}')
    return plans


def move_no_replace(source, target):
    """Same-directory hard-link/unlink rename: never replace an existing file."""
    os.link(source, target)
    try:
        Path(source).unlink()
    except BaseException:
        Path(target).unlink()
        raise


def replace_cue(path, content):
    """Commit the tiny rewritten CUE atomically; image contents are untouched."""
    path = Path(path)
    mode = path.stat().st_mode & 0o777
    fd, temporary = tempfile.mkstemp(prefix='.disc-id-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as file:
            file.write(content)
            file.flush()
            os.fsync(file.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        if os.path.lexists(temporary):
            os.unlink(temporary)


def apply_rename(plan):
    completed = []
    cue_written = False
    try:
        for source, expected in plan['_identities'].items():
            if source.is_symlink() or identity(source) != expected:
                raise ValueError(f'Input changed after planning: {source}')
        for move in plan['renames']:
            move_no_replace(move['from'], move['to'])
            completed.append(move)
        if '_cueOriginal' in plan and plan['_cueOriginal'] != plan['_cueUpdated']:
            replace_cue(plan['target'], plan['_cueUpdated'])
            cue_written = True
        if inspect_disc(plan['target'])['serial'] != plan['serial']:
            raise ValueError('Disc serial changed after renaming')
        plan['status'] = 'renamed'
    except (ValueError, OSError, UnicodeError) as error:
        rollback_errors = []
        if cue_written:
            try:
                replace_cue(plan['target'], plan['_cueOriginal'])
            except OSError as rollback_error:
                rollback_errors.append(str(rollback_error))
        for move in reversed(completed):
            try:
                move_no_replace(move['to'], move['from'])
            except OSError as rollback_error:
                rollback_errors.append(str(rollback_error))
        plan.update(status='failed', error=str(error), rollbackErrors=rollback_errors)


def public_plan(plan):
    return {key: value for key, value in plan.items() if not key.startswith('_')}


def rename_discs(inputs, dry_run=False, log=None):
    plans = rename_plan(inputs)
    if not dry_run and any(plan['status'] == 'planned' for plan in plans):
        first = Path(inputs[0]).resolve()
        stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
        log = Path(log) if log else (first if first.is_dir() else first.parent) / f'disc-id-renames-{stamp}.jsonl'
        # Journal all intended operations before touching inputs. Each completed
        # disc then has a second record; an interrupted run retains its intent.
        with log.open('x', encoding='utf-8') as file:
            def record(plan):
                file.write(json.dumps(public_plan(plan)) + '\n')
                file.flush()
                os.fsync(file.fileno())
            for plan in plans:
                record(plan)
            for plan in plans:
                if plan['status'] == 'planned':
                    apply_rename(plan)
                    record(plan)
        for plan in plans:
            plan['log'] = str(log.resolve())
    return [public_plan(plan) for plan in plans]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('images', nargs='+', type=Path, help='Disc files or folders (folders are scanned recursively)')
    parser.add_argument('--rename', action='store_true', help='Rename identified discs and their CUE tracks by serial')
    parser.add_argument('--dry-run', action='store_true', help='Preview --rename without modifying files or writing a log')
    parser.add_argument('--log', type=Path, help='New JSONL journal path (default: timestamped log in the first input folder)')
    args = parser.parse_args()
    if (args.dry_run or args.log) and not args.rename:
        parser.error('--dry-run and --log require --rename')
    try:
        if args.rename:
            reports = rename_discs(args.images, args.dry_run, args.log)
        else:
            candidates, _ = discover_images(args.images)
            reports = []
            for path in candidates:
                try:
                    reports.append(inspect_disc(path))
                except (ValueError, OSError, UnicodeError) as error:
                    reports.append({'path': str(path), 'error': str(error)})
    except (ValueError, OSError, UnicodeError) as error:
        reports = [{'error': str(error)}]
    for report in reports:
        print(json.dumps(report))
    raise SystemExit(int(any('error' in report for report in reports)))


if __name__ == '__main__':
    main()
