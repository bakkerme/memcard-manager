#!/usr/bin/env python3
"""Build a deterministic research package from a committed game spec.

The definition tree is rendered from tracked sources and the spec. Private BIOS
and save bytes are copied only when their sha256 matches the spec, and they are
never read from the git tree. Disc images stay runner asset dependencies. The
spec asset is the primary disc. extra_assets may list further discs, and each
one is included the same way. Their destinations must not overlap.
"""
import argparse
import hashlib
import hmac
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from packaging_contract import CONTRACT_SOURCE, copy_contract, relocate_contract_links

REPO = Path(__file__).resolve().parents[2]
ASSEMBLER_VERSION = '1'
PLACEHOLDER_SHA = '0' * 64
SPEC_FORMAT = 'memcard-research-spec/v1'
TOOL_PATHS = (
    'tools/ps1_replay/Containerfile',
    'tools/ps1_replay/build-lock.json',
    'tools/ps1_replay/host.py',
    'tools/ps1_replay/runner.py',
    'tools/ps1_replay/adapter.lua',
    'tools/ps1_replay/recipe.json',
    'tools/ps1_replay/review.py',
    'tools/ps1_replay/README.md',
    'tools/save_research/harness.py',
    'tools/save_research/parser_template.py',
    'tools/save_research/disc_id.py',
    # host.py bind-mounts this decoder on every launch. Without it, ensure
    # succeeds and bootstrap/calibrate/qualify fail on a missing source path.
    'tools/inspect_mml2.py',
)
SKILL_ROOTS = (
    '.agents/skills/ps1-save-research/',
    '.agents/skills/emulator-replay/',
)
READER_PREFIX = 'src-tauri/src/card/'
TEMPLATE_PATHS = (
    'research/templates/AGENTS.md',
    'research/templates/brief.md',
    'research/templates/question.md',
)
# Closeout paths remain a trio even when a scoped job is blocked. Publication
# layout and quotas live in research/output-packaging.md, copied into each job.
REQUIRED_OUTPUTS = [
    'outputs/handoff.md',
    'outputs/research.json',
    'outputs/result.json',
]
HEX64 = re.compile(r'^[0-9a-f]{64}$')
COMMIT_RE = re.compile(r'^[0-9a-f]{40}(?:[0-9a-f]{24})?$')
CHECK_ID = re.compile(r'^[a-z0-9][a-z0-9-]{0,99}$')
NAME_RE = re.compile(r'^[a-z][a-z0-9-]{0,63}$')


def git_env():
    env = os.environ.copy()
    env['LC_ALL'] = 'C'
    env['TZ'] = 'UTC'
    return env


def git(repo, *args, text=True):
    result = subprocess.run(['git', '-C', str(repo), *args], check=True,
                            capture_output=True, env=git_env())
    if text:
        return result.stdout.decode()
    return result.stdout


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def list_tracked(repo):
    raw = git(repo, 'ls-files', '-z', text=False)
    return [item.decode() for item in raw.split(b'\0') if item]


def safe_rel(value, label):
    if not isinstance(value, str) or not value or '\\' in value or value.startswith('/'):
        raise ValueError(f'{label} must be a relative posix path')
    if any(part in ('', '.', '..') for part in value.split('/')):
        raise ValueError(f'{label} must be a relative posix path')
    return value


