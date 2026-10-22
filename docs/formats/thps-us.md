# Tony Hawk's Pro Skater US read-only summary

Supports only `BA` / `SLUS-00860` / `THPSGN0` or `THPSGCL`, one 8192-byte SC block, `SC 11 01` and normalized Shift-JIS title beginning `THPS CAREER ー `. These are two career files, each containing eleven skater totals; zero is not inferred to mean an empty file or locked skater. Replays and other suffixes, regional releases and revisions are unqualified. The source calls the game Tony Hawk's Skateboarding; the observed US product code is the Pro Skater release.

Source: RyudoSynbios/game-tools-collection, MIT copyright 2024 RyudoSynbios, pinned [47203c0683893fb41f59154c5c60752125199ca5](https://github.com/RyudoSynbios/game-tools-collection/tree/47203c0683893fb41f59154c5c60752125199ca5/src/lib/templates/tony-hawk-s-skateboarding/saveEditor), `template.ts`, `utils.ts`; [retained notice](../licenses/game-tools-collection-MIT.txt).

SC-relative offsets (exclude card/MCS wrapper): eleven one-byte tape totals at `0x288 + i*0xe`, in the upstream skater order Tony Hawk through Officer Dick. Values are retained as stored, not recomputed from goals, clamped, or used to infer unlocks. Integrity is `sum(byte + 1)` over `[0x280,0xc80)` modulo 65536, compared to LE u16 at `0x236`. No medal/goal/score/option fields are decoded.

Public card `tests/saves/tony-hawk-s-skateboarding/playstation/usa.mcr` SHA-256 `db8859ad5ee9635cf81d59484f8539aed18c33dc3f29373969806bf5ed3c371c`: both observed IDs and career titles match, stored checksums 0x2839 and 0x2874 pass, all eleven stored tape totals are zero in each. PASS/FAIL title text is not treated as an integrity result. No emulator, in-game display or save/reload validation was performed.

Synthetic tests cover the first/last skater stride and nonzero totals, corruption at the final covered byte, every truncation, malformed identity/layout and input immutability. Third-party saves are not redistributed. Optional hash-pinned regression:

```
GAME_TOOLS_COLLECTION=/absolute/source/checkout cargo test --manifest-path src-tauri/Cargo.toml thps_public_card -- --ignored
```
