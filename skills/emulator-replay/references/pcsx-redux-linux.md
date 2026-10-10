# PCSX-Redux Linux adapter notes

These are implementation findings from a 10 October 2026 ARM64 pilot. Verify APIs and flags against the selected build's official documentation/source before applying them to another version.

## Official sources

- [Downloads](https://github.com/grumpycoders/pcsx-redux#where), [official ARM64 development manifest](https://distrib.app/storage/manifests/pcsx-redux/dev-linux-arm64/manifest.json).
- [Controller overrides, execution, screenshots and states](https://pcsx-redux.consoledev.net/Lua/redux-basics/).
- [Events](https://pcsx-redux.consoledev.net/Lua/events/), [memory/registers](https://pcsx-redux.consoledev.net/Lua/memory-and-registers/), [file API](https://pcsx-redux.consoledev.net/Lua/file-api/).
- [Lua HTTP handlers](https://pcsx-redux.consoledev.net/Lua/web-server/), [web server](https://pcsx-redux.consoledev.net/web_server/), [CLI flags](https://pcsx-redux.consoledev.net/cli_flags/).

## Runtime

Download once from an immutable official artifact URL, verify SHA-256, then extract the AppImage using `--appimage-extract` without FUSE. Retain the archive lock and executable's actual version. Do not silently select the newest development build: diagnose compatibility with an explicit controlled build comparison when needed.

The verified pilot used Xvfb, Mesa llvmpipe, `LIBGL_ALWAYS_SOFTWARE=1`, `GALLIUM_DRIVER=llvmpipe`, `LP_NUM_THREADS=4`, and `SDL_AUDIODRIVER=dummy`. One Podman container had 4 CPUs, 3 GiB, `--network none`, no published ports, read-only input mounts, and a writable evidence/configuration directory. Resource limits and interpreter mode are pilot choices, not universal requirements.

Relevant flags included `-portable`, `-bios`, `-iso` pointing to the cue, `-memcard1`, `-memcard2`, `-noupdate`, `-noshaders`, `-fastboot`, `-no-viewports`, `-no-debugger`, `-no-gdb`, `-no-pcdrv`, `-webserver`, `-webserver-port 8080`, `-stdout`, `-lua_stdout`, and `-dofile` for the adapter. Use disposable media for both slots when mounted.

Read `PCSX.settings.emulator` at runtime. The pilot checked `Dynarec`, `HardwareRenderer`, `AutoUpdate`, `LinearFiltering`, and `FastBoot`. A persisted profile may retain pre-CLI defaults until exit, so inspecting only the profile is insufficient. Save renderer output as evidence too.

## Commands and frame timing

The controller binding was `PCSX.SIO0.slots[1].pads[1]`, with button constants in `PCSX.CONSTS.PAD.BUTTON`. `pad.setOverride(button)` pressed a button; `pad.clearOverride(button)` released it. Clear all supported overrides before starting an action and on every exit path.

Use `PCSX.Events.createEventListener('GPU::Vsync', callback)` with a strong retained reference. Increment an adapter frame counter in the callback. At the target count, immediately call `PCSX.pauseEmulator()` and release overrides; defer complex completion/capture work with `PCSX.nextTick`. Record the count again when completion occurs and compare actual with requested. Merely deferring pause to the next main-loop tick can overshoot; qualify behavior on the installed build.

Every production recipe action should use validated known buttons and bounded positive integer frame counts. Keep unrestricted resume/reset/settings changes in diagnostic commands rather than a frozen qualification recipe.

The Lua HTTP implementation needed explicit initialization:

```lua
PCSX.WebServer = PCSX.WebServer or {}
PCSX.WebServer.Handlers = PCSX.WebServer.Handlers or {}
```

Handlers appeared under `/api/v1/lua/<handler-name>`. The coordinator sent a unique command ID, received an acceptance acknowledgement, then polled status until that same ID completed. Acceptance alone did not mean execution finished. The adapter rejected overlapping work, while allowing emergency release to interrupt a pending frame wait. Use main-loop dispatch, bounded polling, process-exit checks, and overall deadlines.

## RAM, captures, and state

`PCSX.getMemoryAsFile()` exposes mapped reads such as `readU8At`, `readU32At`, and `readAt`; close the handle. Bound RAM ranges and validate addresses for the exact game region/revision. Record raw ranges alongside interpreted stable fields, excluding volatile registers from equality criteria unless deliberately required.

`PCSX.GPU.takeScreenShot()` returned width, height, a binary data slice, and a bpp enum. In the tested builds, enum 0 meant 16-bit RGB555 little-endian; the other supported mode was 24-bit RGB. Convert according to the actual format, preserve the raw bytes and dimensions, and hash raw pixels separately from PNG encoding. Reject zero-sized captures; a uniformly black initialized image also fails a gate expecting visible gameplay.

`PCSX.createSaveState()` returned a raw binary slice serialized with `tostring`. Restore it using `Support.File.open` and `PCSX.loadSaveState`, then close the file. These were native raw Redux states, distinct from compressed UI files and other emulators' formats. Capture with all buttons released and the emulator paused. Do not equate a successful load call or matching RAM with resumed execution.

For restore faults, retain frame progress and direct `PCSX.getRegisters()` diagnostics, including PC, GPR, and CP0 when supported. Compare cold versus initialized-process restoration from the same state/card. Avoid adding unnecessary state-decoding machinery to the first pilot; direct registers and bounded progress were sufficient here.

## Evidence ownership

Bind visual observations to immutable capture hashes. A passing mechanical run should remain awaiting review until the expected scene has been inspected. Preserve earlier failed trials in distinct run directories, stop task-owned processes after collection, and exclude private evidence from Git.

In the original memcard-viewer checkout, the executable example is `tools/ps1_replay/` and the report is `docs/research/linux-replay-first-test.md`. Those paths are optional implementation references; this skill does not require that repository or its private assets.
