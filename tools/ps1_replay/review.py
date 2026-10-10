#!/usr/bin/env python3
"""Record visual evidence after an operator or agent inspects local PNGs.

This does not recognize screens. It records an explicit reviewer observation,
bound to exact artifact hashes; matching screenshots alone never pass the test.
"""
import argparse
import json
from pathlib import Path

from runner import GateError, save_json, sha


def review_baseline(run, reviewer, note):
    baseline = run / 'baseline'
    manifest = json.loads((baseline / 'manifest.json').read_text())
    for name, digest in manifest['files'].items():
        if sha(baseline / name) != digest:
            raise GateError(f'Baseline changed: {name}')
    save_json(baseline / 'visual-review.json', {'reviewer': reviewer, 'scene': 'Nino Pad',
              'note': note, 'pngSha256': sha(baseline / 'gameplay.png')})


def review_equipment(run, reviewer, note):
    verdict = json.loads((run / 'qualification.json').read_text())
    if (verdict['status'] != 'awaiting-visual-review' or not verdict.get('originalInputsUnchanged')
            or [r['trial'] for r in verdict.get('trials', [])] != [1, 2, 3]):
        raise GateError('Three complete, input-preserving trials are required')
    if len({r['capture']['rawSha256'] for r in verdict['trials']}) != 1:
        raise GateError('Final framebuffer hashes differ')
    if len({r['capture']['frame'] for r in verdict['trials']}) != 1:
        raise GateError('Final frame checkpoints differ')
    manifest = json.loads((run / 'baseline/manifest.json').read_text())
    for name, digest in manifest['files'].items():
        if sha(run / 'baseline' / name) != digest:
            raise GateError(f'Baseline changed: {name}')
    for result in verdict['trials']:
        if result['capture'].get('uniformBlack'):
            raise GateError('Equipment framebuffer is black')
        folder = run / f"trial-{result['trial']}"
        if sha(folder / 'equipment.png') != result['capture']['pngSha256']:
            raise GateError('Equipment capture changed after qualification')
        if sha(folder / 'equipment.raw') != result['capture']['rawSha256']:
            raise GateError('Framebuffer changed after qualification')
        if sha(folder / 'card.mcr') != sha(run / 'baseline/card.mcr'):
            raise GateError('Disposable card changed after qualification')
    verdict.update(status='passed', visualReview={'reviewer': reviewer, 'note': note,
                   'screen': 'Equipment', 'gearMatchesKnownSave': True})
    save_json(run / 'result.json', verdict)


def failure(run, reviewer, gate, note, capture):
    folder = run / 'calibration'
    original = json.loads((run / 'inputs.json').read_text())
    seed_hash = original['/inputs/seed.mcr']
    calibration = json.loads((run / 'calibration.json').read_text())
    actions = [json.loads(line) for line in (folder / 'actions.jsonl').read_text().splitlines()]
    frames = [a for a in actions if a['action'] in ('frames', 'press') and 'result' in a]
    expected = json.loads((Path(__file__).parent / 'recipe.json').read_text())['expected']
    actual = next((a['result'] for a in reversed(actions) if a['action'] == 'values'), {})
    metadata = json.loads((folder / (capture + '.json')).read_text())
    seed_path = Path(__file__).resolve().parents[2] / 'tests/fixtures/ps1/local/mml2/mml2-viewer-qa.mcr'
    differences = None
    if seed_path.is_file() and sha(seed_path) == seed_hash:
        seed, current = seed_path.read_bytes(), (folder / 'card.mcr').read_bytes()
        differences = {'changedOffsets': [hex(i) for i, (a, b) in enumerate(zip(seed, current)) if a != b],
                       'sameLength': len(seed) == len(current), 'saveBlocksUnchanged': seed[8192:] == current[8192:]}
    verdict = {'status': 'failed', 'gate': gate, 'reason': note,
        'reviewer': reviewer, 'originalInputsUnchanged': calibration.get('originalInputsUnchanged', False),
        'calibrationCardUnchanged': sha(folder / 'card.mcr') == seed_hash,
        'cardDifferences': differences,
        'knownSaveValuesMatch': all(actual.get(k) == v for k, v in expected.items()),
        'qualifiedFrameActions': len(frames),
        'frameCountsExact': all(a['result']['actualFrames'] == a['frames'] for a in frames),
        'finalCapture': {'file': f'calibration/{capture}.png', 'pngSha256': sha(folder / (capture + '.png')),
                         'rawSha256': sha(folder / (capture + '.raw')), **metadata},
        'replayTrialsCompleted': 0,
        'emulator': json.loads((run / 'emulator-version.json').read_text()),
        'container': json.loads((run / 'container.json').read_text())}
    save_json(run / 'result.json', verdict)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['baseline', 'equipment', 'failure'])
    parser.add_argument('--run', type=Path, required=True)
    parser.add_argument('--reviewer', required=True)
    parser.add_argument('--note', required=True)
    parser.add_argument('--gate', default='load-scene')
    parser.add_argument('--capture', default='loaded-final')
    args = parser.parse_args()
    if args.mode == 'baseline':
        review_baseline(args.run, args.reviewer, args.note)
    elif args.mode == 'equipment':
        review_equipment(args.run, args.reviewer, args.note)
    else:
        failure(args.run, args.reviewer, args.gate, args.note, args.capture)


if __name__ == '__main__':
    main()
