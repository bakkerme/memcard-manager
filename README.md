# memcard-viewer

A desktop PlayStation 1 memory card manager built with Tauri, React, and Rust. Browse all fifteen blocks, back up a whole card, export saves to a local library with version history, and compose a new card from selected saves.

![memcard-viewer desktop app showing real saves from blue.mcr and the linked-block inspector](docs/images/memcard-viewer.jpg)

*The macOS app displaying real saves and icons from `blue.mcr`, with Worms World Party’s linked blocks selected.*

## What works today

- **Fifteen-block gallery:** animated pixel icons, empty blocks, deleted saves, and linked continuation blocks in their physical positions, including non-contiguous chains.
- **Save inspector:** title, region, product code, identifier, linked blocks, and directory XOR checksum status.
- **Whole-card backup:** timestamped `.mcr` files in `Documents/memcard-viewer/backups`, without overwriting earlier backups.
- **Compose a card:** select saves and export a new raw `.mcr` with their linked blocks packed together.
- **Local save library:** export active saves as `.mcs`, browse them by game, and inspect preserved versions and capture sources.
- **Read physical cards:** Sony PS3 Memory Card Adaptor (`054C:02EA`), with read progress and automatic adaptor detection. Hardware access is read-only.

PS1 only: 128 KB and 15 save blocks. Supported card formats are raw `.mcr` and related formats, DexDrive `.gme`, and VGS `.vgs`/`.mem`. PS2, VMP, and MCX are not supported.

## Run locally

On macOS, install Node.js (22.12+), [Rust](https://rustup.rs/), the Xcode Command Line Tools, and libusb. The app targets macOS 13 or later.

```bash
xcode-select --install  # if the command line tools are not installed
brew install libusb
npm ci
npm run tauri -- dev
```

Open `blue.mcr` from the project root with **Open card**. With the adaptor connected, click **Slot 1** or its reload button to read a physical card. The app also attempts a read when it detects an adaptor at startup. A card read does not automatically back up saves.

The web preview runs with `npm run dev` at `http://localhost:1420`. Card parsing, filesystem operations, and USB access require the desktop app. For a layout preview with explicitly synthetic icons and save data, visit `http://localhost:1420/?demo=1` in development mode.

## Back up or compose a card

Click **Backup** to save the complete open card as a timestamped raw `.mcr`. The completion banner includes **Reveal in Finder**.

Click a save to select it; use **Cmd-click** on macOS or **Ctrl-click** to select multiple saves. **New memory card** (also **Add virtual card** in the sidebar) exports the selected saves to a new `.mcr` through a save dialog. Linked blocks travel together. The result is a card file; a persistent collection of named virtual cards is not implemented yet.

## Export saves with Sync

Open a card file or read Slot 1, then click **Sync** and choose a directory. Sync exports all active saves as `.mcs` files in folders named by PS1 product code (`unknown-game` when missing). Names containing lowercase or unsafe characters are encoded to keep distinct PS1 filenames separate on case-insensitive filesystems. Each file contains the save directory header and all linked blocks in chain order.

The chosen directory is remembered and shared with **Local backups**. Later Sync runs update changed saves and skip identical contents. Files absent from the open card are retained; deleted saves are excluded. Sync exports the currently loaded snapshot, so reload Slot 1 first if the physical card has changed. It never writes back to the card.

### Preserved versions

Before replacing a latest dump, Sync preserves both the existing and incoming versions in `<game>/.snapshots/<save filename>/<content hash>.mcs`, with capture time and source information in a JSON sidecar. The ordinary `.mcs` remains the latest dump.

Identical contents reuse a snapshot, even when a save moves between blocks. Different contents under the same game filename remain separate versions. Capture sources identify loaded card images, not physical card serial numbers; the app does not infer which playthrough a version belongs to.

Snapshot publication requires filesystem hard-link support, such as APFS; FAT/exFAT destinations are not supported. If history cannot be safely written, Sync stops before replacing the affected latest dump. Keep each latest `.mcs` alongside its history: archived versions without a latest file are not currently listed in the library.

## Browse local backups

Click **Local backups** under All Saves, then **Choose folder**. The app recursively reads `.mcs` files, groups them by product code, and shows their icons, titles, and block counts. Select a save for metadata and **Reveal in Finder**. The inspector’s **Snapshots** menu lets you inspect and reveal earlier versions; history files do not inflate the main save count.

Use **Refresh** after external changes or **Change folder** to choose another directory. Unreadable files are reported while readable saves remain visible. Browsing the library leaves the open card and backup files untouched. Open whole-card `.mcr` backups through **Open card**.

## Current limits

Automatic save backup, hardware write-back/two-way sync, cloud storage, persistent virtual-card management, and individual save import/export, rename, duplicate, and delete controls are not implemented. Unavailable controls remain visible and disabled. Manual **Sync** is a one-way export to a local directory.

## Development

```bash
npm test                           # Rust unit tests and Vitest
npm run build                      # TypeScript check and production web build
npm run tauri -- build --no-bundle  # native release binary
npm run tauri -- build --bundles app # macOS app bundle
```

Hardware integration tests are ignored by default and require a real adaptor and PS1 card. Application code is in `src/`; the Rust parser, backup/library logic, and USB implementation are in `src-tauri/src/`. [PRODUCT.md](PRODUCT.md) records scope and [DESIGN.md](DESIGN.md) records the visual system.

## License and credits

GPL-3.0-or-later. The parser is ported from [MemcardRex](https://github.com/ShendoXT/memcardrex) Core by Shendo; attribution remains with the parser. The bundled Encode Sans Expanded font is distributed under the [SIL Open Font License](src/assets/fonts/OFL.txt). The PlayStation monogram uses the Simple Icons CC0 silhouette. This is an independent project, not an official Sony application.
