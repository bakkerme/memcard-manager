# memcard-viewer

Primary plan is the canvas beside chat: open `memcard-viewer-plan.canvas.tsx` in this Cursor project.

## Snapshot

- **Product:** PS1 memory card viewer and backup tool. Icon-first gallery, not a filename table. Whole-card clone and compose-a-new-card from selected saves.
- **Stack:** Tauri 2 (Rust + React), macOS first. USB via rusb/libusb. Do not wrap MemcardRex UI.
- **v1:** PS1 only. Virtual raw + GME + VGS. PS3 Memory Card Adaptor **read** and backup. No hardware write, no PS2, no VMP/MCX, no DexDrive.
- **Reuse:** Port `ps1card`, `PS3MemCardAdaptor`, and `HardwareCardTransfer` from `/Users/brandon/sources/memcardrex` as spec + test oracle. GPL-3.0 if Core is ported.
- **Workspace:** this directory was empty at plan time (no git, no code).
