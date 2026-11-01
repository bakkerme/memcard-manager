# @@TITLE@@ save-details decoder

## Goal

@@GOAL@@

Target fields, in order: @@FIELDS@@.

This job is cheat-and-diff. There is no public save editor to copy a field map from. Discover the bytes by poking guest RAM, letting the game write a save, and comparing that save with a control. You choose the order, the codes, and how many sessions to run. One lives poke is not the whole job. A RAM address is a search hint: it is not a disk offset, a unit, or proof that the value persists.

The decoder should help a person tell saves apart and track progress. A single fully completed save only shows the "everything set" corner. Also keep an early save and at least one mid-progress save.

## Acceptance

- A read-only decoder and suite for @@PRODUCT_CODE@@ / @@SERIAL@@, following the shape of `inputs/current-reader`.
- Known values from emulator-generated saves, plus boundary and malformed cases. Synthetic edits stay labelled synthetic.
- Two different progress snapshots must not collapse to the same summary.
- Leave checksum status null unless a save the game accepts and a save the game rejects establish an algorithm. A guessed formula that matches one sample is not a passing checksum.
- `native-load` reloads a game-written save from a fresh boot and reads the progress screen. Emulator state restore does not count. If that screen shows a completion percent, record the number. If it shows counters or a title and no percent, record that and keep the capture.
- Required checks below are mandatory. If one cannot be done, the outcome is partial with the blocker, never complete.
- Confidence on every delivered field is one of: unknown, partial, candidate, corroborated, code-traced, save-reload-verified. The labels in the field table below are the starting point, not a result.

## Known non-sources

These describe something else. Cite them as negative checks. Do not port their layouts into the decoder.

@@NON_SOURCES@@

## Starting field table

No offsets are supplied. Confidence is a label, not a measurement.

@@FIELD_MAP@@

## Layout notes

@@LAYOUT@@

## Supplied inputs

- Save: `@@SAVE_PATH@@` (sha256 `@@SAVE_SHA@@`). @@SAVE_PROVENANCE@@
- BIOS: `@@BIOS_PATH@@` (sha256 `@@BIOS_SHA@@`). @@BIOS_PROVENANCE@@
  The BIOS is a packaged private input. It is not an asset-loader download.
- Disc: runner dependency `psx-rom` `@@DISC_ID@@`, materialized at `@@DISC_DEST@@` before the first turn. @@DISC_PROVENANCE@@@@EXTRA_DISCS@@
  Do not package or publish disc or BIOS bytes. Parse a `title:` asset id on the first colon only; the title itself may contain a colon.
- Current reader: `inputs/current-reader/`. Tooling: `tools/ps1_replay/` (including the Containerfile and `host.py`) and `tools/save_research/`. Skills: `skills/ps1-save-research/` and `skills/emulator-replay/`.
- This question's source spec: `inputs/spec.json`.

## Required checks

@@CHECKS@@

@@CHECK_NOTES@@

## Published outputs

Follow the [output-packaging contract](../output-packaging.md). Deliver the field ledger, read-only decoder and its tests, a small fixture set, and the evidence needed for the acceptance criteria above.

## Out of scope

Hardware writes, other consoles, other regions of this game, and redistributing BIOS, disc, or save bytes. Production parser changes belong in a later memcard-manager change after this job's evidence, not in this package's inputs.
