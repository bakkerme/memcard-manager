"""Container command contracts, without contacting a Docker/Podman daemon."""
from contextlib import contextmanager
import io
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import host


@contextmanager
def hidden_session_env():
    """Drop host deadline overrides so command snapshots stay exact."""
    with patch.dict(os.environ):
        os.environ.pop('PS1_REPLAY_BASELINE_SECONDS', None)
        os.environ.pop('PS1_REPLAY_SESSION_SECONDS', None)
        yield


class EngineTests(unittest.TestCase):
    def test_launch_requires_explicit_assets_before_contacting_engine(self):
        with patch('sys.stderr', io.StringIO()), patch.object(host, 'call') as call:
            with self.assertRaises(SystemExit) as error:
                host.main(['--engine', 'docker', 'bootstrap', '--run', 'new-run'])
            self.assertEqual(error.exception.code, 2)
            call.assert_not_called()

    def test_preflight_queries_selected_daemon_architecture(self):
        for engine, template, arch in [('docker', '{{.Architecture}}', 'aarch64'),
                                       ('podman', '{{.Host.Arch}}', 'arm64')]:
            def fake(argv, **kwargs):
                if argv[1] == 'info':
                    return SimpleNamespace(stdout=arch)
                if argv[1:3] == ['image', 'inspect']:
                    return SimpleNamespace(stdout='[]')
                raise AssertionError(argv)
            with patch.object(host, 'call', side_effect=fake) as call:
                report = host.preflight(engine)
                self.assertEqual(report['architecture'], arch)
                self.assertFalse(report['image_present'])
                info_calls = [c.args[0] for c in call.call_args_list if c.args[0][1] == 'info']
                self.assertEqual(info_calls, [[engine, 'info', '--format', template]])
        with patch.object(host, 'call', return_value=SimpleNamespace(stdout='x86_64')):
            with self.assertRaisesRegex(RuntimeError, 'native Linux ARM64'):
                host.preflight('docker')

    def test_local_docker_image_without_registry_digest_is_supported(self):
        info = {'Id': 'sha256:local', 'Architecture': 'arm64', 'RepoDigests': []}
        self.assertIsNone(host.image_metadata(info, 'docker')['digest'])
        self.assertEqual(host.image_metadata(info | {'Digest': 'sha256:registry'}, 'podman')['digest'], 'sha256:registry')

    def test_build_and_commands_use_selected_engine(self):
        digest = host.tool_input_hash()
        for engine in ('docker', 'podman'):
            missing = {'Id': 'sha256:test', 'Architecture': 'arm64'}
            built = {'Id': 'sha256:test', 'Architecture': 'arm64',
                     'Config': {'Labels': {'research.input_hash': digest}}}
            with patch.object(host, 'preflight'), patch.object(host, 'download'), \
                    patch.object(host, 'image_info', side_effect=[missing, built]), \
                    patch.object(host, 'call') as call, patch('sys.stdout', io.StringIO()):
                host.main(['--engine', engine, 'build'])
                argv = call.call_args.args[0]
                self.assertEqual(argv[:4], [engine, 'build', '--platform', 'linux/arm64'])
                self.assertIn(f'research.input_hash={digest}', argv)
                with hidden_session_env():
                    host.main(['--engine', engine, 'command', '--name', 'worker', 'frames', '--frames', '4'])
                self.assertEqual(call.call_args.args[0], [engine, 'exec', 'worker', 'python3', '/tools/ps1_replay/runner.py', 'command', 'frames', '--frames', '4'])

    def test_worker_mounts_inputs_read_only_and_records_engine(self):
        for engine in ('docker', 'podman'):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); disc = root / 'disc'; disc.mkdir()
                bios = root / 'bios.bin'; bios.write_bytes(b'bios')
                card = root / 'card.mcr'; card.write_bytes(b'card')
                run = root / 'run'
                calls = []

                def fake_run(argv, **kwargs):
                    calls.append(argv)
                    if argv[1] == 'ps':
                        return SimpleNamespace(stdout='stale123\n', returncode=0)
                    return SimpleNamespace(stdout='', returncode=0)

                with patch.object(host, 'preflight'), \
                        patch.object(host, 'image_info', return_value={'Id': 'sha256:test', 'Architecture': 'arm64'}), \
                        patch.object(host, 'call') as call, \
                        patch.object(host.subprocess, 'run', side_effect=fake_run), \
                        hidden_session_env():
                    host.main(['--engine', engine, 'bootstrap', '--run', str(run), '--bios', str(bios), '--disc', str(disc), '--card', str(card)])
                    argv = call.call_args.args[0]
                    self.assertEqual(argv[:3], [engine, 'run', '--rm'])
                    self.assertNotIn('-e', argv)
                    self.assertEqual(argv[argv.index('--label') + 1], 'memcard.ps1-replay=1')
                    self.assertEqual(calls, [
                        [engine, 'ps', '-aq', '--filter', 'name=^ps1-replay$',
                         '--filter', 'label=memcard.ps1-replay=1'],
                        [engine, 'rm', '-f', 'stale123'],
                    ])
                    self.assertIn(f'{card.resolve()}:/inputs/seed.mcr:ro', argv)
                    self.assertIn(f'{run.resolve()}:/work:rw', argv)
                    self.assertEqual(argv[argv.index('--network') + 1], 'none')
                    self.assertEqual(json.loads((run / 'container.json').read_text())['engine'], engine)

    def test_session_deadline_overrides_are_forwarded(self):
        env = {'PS1_REPLAY_BASELINE_SECONDS': '600', 'PS1_REPLAY_SESSION_SECONDS': '1200'}
        info = {'Id': 'sha256:test', 'Architecture': 'arm64'}
        for engine in ('docker', 'podman'):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); disc = root / 'disc'; disc.mkdir()
                bios = root / 'bios.bin'; bios.write_bytes(b'bios')
                card = root / 'card.mcr'; card.write_bytes(b'card')
                run = root / 'run'
                with patch.dict(os.environ, env), patch.object(host, 'preflight'), \
                        patch.object(host, 'image_info', return_value=info), \
                        patch.object(host, 'call') as call, \
                        patch.object(host.subprocess, 'run', return_value=SimpleNamespace(stdout='', returncode=0)):
                    host.main(['--engine', engine, 'bootstrap', '--run', str(run), '--bios', str(bios),
                               '--disc', str(disc), '--card', str(card)])
                    argv = call.call_args.args[0]
                    self.assertLess(argv.index('-e'), argv.index(info['Id']))
                    self.assertEqual(argv[argv.index('-e') + 1], 'PS1_REPLAY_BASELINE_SECONDS=600')
                    self.assertIn('PS1_REPLAY_SESSION_SECONDS=1200', argv)
            with patch.dict(os.environ, env), patch.object(host, 'call') as call:
                host.main(['--engine', engine, 'command', '--name', 'worker', 'frames', '--frames', '4'])
                argv = call.call_args.args[0]
                self.assertEqual(argv[1], 'exec')
                self.assertLess(argv.index('-e'), argv.index('worker'))
                self.assertIn('PS1_REPLAY_BASELINE_SECONDS=600', argv)
                self.assertIn('PS1_REPLAY_SESSION_SECONDS=1200', argv)

    def test_invalid_session_deadline_is_rejected_before_the_engine(self):
        for value in ('nope', '0', '-5', 'nan', 'inf'):
            with patch.dict(os.environ, {'PS1_REPLAY_SESSION_SECONDS': value}), \
                    patch.object(host, 'call') as call:
                with self.assertRaisesRegex(RuntimeError, 'positive number'):
                    host.main(['--engine', 'docker', 'command', 'frames', '--frames', '1'])
                call.assert_not_called()

    def test_derived_tag_is_stable_for_the_same_files(self):
        first = host.image_name()
        second = host.derived_tag(host.tool_input_hash())
        self.assertEqual(first, second)
        self.assertEqual(first, host.image_name())
        self.assertRegex(first, r'^localhost/rr-tool/ps1-replay:in-[0-9a-f]{16}$')
        self.assertEqual(host.legacy_image_name(), 'localhost/ps1-replay:70')

    def _labelled(self, digest, architecture='arm64', image_id='sha256:test'):
        return {'Id': image_id, 'Architecture': architecture,
                'Config': {'Labels': {'research.input_hash': digest}}}

    def test_ensure_builds_when_the_image_is_absent(self):
        digest = host.tool_input_hash()
        built = self._labelled(digest, image_id='sha256:built')
        calls = []

        def fake(argv, **kwargs):
            calls.append(argv)
            if argv[1:3] == ['image', 'inspect']:
                raise host.subprocess.CalledProcessError(1, argv)
            return SimpleNamespace(stdout='', returncode=0)

        with patch.object(host, 'preflight'), patch.object(host, 'download') as download, \
                patch.object(host, 'call', side_effect=fake), \
                patch.object(host, 'image_info', side_effect=[None, built]), \
                patch('sys.stdout', io.StringIO()) as stdout:
            host.main(['--engine', 'docker', 'ensure'])
        download.assert_called_once()
        self.assertTrue(any(argv[1] == 'build' and f'research.input_hash={digest}' in argv for argv in calls))
        self.assertEqual(json.loads(stdout.getvalue())['action'], 'built')

    def test_ensure_skips_when_the_label_matches(self):
        digest = host.tool_input_hash()
        present = self._labelled(digest)
        with patch.object(host, 'preflight'), patch.object(host, 'download') as download, \
                patch.object(host, 'image_info', return_value=present), \
                patch.object(host, 'call') as call, patch('sys.stdout', io.StringIO()) as stdout:
            host.main(['--engine', 'docker', 'ensure'])
        download.assert_not_called()
        call.assert_not_called()
        report = json.loads(stdout.getvalue())
        self.assertEqual(report['action'], 'reused')
        self.assertEqual(report['inputHash'], digest)
        self.assertEqual(report['tag'], host.derived_tag(digest))

    def test_ensure_rebuilds_when_the_label_mismatches(self):
        digest = host.tool_input_hash()
        stale = self._labelled('0' * 64, image_id='sha256:stale')
        built = self._labelled(digest, image_id='sha256:fresh')
        with patch.object(host, 'preflight'), patch.object(host, 'download') as download, \
                patch.object(host, 'image_info', side_effect=[stale, built]), \
                patch.object(host, 'call') as call, patch('sys.stdout', io.StringIO()) as stdout:
            host.main(['--engine', 'docker', 'ensure'])
        download.assert_called_once()
        self.assertEqual(call.call_args.args[0][1], 'build')
        self.assertEqual(json.loads(stdout.getvalue())['action'], 'built')

    def test_ensure_aborts_when_the_zip_sha_mismatches(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ('Containerfile', 'build-lock.json', 'runner.py', 'adapter.lua'):
                (root / name).write_bytes((host.ROOT / name).read_bytes())
            lock = json.loads((root / 'build-lock.json').read_text())
            cache = root / '.cache'
            cache.mkdir()
            (cache / f"pcsx-redux-{lock['buildId']}.zip").write_bytes(b'not-the-emulator')
            with patch.object(host, 'ROOT', root), patch.object(host, 'preflight'), \
                    patch.object(host, 'image_info', return_value=None), \
                    patch.object(host, 'call') as call, patch('sys.stderr', io.StringIO()) as err:
                with self.assertRaises(SystemExit) as error:
                    host.main(['--engine', 'docker', 'ensure'])
        self.assertEqual(error.exception.code, 2)
        self.assertIn('checksum mismatch', err.getvalue())
        self.assertNotIn('\n', err.getvalue().rstrip('\n'))
        self.assertNotIn('Traceback', err.getvalue())
        call.assert_not_called()

    def test_ensure_rejects_a_non_arm64_image(self):
        digest = host.tool_input_hash()
        present = self._labelled(digest, architecture='amd64')
        with patch.object(host, 'preflight'), patch.object(host, 'download') as download, \
                patch.object(host, 'image_info', return_value=present), \
                patch.object(host, 'call') as call, patch('sys.stderr', io.StringIO()) as err:
            with self.assertRaises(SystemExit) as error:
                host.main(['--engine', 'docker', 'ensure'])
        self.assertEqual(error.exception.code, 2)
        self.assertIn('ARM64', err.getvalue())
        self.assertNotIn('Traceback', err.getvalue())
        download.assert_not_called()
        call.assert_not_called()
        built = self._labelled(digest, architecture='amd64', image_id='sha256:wrong')
        with patch.object(host, 'preflight'), patch.object(host, 'download'), \
                patch.object(host, 'image_info', side_effect=[None, built]), \
                patch.object(host, 'call'), patch('sys.stderr', io.StringIO()) as err:
            with self.assertRaises(SystemExit) as error:
                host.main(['--engine', 'podman', 'ensure'])
        self.assertEqual(error.exception.code, 2)
        self.assertIn('ARM64', err.getvalue())

    def test_missing_image_is_a_clean_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            disc = root / 'disc'
            disc.mkdir()
            bios = root / 'bios.bin'
            bios.write_bytes(b'bios')
            card = root / 'card.mcr'
            card.write_bytes(b'card')
            run = root / 'run'
            tag = host.image_name()
            with patch.object(host, 'preflight', return_value={'engine': 'docker', 'architecture': 'arm64',
                                                              'image': tag, 'image_present': False}), \
                    patch.object(host, 'image_info', return_value=None), \
                    patch.object(host, 'call') as call, patch('sys.stderr', io.StringIO()) as err:
                with self.assertRaises(SystemExit) as error:
                    host.main(['--engine', 'docker', 'bootstrap', '--run', str(run),
                               '--bios', str(bios), '--disc', str(disc), '--card', str(card)])
        self.assertEqual(error.exception.code, 2)
        message = err.getvalue()
        self.assertIn(f'worker image {tag} missing; run host.py ensure', message)
        self.assertNotIn('Traceback', message)
        call.assert_not_called()
        with patch.object(host, 'call', return_value=SimpleNamespace(stdout='[]')):
            self.assertIsNone(host.image_info('docker', tag))

    def test_image_tag_override_is_used_for_the_missing_image_error(self):
        override = 'localhost/rr-tool/ps1-replay:in-abcdef0123456789'
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            disc = root / 'disc'
            disc.mkdir()
            bios = root / 'bios.bin'
            bios.write_bytes(b'bios')
            card = root / 'card.mcr'
            card.write_bytes(b'card')
            with patch.object(host, 'preflight'), patch.object(host, 'image_info', return_value=None), \
                    patch('sys.stderr', io.StringIO()) as err:
                with self.assertRaises(SystemExit) as error:
                    host.main(['--engine', 'docker', '--image-tag', override, 'bootstrap',
                               '--run', str(root / 'run'), '--bios', str(bios),
                               '--disc', str(disc), '--card', str(card)])
        self.assertEqual(error.exception.code, 2)
        self.assertIn(override, err.getvalue())
        self.assertIn('run host.py ensure', err.getvalue())


if __name__ == '__main__':
    unittest.main()
