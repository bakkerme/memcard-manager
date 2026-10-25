#!/usr/bin/env python3
"""Build/start a Docker or Podman worker; experiment coordination stays in Linux."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
def image_name():
    return 'localhost/ps1-replay:' + str(json.loads((ROOT / 'build-lock.json').read_text())['buildId'])


def call(argv, **kwargs):
    return subprocess.run(argv, check=True, **kwargs)


def download():
    lock = json.loads((ROOT / 'build-lock.json').read_text())
    cache = ROOT / f".cache/pcsx-redux-{lock['buildId']}.zip"
    cache.parent.mkdir(exist_ok=True)
    if not cache.exists():
        temporary = cache.with_suffix('.partial')
        try:
            with urllib.request.urlopen(lock['url'], timeout=60) as source, temporary.open('wb') as dest:
                while chunk := source.read(1024 * 1024):
                    dest.write(chunk)
            if hashlib.sha256(temporary.read_bytes()).hexdigest() != lock['sha256']:
                raise RuntimeError('Official artifact checksum mismatch')
            temporary.replace(cache)
        finally:
            temporary.unlink(missing_ok=True)
    if hashlib.sha256(cache.read_bytes()).hexdigest() != lock['sha256']:
        raise RuntimeError('Cached emulator checksum mismatch')
    # The build context has one canonical artifact path; the versioned cache is retained.
    import shutil
    shutil.copyfile(cache, ROOT / '.cache/pcsx-redux.zip')


def image_info(engine):
    return json.loads(call([engine, 'image', 'inspect', image_name()], capture_output=True, text=True).stdout)[0]


def image_metadata(info, engine):
    # Docker local builds have no Digest key and may have no registry digest.
    digests = info.get('RepoDigests') or []
    return {'engine': engine, 'imageId': info['Id'],
            'digest': info.get('Digest') or (digests[0] if digests else None),
            'architecture': info['Architecture']}


def preflight(engine):
    template = '{{.Architecture}}' if engine == 'docker' else '{{.Host.Arch}}'
    arch = call([engine, 'info', '--format', template], capture_output=True,
                text=True, timeout=30).stdout.strip().lower()
    if arch not in ('arm64', 'aarch64'):
        raise RuntimeError(f'{engine} engine reports {arch!r}; this pinned worker requires native Linux ARM64')
    return {'engine': engine, 'architecture': arch}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', choices=('podman', 'docker'), default='podman',
                        help='Container CLI (default: podman); place before the subcommand')
    sub = parser.add_subparsers(dest='mode', required=True)
    sub.add_parser('preflight')
    sub.add_parser('build')
    for mode in ('calibrate', 'bootstrap', 'qualify', 'diagnose'):
        p = sub.add_parser(mode)
        p.add_argument('--run', required=True, type=Path)
        p.add_argument('--disc', type=Path, required=True, help='Explicit complete cue/bin directory')
        p.add_argument('--bios', type=Path, required=True, help='Explicit BIOS file visible to the engine')
        p.add_argument('--card', type=Path, required=True, help='Explicit whole-card seed file')
        p.add_argument('--name', default='ps1-replay')
        if mode == 'diagnose':
            p.add_argument('--state', required=True, type=Path)
    p = sub.add_parser('command')
    p.add_argument('--name', default='ps1-replay')
    p.add_argument('arguments', nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if args.mode == 'command':
        call([args.engine, 'exec', args.name, 'python3', '/tools/ps1_replay/runner.py', 'command', *args.arguments])
        return
    environment = preflight(args.engine)
    if args.mode == 'preflight':
        print(json.dumps(environment, indent=2))
        return
    if args.mode == 'build':
        download()
        call([args.engine, 'build', '--platform', 'linux/arm64', '-t', image_name(),
              '-f', str(ROOT / 'Containerfile'), str(ROOT)])
        info = image_info(args.engine)
        if info['Architecture'] != 'arm64':
            parser.error('Built worker image must be ARM64')
        print(json.dumps(image_metadata(info, args.engine), indent=2))
        return
    for path in (args.disc, args.bios, args.card):
        if not path.exists():
            parser.error(f'Missing input: {path}')
    if not args.disc.is_dir() or not args.bios.is_file() or not args.card.is_file():
        parser.error('--disc must be a directory; --bios and --card must be files')
    if args.mode == 'diagnose' and not args.state.is_file():
        parser.error(f'Missing diagnostic state: {args.state}')
    if args.mode == 'qualify':
        if not (args.run / 'baseline/manifest.json').is_file():
            parser.error('Qualification requires an existing run baseline')
        if any((args.run / f'trial-{n}').exists() for n in range(1, 4)):
            parser.error('Trials already exist; use a new run directory with a copied baseline')
    else:
        if args.run.exists() and any(args.run.iterdir()):
            parser.error('Calibration/bootstrap requires a new empty run directory')
    info = image_info(args.engine)
    if info['Architecture'] != 'arm64':
        parser.error('Worker image must be native ARM64')
    args.run.mkdir(parents=True, exist_ok=True)
    metadata = {**image_metadata(info, args.engine),
                'cpuLimit': 4, 'memoryLimitBytes': 3 * 1024**3, 'network': 'none'}
    (args.run / 'container.json').write_text(json.dumps(metadata, indent=2) + '\n')
    mounts = [(ROOT / name, '/tools/ps1_replay/' + name, 'ro') for name in
              ('runner.py', 'adapter.lua', 'recipe.json', 'build-lock.json')]
    mounts += [(REPO / 'tools/inspect_mml2.py', '/tools/inspect_mml2.py', 'ro'),
              (args.disc, '/inputs/disc', 'ro'),
              (args.bios, '/inputs/bios.bin', 'ro'), (args.card, '/inputs/seed.mcr', 'ro'),
              (args.run, '/work', 'rw')]
    if args.mode == 'diagnose':
        mounts.append((args.state, '/inputs/diagnostic.state', 'ro'))
    # A Runner hard kill (SIGKILL to the worker process group) kills this CLI but
    # not the container. Foreground `run --rm` never gets to remove it, so a fixed
    # --name then fails every later run with "name already in use". Remove only a
    # stale container this tool labelled.
    try:
        stale = subprocess.run([args.engine, 'ps', '-aq', '--filter', 'name=^' + args.name + '$',
                                '--filter', 'label=memcard.ps1-replay=1'], capture_output=True, text=True, timeout=30)
        for cid in stale.stdout.split():
            subprocess.run([args.engine, 'rm', '-f', cid], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        pass  # best effort; `run` reports a real conflict
    argv = [args.engine, 'run', '--rm', '--label', 'memcard.ps1-replay=1', '--platform', 'linux/arm64', '--name', args.name, '--cpus', '4', '--memory', '3g',
            '--network', 'none', '--entrypoint', 'python3']
    for path, dest, mode in mounts:
        argv += ['-v', f'{path.resolve()}:{dest}:{mode}']
    argv += [info['Id'], '/tools/ps1_replay/runner.py', args.mode]
    call(argv)


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
