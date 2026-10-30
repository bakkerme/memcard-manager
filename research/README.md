# Research Runner in memcard-viewer

The project consumes the published **Research Runner 0.4.0** wheel. It needs
Python 3.11+; no sibling Runner checkout, local Codex, Docker or desktop build.
0.4.0 is the runner deployed for current packages: it accepts v2 manifests
with asset dependencies and does not provision tool images. `--format v3` is
for a later runner that understands `tool_images`.

## Setup

From the repository root, using Python 3.11 or newer (POSIX shell):

```sh
python3 -m venv .research-runner/venv
source .research-runner/venv/bin/activate
python -m pip install -r requirements-research.txt
cp -n research-runner.example.toml research-runner.toml
# Edit research-runner.toml to set your service URL before querying the server.
research-runner --version
research-runner docs --output .research-runner/docs
research-runner install-skill
research-runner capabilities
```

On Windows PowerShell, create the environment with
`py -3 -m venv .research-runner/venv` (with Python 3.11+ installed), activate it
with `.\.research-runner\venv\Scripts\Activate.ps1`, and copy the example using
`Copy-Item research-runner.example.toml research-runner.toml` only if the local
config does not exist. The remaining Python/Runner commands are the same.

The service URL lives in ignored `research-runner.toml`. The checked-in example
uses loopback for illustration; it does not launch a server. Set the URL of a
reachable service, which may run on a different host. Existing operators should
keep their local config: changing the URL can invalidate stored run identities.
If authentication is enabled, set `RESEARCH_RUNNER_TOKEN` in your environment,
never in a file in a definition.

Version control the example config, requirements, bundled skill and portable definition metadata.
Private inputs are ignored by `research/.gitignore`; **ignored files are still
packaged**. Never put credentials or unrelated private files in a definition.
Generated packages, runs, receipts, cursors, docs and results stay in ignored
`.research-runner/`. Preserve that directory for interrupted-operation recovery.

## Define research

`ps1-summary` is an unfilled project template, not a research job ready to start.
For a new question:

```sh
research-runner new my-game
```

Edit `research/my-game/inputs/question.md`, `brief.md`, `AGENTS.md` and
`manifest.json`. Copy the PS1 constraints from `research/ps1-summary/AGENTS.md`.
Set the exact release, small summary target, acceptance criteria and budget.
Place every required save, reader/helper, spec and evidence file in `inputs/`,
with relative paths and a size/hash/provenance ledger. Include BIOS and the entire
matching disc/CUE set when required for runtime research. Copy the relevant
research tools and skills into the definition too; Runner uploads only this tree.
The repo-level skill does not automatically copy tools into a job.

Review current readers/specs first: historical unfinished scopes may now be
supported. Preserve originals and independent field evidence. Static parser tests,
emulator boot, in-game save/load and semantic confidence are separate checks.
Required dependencies must be rehashed on the executing host before use, including
the tool-image input hash and the recorded runtime image ID before runtime work.
Report unavailable and oversized inputs distinctly, with path, size, SHA-256 and
affected scope.

## Assemble a game package

Game packages are generated, not hand-copied. The committed spec carries no BIOS,
disc, or save bytes. The disc is a v2 `dependencies` entry (`psx-rom`), resolved
by the operator loader the same way the Tomb Raider package fetched its disc.
BIOS is a private input: the assembler copies it only after its sha256 matches
the spec. A save is packaged only when the spec declares one. Rayman does not:
`no_save_note` tells the job to generate saves in the emulator.

```sh
# BIOS digest in research/specs/rayman-us.json is the Tomb Raider ps-22a.bin sha256.
# No RAYMAN_SAVE is required; the spec has no save input.
PS1_BIOS=/path/to/ps-22a.bin \
  python3 tools/save_research/assemble_package.py \
  --spec research/specs/rayman-us.json --format v2 --out /tmp/rayman-pkg
```

Run that twice. Both prints must show the same `sha256`. `--format v2` is what
Runner 0.4.0 can verify and start. The package contains `tools/ps1_replay/`
(Containerfile, lock, host) and `inputs/tool-images.json`. Its AGENTS.md step 0
is `python3 tools/ps1_replay/host.py --engine docker ensure`. `--format v3` adds
`tool_images`, `required_checks` and `provenance` on the manifest for a runner
that provisions the image itself; 0.4.0 rejects that format, so tonight stays v2.

The archive is byte-stable for the same spec, git commit, and private-input
bytes. A dirty worktree is refused unless `--allow-dirty` is set, which records
a hash of the diff. `--out` should sit outside the repository so the package
file itself does not dirty the next run.

## Submit, run and collect

```sh
research-runner capabilities
research-runner package my-game
# Review the archive path and SHA-256 printed above:
research-runner verify /absolute/path/printed/by/package.rrpkg
research-runner submit my-game                    # preview
research-runner submit my-game --apply            # upload only
research-runner start my-game --apply             # explicit execution
research-runner status my-game
research-runner monitor my-game --timeout 60      # read-only, when desired
research-runner pull my-game                     # preview
research-runner pull my-game --apply
research-runner results my-game
```

