# Research from a memcard-manager clone

Use this route when the checkout contains `tools/ps1_replay/host.py`. Work from the repository root; read `tools/ps1_replay/README.md` for executable build/bootstrap/qualification commands. The cloned `.agents/skills/emulator-replay/` is the repository skill source. No user-level installation, developer home directory, desktop app build, or prior local evidence is required.

## Establish the environment and question

Identify the research question, title/region/revision, supplied assets, and intended evidence before launching anything. For example: use the known US Legends 2 position-1 save to verify Equipment input/replay, or investigate a specific saved field. A request to research a field does not automatically require the entire three-trial replay suite; use the qualification gates relevant to the evidence being claimed.

The existing executable example is a Legends 2-specific pilot, not a generic loader for arbitrary discs. Its `recipe.json`, adapter RAM addresses, input validation, `tools/inspect_mml2.py` decoder, baseline checks, and review labels are coupled to US `SLUS-01140` and `BASLUS-01140-DASH20`. For another game, adapt those pieces as research tooling, calibrate from visible gameplay, and create a new recipe/baseline. Do not reuse its game-specific card preparation without format evidence.

The pinned worker is native Linux ARM64. Check the execution environment and Docker or Podman engine architecture before building. On Linux, use the available local engine; on macOS, inspect the existing ARM64 Podman VM and start it if needed, recording its initial state. An AMD64 worker or emulated ARM64 run needs its own build/configuration and qualification; the historical result does not cover it.

## Host and worker boundary

`host.py` builds the image and launches the worker. Select Docker with `python3 tools/ps1_replay/host.py --engine docker preflight`, followed by `--engine docker build` and the same option for every launch/command. Podman remains the default. Preflight checks daemon access and ARM64 architecture; it does not validate input visibility or game boot. Docker command handling is unit-tested, while the historical emulator qualification used Podman. DGX Spark is an ARM64 target but still needs a fresh boot/baseline check. Local Docker images may have no registry digest; record the immutable image ID. It needs Python 3 and an accessible Docker CE or Podman engine. The Python coordinator, Lua adapter, PCSX-Redux, Xvfb, rendering, input, and RAM capture execute inside the worker. Original assets and code are mounted read-only; the run directory is mounted writable. Network access is needed during the image/artifact build, while measured workers use `--network none` with no published ports.

If Codex itself is running in another container, check engine access and bind-mount visibility before launching a nested worker. Bind-mount paths passed to the selected engine must be visible to the engine, not merely present in the agent's private filesystem. Cloning into an ordinary container does not install an engine or grant nested-container capability. Report a missing engine, incompatible architecture, or inaccessible input as a concrete environment gate; do not claim the game failed to boot without having launched it. Do not silently switch to desktop focus or keyboard automation to work around missing infrastructure.

## Inputs and retained evidence

A fresh clone has no private BIOS, extracted cue/bin directory, known QA card, baseline state, or trial captures. Use the user's supplied paths with `--bios`, `--disc`, and `--card` on every worker launch. The launcher requires all three explicitly, with no author-machine asset defaults. Missing assets should be identified precisely; do not substitute an unrelated card or another save/release.

For the existing example, `--disc` points to a directory with exactly one cue and its bin dependencies; `--card` is the full raw 128-KiB QA image containing the known save. Confirm input identifiers/checksums with the existing reader. Keep originals read-only and outside Git and the image. Use a new ignored `tools/ps1_replay/runs/<experiment>/` directory and preserve it outside the disposable worker.

A fresh environment must bootstrap its own paused gameplay state/card, inspect the captured scene, and record that observation before qualification. Use the same explicit inputs, pinned image, and frozen recipe for subsequent trials. Inspect all final captures before recording a pass. The historical report is a case study, not evidence for a new run.

## Readiness pilots and preservation failures

For a request to test the setup, distinguish bundle integrity, engine access,
boot, load, input/replay and preservation. Retain a failed startup probe and use
bounded diagnosis within the requested pilot rather than treating it as the end
of all readiness work. Read [the startup preservation protocol](startup-preservation.md)
for reproducible boot-only scratch writes and a frozen initialized disposable
baseline. Exact whole-card equality remains required during measured replay.
The existing Legends 2 preparation does not certify another game's startup.

## Cheat-and-diff saves

Guest RAM pokes stay off unless `calibrate` or `diagnose` is started with `--guest-writes`. PCSX-Redux has no `.cht` loader; send one `poke` at a time, with `--sticky` when the code has to be rewritten every guest vsync. Before poking, confirm the current scene can reach a save (menu or save point). Keep one control save from a separate `--guest-writes` session that never issues `poke`, so the game's card write is tagged `control`. The harness does not patch the memory-card file; the game writes the save, and a card produced after a landed poke is tagged cheat-derived. Commands, GameShark mapping, and the evidence file are in `tools/ps1_replay/README.md`.

## Research output

Retain the complete private evidence bundle and report the requested finding with its exact game/build/input scope. Version the reusable recipe, harness changes, and a concise report when requested; exclude asset bytes, RAM, states, screenshots, and local caches. A production reader change should follow from verified field evidence and its own parser tests rather than automatically shipping every experimental observation.

Coordinator failure tests run with `python3 -m unittest discover -s tools/ps1_replay -p 'test_*.py' -v`, using only Python 3 and no private assets or engine. Passing them establishes harness behavior, not emulator compatibility. Once a worker exits, release/cleanup evidence should show what happened; stop only task-owned processes and restore any temporarily started VM's initial state.
