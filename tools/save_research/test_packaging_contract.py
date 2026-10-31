"""One source policy, portable job links, and generated manual starter copies."""
import json
from pathlib import Path
import re
import shutil
import tempfile
import unittest

from packaging_contract import CONTRACT_INPUT, CONTRACT_SOURCE, REPO, sync_definition


class ContractTests(unittest.TestCase):
    def test_source_retains_publication_requirements(self):
        text = (REPO / CONTRACT_SOURCE).read_text()
        for required in ('4,096 output entries (files and directories)',
                         '32 MiB per published file', '128 MiB per generation',
                         'separate 512-entry admission limit', 'final compressed',
                         'Runner receipts', 'sharding does not reduce that total',
                         'outputs/handoff.md', 'outputs/research.json', 'outputs/result.json',
                         'outputs/checkpoint.json', 'outputs/tool-image.json',
                         'outputs/field-ledger.md', 'outputs/decoder/', 'outputs/fixtures/',
                         'outputs/evidence-screenshots.tar.gz', 'outputs/evidence-calibration.tar.gz',
                         'outputs/evidence-actions.tar.gz', 'INDEX.txt',
                         'tar --sort=name --mtime=@0', 'gzip -n', 'Cite each archive once'):
            self.assertIn(required, text)

    def test_checked_in_starter_is_current_and_required_outputs_stay_a_trio(self):
        definition = REPO / 'research/ps1-summary'
        sync_definition(definition, check=True)
        manifest = json.loads((definition / 'manifest.json').read_text())
        self.assertEqual(manifest['inputs'].count(CONTRACT_INPUT), 1)
        self.assertEqual(set(manifest['required_outputs']), {
            'outputs/handoff.md', 'outputs/research.json', 'outputs/result.json'})
        self.assertEqual((definition / CONTRACT_INPUT).read_bytes(), (REPO / CONTRACT_SOURCE).read_bytes())

    def test_repo_skill_template_and_starter_links_resolve(self):
        paths = [REPO / 'AGENTS.md', REPO / 'research/README.md']
        paths += list((REPO / '.agents/skills').rglob('*.md'))
        paths += list((REPO / 'research/templates').rglob('*.md'))
        paths += list((REPO / 'research/ps1-summary').rglob('*.md'))
        expected = {(REPO / CONTRACT_SOURCE).resolve(),
                    (REPO / 'research/ps1-summary' / CONTRACT_INPUT).resolve()}
        linked = set()
        for path in paths:
            if path.resolve() in expected:
                continue
            text = path.read_text()
            for target in re.findall(r'\[[^\]]+\]\(([^)\s]*output-packaging\.md)\)', text):
                self.assertIn((path.parent / target).resolve(), expected, path)
                self.assertTrue((path.parent / target).is_file(), path)
                linked.add(path)
            self.assertNotIn('4,096', text, path)
            self.assertNotIn('32 MiB', text, path)
        self.assertTrue(set(paths[:2]).issubset(linked))
        for directory in ('research/templates', 'research/ps1-summary'):
            self.assertIn(REPO / directory / 'AGENTS.md', linked)
            self.assertIn(REPO / directory / 'brief.md', linked)
        for skill in ('research-runner', 'ps1-save-research', 'emulator-replay'):
            self.assertIn(REPO / '.agents/skills' / skill / 'SKILL.md', linked)

    def test_install_in_manual_definition_is_idempotent_and_check_detects_drift(self):
        with tempfile.TemporaryDirectory() as tmp:
            definition = Path(tmp) / 'definition'
            shutil.copytree(REPO / 'research/ps1-summary', definition)
            manifest_path = definition / 'manifest.json'
            manifest = json.loads(manifest_path.read_text())
            manifest['inputs'].remove(CONTRACT_INPUT)
            manifest_path.write_text(json.dumps(manifest))
            (definition / CONTRACT_INPUT).unlink()
            # Runner can use nested control files; all installed links must work.
            (definition / 'nested').mkdir()
            (definition / 'AGENTS.md').rename(definition / 'nested/AGENTS.md')
            manifest['agents'] = 'nested/AGENTS.md'
            manifest_path.write_text(json.dumps(manifest))
            (definition / 'brief.md').write_text('A new manual question.\n')
            before = manifest_path.read_bytes()
            with self.assertRaisesRegex(ValueError, 'needs refresh'):
                sync_definition(definition, check=True)
            self.assertEqual(manifest_path.read_bytes(), before)
            self.assertFalse((definition / CONTRACT_INPUT).exists())
            sync_definition(definition)
            sync_definition(definition, check=True)
            self.assertIn('](../inputs/output-packaging.md)',
                          (definition / 'nested/AGENTS.md').read_text())
            self.assertIn('](inputs/output-packaging.md)', (definition / 'brief.md').read_text())
            first = {p.relative_to(definition): p.read_bytes()
                     for p in definition.rglob('*') if p.is_file()}
            sync_definition(definition)
            self.assertEqual(first, {p.relative_to(definition): p.read_bytes()
                                     for p in definition.rglob('*') if p.is_file()})
            source = Path(tmp) / CONTRACT_SOURCE
            source.parent.mkdir()
            source.write_text('Updated authoritative policy.\n')
            with self.assertRaisesRegex(ValueError, 'needs refresh.*output-packaging'):
                sync_definition(definition, repo=Path(tmp), check=True)
            sync_definition(definition, repo=Path(tmp))
            self.assertEqual((definition / CONTRACT_INPUT).read_bytes(), source.read_bytes())

    def test_runner_packages_manual_starter_with_the_contract(self):
        # Use the pinned installed wheel, as the standalone assembler does.
        from assemble_package import load_research_runner
        package = load_research_runner().package
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / 'manual.rrpkg'
            package.assemble(REPO / 'research/ps1-summary', archive)
            manifest, inventory = package.verify(archive)
            self.assertIn(CONTRACT_INPUT, manifest['inputs'])
            self.assertIn(CONTRACT_INPUT, inventory)


if __name__ == '__main__':
    unittest.main()