Submission and starting are separate decisions. Check `capabilities` for the
service's concurrency and input/output limits; when it allows one running job,
finish/collect it before starting another. Retry uncertain submit/start
responses using the same command and run name. If definition bytes change, use
`submit my-game --run my-game-second --apply`, then address `my-game-second` for
start/status/monitor/pull/results. Monitoring never starts, resumes or approves.
An interrupted pull resumes its pinned generation when repeated; keep `.partial`
files and receipts. A dashboard job can be adopted with
`research-runner attach my-existing-run REMOTE_JOB_ID` (a read-only remote lookup).

Closeout requires `outputs/result.json`, `outputs/handoff.md` and
`outputs/research.json` using the bundled `inputs/contracts.json` schemas. Those
three paths are the manifest `required_outputs`. Also publish a loose
`outputs/field-ledger.md`, `outputs/decoder/`, and a small `outputs/fixtures/`
set. Pack screenshots, raw frames, calibration dumps, and action logs into a few
archives under `outputs/` (`evidence-screenshots.tar.gz`,
`evidence-calibration.tar.gz`, `evidence-actions.tar.gz`) and cite each archive
from `research.json`. Publication accepts about 512 entries, 32 MiB per file,
and 128 MiB per generation. A job that left those captures loose wrote 655 files
and could not publish.

`results` verifies delivery and validates the research report; it never executes
reported reproduction commands. Researcher validation and independent local
reproduction remain separate facts. Review returned commands before execution.

## Migrated continuations

The locally retained complete-input packages were imported into `pacman-us`
(`SLUS-00439`) and `bomberman-us` (`SLUS-00823`). Their BIOS, entire disc/CUE sets,
fixtures, recovered readers/helpers, history and provenance retain the original
relative paths. This is a migration exception to the new `inputs/` layout because
existing tools derive their workspace root from their source location. Private
workspaces, including their briefs and manifests, are excluded from version
control. Their old host names, image digests and architecture checks are provenance
for those particular experiments, not requirements of this integration. A fresh
clone contains only the portable template; restore/import private archives into a
fresh definition before using a historical continuation. Do not package a
metadata-only copy and describe it as complete.

The import preserves the four-hour budget, 15-minute finalization reserve and
required final exports, and adds structured research reports. It keeps the old
brief/agents/manifest in `preparation/runner-migration-original/` and records
source archive hashes and file hashes in `preparation/runner-migration.json`.
It does not adopt historical remote jobs, submit, start or launch an emulator.
Before another run, review the current parser support and update the question:
these are preserved historical scopes, not a claim that all work remains needed.

To import another retained package, use its exact expected hash from the original
preparation receipt and a fresh definition name:

```sh
python tools/save_research/prepare_runner.py \
  --name another-continuation \
  --archive /path/to/complete-inputs.rrpkg \
  --sha256 EXPECTED_ARCHIVE_SHA256
```

The helper uses the installed Runner library for safe validation/extraction and
packaging. It preserves every dependency, rejects changed archives and refuses
overwrite. Packaging failure leaves the new definition for inspection; repair it
and run `research-runner package NAME`, rather than rerunning import over it.
All job identities and transport are owned by Runner. The old `--runner-repo`,
`PYTHONPATH`, handcrafted queue/receipt commands and pinned-checkout requirement
have been removed. Old run directories and packages remain unchanged. Use Runner for job transport;
no separate SSH/rsync transfer utility is required.

## Dependency and validation boundary

- Release: [v0.4.0](https://github.com/bakkerme/research-runner/releases/tag/v0.4.0).
- Source commit: `9078a12831d896bf36332ec2b35d5327656ec2c1`.
- Wheel: `research_runner-0.4.0-py3-none-any.whl`.
- SHA-256: `fdd3d7c5368d805c6e43d01dd24594ac3c1364a47e60bb563bb921030ade69ae`, checked against release `SHA256SUMS` and pinned in requirements.
- The Pac-Man and Bomberman migration above was imported with the previous 0.3.0 pin (`f77c24999d6fbbc621df7f438be77c4d2d8f2aa2`, wheel sha256 `df0369af91b549311e529ff3fd8f62a3658a3def3e9b1dd50908cd9ca2c3f88b`). New packages use 0.4.0 so disc dependencies verify.
- Server version, limits, supported execution engines and `structured_research`
  support must be discovered with `research-runner capabilities` for the configured
  service. If structured reports are not enforced there, 0.4.0 `results` still
  validates them locally. This integration does not deploy or upgrade a server.
- The producer requires no GPU or particular CPU architecture. Emulator tools are
  workload-specific: the image identity is the input hash plus the recorded image
  ID, and the Legends 2 recipe is qualified on native Linux ARM64 only.
- Local migration tests and actual package verification qualify producer behavior,
  not native Codex authentication, emulator execution or new field findings.
- No research job was submitted or started during this migration.

Full maintained docs and schemas: `.research-runner/docs/docs/client-workflow.md`
and `.research-runner/docs/schemas/` after exporting the installed documentation.