def write_bytes(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    path.chmod(0o644)


def write_text(path, text):
    if '\r' in text:
        raise ValueError(f'refusing CR bytes in generated file {path.name}')
    if not text.endswith('\n'):
        text += '\n'
    write_bytes(path, text.encode('utf-8'))


def write_json(path, value):
    write_text(path, json.dumps(value, indent=2, sort_keys=True) + '\n')


def load_spec(path):
    path = Path(path)
    try:
        raw = path.read_bytes()
        spec = json.loads(raw.decode('utf-8'))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f'cannot read game spec {path}: {error}') from error
    if not isinstance(spec, dict) or spec.get('format') != SPEC_FORMAT:
        raise ValueError(f'spec format must be {SPEC_FORMAT}')
    if not isinstance(spec.get('name'), str) or not NAME_RE.fullmatch(spec['name']):
        raise ValueError('spec name must match [a-z][a-z0-9-]{0,63}')
    for key in ('serial', 'product_code', 'title', 'goal'):
        if not isinstance(spec.get(key), str) or not spec[key].strip():
            raise ValueError(f'spec {key} is required')
    asset = spec.get('asset')
    if not isinstance(asset, dict):
        raise ValueError('spec asset is required')
    for key in ('asset_type', 'asset_id', 'destination'):
        if not isinstance(asset.get(key), str) or not asset[key]:
            raise ValueError(f'spec asset.{key} is required')
    safe_rel(asset['destination'], 'asset destination')
    validate_extra_assets(spec)
    if not isinstance(spec.get('fields_target'), list) or not spec['fields_target']:
        raise ValueError('spec fields_target must be a non-empty list')
    cross = spec.get('cross_check')
    if not isinstance(cross, dict):
        raise ValueError('spec cross_check is required')
    for key in ('name', 'url', 'license'):
        if not isinstance(cross.get(key), str) or not cross[key].strip():
            raise ValueError(f'spec cross_check.{key} is required')
    if not str(cross['url']).startswith('https://'):
        raise ValueError('spec cross_check.url must be https')
    privates = spec.get('private_inputs')
    if not isinstance(privates, list) or not privates:
        raise ValueError('spec private_inputs must be a non-empty list')
    seen = set()
    for item in privates:
        if not isinstance(item, dict):
            raise ValueError('spec private input must be an object')
        for key in ('role', 'path', 'sha256', 'provenance'):
            if not isinstance(item.get(key), str) or not item[key]:
                raise ValueError(f'private input {key} is required')
        safe_rel(item['path'], f"private input {item['role']} path")
        if item['role'] in seen or item['path'] in seen:
            raise ValueError('duplicate private input role or path')
        seen.add(item['role'])
        seen.add(item['path'])
        if not HEX64.fullmatch(item['sha256']):
            raise ValueError(f"private input {item['role']} sha256 must be 64 lowercase hex digits")
        if item['sha256'] == PLACEHOLDER_SHA:
            raise ValueError(
                f"private input {item['role']} sha256 is still the placeholder; "
                'record the real digest in the spec before assembly')
    roles = {item['role'] for item in privates}
    if 'bios' not in roles:
        raise ValueError('spec private_inputs must include bios')
    resources = spec.get('resources')
    if not isinstance(resources, dict):
        raise ValueError('spec resources are required')
    for key in ('cpu', 'memory_mb', 'time_seconds'):
        if key not in resources:
            raise ValueError(f'spec resources.{key} is required')
    checks = spec.get('required_checks')
    if not isinstance(checks, list) or not checks:
        raise ValueError('spec required_checks must be a non-empty list')
    if len(checks) != len(set(checks)) or len(checks) > 50:
        raise ValueError('required_checks must be unique and at most 50')
    for check in checks:
        if not isinstance(check, str) or not CHECK_ID.fullmatch(check):
            raise ValueError(f'invalid required check id: {check!r}')
    policy = spec.get('policy') or {
        'finalization_seconds': 900, 'termination_seconds': 60, 'checkpoint_seconds': 30}
    if not isinstance(policy, dict):
        raise ValueError('spec policy must be an object')
    spec = dict(spec)
    spec['policy'] = {key: policy[key] for key in
                      ('finalization_seconds', 'termination_seconds', 'checkpoint_seconds')}
    return spec


def resolve_private(spec, bios, save):
    overrides = {}
    if bios:
        overrides['bios'] = Path(bios)
    if save:
        overrides['save'] = Path(save)
    resolved = []
    for item in spec['private_inputs']:
        role = item['role']
        source = overrides.get(role)
        if source is None:
            env = item.get('path_env')
            if isinstance(env, str) and os.environ.get(env):
                source = Path(os.environ[env])
        if source is None:
            hint = item.get('path_env') or f'--{role}'
            raise ValueError(f'private input {role} is missing; set {hint} or pass --{role}')
        source = Path(source)
        if not source.is_file():
            raise ValueError(f'private input {role} is missing: {source}')
        actual = sha256_file(source)
        if not hmac.compare_digest(actual, item['sha256']):
            raise ValueError(
                f"private input {role} sha256 mismatch: expected {item['sha256']} observed {actual}")
        resolved.append((item, source, actual))
    return resolved


