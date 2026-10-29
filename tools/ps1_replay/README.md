# Linux Legends 2 input/replay harness

Research-only tooling, separate from the desktop app. One PCSX-Redux process per
trial, driven by Lua controller overrides and guest-vsync events. It never
patches guest memory or saves through the game. Originals are mounted read-only;
each worker receives a disposable card and separate portable profile.

## First result

The 10 October 2026 test **passed three fresh-process Equipment replays** on
official ARM64 build 70 (`d42cdae5`). Each reached relative frame 288 with
identical framebuffer hashes and matching known-save RAM. All originals,
prepared disposable cards and baseline artifacts remained unchanged.
That result is historical. `build-lock.json` and `recipe.json` now pin build
206 (`85baf2cd`, archive sha256
`14b1f77c98ee238c32463c96b21b91e1e8a45cc26ec8f48706da0f54ea0fb5da`).
Build 206 is the artifact `host.py` downloads; it is not a new qualification
of the Legends 2 recipe.
See [the test report](../../docs/research/linux-replay-first-test.md).
The reusable [emulator-replay skill](../../.agents/skills/emulator-replay/SKILL.md)
captures the qualification workflow with separate Redux notes and a Legends 2
case study. Codex discovers it from the repository's `.agents/skills/` directory
after cloning; no separate user-level skill installation is required.

Build 79 (`6eda90ff`) has a reproduced black-screen loading regression in this
setup. Build 70 also needs its fixed 663-frame BIOS startup before state restore;
cold restore faults. Each trial performs that startup, then restores the exact
state/card pair and resets the relative frame count. `recipe.json` version 2
records the working boot sequence and Select/Down/Down/Cross Equipment recipe.

Before measured boot, the harness prepares block 0/sector 63 (the card write-test
frame) in the disposable copy by mirroring sector 0. Only the known zero-filled
or already initialized frame is accepted. The source is unchanged; directories
and save blocks are unchanged. `card-initial.json` records source/initial hashes
and exact preparation offsets. This avoids incidental BIOS scratch writes
changing the measured card image. Trials use exact copies of the initialized
baseline card, with no additional preparation.

## Readiness pilots and startup card writes

A request to check that the setup works includes bounded diagnosis of a failed
startup gate within the requested pilot. Retain the strict failure and continue
independent readiness checks; a reproducible write confined to the card's
write-test frame is a candidate initialization step, not an automatic blocker
for the entire task. It is not a passing preservation result either.

For a new game or card, follow the skill's
[startup preservation protocol](../../.agents/skills/emulator-replay/references/startup-preservation.md).
Reproduce the write during boot only, record the exact before/after bytes and
hashes, verify every byte outside the scratch frame and every original is
unchanged, and freeze the observed initialized disposable card before measured
replay. Subsequent runs still require exact whole-card equality. The Legends 2
three-byte preparation above is not a generic rule for other cards or games.

Suggested prompt for a single readiness pilot:

> Validate the bundle, engine access and one game's boot/load/input/replay within
> a single pilot. Retain failures and diagnose startup issues within that scope.
> If boot alone reproducibly changes only the documented scratch frame, record
> and freeze the observed initialized disposable card, then repeat replay with
> exact whole-card preservation. Keep originals unchanged and report each gate
> separately. Unexplained writes remain blockers for the affected gate. Do not
> launch the overnight batch.

## Build and reproduce

Start from a clone and invoke `$emulator-replay` with the requested game/save,
research question, and supplied asset paths. See the skill's
[repository research workflow](../../.agents/skills/emulator-replay/references/repository-research.md)
for the host/worker boundary and container preflight.

Requires Python 3 on the coordinator host and Docker CE or Podman backed by native
Linux ARM64. Select Docker with `host.py --engine docker SUBCOMMAND`; the default
remains Podman. Use the same engine for build, worker launch and commands.
On Linux this can be a local engine; on macOS use a Linux ARM64 Podman VM.
An agent inside another container also needs an accessible engine with access
to the mount paths. An ordinary container without an engine cannot launch this
worker. The pinned build and recipe have not been qualified on AMD64 or under
cross-architecture emulation.

For a DGX Spark with Docker CE, begin with:

```sh
python3 tools/ps1_replay/host.py --engine docker preflight
python3 tools/ps1_replay/host.py --engine docker ensure
```

`ensure` is idempotent. It builds the worker only when the input-hash tag is
missing or the image label `research.input_hash` does not match the packaged
Containerfile, build lock, runner, and adapter. A missing image is an error on
later launch modes: `worker image <tag> missing; run host.py ensure`. `build`
is the same provisioning step. `preflight` checks engine access, daemon
architecture, and `image_present`. It does not check game boot or mount
visibility. Docker command construction/metadata handling is regression-tested;
the historical emulator pass used Podman. Bootstrap and inspect gameplay on the
Spark before unattended research. No NVIDIA GPU access is required by this
software-rendered worker. The worker identity is that input hash plus the image
ID `ensure` prints. A previously recorded image ID is not portable: apt packages
inside the image are unpinned, so rebuilds are tagged by input hash and the
resulting ID is evidence. `--image-tag` selects a tag the runner already
provisioned. The legacy `localhost/ps1-replay:<buildId>` tag is applied as an
alias when the image is built.

