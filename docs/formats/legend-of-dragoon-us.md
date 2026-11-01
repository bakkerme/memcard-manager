# The Legend of Dragoon USA read-only summary

Supports directory prefix `BA`, product `SCUS-94491`, and an 8-byte story identifier `drgn` plus four decimal digits from `0000` through `0255`. One 8192-byte reconstructed SC payload. `SCUS-94584`, `SCUS-94585` and `SCUS-94586` are later disc serials and are not save product codes. `SCES-03047` and `SCESP03047` are not accepted. The icon-frame byte is not gated: no USA header sample was available to pin it. Byte 3 must be one block. The save title is Shift-JIS at SC `0x4` for up to 64 bytes, stopped at the first `0x00`, then NFKC-normalized. An empty title is shown as `Saved game`. A malformed title is rejected.

Offsets are relative to the reconstructed SC payload. LODEdit's offsets include the 128-byte MCS directory header, so `SC = LODEdit - 0x80`. The plugin's `currentSaveSlot * 0x600` multiplier stays 0 in the pinned window and is not applied. Dragoon-Editor reads a raw `.mcr` data block directly, which is the same SC payload. Displayed party, gold, playtime and Dragoon spirits are the game copy. A differing save-file preview is reported in a note and does not replace those values.

Confidence is editor-backed. Nothing here was checked by a save/reload on the USA game.

| Field | Offset / encoding | Confidence |
| --- | --- | --- |
| Party 1–3 | u32 LE game copies at `0x288`, `0x28c`, `0x290`. Previews at `0x188`, `0x18c`, `0x190` | Both editors. `0` Dart, `1` Lavitz, `2` Shana, `3` Rose, `4` Haschel, `5` Albert, `6` Meru, `7` Kongol, `8` Miranda, `0xffffffff` Empty. Any other word stays `Unknown` plus the raw value |
| Gold | u32 LE at `0x294`. Preview at `0x19c` | Both editors use this address. Dragoon-Editor reads 4 bytes. LODEdit reads and writes only the low 16 bits. The stored 32-bit word is shown, including values above 65535 and above 99999999. There is no clamp |
| Playtime | u32 LE frames at `0x2a0`. Preview at `0x1a0` | LODEdit only. NTSC frames divided by 60 with integer division, then `hh:mm:ss`. 59 frames are `00:00:00` and 60 frames are `00:00:01`. Dragoon-Editor does not define playtime. Hours are not clamped at 99 |
| Dragoon spirits | u8 bitfield at `0x39c`. Preview at `0x1a4` | LODEdit. Bits from low to high: Red Eyed, Blue Sea, Jade, Golden, Violet, Silver, Darkness, Divine. The raw byte is shown with the names |
| Key items | seven bytes at `0x39d` | LODEdit named bits only. Temporary bits are not given names. All seven bytes are also shown raw |
| Equipment bag | count u8 at `0x3e4`, 255 IDs at `0x3e8` | LODEdit calls this the armor list. It holds equipment IDs, not only body armor. The count selects how many slots are listed. ID 255 is None and 254 is Invalid |
| Items | count u8 at `0x3e6`, 32 IDs at `0x4e9` | LODEdit consumable list. A count above 32 is noted and only the first 32 slots are listed |
| Character record | nine 44-byte records from `0x52c`, in ID order | Both editors. Experience u32 at `+0`, HP u16 at `+8`, MP u16 at `+10`, SP u16 at `+12`, level u8 at `+18`, Dragoon level u8 at `+19`, weapon/helmet/armor/boots/accessory u8 at `+20` through `+24`. Level 0 stays 0. ID 0 is Broad Sword, not an empty slot |
| Dart max HP | u16 LE at `0x198` | LODEdit save-file preview only (`0x214` in its buffer, plus 4). It is not in Dragoon-Editor's character struct. Current HP stays the character record |
| Saved title | SC `0x4`, Shift-JIS | The save-screen label. It is not a location table and not a completion percentage |

## Names

