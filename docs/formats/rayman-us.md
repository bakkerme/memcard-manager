# Rayman USA read-only summary

Supports directory prefix `BI` or `BA`, product `SLUS-00005`, one 8192-byte reconstructed SC payload, header `SC 13 01`, and ASCII title prefix `Rayman` at offset `0x4`. `BI` is confirmed on two USA saves: a fresh disc load whose directory name was `BISLUS-00005aaa63592`, and the GameFAQs sample `BISLUS-00005smd56832`. `BA` is accepted because it was requested for this product. It was not observed. `SLES-00049` and `SLPS-00026` are not accepted. The 8-byte directory suffix is the save id (`aaa63592`, `smd56832`), not a fixed token.

The card metadata still names directory country `BI` as Japan. That label is the shared country-code name. It does not make this product the Japanese release.

Offsets are relative to the reconstructed SC payload.

| Field | Offset | Confidence |
| --- | --- | --- |
| Header | `SC 13 01`, title `Rayman` | Confirmed on the fresh native load and present on the GameFAQs sample |
| Filename | `0xb`, three stored bytes | Confirmed. The native menu showed AAA for stored `aaa`. Only that letter mapping was seen in-game, so the reader shows the stored bytes (`aaa`, `smd`) |
| Completion | `0xf`, three ASCII digits `000`..=`100` | The digits are the confirmed field. Only `000%` was seen in-game. Other digit strings are shown with leading zeros. Anything else is `Unknown` plus the raw hex |
| Continues | `0x200`, raw u8 | Confirmed at 5 on the fresh load. Bytes above the editor UI limit of 99 are still shown raw |
| Lives | `0x400`, raw u8 | Confirmed at 3 on the fresh load. Same raw treatment above 99 |
| Current level | `0x1080`, u8 | Editor-derived and unconfirmed. Labels 0..=23 come from the editor. Other ids stay `unknown` with the raw value |
| Powers | `0x300` bits 0, 1, 2, 7 and `0x301` bit 0 | Editor-derived and unconfirmed. Punch, Hang, Helicopter, Grab, Run. Other bits in those two bytes stay as an unknown mask. The raw bytes are always shown |
| Cages | 102 bits, 6 per level for the first 17 editor levels | Editor-derived and unconfirmed. The summary is only the total. Per-level rows are not presented as progress |
| Checksum | None | No algorithm is known. `checksumOk` stays null. Nothing is validated |

Cages, powers and current level are labeled `(unconfirmed)` and their values are prefixed `editor:`. They are not reliable progress. The GameFAQs file is labeled 100% complete and still reads as level 0, 0/102 cages, powers `0000`, completion `000`, lives 3 and continues 5. Other ranges in that file are populated, including `0x2d0` = `0x64`, most little-endian words in `0x1088..=0x10ef` equal to `0x0001`, and nonzero bytes in `0x340..0x400`, `0x1000..=0x1330` and `0x1610..=0x16f0`. Either the upload label is wrong or the editor offsets for the unconfirmed fields are wrong. Those populated bytes are not decoded. In particular, `0x2d0` is not treated as the completion percentage. Completion stays the three ASCII digits at `0xf`.

## Editor discrepancies

The map was checked against RyudoSynbios/game-tools-collection, MIT, commit `c4c80f40fd0e102e2dac7257851f43167b2c350f`, file `src/lib/templates/rayman/saveEditor/template.ts`. The TypeScript is not copied. The license notice already in this repo covers that copyright.

- The template validator expects payload offset `0x0` to be the ASCII product code (`SLUS-00005`, `SLES-00049`, or `SLPS-00026`). Both real USA saves start with `SC 13 01` and `Rayman`. The product code is the directory filename (`BISLUS-00005...`). Filename `0xb`, completion `0xf`, continues `0x200` and lives `0x400` were confirmed on the SC payload, so the unconfirmed fields are not off by a missing 10-byte product-code header. They use the same origin as the confirmed fields inside that template, and the 100% labeled sample still contradicts them.
- The template models three slots of `0x2000` bytes. Both observed USA saves are one 8192-byte block. A second linked block is rejected and stays browseable without game details.
- The letter resource maps `0x61`..=`0x7a` to A..=Z and `0x20` to space. Only `0x61` → A was checked in-game. Stored `smd` would display as SMD under that map. This reader does not apply it.
- Health at `0x480` / `0x409` and tins at `0x406` are in the template and are not decoded.
- The template has no checksum.

## Fixture

`src-tauri/src/card/game_details/testdata/rayman-gamefaqs-100.gme` is the trimmed DexDrive image used for header, name and product-code parsing. It is 20288 bytes: a 3904-byte DexDrive header, the directory block, and one save block. The save block begins at file offset `0x2f40`. SHA-256 `b5e0ddabdf2a7ff9dc031a1922a14c9122ca91f39329c43f6a326e7b71132e78`. The directory frame is `51`, size 8192, next `ffff`, name `BISLUS-00005smd56832`, and its XOR checks. The image is not a full 128 KB card, so `Ps1Card::open` does not accept it directly. The test reads that layout, then imports the directory frame plus save block onto a formatted card.

Synthetic tests rebuild the decoder kit's `make_suite.py` slots in memory and pin SHA-256 for `zero-candidate`, `upper-editor-range`, `unknown-raw-values`, `all-102-cages`, `cage-0-0`, `power-0x300-0`, `power-0x300-7` and `power-0x301-0`. Those hashes are byte regressions. They are not native progress values. Passing them does not confirm cage, power or level meanings.
