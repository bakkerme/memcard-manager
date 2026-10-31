# Research Runner output-packaging contract

Source of truth: `research/output-packaging.md` in memcard-manager. Standalone
jobs receive an identical copy at `inputs/output-packaging.md`. Edit the source
and run `python3 tools/save_research/packaging_contract.py` to refresh the manual
starter; do not maintain generated copies separately.

Research Runner currently allows 4,096 output entries (files and directories), 32 MiB per published file, and 128 MiB per generation. Check the server capabilities for the active limits; uploaded input packages have a separate 512-entry admission limit. A past MGS job with 655 loose evidence entries failed under the former 512-entry output cap; that is historical context, not the current output limit.

Local run directories (`tools/save_research/runs/`, `tools/ps1_replay/runs/`) stay unpacked. For a Runner `outputs/` tree, pack bulky per-step evidence into a few reproducible archives and leave the important files loose:

- `outputs/handoff.md`, `outputs/field-ledger.md`, `outputs/research.json`, `outputs/result.json`
- `outputs/checkpoint.json` and `outputs/tool-image.json`, which the runner and the tool-image check read during the job
- `outputs/decoder/`: the read-only decoder and its tests
- `outputs/fixtures/`: the small set of saves and sidecars the suite loads

Other single-file reports may stay loose. Pack per-step screenshots, raw frames, calibration dumps, and action logs into:

- `outputs/evidence-screenshots.tar.gz`
- `outputs/evidence-calibration.tar.gz`
- `outputs/evidence-actions.tar.gz`

One archive is enough when only one kind is bulky. Measure each final compressed `.tar.gz` file and shard the evidence into additional archives until every published archive is at most 32 MiB. The limit applies to the archive itself, not each member inside it. Keep all published files together (archives, loose reports, fixtures and Runner receipts) within 128 MiB per generation; sharding does not reduce that total. If the evidence cannot fit, report the publication limitation rather than silently discarding it. Stage the files outside `outputs/`, write `INDEX.txt` in that staging directory listing every archived path, write and verify the size and contents of each archive, then remove the packed directories from `outputs/` so those entries are not published twice. Build each archive with sorted names, a zero timestamp, and no gzip filename metadata:

```sh
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C staging/screenshots . \
  | gzip -n -9 > outputs/evidence-screenshots.tar.gz
```

In `outputs/research.json`, set each evidence `path` to the published archive and `sha256` to that archive's digest. Name decisive members in the description. Cite each archive once; the research report caps evidence items, so one item per capture will not fit.

New definitions default to the `required_outputs` trio: `outputs/handoff.md`, `outputs/research.json`, and `outputs/result.json`. The question defines the field ledger, decoder, fixtures, and evidence required for its scope.

Existing and imported manifests remain authoritative: preserve every entry in their `required_outputs`, including extra legacy paths. These additional requirements also apply to partial or blocked closeout; the default trio does not waive them. If a required output cannot be produced, report its blocker explicitly rather than silently dropping the manifest requirement.
