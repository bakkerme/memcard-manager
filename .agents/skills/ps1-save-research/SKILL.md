---
name: ps1-save-research
description: "Reverse engineer PS1 game-save layouts, persistent fields, checksums, enums, inventory, and flags from supplied saves, controlled comparisons, and serialization traces. Use to investigate unknown save data or extend a read-only decoder. Supports research from a cloned repository with optional container emulator experiments; ordinary card browsing and emulator setup alone use existing tools."
---

# PS1 Save Reverse Engineering

Produce a partial, reproducible specification whose claims can be traced to exact saves, observed game values, and code paths. Keep candidate meanings distinct from verified persistence and from English names or units.

## Start with the question and input boundary

Identify the requested field or format, game product code, region/revision, save identifier, container, and available observations. Inspect existing readers/specifications before reverse engineering a supported field again. Hash originals, work on copies, and retain provenance. A clone supplies tools and skills, not private assets or prior experiment evidence.

For standalone parser output without the desktop application, read [the parser harness boundary](references/standalone-parsers.md). Use the supplied Python plug-in harness when available and keep file-contract tests distinct from independent field evidence.

In memcard-manager, read [the repository guide](references/repository-guide.md) for extraction, existing readers, and the container boundary. Begin with file analysis when no working emulator is available; report dynamic verification as untested rather than blocking all research.

Distinguish whole-card containers, individual-save wrappers, reconstructed SC payloads, game-data substructures, virtual RAM, and emulator states. Establish one declared offset origin for each artifact. Reconstruct linked blocks through validated directory entries in chain order, including non-contiguous chains; never assume the save is a contiguous card slice. Account for wrapper/header bytes explicitly. A filename or byte pattern alone does not establish region or format.

## Develop and test hypotheses

1. Inventory samples and known visible values. Check sizes, identifiers, headers, chains, and any already understood integrity fields. Record malformed inputs without relabeling them as another release.
2. Compare matching normalized payloads. Group adjacent changed bytes and inspect little/big endian integers, signedness, bitfields, text encoding, record stride, and repeated banks as hypotheses. Separate timestamps, counters, checksums, thumbnails, and incidental padding from a candidate gameplay change. Different bytes do not by themselves identify a field.
3. Design a small discriminating experiment. Prefer baseline/no-op saves plus one-variable changes, a reversal or second value, and an independent sample. Record what else changed. Read [the experiment protocol](references/controlled-experiments.md) when generating saves, tracing serialization, or assigning confidence.
4. If runtime evidence is needed, use the sibling `$emulator-replay` skill when available to establish a working isolated worker, visible scene, reliable input, RAM capture, and baseline restoration. The tested Legends 2 harness is game-specific and currently ARM64. Adapt its validation, recipe, addresses, and review labels before using another title. For cheat-and-diff field discovery, opt in to logged guest RAM pokes and then let the game write the save. Read `tools/ps1_replay/README.md` first: confirm a save can be triggered from the current scene, and keep a control save with no pokes.
5. Trace RAM candidates through the game's save packer and loader where possible. Record overlay/scene, virtual addresses, widths, destination offsets, conversions, masks, strides, and merge behavior. A region-matched cheat address can orient a search; it is not a disk offset or proof of ownership, persistence, or units.
6. Verify independent predictions. Check changed values against serialized bytes and, when performed, a fresh load of the resulting disposable card. Native emulator state restoration verifies a RAM snapshot; it does not prove the game's save/load path. The existing input/replay pilot does not qualify in-game saving: validate save persistence separately before relying on generated saves.

The included [payload comparison helper](scripts/compare_payloads.py) is Python-standard-library, read-only tooling. Run it on equal-sized, already normalized payloads; it emits hashes, exact changed spans, and XOR masks. It does not extract cards, recognize games, infer types, or identify checksums. Keep its raw-byte JSON output in ignored evidence:

```sh
python3 .agents/skills/ps1-save-research/scripts/compare_payloads.py before.sc after.sc > tools/save_research/runs/experiment/diff.json
```

Create the evidence directory first. Treat bytes in this report as private save evidence.

