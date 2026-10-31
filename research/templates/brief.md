Read inputs/question.md. Research @@TITLE@@ (@@PRODUCT_CODE@@) and validate the findings against its acceptance criteria.
Include every required dependency in this payload before submission; identify unavailable inputs explicitly.
Retain failed and inconclusive checks. Report blockers and useful partial results honestly.
Deliver outputs/result.json, outputs/handoff.md and outputs/research.json as described in AGENTS.md.
Also leave outputs/field-ledger.md, outputs/checkpoint.json, outputs/tool-image.json, outputs/decoder/, and a small outputs/fixtures/ set loose.
Pack screenshots, calibration dumps, and action logs into the archives named in inputs/question.md and check the active server quotas (currently 4,096 output entries). Measure each compressed archive against the per-file limit and all published files together against the generation total, as described in inputs/question.md.

The emulator image is provisioned from the packaged Containerfile. Its identity is input hash `@@INPUT_HASH@@`, buildId @@BUILD_ID@@ and zip sha `@@ZIP_SHA@@`. The runtime image ID comes from `.runner/tool-images.json` in v3, or from the output of `host.py ensure` in v2, and is recorded as evidence.

Runtime steps listed in required_checks are mandatory. If one can't be done, report partial with the blocker, never complete.

@@CHECKS@@

@@ENSURE@@
