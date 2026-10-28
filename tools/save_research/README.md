# Standalone parser research

## Research Runner

Use the repo-local [Research Runner workflow](../../research/README.md): install
`requirements-research.txt`, activate `.research-runner/venv`, define work under
`research/NAME/`, and use the installed CLI for package/submit/start/monitor/pull/results.
No Runner source checkout or custom submission/collection commands are needed.

`prepare_runner.py` imports a complete, SHA-256-pinned legacy v2 archive into
a fresh definition, retaining every dependency and its relative path, and adds the
structured research closeout contract. It delegates extraction and packaging to
the installed library and never submits or starts work. See the workflow for the
Pac-Man/Bomberman migration, input preservation, current server compatibility,
required dependency verification and recovery commands.

`assemble_package.py` is the other direction: it builds a new definition from a
committed game spec (`research/specs/<game>.json`) and the tracked tools, skills,
and current reader. It does not submit or start work. BIOS and save files are
passed with `--bios`/`--save` or the spec's `path_env` variables and must match
the spec sha256. Disc bytes are not packaged; the manifest lists them as a
`psx-rom` dependency. `--format v2` (the default) is the package Runner 0.4.0
can run, and its first instruction is `host.py ensure`. Two assemblies with the
same spec, commit, and private-input bytes produce the same archive sha256.

```sh
python3 tools/save_research/assemble_package.py \
  --spec research/specs/rayman-us.json --format v2 --out /tmp/rayman-pkg
```

The Rayman spec's save digest is a 64-zero placeholder until the GameFAQs file
is hashed. The BIOS digest is the Tomb Raider `ps-22a.bin` sha256. Neither file
belongs in git.

Copy this directory's `harness.py` and `parser_template.py` with your private
suite. Python 3.10+ and the standard library are sufficient. No desktop app,
Node.js, Rust, BIOS, disc or container is needed for file-only parser validation.
Docker/Podman and matching discs/BIOS are optional inputs for runtime research.
For an unattended Docker worker, use a dedicated research account and rootless
Docker; follow the [coordinator setup and resource-limit checks](../ps1_replay/README.md).
File-only parser checks do not require engine permissions.

## Build a server bundle from the current checkout

Run locally, from the repository root:

```sh
python3 tools/save_research/build_bundle.py --output tools/save_research/runs/server-bundle
```

The builder finds the latest local `runs/*/parser-jobs.json` backlog (or use
`--backlog FILE`), checks current decoder dispatch and skips newly covered releases.
It copies exact deduplicated samples, preserves card-only originals, exports
their linked save chains through the existing Rust parser, creates per-release
parser templates/suites, and copies the research tools, skills and licence.
Local Rust/Cargo is needed only for card-only extraction; the server needs no
Rust or desktop stack. Preparation never changes source saves. Output must be a
new folder; failures leave an inspectable `build-error.json` and partial bundle.
Unusual SC headers are retained as input warnings, not repaired or certified.

Available matching discs are found under `Test Roms/` and copied with all cue
dependencies. Use `--roms DIR` (repeatable) for other locations or `--no-roms` to
leave every ROM folder empty. Use `--bios FILE` to copy an explicit BIOS; otherwise
the BIOS folder starts empty. No source caches, previous runs, desktop build or
already-covered game fixtures are bundled. The included Legends 2 emulator
recipe remains a game-specific reference, requiring adaptation for other games.

Fill `inputs/roms/PRODUCT-CODE/` with complete disc subfolders, then from inside
the generated bundle run:

```sh
python3 tools/save_research/build_bundle.py --refresh-roms .
python3 tools/save_research/validate_bundle.py . --output results/preflight-001.json
sha256sum -c checksums.sha256
sha256sum -c tooling.sha256
```

Refresh reads disc contents, updates `jobs.json` with exact serial matches and
missing/ambiguous states, records BIOS paths/hashes, and rebuilds input checksums.
It does not rename images, guess save/disc aliases, or infer matching revisions.
All runtime paths are relative to the bundle. Suites live at its root to keep
their input paths inside the parser runner's path boundary. Templates and empty
expectations are starting points, not complete tests or working game decoders.