def source_paths(repo):
    tracked = list_tracked(repo)
    tracked_set = set(tracked)
    missing = [path for path in TOOL_PATHS if path not in tracked_set]
    if missing:
        raise ValueError('required tool file is not tracked: ' + ', '.join(missing))
    missing = [path for path in (*TEMPLATE_PATHS, CONTRACT_SOURCE) if path not in tracked_set]
    if missing:
        raise ValueError('template is not tracked: ' + ', '.join(missing))
    selected = list(TOOL_PATHS)
    for path in tracked:
        if path.startswith(SKILL_ROOTS):
            selected.append(path)
        elif path.startswith(READER_PREFIX) and '/testdata/' not in path:
            # Reader unit-test saves stay in the repo (include_bytes!). A package
            # carries a save only when the spec declares that private input.
            selected.append(path)
    return selected


def destination_for(path):
    if path.startswith(READER_PREFIX):
        return 'inputs/current-reader/' + path[len(READER_PREFIX):]
    for root in SKILL_ROOTS:
        if path.startswith(root):
            return 'skills/' + path[len('.agents/skills/'):]
    return path


def worktree_status(repo):
    return git(repo, 'status', '--porcelain', '-uall')


def dirty_diff_hash(repo):
    diff = git(repo, 'diff', 'HEAD', '--binary', text=False)
    untracked = git(repo, 'ls-files', '--others', '--exclude-standard', '-z', text=False)
    parts = [diff]
    paths = sorted(item.decode() for item in untracked.split(b'\0') if item)
    for path in paths:
        parts.append(path.encode() + b'\0' + sha256_file(Path(repo) / path).encode() + b'\n')
    return sha256_bytes(b''.join(parts))


def render_text(template, values):
    text = template.replace('\r\n', '\n')
    for key, value in values.items():
        text = text.replace(f'@@{key}@@', value)
    leftover = sorted(set(re.findall(r'@@[A-Z0-9_]+@@', text)))
    if leftover:
        raise ValueError('unreplaced template tokens: ' + ', '.join(leftover))
    if not text.endswith('\n'):
        text += '\n'
    return text


def markdown_checks(spec):
    return '\n'.join(f'- `{check}`' for check in spec['required_checks'])


def markdown_fields(spec):
    lines = []
    for field in spec.get('fields') or []:
        if not isinstance(field, dict) or 'id' not in field:
            continue
        detail = ', '.join(f'{key} {field[key]}' for key in
                           ('offset', 'length', 'type', 'bit', 'note') if key in field)
        lines.append(f"- `{field['id']}`: {detail}" if detail else f"- `{field['id']}`")
        for flag in field.get('flags') or []:
            lines.append(f"  - {flag.get('label', 'flag')} at {flag.get('offset')} bit {flag.get('bit')}")
    for level in spec.get('levels') or []:
        if not isinstance(level, dict):
            continue
        unlock = level.get('unlock') or {}
        lines.append(
            f"- level {level.get('index')} {level.get('name')}: unlock {unlock.get('offset')} bit {unlock.get('bit')}")
        for cage in level.get('cages') or []:
            lines.append(f"  - cage at {cage.get('offset')} bit {cage.get('bit')}")
    if not lines:
        return 'The spec did not include an editor field table.\n'
    return '\n'.join(lines) + '\n'


def ensure_section(fmt):
    if fmt == 'v2':
        return (
            '## Step 0 — provision the emulator image\n\n'
            'This is the first required step. Run it before any emulator command:\n\n'
            '```sh\n'
            'python3 tools/ps1_replay/host.py --engine docker ensure\n'
            '```\n\n'
            '`ensure` builds the image only when the tag is missing or its '
            '`research.input_hash` label does not match. Save the printed JSON, '
            'including `imageId` and `action`, as `outputs/tool-image.json` and cite '
            'it as evidence for `tool-image-provisioned`. Runner 0.4.0 does not '
            'provision the image itself; this step is how the package builds its container.\n'
        )
    return (
        'The runner provisions this image before the first turn. Read '
        '`.runner/tool-images.json` and record its image ID as the evidence for '
        '`tool-image-provisioned`. Do not rebuild the image in the worker. '
        'Pass that tag to every emulator command, for example '
        '`python3 tools/ps1_replay/host.py --engine docker --image-tag <tag> ...`; '
        'the v3 tag pins the whole tools/ps1_replay context, so host.py\'s built-in '
        'default tag differs.\n'
    )


