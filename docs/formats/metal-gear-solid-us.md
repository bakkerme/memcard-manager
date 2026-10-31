# Metal Gear Solid USA read-only summary

Supports directory prefix `BA`, product `SLUS-00594`, one 8192-byte reconstructed SC payload, and an 8-byte story identifier that starts with `G` and is printable ASCII. The observed filename shape is `BASLUS-00594GHHMMCLI`. VR training identifiers start with `V` and are rejected. `SLES-01370`, Disc 2 executable serial `SLUS-00776`, and `SLUS-00957` are not accepted. Header byte 2 may be `0x11`, `0x12` or `0x13`; byte 3 is one block. The save title is Shift-JIS at SC `0x4` for up to 64 bytes, stopped at the first `0x00`, then NFKC-normalized.

Offsets in the bank table are relative to one `0xa40`-byte bank. The three banks start at SC `0x100`, `0xb40` and `0x1580`. A bank field is shown only when all three banks are byte-identical, each has the structural markers below, and each CRC32 matches. The reader does not pick a bank when the copies disagree. Filename fields come from the directory identifier and stay visible when the banks are withheld.

MGSEdit MCS offsets include the 128-byte directory header. `MCS = SC + 0x80`, and the first bank is SC `0x100`, so a bank offset is `MCS - 0x180`.

