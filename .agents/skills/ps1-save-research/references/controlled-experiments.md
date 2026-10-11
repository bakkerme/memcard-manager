# Controlled persistence experiments

Use when file comparisons cannot resolve a candidate, when capturing new game saves, or when claiming persistence. The task may need only part of this protocol; choose the smallest experiment that distinguishes the live hypotheses.

## Experiment record

Record an experiment ID and question, game/release and save identifier, original input hashes, normalization/extraction method, emulator build/image/settings, baseline state and paired card, recipe/actions with frame checkpoints, before/after screenshots and RAM, generated card/save hashes, and observed reload behavior. Retain the record with private evidence outside the disposable worker. Every offset must say whether it is card, wrapper, SC payload, game-data-relative, RAM, or a record-relative coordinate.

Use a field ledger like this; leave unmeasured cells explicit:

| Origin + offset | Width/encoding | Candidate | Evidence | Confidence | Counterexample / next test |
| --- | --- | --- | --- | --- | --- |
| SC payload + `0x...` | u16 little endian | Current currency | Sample hashes, displayed values, pack/load anchors | Candidate / corroborated / code-traced / save-reload-verified | Another value, threshold, release |

Confidence is not a mandatory ladder. A code trace can establish serialization without live experiments, while a controlled save/load can establish persistence without locating the packer. Document both claims separately.

## Design a discriminating comparison

Restore the same baseline/card for each branch. Capture a no-op save control to measure incidental changes, then change one meaningful variable. Try a second value or reversal and repeat if noise prevents isolation. Record side effects rather than claiming that only the intended variable changed.

Useful probes include changing an equipped item while holding inventory stable, acquiring an item without equipping it, consuming one unit, testing a value across an integer boundary, changing one name character, and triggering an event while inspecting nearby flag bits. Do not prescribe an inventory change when the research question can be answered read-only.

Create game saves only in disposable writable card copies within the requested research scope. Keep original inputs read-only. Separately qualify that the game saves successfully, the emulator flushes the card to the retained file, and a fresh game load reads the intended slot. Shut down or use a documented flush before copying card evidence; a screenshot of a successful save dialog alone does not establish disk persistence. Recompute original hashes afterward.

The existing Legends 2 input/replay harness rejects card changes by design. Save experiments require a distinct mode/acceptance contract that allows the intended save changes while retaining originals and exact byte-diff evidence. Do not disable preservation checks globally or mislabel a save experiment as the original passing replay test.

## Trace serialization

Use source/decompilation or debugger observations for the exact region/revision. Record the active overlay and scene because the same virtual address may hold different code later. Follow the runtime value to stores into the save buffer, and follow load reads back to runtime state. Retain enough instruction/operation anchors to reproduce the inference without redistributing the executable.

Look for transformed values, sign extension, packed bits/words, scattered stores, count-versus-capacity, record loops, alignment gaps, and selective/global merge rules. A `memcpy` proves a span was copied; it does not name every byte. Distinguish nominal allocation size, actual serialized range, checksum coverage, and card block allocation.

## Check integrity separately

Test the proposed checksum against independent valid inputs, then alter data inside a covered range and the stored checksum in disposable analysis copies. Compare outside-range mutations too when useful. Record that a negative corruption test validates checksum detection, not game semantics. Never mark a checksum verified because the stored and computed values were both manufactured by the same speculative algorithm.

Corrupted/edited cards need not be loaded in the game to prove an offline reader rejects them. If testing game acceptance is necessary, keep it in a separate disposable experiment and record whether the game checks a preview segment or full save.

## Falsify and report

Check independent samples, different positions, changed values, empty/unknown IDs, and relevant difficulty/progress conditions. Treat another region/revision as unsupported until positively validated. A label on a downloaded save is a claim to cross-check, not ground truth about its bytes.

State failures, incidental diffs, and alternatives that remain possible. Mark unavailable live verification precisely. The useful outcome may be a candidate offset plus a discriminating next test, rather than a premature implementation.
