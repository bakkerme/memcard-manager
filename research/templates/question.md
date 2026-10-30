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
- `native-load` reads the on-screen completion percent. A screenshot of that screen is evidence. Pack it in `outputs/evidence-screenshots.tar.gz` and cite that archive plus the member name.
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
- Disc: runner dependency `psx-rom` `@@DISC_ID@@`, materialized at `@@DISC_DEST@@` before the first turn. @@DISC_PROVENANCE@@
  Do not package or publish disc or BIOS bytes.
- Current reader: `inputs/current-reader/`. Tooling: `tools/ps1_replay/` (including the Containerfile and `host.py`) and `tools/save_research/`. Skills: `skills/ps1-save-research/` and `skills/emulator-replay/`.
- Editor field table and this question's source spec: `inputs/spec.json`.

## Required checks

@@CHECKS@@

## Published outputs

Research Runner publishes about 512 output entries, 32 MiB per file, and 128 MiB per generation. An MGS job that left screenshots, calibration dumps, and action logs as individual files wrote 655 entries and could not publish.

Leave these loose:

- `outputs/handoff.md`, `outputs/field-ledger.md`, `outputs/research.json`, `outputs/result.json`
- `outputs/checkpoint.json` and `outputs/tool-image.json` (the runner and the tool-image check read these paths during the job)
- `outputs/decoder/`: the read-only decoder and its tests
- `outputs/fixtures/`: the small set of saves and sidecars the suite loads

Other single-file reports may stay loose. Pack per-step screenshots, raw frames, calibration dumps, and action logs into `outputs/evidence-screenshots.tar.gz`, `outputs/evidence-calibration.tar.gz`, and `outputs/evidence-actions.tar.gz`. One archive is enough when only one kind is bulky. Split a kind into a second archive when one file would exceed 32 MiB. Stage the files outside `outputs/`, write `INDEX.txt` in that staging directory listing every archived path, write the archive, then remove the packed directories from `outputs/` so those entries are not published twice. Build each archive with sorted names, a zero timestamp, and no gzip filename metadata:

```sh
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C staging/screenshots . \
  | gzip -n -9 > outputs/evidence-screenshots.tar.gz
```

In `outputs/research.json`, set each evidence `path` to the published archive and `sha256` to that archive's digest. Name decisive members in the description. Cite an archive once per kind.

`required_outputs` stays `outputs/handoff.md`, `outputs/research.json`, and `outputs/result.json`. Publish the field ledger, decoder, fixtures, and archives because this question asks for them.

## Out of scope

Hardware writes, other consoles, and redistributing BIOS, disc, or save bytes. Production parser changes belong in a later memcard-manager change after this job's evidence, not in this package's inputs.