Shared equipment IDs use Dragoon-Editor's display spelling. That includes `Satori Vest` for ID 71, where LODEdit's enum is `Sator_Vest`, and `Indora's Axe` / `Spear Of Terror`. Consumable names are LODEdit's enum names with underscores written as spaces, including `Angels Prayer`. IDs neither list names stay `Unknown (N)`. The unnamed gaps include 75, 87, 92, 102, 129, 132, 141, 145, 147, 155, 158–192, 196, 213, 219, 226, 239 and 251–253.

Named goods are LODEdit's `Goods2` through the named bits of `Goods5`. The enum's `Temporary37` and later bits stay inside the raw goods bytes.

## Not decoded

These were left out because the two editors do not agree, or neither one defines them:

- The u16 at character `+14`. LODEdit calls it Dragoon experience. Dragoon-Editor calls it max SP. Changing it does not change this summary.
- The bytes at character `+4`, `+16` and the addition counters. LODEdit has per-addition hit counts. They are not part of this summary.
- Stardust. Dragoon-Editor reads one byte at `0x29c`. LODEdit's readme lists stardust, and its pinned code has no stardust offset. It is not shown.
- Warps, treasures, menu options, story events and a story completion percentage. LODEdit lists those as unfinished. No percentage is invented.
- A game checksum. Neither editor documents one. `checksumOk` stays null. A public note that the save has no checksum is not treated as a passing result.

## Filename evidence

Public PS3 PSV names `BASCUS-944916472676E30303030` and `BASCUS-944916472676E30303031` decode as `BA` + `SCUS-94491` + `drgn0000` / `drgn0001`. That is filename-listing evidence for the directory identifier, not a decoded save body. Each of those listings is about 8.13 KB, which matches one 8192-byte block plus a PSV wrapper. The reader accepts `drgn0000` through `drgn0255` and rejects other identifiers, including a second block.

## Sources

Field meanings and offsets were checked against both public editors. No editor source was copied.

- LODEdit, [greenbrettmichael/memcardrex-plugins-LODEdit commit `eecc2dab`](https://github.com/greenbrettmichael/memcardrex-plugins-LODEdit/tree/eecc2dab6bf97a37e8aa9d72b864f4066f63dad2). Files used: `ReadME.md`, `LODEdit/Plugin.cs`, `SaveData.cs`, `Gold.cs`, `TimePlayed.cs`, `Party.cs`, `DragoonStats.cs`, `Inventory.cs`, `Goods.cs`, `CharacterStats.cs`, `mainWindow.cs`. No license file is present. The reader implements the numeric layout independently.
- Dragoon-Editor, [mcred/Dragoon-Editor commit `29a92d74`](https://github.com/mcred/Dragoon-Editor/tree/29a92d740e6de57d67ac55c041e5c28bb4e5b37c), MIT, copyright 2020 Derek Smart (McRed). Files used: `internal/app/storage/Card.go`, `Slot.go`, `internal/app/common/Positions.go`, `Collectibles.go`, `internal/app/characters/Character.go`, `Characters.go`, and the inventory name lists. [Retained license](../licenses/dragoon-editor-MIT.txt).

A separate public note on [セーブエディター.com, 2020-01-04](https://emu.web-g-p.com/info/bbs/patchcode/ps1/bbs.cgi?list=pickup&num=1955) says this save has no checksum, puts 4-byte gold at raw-card `0x2294` (SC `0x294`) with max bytes `FF E0 F5 05`, dragoon spirits at raw `0x239C` (SC `0x39C`), equipment IDs at raw `0x23E8–0x24E6` and consumables at raw `0x24E9–0x2508`. Those extents agree with the editors after removing the directory block. The note is corroboration for width and coverage. It is not a source of extra field names.

## Tests

Synthetic SC payloads place the editor values at the offsets above. They cover the 32-bit gold word, the 60 Hz hour boundary, ID order versus Dragoon-Editor's tab order, unknown IDs, level 0, preview disagreements, a consumable count above 32, and the bytes that must not change the summary. Rejection covers other product codes, regions, identifiers, a short or long buffer, a bad SC header and a malformed title. A formatted card distinguishes `drgn0000` from another identifier and from a two-block chain. No GameFAQs card or ROM is committed.
