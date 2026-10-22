# Original Resident Evil US read-only summary

Supports only `BA` / `SLUS-00170` / identifier `1` (upstream slot 2), one 8192-byte SC block, `SC 13 01` and normalized Shift-JIS title beginning `RESIDENT EVIL `. Other slots/releases, Director's Cut and Windows files are not qualified. Product code does not certify all disc revisions.

Source: MIT copyright 2024 RyudoSynbios, game-tools-collection [47203c0683893fb41f59154c5c60752125199ca5](https://github.com/RyudoSynbios/game-tools-collection/tree/47203c0683893fb41f59154c5c60752125199ca5/src/lib/templates/resident-evil/saveEditor), `template.ts` and `utils.ts`; [retained license](../licenses/game-tools-collection-MIT.txt).

Offsets include SC header and exclude card/MCS wrappers. Time LE u32 `0x224` / 60 for US (not PAL /50), health LE u16 `0x21e` without threshold/unit conversion or clamping, ink ribbons used u8 `0x228`, character bit 0 of `0x22b` (0 Chris Redfield, 1 Jill Valentine), location **big-endian** u16 `0x200` kept hex. No game integrity algorithm is established; `checksumOk` remains null, never passing. Inventory, item box, story flags, condition thresholds and named-room mapping are omitted.

Static source-card evidence: `tests/saves/resident-evil/playstation/usa.mcr` SHA-256 `519ab9dee658dc3bdb927f9775b903f605cb8eb94df807591a36b30bdd4316cb` contains `BASLUS-001701`, matching header/title; Chris, time 15984 ticks (00:04:26), health 140, ink ribbons used 1, location 0x0006. The time and field meanings are source/sample corroboration, not independently observed in-game displays. No emulator save/reload validation.

Synthetic tests distinguish timer rate, endian order, character bit from reserved bits, uncapped raw health and unknown integrity. Every truncation, overlong payload, incorrect release/region/suffix/header/title is rejected without mutation. Third-party cards are not redistributed. Optional hash-pinned regression:

```
GAME_TOOLS_COLLECTION=/absolute/source/checkout cargo test --manifest-path src-tauri/Cargo.toml resident_evil_public_card -- --ignored
```