| Field | Offset / encoding | Confidence |
| --- | --- | --- |
| Structural marker | u32 LE `0` = `0xa40`, u32 LE `8` = `95`, u32 LE `12` = `2048` | Required before a bank is read. If no bank has these markers, game details are refused |
| Checksum | u32 LE at `4` (MCS `0x184`), CRC-32/ISO-HDLC over bank `[8, 0xa40)` (2616 bytes) | Confirmed on the USA pack/load path and on a broken/repaired native pair. Three identical valid banks set `checksumOk` true. Three identical structural banks with bad CRCs set it false and withhold bank fields. Any disagreement sets `checksumOk` null and withholds bank fields. The save title and the directory filename are outside the checksum. The editor writes this checksum for the first bank only; that behavior is not used |
| Difficulty | u16 LE at `0x42` (MCS `0x1c2`) | Native. `0` Easy, `1` Normal, `2` Hard, `3` Extreme. Easy and Normal were seen on native save lists. Hard and Extreme are the code-traced labels. Other values stay `Unknown` plus the raw word |
| Filename difficulty | minute characters of `GHHMMCLI`, bit `0x40` | Editor. The tens digit bit is Hard or Extreme. The ones digit bit is Normal or Extreme. Easy and Normal bits agreed with native titles in job 377ea84c. Hard and Extreme follow the editor's encode path. This is not a replacement for the bank difficulty word |
| Weapons | ten signed16 current-ammo words at `0x62 + 2*id` (MCS `0x1e2`), capacities at `0x76 + 2*id` (MCS `0x1f6`) | Native counts, editor names and order: Socom, FA-MAS, Grenades, Nikita, Stinger, Claymore, C4, Stun grenades, Chaff grenades, PSG-1. Ammo `0` is still carried. Ammo `-1` is absent. Other negative words stay unknown; the editor would mask the sign bit and clamp at 9999, and this reader does neither |
| Items | 24 signed16 words at `0x8a + 2*id` (MCS `0x20a`) | Editor order. The save.txt prose says 23 words; the editor arrays and the name list are 24 and end at max rations. Native words: Stealth and Bandana (`1` available, `-1` absent, other values including `0` unknown), rations, medicine, diazepam (zero is not confirmed absence; negatives stay unknown), and ID card levels `1` through `7`. The other words follow the editor: a positive word is available and zero or a negative word is absent. Timer bomb is the editor's "seconds left" count. Those other words were not cold-boot re-proven |
| Max rations, max medicine, max diazepam | u16 LE at `0xba`, `0xbc`, `0xbe` (MCS `0x23a`, `0x23c`, `0x23e`) | Native in job 377ea84c. Menu rations `4/5` became `4/7`, medicine `1/1` became `1/6`, and diazepam `1/9` became `1/12`. The stored maximum is shown, including `0` and `65535`. Pickup clamping was not tested |
| Radar | u16 LE at `0x44` (MCS `0x1c4`), mask `0x0800` | Native. Set hides the radar; clear leaves it visible. Job 377ea84c cold-loaded visible, then hidden, then visible again |
| Option flags | the same u16 at `0x44` | Raw. Bit `0x0800` is the radar field. The other bits are shown as the whole word and are not named. Observed original words included `0x2400` and `0x2000` with this bit clear |
| Generic flags | u32 LE at `0x3c` (MCS `0x1bc`) | Raw. The editor marks this word unknown. No bit is named |
| ID card | signed16 at `0xac`, item 17 | Native level `7` was described by the item menu as opening level-7 doors. Levels `1` through `6` are the same word and were not each cold-loaded. Anything else stays unknown |
| Life | signed16 at `0x56` (MCS `0x1d6`) | Native stored life. Negative words stay unknown. A stored value above max life is kept. The editor UI clamps at 1400; this reader does not |
| Max life | signed16 at `0x58` (MCS `0x1d8`) | Native stored maximum. Only a positive word is shown as a maximum. The editor UI clamp is not applied |
| Stage | 16-byte ASCII at `0x20`, stopped at `0x00` | Native. `s16d` is Cmnd room and `s18a` is Spply Rte. Other ids stay raw. `rank` is not renamed Dock. A non-ASCII id is unknown. This is separate from the filename location letter |
| Filename location | character `L` of `GHHMMCLI` | Native for `A` Dock, `]` Cmnd room and `^` Spply Rte. The other letters `B` through `_` use the editor table and are marked editor. The editor spelling Blast Furnance is kept. Any other byte stays raw with no name |
| Filename time | `HHMM` of `GHHMMCLI` | Native. Bit `0x40` is stripped from each minute character before the digits are read. Minutes `60` or more, or a non-digit after that strip, stay unknown. `99:59` is accepted as digits |
| Filename code, disc, clear marker | character `C` of `GHHMMCLI` | The raw character is always shown. Disc 2 is the editor rule `C >= H`; anything lower is Disc 1. `@` is the editor's new-game marker. Other codes are "cleared" when `(C - 0x40) & 7` is nonzero and "no clear bits" when it is zero. That integer is not shown. Job 377ea84c saw `C` beside a native Disc 1 label and `H` beside Disc 2; that was not an isolated disc edit and does not confirm a clear count |
| Saved title | SC `0x4`, Shift-JIS | The save-screen label. It is not a completion percentage and not proof of the live scene |
| Playtime | u32 LE frames at `0x10` (MCS `0x190`) | Native 60 frames per second, which is also the editor's NTSC rate. The dialog shows `hh:mm:ss` plus the frame count. The in-game label and the filename truncate to minutes. Job 377ea84c's title times agree with that minute floor (`09:36`, `07:06`, `06:35`). The editor's PAL 50 Hz rate is not used. The editor UI clamps at `99:59:59`; the stored count is not clamped |
| Save count | u16 LE at `0xf0` (MCS `0x270`) | Native. Increments across native saves and wraps `65535` to `0`, so the high byte is part of the count. The editor text calls this byte an 8-bit counter |
| Continues | signed16 at `0xee` | Native stored count. A cold load of `321` stayed `321` through Game Over and became `322` only after Continue. An immediate Codec save can still store the older count. The exact update boundary is unconfirmed. Negative words stay unknown. This word is not in the editor map |
| Meryl | byte at `0x15a` (MCS `0x2da`), mask `0x08` | Editor only. save.txt draws the byte as `????O???` and says bit O means Meryl dies. The same left-to-right bit diagram names the radar mask, so O is bit 3. mainWindow.cs does not implement the flag, and job 377ea84c did not cold-load it. The value says whether bit O is set. It does not say the other bits are understood |
| Torture flags | the same byte at `0x15a` | Raw. Shown so bits other than O stay visible |

