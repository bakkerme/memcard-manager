# Mega Man Legends 2 — US memory-card save, research v1

This is an independently derived, partial specification for the original US PS1 release, **SLUS-01140**. The production app's read-only game-details pilot displays named location preview and difficulty, exact playtime, Zenny, health and named equipment. Inventory storage and flag storage have been located, but their names and meanings are incomplete. This does not certify every revision or supply an editor or regional conversion spec.

The executable's packing/restoring routines, two private memory-card saves, nine public save cards, and in-game DuckStation checks provide the evidence. No game executable, ROM, save bytes, emulator source code or third-party editor implementation is redistributed here. The app's existing MemcardRex parser attribution is unchanged.

## Coordinate system and identification

All offsets below are relative to the **8192-byte reconstructed SC payload**, beginning `53 43 11 01`. An MCS export adds a **128-byte directory header**, so an SC offset `0x200` is MCS offset `0x280`. A whole card additionally requires finding the directory entry and reconstructing its block chain with the existing card parser; do not search a card blindly for a matching byte pattern.

Observed filenames are `BASLUS-01140-DASH20` and `BASLUS-01140-DASH21`. The game iterates five file positions, so `DASH2` plus a digit `0..4` identifies a candidate position, displayed as 1..5. Private fixtures cover positions 0 and 1; public fixtures also cover positions 2 and 4. Both saves occupy one 8192-byte block; their MCS exports are 8320 bytes, with directory size 8192 at MCS `0x04`.

The title is Shift-JIS at SC `0x04..0x43`, using full-width characters. NFKC normalization produces `MEGAMAN LEGENDS2[1]03:41:26` or `[2]04:47:52`. The standard icon/palette occupy `0x60..0xff`. The tool conservatively requires the observed header, title and (when present) US directory identifier. It rejects other regions, layouts and containers.

## Checksums and file extent

The game uses four **additive little-endian u32 sums modulo 2^32**, not CRCs. Sum every aligned word from the range start up to, but excluding, the checksum word:

| Data start | Data end, exclusive | Stored u32 checksum |
| --- | --- | --- |
| `0x000` | `0x17c` | `0x17c` |
| `0x200` | `0x27c` | `0x27c` |
| `0x280` | `0x3fc` | `0x3fc` |
| `0x400` | `0xbfc` | `0xbfc` |

The load-screen overlay's validator at `0x800B0F3C` accepts a preview-only argument that checks the first segment; full load checks all four. Its read request at `0x800B0BE0` reads `0xc00` bytes for full load, versus `0x180` for preview. The gameplay overlay's save checksum writer is `0x800B1170`.

`0x180..0x1ff` and `0xc00..0x1fff` are outside these sums. Some *checksummed* bytes are also unused structure padding. The saves contain unrelated stale MIPS instructions/audio bytes in gaps: these are not encryption, compression, or automatically meaningful fields. Preserve all bytes when archiving. A passing sum does not prove that every value is sensible or that an unobserved game revision uses this layout.

## Confirmed fields and structural mappings

Integers are little-endian. “Confirmed storage” means traced through packing/restoring code; it does not certify an enum's English names or physical units.

