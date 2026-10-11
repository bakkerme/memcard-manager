# Standalone parser deliverables

When the requested output is a portable research parser, use `tools/save_research/harness.py` and its README if supplied. Python 3.10+ is sufficient for that runner. A parser-only kit need not contain the desktop app, Rust or Node dependencies. It still needs normalized save inputs, pinned hashes, the exact release/identifier boundary and evidence for interpreted fields.

Implement `inspect_save(source: bytes, metadata: dict) -> dict`, returning nonempty JSON `fields` and `checksumOk` as true, false or null. Validate identifiers and layout; raise ValueError for intentional unsupported/malformed input. Keep unknown values explicit. The harness does not extract cards: prepare MCS/SC samples using the existing attributed card engine, preserving chain order and source provenance. Retain original cards for deleted-only samples; readable bytes do not prove loadability.

Use relative paths in one suite per release, with pinned input hashes. Add independent known-value expectations with evidence references, then negative/truncation/release/boundary and checksum-corruption cases as relevant. Candidate output is not its own evidence. Record expected values as byte regressions if no independent observation exists.

Run `python3 tools/save_research/harness.py SUITE.json --output NEW_REPORT.json`. Contract-only success is labelled `contract_passed`; supplied assertions produce `expectations_passed`. Reports include input/parser/manifest hashes, per-case errors and timing. Neither status certifies field meaning or in-game save persistence. Document unknown checksums and untested fields in a partial spec/ledger; do not promote a mechanical pass to complete parser coverage.

The runner uses a child-process timeout but is not a security sandbox. Mount original inputs read-only inside Docker or Podman when enforcing preservation, with a separate writable evidence directory. Keep the engine/socket on the coordinator host, not inside parser workers. For dynamic experiments use emulator-replay; Docker/Podman portability does not certify a new game, host, save routine or baseline.