## Integrity and interpretation

Derive integrity algorithms independently of field meaning: record covered ranges, byte/word order, arithmetic width, overflow, seed/final transforms, excluded/stored checksum locations, and preview versus full-load behavior. Distinguish card-directory/frame XOR checks from the game's payload checksums; success at one layer does not validate the other. Verify across multiple independent saves and corruption probes on disposable copies when useful. Do not repair originals or choose an algorithm merely because one sample matches. Unknown checksum support stays unknown, not passing.

An ID's location, name, and behavior are separate claims. Preserve unknown IDs and raw values. Do not equate equipped items with owned inventory, unused slot bytes with unlock flags, zero-filled data with absence, or record padding with attributes. Constant or equal counters can still be independently serialized fields. Stale code/pointers in padding do not establish compression or encryption; never follow stored pointers in an offline file reader.

For units, compare boundary values and actual game display: stored ticks need not equal rendered FPS, raw health need not equal hearts, and a load-menu location label need not equal the runtime scene name. For flags, establish byte/bit order and distinguish transient, one-way, repeated, and derived state before naming individual bits.

## Deliver evidence and a reader boundary

Maintain a field ledger with offset origin, range/width, endianness or encoding, proposed meaning, raw/visible values, sample hashes, experiment/trace anchors, confidence, counterexamples, and remaining tests. Use confidence terms such as candidate, corroborated, code-traced, and save/reload-verified; state what each claim actually rests on.

Write a concise partial specification and an explicit unsupported/unknown list. Preserve reproducible commands, hypotheses rejected, integrity results, and the next useful experiment. Cite pinned external code/documentation with license and attribution when used; do not bundle private saves, ROMs, BIOS, disassembly dumps, or third-party implementation code without appropriate rights.

When implementation is requested, add a bounded read-only research decoder first, then promote verified fields into the production reader with exact supported identifiers/layouts and meaningful known-value/malformed-input tests. Keep parser attribution. Synthetic tests validate parser behavior, not a hypothesized field's real-world meaning. A requested research finding does not automatically authorize a production editor or hardware write feature.

## Publish runner outputs under the entry cap

Research Runner publishes about 512 output entries, with a separate 32 MiB per-file limit and 128 MiB per generation. An MGS job that left screenshots, calibration dumps, and action logs as individual files wrote 655 entries and could not publish.

Local run directories (`tools/save_research/runs/`, `tools/ps1_replay/runs/`) stay unpacked. When the deliverable is a Runner `outputs/` tree, pack that bulky per-step evidence into a few reproducible archives and leave the important files loose:

- `outputs/handoff.md`, `outputs/field-ledger.md`, `outputs/research.json`, `outputs/result.json`
- `outputs/checkpoint.json` and `outputs/tool-image.json`, which the runner and the tool-image check read during the job
- `outputs/decoder/`: the read-only decoder and its tests
- `outputs/fixtures/`: the small set of saves and sidecars the suite loads

Other single-file reports may stay loose. Pack per-step screenshots, raw frames, calibration dumps, and action logs into:

- `outputs/evidence-screenshots.tar.gz`
- `outputs/evidence-calibration.tar.gz`
- `outputs/evidence-actions.tar.gz`

One archive is enough when only one kind is bulky. Split a kind into a second archive when one file would exceed 32 MiB. Stage the files outside `outputs/`, write `INDEX.txt` in that staging directory listing every archived path, write the archive, then remove the packed directories from `outputs/` so those entries are not published twice. Build each archive with sorted names, a zero timestamp, and no gzip filename metadata:

```sh
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C staging/screenshots . \
  | gzip -n -9 > outputs/evidence-screenshots.tar.gz
```

In `outputs/research.json`, set each evidence `path` to the published archive and `sha256` to that archive's digest. Name decisive members in the description. Cite an archive once per kind; the research report caps evidence items, so one item per capture will not fit.

The package `required_outputs` list stays `outputs/handoff.md`, `outputs/research.json`, and `outputs/result.json`. Publish the field ledger, decoder, fixtures, and archives because the question asks for them.