| SC offset | Type / length | Meaning | Evidence / limits |
| --- | --- | --- | --- |
| `0x128` | u8 | Saved map/stage ID | Copied to runtime `0x8009C808`; both samples 23. Exact map naming needs a table. |
| `0x129` | u8 | Map's secondary ID | Runtime `0x8009C809`; both 0. Room/substage interpretation is provisional. |
| `0x12a..0x12d` | four u8 | Additional world/progress state | Runtime `0x8009C80a..0x8009C80d`; names unresolved. |
| `0x12e` | u8 | Difficulty ID | Runtime `0x8009C80e`, consumed by difficulty-dependent initialization. Both private saves are 1 (Normal). Four starting modes mapped below; internal level 2 remains unnamed. |
| `0x12f` | u8 | Load-menu location preview ID | Preview loader explicitly copies this field. Both 8, observed as **Nino Pad** in the game's load menu. The one-based 25-entry table is mapped below. |
| `0x130` | u32 | Shared record value | Restorer keeps the smaller value across files. Purpose unresolved; both `0xffffffff`. |
| `0x134`, `0x136` | two u16 | Shared record/unlock fields | Restorer keeps the larger value across files. Both 0. Do not guess ending flags. |
| `0x138` | u32 | Displayed playtime, 60-Hz ticks | Runtime `0x8009C818`; truncate `/60` to seconds. Both title and load-menu times match. Gameplay rendering can be 30 FPS; that is not the counter's unit. |
| `0x13c` | u32 | Second time counter | Runtime `0x8009C81c`; equal in these samples, but separately packed/restored. Purpose unresolved; do not use as a required duplicate. |
| `0x200` | u32 | Zenny | Runtime `0x8009C820`; save/load copies traced, published Zenny address agrees. |
| `0x204..0x214` | five u32 | Additional counters | Copied to runtime `0x8009C824..0x8009C834`; names unresolved. |
| `0x218..0x240` | scattered values | Reputation/world state | Packing is field-by-field with holes; see routine addresses below. Not a contiguous RAM dump. |
| `0x24c..0x26b` | 32 bytes | Stored list/state | Runtime `0x8009C884..0x8009C8a3`; names unresolved. |
| `0x280`, `0x284`, `0x288` | three i32 | Player position components, raw | Restored to player `+0x10,+0x14,+0x18`. Axis order/scale unverified. |
| `0x28c` | u16 | Player rotation, raw | Restored to player `+0x2a`; scale unverified. |
| `0x290` | u16 | Current health, raw units | Runtime `0x8008C120`; both 128. |
| `0x292` | u16 | Maximum health, raw units | Runtime `0x8008C122`; both 128. Do not label these as hearts/bars yet. |
| `0x294..0x2a6` | scattered values | Player status / weapon state | Some counters and selectors; names unresolved. `0x2a4` restores to player `+0x18e`. |
| `0x2a8..0x2b2` | six u16 | Weapon/player parameters | Copied to player `+0x190..+0x19a`; names/scales unverified. |
| `0x2b4` | u8 | Equipped helmet ID | Runtime `0x8008C252`. Both 1. |
| `0x2b5` | u8 | Equipped shoes ID | Runtime `0x8008C253`. 1 then 2. |
| `0x2b6` | u8 | Equipped armor ID | Runtime `0x8008C254`. 2 then 3. |
| `0x2b8` | u8 | Equipped buster part 1 ID | Runtime `0x8008C258`. Both 29. |
| `0x2b9` | u8 | Equipped buster part 2 ID | Runtime `0x8008C259`. 14 then 13. |
| `0x2ba` | u8 | Equipped buster part 3 ID | Runtime `0x8008C25a`, confirmed by published modifier address and the traced packed u32. Both samples 0 (None). Third-slot unlock state is not inferred. |
| `0x2bb` | u8 | Additional packed buster state | Runtime `0x8008C25b`. Purpose unresolved; this is not the third equipped part. |
| `0x2bc..0x2df` | 36 bytes | Player inventory/list storage | Copied to player `+0x1ac`, runtime `0x8008C25c`. Sample lists resemble IDs, but ownership/count/order semantics are unresolved. |
| `0x2e0 + 8*i` | 20 records | Weapon-related record bank | For each `i=0..19`, only bytes `+0..+4` are explicitly packed/restored to player `+0x1f4+8*i`. Bytes `+5..+7` are padding, not verified attributes. |
| `0x400..0x51f` | 288 bytes | Global flag bitset | Copied to/from runtime `0x80098548`. Flag ID `n` uses byte `n >> 3`, bit `n & 7`; covers IDs `0..0x8ff`. Story/item meaning requires mapping. |
| `0x520..0xb1f` | 1536 bytes | World/object state bank | Copied to/from runtime `0x800974f8`; internal records unparsed. |
| `0xb20..0xb33` | 20 bytes | Navigation/runtime state | Copy of runtime `0x80078f48..0x80078f5b`; internal pointers occur here. Never follow them in a file reader. |
| `0xb34..0xb9f` | nine 12-byte records | Additional world state | Runtime global-state base `+0xac..+0x117`; meanings unresolved. |

## Reproduction anchors

These are **PS1 virtual RAM addresses**, not disk offsets. Code at a given address can change when an overlay changes. The first snapshot was on the load screen; the second was in the Nino Pad gameplay scene. Treat overlay identity as part of the evidence.

