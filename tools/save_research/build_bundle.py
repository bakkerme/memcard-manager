#!/usr/bin/env python3
"""Assemble a private, portable parser/emulator research folder from this checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess

from disc_id import discover_images, inspect_disc
from harness import local_file

REPO = Path(__file__).resolve().parents[2]


def sha(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as source:
        for data in iter(lambda: source.read(1024 * 1024), b''):
            value.update(data)
    return value.hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


def copy_verified(source, destination, expected=None):
    source, destination = Path(source), Path(destination)
    before = sha(source)
    if expected is not None and before != expected:
        raise ValueError(f'Source changed since inventory: {source}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with source.open('rb') as original, destination.open('xb') as target:
        shutil.copyfileobj(original, target, 1024 * 1024)
    if sha(destination) != before or sha(source) != before:
        raise ValueError(f'Copy/source verification failed: {source}')
    return before


def copy_code(repo, output):
    for folder in ('tools/ps1_replay', '.agents/skills/emulator-replay', '.agents/skills/ps1-save-research'):
        source = repo / folder
        if not source.is_dir():
            raise ValueError(f'Missing required research tooling: {source}')
        for path in sorted(source.rglob('*')):
            relative = path.relative_to(source)
            if path.is_file() and not path.is_symlink() and not any(part in ('runs', '.cache', '__pycache__') for part in relative.parts) and path.suffix != '.pyc':
                copy_verified(path, output / folder / relative)
    for name in ('harness.py', 'parser_template.py', 'disc_id.py', 'build_bundle.py', 'validate_bundle.py', 'README.md',
                 'test_harness.py', 'test_disc_id.py', 'test_build_bundle.py', 'test_validate_bundle.py'):
        copy_verified(repo / 'tools/save_research' / name, output / 'tools/save_research' / name)
    for name in ('tools/inspect_mml2.py', 'tools/inspect_ape_escape.py', 'LICENSE',
                 'docs/save-details.md', 'docs/formats/mega-man-legends-2.md',
                 'docs/formats/ape-escape.md', 'docs/research/linux-replay-first-test.md',
                 'docs/planning/ps1-save-format-catalogue.json'):
        copy_verified(repo / name, output / name)


def extract_with_existing_engine(repo, source, slot, target):
    result = subprocess.run(['cargo', 'run', '--quiet', '--manifest-path', str(repo / 'src-tauri/Cargo.toml'),
                             '--example', 'export_research_save', '--', str(source), str(slot), str(target)],
                            capture_output=True, text=True, timeout=180)
    if result.returncode:
        raise RuntimeError('Existing card-engine extraction failed: ' + result.stderr[-4000:])


def validate_sample(path, sample):
    data = Path(path).read_bytes()
    if len(data) != 128 + sample['blocks'] * 8192:
        raise ValueError('MCS payload size disagrees with the inventory')
    if data[:1] not in (b'\x51', b'\xa1') or int.from_bytes(data[4:8], 'little') != len(data) - 128:
        raise ValueError('MCS directory type/size disagrees with the payload')
    actual = data[10:30].split(b'\0', 1)[0].decode('ascii')
    if actual != sample['filename'] or hashlib.sha256(data[128:]).hexdigest() != sample['payloadSha256']:
        raise ValueError('MCS identifier or payload hash disagrees with the inventory')
    warnings = []
    if data[128:130] != b'SC':
        warnings.append('Payload lacks the conventional SC signature; retain original bytes and investigate layout/corruption')
    if data[131] != sample['blocks']:
        warnings.append('SC block-count byte disagrees with the directory/payload geometry; layout requires review')
    return warnings


def discover_discs(roots):
    candidates, reports = set(), []
    for root in roots:
        root = Path(root)
        if not root.is_dir():
            reports.append({'path': str(root), 'error': 'ROM root is unavailable'})
            continue
        try:
            found, _ = discover_images([root])
            candidates.update(found)
        except (ValueError, OSError, UnicodeError) as error:
            reports.append({'path': str(root), 'error': str(error)})
    for path in sorted(candidates):
        try:
            report = inspect_disc(path)
        except (ValueError, OSError, UnicodeError) as error:
            report = {'path': str(path), 'error': str(error)}
        reports.append(report)
    return reports


def refresh_roms(bundle, roots):
    """Re-scan added images and update portable paths; no disc is renamed/copied."""
    bundle = Path(bundle).resolve()
    manifest = json.loads((bundle / 'jobs.json').read_text())
    # Refresh may intentionally add/change ROMs; it must not bless altered saves.
    for job in manifest['jobs']:
        for release in job['releases']:
            for sample in release['samples']:
                path = local_file(bundle, sample['path'])
                if sha(path) != sample['fileSha256']:
                    raise ValueError(f'Save input changed; ROM refresh refused: {sample["path"]}')
                validate_sample(path, sample)
    reports = discover_discs(roots)
    serials = {}
    for report in reports:
        if 'serial' in report:
            serials.setdefault(report['serial'], []).append(report)
    for job in manifest['jobs']:
        for release in job['releases']:
            matches = serials.get(release['productCode'], [])
            release['romStatus'] = 'missing' if not matches else 'matched-serial' if len(matches) == 1 else 'ambiguous'
            release['discs'] = []
            for disc in matches:
                paths = [Path(path).resolve() for path in disc['files']]
                if not all(path.is_relative_to(bundle / 'inputs/roms') for path in paths):
                    raise ValueError('ROM refresh paths must remain inside inputs/roms')
                release['discs'].append({**{k: v for k, v in disc.items() if k not in ('path', 'files')},
                    'path': str(Path(disc['path']).relative_to(bundle)),
                    'files': [{'path': str(path.relative_to(bundle)), 'sha256': sha(path)} for path in paths]})
    manifest['romScan'] = [{**{k: v for k, v in report.items() if k not in ('path', 'files')},
                           'path': str(Path(report['path']).relative_to(bundle))} for report in reports]
    manifest['bios'] = [{'path': str(path.relative_to(bundle)), 'sha256': sha(path),
                         'compatibilityVerified': False}
                        for path in sorted((bundle / 'inputs/bios').rglob('*')) if path.is_file()]
    write_json(bundle / 'jobs.json', manifest)
    write_checksums(bundle)
    return manifest


def write_checksums(bundle):
    inputs = sorted(path for path in (bundle / 'inputs').rglob('*') if path.is_file())
    (bundle / 'checksums.sha256').write_text(''.join(f'{sha(path)}  {path.relative_to(bundle)}\n' for path in inputs))


def write_tooling_checksums(bundle):
    paths = [bundle / name for name in ('LICENSE', 'AGENTS.md', 'RUN.md') if (bundle / name).is_file()]
    for folder in ('tools', '.agents', 'docs'):
        paths.extend(path for path in (bundle / folder).rglob('*') if path.is_file()
                     and not any(part in ('runs', '.cache', '__pycache__') for part in path.relative_to(bundle).parts)
                     and path.suffix != '.pyc')
    (bundle / 'tooling.sha256').write_text(''.join(f'{sha(path)}  {path.relative_to(bundle)}\n' for path in sorted(paths)))


def build_bundle(repo, backlog, output, rom_roots=None, bios=None, copy_roms=True,
                 extractor=extract_with_existing_engine):
    repo, backlog, output = Path(repo).resolve(), Path(backlog).resolve(), Path(output).resolve()
    source_manifest = json.loads(backlog.read_text())
    if source_manifest.get('schemaVersion') != 1 or not source_manifest.get('jobs'):
        raise ValueError('Expected a version-1 backlog manifest with jobs')
    if output.exists():
        raise ValueError('Output already exists; choose a new directory. Existing files are never overwritten.')
    if output == repo or repo.is_relative_to(output) or output == backlog.parent:
        raise ValueError('Choose a separate output directory')
    rom_roots = [Path(path).resolve() for path in (rom_roots if rom_roots is not None else [repo / 'Test Roms'])]
    source_paths = [Path(s['path']).resolve() for j in source_manifest['jobs'] for r in j['releases'] for s in r['samples']]
    if any(path.is_relative_to(output) for path in source_paths) or any(root == output or root.is_relative_to(output) or output.is_relative_to(root) for root in rom_roots):
        raise ValueError('Output must not contain source inputs or overlap ROM roots')
    enabled_source = (repo / 'src-tauri/src/card/game_details.rs').read_text()
    enabled = set(re.findall(r'"([A-Z]{4}[-P]\d{5})"\s*=>', enabled_source))
    output.mkdir(parents=True)
    try:
        copy_code(repo, output)
        for folder in ('inputs/bios', 'inputs/cards', 'inputs/roms', 'inputs/saves', 'parsers', 'suites', 'results', 'specs'):
            (output / folder).mkdir(parents=True, exist_ok=True)
        jobs, copied_cards, skipped = [], {}, []
        for job in source_manifest['jobs']:
            releases = []
            for release in job['releases']:
                code = release['productCode']
                if code in enabled:
                    skipped.append(code)
                    continue
                if not re.fullmatch(r'[A-Z]{4}[-P]\d{5}', code):
                    raise ValueError('Unsafe/unknown product code in backlog')
                (output / 'inputs/roms' / code).mkdir()
                (output / 'inputs/saves' / code).mkdir()
                parser_path = output / 'parsers' / (code + '.py')
                copy_verified(repo / 'tools/save_research/parser_template.py', parser_path)
                samples, cases = [], []
                for number, sample in enumerate(release['samples'], 1):
                    original = Path(sample['path']).resolve()
                    if sha(original) != sample['fileSha256']:
                        raise ValueError(f'Source changed since inventory: {original}')
                    dest = output / 'inputs/saves' / code / (sample['payloadSha256'] + '.mcs')
                    provenance = {'sourceFileSha256': sample['fileSha256'], 'sourceFilename': original.name,
                                  'sourceSlot': sample['slot'], 'deleted': sample['deleted']}
                    if original.suffix.lower() == '.mcs':
                        copy_verified(original, dest, sample['fileSha256'])
                    else:
                        card_id = sample['fileSha256']
                        if card_id not in copied_cards:
                            card_path = output / 'inputs/cards' / (card_id + original.suffix.lower())
                            copy_verified(original, card_path, card_id)
                            copied_cards[card_id] = card_path
                        provenance['cardPath'] = str(copied_cards[card_id].relative_to(output))
                        extractor(repo, copied_cards[card_id], sample['slot'], dest)
                    warnings = validate_sample(dest, sample)
                    if sha(original) != sample['fileSha256']:
                        raise ValueError(f'Original changed during preparation: {original}')
                    relative = str(dest.relative_to(output))
                    samples.append({k: sample[k] for k in ('identifier', 'filename', 'title', 'region', 'blocks', 'payloadSha256', 'activeCopyAvailable')} |
                                   {'path': relative, 'fileSha256': sha(dest), 'provenance': provenance,
                                    'inputWarnings': warnings})
                    cases.append({'id': f'{code}-sample-{number}', 'input': relative, 'sha256': sha(dest),
                                  'metadata': {'productCode': code, 'region': sample['region'], 'identifier': sample['identifier']},
                                  'expect': {}, 'evidence': {}})
                # Suite paths stay within bundle root, as required by the parser harness.
                suite_path = output / ('suite-' + code + '.json')
                write_json(suite_path, {'schemaVersion': 1, 'parser': str(parser_path.relative_to(output)), 'cases': cases})
                releases.append({k: release[k] for k in ('productCode', 'region', 'identifiers', 'blockCounts')} |
                                {'samples': samples, 'suitePath': suite_path.name, 'romStatus': 'missing', 'discs': []})
            if releases:
                jobs.append({'game': job['game'], 'status': 'needs-parser', 'deletedOnly': job['deletedOnly'], 'releases': releases})
        bios_files = []
        if bios is not None:
            bios = Path(bios).resolve()
            digest = copy_verified(bios, output / 'inputs/bios' / bios.name)
            bios_files.append({'path': 'inputs/bios/' + bios.name, 'sha256': digest, 'compatibilityVerified': False})
        manifest = {'schemaVersion': 1, 'backlogSha256': sha(backlog), 'jobs': jobs,
                    'skippedParserReleases': skipped, 'bios': bios_files,
                    'matchingPolicy': 'Exact disc boot serial match only; save/disc aliases, revisions and multidisc relationships require explicit evidence.'}
        write_json(output / 'jobs.json', manifest)
        source_scan = discover_discs(rom_roots) if copy_roms else []
        manifest['sourceRomScan'] = [{**{k: v for k, v in report.items() if k not in ('path', 'files')},
                                    'sourceFilename': Path(report['path']).name} for report in source_scan]
        write_json(output / 'jobs.json', manifest)
        if copy_roms:
            for report in source_scan:
                if report.get('serial') not in {r['productCode'] for j in jobs for r in j['releases']}:
                    continue
                cue = Path(report['path'])
                disc_folder = output / 'inputs/roms' / report['serial'] / cue.stem
                if disc_folder.exists():
                    raise ValueError('Multiple source discs share a destination name; rename the source folder/file to distinguish them')
                disc_folder.mkdir()
                for path in map(Path, report['files']):
                    relative = path.relative_to(cue.parent)
                    copy_verified(path, disc_folder / relative)
        manifest = refresh_roms(output, [output / 'inputs/roms'])
        (output / '.gitignore').write_text('inputs/\nresults/\ntools/ps1_replay/.cache/\ntools/ps1_replay/runs/\n')
        (output / 'AGENTS.md').write_text('''# Standalone PS1 save research

Use .agents/skills/ps1-save-research and emulator-replay. Start with jobs.json.
Implement only the listed unsupported releases. Work on active saves first;
deleted-only samples need explicit loadability checks. Do not change originals.
Parser interface and validation instructions: tools/save_research/README.md.
Suite expectations begin empty: add independent known-value, malformed, boundary
and corruption cases before claiming useful coverage. Template parsers reject
all inputs until implemented. Keep unknown IDs/checksums and partial evidence
explicit. Mount originals read-only for runtime research; retain failures.
The included replay recipe is specific to US Legends 2, not a generic loader.
Adapt and qualify each new game. Default server engine is Docker CE on ARM64;
pass --engine docker explicitly to host.py. File parsing needs only Python;
emulator research also needs matching discs, BIOS and an accessible engine.
Readiness pilots include bounded startup diagnosis within the requested scope.
For card writes, read .agents/skills/emulator-replay/references/startup-preservation.md.
Retain failed probes; only an evidenced boot-initialized disposable baseline can
precede exact whole-card preservation during replay. Report each gate separately.
Produce per-release parser code, a partial spec/ledger and validation report.
Do not schedule or launch an overnight job merely because this bundle exists.
''')
        (output / 'RUN.md').write_text('''# Portable research bundle

This folder is private and self-contained. Source originals were copied and
verified; card-only samples were exported with the existing MemcardRex parser
port by Shendo. GPL-3.0-or-later; attribution remains in copied tools/docs.

Fill inputs/roms/PRODUCT-CODE/ with each complete cue/bin directory (one cue per
disc subfolder), or an ISO. Then run from this folder:

```sh
# Optional: preview and apply serial-based filenames across all ROM subfolders.
python3 tools/save_research/disc_id.py inputs/roms --rename --dry-run
python3 tools/save_research/disc_id.py inputs/roms --rename
python3 tools/save_research/build_bundle.py --refresh-roms .
python3 tools/save_research/validate_bundle.py . --output results/preflight-001.json
sha256sum -c checksums.sha256
sha256sum -c tooling.sha256
python3 tools/ps1_replay/host.py --engine docker preflight
```

Refresh updates jobs.json with boot serials and missing/ambiguous matches; it
does not rename images or force mismatches. CHD, PBP and archives need separate
extraction/conversion. Disc IDs are read from SYSTEM.CNF, with ISO directory and
PS-X EXE header checks. A serial match does not certify revision or save loading.
Add BIOS inputs explicitly; supplied BIOS compatibility is not assumed. Refresh
updates BIOS paths/hashes too. Never replace input files during an active run.

For a requested single-game readiness pilot, read the startup preservation
protocol in .agents/skills/emulator-replay/references/startup-preservation.md
and the suggested prompt in tools/ps1_replay/README.md. Retain failed probes and
diagnose startup issues within that pilot; report bundle, engine, boot, load,
input/replay and preservation separately. Reproducible boot-only scratch writes
may establish an initialized disposable baseline, followed by exact whole-card
checks. Unexplained writes remain blockers for the affected gate. Do not infer
save/load persistence or launch the overnight batch from readiness alone.

Run each root-level suite after implementing its parsers/PRODUCT-CODE.py:

```sh
python3 tools/save_research/harness.py suite-PRODUCT-CODE.json --output results/PRODUCT-CODE/run-001.json
```

Expectations are intentionally empty. A contract-only pass is not field evidence.
Add independent values and negative/boundary/checksum cases as research proceeds.
The suites/ folder is available for auxiliary manifests; root-level suites keep
input paths within the runner's path boundary. No desktop app/Rust is required
on the server. The generic tooling is copied; caches and previous runs are omitted.
The Legends 2 recipe is a reference and must be adapted before another title runs.

Check the Python-only tools without the desktop app or emulator:

```sh
python3 -m unittest discover -s tools/save_research -p 'test_*.py'
python3 -m unittest discover -s tools/ps1_replay -p 'test_*.py'
```

Bundle preflight checks all input/tooling hashes, portable paths, suite metadata,
and disc contents. Missing ROMs are warnings for file-only research; use
`validate_bundle.py . --require-roms` when every release must have a disc.
Parser templates intentionally fail until implemented. Do not count a template
rejection or an empty expectation set as completed research. This bundle is
preparation only; a real Docker emulator run still needs server qualification.
''')
        write_checksums(output)
        write_tooling_checksums(output)
        return manifest
    except Exception as error:
        # Keep partial output/evidence so a preparation failure is inspectable.
        write_json(output / 'build-error.json', {'error': str(error)})
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, help='New folder to create (never overwritten)')
    parser.add_argument('--repo', type=Path, default=REPO)
    parser.add_argument('--backlog', type=Path)
    parser.add_argument('--roms', action='append', type=Path, help='ROM source directory; repeatable, default: Test Roms')
    parser.add_argument('--bios', type=Path, help='Optional BIOS to copy; otherwise leave its folder empty')
    parser.add_argument('--no-roms', action='store_true', help='Create all ROM folders without copying available discs')
    parser.add_argument('--refresh-roms', type=Path, metavar='BUNDLE', help='Identify manually added ROMs and refresh jobs/checksums')
    args = parser.parse_args()
    try:
        if args.refresh_roms:
            manifest = refresh_roms(args.refresh_roms, [args.refresh_roms.resolve() / 'inputs/roms'])
        else:
            if args.output is None:
                parser.error('--output is required when building')
            backlog = args.backlog
            if backlog is None:
                available = list((args.repo / 'tools/save_research/runs').glob('*/parser-jobs.json'))
                if not available:
                    parser.error('No local backlog found; supply --backlog')
                backlog = max(available, key=lambda path: path.stat().st_mtime)
            manifest = build_bundle(args.repo, backlog, args.output, args.roms, args.bios, not args.no_roms)
        releases = [r for job in manifest['jobs'] for r in job['releases']]
        print(json.dumps({'games': len(manifest['jobs']), 'releases': len(releases),
                          'samples': sum(len(r['samples']) for r in releases),
                          'missingRoms': [r['productCode'] for r in releases if r['romStatus'] == 'missing'],
                          'ambiguousRoms': [r['productCode'] for r in releases if r['romStatus'] == 'ambiguous']}, indent=2))
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        parser.exit(1, str(error) + '\n')


if __name__ == '__main__':
    main()
