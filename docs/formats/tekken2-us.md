# Tekken 2 US read-only summary

Supports only `BA` / `SLUS-00213` / `TEKKEN-2`, one 8192-byte SC block, `SC 13 01` and the Shift-JIS title normalized to `[TEKKEN2]`. No PAL, Japanese or Tekken 3 layout substitution. Save IDs do not prove every disc revision.

Adapted from MIT copyright 2024 RyudoSynbios, game-tools-collection [47203c0683893fb41f59154c5c60752125199ca5](https://github.com/RyudoSynbios/game-tools-collection/tree/47203c0683893fb41f59154c5c60752125199ca5/src/lib/templates/tekken-2/saveEditor), `template.ts` and `utils.ts`. [Retained license](../licenses/game-tools-collection-MIT.txt).

Offsets include the reconstructed SC header; no MCS/card offsets. Copy `[0x200,0x400)` to a bounded buffer; retain byte 0, then decode byte i as `encoded[i] - (encoded[i-1]*5 + 1)` modulo 256. The previous byte must be encoded, not already decoded. Integrity is the modulo-256 sum of all 512 decoded bytes equalling zero (decoded byte 0 is the negative sum of bytes 1..511). The US timer is LE u32 at decoded `0x9`, divided by 60, displayed to whole seconds. Unlocks are the 23 little-endian bits beginning at decoded `0x1` with the upstream character order; reserved bit 23 is not named.

Public `tests/saves/tekken-2/playstation/usa.mcr` SHA-256 `f784bdd4527a01ac88242adceb3ce6c1a175c6719108080b9aac6ff844631d2c` has identifier `BASLUS-00213TEKKEN-2`, matching checksum, timer 1858 ticks (00:00:30), flags `fe 02 0c`. This is source/sample evidence, not an in-game display observation or emulator save/reload qualification. Time attack/survival records, arcade defeat flags and options are omitted.

Default tests use synthetic forward-encoded data, boundary time and first/last unlock bits, every truncation, wrong IDs/regions/layouts, corruption and immutable input. Third-party cards are not redistributed. Optional hash-pinned regression:

```
GAME_TOOLS_COLLECTION=/absolute/source/checkout cargo test --manifest-path src-tauri/Cargo.toml tekken2_public_card -- --ignored
```
