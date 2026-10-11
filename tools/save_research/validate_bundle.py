#!/usr/bin/env python3
"""Read-only preflight for a portable research bundle; does not execute parsers."""
import argparse
import ast
import json
from pathlib import Path
import re
import sys

from build_bundle import sha, validate_sample
from disc_id import inspect_disc
from harness import local_file

REQUIRED = ('LICENSE', 'AGENTS.md', 'RUN.md', 'jobs.json', 'checksums.sha256', 'tooling.sha256',
            'tools/save_research/harness.py', 'tools/save_research/parser_template.py',
            'tools/save_research/disc_id.py', 'tools/save_research/build_bundle.py',
            'tools/save_research/validate_bundle.py', 'tools/ps1_replay/host.py',
            'tools/ps1_replay/runner.py', 'tools/ps1_replay/adapter.lua',
            'tools/ps1_replay/recipe.json', 'tools/ps1_replay/build-lock.json',
            'tools/ps1_replay/Containerfile', 'tools/inspect_mml2.py',
            '.agents/skills/ps1-save-research/SKILL.md', '.agents/skills/emulator-replay/SKILL.md')


def validate_bundle(bundle, require_roms=False):
    root = Path(bundle).resolve()
    errors, warnings, hashes, checked = [], [], {}, {}
    missing, ambiguous, codes = [], [], set()
    templates, without_values = [], []
    counts = {'games': 0, 'releases': 0, 'samples': 0, 'matchedRoms': 0}

    def check(label, action):
        try:
            return action()
        except (OSError, ValueError, KeyError, TypeError, SyntaxError) as error:
            errors.append(f'{label}: {error}')
            return None

    def file(relative):
        return local_file(root, relative)

    def verified(relative, expected):
        if not isinstance(expected, str) or not re.fullmatch(r'[0-9a-f]{64}', expected):
            raise ValueError('Invalid SHA-256')
        path = file(relative)
        if path not in hashes:
            hashes[path] = sha(path)
        if hashes[path] != expected:
            raise ValueError(f'SHA-256 mismatch: {relative}')
        return path

    def checksum_list(name, inputs):
        entries = {}
        for line in file(name).read_text().splitlines():
            if not line:
                continue
            match = re.fullmatch(r'([0-9a-f]{64})  (.+)', line)
            if not match or match[2] in entries:
                raise ValueError('Malformed or duplicate checksum entry')
            digest, relative = match.groups()
            if inputs != relative.startswith('inputs/'):
                raise ValueError(f'Wrong checksum ledger for {relative}')
            verified(relative, digest)
            entries[relative] = digest
        if not entries:
            raise ValueError('Empty checksum ledger')
        return entries

    for relative in REQUIRED:
        check(relative, lambda relative=relative: file(relative))
    for name in ('checksums.sha256', 'tooling.sha256'):
        checked[name] = check(name, lambda name=name: checksum_list(name, name == 'checksums.sha256')) or {}
    actual_inputs = {str(path.relative_to(root)) for path in (root / 'inputs').rglob('*') if path.is_file()}
    if actual_inputs != set(checked['checksums.sha256']):
        errors.append('Input checksum ledger does not cover exactly the current input files')
    tooling = {str(path.relative_to(root)) for folder in ('tools', '.agents', 'docs')
               for path in (root / folder).rglob('*') if path.is_file()
               and not any(part in ('runs', '.cache', '__pycache__') for part in path.relative_to(root).parts)
               and path.suffix != '.pyc'} | {'LICENSE', 'AGENTS.md', 'RUN.md'}
    if tooling != set(checked['tooling.sha256']):
        errors.append('Tooling checksum ledger does not cover exactly the shipped tools and instructions')
    for relative in tooling:
        if relative.endswith('.py'):
            check(relative, lambda relative=relative: ast.parse(file(relative).read_text(), filename=relative))
    manifest = check('jobs.json', lambda: json.loads(file('jobs.json').read_text()))
    if isinstance(manifest, dict):
        def jobs():
            if manifest.get('schemaVersion') != 1 or not isinstance(manifest.get('jobs'), list):
                raise ValueError('Expected a version-1 jobs manifest')
            counts['games'] = len(manifest['jobs'])
            for job in manifest['jobs']:
                for release in job['releases']:
                    code = release['productCode']
                    if not re.fullmatch(r'[A-Z]{4}[-P]\d{5}', code) or code in codes:
                        raise ValueError(f'Invalid or duplicate release: {code}')
                    codes.add(code); counts['releases'] += 1
                    suite = json.loads(file(release['suitePath']).read_text())
                    if suite.get('schemaVersion') != 1 or suite['parser'] != f'parsers/{code}.py':
                        raise ValueError(f'Suite/parser mismatch for {code}')
                    ast.parse(file(suite['parser']).read_text())
                    if file(suite['parser']).read_bytes() == file('tools/save_research/parser_template.py').read_bytes():
                        templates.append(code)
                    if not any(case.get('expect') for case in suite['cases'] if 'expectErrorContains' not in case):
                        without_values.append(code)
                    for helper in suite.get('parserDependencies', []):
                        file(helper)
                    samples = {sample['path']: sample for sample in release['samples']}
                    if not samples or len(samples) != len(release['samples']):
                        raise ValueError(f'Empty or duplicate sample list: {code}')
                    counts['samples'] += len(samples)
                    case_ids, case_inputs = set(), set()
                    for case in suite['cases']:
                        if not isinstance(case['id'], str) or not case['id'] or case['id'] in case_ids:
                            raise ValueError(f'Invalid or duplicate case ID: {code}')
                        case_ids.add(case['id']); case_inputs.add(case['input'])
                        verified(case['input'], case['sha256'])
                        if case['metadata']['productCode'] != code:
                            raise ValueError(f'Case release mismatch: {case["id"]}')
                        if case['input'] in samples:
                            sample = samples[case['input']]
                            if (case['sha256'] != sample['fileSha256'] or
                                    any(case['metadata'][key] != sample[key] for key in ('region', 'identifier'))):
                                raise ValueError(f'Case/sample metadata mismatch: {case["id"]}')
                    if not set(samples).issubset(case_inputs):
                        raise ValueError(f'Suite omits an inventoried sample: {code}')
                    for relative, sample in samples.items():
                        path = verified(relative, sample['fileSha256'])
                        warnings.extend(f'{code}: {warning}' for warning in validate_sample(path, sample))
                        provenance = sample.get('provenance', {})
                        if 'cardPath' in provenance:
                            verified(provenance['cardPath'], provenance['sourceFileSha256'])
                    discs = release.get('discs', [])
                    status = 'missing' if not discs else 'matched-serial' if len(discs) == 1 else 'ambiguous'
                    if release.get('romStatus') != status:
                        raise ValueError(f'Inconsistent ROM status for {code}')
                    if status == 'missing':
                        missing.append(code)
                    elif status == 'ambiguous':
                        ambiguous.append(code)
                    else:
                        counts['matchedRoms'] += 1
                    for disc in discs:
                        disc_path = file(disc['path'])
                        if not disc_path.is_relative_to(root / 'inputs/roms'):
                            raise ValueError('Disc path must stay inside inputs/roms')
                        identified = inspect_disc(disc_path)
                        if identified['serial'] != code or disc['serial'] != code:
                            raise ValueError(f'Embedded disc ID disagrees with release: {code}')
                        declared_files = {str(verified(item['path'], item['sha256'])) for item in disc['files']}
                        if declared_files != set(identified['files']):
                            raise ValueError(f'Incomplete CUE dependency manifest: {code}')
            for bios in manifest.get('bios', []):
                verified(bios['path'], bios['sha256'])
        check('manifest/suites/inputs', jobs)
        if not manifest.get('bios'):
            warnings.append('No BIOS supplied; emulator research is unavailable')
        warnings.append('Disc/BIOS compatibility and Docker emulator boot require server qualification')
    else:
        errors.append('jobs.json must contain an object')
    if missing:
        (errors if require_roms else warnings).append('Missing ROMs: ' + ', '.join(missing))
    if ambiguous:
        (errors if require_roms else warnings).append('Ambiguous ROMs: ' + ', '.join(ambiguous))
    if templates:
        warnings.append(f'{len(templates)} parsers are unimplemented templates')
    if without_values:
        warnings.append(f'{len(without_values)} suites lack positive known-value expectations')
    return {'schemaVersion': 1, 'status': 'failed' if errors else 'passed', 'pythonVersion': sys.version,
            'counts': counts, 'missingRoms': missing, 'ambiguousRoms': ambiguous,
            'templateParsers': templates, 'suitesWithoutKnownValues': without_values,
            'hashedFiles': len(hashes), 'errors': errors, 'warnings': list(dict.fromkeys(warnings)),
            'interpretation': 'Bundle integrity and structure only; no parser semantics or emulator execution certified.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--require-roms', action='store_true')
    parser.add_argument('--output', type=Path, help='Optional new JSON evidence file; existing reports are never overwritten')
    args = parser.parse_args()
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open('x') as file:
            report = validate_bundle(args.bundle, args.require_roms)
            json.dump(report, file, indent=2); file.write('\n')
    else:
        report = validate_bundle(args.bundle, args.require_roms)
    print(json.dumps(report, indent=2))
    raise SystemExit(report['status'] == 'failed')


if __name__ == '__main__':
    main()
