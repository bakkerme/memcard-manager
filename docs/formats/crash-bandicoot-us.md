# Crash Bandicoot US read-only summary

Supports `BA` / `SCUS-94900`, **only observed identifiers `00867900` and `01313863`**, one 8192-byte reconstructed SC block, `SC 13 01` header and a valid Shift-JIS title normalized to `Crash (<digits>%)`. Other random save-name suffixes remain unsupported until qualified; no region or revision conversion. Each file is one adventure, not an array of internal profiles.

Adapted from RyudoSynbios/game-tools-collection, MIT copyright 2024 RyudoSynbios, pinned at [47203c0683893fb41f59154c5c60752125199ca5](https://github.com/RyudoSynbios/game-tools-collection/tree/47203c0683893fb41f59154c5c60752125199ca5/src/lib/templates/crash-bandicoot/saveEditor): `template.ts`, `utils.ts`; common `utils/checksum.ts` zeroes the stored checksum before generation. [Retained MIT notice](../licenses/game-tools-collection-MIT.txt).

All offsets include the SC header and exclude card/MCS wrappers. The editor adds `(icon frames - 1) * 0x80` to the base `0x100` structure: three frames put it at `0x200`. Current level is LE u32 at `0x204` (kept raw). Preview gems and keys are bits 5..9 and 10..11 of LE u16 at `0x200`. Completion is stored Shift-JIS title text, not a derived percentage. Checksum: seed `0x12345678`, add each byte with u32 wrapping then rotate left 3 over `[0x200,0x280)`, treating `[0x27c,0x280)` as zero; compare LE u32 at `0x27c`. A mismatch warns; it does not repair or mutate bytes.

Static source and public-card evidence only. `tests/saves/crash-bandicoot/playstation/usa.mcr` SHA-256 `adeeaa6d1f10909cb765767f676346eedcb566e1788b818b255570412c8ba964` contains both identifiers; stored checksums `0x1144870a` and `0x115466eb` match, previews are 1% and 2%. No in-game display or save/reload qualification was performed. Lives, playtime, level names and individual collectibles are not decoded.

Tests use synthetic data by default, including every truncation, malformed layout/identifier/region and corruption. No third-party saves are redistributed. To run the hash-pinned public-card regression after separately cloning the source:

```
GAME_TOOLS_COLLECTION=/absolute/path/to/game-tools-collection cargo test --manifest-path src-tauri/Cargo.toml crash_public_card -- --ignored
```
