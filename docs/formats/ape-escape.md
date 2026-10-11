# Ape Escape: original US adventure save

Partial, read-only specification, researched on 2026-10-11. Supported identity:
**BASCUS-94423SYS**, product **SCUS-94423**, region **BA**, one 8192-byte block.
The companion `PKT` file, Japanese/PAL releases, mods and other revisions are
unsupported. A matching serial alone does not certify a modified executable.

Select a save and choose **View game details** to see four profiles, completion,
saved stage, monkeys and Specter Coins collected/remaining, per-level progress,
time-trial times/medals/targets, race coin rewards, unlocked and assigned gadgets,
extra lives, energy chips and pellet ammunition. Empty profiles remain visible.
Unknown glyphs, enums and inconsistent counters are exposed rather than guessed.

## Offset origin and layout gates

All `SC` offsets below include the SC header, relative to a reconstructed payload.
An MCS wrapper adds 128 bytes; a whole card requires directory/chain validation
first. The production card engine provides that boundary. The standalone research
reader accepts a validated one-block MCS or an explicitly identified SC payload;
it does not slice whole cards.

Required header: `SC`, icon mode `0x11`, one block, Shift-JIS title normalizing to
`Ape Escape`, and bytes `01 00 43 52 44 30 00 00` at SC `0x50..0x58` (version 1,
`CRD0`). Four profile allocations start at `0x180`, `0x920`, `0x10c0`, `0x1860`:
**stride 0x7a0**, not 0x780. All multibyte fields here are little-endian.

## Evidence and confidence

The supplied MCS SHA-256 is
`1bd7886ce5c1ed87ed2fdd73659cd135eb33aedbd14d1152e23a2709d6962473`.
It contains File1 at Dark Ruins: computed 10.2%, 23 monkeys, two level coins,
three extra lives, 68 chips, club/net/radar and three empty profiles. Stored
per-stage monkey counts agree with the capture-bit count in this profile.

