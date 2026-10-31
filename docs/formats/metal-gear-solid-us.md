# Metal Gear Solid USA read-only summary

Supports directory prefix `BA`, product `SLUS-00594`, one 8192-byte reconstructed SC payload, and an 8-byte story identifier that starts with `G` and is printable ASCII. The observed filename shape is `BASLUS-00594G……`. VR training identifiers start with `V` and are rejected. `SLES-01370`, Disc 2 executable serial `SLUS-00776`, and `SLUS-00957` are not accepted. Header byte 2 may be `0x11`, `0x12` or `0x13`; byte 3 is one block. The save title is Shift-JIS at SC `0x4` for up to 64 bytes, stopped at the first `0x00`, then NFKC-normalized.

Offsets in the table are relative to one `0xa40`-byte bank. The three banks start at SC `0x100`, `0xb40` and `0x1580`. A field is shown only when all three banks are byte-identical, each has the structural markers below, and each CRC32 matches. The reader does not pick a bank when the copies disagree.

| Field | Offset / encoding | Confidence |
| --- | --- | --- |
| Structural marker | u32 LE `0` = `0xa40`, u32 LE `8` = `95`, u32 LE `12` = `2048` | Required before a bank is read. If no bank has these markers, game details are refused |
| Checksum | u32 LE at `4`, CRC-32/ISO-HDLC over bank `[8, 0xa40)` (2616 bytes) | Confirmed on the USA pack/load path and on a broken/repaired native pair. Three identical valid banks set `checksumOk` true. Three identical structural banks with bad CRCs set it false and withhold bank fields. Any disagreement sets `checksumOk` null and withholds bank fields. The save title is outside the checksum |
| Difficulty | u16 LE at `0x42` (first copy SC `0x142`) | `0` Easy, `1` Normal, `2` Hard, `3` Extreme. Easy and Normal were seen on native save lists. Hard and Extreme are the code-traced labels; their gameplay effects were not separately established. Other values stay `Unknown` plus the raw word |
| Weapons | ten signed16 current-ammo words at `0x62 + 2*id`, capacities at `0x76 + 2*id` | SOCOM, FAMAS, Grenade, Nikita, Stinger, Claymore, C4, Stun Grenade, Chaff Grenade, PSG1. Ammo `0` is still carried (SOCOM `0/217`). Ammo `-1` is absent (Claymore). Other negative words stay unknown. Native anchors include Claymore `2/5`, Stun Grenade `14/31` and Chaff Grenade `7/29` |
| Rations, medicine, diazepam | signed16 at `0xa4`, `0xa6`, `0xa8` | Positive counts were checked in the item menu, including medicine `4` while the menu capacity showed `1`. The count is shown as stored. Zero is not confirmed absence. Negative words stay unknown. No item capacity is stored or inferred |
| ID card | signed16 at `0xac` | Level `7` was described by the item menu as opening level-7 doors. Levels `1` through `6` are the same word and were not each cold-loaded. Anything else stays unknown |
| Stealth | signed16 at `0x9e` (item 10) | `1` available, `-1` absent, confirmed by a one-variable cold boot. Other words stay unknown. This is current inventory, not a clear count or an ending |
| Bandana | signed16 at `0xa0` (item 11) | `1` available, `-1` absent, confirmed by a separate cold boot. Same limit as Stealth |
| Life | signed16 at `0x56` | Stored life. Negative words stay unknown. A stored value above max life is kept; a native load of stored `1000` was clamped to max life `768` on screen |
| Max life | signed16 at `0x58` | Stored maximum. Only a positive word is shown as a maximum |
| Stage | 16-byte ASCII at `0x20`, stopped at `0x00` | `s16d` is Cmnd room and `s18a` is Spply Rte. Other ids stay raw. `rank` is not renamed Dock. A non-ASCII id is unknown |
| Saved title | SC `0x4`, Shift-JIS | The save-screen label. It is not a completion percentage and not proof of the live scene |
| Playtime | u32 LE ticks at `0x10` | 60 ticks per second. The dialog shows `hh:mm:ss` plus the tick count. The in-game label truncates to minutes (`09:37`, `09:39` on the native controls) |
| Save count | u16 LE at `0xf0` | Increments across native saves and wraps `65535` to `0` |
| Continues | signed16 at `0xee` | The stored count. A cold load of `321` stayed `321` through Game Over and became `322` only after Continue. An immediate Codec save can still store the older count. The exact update boundary is unconfirmed. Negative words stay unknown |

## Not decoded

These were requested or suggested elsewhere and are not fields in this reader:

- Story completion percentage. Examined story screens show difficulty, time and area. `COMPLETE` on a file dialog is I/O status.
- Clear count. A candidate word at `0x40` was not confirmed.
- Tuxedo and ending history. Candidate bytes around `0x44` were not confirmed. Stealth or Bandana possession is not an ending.
- Radar mode and PAL key state.
- Disc number. Disc 2 story saves in the research still use directory product `SLUS-00594`.
- Item 12 and the other item words. An early Stealth-at-item-12 lead was corrected.
- Codec contact names and any other bank bytes.

Changing `0x40`, `0x46` or `0xaa` does not change the displayed summary. Those probes are in the unit test so a later edit cannot silently start reporting them.

## Banks and checksum

The tested USA loader calls bank 0. This reader still refuses to choose among disagreeing copies. A uniform CRC failure does not show the damaged values with a warning; the title stays visible and the bank fields are withheld. Padding after the third bank (SC `0x1fc0` onward) is outside every CRC.

No original GameFAQs card is committed. Private saves stay private. The unit tests rebuild the research decoder's synthetic `sample()` payload in memory and pin SHA-256 `545bce6ebee5aeb6be7bdb9ff59ef5aa6734e85814e635303b29f77c3aa3155a`. That hash checks the byte layout. It is not a native save. Native anchor values are written into that layout at the offsets above.

## Source jobs

- Job `847c6930`: difficulty, weapons and ammo, rations, medicine, diazepam, ID card, life, max life, stage and title, playtime, save count, continues, and the per-bank CRC32.
- Job `c2c8f3aa`: Stealth at item 10 and Bandana at item 11. Clear count, Tuxedo and endings stayed unconfirmed there, and they stay unconfirmed here.

The Nieko Maatjes note at <https://nieko.net/blog/metal-gear-solid-mgs1---all-weapons-and-items-hack> is a citation for an early difficulty-offset lead only. The saved marker is the 16-bit word at SC `0x142`, not a single byte. No editor source was copied. Shendo's MGSEdit is not the layout used here, and its `SLES-01370` claim is not support for that release.

Disc 2 cue/bin packaging and any `extra_assets` assembler path are unchanged. The research handoff did not include assembler code, and the current package assembler takes one disc dependency. Story saves from either disc are still `SLUS-00594` files.
