# Read-only PS1 game details

The original twelve-game coverage proposal, a Mega Man Legends 2 pilot, an Ape Escape reader, and eleven October 2026 exact-release readers are implemented as useful summaries. Select a save on a card, in Local saves, or in snapshot history and choose **View game details**. Existing Digimon World 2 details remain available.

These are narrow decoders for original PS1 saves. They do not edit, repair, convert regions, or write back to hardware. An unknown game remains browseable using ordinary save metadata. Known releases with unsupported identifiers or malformed layouts show a details error. A checksum mismatch warns while leaving decoded values available; an unknown checksum is explicitly unverified.

The [overnight reader audit](formats/overnight-readers.md) gives the new release matrix, field boundaries, provenance and limitations.

## Supported releases and fields

Only the following product codes are enabled. Each was exercised with a real save; upstream claims about other releases are not app support. Product codes alone cannot identify every disc revision or mod. Size, identifier, release-specific header, linked-block structure and selected layout markers are checked before decoding. Unknown enumerated values and undecoded location IDs remain explicit.

| Game | Product code / region | Blocks | Implemented fields |
| --- | --- | ---: | --- |
| Crash Bandicoot | SCUS-94900 / NTSC-U | 1 | Stored completion/gems/keys preview, raw level; rotating additive checksum; [boundaries and evidence](formats/crash-bandicoot-us.md) |
| Grandia | SCUSP94457 / NTSC-U | 1 | Disc/transition, time, Gold, raw location ID; integrity unverified; [evidence](formats/grandia-us.md) |
| Final Fantasy VII | SCES-00867 / English PAL | 1 | Disc, playtime, Gil, GP, saved location preview, raw story value; nine stored character records with names, levels, HP, MP, XP and party membership; checksum |
| Final Fantasy VIII | SLUSP00892 / NTSC-U | 1 | Disc, playtime, Squall/Laguna Gil, location ID; eight character records with levels, HP and XP; acquired Guardian Forces and XP; both checksum copies |
| Final Fantasy IX | SLES-02965 / English PAL | 1 | Disc, saved location preview, playtime, Gil, save count, location ID; nine stored character names and levels; checksum |
| Final Fantasy Tactics | SCUS-94221 / NTSC-U | 1 | Name, playtime, funds, location ID; registered unit names, levels and job IDs; parity checksum |
| Chrono Cross | SLUSP01041 / NTSC-U | 1 | Playtime, Gold, total/current stars, location ID; additive checksum |
| Castlevania: Symphony of the Night | SLUS-00067 / NTSC-U | 1 | Name, character, level, playtime, Gold, map discovery and kills; checksum unverified |
| Gran Turismo | SCUS-94194 / NTSC-U | 5 | Playtime, credits, days, garage count; passed/gold tests for B/A/IA licences; checksum |
| Gran Turismo 2 | SCES-02380 / English PAL | 4 | Credits, days, races, wins, garage count, career progress; passed/gold tests for all six licences; CRC32 |
| CTR: Crash Team Racing | SCUS-94426 / NTSC-U | 1 | Four adventure profiles with names, empty state, completion, trophies, relics, race CTR tokens, keys and gems; checksum; ghosts excluded |
| Spyro the Dragon | SCUS-94228 / NTSC-U | 1 | Three profiles with empty state, location, lives, dragons, gems, eggs; 35 per-level dragon/gem records; per-profile checksums |
| Tekken 2 | SLUS-00213 / NTSC-U | 1 | Playtime, 23 character unlock flags; deobfuscation and additive checksum; [evidence](formats/tekken2-us.md) |
| Tekken 3 | SLUS-00402 / NTSC-U | 1 | Playtime, 21 character unlocks, Tekken Ball/Theatre; character arcade usage and versus wins/losses; deobfuscation and checksum |
| Silent Hill | SLUS-00707 / NTSC-U | 1 | Eleven in-game save positions, empty state, playtime, difficulty, saves, health, location ID, Normal/Next Fear; shared ending unlocks; slot, preview and options checksums |
| Mega Man Legends 2 | SLUS-01140 / NTSC-U | 1 | Saved location preview, playtime to seconds, Zenny, current/maximum health in stored units, named saved difficulty, named equipped helmet/shoes/armor and three buster-part slots; all four additive checksums |
| Ape Escape | SCUS-94423 / NTSC-U | 1 | Four profiles; completion, saved stage, overall and per-level monkeys/coins remaining, best times, trial medals and exclusive targets, race coin rewards, gadgets/button assignments, extra lives, chips and pellets; checksum unverified |
| Azure Dreams | SLES-01219 / PAL | 3 | Time, money, buildings and game count; XOR integrity |
| Yu-Gi-Oh! Forbidden Memories | SLUS-01411 / NTSC-U | 1 | Deck, chest and owned card counts, StarChips, separate banks; integrity unverified in app |
| Klonoa: Door to Phantomile | SLUS-00585 / NTSC-U | 1 | Three profiles, selected stage/lives/rescued total and options; selected-record checksum |
| Digimon Digital Card Battle | SLES-03900 / PAL | 2 | Time, area, seen and owned cards, completion preview; byte integrity |
| Crash Bash | SCUS-94570 / NTSC-U | 1 | Four profiles, completion preview and collectible group totals; checksum |
| Pac-Man World | SCES-00934 / PAL | 1 | Three profiles, lives, time, friends and score; checksum |
| Pac-Man World | SLUS-00439 / NTSC-U | 1 | Three occupied/empty slots, quest stages cleared out of 23, friends rescued out of 6, saved playtime; integrity unverified |
| Bomberman Fantasy Race | SLUS-00823 / NTSC-U | 1 | Profile names, occupied/empty/unknown slots, owned normal/mirrored course-ticket counts; global and profile XOR8/sum8 checks |
| Mega Man Legends | SLUS-00603 / NTSC-U | 1 | Time, Zenny, buster-part counts and raw location IDs; integrity unverified in app |
| Kula World | SCES-01000 / PAL | 1 | Four slots, mode, score, level, fruit and elapsed seconds; integrity unverified |
| Tomb Raider II | SLUS-00437 / NTSC-U | 1 | Total level time, recorded dragon secrets, current position and cleared marker; additive checksum; [evidence](formats/tomb-raider2-us.md) |
| Tomb Raider | SLES-00024 / PAL | 1 | Current level and level-start inventory; checksum |

