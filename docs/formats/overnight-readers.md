# October 2026 overnight research: read-only archive summaries

The production readers use reconstructed save-payload offsets (including the header). The original card-chain and
MCS/GME/VGS handling stays in the existing engine. These readers accept only the product
code, region, identifier, block count and release-specific layout markers. Most use an
SC header and observed title pattern; the exact US Pac-Man exception is specified below.
They show a useful subset of a save for archive browsing. They do not edit, repair, or
write saves, and no checksum success is presented as proof of in-game loadability.

## Readiness at the practical summary threshold

The original source of this assessment is the **2026-10-13 09:18:34 UTC** Spark snapshot in
`tools/save_research/runs/transfers/snapshots/spark/20261013T091834Z-b81b23c0/`,
not the older October 12 closeout. Its `results/overnight-20261012T213658Z/FINAL-LEDGER.md`
and the later `runs/overnight-review-2026-10-13/REVIEW.md` record the handoffs.
The later review independently reran 2,152 main research cases plus a Kula release
negative, verified 2,419 archive files, and reconstructed selected native save payloads.
Research cases include synthetic and code-model cases; they are not all gameplay proof.
The ignored recovered `recovery/parsers/`, suites and native evidence are private
provenance, not production code or redistributable fixtures.

The Pac-Man World US row was superseded by the October 18 continuation and October 19
local review in ignored `tools/save_research/runs/review-d991f863/`. Its exact US disc
is now present and hash-verified. The review reran all 23 research cases, verified
386 artifact entries, compared both native RAM bodies to source saves, inspected the
seven cited screenshots and independently recovered the 23-entry quest table from
the verified executable. The [US specification](pac-man-world-us.md) records the
accepted scope and the separate Rust verification commands.

| Release | Readiness | Production summary or concrete gap |
| --- | --- | --- |
| Azure Dreams SLES-01219 PAL | Ready, integrated | Time, money, buildings, game count; 13,470G to 9,470G native purchase saved and freshly loaded. Town promotion and inventory excluded. |
| Yu-Gi-Oh! Forbidden Memories SLUS-01411 US | Ready, integrated | Separate deck/chest ownership totals and StarChips per bank. ID 8/167 exchange and option change survived save/fresh load. Bank validator and English card list excluded. |
| Klonoa SLUS-00585 US | Ready, integrated | Three selected profile previews, lives, rescued total, control and sound. Options survived save/fresh load; stage-progress persistence and on-disk repair remain open. |
| Digimon Digital Card Battle SLES-03900 PAL | Ready, integrated | Time, area, seen cards, owned copies, completion preview and checksum. Profile no-op save/fresh load; natural card awards still open. |
| Crash Bash SCUS-94570 US | Ready, integrated | Four profiles, preview completion and collectible group totals with checksum. Name change saved/fresh loaded; group-to-minigame IDs unknown. |
| Pac-Man World SCES-00934 PAL | Ready, integrated | Three profiles, lives, time, friends and score with checksum. Native load verified; percentage and normal save remain open. |
| Mega Man Legends SLUS-00603 US | Ready, integrated | Gameplay time, Zenny, owned/equipped buster-part counts and raw location IDs. Native equipment UI and public save load; detailed names and changed-equipment save remain open. Game checksum not checked in this reader. |
| Kula World SCES-01000 PAL | Ready, integrated | Four occupied slots, mode, score, level, fruit and elapsed seconds. Arcade save/fresh load; checksum and other modes remain open. |
| Tomb Raider SLES-00024 PAL | Ready, integrated | Current level, available level starts and their recorded weapon/medipack/Scion inventory. Native load observed; current checkpoint inventory, time and normal save remain open. |
| Crash 3 SCUS-94244 US | Partial | Lives and fruit persist, but completion and collectible meanings remain unknown. Only counters would be a thin archive summary. |
| Ghost in the Shell SCES-01050 PAL | Partial | Checksum, option and grade encoding traced, but mission identities and meaningful progress summary remain unresolved. |
| Point Blank 2 SCES-02180 PAL | Partial | High-score pools and formatting traced; record identities and normal save remain unresolved. |
| Worms World Party SLUS-01448 US | Partial | No-op save/load and rank/stat writers traced; statistic columns and mission award meanings are not yet clear enough for labeled summaries. |
| Monster Rancher SCES-02872 PAL | Partial, runtime blocked | Six statistics and frozen records traced, but boot movie queue stalls; save/load qualification absent. |
| Um Jammer Lammy SCUS-94448 US | Partial, runtime blocked | Settings labels traced; forced exit 137 left no clean save/reload evidence or useful progress summary. |
| Pac-Man World SLUS-00439 US | Ready, integrated | Three occupied/empty slots, quest stages cleared out of 23, friends rescued out of 6 and saved playtime. Native existing-save load evidenced; percentage, location, lives, score, integrity and new-save persistence excluded. Separate US header/layout gate. |
| Bomberman Fantasy Race SLUS-00823 US | Blocked | Time formatting is matching-source derived; matching US disc is missing, and profile ownership/integrity is not qualified. |

All entries are exact releases. Other regions and revisions are unsupported. The final
research ledger leaves complete-format objectives open, which does not prevent these
bounded read-only summaries.

## Implemented field boundaries

- **Azure:** observed `SC 13 03`, identifier `00`, three blocks. Preview ticks
  `0x21c` are divided by 60; money `0x224`, buildings `0x228`, game count
  `0x22c`. The second money copy is `0x2d5c`; disagreement is a note. The
  XOR of little-endian words `0x208..0x6000` is stored at `0x204`.
