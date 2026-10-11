# Case study: Legends 2 ARM64 replay

Verified 10 October 2026 in memcard-viewer's research harness. These observations concern US `SLUS-01140` and one known save. Do not automatically apply the build pin, memory addresses, scratch preparation, or warmup to another title or environment.

## What worked

Three fresh PCSX-Redux processes loaded the same paused gameplay state/card, opened Equipment without desktop focus or user input, and produced identical framebuffer hashes and stable values. This qualified single-worker input/replay; saving, parallel isolation, and rendering without Xvfb remained unqualified.

The successful official ARM64 build was **70**, version `d42cdae5`, changeset `d42cdae5bcfbb7e6565b52d4e14e9ca1ede9bfc4`. The [immutable archive](https://distrib.app/storage/assets/8fd/737/d3d/b33c10bb3df955de9abd29bcc75f153caeee2d9a8209c027c2f445e/PCSX-Redux-d42cdae5-linux-aarch64.zip) had SHA-256 `f9568b93db98b41e742f95630bc3e6c2cb7cf940f32f1cb59c4e754d828a4ee6` and contained `PCSX-Redux-HEAD-aarch64.AppImage`.

The pilot used interpreter CPU, software GPU/llvmpipe, Xvfb, dummy audio, fastboot, and disabled custom shaders, updates, and filtering. Image digest was `sha256:8e6ef7ef1fe9ac0ab99b861dba3070e442a8c27a3b835edfd4b16b509945da8b`; rebuilding dependencies need not reproduce that digest.

## Diagnostic findings

**Black gameplay was not a missing BIOS.** Official build 79 (`6eda90ff`) recognized the existing BIOS, rendered cinematics/load menus, and loaded correct save RAM, yet gameplay stayed black even during uninterrupted execution. Build 70 rendered gameplay with the same assets. This brackets a compatibility regression; the exact upstream cause was not isolated. [Source comparison](https://github.com/grumpycoders/pcsx-redux/compare/d42cdae5bcfbb7e6565b52d4e14e9ca1ede9bfc4...6eda90ff550af3c8e8888094db96d211f5cba652) offers candidates, not proof of one causal change.

**Cold restoration needed initialization in this build.** A fresh build-70 process restored correct RAM but faulted with `ReservedInstruction` at `0xa0010040`, with no vsync progress. Fixed startup probes of 1, 2, 60, and 600 vsyncs (663 total) before restoration solved it in all three trials. The adapter reset the relative counter after restore. The exact uninitialized emulator bookkeeping remains unknown; 663 is a measured recipe, not a universal minimum.

**BIOS initialization changed a scratch frame.** Original boot changed offsets `0x1f80`, `0x1f81`, and `0x1fff` to `4d`, `43`, and `0e`. These lie in block 0/sector 63, the documented [memory-card write-test frame](https://psx-spx.consoledev.net/ps1/sio/controllersandmemorycards/memory-card-data-format/), used by [BIOS card-change acknowledgement](https://psx-spx.consoledev.net/ps1/kernelbios/memory-card-functions/).

Before measured boot, a guarded preparation copied sector 0 into sector 63 of the disposable card. It accepted only an all-zero scratch frame or one already equal to sector 0, logged both hashes and exact differences, and verified directories/save blocks were unchanged. Originals stayed read-only. The prepared card remained byte-for-byte unchanged during measured execution; trials used exact baseline copies without additional preparation. Unknown contents would require investigation, not normalization.

**Start and Select serve different menus.** Start opened Pause; gameplay Select opened inventory/status. Guessing controls obscured successful navigation until stepwise capture resolved it.

## Save and stable values

Selected position 1 was `BASLUS-01140-DASH20`. The load menu showed **Nino Pad, 03:41:26**. Gameplay showed MegaMan/Data on the ship deck; Status called it **Nino Island / Flying Ship Dock**. The different scene labels were explicitly reconciled rather than treating one label as universal.

| Field | Address/type | Expected |
| --- | --- | --- |
| Map / secondary | `0x8009c808` / `0x8009c809`, U8 | 23 / 0 |
| Difficulty | `0x8009c80e`, U8 | 1 (Normal) |
| Zenny | `0x8009c820`, U32 | 11,950 |
| Helmet / shoes / armor | `0x8008c252` / `253` / `254`, U8 | 1 / 1 / 2 |
| Buster parts | `0x8008c258` / `259` / `25a`, U8 | 29 / 14 / 0 |

Visible Equipment showed Normal Helmet, Jet Skates, Padded Armor, Accessory Pack, Buster Unit, and Machine Gun Arm. Relevant RAM captures covered `0x8008c0b0` length `0x300` and `0x8009c7f8` length `0x100`. Addresses and interpretation require this exact release's reader evidence.

## Frozen recipe and outcome

After restoring the paused gameplay baseline with all buttons released:

| Action | Held vsyncs | Released wait |
| --- | ---: | ---: |
| Select | 6 | 120 |
| Down | 6 | 12 |
| Down | 6 | 12 |
| Cross | 6 | 120 |

The final checkpoint was **288 relative vsyncs**, following the frame-5751 baseline. The discarded 663 startup frames were logged separately. All requested/actual action counts matched. Trials took 18.37, 18.38, and 18.26 seconds including startup, within their five-minute deadlines.

All final raw RGB555 framebuffers hashed to `f389cbb00ac586d0de77755c0813734cf0e9eda333c888cf1e015761c33cc402`. All three captures were also visually inspected for Equipment and expected gear. Stable RAM, originals, prepared cards, and the baseline state/card stayed unchanged. Cleanup acknowledged override release and clean emulator exit.

The local report and ignored machine-readable evidence retain the complete provenance. Eighteen simulated coordinator tests passed separately from the real emulator qualification. The useful general lesson is to validate visible scene, frame behavior, and resumed execution independently, while retaining failed evidence and preserving the exact tested scope.
