# Research from this repository

In a clone of memcard-manager, work from the root and use the checked-in `.agents/skills/ps1-save-research/` skill. Private inputs are supplied separately; no author home directory or prior state is needed. Begin with Python file analysis in the research environment. Emulator experiments use the isolated worker described by the sibling emulator-replay skill, with explicit asset paths and engine/architecture checks.

For parser-only output, use the [standalone harness boundary](standalone-parsers.md); `tools/save_research/` is independent of the desktop application.

In a standalone server bundle, start with `jobs.json`, the root-level suites and `tools/save_research/README.md`; run `validate_bundle.py` before research. Production Rust/frontend sources below are repository references and are intentionally omitted from that minimal bundle. The copied specs and Python readers remain available.

## Existing sources of truth

- `src-tauri/src/card/engine.rs` and `mod.rs`: card/container identification, directory/block-chain reconstruction, and save extraction. Reuse these boundaries; do not assume every save occupies one contiguous block.
- `src-tauri/src/card/game_details.rs`, `game_details/summaries.rs`, and game-specific modules: supported reader dispatch and field types; unknown integrity support is optional, not automatically true.
- `docs/save-details.md` and `docs/formats/mega-man-legends-2.md`: per-release evidence, implemented fields, checksums, and limits.
- `tools/inspect_mml2.py` and `tools/test_inspect_mml2.py`: executable read-only Legends 2 example accepting a reconstructed 8192-byte SC payload or 8320-byte MCS. It does not parse a whole card or prove compatibility with another game.
- `tests/fixtures/ps1/README.md` and `manifest.json`: corpus provenance, hashes, rights boundaries, and opt-in local-corpus commands.
- `tools/ps1_replay/README.md`: container setup and input/replay procedure. The existing recipe/addresses/decoder checks are specific to US Legends 2 on native Linux ARM64.

Read the relevant existing code/specification when a supported title overlaps the request. The user's requested scope controls whether to document a finding, extend research tooling, or change a production decoder.

## Offset and evidence example

The US Legends 2 spec defines offsets relative to the reconstructed SC payload, including its SC header. An MCS directory wrapper adds 128 bytes; a whole card requires directory lookup and chain reconstruction first. Its armor candidate was traced from RAM `0x8008C254` to SC offset `0x2b6`. That maps serialization; it does not establish every armor name or inventory ownership. Do not transfer either offset to another release.

The four known Legends 2 checksum ranges and little-endian additive algorithm belong to that documented release. Use the existing reader to check known saves instead of rediscovering it or applying it to unrelated games. Preview loading and full loading can have different coverage. Copied structures include scattered fields and record padding, so the save is not a simple contiguous RAM dump.

The helper `scripts/compare_payloads.py` compares bytes after extraction; it never strips 128 bytes blindly. Declare your extraction method and offset origin in the experiment record. Use `tools/save_research/runs/<experiment>/` for new ignored evidence, or the existing emulator run directory for runtime artifacts.

## Container and execution boundary

Python-only analysis and the comparison helper need no Node.js, Rust, BIOS, game disc, desktop focus, or emulator. Card extraction through existing Rust tooling may need its normal Rust dependencies; do not install the desktop stack merely to run normalized-payload comparisons.

Dynamic research needs user-supplied region-matched BIOS/disc/card inputs and a suitable Docker CE or Podman engine. A Codex agent in an outer container must have engine access and mount paths visible to that engine. A fresh clone alone does not provide that infrastructure. If unavailable, continue useful static work and distinguish untested runtime claims.

For a requested readiness pilot, follow the sibling emulator-replay [startup preservation protocol](../../emulator-replay/references/startup-preservation.md). Retain failed probes and diagnose reproducible boot-only scratch writes within the pilot before freezing an initialized disposable baseline. Require exact whole-card equality afterward and keep unrelated file-analysis checks available. Readiness is separate from save/load persistence.

When generating new saves, use the experiment protocol and a separate save-persistence qualification. The existing three-trial Equipment replay intentionally performs no in-game save and requires unchanged disposable cards. Its historical pass is not proof of saving, another title, or another architecture.

## Publishable versus private artifacts

Keep reports, field ledgers without raw private dumps, decoder code, recipes, and synthetic test fixtures in version control as appropriate to the task. Keep user saves, ROM/BIOS bytes, complete RAM/disassembly dumps, emulator states, screenshot evidence, and generated card copies in ignored/private storage. Preserve unrelated file moves and user edits in the checkout.

A Research Runner publication is a separate tree under `outputs/`. Follow the skill's publish section: leave `handoff.md`, `field-ledger.md`, `research.json`, `result.json`, `checkpoint.json`, `tool-image.json`, `decoder/`, and a small fixtures set loose, and pack screenshots, calibration dumps, and action logs into a few archives following the active server quotas: currently 4,096 output entries (files and directories), 32 MiB per published file, and 128 MiB per generation. Measure each compressed archive and shard it to meet the per-file limit; all published files together, including Runner receipts, must meet the generation total.

Use the existing game reader tests when a decoder changes. `python3 -m unittest discover -s tools -p 'test_inspect_mml2.py' -v` checks the standalone Legends 2 reader; its private-corpus test is optional and may be skipped. Research-tool validation does not replace production Rust tests for a shipped parser change.
