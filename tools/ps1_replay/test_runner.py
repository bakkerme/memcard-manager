"""Coordinator regressions: no emulator, ROM, BIOS or Podman required."""
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch
from types import SimpleNamespace

import runner
import review


class Process:
    returncode = 7

    def poll(self):
        return self.returncode


class FakeAdapter:
    def __init__(self, mode='success'):
        self.mode = mode
        self.commands = []
        self.result = None

    def __call__(self, endpoint, params=None):
        if endpoint == 'replay_command':
            self.commands.append(params)
            if params['action'] == 'release':
                self.result = {'done': params['id'], 'result': {'released': True}}
            elif params['action'] == 'quit':
                self.result = {}
            elif self.mode == 'success':
                self.result = {'done': params['id'], 'result': {'actualFrames': int(params['frames'])}}
            elif self.mode == 'overshoot':
                self.result = {'done': params['id'], 'result': {'actualFrames': int(params['frames']) + 1}}
            else:
                self.result = {}
            return {'accepted': params['id']}
        return self.result or {'ready': True}


class CoordinatorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.log = Path(self.temp.name) / 'actions.jsonl'

    def test_missing_inputs_fail_before_launch(self):
        with patch.object(runner.Path, 'glob', return_value=[]):
            with self.assertRaisesRegex(runner.GateError, 'CUE'):
                runner.input_hashes()

    def test_card_preparation_changes_only_unused_scratch_before_launch(self):
        source, destination = Path(self.temp.name) / 'source.mcr', Path(self.temp.name) / 'card.mcr'
        data = bytearray(131072); data[:2] = b'MC'; data[127] = ord('M') ^ ord('C')
        data[8192:] = bytes([37]) * (131072 - 8192)
        source.write_bytes(data)
        result = runner.prepare_card(source, destination, True)
        self.assertEqual(result['changedOffsets'], ['0x1f80', '0x1f81', '0x1fff'])
        self.assertTrue(result['saveBlocksUnchanged'])
        self.assertEqual(source.read_bytes(), data)
        self.assertEqual(runner.prepare_card(destination, source.with_name('again.mcr'), True)['changedOffsets'], [])

    def test_card_preparation_refuses_unfamiliar_scratch_data(self):
        source = Path(self.temp.name) / 'source.mcr'
        data = bytearray(131072); data[:2] = b'MC'; data[0x1f80] = 7
        source.write_bytes(data)
        with self.assertRaisesRegex(runner.GateError, 'scratch frame'):
            runner.prepare_card(source, source.with_name('copy.mcr'), True)

    def test_emulator_exit_is_distinct_from_timeout(self):
        client = runner.Client(self.log, process=Process())
        with self.assertRaisesRegex(runner.GateError, 'Emulator exited: 7'):
            client.ready()

    def test_deadline_stops_worker(self):
        client = runner.Client(self.log, deadline=time.monotonic() - 1)
        with self.assertRaisesRegex(runner.GateError, 'deadline'):
            client.check()

    def test_stalled_press_releases_buttons_and_records_failure(self):
        adapter = FakeAdapter('stall')
        client = runner.Client(self.log, request=adapter)
        with self.assertRaisesRegex(runner.GateError, 'stalled'):
            client.action('press', timeout=.01, frames=6, button='START')
        self.assertEqual([c['action'] for c in adapter.commands], ['press', 'release'])
        self.assertIn('stalled', json.loads(self.log.read_text())['error'])

    def test_frame_overshoot_is_rejected_and_releases_buttons(self):
        adapter = FakeAdapter('overshoot')
        client = runner.Client(self.log, request=adapter)
        with self.assertRaisesRegex(runner.GateError, 'overshot'):
            client.action('frames', frames=60)
        self.assertEqual(adapter.commands[-1]['action'], 'release')

    def test_acknowledged_frames_and_action_log(self):
        client = runner.Client(self.log, request=FakeAdapter())
        self.assertEqual(client.action('frames', frames=60)['actualFrames'], 60)
        self.assertEqual(json.loads(self.log.read_text())['result']['actualFrames'], 60)

    def test_transport_failure_attempts_release(self):
        actions = []

        def request(endpoint, params=None):
            if params:
                actions.append(params['action'])
            raise ConnectionResetError('disconnected')

        client = runner.Client(self.log, request=request)
        with self.assertRaises(ConnectionResetError):
            client.action('press', frames=6, button='START')
        self.assertEqual(actions, ['press', 'release'])

    def test_ram_mismatch_does_not_pass(self):
        with self.assertRaisesRegex(runner.GateError, 'zenny'):
            runner.check_values({'zenny': 0}, {'zenny': 11950})

    def test_diagnostic_execution_cannot_enter_a_qualification_recipe(self):
        worker = SimpleNamespace(client=SimpleNamespace(action=lambda **_: self.fail('Executed unsafe recipe')))
        for step in ({'action': 'resume'}, {'action': 'frames', 'frames': 0},
                     {'action': 'press', 'frames': 1, 'button': 'UNKNOWN'}):
            with self.assertRaisesRegex(runner.GateError, 'bounded controller'):
                runner.run_recipe(worker, [step])

    def test_black_gameplay_never_becomes_a_baseline(self):
        client = SimpleNamespace(action=lambda *_args, **_kwargs: {'zenny': 11950})
        worker = SimpleNamespace(client=client, capture=lambda _name: {'uniformBlack': True})
        with self.assertRaisesRegex(runner.GateError, 'uniformly black'):
            runner.baseline(worker, {'zenny': 11950})

    def test_visual_review_cannot_pass_incomplete_trials(self):
        directory = Path(self.temp.name)
        runner.save_json(directory / 'qualification.json', {'status': 'awaiting-visual-review',
                         'originalInputsUnchanged': True, 'trials': [{'trial': 1}]})
        with self.assertRaisesRegex(runner.GateError, 'Three complete'):
            review.review_equipment(directory, 'test', 'Equipment observed')
        self.assertFalse((directory / 'result.json').exists())

    def test_visual_review_rejects_differing_framebuffer_hashes(self):
        directory = Path(self.temp.name)
        runner.save_json(directory / 'qualification.json', {
            'status': 'awaiting-visual-review', 'originalInputsUnchanged': True,
            'trials': [{'trial': n, 'capture': {'rawSha256': str(n), 'frame': 288}} for n in (1, 2, 3)]})
        with self.assertRaisesRegex(runner.GateError, 'framebuffer hashes differ'):
            review.review_equipment(directory, 'test', 'Equipment observed')
        self.assertFalse((directory / 'result.json').exists())

    def test_baseline_card_tampering_is_rejected(self):
        directory = Path(self.temp.name) / 'baseline'
        directory.mkdir()
        (directory / 'card.mcr').write_bytes(b'changed')
        runner.save_json(directory / 'manifest.json', {'files': {'card.mcr': 'original'}})
        with patch.object(runner, 'WORK', Path(self.temp.name)):
            with self.assertRaisesRegex(runner.GateError, 'Baseline changed: card.mcr'):
                runner.verify_baseline({})

    def test_quit_acknowledgement_does_not_require_a_live_server(self):
        client = runner.Client(self.log, request=FakeAdapter())
        self.assertEqual(client.action('quit'), {'scheduled': True})

    def test_cleanup_distinguishes_exited_process_from_release_failure(self):
        with patch.object(runner, 'WORK', Path(self.temp.name)):
            worker = runner.Worker('exited')
            worker.process = Process()
            worker.client = SimpleNamespace(release_safely=lambda: self.fail('Exited process has no live adapter'))
            worker.close()
            cleanup = json.loads((worker.directory / 'cleanup.json').read_text())
            self.assertIsNone(cleanup['releaseAcknowledged'])
            self.assertTrue(cleanup['emulatorStopped'])
            self.assertEqual(cleanup['exitCode'], 7)

    def test_preservation_failure_does_not_erase_original_gate(self):
        verdict = {}
        runner.record_failure(verdict, 'load-scene', 'Black framebuffer')
        runner.record_failure(verdict, 'card-preservation', 'Card changed')
        self.assertEqual(verdict['gate'], 'load-scene')
        self.assertEqual(verdict['error'], 'Black framebuffer')
        self.assertEqual(len(verdict['failures']), 2)


if __name__ == '__main__':
    unittest.main()
