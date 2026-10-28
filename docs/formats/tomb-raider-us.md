# Tomb Raider US read-only summary

Supports `BA` / `SLUS-00152` / `TOMBRAID` (directory `BASLUS-00152TOMBRAID`), one 8192-byte reconstructed SC payload, header `SC 11 01`, and the Shift-JIS title normalized to `Tomb Raider`. SC has no directory identity. Region and product code come from the directory entry; a relabeled PAL payload is not distinguished by the header alone.

The saved current level is a little-endian signed halfword at SC `0x33e`. It indexes the same 22-name table as PAL Tomb Raider (`SLES-00024`): 0 is Lara's Home and 15 is The Great Pyramid, followed by cutscene, title and current-position labels. Negative or out-of-range values display as `Unknown level (N)` with the raw signed value. The saved word does not prove that a current-position checkpoint is available.

Payload integrity is the byte sum of SC `[0x200,0xf58)` modulo 256, which must be zero. The stored byte at `0x357` is the negative of the sum of the other covered bytes. Directory XOR is a separate check. A mismatch leaves the level visible and sets `checksumOk` to false. This measures payload arithmetic only. There is no emulator load validation.

Not emitted for this release: current-position availability (candidate bit `0x01000000` at `0x330`), level-start inventory records (`0x200 + i*14`), health, playtime and checkpoint inventory. Those fields remain on the PAL reader.

The public GameFAQs sample from contributor Kasket_Darkfyre ([save 381](https://gamefaqs.gamespot.com/ps/199040-tomb-raider/saves/381)) stores level 15, The Great Pyramid. The bytes are not redistributed. Place the GME at `tests/fixtures/ps1/local/tomb-raider-1996.381.gme` for the shared corpus. Hashes:

| Wrapper | SHA-256 |
| --- | --- |
| `tomb-raider-1996.381.gme` | `273fedac22c71c0bbc1b621f49a1d822b02698d24cc9fbebed7515ea66a7c73f` |
| whole card `.mcr` | `7439dd491a2244c1a6012189c66c1d03c7edc117159fb58a8291b91a5fa8f3c4` |
| `.mcs` | `ec49622a97e1175e201783cb6372e97e1a502902a30fb263842e4d95b70c3df5` |
| SC payload | `5699244b0e6ca09b64c71fd9790c37d00a352d3af60fbc936038976791cc06eb` |

Synthetic tests cover the level table, checksum coverage inside and outside `[0x200,0xf58)`, and rejection of region `BE`, a wrong identifier, header bytes 2 and 3, a title that does not normalize to Tomb Raider, a short payload and length 8193. The ignored real-sample test reads the four paths from the environment:

```sh
TOMB_RAIDER_US_MCS=/path/BASLUS-00152TOMBRAID.mcs \
TOMB_RAIDER_US_SC=/path/BASLUS-00152TOMBRAID.sc \
TOMB_RAIDER_US_MCR=/path/tomb-raider-us.mcr \
TOMB_RAIDER_US_GME=/path/tomb-raider-1996.381.gme \
  cargo test --manifest-path src-tauri/Cargo.toml tomb_raider_us_real_samples -- --ignored
```

The shared corpus, when the local GME is present, checks the same level and checksum through `Ps1Card::open`:

```sh
cargo test --manifest-path src-tauri/Cargo.toml shared_completion_corpus -- --ignored
```
