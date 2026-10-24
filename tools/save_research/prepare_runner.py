#!/usr/bin/env python3
"""Import a hash-pinned legacy v2 package into the repo-local Runner workflow.

Preserves every packaged dependency and its relative path. No remote mutations,
job IDs, control keys, custom transport or source-checkout imports.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import tempfile

REPO = Path(__file__).resolve().parents[2]


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def checked(root, relative):
    root = Path(root).absolute()
    p = root / relative
    if not p.resolve().is_relative_to(root.resolve()) or not p.is_file():
        raise ValueError(f'Unsafe/missing dependency: {p}')
    if any(part.is_symlink() for part in (p, *p.parents) if part != root and part.is_relative_to(root)):
        raise ValueError(f'Unsafe/missing dependency: {p}')
    return p


def suite_files(root, suite):
    p = checked(root, suite)
    data = json.loads(p.read_text())
    paths = [p, checked(root, (p.parent / data['parser']).relative_to(root))]
    paths += [checked(root, (p.parent / x).relative_to(root)) for x in data.get('parserDependencies', [])]
    for case in data['cases']:
        fixture = checked(root, (p.parent / case['input']).relative_to(root))
        if sha(fixture) != case['sha256']:
            raise ValueError(f'Fixture hash mismatch: {fixture}')
        paths.append(fixture)
    return paths


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def import_package(repo, name, archive, expected_sha256):
    # Keep the module usable by standard-library file-audit tests without Runner.
    from runner.package import deploy
    from runner.protocol import schemas

    if not re.fullmatch(r"[a-z][a-z0-9-]{0,63}", name):
        raise ValueError("Invalid definition name")
    archive = Path(archive).resolve()
    if not re.fullmatch('[0-9a-f]{64}', expected_sha256) or sha(archive) != expected_sha256:
        raise ValueError('Archive SHA-256 mismatch')
    target = repo.root / 'research' / name
    if (repo.root / 'research').is_symlink() or target.is_symlink():
        raise ValueError('Definition paths must not contain links')
    if target.exists():
        raise ValueError('Refusing to overwrite definition: ' + str(target))
    scratch = repo.state / 'imports'
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=scratch) as temporary:
        stage = Path(temporary) / 'definition'
        manifest = deploy(archive, stage)  # Runner validates paths and every envelope hash.
        if manifest['format'] != 'research-package/v2':
            raise ValueError('Migration requires a v2 native research package')
        if sha(archive) != expected_sha256:
            raise ValueError('Archive changed during import')
        (stage / 'package.json').unlink()
        (stage / 'outputs').rmdir()
        # Preserve the full old workspace layout: historical helpers derive ROOT
        # from __file__. Moving those files below inputs/ would break their paths.
        archive_manifest = json.dumps(manifest)
        original = json.loads(archive_manifest)
        manifest['name'] = name
        manifest['required_outputs'] = list(dict.fromkeys([
            *manifest['required_outputs'], 'outputs/research.json']))
        occupied = [p for p in stage.rglob('*') if p.is_file()]
        additions = ['inputs/contracts.json', 'inputs/question.md', 'preparation/runner-migration.json']
        if any((stage / p).exists() for p in additions):
            raise ValueError('Migration metadata would overwrite an existing input')
        write(stage / additions[0], schemas())
        (stage / additions[1]).write_text(
            '# Legacy continuation\n\nRead the retained brief and scoped target. Audit current project support and '
            'remaining evidence gaps before starting new discovery. Historical readiness and missing-input '
            'claims are provenance, not a current qualification. Do not repeat already-completed research.\n', encoding='utf-8')
        template_agents = (repo.root / 'research/ps1-summary/AGENTS.md').read_text(encoding='utf-8')
        # Preserve original instructions separately and give the new closeout
        # contract precedence without changing any scientific input or helper.
        history = stage / 'preparation/runner-migration-original'
        if history.exists():
            raise ValueError('Package was already migrated')
        history.mkdir(parents=True)
        for key in ('brief', 'agents'):
            source = stage / manifest[key]
            (history / f'{key}.md').write_bytes(source.read_bytes())
        (history / 'manifest.json').write_text(archive_manifest + '\n', encoding='utf-8')
        agents = stage / manifest['agents']
        agents.write_text(agents.read_text(encoding='utf-8') + '\n\n'
                         '# Runner 0.3 closeout contract (takes precedence)\n\n' + template_agents,
                         encoding='utf-8')
        brief = stage / manifest['brief']
        brief.write_text('# Migration review\n\nRead inputs/question.md first. Preserve the original workspace paths. '
                         'Historical instructions below are scoped by the current AGENTS.md and Runner deadline. '
                         'Also deliver outputs/research.json using inputs/contracts.json.\n\n' +
                         brief.read_text(encoding='utf-8'), encoding='utf-8')
        write(stage / additions[2], {
            'format': 'memcard-runner-migration/v1',
            'verified_utc': datetime.now(timezone.utc).isoformat(),
            'source_archive': str(archive), 'source_sha256': expected_sha256,
            'original_manifest': original,
            'preserved_files': {str(p.relative_to(stage)): sha(p) for p in occupied
                                if str(p.relative_to(stage)) not in (manifest['brief'], manifest['agents'])},
            'note': 'Byte verification only; runtime dependencies must be verified on executing host.',
        })
        declared = {manifest['brief'], manifest['agents'], *manifest['skills']}
        manifest['inputs'] = sorted(str(p.relative_to(stage)) for p in stage.rglob('*')
                                    if p.is_file() and p != stage / 'manifest.json'
                                    and str(p.relative_to(stage)) not in declared)
        write(stage / 'manifest.json', manifest)
        target.parent.mkdir(parents=True, exist_ok=True)
        stage.rename(target)
    # Packaging, deterministic cache and all future job identity/recovery belong to Runner.
    package = repo.package(name)
    write(repo.state / 'migrations' / f'{name}.json', {
        'definition': name, 'source_sha256': expected_sha256, 'package': package,
        'submitted': False, 'started': False,
    })
    return package


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--repo', type=Path, default=REPO)
    ap.add_argument('--name', required=True)
    ap.add_argument('--archive', type=Path, required=True)
    ap.add_argument('--sha256', required=True, help='Expected original archive hash from its preparation receipt')
    args = ap.parse_args()
    from runner import ResearchRepo
    print(json.dumps(import_package(ResearchRepo(args.repo), args.name, args.archive, args.sha256), indent=2))


if __name__ == '__main__':
    main()
