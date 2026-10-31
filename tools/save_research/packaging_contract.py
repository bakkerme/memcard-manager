#!/usr/bin/env python3
"""Install the authoritative packaging contract in a manual Runner definition.

With no arguments, refresh the checked-in ps1-summary starter. --check validates
the generated copy, manifest declaration and instruction links without writing.
"""
import argparse
import json
from pathlib import Path
import posixpath
import re

REPO = Path(__file__).resolve().parents[2]
CONTRACT_SOURCE = 'research/output-packaging.md'
CONTRACT_INPUT = 'inputs/output-packaging.md'
CONTRACT_LINK = re.compile(r'(\[[^\]]+\]\()[^\s()]*output-packaging\.md(\))')


def contract_link(destination):
    return posixpath.relpath(CONTRACT_INPUT, posixpath.dirname(destination) or '.')


def relocate_contract_links(text, destination):
    """Keep packaging links relative to the file's location in the job tree."""
    return CONTRACT_LINK.sub(lambda match: match[1] + contract_link(destination) + match[2], text)


def copy_contract(definition, repo=REPO):
    source = Path(repo) / CONTRACT_SOURCE
    if source.is_symlink() or not source.is_file():
        raise ValueError(f'packaging contract must be a regular file: {CONTRACT_SOURCE}')
    target = Path(definition) / CONTRACT_INPUT
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(source.read_bytes())
    target.chmod(0o644)


def sync_definition(definition, repo=REPO, check=False):
    definition = Path(definition).resolve()
    manifest_path = definition / 'manifest.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    expected = (Path(repo) / CONTRACT_SOURCE).read_bytes()
    updates = {CONTRACT_INPUT: expected}
    if CONTRACT_INPUT not in manifest['inputs']:
        manifest['inputs'].append(CONTRACT_INPUT)
        updates['manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    for key in ('agents', 'brief'):
        name = manifest[key]
        path = definition / name
        if not path.resolve().is_relative_to(definition):
            raise ValueError(f'instruction path escapes definition: {name}')
        text = path.read_text(encoding='utf-8')
        linked = relocate_contract_links(text, name)
        if not CONTRACT_LINK.search(linked):
            linked = text.rstrip() + f'\n\nFollow the [output-packaging contract]({contract_link(name)}).\n'
        updates[name] = linked.encode('utf-8')
    stale = []
    for name, data in updates.items():
        path = definition / name
        if any(p.is_symlink() for p in (path, *path.parents) if p.is_relative_to(definition)):
            raise ValueError(f'links forbidden in definition: {name}')
        if not path.is_file() or path.read_bytes() != data:
            stale.append(name)
    if stale and check:
        raise ValueError('packaging contract needs refresh: ' + ', '.join(stale))
    for name in stale:
        path = definition / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(updates[name])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--definition', type=Path, default=REPO / 'research/ps1-summary')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args(argv)
    try:
        sync_definition(args.definition, check=args.check)
    except (OSError, ValueError, KeyError) as error:
        parser.exit(2, str(error) + '\n')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