- Gameplay save packer: `0x800B0C90`; global state base `0x8009C7F8`, player base `0x8008C0B0`. It stores time at `0x138`, Zenny at `0x200`, health at `0x290/292`, equipment at `0x2b4..2b9`, and the three large state banks.
- Gameplay global restorer: `0x800AE4D8..0x800AE6F4`, with save buffer `0x801DE800`. Zenny restore is `0x800AE5AC`; difficulty restore is `0x800AE51C`.
- Player restore: `0x800C3ADC..0x800C3C4C`. Health restore is `0x800C3B44/48`, equipment restore `0x800C3BD0..DC`, then 36-byte list and 20-record loops.
- Load-screen preview decoder: `0x800B0794..0x800B07D4`; location byte `0x12f` and displayed time `0x138` are explicitly selected.
- Standard memory copier `0x80015968` copies 32 bytes per iteration. The flag bank uses 9 iterations; object bank 48. This establishes actual bank sizes rather than inferring them from padding.

DuckStation 0.1-11894 produced save-state version 87. For research, its zstd-compressed state starts at the header's `offset_to_data`. In the decompressed state, the Bus marker is followed by RAM size and 60 bytes of access-time tables before the 2 MiB RAM buffer. This is an emulator-version-specific extraction detail; the inspection tool itself needs no emulator or ROM.

The code authors' [GameHacking RAM codes](https://gamehacking.org/game/89272) supplied cross-checks for Zenny, health, time and equipment address meanings. Those are RAM codes, **not a published memory-card specification**. This document independently traces those values into save offsets. No cheat was applied. [Dash2-Toolkit](https://github.com/kion-dgl/Dash2-Toolkit) is useful context for savestate/asset research, but does not supply the memory-card layout used here.

## Equipment name mappings

