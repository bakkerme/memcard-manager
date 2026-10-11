# Startup preservation in a readiness pilot

Use this protocol when an emulator readiness pilot fails because a disposable
card changes during startup. Keep the original failed verdict and its evidence.
A setup-validation request includes bounded diagnosis within the requested
pilot; it does not require abandoning all checks after the first failure or
asking again before each diagnostic retry. Explicit trial/time limits still
apply. Additional games, save generation and batch runs remain separate scope.

## Establish the initialization boundary

1. Preserve the failed run, before/after card bytes, exact changed offsets,
   source hashes and cleanup outcome. Original inputs stay mounted read-only.
   Compare persisted card files after clean emulator exit so pending writes
   are included. A changed-byte count alone does not identify the write.
2. Locate each difference using the card format. On a raw 128-KiB PS1 card,
   block 0/frame 63 occupies `0x1F80..0x1FFF` and is the write-test frame.
   Card acknowledgement can write there before any save is loaded. These
   offsets do not apply directly to wrapped cards or other media formats.
3. Reproduce the change in a fresh process using a fresh disposable copy and
   the same BIOS, emulator build and settings, without loading or saving a game
   save. Retain the boot-only evidence. Verify all originals and every byte
   outside the documented scratch frame are unchanged, including directory
   entries, allocation/broken-sector metadata and all save blocks.
4. If the boot-only change is reproducible and confined to that frame, retain
   the observed initialized disposable card as a new private baseline artifact.
   Record original and initialized hashes, exact old/new bytes, boot conditions
   and the comparison boundary. Reuse those observed bytes; do not guess a
   replacement, erase unfamiliar contents, or change the original input hash.
5. Start measured boot/load/input/replay from exact copies of that frozen
   initialized card. Require exact whole-card equality after each measured
   run, including scratch bytes; verify after clean exit. When native states
   are used, freeze and verify the matching state/card pair. A repeat write
   that changes the initialized baseline still fails preservation.

The checked-in Legends 2 launcher instead has a guarded, previously qualified
sector-0-to-sector-63 preparation for its specific fixture. It accepts only a
zero-filled frame or one already equal to sector 0. That rule and its historical
three changed bytes are not a generic preparation or expected byte count for a
new game/card. New observations need their own evidence and recipe.

## Verdicts and stopping conditions

Report bundle integrity, engine access, boot, load, input/replay and preservation
as separate gates. Describe the original probe as failed and the initialization
as a separately evidenced preparation. Only successful measured trials support
a replay pass; boot-only evidence does not prove save/load persistence.

A change to an original, a write outside the scratch frame, inconsistent boot
results, or an unexplained payload remains a failure of the affected gate.
Retain it and stop that measured trial; continue independent static checks when
useful. Do not mask scratch bytes, repeatedly adopt changed cards as new
baselines, or add a universal exception for arbitrary writes.

Background: [PS1 card layout](https://psx-spx.consoledev.net/ps1/sio/controllersandmemorycards/memory-card-data-format/)
and [card acknowledgement](https://psx-spx.consoledev.net/ps1/sio/controllersandmemorycards/memory-card-read-write-commands/).
