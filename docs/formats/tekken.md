# Tekken read-only summary

Supports one 8192-byte reconstructed SC payload, header `SC 13 01`, and directory identifier `<TEKKEN>`.

| Region | Directory | Product | Title after Shift-JIS decode and NFKC |
| --- | --- | --- | --- |
| NTSC-U | `BA` | `SLUS-00006` | `True fighters love TEKKEN !` |
| PAL | `BE` | `SCES-00005` | `True fighters love TEKKEN !` |
| NTSC-J | `BI` | `SLPS-00040` | `真の格闘ゲームファンを熱くさせる![鉄拳]` |

Other product codes, regions, identifiers and titles are rejected. Tekken 2 and Tekken 3 stay on their own readers. A save id does not prove every disc revision. `japan-rev1` is an upstream filename, not a verified disc revision.

Adapted from MIT copyright 2024 RyudoSynbios, game-tools-collection [15d71340c3f1717745139c2e0774510cd37dfbdd](https://github.com/RyudoSynbios/game-tools-collection/tree/15d71340c3f1717745139c2e0774510cd37dfbdd/src/lib/templates/tekken/saveEditor), `template.ts` and `utils.ts`. [Retained license](../licenses/game-tools-collection-MIT.txt).

Confidence for every field below is the same: ported from that editor and checked by a fixture round-trip on the public cards. This is not an in-game display observation and not an emulator save/reload qualification.

Offsets include the reconstructed SC header and exclude MCS and card wrappers. The template lists the product-code bytes at validator offset `0`. On these cards that check is the directory product code. The payload starts with `SC 13 01`, and the progress offsets below were read on that SC payload.

The template defines no checksum. `checksumOk` stays null.

| Field | Offset | Reading |
| --- | --- | --- |
| Unlocked characters | 17 little-endian bits at `0x20c` | Editor order: Kazuya, Paul, Law, Jack, Nina, King, Yoshimitsu, Michelle, Wang, Heihachi, Lee, Prototype Jack, Armor King, Anna, Ganryu, Kuma, Kunimitsu. Other bits in the third byte stay an unknown mask |
| Kazuya costume | `0x214` bit 0 | The only costume flag the template enables. The other costume rows are `disabled` placeholders at offset `0` and are not read. Other bits in the byte stay an unknown mask |
| Arcade completed | 17 little-endian bits at `0x210` | Same character order. Other bits in the third byte stay an unknown mask |
| Time places | 16 records of 12 bytes at `0x228` | Little-endian u32 time, u16 character, u16 fight count, 3 name bytes. An all-zero record is omitted. Fight count is not clamped to the editor's 1–5 control |
| Versus places | 16 records of 12 bytes at `0x2e8` | Little-endian u32 wins and u16 character. The template's hidden u16 at `+6` and hidden u32 at `+8` are not shown. An all-zero record is omitted |
| Character totals | 17 little-endian u32 values at `0x3a8` | One unnamed count per character, in the same order. Zero counts are omitted from the summary. The editor's 99999 control is not applied |

Time uses the template's chrono group. Divide the stored count by 50 for `SCES-00005` and by 60 otherwise (`utils.ts` replaces the template's `/50` when the region is not Europe). Multiply that quotient by 60 and read whole hours; that bucket is the minute count. Seconds and milliseconds are the components of the unshifted quotient. The result is `minutes:seconds.milliseconds`. It is not labeled playtime. The template has no playtime field.

Names are shown when every byte is in the editor set `[ .0-9?A-Z]`. Any other byte is `Unknown name` plus the three raw bytes. Character ids outside 0..=16 are `Unknown` plus the raw id.

Options (`0x201`..=`0x207`), both players' button config (`0x215`..=`0x226`), and the hidden versus words are not decoded.

## Public cards

Hashes are SHA-256 of the files in `tests/saves/tekken/playstation/` at the commit above. The cards are not redistributed.

| File | SHA-256 | Directory name |
| --- | --- | --- |
| `usa.mcr` | `c03248bf8b5d9234c3ad96c517337f1fcee4c672e4a58bdca59780d27fdcece0` | `BASLUS-00006<TEKKEN>` |
| `europe.mcr` | `e2670d27d3e240314218587f763e0bb4c015f15a966fe634053f2a02a165645d` | `BESCES-00005<TEKKEN>` |
| `japan.mcr` | `d0dba92f9a33ac91cdf9335a30d93f8eb3755cdd2f92795dcb9942287116b02f` | `BISLPS-00040<TEKKEN>` |
| `japan-rev1.mcr` | `9e22e6a430e60ed40dfb15faafcabc4c7186cae0814953c4630eaf9420bca006` | `BISLPS-00040<TEKKEN>` |
| `filled.mcr` | `b696b0a47725c2cfea58796e3e39357f93d7e52d7a94880dbc661536c627291e` | Europe save in slot 10 and Japan save in slot 12; other saves are not Tekken |

On `usa.mcr`, `japan.mcr` and `japan-rev1.mcr` the first time count is 180000. On `europe.mcr` it is 150000. Both readings are `50:00.000` with the regional divisor, then one minute per place through `65:00.000`. Initials are `PAS`, `ABE`, `ARA`, `TAK`, `BES`, `KUN`, `CHA`, `K.M`, `SAM`, `K.N`, `K.K`, `KIN`, `KMT`, `NAK`, `KIM`, `PIX`. Each fight count is 2. Unlock byte `0x20c` is `0xff` and the following flag bytes are 0, so the eight starting characters are set and the other nine are clear. Arcade flags, the Kazuya costume bit and all seventeen character totals are 0. Versus wins are 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1 for Michelle, Yoshimitsu, King, Nina, Jack, Law, Paul, Kazuya, then that character order again. `filled.mcr` matches those progress bytes for its Europe and Japan Tekken saves. `japan-rev1.mcr` differs from `japan.mcr` at `0x209..=0x20b` and `0x3f4..=0x3f5`, outside the mapped fields, and decodes to the same summary.

Default tests build synthetic payloads, including the observed title bytes, a fractional time (`1234567` → `342:56.117` at 60 Hz and `411:31.340` at 50 Hz), unknown bits, an unclamped total and fight count, every truncation, and wrong region, identifier, title and header bytes. No third-party card is committed.

```
GAME_TOOLS_COLLECTION=/absolute/source/checkout cargo test --manifest-path src-tauri/Cargo.toml tekken_public_cards -- --ignored
```
