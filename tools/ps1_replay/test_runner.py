"""Coordinator regressions: no emulator, ROM, BIOS or Podman required."""
import io
import json
import os
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
        self._env = patch.dict(os.environ)
        self._env.start()
        os.environ.pop('PS1_REPLAY_GUEST_WRITES', None)
        self.addCleanup(self._env.stop)

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

    def test_transient_status_poll_failure_does_not_abort_action(self):
        calls = {'n': 0}
        def request(endpoint, params=None):
            if endpoint == 'replay_command':
                return {'accepted': params['id']}
            calls['n'] += 1
            if calls['n'] == 1:
                raise OSError('timed out')
            return {'done': ident[0], 'result': {'ok': True}}
        ident = [None]
        original = runner.uuid.uuid4
        class Fixed:
            hex = 'f' * 32
        runner.uuid.uuid4 = lambda: Fixed
        ident[0] = 'f' * 32
        try:
            client = runner.Client(self.log, request=request)
            self.assertEqual(client.action('capture', timeout=5), {'ok': True})
        finally:
            runner.uuid.uuid4 = original

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

    def _client(self, result, seen):
        def request(endpoint, params=None):
            if endpoint == 'replay_command':
                seen.append(params)
                request.ident = params['id']
                return {'accepted': params['id']}
            return {'done': request.ident, 'result': dict(result)}
        return runner.Client(self.log, request=request)

    def test_guest_writes_stay_disabled_unless_the_session_opts_in(self):
        for value in ('', 'true', 'yes', '0'):
            with patch.dict(os.environ, {'PS1_REPLAY_GUEST_WRITES': value}):
                self.assertFalse(runner.guest_writes_enabled())
        self.assertFalse(runner.guest_writes_enabled())
        with patch.dict(os.environ, {'PS1_REPLAY_GUEST_WRITES': '1'}):
            self.assertTrue(runner.guest_writes_enabled())
            runner.assert_guest_writes_allowed('calibrate')
            runner.assert_guest_writes_allowed('diagnose')
            with self.assertRaisesRegex(runner.GateError, 'only available for calibrate and diagnose'):
                runner.assert_guest_writes_allowed('qualify')
            with self.assertRaisesRegex(runner.GateError, 'only available for calibrate and diagnose'):
                runner.assert_guest_writes_allowed('bootstrap')
        runner.assert_guest_writes_allowed('qualify')

    def test_poke_is_rejected_and_logged_when_guest_writes_are_disabled(self):
        seen = []
        client = self._client({'before': 0, 'after': 1, 'frame': 1}, seen)
        card = Path(self.temp.name) / 'card.mcr'
        card.write_bytes(b'MC-original')
        with self.assertRaisesRegex(runner.GateError, 'Guest memory writes are disabled'):
            client.action('poke', address=0x80010000, size=2, value=0)
        with self.assertRaisesRegex(runner.GateError, 'Guest memory writes are disabled'):
            client.action('poke-clear')
        self.assertEqual(seen, [])
        self.assertFalse((Path(self.temp.name) / 'memory-writes.jsonl').exists())
        self.assertEqual(card.read_bytes(), b'MC-original')
        logged = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertEqual([row['action'] for row in logged], ['poke', 'poke-clear'])
        self.assertIn('disabled', logged[0]['error'])

    def test_poke_records_before_after_frame_and_timestamp_when_enabled(self):
        seen = []
        card = Path(self.temp.name) / 'card.mcr'
        card.write_bytes(b'MC-original')
        result = {'address': 0x80010000, 'size': 2, 'value': 0, 'before': 4, 'after': 0,
                  'frame': 90, 'sticky': True, 'skipped': False, 'reapplied': False,
                  'when': {'op': 'eq', 'address': 0x80010004, 'size': 2, 'value': 0}}
        client = self._client(result, seen)
        with patch.dict(os.environ, {'PS1_REPLAY_GUEST_WRITES': '1'}):
            returned = client.action('poke', address=0x80010000, size=2, value=0, sticky=True,
                                     when_op='eq', when_address=0x80010004, when_size=2, when_value=0)
        self.assertEqual(seen[0]['action'], 'poke')
        self.assertEqual(seen[0]['address'], 0x80010000)
        self.assertEqual(seen[0]['size'], 2)
        self.assertEqual(seen[0]['value'], 0)
        self.assertEqual(seen[0]['sticky'], '1')
        self.assertEqual(seen[0]['when_op'], 'eq')
        self.assertEqual(seen[0]['when_value'], 0)
        self.assertEqual(len(seen), 1)
        self.assertEqual(returned['before'], 4)
        self.assertEqual(returned['after'], 0)
        self.assertEqual(returned['frame'], 90)
        self.assertIsInstance(returned['timestamp'], float)
        line = json.loads((Path(self.temp.name) / 'memory-writes.jsonl').read_text())
        for key in ('address', 'size', 'value', 'before', 'after', 'frame', 'timestamp'):
            self.assertIn(key, line)
        self.assertEqual(line['address'], 0x80010000)
        self.assertEqual(line['size'], 2)
        self.assertEqual(line['value'], 0)
        self.assertEqual(line['before'], 4)
        self.assertEqual(line['after'], 0)
        self.assertEqual(line['frame'], 90)
        self.assertFalse(line['skipped'])
        self.assertTrue(line['sticky'])
        self.assertEqual(line['when']['value'], 0)
        self.assertEqual(card.read_bytes(), b'MC-original')

    def test_poke_rejects_bad_width_range_alignment_and_partial_conditions(self):
        seen = []
        client = self._client({}, seen)
        with patch.dict(os.environ, {'PS1_REPLAY_GUEST_WRITES': '1'}):
            cases = (
                ({'address': 0x80010000, 'size': 3, 'value': 1}, 'size must be 1, 2, or 4'),
                ({'address': 0xA0010000, 'size': 2, 'value': 1}, '0x80000000-0x80200000'),
                ({'address': 0x80010001, 'size': 2, 'value': 1}, 'aligned'),
                ({'address': 0x80010000, 'size': 2, 'value': 0x10000}, 'does not fit'),
                ({'address': 0x80010000, 'size': 1, 'value': -1}, 'does not fit'),
                ({'address': True, 'size': 1, 'value': 1}, 'must be integers'),
                ({'address': 0x80010000, 'size': 2, 'value': 1, 'when_op': 'eq'}, 'Poke condition needs'),
                ({'address': 0x80010000, 'size': 2, 'value': 1, 'when_op': 'gt',
                  'when_address': 0x80010004, 'when_size': 2, 'when_value': 1}, 'eq or ne'),
            )
            for params, message in cases:
                with self.assertRaisesRegex(runner.GateError, message):
                    client.action('poke', **params)
        self.assertEqual(seen, [])
        self.assertFalse((Path(self.temp.name) / 'memory-writes.jsonl').exists())

    def test_watch_reads_a_caller_supplied_address_without_enabling_writes(self):
        seen = []
        client = self._client({'address': 0x80010002, 'size': 1, 'value': 7, 'frame': 3, 'name': 'lives'}, seen)
        self.assertEqual(client.action('watch', address=0x80010002, size=1, name='lives')['value'], 7)
        self.assertEqual(seen[0]['action'], 'watch')
        self.assertNotIn('sticky', seen[0])
        self.assertFalse((Path(self.temp.name) / 'memory-writes.jsonl').exists())
        with self.assertRaisesRegex(runner.GateError, 'aligned'):
            client.action('watch', address=0x80010001, size=2)

    def test_qualification_recipe_rejects_pokes_even_when_writes_are_enabled(self):
        worker = SimpleNamespace(client=SimpleNamespace(action=lambda **_: self.fail('Executed poke')))
        with patch.dict(os.environ, {'PS1_REPLAY_GUEST_WRITES': '1'}):
            with self.assertRaisesRegex(runner.GateError, 'bounded controller'):
                runner.run_recipe(worker, [{'action': 'poke', 'address': 0x80010000, 'size': 2, 'value': 1}])

    def test_card_evidence_tags_control_and_cheat_derived_saves(self):
        initial, final = 'a' * 64, 'b' * 64
        closed = runner.classify_card(initial, initial, [], False)
        self.assertTrue(closed['calibrationCardUnchanged'])
        self.assertFalse(closed['failPreservation'])
        self.assertNotIn('cardEvidence', closed)
        changed = runner.classify_card(initial, final, [], False)
        self.assertTrue(changed['failPreservation'])
        control = runner.classify_card(initial, final, [], True)
        self.assertFalse(control['failPreservation'])
        self.assertTrue(control['cardEvidence']['control'])
        self.assertFalse(control['cardEvidence']['cheatDerived'])
        self.assertFalse(control['cardEvidence']['harnessPatchedCard'])
        poked = runner.classify_card(initial, final, [
            {'address': 1, 'skipped': False}, {'address': 1, 'skipped': True, 'reapplied': True}], True)
        self.assertFalse(poked['failPreservation'])
        self.assertTrue(poked['cardEvidence']['cheatDerived'])
        self.assertFalse(poked['cardEvidence']['control'])
        self.assertEqual(poked['guestMemoryWrites'], 1)
        skipped = runner.classify_card(initial, final, [{'skipped': True}], True)
        self.assertFalse(skipped['cardEvidence']['cheatDerived'])
        self.assertFalse(skipped['cardEvidence']['control'])
        self.assertEqual(skipped['guestMemoryWrites'], 0)
        unwritten = runner.classify_card(initial, initial, [{'skipped': False}], True)
        self.assertFalse(unwritten['cardEvidence']['cheatDerived'])
        self.assertFalse(unwritten['cardEvidence']['control'])

    def test_command_poke_exits_when_guest_writes_are_disabled(self):
        with patch.object(runner, 'WORK', Path(self.temp.name)):
            (Path(self.temp.name) / 'calibration').mkdir()
            err = io.StringIO()
            with patch('sys.stderr', err):
                with self.assertRaises(SystemExit) as error:
                    runner.main(['command', 'poke', '--address', '0x80010000', '--size', '1', '--value', '0'])
            self.assertEqual(error.exception.code, 1)
            self.assertIn('disabled', err.getvalue())

    def test_memory_write_evidence_rejects_a_corrupt_line(self):
        path = Path(self.temp.name)
        (path / 'memory-writes.jsonl').write_text('{"address":1}\nnot-json\n')
        with self.assertRaisesRegex(runner.GateError, 'line 2'):
            runner.load_memory_writes(path)

    def test_adapter_poke_is_opt_in_and_uses_sized_file_writes(self):
        source = Path(__file__).with_name('adapter.lua').read_text()
        self.assertIn("os.getenv('PS1_REPLAY_GUEST_WRITES') == '1'", source)
        self.assertIn('m:writeU8At(value, address)', source)
        self.assertIn('m:writeU16At(value, address)', source)
        self.assertIn('m:writeU32At(value, address)', source)
        self.assertNotIn('getMemPtr(', source)
        self.assertNotIn('card.mcr', source)

    def test_preservation_failure_does_not_erase_original_gate(self):
        verdict = {}
        runner.record_failure(verdict, 'load-scene', 'Black framebuffer')
        runner.record_failure(verdict, 'card-preservation', 'Card changed')
        self.assertEqual(verdict['gate'], 'load-scene')
        self.assertEqual(verdict['error'], 'Black framebuffer')
        self.assertEqual(len(verdict['failures']), 2)


if __name__ == '__main__':
    unittest.main()
