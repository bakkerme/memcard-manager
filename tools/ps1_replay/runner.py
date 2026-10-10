#!/usr/bin/env python3
"""Single-worker Linux integration test. Standard library except PNG conversion.

The coordinator runs inside the worker container. Calibration commands use the
same adapter as qualification; no desktop keyboard automation is involved.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent
WORK = Path('/work')
URL = 'http://127.0.0.1:8080/api/v1/lua/'
EMU = '/opt/squashfs-root/AppRun'


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def save_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


class GateError(RuntimeError):
    pass


def record_failure(verdict, gate, error):
    # Cleanup/preservation checks must not erase the original failed gate.
    verdict.setdefault('failures', []).append({'gate': gate, 'error': error})
    if 'error' not in verdict:
        verdict['error'] = error
        verdict['gate'] = gate
    verdict['status'] = 'failed'


class Client:
    def __init__(self, log, process=None, deadline=None, request=None):
        self.log = Path(log)
        self.process = process
        self.deadline = deadline or time.monotonic() + 300
        self.request = request or self.http

    @staticmethod
    def http(endpoint, params=None):
        url = URL + endpoint
        if params:
            url += '?' + urllib.parse.urlencode(params)
        with urllib.request.urlopen(url, timeout=2) as r:
            return json.load(r)

    def check(self):
        if self.process is not None and self.process.poll() is not None:
            raise GateError(f'Emulator exited: {self.process.returncode}')
        if time.monotonic() >= self.deadline:
            raise GateError('Five-minute worker deadline exceeded')

    def ready(self):
        while True:
            self.check()
            try:
                if self.request('replay_status').get('ready'):
                    return
            except (OSError, ValueError):
                pass
            time.sleep(.05)

    def action(self, action, timeout=60, **params):
        self.check()
        started = time.monotonic()
        ident = uuid.uuid4().hex
        record = {'id': ident, 'action': action, **params}
        try:
            ack = self.request('replay_command', {'id': ident, 'action': action, **params})
            if ack.get('accepted') != ident:
                raise GateError(f'Action rejected: {ack}')
            if action == 'quit':
                record['result'] = {'scheduled': True}
                return record['result']
            while True:
                self.check()
                if time.monotonic() - started > timeout:
                    raise GateError(f'Action stalled: {action}')
                state = self.request('replay_status')
                if state.get('done') == ident:
                    result = state['result']
                    if result.get('error'):
                        raise GateError(result['error'])
                    if action in ('press', 'frames') and result.get('actualFrames') != int(params['frames']):
                        raise GateError(f'Frame boundary overshot: {result}')
                    record['result'] = result
                    return result
                time.sleep(.02)
        except Exception as e:
            record['error'] = str(e)
            self.release_safely()
            raise
        finally:
            record['elapsedSeconds'] = time.monotonic() - started
            with self.log.open('a') as f:
                f.write(json.dumps(record) + '\n')

    def release_safely(self):
        try:
            ident = uuid.uuid4().hex
            ack = self.request('replay_command', {'id': ident, 'action': 'release'})
            end = time.monotonic() + 2
            while time.monotonic() < end and ack.get('accepted') == ident:
                if self.request('replay_status').get('done') == ident:
                    return True
                time.sleep(.02)
        except Exception:
            pass
        return False


def convert_capture(directory, name):
    from PIL import Image
    meta = json.loads((directory / (name + '.json')).read_text())
    raw = (directory / (name + '.raw')).read_bytes()
    if meta['bpp'] == 16:
        # PS1 framebuffer words are RGB555, little endian.
        rgb = bytearray()
        for i in range(0, len(raw), 2):
            word = raw[i] | raw[i + 1] << 8
            rgb.extend(((word & 31) * 255 // 31, ((word >> 5) & 31) * 255 // 31,
                        ((word >> 10) & 31) * 255 // 31))
        raw = bytes(rgb)
    image = Image.frombytes('RGB', (meta['width'], meta['height']), raw)
    image.save(directory / (name + '.png'))
    return {**meta, 'uniformBlack': image.getbbox() is None,
            'rawSha256': sha(directory / (name + '.raw')),
            'pngSha256': sha(directory / (name + '.png'))}


def input_hashes():
    files = [Path('/inputs/bios.bin'), Path('/inputs/seed.mcr')]
    cues = list(Path('/inputs/disc').glob('*.cue'))
    if len(cues) != 1:
        raise GateError('Expected exactly one CUE in /inputs/disc')
    files.extend(sorted(Path('/inputs/disc').glob('*')))
    if any(not f.is_file() for f in files):
        raise GateError('Missing input asset')
    data = Path('/inputs/seed.mcr').read_bytes()
    if len(data) != 131072 or data[:2] != b'MC':
        raise GateError('Seed must be a raw 128-KiB PS1 card')
    # Verify DASH20 through its directory entry, not a payload byte search.
    for block in range(1, 16):
        entry = data[block * 128:(block + 1) * 128]
        if entry[0] == 0x51 and entry[10:30].split(b'\0')[0] == b'BASLUS-01140-DASH20':
            if int.from_bytes(entry[4:8], 'little') != 8192 or int.from_bytes(entry[8:10], 'little') != 0xffff:
                raise GateError('Expected one unlinked DASH20 block')
            sys.path.insert(0, str(ROOT.parent))
            from inspect_mml2 import inspect_save
            decoded = inspect_save(entry + data[block * 8192:(block + 1) * 8192])
            if not decoded['checksumOk']:
                raise GateError('Seed save checksum failure')
            save_json(WORK / 'seed-decoded.json', decoded)
            break
    else:
        raise GateError('Seed card has no supported DASH20 save')
    return {str(f): sha(f) for f in files}


def prepare_card(source, destination, initialize_scratch=False):
    data = Path(source).read_bytes()
    original = data
    if initialize_scratch:
        # Retail BIOS card detection probes sector 63 with a copy of sector 0.
        # Prepare this unused block-0 scratch frame before the measured run.
        # Refuse to overwrite any unfamiliar data; directory and saves are untouched.
        offset = 63 * 128
        scratch = data[offset:offset + 128]
        if len(data) != 131072 or data[:2] != b'MC' or scratch not in (bytes(128), data[:128]):
            raise GateError('Unexpected card scratch frame; refusing preparation')
        data = data[:offset] + data[:128] + data[offset + 128:]
    Path(destination).write_bytes(data)
    return {'sourceSha256': hashlib.sha256(original).hexdigest(),
            'initialSha256': hashlib.sha256(data).hexdigest(),
            'preparation': 'mirror-sector-0-to-scratch-sector-63' if initialize_scratch else 'exact-copy',
            'changedOffsets': [hex(i) for i, (a, b) in enumerate(zip(original, data)) if a != b],
            'saveBlocksUnchanged': original[8192:] == data[8192:]}


class Worker:
    def __init__(self, stage, baseline=False):
        self.directory = WORK / stage
        self.directory.mkdir(parents=True, exist_ok=False)
        self.stage = stage
        self.process = None
        self.log = None
        self.client = None
        self.baseline = baseline

    def __enter__(self):
        started = time.monotonic()
        try:
            self.log = (self.directory / 'emulator.log').open('wb')
            card = WORK / 'baseline/card.mcr' if self.baseline else Path('/inputs/seed.mcr')
            preparation = prepare_card(card, self.directory / 'card.mcr', not self.baseline)
            save_json(self.directory / 'card-initial.json', preparation)
            profile = self.directory / 'profile'
            profile.mkdir()
            save_json(profile / 'pcsx.json', {
                'emulator': {'AutoUpdate': False, 'ShownAutoUpdateConfig': True,
                             'LinearFiltering': False, 'HardwareRenderer': False, 'Dynarec': False,
                             'Debug': {'WebServer': True, 'WebServerPort': 8080}},
                'SPU': {'Backend': 'dummy', 'Mute': True},
                'gui': {'ShowMenu': False}})
            env = dict(os.environ, REPLAY_STAGE=self.stage)
            cue = next(Path('/inputs/disc').glob('*.cue'))
            args = [EMU, '-portable', str(profile), '-bios', '/inputs/bios.bin',
                    '-iso', str(cue), '-memcard1', str(self.directory / 'card.mcr'),
                    '-memcard2', str(self.directory / 'unused.mcr'), '-noupdate',
                    '-noshaders', '-fastboot', '-no-viewports', '-no-debugger', '-no-gdb',
                    '-no-pcdrv', '-webserver', '-webserver-port', '8080',
                    '-stdout', '-lua_stdout', '-dofile', str(ROOT / 'adapter.lua')]
            save_json(self.directory / 'launch.json', {'argv': args, 'environment': {
                k: env.get(k) for k in ('DISPLAY', 'LIBGL_ALWAYS_SOFTWARE', 'GALLIUM_DRIVER',
                                        'LP_NUM_THREADS', 'SDL_AUDIODRIVER')}})
            self.process = subprocess.Popen(args, env=env, stdout=self.log,
                                            stderr=subprocess.STDOUT, start_new_session=True)
            self.client = Client(self.directory / 'actions.jsonl', self.process, started + (300 if self.baseline else 900))
            self.client.ready()
            # The on-disk profile can still contain pre-CLI defaults until exit.
            # Inspect the live Lua settings rather than certifying that stale file.
            settings = self.client.action('settings')
            wanted = {'Dynarec': False, 'HardwareRenderer': False, 'AutoUpdate': False,
                      'LinearFiltering': False, 'FastBoot': True}
            if any(settings.get(k) != v for k, v in wanted.items()):
                raise GateError(f'Runtime emulator settings differ from qualification defaults: {settings}')
            if 'Audio: driver dummy' not in (self.directory / 'emulator.log').read_text(errors='replace'):
                raise GateError('Dummy audio backend was not initialized')
            save_json(self.directory / 'settings-verified.json', {
                'emulator': wanted, 'audioDriver': 'dummy', 'shadersDisabledByCLI': True})
            if self.baseline:
                # This build's restored guest kernel faults if loaded before BIOS
                # initialization. Warm every fresh process identically, then restore
                # the state/card pair and reset the adapter's relative frame count.
                for frames in (1, 2, 60, 600):
                    self.client.action('frames', frames=frames)
                self.capture('before-restore')
                self.client.action('restore')
            return self
        except Exception:
            self.close()
            raise

    def close(self):
        released = None
        if self.client:
            if self.process is not None and self.process.poll() is None:
                released = self.client.release_safely()
        if self.process and self.process.poll() is None:
            try:
                self.client.action('quit', timeout=2)
            except Exception:
                pass
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGTERM)
                try:
                    self.process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    os.killpg(self.process.pid, signal.SIGKILL)
                    self.process.wait()
        if self.log:
            self.log.close()
        if self.process:
            save_json(self.directory / 'process.json', {'exitCode': self.process.poll()})
            save_json(self.directory / 'cleanup.json', {
                'releaseAcknowledged': released,
                'emulatorStopped': self.process.poll() is not None,
                'exitCode': self.process.poll()})

    def __exit__(self, *_):
        self.close()

    def capture(self, name):
        self.client.action('capture', name=name)
        return convert_capture(self.directory, name)


def check_values(actual, expected):
    differences = {k: {'expected': v, 'actual': actual.get(k)} for k, v in expected.items()
                   if actual.get(k) != v}
    if differences:
        raise GateError(f'Known-save values differ: {differences}')


def run_recipe(worker, steps):
    for index, step in enumerate(steps):
        if (step.get('action') not in ('frames', 'press')
                or type(step.get('frames')) is not int or not 1 <= step['frames'] <= 36000
                or (step['action'] == 'press' and step.get('button') not in
                    ('SELECT', 'START', 'UP', 'RIGHT', 'DOWN', 'LEFT', 'L2', 'R2',
                     'L1', 'R1', 'TRIANGLE', 'CIRCLE', 'CROSS', 'SQUARE', 'L3', 'R3'))):
            raise GateError(f'Recipe step {index} must be bounded controller input or a frame wait')
        worker.client.action(**step)
        worker.capture(f'step-{index:02}')


def baseline(worker, expected):
    check_values(worker.client.action('values'), expected)
    worker.client.action('release')
    if worker.capture('gameplay')['uniformBlack']:
        raise GateError('Load-scene gate failed: gameplay framebuffer is uniformly black')
    worker.client.action('save', name='state.bin')
    dest = WORK / 'baseline'
    dest.mkdir(exist_ok=False)
    for name in ('state.bin', 'card.mcr', 'gameplay.raw', 'gameplay.json', 'gameplay.png'):
        shutil.copyfile(worker.directory / name, dest / name)
    save_json(dest / 'manifest.json', {'expected': expected,
        'frame': worker.client.action('values')['frame'],
        'container': json.loads((WORK / 'container.json').read_text()),
        'inputs': json.loads((WORK / 'inputs.json').read_text()),
        'cardPreparation': json.loads((worker.directory / 'card-initial.json').read_text()),
        'recipeSha256': sha(ROOT / 'recipe.json'),
        'buildLockSha256': sha(ROOT / 'build-lock.json'),
        'files': {name: sha(dest / name) for name in ('state.bin', 'card.mcr')},
        'visualReviewRequired': True,
        'observation': 'Confirm Nino Pad gameplay.png before qualifying'})


def verify_baseline(recipe):
    directory = WORK / 'baseline'
    manifest = json.loads((directory / 'manifest.json').read_text())
    for name, digest in manifest['files'].items():
        if sha(directory / name) != digest:
            raise GateError(f'Baseline changed: {name}')
    if manifest['buildLockSha256'] != sha(ROOT / 'build-lock.json'):
        raise GateError('Baseline uses a different emulator build')
    if manifest['recipeSha256'] != sha(ROOT / 'recipe.json'):
        raise GateError('Baseline uses a different frozen recipe')
    if manifest['container']['imageId'] != json.loads((WORK / 'container.json').read_text())['imageId']:
        raise GateError('Baseline uses a different container image')
    if manifest['inputs'] != json.loads((WORK / 'inputs.json').read_text()):
        raise GateError('Baseline uses different original inputs')
    check_values(manifest['expected'], recipe['expected'])
    review = json.loads((directory / 'visual-review.json').read_text())
    if review['pngSha256'] != sha(directory / 'gameplay.png') or review['scene'] != 'Nino Pad':
        raise GateError('Baseline visual review is missing or stale')


def qualify(recipe):
    if recipe['status'] != 'calibrated' or not recipe['equipment']:
        raise GateError('Recipe must be calibrated before qualification')
    verify_baseline(recipe)
    if recipe.get('buildId') != json.loads((ROOT / 'build-lock.json').read_text())['buildId']:
        raise GateError('Recipe is calibrated for another emulator build')
    results = []
    for n in range(1, 4):
        started = time.monotonic()
        with Worker(f'trial-{n}', baseline=True) as worker:
            check_values(worker.client.action('values'), recipe['expected'])
            run_recipe(worker, recipe['equipment'])
            values = worker.client.action('memory', address=0x8008c0b0, length=0x300, name='player')
            worker.client.action('memory', address=0x8009c7f8, length=0x100, name='global')
            check_values(values, recipe['expected'])
            capture = worker.capture('equipment')
            if capture['uniformBlack']:
                raise GateError(f'Trial {n} has a uniformly black Equipment framebuffer')
            results.append({'trial': n, 'capture': capture, 'values': values,
                            'elapsedSeconds': time.monotonic() - started})
        if sha(worker.directory / 'card.mcr') != sha(WORK / 'baseline/card.mcr'):
            raise GateError(f'Trial {n} wrote to its disposable card')
    save_json(WORK / 'trials.json', results)
    if len({r['capture']['rawSha256'] for r in results}) != 1:
        raise GateError('Framebuffer hashes differ; inspect captures before claiming consistency')
    if len({r['capture']['frame'] for r in results}) != 1:
        raise GateError('Final emulated-frame checkpoints differ')
    manifest = json.loads((WORK / 'baseline/manifest.json').read_text())
    for name, digest in manifest['files'].items():
        if sha(WORK / 'baseline' / name) != digest:
            raise GateError(f'Baseline changed during trials: {name}')
    # Human/agent observation of the captured Equipment screen is a separate
    # evidence requirement. Identical hashes alone do not prove the right menu.
    return {'status': 'awaiting-visual-review', 'trials': results,
            'requiredReview': 'Confirm Equipment label and visible gear in all three equipment.png captures',
            'cardsUnchanged': True, 'baselineUnchanged': True,
            'buildId': recipe['buildId'], 'version': json.loads((ROOT / 'build-lock.json').read_text())['version'],
            'recipeSha256': sha(ROOT / 'recipe.json'),
            'baselineStateSha256': sha(WORK / 'baseline/state.bin'),
            'baselineCardSha256': sha(WORK / 'baseline/card.mcr'),
            'cardPreparation': json.loads((WORK / 'baseline/manifest.json').read_text())['cardPreparation']}


def command(args):
    directory = WORK / args.stage
    client = Client(directory / 'actions.jsonl')
    params = {k: v for k, v in vars(args).items() if k in ('button', 'frames', 'name', 'address', 'length') and v is not None}
    if args.action == 'baseline':
        proxy = type('CalibrationWorker', (), {'directory': directory, 'client': client,
                    'capture': lambda _, name: (client.action('capture', name=name), convert_capture(directory, name))[1]})()
        baseline(proxy, json.loads((ROOT / 'recipe.json').read_text())['expected'])
        result = {'baseline': 'created'}
    else:
        result = client.action(args.action, **params)
        if args.action == 'capture':
            result = convert_capture(directory, args.name)
    print(json.dumps(result))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='mode', required=True)
    sub.add_parser('calibrate')
    sub.add_parser('qualify')
    sub.add_parser('bootstrap')
    sub.add_parser('diagnose')
    p = sub.add_parser('command')
    p.add_argument('action', choices=['frames', 'press', 'resume', 'release', 'capture', 'memory', 'values', 'diagnostics', 'spu_irq', 'reset_normal', 'reset_fast', 'restore_saved', 'restore_diagnostic', 'save', 'baseline', 'quit'])
    p.add_argument('--stage', default='calibration')
    for flag in ('button', 'name'):
        p.add_argument('--' + flag)
    for flag in ('frames', 'address', 'length'):
        p.add_argument('--' + flag, type=lambda x: int(x, 0))
    args = parser.parse_args()
    if args.mode == 'command':
        command(args); return
    WORK.mkdir(exist_ok=True)
    verdict = {'status': 'failed', 'gate': 'inputs'}
    inputs = None
    display = None
    display_log = None
    try:
        inputs = input_hashes()
        save_json(WORK / 'inputs.json', inputs)
        if sha(ROOT / 'build-lock.json') != sha('/opt/harness/build-lock.json'):
            raise GateError('Mounted build lock differs from the worker image')
        verdict['gate'] = 'rendering'
        display_log = (WORK / 'xvfb.log').open('wb')
        display = subprocess.Popen(['Xvfb', ':99', '-screen', '0', '1024x768x24', '-nolisten', 'tcp'],
                                   stdout=display_log, stderr=subprocess.STDOUT)
        for _ in range(100):
            if Path('/tmp/.X11-unix/X99').exists():
                break
            if display.poll() is not None:
                raise GateError('Xvfb exited')
            time.sleep(.05)
        renderer = subprocess.run(['glxinfo', '-B'], capture_output=True, text=True, timeout=20)
        (WORK / 'renderer.txt').write_text(renderer.stdout + renderer.stderr)
        if renderer.returncode or 'llvmpipe' not in renderer.stdout:
            raise GateError('Mesa llvmpipe unavailable')
        version = subprocess.run([EMU, '-version'], capture_output=True, text=True, timeout=20)
        (WORK / 'emulator-version.json').write_text(version.stdout)
        lock = json.loads((ROOT / 'build-lock.json').read_text())
        reported = json.loads(version.stdout)
        if version.returncode or reported.get('version') != lock['version'] or reported.get('changeset') != lock['changeset']:
            raise GateError('Unexpected emulator build version')
        recipe = json.loads((ROOT / 'recipe.json').read_text())
        save_json(WORK / 'recipe-used.json', recipe)
        verdict['gate'] = 'input-and-boot' if args.mode != 'qualify' else 'replay'
        if args.mode == 'qualify':
            verdict.update(qualify(recipe))
        else:
            with Worker('calibration') as worker:
                if args.mode == 'diagnose':
                    worker.client.action('restore_diagnostic')
                    verdict['diagnosticOnly'] = True
                else:
                    for frames in (1, 2, 60):
                        worker.client.action('frames', frames=frames)
                    # The BIOS has not configured a visible framebuffer at frame 63.
                    worker.client.action('frames', frames=600)
                worker.capture('startup')
                print('CALIBRATION_READY', flush=True)
                if args.mode == 'bootstrap':
                    run_recipe(worker, recipe['boot'])
                    verdict['gate'] = 'load-scene'
                    baseline(worker, recipe['expected'])
                else:
                    while worker.process.poll() is None:
                        worker.client.check()
                        time.sleep(.1)
                    if worker.process.returncode != 0:
                        raise GateError(f'Emulator exited: {worker.process.returncode}')
            verdict['status'] = 'calibration-complete'
    except Exception as e:
        record_failure(verdict, verdict['gate'], str(e))
    finally:
        if inputs:
            changed = [path for path, digest in inputs.items() if sha(path) != digest]
            verdict['originalInputsUnchanged'] = not changed
            if changed:
                record_failure(verdict, 'input-preservation', f'Original inputs changed: {changed}')
            calibration_card = WORK / 'calibration/card.mcr'
            if calibration_card.exists():
                initial = json.loads((calibration_card.parent / 'card-initial.json').read_text())
                verdict['cardPreparation'] = initial
                verdict['calibrationCardUnchanged'] = sha(calibration_card) == initial['initialSha256']
                if not verdict['calibrationCardUnchanged']:
                    record_failure(verdict, 'card-preservation', 'Calibration wrote its disposable card')
        if display and display.poll() is None:
            display.terminate(); display.wait(timeout=5)
        if display_log:
            display_log.close()
        save_json(WORK / ('qualification.json' if args.mode == 'qualify' else 'calibration.json'), verdict)
        if verdict['status'] == 'failed':
            save_json(WORK / 'result.json', verdict)
    print(json.dumps(verdict), flush=True)
    if verdict['status'] == 'failed':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
