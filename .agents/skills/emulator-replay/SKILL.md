---
name: emulator-replay
description: "Build, debug, and qualify reproducible emulator automation using scripted controller input, emulated-frame checkpoints, native save states, and retained evidence. Use for unattended game navigation, fresh-process replay, black-screen or restore diagnostics, and isolated Linux emulator workers. Includes PCSX-Redux guidance and a verified PS1 case study. Ordinary gameplay and save-format decoding alone do not need this workflow."
---

# Emulator Replay Qualification

Turn a game interaction into a reproducible, bounded integration test. Separate successful automation from proof that it reached the intended scene.

## Research from a repository clone

When the checkout contains `tools/ps1_replay/host.py`, first read [the repository research workflow](references/repository-research.md). Use the checked-in harness and explicit input paths to run the research in a Linux worker. A clone supplies instructions and tooling; it does not supply private assets, an engine, or a qualified baseline. The repository worker supports Docker CE and Podman through an explicit `--engine` option; keep the same selection for build, launch and commands. Keep this routing conditional so the skill remains usable for other emulators and repositories.

## Establish the contract

Inspect existing tooling and evidence before building another harness. Identify the emulator, host/guest architecture, runtime, required assets, region/revision, selected save, expected visible scene, stable values, and requested scope. Choose explicit trial count, frame checkpoints, and wall-clock deadlines; three fresh-process trials and five minutes per trial are useful pilot defaults when the task leaves them open.

Distinguish input/replay, persistent saving, parallel isolation, and rendering without a virtual display. A pass in one scope does not qualify the others. Preserve the application's interfaces when the task is research tooling.

For PCSX-Redux, read [the Linux adapter notes](references/pcsx-redux-linux.md). Consult [the Legends 2 case](references/legends2-case.md) only for that title or a comparable diagnostic problem; its build, addresses, startup counts, and card preparation are case-specific.

## Isolate and pin

- Hash original BIOS, disc files including cue dependencies, and card/save inputs. Mount originals read-only; use separate writable configuration and disposable save media per worker. Keep private assets, states, RAM, screenshots, and download caches outside version control and container images.
- The worker identity is the tool-image input hash plus the runtime image ID recorded when the image was provisioned. Pin the official executable artifact checksum, reported version/changeset, architecture, effective settings, and launch arguments. A base-image digest alone does not pin packages resolved during a rebuild. Verify the actual running binary and renderer. A missing image is an error to provision from the packaged Containerfile (`host.py ensure` on a v2 package, or the runner tool-image receipt on v3), not a step that can be skipped. Do not pin a pre-existing image ID in place of the input hash.
- For a local Linux pilot, prefer one native-architecture worker, a virtual display, software rendering, dummy audio, disabled shaders/updates, explicit resource limits, and container-local control transport without published ports. Adapt these to the emulator and requested environment; record deviations.

## Qualify in gates

Progress through rendering, BIOS/disc boot, save/scene loading, controller input and frame timing, capture, state restoration, then repeatability. Record the first failing gate. Retain a failed attempt while diagnosing and retrying within the user's scope; create new evidence for changed conditions.

1. **Boot visibly.** Capture stages between boot and loaded gameplay. Confirm the BIOS was actually recognized and required files opened. Correct save values in RAM do not prove playable gameplay. Distinguish a framebuffer not yet initialized from a black frame after expected gameplay. Reject black or empty captures when the target should be visible.
2. **Calibrate navigation.** Use the adapter for small press/release steps with captures and RAM reads between them. Discover the actual menu button rather than assuming conventional controls. Calibration may compare settings/builds or run continuously to diagnose a stall; it is separate from qualification.
3. **Measure frame semantics.** Subscribe to the emulator's guest-vsync or equivalent event. Qualify short and longer waits, pause timing, and release timing before relying on them. Record requested and actual counts; fail on overshoot or stalls. UI FPS, polling cycles, and wall-clock sleeps are not emulated-frame counts. Execute complex operations in the emulator's documented safe context and retain event subscriptions.
4. **Freeze a baseline.** Verify the intended visible gameplay and stable values, release all buttons, pause, and create a native emulator state. Pair it with the exact disposable card/disk. Bind the state, card, capture, inputs, build, image, settings, adapter, and versioned recipe to hashes. Record the baseline frame and visual observation.
5. **Restore in a fresh process.** Test restoration before the full replay. Loading correct RAM is insufficient if execution then faults. If a startup warmup solves a reproducible cold-restore failure, measure and freeze that initialization separately from post-restore frames. Do not assume another emulator's state format is compatible or hide a guest-memory patch in the replay.
6. **Replay sequentially.** Each fresh process receives the matching baseline media, restores the state, resets or offsets the measured frame counter, executes the frozen recipe, pauses at the same checkpoint, and captures framebuffer plus relevant RAM. Release overrides even on timeout or error.

Changing build, settings, recipe, adapter timing, or input assets invalidates the previous qualification; capture/review a new baseline and repeat the affected gates.

## Inspect discrepancies

Compare raw framebuffer hashes as well as stable RAM and visible labels/gear. Identical black screens can be consistently wrong. Bind visual review to exact capture hashes; inspect every final trial. A mismatch stays unresolved until inspected and explained. If tolerance is appropriate for an intentionally nondeterministic game, define a new acceptance contract explicitly rather than silently weakening an exact-match test.

For startup card-preservation failures in a readiness pilot, read [the startup preservation protocol](references/startup-preservation.md). Retain the strict failed probe and diagnose within the requested pilot: boot-only evidence may justify freezing an observed initialized disposable card before measured replay. Keep originals unchanged and require exact whole-card equality afterward, including scratch bytes. Unexplained writes still fail the affected gate; report independent readiness gates separately. Do not infer save/load persistence or generalize a case-specific preparation.

## Evidence and failure handling

Retain a machine-readable verdict and manifest with provenance hashes, effective settings, baseline state/media, raw frames and viewable screenshots, RAM values/ranges, recipe/action logs, requested/actual frames, elapsed times, failing step, and cleanup outcomes. Keep calibration, diagnostics, mechanical completion, and reviewed qualification distinct.

Local `tools/ps1_replay/runs/` directories stay unpacked. When publishing evidence as a Research Runner job, follow the [output-packaging contract](../../../research/output-packaging.md). Retain the replay verdict, captures, RAM, action logs and cleanup evidence required above.

Use bounded per-action and overall deadlines. On failure, attempt acknowledged release of every controller override, retain available evidence, and stop only owned emulator processes. Preserve the original failure if cleanup or integrity checks also fail. Restore a VM's initial running/stopped state when the task started it temporarily.

Test coordinator handling of missing inputs, early emulator exit, stalled actions, frame overshoot, failed transport, deadline expiry, and button cleanup with simulated responses. Also test baseline tampering and evidence mismatches where relevant. These tests do not replace the local-assets integration run.

Report the qualified scope, exact build/configuration, trial results, asset preservation, evidence location, and unresolved causes. Describe a workaround as measured for its tested environment; do not claim an upstream fix from a successful version comparison.