`validate_bundle.py` checks all supplied input/tooling hashes, exact CUE
dependencies and embedded disc IDs, suite/sample metadata, parser syntax and
portable paths without executing candidate parsers. Missing/ambiguous ROMs are
reported as warnings so file-only research can proceed; use `--require-roms` to
make those conditions fail preflight. It explicitly lists template parsers and
suites lacking known-value expectations. Existing evidence files are preserved.
`tooling.sha256` pins the preparation tools/instructions separately from private
inputs; parser implementations and suites remain editable research deliverables.
ROM refresh rejects altered save samples instead of accepting new hashes for
them. Add new evidence as new inputs/cases with explicit pinned hashes.

The copied unit tests run with Python 3 alone:

```sh
python3 -m unittest discover -s tools/save_research -p 'test_*.py'
python3 -m unittest discover -s tools/ps1_replay -p 'test_*.py'
```

For a server readiness pilot, use the
[startup preservation protocol and suggested prompt](../ps1_replay/README.md#readiness-pilots-and-startup-card-writes).
Report bundle integrity, engine access, boot, load, input/replay and preservation
as separate gates. A failed startup preservation probe calls for bounded
diagnosis, not relabeling the run as passed or abandoning unrelated checks.
This does not qualify in-game saving or authorize the overnight batch.

## Identify discs directly

```sh
python3 tools/save_research/disc_id.py '/path/to/game.cue'
# Identify every supported disc in a folder, including nested folders:
python3 tools/save_research/disc_id.py 'Test Roms'
# Preview renames (no files changed):
python3 tools/save_research/disc_id.py 'Test Roms' --rename --dry-run
# Apply renames, retaining a timestamped JSONL journal in that folder:
python3 tools/save_research/disc_id.py 'Test Roms' --rename
```

Folder mode processes CUE sets once, including their audio tracks, plus ISO and
unreferenced BIN files. Rename mode uses `SLUS-00603.cue` and `SLUS-00603.bin` for
a single-file disc; separate track files become `SLUS-00603 (Track 01).bin`, etc.
It rewrites CUE FILE references while preserving comments, BOM and line endings.
Disc image bytes and game folder names are unchanged. Archives, CHD/PBP,
incomplete downloads, documentation and symlinks are ignored during folder scans;
extract archives before scanning. Explicit unsupported file inputs report errors.
Unidentified discs, shared CUE dependencies and conflicting destinations are
reported and skipped, never overwritten or guessed. Exit status is nonzero if
any candidate was skipped or failed; other valid discs can still be renamed.
Rerunning is safe: files already using the canonical names report `unchanged`.
The journal records original/new paths and original CUE text before mutation,
then records completion or failure. Ordinary failures roll back that disc;
after a process interruption, use the journal to inspect/recover partial changes.
Use `--log /path/to/new-log.jsonl` to choose its location. Input changes between
planning and applying are rejected. Renaming needs a filesystem with hard-link
support. It reads only disc metadata, rather than hashing/copying entire images.
If renaming ROMs already inside a bundle, run `build_bundle.py --refresh-roms`
afterwards to update its recorded paths and checksums.

The standard-library reader resolves ISO9660 directory entries, reads
`SYSTEM.CNF`'s PS1 `BOOT` target, and verifies its PS-X EXE header. For example,
`SLUS_006.03;1` becomes `SLUS-00603`. It supports cooked 2048-byte ISO and common
Mode 1/2352, Mode 2/2352 and Mode 2/2336 cue/bin data layouts, including INDEX 01
offsets and separate audio-track dependencies. Reads are bounded and do not scan
arbitrary strings through the disc. CHD/PBP/archives, nonstandard boot filenames,
missing configuration, unsupported layouts and malformed extents remain explicit
errors. A serial identifies the boot release, not a certified revision or save
compatibility. Save IDs may differ for multidisc games or unusual save naming.
See [Sony's boot/executable naming specification](https://psx.arthus.net/sdk/Psy-Q/DOCS/TECHNOTE/TECHCHK.PDF),
and [PSX-SPX disk-image formats](https://psx-spx.consoledev.net/ps1/cdr/cdromfileformats/diskimages/).

## Parser plug-in boundary

Implement `inspect_save(source: bytes, metadata: dict) -> dict` in a parser module.
The harness passes individual MCS bytes or normalized SC payloads unchanged.
The parser owns exact release, identifier, size, header and layout validation.
Whole-card extraction remains a separate preparation step using the existing
attributed card engine; join linked blocks in chain order. Keep original cards
for provenance, especially deleted-only samples. The harness is not a card parser.

Use relative paths in a suite like:

```json
{
  "schemaVersion": 1,
  "parser": "parsers/azure_dreams.py",
  "cases": [{
    "id": "pal-profile-1",
    "input": "inputs/saves/azure-dreams.mcs",
    "sha256": "REPLACE_WITH_SHA256_OF_THE_EXACT_INPUT_FILE",
    "metadata": {"productCode": "SLES-01219", "region": "BE", "identifier": "00"},
    "expect": {},
    "evidence": {}
  }]
}
```

The placeholder above intentionally cannot pass. Pin the real input hash and
replace the template with a parser. A successful parser must return a nonempty
`fields` object and `checksumOk: true`, `false` or `null`. Optional keys may
include `unknowns`, profile records and offset/evidence annotations. Unknown
IDs remain explicit raw values; checksum mismatch may still return decoded data.
Keep a parser self-contained or declare any helper source files in the suite's
`parserDependencies` array of relative paths. Declared helpers are hashed and
checked for changes too; undeclared imports are not covered by the source ledger.

Add independently justified expectations as JSON pointers, for example
`"expect": {"/fields/zenny": 11950, "/checksumOk": true}`. Attach per-field
evidence in `evidence`, such as source commit/spec section, observed game display,
or controlled experiment path and confidence. Do not create expectations from
the candidate parser's own output and call them verified. The runner records
evidence references; it does not inspect their truth or certify field meanings.

Add negative cases with their own pinned fixture/hash and
`"expectErrorContains": "unsupported identifier"`. Only the parser's `ValueError`
counts as an intentional rejection; crashes/timeouts never satisfy this test.
Include truncated layouts, wrong releases/identifiers, unknown values,
documented boundary values and each checksum's corruption probes where supported.
Corruption fixtures are disposable copies, not repairs to original saves.

```sh
python3 tools/save_research/harness.py suite.json --output results/run-001.json --timeout 10
```

Each case runs in a fresh process with a deadline. Failures do not stop remaining
cases. Reports retain decoded values, parser diagnostics/errors, exact parser,
manifest and fixture hashes, declared parser-source hashes, runner hash, Python
version, expected-value checks, timing and input preservation.
Sibling parser helpers can be imported normally from the parser directory;
declare them in `parserDependencies` for the source-hash ledger. Inputs are
checked again after the complete suite to catch changes by later cases.
Existing reports are never overwritten. `contract_passed` means no known-value
expectations were supplied; `expectations_passed` means supplied assertions passed.
Neither proves save/load persistence or that guessed labels are correct. Exit 0
means no declared check failed, including suites with contract-only cases.

This subprocess boundary is not a security sandbox. For enforced read-only
inputs, run inside a Docker or Podman worker with the suite mounted read-only
and a separate writable results mount. Do not mount a container-engine socket
into the parser worker. A host-side preservation check detects changes but
cannot prevent or repair them. Keep run artifacts under ignored `runs/` folders.

## Overnight completion criteria

Keep one suite, parser, partial format spec and field ledger per exact release.
Track missing independent expectations, negative/boundary tests and unknown
checksums separately from mechanical passes. Only mark researched fields verified
when their documented evidence supports that confidence. Verify game save/load
persistence separately when generating new samples; emulator state restore is
not an in-game save/load test. A practical overnight job can deliver partial
decoders and exact blockers without claiming complete game coverage.

Check the runner without private assets:

```sh
python3 -m unittest discover -s tools/save_research -p 'test_*.py' -v
```
