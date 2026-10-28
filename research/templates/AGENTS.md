# Research execution

Work within the brief, supplied inputs and Runner deadline. Preserve original inputs.
Write useful partial artifacts under outputs/ and maintain outputs/checkpoint.json.
Use the JSON schemas in inputs/contracts.json: result, checkpoint and research.
The closeout result uses format runner-result/v1 with outcome complete/partial/blocked,
reason scope_complete/budget_exhausted/no_actionable_work/user_requested, summary, next_action and blockers.
Complete requires scope_complete; blocked requires no_actionable_work and nonempty blockers.
Respect Runner finalization requests; finish early when no permitted work remains.

Write outputs/research.json using runner-research/v1. Include scope, findings, evidence, validation,
reproduction, deliverables and limitations. Findings refer to evidence IDs and state confidence
(supported/tentative/unresolved). Validation checks record method, expected, observed, status
(passed/failed/inconclusive/not_run) and evidence IDs. Passed checks require retained evidence.
Include at least one validation check, including an explicit not_run explanation if blocked.
Evidence and deliverable paths start with outputs/ and must be published files.
Record dependency hashes/provenance and reproduction commands; disclose external tool requirements.
Validation performed during research is welcome. Distinguish observations from assumptions,
synthetic checks from native behavior, and delivery hashes from scientific validation.
Do not claim unperformed checks passed. Do not send messages or change external systems without task authorization.

## PS1 research boundary

Keep research read-only and limited to @@PRODUCT_CODE@@ (@@SERIAL@@, @@TITLE@@).
Preserve inspect_save(source, metadata), unknown IDs, bank ambiguity and checksum nulls.
Do not change production application code, original inputs, Git history or credentials.
Record a small summary target and evidence ledger. Parser passes and synthetic fixtures
do not prove field meanings; emulator state restoration does not prove in-game saving.
Require independent known-value evidence and relevant boundary/malformed cases.
Use the supplied ps1-save-research skill and emulator-replay skill for runtime work.
Verify actual dependency bytes and the tool-image identity on this host before use.
Record expected/observed hashes, locations, provenance and verification time.
No emulator until exact-release assets, engine architecture/access and mount visibility
are checked. Use read-only originals, disposable cards, bounded operations, unique owned
workers and guaranteed teardown. Keep useful failures and partial findings; never turn an unknown checksum into passing.
Keep outputs within server publication quotas; never publish ROM or BIOS bytes.
Runner owns deadlines and finalization. A copied /goal prompt supplies scientific criteria
only; do not start another scheduler, goal or availability retry loop.

## Emulator image

The emulator image is provisioned from the packaged Containerfile. Its identity is input hash `@@INPUT_HASH@@`, buildId @@BUILD_ID@@ and zip sha `@@ZIP_SHA@@`. The runtime image ID comes from `.runner/tool-images.json` in v3, or from the output of `host.py ensure` in v2, and is recorded as evidence.
Tag: `@@TAG@@`.

@@ENSURE@@

Unpinned package installs and toolchain rebuilds are forbidden. The declared tool image built from the packaged Containerfile is the exception.

## Required checks

Runtime steps listed in required_checks are mandatory. If one can't be done, report `partial` with the blocker, never `complete`.

@@CHECKS@@

Native load of the supplied save is the `native-load` check. Load the save in the emulator and read the completion percent on the save or select screen. Retain the capture. This load is required.
