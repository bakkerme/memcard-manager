"""Deterministic research-package assembly. No BIOS, disc, or save bytes are committed."""
import filecmp
import json
import os
import posixpath
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import assemble_package as package_builder
from packaging_contract import CONTRACT_INPUT, CONTRACT_SOURCE
from assemble_package import (
    PLACEHOLDER_SHA, READER_PREFIX, REQUIRED_OUTPUTS, SKILL_ROOTS, TEMPLATE_PATHS,
    TOOL_PATHS, assemble_package as build_package,
)

REAL = Path(__file__).resolve().parents[2]
ABS_PATH = re.compile(r'(?:/home/|/Users/|/workspace|/tmp/|/opt/|[A-Za-z]:\\)')
ISO_TIME = re.compile(r'\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}')
OPTIONAL_LOAD = re.compile(r'(?i)optional.{0,80}load|load.{0,80}optional')


def sha256(path):
    return package_builder.sha256_file(path)


def copy_tree(source, dest):
    if source.is_symlink() or not source.is_file():
        return
    if '__pycache__' in source.parts or source.suffix == '.pyc':
        return
    target = dest / source.relative_to(REAL)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)


def snapshot_sources(dest):
    for relative in TOOL_PATHS:
        copy_tree(REAL / relative, dest)
    for relative in (*TEMPLATE_PATHS, CONTRACT_SOURCE):
        copy_tree(REAL / relative, dest)
    for root in SKILL_ROOTS:
        for path in (REAL / root).rglob('*'):
            copy_tree(path, dest)
    for path in (REAL / READER_PREFIX).rglob('*'):
        copy_tree(path, dest)


def git(repo, *args):
    subprocess.run(['git', '-C', str(repo), *args], check=True, capture_output=True, text=True)


def init_fixture(repo):
    snapshot_sources(repo)
    git(repo, 'init', '-b', 'main')
    git(repo, 'config', 'user.email', 'assembler-test@example.com')
    git(repo, 'config', 'user.name', 'Assembler Test')
    git(repo, 'add', '-A')
    git(repo, 'commit', '-m', 'fixture', '--no-gpg-sign')


def filled_spec(bios, save, directory, include_save=True):
    """Copy the committed spec and pin private-input digests to these files.

    The committed Rayman spec has no save. Tests of the save-present path pass
    include_save=True, which declares one. The optional-save path leaves it out.
    """
    spec = json.loads((REAL / 'research/specs/rayman-us.json').read_text())
    roles = {item['role'] for item in spec['private_inputs']}
    if include_save and 'save' not in roles:
        spec['private_inputs'].append({
            'role': 'save',
            'path': 'inputs/saves/rayman-na.gme',
            'path_env': 'RAYMAN_SAVE',
            'sha256': '0' * 64,
            'provenance': 'Fixture save supplied at assemble time.',
        })
    for item in spec['private_inputs']:
        if item['role'] == 'bios':
            item['sha256'] = sha256(bios)
        elif item['role'] == 'save':
            if save is None:
                raise ValueError('save file required when the spec declares one')
            item['sha256'] = sha256(save)
    path = directory / 'rayman-us.json'
    path.write_text(json.dumps(spec, indent=2, sort_keys=True) + '\n')
    return path


def archive_text(archive, name):
    with tarfile.open(archive, 'r:gz') as tar:
        return tar.extractfile(name).read().decode()


def envelope(archive):
    with tarfile.open(archive, 'r:gz') as tar:
        return json.loads(tar.extractfile('package.json').read())


class AssembleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        root = Path(cls.temp.name)
        cls.repo = root / 'repo'
        cls.repo.mkdir()
        init_fixture(cls.repo)
        cls.bios = root / 'bios.bin'
        cls.save = root / 'rayman.gme'
        cls.bios.write_bytes(b'bios-fixture')
        cls.save.write_bytes(b'save-fixture')
        cls.spec = filled_spec(cls.bios, cls.save, root)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def assemble(self, out, **kwargs):
        options = dict(fmt='v2', bios=self.bios, save=self.save, repo=self.repo)
        options.update(kwargs)
        return build_package(self.spec, out, **options)

    def test_assembling_twice_is_identical_across_directory_cwd_and_mtime(self):
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            os.utime(self.repo / 'tools/ps1_replay/host.py', (1_700_000_000, 1_700_000_000))
            before = os.environ.get('SOURCE_DATE_EPOCH')
            os.environ['SOURCE_DATE_EPOCH'] = '0'
            try:
                left = self.assemble(first)
                os.environ['SOURCE_DATE_EPOCH'] = '1700000000'
                command = [sys.executable, str(HERE / 'assemble_package.py'),
                           '--spec', str(self.spec), '--out', second, '--format', 'v2',
                           '--bios', str(self.bios), '--save', str(self.save),
                           '--repo', str(self.repo)]
                completed = subprocess.run(command, cwd=first, check=True, capture_output=True, text=True)
            finally:
                if before is None:
                    os.environ.pop('SOURCE_DATE_EPOCH', None)
                else:
                    os.environ['SOURCE_DATE_EPOCH'] = before
            right = json.loads(completed.stdout)
            self.assertEqual(left['sha256'], right['sha256'])
            self.assertEqual(left['definition_sha256'], right['definition_sha256'])
            self.assertTrue(filecmp.cmp(left['package'], right['package'], shallow=False))
            self.assertTrue(left['package'].endswith(left['sha256'] + '.rrpkg'))

    def test_template_byte_and_private_input_change_the_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            clean = self.assemble(Path(tmp) / 'clean')
            template = self.repo / 'research/templates/brief.md'
            original = template.read_bytes()
            try:
                template.write_bytes(original.replace(b'Read inputs', b'Read  inputs', 1))
                with self.assertRaisesRegex(ValueError, 'dirty worktree'):
                    self.assemble(Path(tmp) / 'refused')
                dirty = self.assemble(Path(tmp) / 'dirty', allow_dirty=True)
                self.assertNotEqual(dirty['sha256'], clean['sha256'])
            finally:
                template.write_bytes(original)
            restored = self.assemble(Path(tmp) / 'restored')
            self.assertEqual(restored['sha256'], clean['sha256'])

            changed = Path(tmp) / 'changed.gme'
            changed.write_bytes(b'save-fixture-changed')
            with self.assertRaisesRegex(ValueError, 'sha256 mismatch'):
                self.assemble(Path(tmp) / 'mismatch', save=changed)
            spec = json.loads(self.spec.read_text())
            for item in spec['private_inputs']:
                if item['role'] == 'save':
                    item['sha256'] = sha256(changed)
            spec_path = Path(tmp) / 'changed-spec.json'
            spec_path.write_text(json.dumps(spec, indent=2, sort_keys=True) + '\n')
            updated = build_package(spec_path, Path(tmp) / 'updated', fmt='v2',
                                    bios=self.bios, save=changed, repo=self.repo)
            self.assertNotEqual(updated['sha256'], clean['sha256'])

    def test_v2_package_verifies_and_carries_its_own_image_build(self):
        try:
            runner = package_builder.load_research_runner()
        except ValueError as error:
            self.skipTest(str(error))
        verify = runner.package.verify
        capabilities = runner.protocol.capabilities
        with tempfile.TemporaryDirectory() as tmp:
            built = self.assemble(tmp)
            manifest, files = verify(built['package'])
            self.assertEqual(manifest['format'], 'research-package/v2')
            self.assertEqual(manifest['inputs'], sorted(manifest['inputs']))
            self.assertEqual(manifest['dependencies'], [{
                'asset_type': 'psx-rom', 'asset_id': 'SLUS-00005', 'destination': 'inputs/roms/game'}])
            self.assertEqual(manifest['required_outputs'], list(REQUIRED_OUTPUTS))
            self.assertNotIn('tool_images', manifest)
            self.assertNotIn('required_checks', manifest)
            self.assertNotIn('image', manifest['runtime'])
            self.assertNotIn('image_id', manifest['runtime'])
            for name in ('tools/ps1_replay/Containerfile', 'tools/ps1_replay/build-lock.json',
                         'tools/ps1_replay/host.py', 'tools/inspect_mml2.py',
                         'inputs/tool-images.json',
                         'inputs/bios/ps-22a.bin', 'inputs/saves/rayman-na.gme'):
                self.assertIn(name, files)
            self.assertNotIn('inputs/roms/game', files)
            agents = archive_text(built['package'], 'AGENTS.md')
            brief = archive_text(built['package'], 'brief.md')
            self.assertIn('python3 tools/ps1_replay/host.py --engine docker ensure', agents)
            self.assertIn('Step 0', agents)
            self.assertIsNone(OPTIONAL_LOAD.search(agents))
            self.assertIsNone(OPTIONAL_LOAD.search(brief))
            self.assertNotIn('image_id', agents)
            self.assertNotIn('image_id', brief)
            self.assertNotIn('rebuild permitted', (agents + brief).lower())
            self.assertIn('input hash', agents)
            lock = json.loads((self.repo / 'tools/ps1_replay/build-lock.json').read_text())
            self.assertEqual(lock['buildId'], 206)
            self.assertIn(lock['sha256'], agents)
            self.assertIn(str(lock['buildId']), agents)
            generated = ('AGENTS.md', 'brief.md', 'inputs/question.md', 'inputs/spec.json',
                         'inputs/provenance.json', 'inputs/tool-images.json', 'inputs/contracts.json')
            with tarfile.open(built['package'], 'r:gz') as tar:
                payload = tar.extractfile('package.json').read().decode()
                self.assertIsNone(ABS_PATH.search(payload))
                self.assertIsNone(ISO_TIME.search(payload))
                for name in generated:
                    text = tar.extractfile(name).read().decode()
                    self.assertIsNone(ABS_PATH.search(text), name)
                    self.assertIsNone(ISO_TIME.search(text), name)
                    self.assertNotIn(str(self.temp.name), text)
            version = tuple(int(part) for part in capabilities()['version'].split('.')[:3])
            v3 = self.assemble(Path(tmp) / 'v3', fmt='v3')
            payload = envelope(v3['package'])['manifest']
            self.assertEqual(payload['format'], 'research-package/v3')
            self.assertEqual(payload['required_checks'], [
                'tool-image-provisioned', 'native-load', 'parser-suite', 'cross-check-editor'])
            image = payload['tool_images'][0]
            self.assertNotIn('input_hash', image)
            try:
                from runner import tool_images
            except ImportError:
                tool_images = None
            if tool_images is not None:
                self.assertEqual(image['tag'], tool_images.derived_tag(image))
                self.assertEqual(package_builder.canonical_tool_image_hash(image),
                                 tool_images.input_hash(image))
            else:
                digest = package_builder.canonical_tool_image_hash(image)
                self.assertEqual(image['tag'], 'localhost/rr-tool/ps1-replay:in-' + digest[:16])
            self.assertNotIn('image_id', json.dumps(image))
            self.assertNotIn('Step 0', archive_text(v3['package'], 'AGENTS.md'))
            if version >= (0, 5, 0):
                verify(v3['package'])

    def test_clean_checkout_assembles_rayman_twice_with_the_same_sha(self):
        status = subprocess.check_output(['git', '-C', str(REAL), 'status', '--porcelain'], text=True)
        if status.strip():
            self.skipTest('commit the worktree before assembling the real Rayman package')
        tracked = set(package_builder.list_tracked(REAL))
        self.assertIn('research/specs/rayman-us.json', tracked)
        self.assertIn('research/templates/AGENTS.md', tracked)
        self.assertIn('tools/ps1_replay/Containerfile', tracked)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bios = root / 'bios.bin'
            bios.write_bytes(b'real-checkout-bios')
            spec = filled_spec(bios, None, root, include_save=False)
            first = build_package(spec, root / 'one', fmt='v2', bios=bios, repo=REAL)
            second = build_package(spec, root / 'two', fmt='v2', bios=bios, repo=REAL)
            self.assertEqual(first['sha256'], second['sha256'])
            self.assertTrue(filecmp.cmp(first['package'], second['package'], shallow=False))
            with tarfile.open(first['package'], 'r:gz') as tar:
                names = tar.getnames()
            self.assertFalse(any(name.endswith('.gme') for name in names))
            self.assertIn(json.loads(spec.read_text())['no_save_note'],
                          archive_text(first['package'], 'inputs/question.md'))
            runner = package_builder.load_research_runner()
            manifest, _files = runner.package.verify(first['package'])
            self.assertEqual(manifest['name'], 'rayman-us-save-decoder')
            self.assertNotIn('provenance', manifest)
            provenance = json.loads(archive_text(first['package'], 'inputs/provenance.json'))
            head = subprocess.check_output(['git', '-C', str(REAL), 'rev-parse', 'HEAD'], text=True).strip()
            self.assertEqual(provenance['memcard_manager_commit'], head)

    def test_committed_rayman_spec_keeps_private_bytes_out_of_git(self):
        spec = json.loads((REAL / 'research/specs/rayman-us.json').read_text())
        self.assertEqual(spec['format'], 'memcard-research-spec/v1')
        self.assertEqual(spec['serial'], 'SLUS-00005')
        self.assertEqual(spec['product_code'], 'BASLUS-00005')
        self.assertEqual(spec['asset']['asset_id'], 'SLUS-00005')
        self.assertEqual(spec['asset']['destination'], 'inputs/roms/game')
        self.assertIn('c4c80f40fd0e102e2dac7257851f43167b2c350f', spec['cross_check']['url'])
        self.assertEqual(spec['cross_check']['license'], 'MIT')
        self.assertIn('Do not copy', spec['cross_check']['note'])
        by_role = {item['role']: item for item in spec['private_inputs']}
        self.assertNotIn('save', by_role)
        self.assertIn('Generate saves in the emulator', spec['no_save_note'])
        self.assertNotIn(PLACEHOLDER_SHA, json.dumps(spec['private_inputs']))
        self.assertEqual(by_role['bios']['sha256'],
                         '71af94d1e47a68c11e8fdb9f8368040601514a42a5a399cda48c7d3bff1e99d3')
        self.assertIn('completion_percent', spec['fields_target'])
        tracked = subprocess.check_output(['git', '-C', str(REAL), 'ls-files'], text=True)
        self.assertNotIn('ps-22a.bin', tracked)
        gme_files = [line for line in tracked.splitlines() if line.endswith('.gme')]
        self.assertEqual(gme_files, [
            'src-tauri/src/card/game_details/testdata/rayman-gamefaqs-100.gme',
        ])
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, 'sha256 mismatch'):
                build_package(REAL / 'research/specs/rayman-us.json', tmp,
                              bios=self.bios, repo=self.repo)

    def test_optional_save_packages_no_save_and_tells_the_agent_to_generate_one(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bios = root / 'bios.bin'
            bios.write_bytes(b'bios-without-save')
            stray = root / 'not-packaged.gme'
            stray.write_bytes(b'this save must not be copied')
            spec = filled_spec(bios, None, root, include_save=False)
            self.assertNotIn('save', {item['role'] for item in json.loads(spec.read_text())['private_inputs']})
            broken = json.loads(spec.read_text())
            broken['private_inputs'] = []
            empty_path = root / 'empty.json'
            empty_path.write_text(json.dumps(broken) + '\n')
            with self.assertRaisesRegex(ValueError, 'non-empty'):
                build_package(empty_path, root / 'rejected-empty', bios=bios, repo=self.repo)
            broken['private_inputs'] = [{
                'role': 'save', 'path': 'inputs/saves/x.gme', 'sha256': sha256(stray),
                'provenance': 'A save without a BIOS is not a launchable package.',
            }]
            no_bios = root / 'no-bios.json'
            no_bios.write_text(json.dumps(broken) + '\n')
            with self.assertRaisesRegex(ValueError, 'must include bios'):
                build_package(no_bios, root / 'rejected-bios', bios=bios, save=stray, repo=self.repo)
            first = build_package(spec, root / 'one', fmt='v2', bios=bios, save=stray, repo=self.repo)
            second = build_package(spec, root / 'two', fmt='v2', bios=bios, repo=self.repo)
            self.assertEqual(first['sha256'], second['sha256'])
            self.assertTrue(filecmp.cmp(first['package'], second['package'], shallow=False))
            runner = package_builder.load_research_runner()
            manifest, files = runner.package.verify(first['package'])
            self.assertEqual(manifest['required_outputs'], list(REQUIRED_OUTPUTS))
            self.assertNotIn('inputs/saves/rayman-na.gme', files)
            self.assertFalse(any(name.endswith('.gme') for name in files))
            self.assertIn('inputs/bios/ps-22a.bin', files)
            question = archive_text(first['package'], 'inputs/question.md')
            agents = archive_text(first['package'], 'AGENTS.md')
            note = json.loads(spec.read_text())['no_save_note']
            self.assertIn(note, question)
            self.assertIn('(none supplied)', question)
            self.assertNotIn('Additional dependency', question)
            self.assertNotIn('@@', question)
            self.assertIn('create saves in the emulator', agents)
            self.assertIn('emulator-generated saves', question)
            self.assertIn('native-load', agents)
            self.assertIn('[output-packaging contract](output-packaging.md)', question)
            policy = archive_text(first['package'], CONTRACT_INPUT)
            self.assertIn('outputs/evidence-screenshots.tar.gz', policy)
            self.assertIn('outputs/field-ledger.md', policy)

    def test_v2_and_v3_include_one_identical_contract_and_resolving_links(self):
        link = re.compile(r'\[[^\]]+\]\(([^)\s]*output-packaging\.md)\)')
        expected = (self.repo / CONTRACT_SOURCE).read_bytes()
        for fmt in ('v2', 'v3'):
            with self.subTest(fmt=fmt), tempfile.TemporaryDirectory() as tmp:
                built = self.assemble(tmp, fmt=fmt)
                manifest = envelope(built['package'])['manifest']
                self.assertEqual(manifest['inputs'].count(CONTRACT_INPUT), 1)
                self.assertEqual(manifest['required_outputs'], list(REQUIRED_OUTPUTS))
                with tarfile.open(built['package'], 'r:gz') as archive:
                    self.assertEqual(archive.extractfile(CONTRACT_INPUT).read(), expected)
                    self.assertEqual(archive.getnames().count(CONTRACT_INPUT), 1)
                    self.assertEqual([name for name in archive.getnames()
                                      if name.endswith('output-packaging.md')], [CONTRACT_INPUT])
                    linked = set()
                    for name in archive.getnames():
                        if not name.endswith('.md') or name == CONTRACT_INPUT:
                            continue
                        text = archive.extractfile(name).read().decode()
                        for target in link.findall(text):
                            resolved = posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
                            self.assertEqual(resolved, CONTRACT_INPUT, name)
                            self.assertIn(resolved, manifest['inputs'])
                            linked.add(name)
                        self.assertNotIn('4,096', text, name)
                    self.assertTrue({'AGENTS.md', 'brief.md', 'inputs/question.md',
                                     'skills/ps1-save-research/SKILL.md',
                                     'skills/ps1-save-research/references/repository-guide.md',
                                     'skills/emulator-replay/SKILL.md'}.issubset(linked))

    def test_untracked_contract_is_rejected_instead_of_omitted(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp) / 'repo'
            repo.mkdir()
            snapshot_sources(repo)
            git(repo, 'init', '-b', 'main')
            git(repo, 'add', '-A')
            git(repo, 'rm', '--cached', CONTRACT_SOURCE)
            with self.assertRaisesRegex(ValueError, 'not tracked.*output-packaging'):
                package_builder.source_paths(repo)

    def test_contract_edits_flow_into_new_packages_without_refreshing_a_copy(self):
        source = self.repo / CONTRACT_SOURCE
        original = source.read_bytes()
        with tempfile.TemporaryDirectory() as tmp:
            before = self.assemble(Path(tmp) / 'before')
            try:
                updated = original + b'\nNew publication guidance.\n'
                source.write_bytes(updated)
                after = self.assemble(Path(tmp) / 'after', allow_dirty=True)
                self.assertNotEqual(before['sha256'], after['sha256'])
                self.assertEqual(archive_text(after['package'], CONTRACT_INPUT).encode(), updated)
            finally:
                source.write_bytes(original)

    def _spec_with(self, directory, extra_assets):
        spec = json.loads(self.spec.read_text())
        spec['extra_assets'] = extra_assets
        path = Path(directory) / 'extra-assets.json'
        path.write_text(json.dumps(spec) + '\n')
        return path

    def test_extra_asset_is_a_second_dependency(self):
        try:
            runner = package_builder.load_research_runner()
        except ValueError as error:
            self.skipTest(str(error))
        disc2 = {
            'asset_type': 'psx-rom',
            'asset_id': 'title:Metal Gear Solid (USA) (Disc 2)',
            'destination': 'inputs/roms/disc2',
            'provenance': 'Operator psx-rom loader, Disc 2 cue/bin. Not a packaged file.',
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec_path = self._spec_with(root, [disc2])
            built = build_package(spec_path, root / 'out', fmt='v2',
                                  bios=self.bios, save=self.save, repo=self.repo)
            manifest, files = runner.package.verify(built['package'])
            self.assertEqual(manifest['dependencies'], [
                {'asset_type': 'psx-rom', 'asset_id': 'SLUS-00005', 'destination': 'inputs/roms/game'},
                {'asset_type': 'psx-rom', 'asset_id': disc2['asset_id'],
                 'destination': 'inputs/roms/disc2'},
            ])
            self.assertNotIn('provenance', manifest['dependencies'][1])
            self.assertNotIn('inputs/roms/game', files)
            self.assertNotIn('inputs/roms/disc2', files)
            self.assertFalse(any(name.startswith('inputs/roms/disc2/') for name in files))
            question = archive_text(built['package'], 'inputs/question.md')
            self.assertIn('`psx-rom` `SLUS-00005`', question)
            self.assertIn(
                '- Additional dependency: `psx-rom` `title:Metal Gear Solid (USA) (Disc 2)`, '
                'materialized at `inputs/roms/disc2` before the first turn. '
                'Operator psx-rom loader, Disc 2 cue/bin. Not a packaged file.',
                question)
            packaged = json.loads(archive_text(built['package'], 'inputs/spec.json'))
            self.assertEqual(packaged['extra_assets'], [disc2])

            spec_path = self._spec_with(root, [dict(disc2, destination='inputs/bios')])
            with self.assertRaisesRegex(ValueError, 'overlaps dependency destination inputs/bios'):
                build_package(spec_path, root / 'overlap', fmt='v2',
                              bios=self.bios, save=self.save, repo=self.repo)

    def test_extra_assets_reject_bad_entries_and_overlapping_destinations(self):
        disc2 = {
            'asset_type': 'psx-rom',
            'asset_id': 'title:Metal Gear Solid (USA) (Disc 2)',
            'destination': 'inputs/roms/disc2',
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            loaded = package_builder.load_spec(self._spec_with(root, [disc2]))
            self.assertEqual(package_builder.dependency_entries(loaded), [
                {'asset_type': 'psx-rom', 'asset_id': 'SLUS-00005', 'destination': 'inputs/roms/game'},
                {'asset_type': 'psx-rom', 'asset_id': disc2['asset_id'],
                 'destination': 'inputs/roms/disc2'},
            ])
            self.assertEqual(package_builder.extra_dependency_note(loaded),
                             '\n- Additional dependency: `psx-rom` '
                             '`title:Metal Gear Solid (USA) (Disc 2)`, '
                             'materialized at `inputs/roms/disc2` before the first turn.')
            omitted = package_builder.load_spec(self.spec)
            self.assertNotIn('extra_assets', omitted)
            self.assertEqual(package_builder.extra_dependency_note(omitted), '')
            self.assertEqual(len(package_builder.dependency_entries(omitted)), 1)

            with self.assertRaisesRegex(ValueError, 'must be a list'):
                package_builder.load_spec(self._spec_with(root, {'asset_type': 'psx-rom'}))
            with self.assertRaisesRegex(ValueError, 'must be an object'):
                package_builder.load_spec(self._spec_with(root, ['not-an-object']))
            with self.assertRaisesRegex(ValueError, r'extra_assets\[0\]\.destination is required'):
                package_builder.load_spec(self._spec_with(root, [{
                    'asset_type': 'psx-rom',
                    'asset_id': disc2['asset_id'],
                }]))
            with self.assertRaisesRegex(ValueError, 'provenance must be a string'):
                package_builder.load_spec(self._spec_with(root, [dict(disc2, provenance=1)]))
            with self.assertRaisesRegex(ValueError, 'relative posix path'):
                package_builder.load_spec(self._spec_with(root, [dict(disc2, destination='../disc2')]))
            for destination in ('inputs/roms/game', 'inputs/roms/game/track', 'inputs/roms'):
                with self.subTest(destination=destination):
                    with self.assertRaisesRegex(ValueError, 'overlaps another dependency'):
                        package_builder.load_spec(self._spec_with(root, [dict(disc2, destination=destination)]))
            nested = dict(disc2, destination='inputs/roms/disc2/extra')
            with self.assertRaisesRegex(ValueError, 'overlaps another dependency'):
                package_builder.load_spec(self._spec_with(root, [disc2, nested]))

    def test_warped_spec_is_cheat_diff_without_offsets_or_a_save(self):
        spec = json.loads((REAL / 'research/specs/crash-bandicoot-warped-us.json').read_text())
        loaded = package_builder.load_spec(REAL / 'research/specs/crash-bandicoot-warped-us.json')
        self.assertEqual(spec['method'], 'cheat-diff')
        self.assertEqual(spec['serial'], 'SCUS-94244')
        self.assertEqual(spec['product_code'], 'SCUS-94244')
        self.assertEqual(spec['asset']['asset_id'], 'title:Crash Bandicoot: Warped (USA)')
        self.assertNotIn('cross_check', spec)
        self.assertNotIn('extra_assets', spec)
        self.assertEqual([field['id'] for field in spec['fields']], spec['fields_target'])
        for field in spec['fields']:
            self.assertIn(field['confidence'], package_builder.CONFIDENCE)
            self.assertNotIn('offset', field)
        self.assertEqual(spec['fields'][0]['confidence'], 'partial')
        self.assertEqual(spec['fields'][2]['confidence'], 'unknown')
        self.assertEqual(set(spec['check_notes']), set(spec['required_checks']))
        self.assertIn('save-gate', spec['required_checks'])
        self.assertIn('cheat-diff', spec['required_checks'])
        self.assertIn('native-load', spec['required_checks'])
        bios = {item['role']: item for item in spec['private_inputs']}
        self.assertNotIn('save', bios)
        self.assertEqual(bios['bios']['sha256'],
                         '71af94d1e47a68c11e8fdb9f8368040601514a42a5a399cda48c7d3bff1e99d3')
        self.assertIn('BASCUS-9424400000000', spec['layout']['directory_filename_lead'])
        self.assertIn('SCES-01420', spec['layout']['scope'])
        self.assertIn('SCPS-10073', spec['layout']['scope'])
        self.assertEqual(loaded['name'], 'crash-bandicoot-warped-us')
        self.assertIn('research/templates/question-cheat-diff.md', package_builder.TEMPLATE_PATHS)
        self.assertTrue((REAL / 'research/templates/question-cheat-diff.md').is_file())

    def test_cheat_diff_package_uses_the_title_disc_and_hides_the_editor_map(self):
        try:
            runner = package_builder.load_research_runner()
        except ValueError as error:
            self.skipTest(str(error))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bios = root / 'bios.bin'
            bios.write_bytes(b'warped-bios-fixture')
            spec = json.loads((REAL / 'research/specs/crash-bandicoot-warped-us.json').read_text())
            spec['private_inputs'][0]['sha256'] = sha256(bios)
            spec_path = root / 'crash-bandicoot-warped-us.json'
            spec_path.write_text(json.dumps(spec, indent=2) + '\n')
            with self.assertRaisesRegex(ValueError, 'sha256 mismatch'):
                build_package(REAL / 'research/specs/crash-bandicoot-warped-us.json', root / 'real',
                              bios=bios, repo=self.repo)
            first = build_package(spec_path, root / 'one', fmt='v2', bios=bios, repo=self.repo)
            second = build_package(spec_path, root / 'two', fmt='v2', bios=bios, repo=self.repo)
            self.assertEqual(first['sha256'], second['sha256'])
            manifest, files = runner.package.verify(first['package'])
            self.assertEqual(manifest['name'], 'crash-bandicoot-warped-us')
            self.assertEqual(manifest['dependencies'], [{
                'asset_type': 'psx-rom',
                'asset_id': 'title:Crash Bandicoot: Warped (USA)',
                'destination': 'inputs/roms/game',
            }])
            self.assertNotIn('required_checks', manifest)
            self.assertIn('inputs/bios/ps-22a.bin', files)
            self.assertFalse(any(name.endswith(('.gme', '.mcr', '.mcs')) for name in files))
            question = archive_text(first['package'], 'inputs/question.md')
            agents = archive_text(first['package'], 'AGENTS.md')
            self.assertIn(spec['no_save_note'], question)
            self.assertIn('cheat-and-diff', question)
            self.assertIn('`save-gate`:', question)
            self.assertIn('`cheat-diff`:', question)
            self.assertIn('confidence partial', question)
            self.assertIn('BASCUS-9424400000000', question)
            self.assertIn('SCUS-94900', question)
            self.assertIn('game-tools-collection', question)
            self.assertIn('CrashEdit', question)
            self.assertIn('[output-packaging contract](output-packaging.md)', question)
            self.assertNotIn('@@', question)
            self.assertNotIn('SLUS-00005', question)
            self.assertNotIn('cited editor has no checksum', question)
            self.assertIsNone(re.search(r'offset\s+0x', question))
            self.assertNotIn('`lives`: offset', question)
            self.assertIn('python3 tools/ps1_replay/host.py --engine docker ensure', agents)
            self.assertIn('python3 tools/ps1_replay/host.py --engine docker ensure', question)
            self.assertIn('create saves in the emulator', agents)
            v3 = build_package(spec_path, root / 'v3', fmt='v3', bios=bios, repo=self.repo)
            payload = envelope(v3['package'])['manifest']
            self.assertEqual(payload['required_checks'], spec['required_checks'])
            v3_question = archive_text(v3['package'], 'inputs/question.md')
            v3_agents = archive_text(v3['package'], 'AGENTS.md')
            self.assertIn('.runner/tool-images.json', v3_question)
            self.assertIn('Do not rebuild the image', v3_question)
            self.assertNotIn('host.py --engine docker ensure', v3_question)
            self.assertNotIn('host.py --engine docker ensure', v3_agents)
            self.assertIn('--image-tag', v3_question)

    def test_cheat_diff_rejects_an_editor_map_and_editor_map_keeps_its_cross_check(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = json.loads((REAL / 'research/specs/crash-bandicoot-warped-us.json').read_text())

            def write(name, value):
                path = root / name
                path.write_text(json.dumps(value) + '\n')
                return path

            offset = json.loads(json.dumps(spec))
            offset['fields'][0]['offset'] = '0x10'
            with self.assertRaisesRegex(ValueError, 'must not include offset'):
                package_builder.load_spec(write('offset.json', offset))
            missing = json.loads(json.dumps(spec))
            del missing['non_sources']
            with self.assertRaisesRegex(ValueError, 'non_sources must be a non-empty list'):
                package_builder.load_spec(write('missing.json', missing))
            crossed = json.loads(json.dumps(spec))
            crossed['cross_check'] = {
                'name': 'editor', 'url': 'https://example.com/editor', 'license': 'MIT'}
            with self.assertRaisesRegex(ValueError, 'use non_sources instead of cross_check'):
                package_builder.load_spec(write('crossed.json', crossed))
            rayman = json.loads((REAL / 'research/specs/rayman-us.json').read_text())
            self.assertNotIn('method', rayman)
            del rayman['cross_check']
            with self.assertRaisesRegex(ValueError, 'spec cross_check is required'):
                package_builder.load_spec(write('rayman.json', rayman))
            rayman = json.loads((REAL / 'research/specs/rayman-us.json').read_text())
            rayman['non_sources'] = spec['non_sources']
            with self.assertRaisesRegex(ValueError, 'only for method cheat-diff'):
                package_builder.load_spec(write('rayman-non-sources.json', rayman))
            partial = json.loads(json.dumps(spec))
            partial['check_notes']['tool-image-provisioned'] = {'v2': 'build with ensure'}
            with self.assertRaisesRegex(ValueError, 'format map must contain v2 and v3'):
                package_builder.load_spec(write('partial-note.json', partial))


if __name__ == '__main__':
    unittest.main()