def template_values(spec, image, fmt):
    cross = spec['cross_check']
    save = next((item for item in spec['private_inputs'] if item['role'] == 'save'), None)
    bios = next(item for item in spec['private_inputs'] if item['role'] == 'bios')
    if save is None:
        save = {'path': '(none supplied)', 'sha256': 'n/a',
                'provenance': spec.get('no_save_note', 'No reference save is packaged. Generate saves in the emulator by playing from a new game and saving to the virtual memory card at several progress points.')}
    return {
        'TITLE': spec['title'],
        'NAME': spec['name'],
        'SERIAL': spec['serial'],
        'PRODUCT_CODE': spec['product_code'],
        'GOAL': spec['goal'],
        'INPUT_HASH': image['input_hash'],
        'TAG': image['tag'],
        'BUILD_ID': str(image['build_id']),
        'ZIP_SHA': image['zip_sha256'],
        'CHECKS': markdown_checks(spec),
        'ENSURE': ensure_section(fmt),
        'FIELD_MAP': markdown_fields(spec),
        'FIELDS': ', '.join(spec['fields_target']),
        'SAVE_PATH': save['path'],
        'SAVE_SHA': save['sha256'],
        'SAVE_PROVENANCE': save['provenance'],
        'BIOS_PATH': bios['path'],
        'BIOS_SHA': bios['sha256'],
        'BIOS_PROVENANCE': bios['provenance'],
        'DISC_ID': spec['asset']['asset_id'],
        'DISC_DEST': spec['asset']['destination'],
        'DISC_PROVENANCE': spec['asset'].get('provenance', ''),
        'CROSS_NAME': cross['name'],
        'CROSS_URL': cross['url'],
        'CROSS_LICENSE': cross['license'],
        'CROSS_COMMIT': cross.get('commit', ''),
        'CROSS_NOTE': cross.get('note', ''),
        'EXTRA_DISCS': extra_dependency_note(spec),
    }


def tool_image_declaration(stage, fmt='v2'):
    replay = stage / 'tools' / 'ps1_replay'
    host = load_replay_host()
    digest = host.tool_input_hash(replay)
    lock = host.load_lock(replay)
    inputs = {rel: host.sha256_file(replay / Path(rel).name) for rel in host.TOOL_IMAGE_INPUTS}
    if fmt == 'v3':
        # Runner 0.5.0 requires every package file inside the context to be pinned,
        # and derives the tag from those pins, so hash the whole staged context.
        inputs = {path.relative_to(stage).as_posix(): host.sha256_file(path)
                  for path in sorted(replay.rglob('*')) if path.is_file()}
    entry = {
        'name': 'ps1-replay',
        'containerfile': 'tools/ps1_replay/Containerfile',
        'context': 'tools/ps1_replay',
        'platform': 'linux/arm64',
        'inputs': inputs,
        'fetch': [{
            'url_from': 'tools/ps1_replay/build-lock.json#url',
            'sha256': lock['sha256'],
            'dest': '.cache/pcsx-redux.zip',
        }],
        'tag': host.derived_tag(digest),
        'input_hash': digest,
        'timeout_seconds': 1800,
        'build_id': lock['buildId'],
        'zip_sha256': lock['sha256'],
    }
    if fmt == 'v3':
        apply_runner_tool_image_identity(entry)
    return entry


def canonical_tool_image_hash(entry):
    """Same material Runner 0.5.0 hashes in runner.tool_images.input_hash.

    Research Runner 0.4.0 has no tool_images module. CI still assembles v3
    packages with that wheel, so this copy keeps the tag identical to 0.5.0.
    """
    material = {
        'containerfile': entry['containerfile'],
        'context': entry['context'],
        'platform': entry['platform'],
        'inputs': dict(entry['inputs']),
    }
    fetch = entry.get('fetch') or []
    if fetch:
        material['fetch'] = [item['sha256'] for item in fetch]
    encoded = json.dumps(material, sort_keys=True, separators=(',', ':'), ensure_ascii=True)
    return hashlib.sha256(encoded.encode('ascii')).hexdigest()


def apply_runner_tool_image_identity(entry):
    tool_images = load_research_runner_tool_images()
    if tool_images is not None:
        entry['input_hash'] = tool_images.input_hash(entry)
        entry['tag'] = tool_images.derived_tag(entry)
        return
    digest = canonical_tool_image_hash(entry)
    entry['input_hash'] = digest
    entry['tag'] = f"localhost/rr-tool/{entry['name']}:in-{digest[:16]}"


