# @@TITLE@@ save-details decoder

## Goal

@@GOAL@@

Target fields: @@FIELDS@@.

The decoder should help a person tell saves apart and track progress. A 100% sample only shows the "everything set" corner. Also load that save in the emulator and read the completion percent shown on the save or select screen. Check ranges with synthetic edits, and label those edits as synthetic.

## Acceptance

- A read-only decoder and suite for @@PRODUCT_CODE@@ / @@SERIAL@@, following the shape of `inputs/current-reader`. There is no Rayman decoder in that tree.
- Known values from the supplied or emulator-generated saves, plus boundary and malformed cases. Synthetic edits stay labelled synthetic.
- Cross-check against the MIT editor cited below. Cite it. Do not copy its source into the deliverable.
- The cited editor has no checksum logic. Leave checksum status null unless a separate experiment establishes an algorithm. Do not invent a passing checksum.
- `native-load` reads the on-screen completion percent. Retain a screenshot of that screen and cite its published evidence archive plus the member name.
- Two different progress snapshots must not collapse to the same summary.
- Required checks below are mandatory. If one cannot be done, the outcome is partial with the blocker, never complete.

## Cross-check

- Name: @@CROSS_NAME@@
- License: @@CROSS_LICENSE@@
- Commit: @@CROSS_COMMIT@@
- URL: @@CROSS_URL@@
- @@CROSS_NOTE@@

Offsets below are that editor's claims for one 0x2000-byte slot, not a copied implementation and not yet confirmed in this package. The product-code ASCII the editor expects sits at the start of the slot (`SLUS-00005` for this USA release). Confirm the slot base after unwrapping the DexDrive container; do not treat wrapper bytes as the slot.

@@FIELD_MAP@@

## Supplied inputs

- Save: `@@SAVE_PATH@@` (sha256 `@@SAVE_SHA@@`). @@SAVE_PROVENANCE@@
- BIOS: `@@BIOS_PATH@@` (sha256 `@@BIOS_SHA@@`). @@BIOS_PROVENANCE@@
  The BIOS is a packaged private input, the same way the Tomb Raider US package carried `inputs/bios/ps-22a.bin`. It is not an asset-loader download.
- Disc: runner dependency `psx-rom` `@@DISC_ID@@`, materialized at `@@DISC_DEST@@` before the first turn. @@DISC_PROVENANCE@@@@EXTRA_DISCS@@
  Do not package or publish disc or BIOS bytes.
- Current reader: `inputs/current-reader/`. Tooling: `tools/ps1_replay/` (including the Containerfile and `host.py`) and `tools/save_research/`. Skills: `skills/ps1-save-research/` and `skills/emulator-replay/`.
- Editor field table and this question's source spec: `inputs/spec.json`.

## Required checks

@@CHECKS@@

## Published outputs

Follow the [output-packaging contract](../output-packaging.md). Deliver the field ledger, read-only decoder and its tests, a small fixture set, and the evidence needed for the acceptance criteria above.

## Out of scope

Hardware writes, other consoles, and redistributing BIOS, disc, or save bytes. Production parser changes belong in a later memcard-manager change after this job's evidence, not in this package's inputs.
