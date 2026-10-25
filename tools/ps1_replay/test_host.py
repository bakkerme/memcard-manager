"""Container command contracts, without contacting a Docker/Podman daemon."""
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import host


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
            with patch.object(host, 'call', return_value=SimpleNamespace(stdout=arch)) as call:
                self.assertEqual(host.preflight(engine)['architecture'], arch)
                self.assertEqual(call.call_args.args[0], [engine, 'info', '--format', template])
        with patch.object(host, 'call', return_value=SimpleNamespace(stdout='x86_64')):
            with self.assertRaisesRegex(RuntimeError, 'native Linux ARM64'):
                host.preflight('docker')

    def test_local_docker_image_without_registry_digest_is_supported(self):
        info = {'Id': 'sha256:local', 'Architecture': 'arm64', 'RepoDigests': []}
        self.assertIsNone(host.image_metadata(info, 'docker')['digest'])
        self.assertEqual(host.image_metadata(info | {'Digest': 'sha256:registry'}, 'podman')['digest'], 'sha256:registry')

    def test_build_and_commands_use_selected_engine(self):
        for engine in ('docker', 'podman'):
            info = {'Id': 'sha256:test', 'Architecture': 'arm64'}
            with patch.object(host, 'preflight'), patch.object(host, 'download'), \
                    patch.object(host, 'image_info', return_value=info), \
                    patch.object(host, 'call') as call, patch('sys.stdout', io.StringIO()):
                host.main(['--engine', engine, 'build'])
                self.assertEqual(call.call_args.args[0][:4], [engine, 'build', '--platform', 'linux/arm64'])
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
                        patch.object(host.subprocess, 'run', side_effect=fake_run):
                    host.main(['--engine', engine, 'bootstrap', '--run', str(run), '--bios', str(bios), '--disc', str(disc), '--card', str(card)])
                    argv = call.call_args.args[0]
                    self.assertEqual(argv[:3], [engine, 'run', '--rm'])
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


if __name__ == '__main__':
    unittest.main()