The production decoder names all **45 documented entries**: 2 helmets, 5 shoe types, 7 armor types and 31 buster parts, plus ID 0 as None in each category. The explicit hexadecimal modifier tables in [Skatr11718's guide](https://www.cheatcodes.com/guide/walkthrough-megaman-legends-2-playstation-13555/) supply ID/name facts for the same runtime addresses traced above. [RPGClassics' equipment catalog](https://shrines.rpgclassics.com/psx/mml2/parts.shtml) independently corroborates the set of names, but its display order is **not** the ID order. No guide prose, prices, descriptions or source code is incorporated into the app.

The first private save's five nonzero equipped IDs were independently checked in DuckStation's Equipment screen on 2026-10-05. The remaining names are documented mappings, not claims that every item was separately equipped in an emulator. Out-of-range values (including the guides' glitch-only modifiers) remain unknown. Inventory ownership, special-weapon upgrade attributes and key-item flags still require separate work.

- Helmets: `01` Normal Helmet; `02` Padded Helmet.
- Shoes: `01` Jet Skates; `02` Hydrojets; `03` Asbestos Shoes; `04` Cleated Shoes; `05` Hover Shoes.
- Armor: `01` Normal Armor; `02` Padded Armor; `03` Padded Armor Omega; `04` Link Armor; `05` Link Armor Omega; `06` Kevlar Armor; `07` Kevlar Armor Omega.
- Buster parts: `01..03` Power Raiser / Alpha / Omega; `04..06` Turbo Charger / Alpha / Omega; `07..09` Range Booster / Alpha / Omega; `0a..0c` Rapid Fire / Alpha / Omega; `0d..11` Blaster Unit, Buster Unit, Power Blaster, Sniper Unit, Autofire Unit; `12..16` those five units' Omega versions in the same order; `17..19` Upgrade Pack, Booster Pack, Energizer Pack; `1a..1c` those packs' Omega versions; `1d..1f` Accessory Pack / Alpha / Omega.

The slot-2 save therefore displays Normal Helmet, Hydrojets, Padded Armor Omega, Accessory Pack and Blaster Unit; this is a published-table/byte-level cross-check, not a second in-game gear observation.

## Two local saves

| Field | Position 1 (`DASH20`) | Position 2 (`DASH21`) |
| --- | ---: | ---: |
| In-game location | Nino Pad | Nino Pad |
| Playtime | 03:41:26 | 04:47:52 |
| 60-Hz ticks | 797203 | 1036325 |
| Zenny | 11,950 | 31,700 |
| Health / maximum, raw | 128 / 128 | 128 / 128 |
| Difficulty ID | 1 | 1 |
| Helmet / shoes / armor IDs | 1 / 1 / 2 | 1 / 2 / 3 |
| Equipped buster IDs 1 / 2 | 29 / 14 | 29 / 13 |
| All four sums match | Yes | Yes |

MCS SHA-256:

- `DASH20`: `13210887c585cbaed345e51ce1763aad2b368e1e658ad7c20354086591c65ce6`
- `DASH21`: `630f4ed27e0b579c155f04b0057f4cbf5df7c125d45d8fd3e4c421a368b5440c`

Stored checksum words, ordered header/global/player/world:

- `DASH20`: `91ddd2aa`, `14ff5ef6`, `6b04ca3d`, `f6c065bd`.
- `DASH21`: `9be51bce`, `4812e24b`, `25028201`, `255e3931`.

The first position was loaded in DuckStation at Nino Pad beside Data and the Flutter. Both positions' location/time previews were inspected in the load menu. The first position's Items screen displayed Energy Canteen and entries for Bottle Rocket, Reaverbot Claw, Cute Piggy, Rusted Mine, Artillery Notes, Stuffed Doll and Refractor B; several entries were dimmed. These observations do **not** yet define which save bit means owned, used for development, or merely discovered. Do not turn that screen list into an ownership assertion.

The position-1 loaded RAM snapshot independently matched Zenny 11950, health 128/128, and equipment bytes `01 01 02` / buster bytes `1d 0e 00 01`. The first save's Equipment screen also confirmed Normal Helmet, Jet Skates, Padded Armor, Accessory Pack and Buster Unit against IDs 1, 1, 2, 29 and 14. Additional public cards exercise other locations and difficulty levels as recorded below. No controlled one-item purchase/use/resave experiment, other region or disc revision has been validated. Original memory-card files were not written.

## Saved-location names

The Nino Pad gameplay RAM snapshot contains **25 fixed-width, 32-byte Shift-JIS names** at `0x801065d8..0x801068f7`, followed by 25 four-byte map-pair records at `0x801068f8..0x8010695b`. Full-width characters are normalized; separators and missing spaces are made readable (for example, `Yosyonke_Pad` becomes Yosyonke Pad). These are save-preview labels, not a catalog of every room in the game.

The save-preview writer at `0x800e94ec` reads map/secondary-map bytes (runtime `+0x10/+0x11`, or SC `0x128/0x129`), scans exactly 25 records, and writes **matched index + 1** to SC `0x12f` at `0x800e950c..0x800e9510`. Thus the names are one-based; ID 0 has no named entry. The app uses the stored preview byte, preserving the game's own summary rather than guessing from world flags. Repeated names below are intentional. Unmapped IDs retain their hexadecimal value.

| Preview ID | Display name | Map / secondary ID |
| --- | --- | --- |
| `1` / `0x01` | Flutter | `0x04` / `0x00` |
| `2` / `0x02` | Sulphur-Bottom | `0x3f` / `0x00` |
| `3` / `0x03` | Yosyonke Pad | `0x08` / `0x00` |
| `4` / `0x04` | Yosyonke Gallery | `0x0b` / `0x01` |
| `5` / `0x05` | Forbidden Island | `0x10` / `0x00` |
| `6` / `0x06` | Manda Pad | `0x24` / `0x00` |
| `7` / `0x07` | Pokte Ruins | `0x24` / `0x02` |
| `8` / `0x08` | Nino Pad | `0x17` / `0x00` |
| `9` / `0x09` | Nino Platform | `0x19` / `0x00` |
| `10` / `0x0a` | Ruminoa City | `0x19` / `0x01` |
| `11` / `0x0b` | Glyde Base Gate | `0x1f` / `0x00` |
| `12` / `0x0c` | Carlbania | `0x23` / `0x00` |
| `13` / `0x0d` | Kimotoma City | `0x29` / `0x00` |
| `14` / `0x0e` | Kimotoma Pad | `0x3c` / `0x03` |
| `15` / `0x0f` | Kimotoma Ruins | `0x3c` / `0x02` |
| `16` / `0x10` | Saul Kada | `0x48` / `0x00` |
| `17` / `0x11` | Manda Ruins | `0x14` / `0x03` |
| `18` / `0x12` | Nino Ruins | `0x35` / `0x03` |
| `19` / `0x13` | Saul Kada Ruins | `0x26` / `0x06` |
| `20` / `0x14` | Calinca Ruins | `0x2f` / `0x05` |
| `21` / `0x15` | Elysium Pad | `0x40` / `0x01` |
| `22` / `0x16` | Elysium | `0x4c` / `0x03` |
| `23` / `0x17` | Master's Room | `0x56` / `0x00` |
| `24` / `0x18` | Forbidden Island | `0x1d` / `0x01` |
| `25` / `0x19` | Kimotoma City | `0x3c` / `0x01` |

Private saves verify ID 8 in DuckStation's load menu. Public cards independently match the map-pair table for IDs 1 (Flutter), 3 (Yosyonke Pad), and 22 (Elysium). The remaining names come from the game's table and writer, not separate emulator visits to every place.

## Difficulty names and limits

SC `0x12e` restores to runtime `0x8009c80e`. Gameplay routine `0x8004de0c` accepts five levels (`0..4`) and selects one of five enemy-parameter columns; `0x800cf3f0` separately handles level 4. **The enum is not the four-item new-game menu order.**

| Stored ID | Display name | Evidence |
| --- | --- | --- |
| `0` | Easy | Eevee-Trainer earliest Easy upload 25998; endgame 25999 agrees. |
| `1` | Normal | Earliest Normal upload 26000; both private saves agree. |
| `2` | Unknown (ID 0x02) | Valid internal level, but no independently traced name/fixture yet. |
| `3` | Hard | Earliest Hard upload 26002; Hard endgame 26005 and Sangogi S-license upload 6812 agree. |
| `4` | Very Hard | Earliest Very Hard upload 26003; endgame 26004 agrees. |

[GameFAQs' save listing](https://gamefaqs.gamespot.com/ps/197897-mega-man-legends-2/saves) supplies labelled starting and ending samples; links and hashes are retained in the fixture manifest. Eevee-Trainer's cards are modified max-stat saves with the display timer reset to zero. All selected Legends 2 payloads pass the four checksums, but that does not establish unmodified gameplay. Upload 26001 is described as a Normal playthrough yet stores ID 3. The [MegaBoyEXE guide](https://gamefaqs.gamespot.com/ps/197897-mega-man-legends-2/faqs/8888) explains that license tests raise difficulty: Class C/B/S/SS correspond to Easy/Normal/Hard/Very Hard. Therefore the app describes the **saved difficulty level**, not necessarily the originally selected mode. Class A is a plausible explanation for internal level 2, but it is not implemented as a verified name.

The independent Sangogi card has 10:53:26 playtime and 263,580 Zenny. The GeorgeGX64 upload 25147 contains the same Legends 2 payload as Sangogi, and 26006 duplicates the Hard starting sample; neither counts as independent evidence. No save or ROM bytes are committed or redistributed.

## Research reader and verification

From the repository root:

```sh
python3 tools/inspect_mml2.py path/to/BASLUS-01140-DASH20.mcs
python3 -m unittest discover -s tools -p 'test_inspect_mml2.py' -v
MML2_LOCAL_FIXTURES=tests/fixtures/ps1/local/mml2 python3 -m unittest discover -s tools -p 'test_inspect_mml2.py' -v
```

The standard-library-only reader emits JSON and opens input only for reading. It supports one raw SC block or MCS, reports all four checksums independently, preserves unknown IDs, and separates unresolved record/flag bytes into `research`. It does not read whole-card containers, repair checksums or modify saves. A separate Rust decoder supplies the verified summary to the app's production game-details dispatch through the existing card/container parser. Checksum mismatch keeps observations available with `checksumOk: false`.

Five test cases pass with the local corpus enabled: time units/hour boundary, raw/MCS equivalence and input preservation; each checksum's damage detection and unchecked gaps; modulo overflow; unknown enums and flag bit order; malformed layout rejection; and hashes/known fields of both real saves. The default run skips the private corpus case. Private saves remain under the ignored fixture directory.

## Production display and next mapping work

The existing read-only game-details dialog shows **saved location, HH:MM:SS, Zenny, health and checksum status**, followed by difficulty and named equipped helmet, shoes, armor and three buster part slots. Equipment names use the published ID mappings described above. Unknown values remain `Unknown (ID 0xNN)`. Items/key items and weapon upgrades await mapping; raw story flags remain in the standalone research tool. No guessed completion percentage or item ownership is displayed. Support is shared by card, Local saves and snapshot inspectors. The production corpus test checks both private saves and nine public cards through MCS import and raw/GME/VGS normalization without changing inputs.

Next experiments should operate on copies in a separate emulator card: capture a baseline, change exactly one item/equipment/difficulty/location value, save into another position, and compare the explicitly packed bytes and flags. Check each new name against both game UI and code/table usage. Next confirm the 36-byte buster-parts list against the menu and controlled acquisition/removal, then map key-item flag IDs and the 20 weapon records. Gear ownership and equipped selectors are separate state; a name table alone does not establish an owned-item flag. Further inventory research is optional for save identification; difficulty level 2 would need a controlled license-test capture before naming it.
