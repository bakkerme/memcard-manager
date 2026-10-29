# Read-only PS1 management summary

This is an unconfigured template. Do not submit until the following are filled in
and the required private inputs and tools are copied into this definition.

- Exact game product code, region/revision, save identifier and supported layout.
- Research question and current reader/specification paths.
- Small summary target: profile identity and useful progress fields.
- Acceptance: independently evidenced known values, boundary/malformed checks,
  partial spec, field ledger, reproducible parser/suite and exact limitations.
- Inputs: supplied saves, matching readers/helpers, evidence and provenance. For
  runtime work, include BIOS and the entire exact disc/CUE dependency set.
- Dependency ledger: relative path, size, SHA-256 and provenance for every required
  file; immutable tool image digest and location when runtime work needs it.
- Budget and finalization reserve in manifest.json.

A missing or oversized dependency blocks its affected scope; distinguish static
analysis from runtime qualification. Report each oversize file's path, bytes, hash
and exceeded server limit. Delivery integrity is separate from scientific validity.

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

`required_outputs` in manifest.json stays `outputs/handoff.md`, `outputs/research.json`, and `outputs/result.json`. Publish the field ledger, decoder, fixtures, and archives because the question asks for them.