For unattended Docker research, prefer a rootless daemon under a dedicated
research account without sudo access. The coordinator inherits the Docker CLI
context and environment: select the rootless context before preflight/build and
keep it selected for every launch and command. Existing images in a different
daemon must be rebuilt or explicitly imported into this daemon.

Check `docker info` for the `rootless` security option, cgroup v2 and the systemd
cgroup driver. The worker requests four CPUs and 3 GB of memory; rootless Docker
requires the corresponding cgroup controllers to be delegated for those limits
to work. Architecture preflight does not certify resource enforcement. Verify
the limits and writable run mount during the server pilot before overnight work.
See Docker's [rootless setup](https://docs.docker.com/engine/security/rootless/)
and [resource-limit requirements](https://docs.docker.com/engine/security/rootless/tips/#limiting-resources).
Do not grant unrestricted access to the root daemon through socket permissions
or passwordless `sudo docker` as a substitute for this boundary. Engine access
belongs to the coordinator; never mount its socket into a research worker.

Every worker launch requires explicit `--bios`, `--disc` and `--card` paths.
`--disc` is a directory
containing exactly one cue and its extracted bin dependencies; `--card` is the
whole-card QA image containing the known position-1 save, not an individual MCS.
ROMs, BIOS, cards, downloaded binaries, states, RAM and screenshots are excluded
from Git and are never included in the container image.

```sh
git clone https://github.com/bakkerme/memcard-manager.git
cd memcard-manager
python3 -m unittest discover -s tools/ps1_replay -p 'test_*.py' -v
# On macOS, inspect/start the existing ARM64 Podman VM if stopped.
podman info
PS1_RESEARCH_BIOS=/absolute/path/ps-22a.bin
PS1_RESEARCH_DISC='/absolute/path/Mega Man Legends 2 (USA)'
PS1_RESEARCH_CARD=/absolute/path/mml2-viewer-qa.mcr
PS1_RESEARCH_RUN=tools/ps1_replay/runs/reproduction
python3 tools/ps1_replay/host.py ensure
python3 tools/ps1_replay/host.py bootstrap --run "$PS1_RESEARCH_RUN" \
  --bios "$PS1_RESEARCH_BIOS" --disc "$PS1_RESEARCH_DISC" --card "$PS1_RESEARCH_CARD"
```

Build needs network access to obtain the locked official emulator artifact and
container dependencies. Measured workers run with networking disabled. A fresh
clone must create and visually verify its own gameplay baseline; the report's
historical result is not a passing verdict for a new environment.

Inspect `$PS1_RESEARCH_RUN/baseline/gameplay.png`, then record the observation and
run qualification using the same inputs and image:

```sh
python3 tools/ps1_replay/review.py baseline --run "$PS1_RESEARCH_RUN" --reviewer NAME --note 'Observed the known Nino Pad ship-deck scene'
python3 tools/ps1_replay/host.py qualify --run "$PS1_RESEARCH_RUN" \
  --bios "$PS1_RESEARCH_BIOS" --disc "$PS1_RESEARCH_DISC" --card "$PS1_RESEARCH_CARD"
```

Inspect all three `$PS1_RESEARCH_RUN/trial-N/equipment.png` captures, then record
the final review:

```sh
python3 tools/ps1_replay/review.py equipment --run "$PS1_RESEARCH_RUN" --reviewer NAME --note 'Equipment and known gear visibly match in all three captures'
```

Read `$PS1_RESEARCH_RUN/result.json` and retain the complete ignored evidence
directory outside the disposable worker. Each rerun needs a new directory;
existing trial evidence is never overwritten. If a VM was started for this run,
restore its initial state after the owned worker has exited.

The downloader uses the immutable artifact URL and SHA-256 in `build-lock.json`.
The image verifies the archive again and extracts the AppImage without FUSE.
`container.json` records the actual image ID/digest and resource limits. Reusing
that image ID preserves its installed runtime dependencies; rebuilding can
resolve newer Ubuntu package versions even though the base-image digest is fixed.

The worker runs under Xvfb/llvmpipe, with four CPU threads, a 3-GiB memory limit,
dummy audio, interpreter CPU, disabled shaders/updates/filtering, and fastboot.
The initial build-79 dynamic-recompiler attempt lost its HTTP connection during boot;
that observation does not establish the underlying cause. All runtime HTTP
traffic stays on container loopback, with `--network none` and no published ports.

## Calibration

Start with a new empty run directory:

```sh
python3 tools/ps1_replay/host.py calibrate --run tools/ps1_replay/runs/calibration \
  --bios "$PS1_RESEARCH_BIOS" --disc "$PS1_RESEARCH_DISC" --card "$PS1_RESEARCH_CARD"
```

Supply the same explicit assets to every `calibrate`, `bootstrap`, `qualify`,
or `diagnose` invocation, using the variables set in the example above.

In another terminal, after `CALIBRATION_READY`, use the same adapter:

```sh
python3 tools/ps1_replay/host.py command frames --frames 120
python3 tools/ps1_replay/host.py command press --button START --frames 6
python3 tools/ps1_replay/host.py command capture --name menu
python3 tools/ps1_replay/host.py command values
python3 tools/ps1_replay/host.py command memory --address 0x8008c0b0 --length 0x300 --name player
```

Every press releases overrides at its final guest vsync. Frame actions fail if
their actual count differs from the requested count. An action has a 60-second
deadline and a trial has a five-minute deadline, including emulator startup.
Interactive calibration has a separate fifteen-minute limit. `PS1_REPLAY_BASELINE_SECONDS`
overrides the trial and CLI budget (default 300); `PS1_REPLAY_SESSION_SECONDS` overrides the
calibration, bootstrap, and diagnose budget (default 900). When either variable is set on
the host, `run` and `exec` forward it into the container; an unset variable keeps that
default. Errors attempt an
acknowledged emergency release, then shut down the owned emulator process;
`cleanup.json` records release acknowledgement and process exit; a null
acknowledgement means the emulator had already exited before cleanup.

The saved recipe is calibrated. If changing a build or action sequence, first
verify visible Nino Pad gameplay and Equipment, then freeze
the exact `boot` and `equipment` steps in `recipe.json`, set `status` to
`calibrated`, return to gameplay, and capture the baseline:

```sh
python3 tools/ps1_replay/host.py command baseline
python3 tools/ps1_replay/host.py command quit
python3 tools/ps1_replay/review.py baseline --run tools/ps1_replay/runs/calibration --reviewer NAME --note 'Observed Nino Pad gameplay and known gear'
python3 tools/ps1_replay/host.py qualify --run tools/ps1_replay/runs/calibration \
  --bios "$PS1_RESEARCH_BIOS" --disc "$PS1_RESEARCH_DISC" --card "$PS1_RESEARCH_CARD"
```

Inspect `baseline/gameplay.png` before recording its review. A uniformly black
baseline is rejected automatically. Qualification also rejects stale baseline,
recipe/build hashes, or visual-review hashes. Each trial starts a fresh emulator,
copies the matching baseline card, restores the Redux state, executes Equipment
steps and retains per-step screenshots, relevant RAM, logs and final hashes.
Matching framebuffer hashes are required; differing hashes leave a failed result.
Baseline build/recipe/input/image hashes and a reviewed gameplay capture must
match before qualification. Every fresh process runs the same 663-frame BIOS
startup before state restoration.

Inspect **all three** `trial-N/equipment.png` captures for the Equipment label and
known gear before recording the final review:

```sh
python3 tools/ps1_replay/review.py equipment --run tools/ps1_replay/runs/calibration --reviewer NAME --note 'Equipment and known gear visibly match in all three captures'
```

Only that reviewed outcome writes `result.json` with `status: passed`. Successful
mechanical replay writes `qualification.json` with `awaiting-visual-review`.
A calibration session ending cleanly means the session completed, not that the
integration test passed. Failure reviews bind an observed gate failure to the
captured evidence; automatic startup/bootstrap failures write a failed result.

## Diagnostics

Calibration-only commands include `resume`, `release`, `diagnostics --name NAME`,
`reset_normal`, `reset_fast`, and `restore_saved --name NAME`. `resume` is an
unbounded diagnostic run within the coordinator's overall deadline; it is
rejected in a qualification recipe. Use `release` to stop it and clear buttons.
The `diagnostics` command saves CPU registers and the Redux state schema without
patching memory. Use distinct names for captures and register dumps.

To reopen a paired failed state/card for diagnosis:

```sh
python3 tools/ps1_replay/host.py diagnose --run tools/ps1_replay/runs/diagnostic \
  --state /absolute/path/state.bin --card /absolute/path/card.mcr \
  --bios "$PS1_RESEARCH_BIOS" --disc "$PS1_RESEARCH_DISC"
```

This mode restores a cold emulator intentionally so startup/restoration failures
can be compared. Diagnostic results never count as qualification.

## Tests

```sh
python3 -m unittest discover -s tools/ps1_replay -p 'test_*.py' -v
```

These simulated coordinator tests cover missing inputs, emulator exit, stalled
actions, frame overshoot, transport failure, deadlines, cleanup and value checks.
They do not qualify real emulator compatibility or hardware behavior.

Primary API references: [CLI flags](https://pcsx-redux.consoledev.net/cli_flags/),
[controller/state/capture](https://pcsx-redux.consoledev.net/Lua/redux-basics/),
[events](https://pcsx-redux.consoledev.net/Lua/events/),
[Lua HTTP handlers](https://pcsx-redux.consoledev.net/Lua/web-server/).
