# Linux Legends 2 input/replay harness

Research-only tooling, separate from the desktop app. One PCSX-Redux process per
trial, driven by Lua controller overrides and guest-vsync events. It never
patches guest memory or saves through the game. Originals are mounted read-only;
each worker receives a disposable card and separate portable profile.

## First result

The 10 October 2026 test **passed three fresh-process Equipment replays** on
pinned official ARM64 build 70 (`d42cdae5`). Each reached relative frame 288 with
identical framebuffer hashes and matching known-save RAM. All originals,
prepared disposable cards and baseline artifacts remained unchanged.
See [the test report](../../docs/research/linux-replay-first-test.md).
The reusable [emulator-replay skill](../../skills/emulator-replay/SKILL.md)
captures the qualification workflow with separate Redux notes and a Legends 2
case study. Its directory can be copied into a Codex skills directory.

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

## Build and reproduce

Requires local Podman with a running Linux ARM64 VM. Default asset paths match
this project's private fixtures and the existing DuckStation BIOS directory.
ROMs, BIOS, cards, downloaded binaries, states, RAM and screenshots are excluded
from Git and are never included in the container image.

```sh
podman machine start
python3 tools/ps1_replay/host.py build
python3 tools/ps1_replay/host.py bootstrap --run tools/ps1_replay/runs/reproduction
```

Inspect `reproduction/baseline/gameplay.png`, then record the observation and
run qualification:

```sh
python3 tools/ps1_replay/review.py baseline --run tools/ps1_replay/runs/reproduction --reviewer NAME --note 'Observed the known Nino Pad ship-deck scene'
python3 tools/ps1_replay/host.py qualify --run tools/ps1_replay/runs/reproduction
```

Inspect all three Equipment captures before writing the passing verdict with
`review.py equipment` as shown below. Each rerun needs a new directory; existing
trial evidence is never overwritten.

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
python3 tools/ps1_replay/host.py calibrate --run tools/ps1_replay/runs/calibration
```

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
Interactive calibration has a separate fifteen-minute limit. Errors attempt an
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
python3 tools/ps1_replay/host.py qualify --run tools/ps1_replay/runs/calibration
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
python3 tools/ps1_replay/host.py diagnose --run tools/ps1_replay/runs/diagnostic --state /absolute/path/state.bin --card /absolute/path/card.mcr
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