FFVIII and Chrono Cross use **P-bearing save product codes**: these are the filenames stored by the game, not replacement disc serials. GT/GT2 settings and CTR ghosts do not share the adventure decoder. The existing original-US Digimon World 2 decoder (`SLUS-01193`, two blocks, three profiles) keeps its JSON shape and detailed roster.

## Regional and interpretation boundaries

- Bomberman Fantasy Race US accepts only `BA` / `SLUS-00823` / `SYSTEM`, one block and exact `SC 11 01` header. Integrity failures and unfamiliar markers produce an explicit unknown slot with identity/progress withheld. Owned normal and mirrored tickets govern Time Attack access; they are not race wins or every ticket offered for purchase. Unmapped name glyphs remain explicit IDs. No currency, animals, ranking identities, course names or overall completion is inferred. The supplied deleted entry is browsable as deleted but was invisible to native Load; no recovery/repair is performed. See the [US specification](formats/bomberman-fantasy-race-us.md).

- Pac-Man World US accepts only `BA` / `SLUS-00439` / `PACMAN20`, its exact nonstandard `43 00 13 01` header, body length `0x784` and body version 2. Its separate reader shows fixed slots, quest completion counts (including special entries), friends and saved displayed time. Native percentage, location, lives, score and game checksum are not decoded. Existing-save native loading is evidenced; newly generated save/flush/reload persistence, timer pause behavior and disc revision remain unqualified. See the [US partial specification](formats/pac-man-world-us.md). The PAL reader and its checksum claims remain separate.
- PAL FFVII stores seconds, so its playtime is not divided by 50 or 60. The user's two SCES-00867 saves pass the checksum. Their time values are byte-level regressions, not an independently observed in-game display.
- PAL FFIX uses 50 frames per second. FFVIII's supported NTSC save stores seconds; Chrono Cross and Tekken 3 use 60-frame counters. Synthetic hour-boundary tests protect these differences.
- FFVIII has a nonstandard CRC lookup entry at index `0xff` (zero instead of `0x1ef0`). Both independent GameFAQs cards match this quirk. GF experience is shown without inferring levels: per-GF levelling differences have not been independently verified.
- FFVII/FFIX records describe stored data, including inactive or temporary characters; presence is not a claim that the character is recruited or playable. Only FFVII's party bytes identify current party membership.
- Spyro's passive `0x52` profile flag identifies unused adventures in the shared card. Its checksum is the sum of the profile bytes preceding the checksum. No 120% formula is guessed: the upstream map leaves its last 10% unresolved; actual dragon/gem/egg totals are shown.
- GT2 career progress uses the documented 219-event model. Early revisions and mods can display different totals; these variants are not certified. No unlicensed car database or editor implementation was incorporated.
- Silent Hill preserves all eleven positions inside each individual SILENTxx file. It does not merge files or playthroughs. Shared options/previews are checked as well as populated slots.
- Mega Man Legends 2 accepts only `-DASH20` through `-DASH24`, the observed one-frame SC header and normalized US title marker. Its display time uses 60-Hz ticks; the second counter is not treated as a required duplicate. All 25 save-preview location IDs use the game's name table. Easy/Normal/Hard/Very Hard map to 0/1/3/4; internal level 2 stays unknown. Difficulty is the saved level, which can change with license tests; equipment names cover 45 documented entries (the first save's equipped gear is also checked in-game); health is not converted to bars. Two private saves and nine public cards exercise positions 1, 2, 3 and 5, locations Flutter/Yosyonke Pad/Nino Pad/Elysium and all four starting modes; other location labels are table-derived, and other revisions are unverified.
- Full inventories, abilities, detailed story flags, compressed Chrono Cross records, car tuning and race records remain outside this first field set. The dialog names the remaining fields per game.
- Ape Escape accepts only the exact `SYS` identifier, one-frame SC header, normalized US title and version-1 `CRD0` marker. Four profiles use a 0x7a0 stride. Completion follows traced game code, including strict medal thresholds and race/challenge bonuses. Ordinary totals are 204 monkeys and 60 coins (50 level coins plus ten race rewards). Empty timers use `0xffffffff`; zero is preserved. Unknown name glyphs, gadget IDs and inconsistent counters stay explicit. No playtime, health, individual monkey names, minigame records or game checksum is inferred. See its [partial format specification](formats/ape-escape.md).