Two independent DexDrive cards exercise seven populated profiles in total across
the three inputs, including partial progress, 100% progress and unset timers.
[WLau's card](https://gamefaqs.gamespot.com/ps/196614-ape-escape/saves/164) reproduces
the uploader's 100% and 83.3% labels from the traced formula. Its saved first name
is `WLaw1`; the second is `WLau2`. The second
[card](https://gamefaqs.gamespot.com/ps/196614-ape-escape/saves/4583), by
XoVoLcom28182, stores 100.0%, 100.0%, 89.2%, 98.5%; its uploader's completion labels
do not match those values. Those four values are byte-level regressions.
Full file hashes and selected expected fields are in the
[fixture manifest](../../tests/fixtures/ps1/manifest.json).

Claims below are **code-traced** (serialization/loading and consumer code), with
independent save corroboration where stated. A fresh isolated PCSX-Redux build-70
ARM64 process reached the supplied save's load preview: File1, Dark Ruins, 10.2%,
with No Data for Files 2–4 (retained capture at emulated frame 3096). This verifies
the preview values. After confirming Load, the entire first 0x740 bytes of
File1 matched restored RAM at 0x800e0018 exactly (frame 4776); the game resumed in
the Time Station. Saved stage is a save-preview value, not the post-load scene.
This corroborates the loader copy without independently observing every field. Synthetic tests verify behavior,
not field meaning. The pause display also showed three extra lives. The disposable emulator card
changed ten system-block bytes, while its SC save payload and all originals
remained unchanged; no scratch repair was applied. No new game-save persistence
experiment is claimed. Private
ROM, BIOS, disassembly, RAM dumps, screenshots and emulator logs stay under the
ignored `tools/save_research/runs/ape-escape-2026-10-11/` directory.

Extracted `SCUS_944.23` SHA-256:
`993b4e23509f23f38a196791c6c2a45eeb96c68a6b650da7787948abc6c25700`.
The PS-X EXE body loads at virtual `0x80010000`, after its `0x800`-byte header.
Packing routines `0x80095b3c`, `0x80095cec`, `0x80095d44` populate RAM
`0x800e0018`. `0x8001078c` copies 0x7a0 bytes into the chosen profile, preserving
the 16-byte name. Loader `0x80095e80` copies only the first 0x740 bytes back to RAM
and restores fields. Initialization `0x80010720` establishes `0xffff` empty markers
and File1–File4 names. Header writer: `0x8001088c`.

The save UI is archive member `0x186`, loaded at `0x80136000`. Level numeric facts
are in member `0x16` of `KKIIDDZZ.DAT`, addressed by `KKIIDDZZ.HED` descriptors
(high 12 bits sector count, low 20 bits sector offset). These are numeric format
facts, not redistributed executable code or game assets.

## Field ledger

`P` means the start of one profile. Confidence is code-traced unless indicated.

| Offset / width | Meaning and evidence | Boundaries / counterexamples |
| --- | --- | --- |
| P+0x00, u16 | Saved first-room ID; `0xffff` empty. UI `0x80137998` tests signed negativity. | Other negative markers rejected. Unknown nonnegative room IDs remain explicit. |
| P+0x02, u8 | Lives including current life. UI `0x80137bcc` displays `(value-1)&255` extra lives. | Do not label the stored byte itself as extra lives; zero preserves the game's wrap to 255. |
| P+0x03, u8 | Energy chips; same preview consumer reads the high byte. | No maximum or health conversion inferred. |
| P+0x04, u32 | Unlocked gadget bitmask, bits 0–7. | Higher bits reported; ownership is distinct from button assignment. |
| P+0x08..0x0b, four u8 | Triangle, Square, Circle, Cross gadget IDs. | IDs 0–7 use the gadget table; `0xff` unassigned; others unknown. |
| P+0x0c / 0x0d, u8 | Guided / explosive pellet counts, packer/load consumers. | Other ammunition/settings bytes not promoted. |
| P+0x18..0x27, 16 bytes | NUL-terminated custom font name; default names and public names corroborate letter/digit ranges. | Uppercase 0x10–29, lowercase 0x30–49, digits 0x6a–73, space 0x7f; other glyphs `[0xNN]`. |
| P+0x28..0x47, 32 u8 | Per-stage clear state: 0 not cleared, 1 cleared, 2 perfect clear. | Unknown states retain their value and contribute no clear points. Race progress uses reward flags instead; not equated with ending flags. |
| P+0x48..0x67, 32 u8 | Stored captured-monkey counters. Game total sums first 25, `0x800962b0`. | Reserved counters still affect the game's total; Stored counters are authoritative; unused capture-flag slots are not used to recalculate totals. |
| P+0x68..0x167, 128 two-byte records | Collected coin room ID + raw flag; first room `0xff` terminates. | Repeated rooms are valid (multiple coins). Ordinary level total 50. Unknown flags/rooms warned; unterminated lists bounded to 128. |
| P+0x2f4..0x5e3, 752 bytes | Eight monkey flag slots per room; **room 0 reserved**. Bit 0x02 means captured, corroborated against counters. | Other bits, individual monkey names and unused slots unresolved. WLau's Monkey Madness has 27 raw capture bits versus counter 24; this is not proof of corruption. Flags remain research-only; production uses counters. |
| P+0x664, u32 | Training unlock mask in research output. | Individual meaning not promoted. |
| P+0x668, u32 | Bit 0x10: Stadium Attack reward; bit 0x1000: Gladiator Attack reward. Five coins each. | Save-preview total `0x801377f8` and load `0x80096168` add these to level coins: ordinary total **60**, not 50. Other bits unresolved. |
| P+0x670, 0x671, 0x674, u8 each | Each contributes 8 completion points when exactly 5. | Exact scoring consumer verified; challenge meaning/names unresolved. Do not call these minigame unlocks. |
| P+0x690..0x70f, 32 u32 | Best times: 60 ticks/second; `0xffffffff` unrecorded. | Zero remains a stored zero, not absence. Stadium/Gladiator race times have no trial medal points. |

Gadget IDs/bits in order: Stun Club, Time Net, Monkey Radar, Slingback Shooter,
Super Hoop, Magic Punch, Sky Flyer, RC Car. IDs are distinct from alphabetic order.

Remaining unknowns include profile bytes 0x0e, capacities/settings, broader flags,
individual monkey identities, detailed story/ending semantics, health, elapsed
playtime, minigame scores and the game checksum algorithm. Allocation padding
0x740..0x79f is not read as gameplay data. No stored pointers are followed.

## Completion calculation

Routine `0x8009816c..0x8009831c` returns integer tenths of a percent. Reproduce it:

1. Sum first 25 stored monkey counters.
2. For each of the 19 collecting stages below, add 15 for clear state 1 or 25 for
   state 2; add 8/7/5 for gold/silver/bronze best time. Thresholds are strict `<`;
   equality with gold qualifies only for silver. Unrecorded times add zero.
3. Add 15 for Peak Point Matrix state 2 (stage index 30, P+0x46).
4. Add 15 for each of the two race reward flags.
5. Add twice the number of level coin records (exclude the ten bonus reward coins).
6. Add 8 for each of P+0x670, 0x671, 0x674 equal to 5.

Divide by ten, retaining one decimal. Normal complete progress:
204 + 19×25 + 19×8 + 15 + 2×15 + 50×2 + 3×8 = 1000, or 100.0%.
Never clamp unusual saved values to 100%; warn and retain the computed result.
The supplied profile is 23 + 5×15 + 2×2 = 102 (10.2%). WLau's second profile has
all collecting stages complete but no normal trial medals and no Peak Point
perfect-clear bonus: 833 (83.3%).

## Levels and medal targets

Stage indices are sparse. Monkey Madness aggregates rooms across internal stage
indices 24–29 into index 24; totals are 24 monkeys and ten coins. Do not display
the subareas as six independent scored levels. Room ranges are half-open.
The executable's first-room table and `0x8009764c` supply saved-stage mapping.
Level member 0x16 uses 16-byte records: +1 monkeys, +3 coins; +8/+a/+c are bronze,
silver and gold u16 tick thresholds. Target times below are exclusive.

| Index | Level | Rooms | Monkeys | Coins | Gold / silver / bronze (seconds) |
| ---: | --- | --- | ---: | ---: | --- |
| 1 | Fossil Field | 1–2 | 4 | 1 | 30 / 35 / 45 |
| 2 | Primordial Ooze | 2–3 | 6 | 1 | 35 / 47 / 60 |
| 3 | Molten Lava | 3–6 | 7 | 1 | 75 / 105 / 130 |
| 4 | Thick Jungle | 6–11 | 14 | 4 | 90 / 120 / 150 |
| 5 | Dark Ruins | 11–15 | 13 | 4 | 80 / 105 / 130 |
| 6 | Cryptic Relics | 15–19 | 8 | 1 | 65 / 85 / 110 |
| 7 | Stadium Attack | 19–20 | — | 5 reward | — |
| 8 | Crabby Beach | 20–22 | 8 | 1 | 70 / 85 / 100 |
| 9 | Coral Cave | 22–24 | 8 | 1 | 85 / 105 / 130 |
| 10 | Dexter's Island | 24–29 | 11 | 3 | 70 / 90 / 110 |
| 11 | Snowy Mammoth | 29–30 | 6 | 1 | 75 / 95 / 120 |
| 12 | Frosty Retreat | 30–33 | 9 | 3 | 70 / 90 / 120 |
| 13 | Hot Springs | 33–36 | 9 | 2 | 75 / 90 / 110 |
| 14 | Gladiator Attack | 36–37 | — | 5 reward | — |
| 15 | Sushi Temple | 37–40 | 12 | 3 | 55 / 70 / 90 |
| 16 | Wabi Sabi Wall | 40–45 | 10 | 3 | 60 / 80 / 100 |
| 17 | Crumbling Castle | 45–53 | 20 | 4 | 45 / 60 / 80 |
| 20 | City Park | 53–56 | 13 | 3 | 35 / 50 / 65 |
| 21 | Specter's Factory | 56–63 | 10 | 2 | 75 / 95 / 125 |
| 22 | TV Tower | 63–69 | 12 | 2 | 65 / 85 / 115 |
| 24 | Monkey Madness | 69–87 | 24 | 10 | 60 / 80 / 100 |
| 30 | Peak Point Matrix | 87–88 | — | — | — |

Times display `ticks/3600 : (ticks/60)%60 . floor((ticks%60)*100/60)`.
One tick below 30 seconds is 00:29.98; exactly 30 seconds is 00:30.00 and silver
at Fossil Field. The reader does not infer total playtime from best-time records.

## Reproduce and extend

```sh
python3 tools/inspect_ape_escape.py 'Test Save Files/ape-escape.mcs'
python3 tools/inspect_ape_escape.py extracted.sc --identifier BASCUS-94423SYS
python3 -m unittest discover -s tools -p 'test_inspect_ape_escape.py' -v
# Optional private normalized corpus, after acquiring/extracting inputs:
APE_ESCAPE_CORPUS=tools/save_research/runs/ape-escape-2026-10-11 \
  python3 -m unittest discover -s tools -p 'test_inspect_ape_escape.py' -v
cargo test --manifest-path src-tauri/Cargo.toml shared_completion_corpus -- --ignored
```

The production corpus check uses the existing engine for GME/raw/VGS normalization,
MCS reimport, fifteen-tile card structure, exact fields and unchanged inputs.
No hardware tests are needed. Research and production readers were checked against
all three inputs. Default Rust tests cover malformed layouts/releases, profile
stride and empties, strict medals, tick boundaries, reward/completion scoring,
unknown IDs/glyphs/bits and over-limit counters. No original input was modified.

Rejected hypotheses: 0x780 profile stride, room-minus-one flag indexing, duplicate
coin-room rejection, a 50-coin overall maximum, and speculative minigame labels
for the three completion bonus bytes. No checksum was selected from coincidental
matches. Directory XOR validity does not establish game-payload integrity.

Next useful experiment: check the in-game progress screens, then generate
baseline/no-op and single-monkey/single-coin saves on a
disposable card, comparing serialized deltas and reloading in a fresh process.
Resolve the three completion bonus bytes and remaining font glyphs before naming
them. Save/reload verification remains distinct from emulator-state restoration.

## External orientation and rights

[Ihavenoname248's incomplete save-structure notes](https://www.speedrun.com/ae1/guides/7u1js)
oriented the initial search; their card-relative positions were independently
traced and normalized. The guide's coverage is partial and is not copied as a
decoder implementation. [ApeEscapeRecomp archive documentation](https://github.com/mstan/ApeEscapeRecomp/blob/master/docs/AOT_OVERLAYS.md)
oriented archive/overlay extraction; descriptors and load addresses were checked
against the supplied disc. No upstream implementation code was incorporated.
Only independently implemented format facts and labels ship under this project's
GPL-3.0-or-later license; existing MemcardRex/Shendo attribution is preserved.
