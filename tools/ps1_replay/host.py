#!/usr/bin/env python3
"""Build/start a Docker or Podman worker; experiment coordination stays in Linux."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
# Files whose bytes define the tool image. Keys are package-relative so the hash
# matches the research-package tool_images declaration from any checkout path.
TOOL_IMAGE_INPUTS = (
    'tools/ps1_replay/Containerfile',
    'tools/ps1_replay/build-lock.json',
    'tools/ps1_replay/runner.py',
    'tools/ps1_replay/adapter.lua',
)
TOOL_IMAGE_PLATFORM = 'linux/arm64'


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def load_lock(root=None):
    root = Path(root or ROOT)
    return json.loads((root / 'build-lock.json').read_text())


def tool_input_hash(root=None):
    """sha256 of the canonical tool-image identity. Sorted keys, no host paths."""
    root = Path(root or ROOT)
    inputs = {rel: sha256_file(root / Path(rel).name) for rel in TOOL_IMAGE_INPUTS}
    payload = {
        'containerfile': 'tools/ps1_replay/Containerfile',
        'context': 'tools/ps1_replay',
        'platform': TOOL_IMAGE_PLATFORM,
        'inputs': inputs,
        'fetch': [load_lock(root)['sha256']],
    }
    canonical = json.dumps(payload, sort_keys=True, separators=(',', ':')).encode()
    return hashlib.sha256(canonical).hexdigest()


def derived_tag(digest=None, root=None):
    digest = digest or tool_input_hash(root)
    return 'localhost/rr-tool/ps1-replay:in-' + digest[:16]


def legacy_image_name(root=None):
    """Alias kept so existing localhost/ps1-replay:<buildId> references still resolve."""
    return 'localhost/ps1-replay:' + str(load_lock(root)['buildId'])


def image_name():
    return derived_tag()


def active_tag(override=None):
    return override or image_name()


def call(argv, **kwargs):
    return subprocess.run(argv, check=True, **kwargs)


# runner.py reads these when the container process starts. Docker and Podman do
# not copy the host environment into `run` or `exec`.
SESSION_ENV_KEYS = ('PS1_REPLAY_BASELINE_SECONDS', 'PS1_REPLAY_SESSION_SECONDS')


def session_env_flags():
    flags = []
    for key in SESSION_ENV_KEYS:
        if key not in os.environ:
            continue
        raw = os.environ[key].strip()
        try:
            seconds = float(raw)
        except ValueError:
            seconds = float('nan')
        if not math.isfinite(seconds) or seconds <= 0:
            raise RuntimeError(f'{key} must be a positive number of seconds')
        flags += ['-e', f'{key}={raw}']
    return flags


def download():
    lock = json.loads((ROOT / 'build-lock.json').read_text())
    cache = ROOT / f".cache/pcsx-redux-{lock['buildId']}.zip"
    cache.parent.mkdir(exist_ok=True)
    if not cache.exists():
        temporary = cache.with_suffix('.partial')
        try:
            with urllib.request.urlopen(urllib.request.Request(lock['url'], headers={'User-Agent': 'research-runner/ps1-replay'}), timeout=60) as source, temporary.open('wb') as dest:
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


def image_label(info):
    labels = {}
    config = info.get('Config')
    if isinstance(config, dict):
        labels.update(config.get('Labels') or {})
    top = info.get('Labels')
    if isinstance(top, dict):
        labels.update(top)
    return labels.get('research.input_hash')


def image_info(engine, tag=None):
    """Return the inspect object, or None when the tag is absent. Never raises IndexError."""
    tag = tag or image_name()
    try:
        result = call([engine, 'image', 'inspect', tag], capture_output=True, text=True)
    except subprocess.CalledProcessError:
        return None
    try:
        data = json.loads(result.stdout or '[]')
    except json.JSONDecodeError as error:
        raise RuntimeError(f'worker image inspect returned invalid JSON: {error}') from error
    if not isinstance(data, list) or not data:
        return None
    return data[0]


def image_metadata(info, engine):
    # Docker local builds have no Digest key and may have no registry digest.
    digests = info.get('RepoDigests') or []
    return {'engine': engine, 'imageId': info['Id'],
            'digest': info.get('Digest') or (digests[0] if digests else None),
            'architecture': info['Architecture']}


def build_argv(engine, tag, digest):
    argv = [engine, 'build', '--platform', TOOL_IMAGE_PLATFORM,
            '--label', f'research.input_hash={digest}', '-t', tag]
    alias = legacy_image_name()
    if alias != tag:
        argv += ['-t', alias]
    argv += ['-f', str(ROOT / 'Containerfile'), str(ROOT)]
    return argv


def ensure_image(engine, tag_override=None):
    """Build the worker only when the input-hash tag is missing or stale. Idempotent."""
    digest = tool_input_hash()
    tag = tag_override or derived_tag(digest)
    info = image_info(engine, tag)
    if info and image_label(info) == digest:
        if info.get('Architecture') != 'arm64':
            raise RuntimeError('Worker image must be native ARM64')
        print(json.dumps({**image_metadata(info, engine), 'tag': tag,
                          'inputHash': digest, 'action': 'reused'}, indent=2))
        return
    download()
    call(build_argv(engine, tag, digest))
    info = image_info(engine, tag)
    if info is None:
        raise RuntimeError(f'worker image {tag} missing after build')
    if info.get('Architecture') != 'arm64':
        raise RuntimeError('Built worker image must be ARM64')
    if image_label(info) != digest:
        raise RuntimeError('Built worker image is missing the input-hash label')
    print(json.dumps({**image_metadata(info, engine), 'tag': tag,
                      'inputHash': digest, 'action': 'built'}, indent=2))


def preflight(engine, tag=None):
    template = '{{.Architecture}}' if engine == 'docker' else '{{.Host.Arch}}'
    arch = call([engine, 'info', '--format', template], capture_output=True,
                text=True, timeout=30).stdout.strip().lower()
    if arch not in ('arm64', 'aarch64'):
        raise RuntimeError(f'{engine} engine reports {arch!r}; this pinned worker requires native Linux ARM64')
    selected = active_tag(tag)
    return {'engine': engine, 'architecture': arch,
            'image': selected, 'image_present': image_info(engine, selected) is not None}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', choices=('podman', 'docker'), default='podman',
                        help='Container CLI (default: podman); place before the subcommand')
    parser.add_argument('--image-tag',
                        help='Worker tag to inspect and run. Defaults to the input-hash tag. '
                             'v3 jobs pass the tag from .runner/tool-images.json')
    sub = parser.add_subparsers(dest='mode', required=True)
    sub.add_parser('preflight')
    sub.add_parser('build')
    sub.add_parser('ensure')
    for mode in ('calibrate', 'bootstrap', 'qualify', 'diagnose'):
        p = sub.add_parser(mode)
        p.add_argument('--run', required=True, type=Path)
        p.add_argument('--disc', type=Path, required=True, help='Explicit complete cue/bin directory')
        p.add_argument('--bios', type=Path, required=True, help='Explicit BIOS file visible to the engine')
        p.add_argument('--card', type=Path, required=True, help='Explicit whole-card seed file')
        p.add_argument('--name', default='ps1-replay')
        if mode in ('calibrate', 'diagnose'):
            p.add_argument('--guest-writes', action='store_true',
                           help='Opt in to logged guest RAM pokes for this session')
        if mode == 'diagnose':
            p.add_argument('--state', required=True, type=Path)
    p = sub.add_parser('command')
    p.add_argument('--name', default='ps1-replay')
    p.add_argument('arguments', nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if args.mode == 'command':
        call([args.engine, 'exec', *session_env_flags(), args.name, 'python3',
              '/tools/ps1_replay/runner.py', 'command', *args.arguments])
        return
    if args.mode in ('build', 'ensure'):
        try:
            preflight(args.engine, args.image_tag)
            ensure_image(args.engine, args.image_tag)
        except (OSError, RuntimeError, subprocess.SubprocessError) as error:
            print(str(error), file=sys.stderr)
            raise SystemExit(2)
        return
    environment = preflight(args.engine, args.image_tag)
    if args.mode == 'preflight':
        print(json.dumps(environment, indent=2))
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
    tag = active_tag(args.image_tag)
    info = image_info(args.engine, tag)
    if info is None:
        parser.error(f'worker image {tag} missing; run host.py ensure')
    if info['Architecture'] != 'arm64':
        parser.error('Worker image must be native ARM64')
    args.run.mkdir(parents=True, exist_ok=True)
    metadata = {**image_metadata(info, args.engine),
                'cpuLimit': 4, 'memoryLimitBytes': 3 * 1024**3, 'network': 'none',
                'guestWrites': bool(getattr(args, 'guest_writes', False))}
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
    argv += session_env_flags()
    # Only the launch flag opts in. An ambient host variable must not enable writes.
    if getattr(args, 'guest_writes', False):
        argv += ['-e', 'PS1_REPLAY_GUEST_WRITES=1']
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
