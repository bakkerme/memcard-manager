# Pac-Man World US: read-only summary

The app supports `BA` / `SLUS-00439` / `PACMAN20` saves with exactly one
8192-byte reconstructed payload. Offsets below include the payload header;
an individual MCS adds a 128-byte directory wrapper. Whole-card extraction,
chain validation and container handling remain in the attributed MemcardRex
Core port. No bytes are repaired or written back.

Only this release is qualified. The PAL `SCES-00934` reader remains separate;
its lives, score and checksum claims are not applied to the US reader.

## Layout and displayed fields

Require exact header `43 00 13 01`, little-endian u32 length `0x784` at
`0x200` and u16 body version 2 at `0x208`. The verified US executable explicitly
writes this nonstandard header. Other releases still require their normal SC
headers. Unknown body length/version returns a details error while retaining
ordinary card browsing.

Four global bytes precede three `0x280`-byte profiles starting at `0x20c`.
The last-selected-slot word at `0x20a` does not hide any profile. Show fixed
**Game 1 of 3**, **Game 2 of 3**, **Game 3 of 3** identities.

| Field | Profile-relative layout | Display rule |
| --- | --- | --- |
| Occupancy | signed LE32 `+0xc` | Negative means empty; empty slots expose no progress, even with stale data |
| Quest completion | LE16 `+0x54 + 2*ID`, masked with 3 | Count state 3 over the 23 IDs below; “N of 23 quest stages cleared” |
| Friends rescued | Six LE16 words at `+0x5a + 2*i` | Count bit 1; “N of 6” |
| Saved playtime | signed LE32 `+0x24c` | Nonnegative ticks divided by 300, truncated, formatted H:MM:SS; negative is Unknown |

Quest IDs in exact executable table order:
`236, 230, 232, 234, 222, 224, 228, 30, 32, 34, 36, 38, 40, 42, 44, 52, 54, 56, 58, 46, 48, 50, 238`.
Special entries remain included. This count is not the game's percentage and
does not identify the current or saved level.

Game and profile `checksumOk` stay `null`. Native percentage, named/raw location,
lives, score, other progress flags and checksum-pattern diagnostics are not
exposed as fields. Friend names are not assigned. Time describes the saved
display counter; pause behavior, wall time and overflow reachability are unqualified.

## Evidence and provenance

Private October 18 continuation outputs and the October 19 review are retained in
ignored `tools/save_research/runs/review-d991f863/`. `REVIEW.md`,
`reviewer-artifact-audit.json`, `reviewer-dependency-check.json` and
`reviewer-validation.json` record the independent review. Preserve the collected
`outputs/research/` alongside `outputs/final/` for semantic evidence; private
saves, screenshots, RAM and game assets are not redistributable fixtures.

The exact extracted US executable SHA-256 is
`02483682359de99ed0adae24992b011c165c2ce363468da4bb3631b5de06c86a`.
The 23-entry pointer table is at `0x8010d078`; record IDs are at pointer+4.
Instruction anchors establish serialization/body size (`0x80074f14..0x80074f40`,
`0x800751b8..0x800751cc`), header writing (`0x80075020..0x80075050`), signed
occupancy (`0x80073608..0x80073614`), stage-state access and state-3 assignment
(`0x800be78c..0x800be80c`, `0x800bde84..0x800bdee0`), rescue bits
(`0x800bede4..0x800bee18`) and displayed timer conversion
(`0x800737dc..0x80073870`). The Rust implementation uses numeric layout facts;
no game code or candidate Python reader is bundled into production.

Both retained native Load RAM bodies exactly equal source bytes `0x208..0x98c`.
Screenshots independently corroborate occupancy, friends and time; stage counts
are code-traced. Two independent save files cover four occupied profiles:

| Save | Slot | Quest stages | Friends | Time |
| --- | --- | --- | --- | --- |
| Original | 1 | 5 of 23 | 1 of 6 | 0:55:09 |
| Original | 2 and 3 | Empty | — | — |
| Public | 1 | 22 of 23 | 6 of 6 | 12:13:22 |
| Public | 2 | 1 of 23 | 0 of 6 | 0:01:37 |
| Public | 3 | 1 of 23 | 0 of 6 | 0:01:39 |

The public listing claims 100 percent, but its native menu shows 99 and an
assumed reconstruction yields 98. The reader deliberately withholds percentage.
Native Load observations and subsequent public Game 3 Quest gameplay support
existing-save loading. No newly generated in-game save/flush/fresh-load cycle was
performed. Calibration deadline failures remain failures; this is not general
replay qualification. The disclosed BIOS scratch-frame write does not establish
whole-card preservation. Disc revision and other variants remain unqualified.

## Validation

Default Rust tests use constructed data, not private bytes. They check known-value
representations, slot independence, stale empty data, signed occupancy extremes,
stage masks/table neighbors, first/last friend bits, timer boundaries/negative
values, layout/version/release/region/identifier rejection and the unchanged PAL
header gate. Engine tests cover Raw/GME/VGS normalization, MCS reimport, malformed
allocation and both terminal-pointer bytes, serialized output and read-only behavior.

With the ignored review outputs present, run the hash-pinned two-save corpus:

```sh
cargo test --manifest-path src-tauri/Cargo.toml pacman_us_private_known_value_corpus --lib -- --ignored
```

Set `PS1_PACMAN_US_REVIEW` to the absolute review directory if relocated.
The corpus checks the independently established values above through the Rust
decoder and whole-card container paths and verifies original files stay unchanged.
It does not launch an emulator or certify new-save persistence. Broader checks:

```sh
npm test
npm run build
```

Integration validation on macOS (19 October 2026): 48 default Rust tests and 21
frontend tests passed; the separate two-save private corpus test passed; TypeScript
checking and the Vite production build passed. Private/hardware tests remain
ignored in the default suite. No native desktop UI or new emulator/save experiment
was performed for this integration.
