# Grandia US read-only summary

Supports only `BA` / `SCUSP94457` / `GRA-S01` (upstream slot 2), one 8192-byte SC block, `SC 13 01`, normalized Shift-JIS title beginning `GRANDIA-` (WHATWG Shift-JIS/NFKC) and exact `GrandiaBupData\0` at SC `0x280`. The P-bearing product code is the game's save filename code, not a disc serial. Other slots, regional layouts, HD remaster and disc revisions are unqualified.

Source: MIT, copyright 2024 RyudoSynbios, game-tools-collection [47203c0683893fb41f59154c5c60752125199ca5](https://github.com/RyudoSynbios/game-tools-collection/tree/47203c0683893fb41f59154c5c60752125199ca5/src/lib/templates/grandia/saveEditor), `template.ts` and `utils.ts`. [Retained license](../licenses/game-tools-collection-MIT.txt).

Offsets relative to reconstructed SC, never card/MCS. The US source adds `0x180` to PAL field offsets: time LE u32 `0x110 + 0x180 = 0x290` at 60 Hz (not PAL 50 Hz), location LE u16 `0x174 + 0x180 = 0x2f4`, disc u8 `0x1a9 + 0x180 = 0x329`, Gold LE u32 `0x1b0 + 0x180 = 0x330`. Disc values 1/2/0x82 mean Disc 1/Disc 2/Transition; unknown values stay numeric. Location IDs remain hex; no unqualified English table lookup. No game integrity algorithm is established: `checksumOk` remains null. Party, inventory and spells are omitted.

Static public-card evidence: `tests/saves/grandia/playstation/usa.mcr`, SHA-256 `2dd2cc33bc05be439045ca86df3f6023c35e0eef9b4d046d21f351525bb44e11`. One `BASCUSP94457GRA-S01` entry, matching marker, 28241 ticks (00:07:50), Gold 200, disc 1, location 0x2C00. These values are source/sample corroboration, not in-game observations. No emulator save/reload validation.

Synthetic tests distinguish shifted offsets, 60-Hz hour boundary, raw location and unknown disc; cover every truncation and malformed identity/header/marker and immutable input. No saves redistributed. Optional hash-pinned regression:

```
GAME_TOOLS_COLLECTION=/absolute/source/checkout cargo test --manifest-path src-tauri/Cargo.toml grandia_public_card -- --ignored
```