def load_research_runner_tool_images():
    load_research_runner()
    import importlib
    try:
        return importlib.import_module('runner.tool_images')
    except ImportError:
        return None


def _resolved(entry):
    try:
        return Path(entry).resolve()
    except OSError:
        return None


def load_research_runner():
    """Import the installed Runner package even if tools/ps1_replay is on sys.path.

    That directory contains runner.py, the emulator coordinator, which would
    otherwise shadow research-runner.
    """
    current = sys.modules.get('runner')
    current_file = str(getattr(current, '__file__', '') or '')
    if current is not None and 'site-packages' in current_file and hasattr(current, 'protocol'):
        return current
    for name in list(sys.modules):
        if name == 'runner' or name.startswith('runner.'):
            del sys.modules[name]
    replay = (REPO / 'tools' / 'ps1_replay').resolve()
    saved = sys.path[:]
    sys.path[:] = [entry for entry in saved if _resolved(entry) != replay]
    try:
        import runner.package
        import runner.protocol
    except ImportError as error:
        raise ValueError('research-runner is not installed; install requirements-research.txt') from error
    finally:
        sys.path[:] = saved
    return sys.modules['runner']


def load_replay_host():
    module = sys.modules.get('ps1_replay_host')
    if module is not None:
        return module
    path = REPO / 'tools' / 'ps1_replay' / 'host.py'
    spec = importlib.util.spec_from_file_location('ps1_replay_host', path)
    module = importlib.util.module_from_spec(spec)
    sys.modules['ps1_replay_host'] = module
    spec.loader.exec_module(module)
    return module


def contracts():
    return load_research_runner().protocol.schemas()


def validate_runner_pieces(manifest):
    protocol = load_research_runner().protocol
    protocol.Policy.model_validate(manifest['policy'])
    for dependency in manifest.get('dependencies', []):
        protocol.AssetDependency.model_validate(dependency)


def seal(stage, dest, manifest):
    package = load_research_runner().package
    assemble, validate_manifest = package.assemble, package.validate_manifest
    try:
        validate_manifest(json.loads(json.dumps(manifest)))
    except ValueError as error:
        if manifest.get('format') != 'research-package/v3' or 'unsupported package format' not in str(error):
            raise
        return seal_unverified(stage, dest, manifest)
    return assemble(stage, dest)


