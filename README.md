# memcard-viewer

PlayStation 1 memory card viewer and backup tool. Icon-first gallery, whole-card clone, and compose a new card from selected saves.

- **License:** GPL-3.0-or-later (parser ported from [MemcardRex](https://github.com/ShendoXT/memcardrex) Core by Shendo)
- **v1 console:** PS1 only (128 KB / 15 slots)
- **Formats:** raw `.mcr` family, DexDrive `.gme`, VGS `.vgs`/`.mem`
- **Hardware:** PS3 Memory Card Adaptor (`054C:02EA`) **read** and backup. No adaptor write in v1.

Requires [Rust](https://rustup.rs/) (`rustc` / `cargo`) and libusb (`brew install libusb` on macOS).

```bash
npm install
npm test
npm run tauri
```

`npm run tauri` starts the desktop app (Vite + Rust). Open `blue.mcr` from the project root, or plug in a PS3 Memory Card Adaptor and use **Read adaptor**.
