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

### Sync saves to a local directory

Open a card file or read Slot 1, then click **Sync** in the top bar and choose a directory. Sync exports all active saves as `.mcs` files in game folders named by PS1 product code (`unknown-game` when missing). Each file contains the save directory header and all linked blocks in chain order, including non-contiguous chains.

Run Sync again into the same directory to update changed saves and skip identical files. Existing saves absent from the card stay in the directory. Deleted saves are excluded. Sync exports the open card snapshot; reload Slot 1 first if the physical card has changed. It never writes to the card.

Sync preserves each distinct save version in `<game>/.snapshots/<save filename>/<content hash>.mcs`, with capture time and source information in a JSON sidecar. The ordinary `.mcs` file remains the latest dump. Existing backups are archived before replacement. Repeated identical contents reuse a snapshot, including when a save moves to different blocks; different contents under the same game filename are preserved as separate snapshots. This protects saves from different cards that use the same filename, but does not infer which playthrough they belong to. Capture sources describe a loaded image, not a stable physical card identity.

### Browse local backups

Click **Local backups** under All Saves, then **Choose folder** to configure your save directory. The app remembers it across launches and uses it for Sync. It reads `.mcs` files in that directory and its game subfolders, groups them by product code, and shows each save’s icon, title, and block count. Select a save for metadata and **Reveal in Finder**.

The inspector’s **Snapshots** menu lets you inspect and reveal earlier versions. History files are excluded from the main save count. Snapshotting stops before replacing the latest dump if its history cannot be safely written. Snapshot publication requires filesystem hard-link support. Keep the latest `.mcs` alongside its history; archived versions without a latest file are not currently listed.

Use **Refresh** after external changes or **Change folder** to select another directory. Unreadable files are reported while other saves remain visible. Library reads leave the open card and backup files untouched. Whole-card `.mcr` backups continue to open through **Open card**.
