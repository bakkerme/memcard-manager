# PS1 game-details fixtures

`manifest.json` records the actual files used, SHA-256 hashes, source links, completion claims and selected regression values. Downloaded cards and user saves belong in `local/`, which is ignored by Git. Do not commit them: the public uploads have no identified redistribution license, and the FFVII saves are user-owned.

For each public manifest entry, open its GameFAQs source URL, download the DexDrive (`.gme`) file, and place it in this folder's `local/` directory with the manifest filename. GameFAQs downloads may require a normal browser; a direct command-line request can return HTTP 403. No account was needed for acquisition on 2026-10-05. The user-supplied `BESCES-00867FF7-S01.mcs` and `...S02.mcs` are optional for other developers and are never fetched from the internet. They were both tested during implementation.

Run the public/local corpus explicitly:

```sh
cargo test --manifest-path src-tauri/Cargo.toml shared_completion_corpus -- --ignored
```

This selects only the corpus test and does not run the ignored USB hardware tests. Missing public files or mismatched hashes fail the check; absent private FFVII files are skipped. `npm test` always runs the constructed-data decoder regressions and the existing card/collection tests without network access or this corpus.

Completion descriptions independently support selected values: Spyro's 80 dragons/14,000 gems/12 eggs, GT's two billion credits and gold tests, GT2's 97 cars/286 wins/60 gold tests, CTR's four 101% profiles, CC's 48 stars, SOTN's 200.6% map discovery, FFT's registered level-50 units, and Tekken unlocks. Other manifest values are fixed byte-level regressions. FFVIII's six main characters are level 100; GF levels are not inferred. FFIX's file is English PAL despite its listing; its actual code and frame rate control decoding.

Capture/acquisition time is separate from uploader dates and from the original game's save time. Binary hashes and current source listings do not prove console/emulator playability.