- **Yu-Gi-Oh:** observed `SC 13 01`, identifier `-YUGIOH`. Banks begin at
  `0x200` and `0x880`; each has forty little-endian deck IDs, 722 chest
  quantity bytes at `+0x50`, StarChips at `+0x5e0`, and a raw save sequence
  at `+0x404`. Only IDs 1–722 count as known card IDs. Each bank is shown
  separately; the retail validator is complex and is not represented as a
  passing checksum here.
- **Klonoa:** observed `SC 13 01`, identifier `KLONOA`. Three profiles have
  primary records at `0x204 + n*0xf8`, four `0x38`-byte histories and options
  at `+0xe0`. A valid latest-history index selects the preview. The selected
  record uses thirteen little-endian words with a wrapping left-shift/add
  checksum at `+0x34`; options and other records are outside that check.
  The UI rolls up checks for occupied profiles with valid history selectors;
  an invalid selector or all-empty profiles leave the overall preview status
  unknown. A mismatch in any selected preview reports a mismatch.
  Rescued total is the sum of twelve bytes at record `+24..+36`, not the
  previously hypothesized popcount. The stage index uses the game's sparse
  fourteen-entry mapping before a Vision label is formed.
- **Digimon Digital Card Battle:** observed `SC 13 02`, identifier `_A`.
  Profile length `0x2774` is required. Byte XOR and byte sum cover
  `0x200..0x2974` and are stored at `0x2974`. Time at `0x224` uses 60 ticks
  per second. The 301 collection bytes at `0x16b2..0x17df` give seen flag
  `0x40` and low-three-bit owned quantity. Stored seen count is checked
  against the flags.
- **Crash Bash:** observed `SC 12 01`, empty identifier. Four `0x194`-byte
  profiles start at `0x200`. Preview completion follows the native group
  formula over five 33-byte arrays. Individual reward names are unknown.
  The wrapping sum over little-endian words `0x200..0x850` is at `0x85c`.
- **Pac-Man World PAL:** observed `SC 11 01`, `PACMAN20` and length `0x784`.
  Three `0x280`-byte profiles start at `0x20c`. A negative stored life word
  marks an empty slot; other values display as stored+1. Nonnegative timer
  ticks divide by 300. Friend count uses bit 1 in six halfwords; score sums
  native level value words times five. Byte sum `0x208..0x98c` is stored at
  `0x204`. No percentage is inferred.
- **Pac-Man World US:** `BA` / `SLUS-00439` / `PACMAN20`, one block with
  exact header `43 00 13 01`, length `0x784` at `0x200` and version 2 at
  `0x208`. Three `0x280`-byte profiles start at `0x20c`. Signed word `+0xc`
  below zero means empty; no progress is shown for empty slots. Quest completion
  counts low-two-bit state 3 at `+0x54 + 2*ID` for the exact 23-entry US quest
  table, including special entries. Friends count bit 1 in six halfwords at
  `+0x5a`. Nonnegative signed ticks at `+0x24c` divide by 300; negative time
  is unknown. No native percentage, location, lives, score or game integrity is
  inferred. See [evidence and limits](pac-man-world-us.md).
- **Mega Man Legends:** observed `SC 11 01`, `-DASH00` or `-DASH01`, and a
  patterned title clock. Gameplay ticks `0x180` use 30 per second; Zenny
  stores ten-unit increments at `0x184`. Buster-part ownership is flags
  `0x500..0x51f` in MSB-first order; equipped IDs are at `0x208`. The
  more complex game integrity algorithm is not ported in this summary.
- **Kula World:** observed `SC 11 01`, `KULA` and exact `KULA WORLD` title.
  Four `0x270`-byte slots start at `0x378`; occupied marker `+0x268` must
  be 0 or 1. Score, level, mode and fruit use observed slot fields. No game
  checksum is established.
- **Tomb Raider:** observed `SC 11 01`, `TOMBRAID` and exact title. The
  signed current level is at `0x33e`; available level-start records are
  14-byte entries from `0x200`, with bit `0x100` indicating a load-menu
  start. A negative byte sum over `0x200..0x357` and `0x358..0xf58` is
  stored at `0x357`. Inventory fields are explicitly level-start snapshots.

Every reader receives exactly the expected reconstructed payload size from dispatch.
Unknown titles/revisions, wrong regions and wrong identifiers return a details error;
the save remains browsable by ordinary card metadata. Unknown checksums are `null`,
while a known mismatch is `false`. None of the new readers mutates input.

## Regression and provenance

Synthetic tests cover Azure integrity/time and corruption, Yu-Gi-Oh's bank
separation and out-of-range IDs, and Klonoa's sparse stage, selection and
record checksum. Ignored corpus tests use the retained snapshot's hashed
MCS saves and assert known values across all nine readers. Those files are
never copied into the repository. Run them when the ignored snapshot exists:

```sh
PS1_RESEARCH_SNAPSHOT="$PWD/tools/save_research/runs/transfers/snapshots/spark/20261013T091834Z-b81b23c0" \
  cargo test --manifest-path src-tauri/Cargo.toml exact_release_research_samples --lib -- --ignored
npm test
npm run build
```

Pac-Man US has separate Rust tests for field boundaries, unsupported layouts,
serialization, Raw/GME/VGS normalization, MCS reimport, input preservation and
malformed terminal links/allocation. Its opt-in, hash-pinned real-save corpus test
uses the newer review directory rather than the October 13 snapshot; commands
are in the [US specification](pac-man-world-us.md#validation).

The production Rust code uses independently implemented numeric layout facts
and observed labels. It does not import candidate Python code, game assets or
save bytes. The existing MemcardRex Core parser attribution remains in place.
