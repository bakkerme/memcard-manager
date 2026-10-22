# Tomb Raider II US read-only summary

Supports `BA` / `SLUS-00437` / `TOMB2`, one 8192-byte reconstructed SC payload, `SC 11 01` and the Shift-JIS title normalized to `Tomb Raider II`. The upstream files named `usa`, `usa-rev1`, `usa-rev2`, `usa-rev3` all meet this boundary. Those filenames are upstream labels, not independently verified disc revisions. PAL/Japanese identifiers and original Tomb Raider remain separate.

Adapted from RyudoSynbios/game-tools-collection (MIT, copyright 2024 RyudoSynbios), pinned [47203c0683893fb41f59154c5c60752125199ca5](https://github.com/RyudoSynbios/game-tools-collection/tree/47203c0683893fb41f59154c5c60752125199ca5/src/lib/templates/tomb-raider-ii-starring-lara-croft/saveEditor), `template.ts` and `utils.ts`; [retained notice](../licenses/game-tools-collection-MIT.txt).

Offsets include SC, exclude MCS/card wrappers. Total recorded playtime sums 18 LE u32 counters at `0x244 + i*0x2c`, using u64 accumulation, then divides by 30. Recorded secrets count only documented bits 0..2 at `0x256 + i*0x2c`, i=0..15 (48 possible); reserved bits and the final two levels are excluded. These are stored level records, not a recomputation from the current checkpoint. Current position is LE u16 at `0x634`, meaningful only when the byte at `0x64c` is 1; 0 means not recorded and other markers are unknown. The 18-name table is copied from upstream. The byte `0x640` is 0/1 game-cleared; other values stay unknown. Game integrity is the modulo-256 sum of `[0x200,0x1ed0)` including checksum byte `0x64b` equalling zero. A mismatch warns without repairing bytes.

Public-card static evidence (SHA-256):

| Upstream file | SHA-256 |
| --- | --- |
| usa.mcr | `34ca73e5715684fe8f2e0e618895c1326b0fe362b8fe1948ba2e8ef0f09377e2` |
| usa-rev1.mcr | `5fae5f2b69ce3c04c9c14d66dfc7166fff1bc944d5683a5ed98bfb7db931c6ff` |
| usa-rev2.mcr | `58c422e298de4ece374c726a349591275afec864e48d51ca15e53b53a40e7e8d` |
| usa-rev3.mcr | `684c2f0499f698c51f10a7f713c21b54561112fce56b77f646c50dc29a8fb5e2` |

Each has one matching identifier, checksum sum zero, level 1 (Venice), recorded time zero, pending marker 1 and cleared marker 0. No emulator, in-game display or save/reload validation performed. Inventory, health, checkpoint statistics, level unlock/cleared flags and per-level record UI are not decoded.

Synthetic tests cover time accumulation, masked secrets, final-level mapping, unknown IDs/markers, corruption, all truncations and identity/layout rejection. No third-party saves committed. Optional public regression:

```
GAME_TOOLS_COLLECTION=/absolute/source/checkout cargo test --manifest-path src-tauri/Cargo.toml tomb2_public_cards -- --ignored
```