def seal_unverified(stage, dest, manifest):
    """Same gzip/tar layout as Runner 0.4.0 assemble(), which rejects format v3."""
    import io
    import tarfile
    import gzip
    package = load_research_runner().package
    declared, hash_file, relpath = package.declared, package.hash_file, package.relpath
    for name in declared(manifest):
        relpath(name)
    expected = set(declared(manifest)) | {'manifest.json'}
    actual = set()
    for path in stage.rglob('*'):
        relative = path.relative_to(stage).as_posix()
        if path.is_symlink():
            raise ValueError('links forbidden')
        if path.is_file():
            actual.add(relative)
        elif not path.is_dir():
            raise ValueError('special input forbidden')
    if actual != expected:
        raise ValueError(f'undeclared or missing payloads: {sorted(actual ^ expected)}')
    files = {}
    for name in sorted(declared(manifest)):
        files[name] = {'size': (stage / name).stat().st_size, 'sha256': hash_file(stage / name)}
    envelope = {'manifest': manifest, 'sha256': {name: meta['sha256'] for name, meta in files.items()}}
    data = json.dumps(envelope, sort_keys=True, separators=(',', ':')).encode()
    dest = Path(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    with dest.open('wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', filename='', mtime=0, compresslevel=6) as gz, \
            tarfile.open(fileobj=gz, mode='w|') as archive:
        members = [('package.json', data)]
        members += [(name, None) for name in sorted(files)]
        for name, payload in members:
            info = tarfile.TarInfo(name)
            info.mode = 0o644
            info.uid = 0
            info.gid = 0
            info.mtime = 0
            if name == 'package.json':
                info.size = len(payload)
                archive.addfile(info, io.BytesIO(payload))
            else:
                info.size = files[name]['size']
                with (stage / name).open('rb') as source:
                    archive.addfile(info, source)
    return hash_file(dest)


def definition_hash(stage):
    inventory = {}
    for path in sorted(stage.rglob('*')):
        if path.is_file():
            inventory[path.relative_to(stage).as_posix()] = sha256_file(path)
    canonical = json.dumps(inventory, sort_keys=True, separators=(',', ':')).encode()
    return sha256_bytes(canonical)


def overlaps(path, destination):
    return path == destination or path.startswith(destination + '/') or destination.startswith(path + '/')


def validate_extra_assets(spec):
    """Accept an optional extra_assets list and reject overlapping destinations.

    Each entry is another runner dependency, same fields as the primary asset.
    Provenance stays on the spec and is not part of the dependency object.
    """
    if 'extra_assets' not in spec:
        return
    extras = spec['extra_assets']
    if not isinstance(extras, list):
        raise ValueError('spec extra_assets must be a list')
    destinations = [spec['asset']['destination']]
    for index, item in enumerate(extras):
        label = f'spec extra_assets[{index}]'
        if not isinstance(item, dict):
            raise ValueError(f'{label} must be an object')
        for key in ('asset_type', 'asset_id', 'destination'):
            if not isinstance(item.get(key), str) or not item[key]:
                raise ValueError(f'{label}.{key} is required')
        safe_rel(item['destination'], f'{label} destination')
        if 'provenance' in item and not isinstance(item['provenance'], str):
            raise ValueError(f'{label}.provenance must be a string')
        for previous in destinations:
            if overlaps(previous, item['destination']):
                raise ValueError(
                    'asset destination overlaps another dependency: '
                    f'{item["destination"]}')
        destinations.append(item['destination'])


def dependency_entries(spec):
    items = [spec['asset'], *(spec.get('extra_assets') or [])]
    return [{key: item[key] for key in ('asset_type', 'asset_id', 'destination')}
            for item in items]


def extra_dependency_note(spec):
    """Question text for discs after the primary asset. Empty when there are none."""
    lines = []
    for asset in spec.get('extra_assets') or []:
        line = (
            f"- Additional dependency: `{asset['asset_type']}` `{asset['asset_id']}`, "
            f"materialized at `{asset['destination']}` before the first turn."
        )
        provenance = asset.get('provenance') or ''
        if provenance:
            line += ' ' + provenance
        lines.append(line)
    if not lines:
        return ''
    return '\n' + '\n'.join(lines)


def assemble_package(spec_path, out, fmt='v2', bios=None, save=None, repo=None, allow_dirty=False):
    if fmt not in ('v2', 'v3'):
        raise ValueError('format must be v2 or v3')
    repo = Path(repo or REPO).resolve()
    spec_path = Path(spec_path).resolve()
    out = Path(out).resolve()
    spec = load_spec(spec_path)
    spec_bytes = spec_path.read_bytes()
    privates = resolve_private(spec, bios, save)
    status = worktree_status(repo)
    if status.strip() and not allow_dirty:
        raise ValueError('refusing to assemble from a dirty worktree; commit the changes or pass --allow-dirty')
    commit = git(repo, 'rev-parse', 'HEAD').strip()
    if not COMMIT_RE.fullmatch(commit):
        raise ValueError('memcard-manager commit is not a git object id')
    provenance = {
        'assembler_version': ASSEMBLER_VERSION,
        'memcard_manager_commit': commit,
        'spec_sha256': sha256_bytes(spec_bytes),
    }
    if status.strip():
        provenance['dirty_diff_sha256'] = dirty_diff_hash(repo)

    import tempfile
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='memcard-package-') as temporary:
        stage = Path(temporary) / 'definition'
        stage.mkdir()
        for relative in source_paths(repo):
            source = repo / relative
            if source.is_symlink() or not source.is_file():
                raise ValueError(f'refusing to package non-regular file: {relative}')
            destination = destination_for(relative)
            data = source.read_bytes()
            if source.suffix == '.md':
                data = relocate_contract_links(data.decode('utf-8'), destination).encode('utf-8')
            write_bytes(stage / destination, data)
        copy_contract(stage, repo)
        for item, source, _digest in privates:
            target = stage / item['path']
            if target.exists():
                raise ValueError(f'private input destination collides with a packaged file: {item["path"]}')
            write_bytes(target, source.read_bytes())
            if sha256_file(target) != item['sha256']:
                raise ValueError(f"private input {item['role']} changed while copying")

        image = tool_image_declaration(stage, fmt)
        values = template_values(spec, image, fmt)
        template_root = repo / 'research' / 'templates'
        for name in ('AGENTS.md', 'brief.md'):
            text = render_text((template_root / name).read_text(encoding='utf-8'), values)
            write_text(stage / name, relocate_contract_links(text, name))
        question = render_text((template_root / 'question.md').read_text(encoding='utf-8'), values)
        extra = spec.get('question_extra')
        if extra:
            if isinstance(extra, list):
                extra = '\n'.join(extra)
            if not isinstance(extra, str):
                raise ValueError('spec question_extra must be a string or list of strings')
            question = question.rstrip('\n') + '\n\n' + extra.rstrip('\n') + '\n'
        write_text(stage / 'inputs' / 'question.md', relocate_contract_links(question, 'inputs/question.md'))
        write_json(stage / 'inputs' / 'contracts.json', contracts())
        write_json(stage / 'inputs' / 'spec.json', spec)
        write_json(stage / 'inputs' / 'provenance.json', provenance)
        declaration = {key: image[key] for key in (
            'name', 'containerfile', 'context', 'platform', 'inputs', 'fetch', 'tag',
            'timeout_seconds')}
        write_json(stage / 'inputs' / 'tool-images.json',
                   {'format': 'memcard-tool-images/v1',
                    'images': [dict(declaration, input_hash=image['input_hash'])]})

        dependencies = dependency_entries(spec)
        skills = sorted(path.relative_to(stage).as_posix()
                        for path in (stage / 'skills').rglob('SKILL.md'))
        if not skills:
            raise ValueError('no SKILL.md files were packaged')
        reserved = {'AGENTS.md', 'brief.md', *skills}
        inputs = sorted(path.relative_to(stage).as_posix()
                        for path in stage.rglob('*')
                        if path.is_file() and path.relative_to(stage).as_posix() not in reserved)
        # Same overlap rule Runner uses at seal: packaged files, reserved names,
        # and the other dependency destinations.
        occupied = inputs + skills + ['AGENTS.md', 'brief.md', 'package.json', 'manifest.json', 'outputs']
        for dependency in dependencies:
            dest = dependency['destination']
            for path in occupied:
                if overlaps(path, dest):
                    raise ValueError(
                        f'packaged path overlaps dependency destination {dest}: {path}')
            occupied.append(dest)
        manifest = {
            'format': f'research-package/{fmt}',
            'name': spec['name'],
            'adapter': 'codex',
            'runtime': {'sdk': 'openai-codex==0.159.0'},
            'brief': 'brief.md',
            'agents': 'AGENTS.md',
            'skills': skills,
            'inputs': inputs,
            'dependencies': dependencies,
            'resources': {key: spec['resources'][key] for key in ('cpu', 'memory_mb', 'time_seconds')},
            'required_outputs': list(REQUIRED_OUTPUTS),
            'policy': spec['policy'],
        }
        if fmt == 'v3':
            manifest['tool_images'] = [declaration]
            manifest['required_checks'] = list(spec['required_checks'])
            manifest['provenance'] = provenance
        if 'image_id' in json.dumps(declaration) or 'image_id' in manifest.get('runtime', {}):
            raise ValueError('tool image declaration must not pin image_id')
        validate_runner_pieces(manifest)
        write_json(stage / 'manifest.json', manifest)
        fingerprint = definition_hash(stage)
        fd, temporary_archive = tempfile.mkstemp(dir=out, suffix='.rrpkg.tmp')
        os.close(fd)
        temporary_archive = Path(temporary_archive)
        try:
            archive_sha = seal(stage, temporary_archive, manifest)
            final = out / f"{spec['name']}-{archive_sha}.rrpkg"
            os.replace(temporary_archive, final)
        finally:
            temporary_archive.unlink(missing_ok=True)
    return {'package': str(final), 'sha256': archive_sha, 'definition_sha256': fingerprint}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--spec', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--format', choices=('v2', 'v3'), default='v2')
    parser.add_argument('--bios', type=Path)
    parser.add_argument('--save', type=Path)
    parser.add_argument('--allow-dirty', action='store_true')
    parser.add_argument('--repo', type=Path, default=REPO)
    args = parser.parse_args(argv)
    try:
        result = assemble_package(args.spec, args.out, fmt=args.format, bios=args.bios,
                                  save=args.save, repo=args.repo, allow_dirty=args.allow_dirty)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
