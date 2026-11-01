# Spyro: Year of the Dragon USA read-only summary

Supports directory prefix `BA`, product `SCUS-94467`, identifier `SPY3_1`, one 8192-byte reconstructed SC payload, header `SC 13 01`, and Shift-JIS title prefix `ＳＰＹＲＯ３` at offset `0x4`. The directory filename in the NTSC-U rev 0 decomp is `BASCUS-94467SPY3_1`. Other product codes, regions and identifiers are not accepted.

Confidence is **code-traced**. The layout comes from the matching decompilation [TheMobyCollective/spyro-3](https://github.com/TheMobyCollective/spyro-3) commit [`e97cd3387ac1bb7b7c9c958f7b7bdc591a2ae273`](https://github.com/TheMobyCollective/spyro-3/tree/e97cd3387ac1bb7b7c9c958f7b7bdc591a2ae273) (CC0). No public save sample was hashed, and nothing here was checked by saving and reloading on a console or emulator. Later revisions of the same serial were not compared. Synthetic tests check the reader against these traced numbers. They are not native progress values.

Offsets are relative to the reconstructed SC payload. The title overlay copies `0x200` bytes of header and icon from its template, then stores three adventures:

| Adventure | SC offset | Size |
| --- | --- | --- |
| 1 | `0x280` | `0x900` |
| 2 | `0xB80` | `0x900` |
| 3 | `0x1480` | `0x900` |

The create path writes `0x1D80` bytes. The directory file is still one 8192-byte block. Bytes after `0x1D80` are not interpreted.

`SaveFile` is `include/memcard.h`. Checksum is the sum of the first `0x8FC` bytes, stored as a little-endian word at `0x8FC`. A mismatch is reported and the stored fields are still shown.

| Field | Where | Confidence |
| --- | --- | --- |
| Empty | `visitedLevels[0]` at slot `0x8` is 0 | Code-traced. `func_title_8007928C` returns -1 for that case and the file select skips it. Other bytes in the slot are ignored for this test |
| Saved location | slot `0x5`, level index | Code-traced. Names 0..=36 are the pointer table `D_80069CF8`. Other indexes stay `unknown` with the raw value. Indexes 37..=39 in that table are the string `Empty` and are not used as names |
| Lives | slot `0x6`, raw u8 | Code-traced stored byte. On load, a value below 4 becomes 4 and a value of 100 or more becomes 0. The reader shows the stored byte |
| Gems | 40 little-endian u16 totals at slot `0x530` | Code-traced. The loader caps indexes 0..=38 to `D_80066F54[i] * 100` and adds index 39 raw. That sum is the HUD counter. The 40 table entries sum to 20000. The decomp comment calls index 39 gems retrieved from Moneybags; that label was not checked on a save, so it is not a separate field |
| Eggs | 40 bitmask bytes at slot `0x580` | Code-traced. The loader counts every set bit. The egg table `D_80066F7C` sums to 149, which is the shown denominator. Extra set bits can exceed 149. A marketed total of 150 is not in this table |
| Completion | title-screen routine `func_title_8007928C` | Code-traced. Numerator is stored gems plus 100 per set egg bit for levels 0..=38. Denominator is the same tables with Super Bonus Round (index 36) omitted: 15000 gems + 148 eggs. Integer division. A nonzero numerator that would truncate to 0% is shown as 1%. A full table is 117%. This is not a gem percentage and it does not include skill points |
| Per-level rows | levels 0..=36 | Code-traced stored gem word and set egg bits against that level's table entry. The gem word is not capped in the row |
| Checksum | slot `0x8FC` | Code-traced additive byte sum. Not checked on a captured save |
| Dragons | none | This game does not store a Spyro 1 dragon tally. Dragon eggs are the egg field |

Not decoded: the 20 skill-point bytes (one traced store writes 3 and a later test masks bit 0, so a nonzero count would be a guess), `ProgressFlags`, Moneybags tolls, speedway and skate records, dialogue bits, and the per-level collection bitmasks. Individual gems are not recounted from those bitmasks. The u16 totals are what the loader adds.