## Layout sources and attribution

Offsets refer to the reconstructed SC payload. MCS contributes a 128-byte directory header; GME/VGS headers belong to the card container and are removed by the existing parser. Multi-block data is joined in pointer order before decoding, including non-contiguous blocks. The new dispatch validates block counts, full 16-bit pointers, termination, continuation types and duplicate links without repairing input.

1. [RyudoSynbios/game-tools-collection](https://github.com/RyudoSynbios/game-tools-collection/tree/cc2bc1044986903833547369b007f57fef2216b9), commit `cc2bc1044986903833547369b007f57fef2216b9`: each game's `src/lib/templates/<game>/saveEditor/template.ts`, `utils.ts`, `utils/resource.ts`, plus Tekken 2's shared deobfuscation. MIT, copyright 2024 RyudoSynbios. Adapted field/checksum definitions and limited English text/name mappings; [license notice](licenses/game-tools-collection-MIT.txt).
2. [ShendoXT/memcardrex-plugins, SpyroEdit](https://github.com/ShendoXT/memcardrex-plugins/tree/6928e9c50600b2ea2c8c501f3424dcc219901133/SpyroEdit), commit `6928e9c50600b2ea2c8c501f3424dcc219901133`: `SpyroEdit/Additional/Spyro the dragon save.txt`, `mainWindow.cs`. MIT, copyright 2017 Shendo. Adapted profile layout and level names; [license notice](licenses/memcardrex-plugins-MIT.txt). Its documented offsets include the MCS header, which is subtracted here.
3. [zyzalfors/GT2SaveEditor](https://github.com/zyzalfors/GT2SaveEditor/tree/cb26bd78b43ea0bda7370489bca28dcf9aaa0668), commit `cb26bd78b43ea0bda7370489bca28dcf9aaa0668`: raw layout constants and event/licence representation in `GT2SaveEditor.py`. No reuse license was found. The reader independently implements those numeric format facts and standard CRC32; upstream code, assets and car database are not copied.

The MemcardRex parser attribution remains in place. Existing Digimon World 2 source credits are in README.md.

Mega Man Legends 2 uses an independently derived [partial format specification](formats/mega-man-legends-2.md), based on save packing/restoring routines, two private saves, nine public cards and DuckStation observations. Only numeric format facts and verified labels are implemented; no game code, ROM assets or save bytes are redistributed.

## Validation

[The fixture manifest](../tests/fixtures/ps1/manifest.json) records SHA-256 hashes, sources, uploaders, the actual save product codes and selected expected values. Shared complete/endgame cards were downloaded for all eleven games besides the user's PAL FFVII; a second FFVIII card independently checks its CRC quirk. Uploader completion claims are cross-checks for selected fields, not proof of every byte or of game playability. Those twelve summary decoders were checked without an emulator or console. The separate Legends 2 pilot used two private saves, nine public cards and DuckStation observations as described in its specification.

Downloaded cards and the user's saves stay in the ignored local fixture folder; public availability is not a redistribution license. Default tests use constructed data and existing licensed/project fixtures. [Acquisition and test instructions](../tests/fixtures/ps1/README.md) explain the explicit local corpus check.

Checks cover all fourteen summary dispatches, serialization, read-only behavior, malformed/truncated data, region/identifier rejection, timing, profile numbering/empties, checksum vectors and the FFVIII quirk. Mega Man Legends 2 tests also cover four independent checksum ranges, wrapping sums, unchecked gaps, unknown IDs and the separate time counter. The local corpus checks known values and checksums, checksum damage, raw/GME/VGS normalization and MCS re-import, including both private Legends 2 saves when present. It does not run ignored hardware tests.

The rebuilt macOS desktop app was also checked using a temporary card containing copies of the two Legends 2 saves. The first save's dialog showed Nino Pad, 03:41:26, 11,950 Zenny, health 128/128, matching checksums and the expected stored equipment/difficulty IDs. A subsequent Equipment-screen check verified the five nonzero gear names, now supplied by the decoder using published ID mappings. The location/difficulty update was also verified in the rebuilt desktop dialog: Nino Pad and Normal. Original save files were not modified.

## Research formats

[Mega Man Legends 2 (US)](formats/mega-man-legends-2.md) has a partial independently derived save specification and a standalone read-only JSON inspection tool. The verified summary fields are enabled in the production game-details dialog; the tool additionally exposes unresolved bytes for research.

[Ape Escape (original US)](formats/ape-escape.md) has an independently traced partial specification and read-only JSON research decoder. The supplied MCS and two independent GameFAQs cards exercise seven populated profiles and both empty/full progress. The corpus checks exact computed values plus container normalization and MCS re-import; checksum support remains unverified. New save persistence is not qualified.