## Not decoded

These were requested or suggested elsewhere and are not fields in this reader:

- Story completion percentage. Examined story screens show difficulty, time and area. `COMPLETE` on a file dialog is I/O status.
- A numeric clear count. The filename marker above is the editor's qualitative rule. The bank word at `0x40` is still not a clear count.
- Tuxedo and ending history. No option bit is named as either. Stealth or Bandana possession is not an ending.
- A bank disc number. Disc 2 story saves in the research still use directory product `SLUS-00594`. The filename disc line is the editor rule, not a separate bank field.

Changing `0x40` or `0x46` does not change the displayed summary. Those probes are in the unit test so a later edit cannot silently start reporting them. `0xaa` is the PAL card key and does change that item record.

## Banks and checksum

The tested USA loader calls bank 0. This reader still refuses to choose among disagreeing copies, including a valid first bank whose mirrors differ. A uniform CRC failure does not show the damaged values with a warning; the title and filename fields stay visible and the bank fields are withheld. Padding after the third bank (SC `0x1fc0` onward) is outside every CRC.

The editor's save.txt says the mirror banks are not used when loading, and its checksum function covers one bank. Both are documented and not followed.

No original GameFAQs card is committed. Private saves stay private. The unit tests rebuild the research decoder's synthetic `sample()` payload in memory and pin SHA-256 `545bce6ebee5aeb6be7bdb9ff59ef5aa6734e85814e635303b29f77c3aa3155a`. That hash checks the byte layout. It is not a native save. Native anchor values are written into that layout at the offsets above.

Job 377ea84c's three synthetic SC fixtures are committed under `src-tauri/src/card/game_details/testdata/`. They are zero-filled parser boundaries, not native saves. The test checks their SHA-256 sidecars, then stamps the structural markers and CRC before decoding. The capacity words in those files are the values under test.

## Source jobs

- Job `847c6930`: difficulty, weapons and ammo, rations, medicine, diazepam, ID card, life, max life, stage and title, playtime, save count, continues, and the per-bank CRC32.
- Job `c2c8f3aa`: Stealth at item 10 and Bandana at item 11. Clear count, Tuxedo and endings stayed unconfirmed there, and they stay unconfirmed here.
- Job `377ea84c`: max rations, max medicine, max diazepam, radar bit `0x0800`, and filename time plus location codes `A`, `]` and `^`. Filename disc/clear, Meryl and a story completion percentage stayed unconfirmed there. Disc, the new-game marker and Meryl are shown from the editor map anyway, with that limit in the dialog notes.

Field names and the remaining offsets were checked against Shendo's MGSEdit (codename Liquid), [memcardrex-plugins commit `6928e9c`](https://github.com/ShendoXT/memcardrex-plugins/blob/6928e9c50600b2ea2c8c501f3424dcc219901133/MGSEdit%20(codename%20Liquid)/Liquid/Additional/Metal%20Gear%20Solid%20save.txt), MIT, copyright 2017 Shendo. The files used are `Additional/Metal Gear Solid save.txt` and `mainWindow.cs`. No editor source was copied. The same commit's license notice is [shared with SpyroEdit](../licenses/memcardrex-plugins-MIT.txt). Its `SLES-01370` claim is not support for that release.

The Nieko Maatjes note at <https://nieko.net/blog/metal-gear-solid-mgs1---all-weapons-and-items-hack> is a citation for an early difficulty-offset lead only. The saved marker is the 16-bit word at SC `0x142`, not a single byte.

Disc 2 cue/bin packaging and any `extra_assets` assembler path are unchanged. Story saves from either disc are still `SLUS-00594` files.
