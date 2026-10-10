# First Linux Legends 2 input/replay test — 10 October 2026

**Result: passed.** Three fresh PCSX-Redux processes restored the same paused
Redux state/card pair, opened Equipment without user input or desktop focus,
and produced identical screens with matching stable RAM. The production app is
unchanged. This qualifies single-worker input/replay, not in-game saving,
parallel isolation or fully headless rendering.

## What fixed the failures

1. **Pin official ARM64 build 70 (`d42cdae5`).** Build 79 (`6eda90ff`)
   recognized the BIOS, rendered cinematics/load menus, and loaded the correct
   save values, but stayed black after loading. Continuous execution reproduced
   that failure. Build 70 rendered the loaded gameplay and Equipment with the
   same original BIOS/disc/card. This brackets an emulator compatibility
   regression between those builds; the exact upstream commit has not been
   isolated or fixed. The official [source comparison](https://github.com/grumpycoders/pcsx-redux/compare/d42cdae5bcfbb7e6565b52d4e14e9ca1ede9bfc4...6eda90ff550af3c8e8888094db96d211f5cba652)
   includes substantial CPU/CD-ROM changes, which are candidates rather than a
   proven individual cause.
2. **Initialize each fresh emulator before restoring.** A cold build-70 restore
   loaded RAM but faulted with `ReservedInstruction` at `0xa0010040` and produced
   no vsyncs. Running the fixed startup probes (1, 2, 60, 600 frames; total 663)
   before restoring solved this in all three fresh-process trials. The adapter
   resets its relative count on restore, so the final checkpoint is frame 288.
   The specific uninitialized emulator bookkeeping behind the cold fault is
   not yet established.
3. **Prepare the disposable card's write-test frame before measured execution.**
   Boot originally changed `0x1f80`, `0x1f81`, `0x1fff` to `4d`, `43`, `0e`.
   These are block 0/sector 63, the [write-test frame](https://psx-spx.consoledev.net/ps1/sio/controllersandmemorycards/memory-card-data-format/).
   The BIOS [card-change acknowledgement](https://psx-spx.consoledev.net/ps1/kernelbios/memory-card-functions/)
   uses a dummy write there. The harness now explicitly mirrors sector 0 into
   this unused scratch frame in the disposable copy **before** measuring boot.
   It records both hashes and the three changed offsets, refuses unfamiliar
   scratch contents, and leaves directories and every save block untouched.
   The original QA card remains byte-for-byte unchanged. The prepared card
   remains byte-for-byte unchanged throughout measured boot and all trials.
4. **Use Select for inventory.** Start opens Pause. The calibrated sequence is
   Select → Down → Down → Cross, with versioned press/release and vsync waits.
   No equipment is changed and no in-game save is performed.

## Qualification evidence

Known position 1 is `BASLUS-01140-DASH20`, shown in the load menu as **Nino Pad,
03:41:26**. Gameplay shows MegaMan/Data on the ship deck; Status names the same
scene **Nino Island / Flying Ship Dock**. RAM is map 23/secondary 0, Normal
(difficulty 1), 11,950 Zenny, helmet/shoes/armor IDs 1/1/2 and buster IDs 29/14/0.
Each Equipment capture visibly shows Normal Helmet, Padded Armor, Jet Skates,
Accessory Pack and Buster Unit; Machine Gun Arm also matches across trials.

| Trial | Relative checkpoint | Elapsed including startup | Screen/RAM | Card |
| --- | ---: | ---: | --- | --- |
| 1 | 288 | 18.37 s | Matches | Unchanged |
| 2 | 288 | 18.38 s | Matches | Unchanged |
| 3 | 288 | 18.26 s | Matches | Unchanged |

The relative checkpoint is 288 vsyncs after the frame-5751 baseline (game
checkpoint 6039). Each fresh process executes 663 startup vsyncs before restoring;
those discarded startup frames are recorded separately in its action log.

- Final raw RGB555 framebuffer SHA-256: `f389cbb00ac586d0de77755c0813734cf0e9eda333c888cf1e015761c33cc402`.
- Final PNG SHA-256: `dc3e4443b5c91c737c3374555c9a3ababa63f275e3b784568794fccce61ff03a`.
- Baseline captured at boot checkpoint 5751; state SHA-256:
  `7f9d9e8f379188ba00471af956ec4fbe27a3fe954e12fbc15a7c6b7fa186cfa5`.
- Matching prepared card SHA-256: `a003f5d6df552b503159d88760194fe432dfc4a372dc3fa27a462d7b64d62bdc`.
- Original QA card SHA-256: `19707878fb58d87733a16e5ac0d89e1380d67d4f811dabddb8fbda38345292cf`.
- Recipe version 2 SHA-256: `8f0ff75f75a937af15e89a92bb0a45df30efac2a7871210ec3ebdc59d0a97fa0`.
- All 12 bounded frame/input actions in each trial matched their requested
  frame counts exactly. Each fresh process stayed within its five-minute limit,
  acknowledged override release and exited with status 0.
- All source BIOS/disc/card hashes and the baseline state/card hashes remained
  unchanged. The source save passed the existing reader's four checksums.

## Runtime and retained artifacts

Official AppImage changeset `d42cdae5bcfbb7e6565b52d4e14e9ca1ede9bfc4`, archive SHA-256
`f9568b93db98b41e742f95630bc3e6c2cb7cf940f32f1cb59c4e754d828a4ee6`. The pinned [download](https://distrib.app/storage/assets/8fd/737/d3d/b33c10bb3df955de9abd29bcc75f153caeee2d9a8209c027c2f445e/PCSX-Redux-d42cdae5-linux-aarch64.zip)
is extracted without FUSE. Image ID `3fc80a5ce917fcb1168b9477792295a1592a1673a72f6fd15af844308e32f25d`;
image digest `sha256:8e6ef7ef1fe9ac0ab99b861dba3070e442a8c27a3b835edfd4b16b509945da8b`.

One native Linux ARM64 Podman container, four CPUs/3 GiB, Xvfb, Mesa llvmpipe,
interpreter CPU, dummy audio, fastboot, disabled custom shaders/updates/filtering.
Only container-loopback HTTP is used; networking is disabled and no ports are
published. Original source assets are mounted read-only. Settings, renderer,
launch arguments, executable version, inputs and resource limits are retained.

Private ignored evidence is in `tools/ps1_replay/runs/2026-10-10-replay70/`:
`result.json` is the reviewed machine-readable passing verdict. The directory
retains the exact baseline and visual review, original hashes, settings,
per-step screenshots/raw framebuffers, relevant RAM, action counts/timings and
cleanup logs. Its baseline came unchanged from
`2026-10-10-qualified70/`, which retains the prepared-card boot/calibration.
Earlier failures remain in neighboring run directories, including build 79's
black-screen reproduction and build 70's cold-restore fault. No private asset
bytes or screenshots are committed in this report.

All **18 simulated harness tests** pass. They exercise coordinator failure
handling, missing inputs, emulator exit, stalls, deadlines, frame overshoot,
button cleanup, bounded recipe validation, guarded scratch preparation,
baseline tampering and mismatched framebuffer review. All harness Python
modules parse. Real emulator qualification is the separate local-assets test
above; hardware and production app behavior were not retested.

See [the harness README](../../tools/ps1_replay/README.md) for reruns.
