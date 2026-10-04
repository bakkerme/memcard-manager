# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Primary users hold a physical PlayStation 1 memory card and a PS3 Memory Card Adaptor. They plug the card in to see what is on it, keep a save-level backup of what changed, and work with a collection that outlives any one card: named virtual cards plus a library of individual saves.

On-disk card files remain a way in and out. They are not the job the product is for.

## Product Purpose

memcard-viewer is a desktop **memory card manager** for physical PS1 cards. Success is: plug in a card, recognize the saves from their icons on a 15-block map, automatically **back up changed saves** into a personal save library (the save is the unit of record, not a whole-card image), keep those saves on named virtual cards, and later keep a physical card in two-way sync with that collection.

## Positioning

A polished physical-card workflow with a persistent collection: adaptor in, per-save auto-backup, named virtual cards, a flat save library. Neighboring tools, including MemcardRex (the Windows editor whose parser this project ports), treat a single card image as the document. This product's claim is three layers at once — live hardware cards, named virtual cards, and a save library — with a finished desktop experience.

## Operating Context

- Desktop app: Tauri 2 shell, React/Vite UI, Rust parser and USB. `npm run tauri` launches it. Bundle config records macOS 13+. libusb is required for the adaptor (`brew install libusb` on macOS).
- Hardware: Sony PS3 Memory Card Adaptor USB `054C:02EA`. Current firmware path reads 1024 frames. macOS may show a USB permission prompt. The adaptor has **one** physical slot.
- Shipping v1 loop: open a card from disk or read the adaptor, browse a 15-slot icon gallery, inspect save metadata, backup a whole card file, compose a new `.mcr` from selected saves.
- Destination workspace (three layers, confirmed):
  - **Physical cards** — live adaptor-backed cards. The UI lists them; one connected card is the current hardware truth.
  - **Virtual cards** — named, persistent 15-block cards. These replace ad-hoc “open a file / compose a new `.mcr`” as the way a card-shaped collection lives on disk.
  - **Save library (“All Saves”)** — a flat collection of individual saves. Near term this is **local backups**. Cloud is a later bucket in the same library, not the next chapter.
- Near-term hardware loop: adaptor present → detect a PS1 card → **auto-backup changed saves** into the library (and onto related virtual cards as needed) → browse the 15-block map. Automatic runs remain auto-backup. Manual Sync exports active saves to a chosen local directory, grouped by game; it is one-way and read-only for the card.
- Later hardware loop: proper **two-way sync** between a connected physical card and the collection (library + virtual cards). Write-back onto hardware arrives with that chapter.

## Capabilities and Constraints

Confirmed in shipping v1:

- PS1 cards only: 128 KB, 15 slots. Not PS2 or other consoles.
- Read raw `.mcr` family, DexDrive `.gme`, and VGS `.vgs`/`.mem`.
- Icon-first gallery with linked blocks, deleted/ghost saves, PocketStation software, and XOR status.
- Manual Sync: choose a local directory and export active saves as `.mcs`, grouped by product code (unknown codes use `unknown-game`). Stable PS1 save filenames distinguish saves within a game. Changed files update, identical files are skipped, and files absent from the card are retained. Deleted saves are excluded. Sync uses the currently opened card snapshot; reload Slot 1 to capture later hardware changes.
- Local backups: a persisted, user-chosen directory shared with Sync. Recursively read `.mcs` files, grouped by game, with icons, metadata, file reveal, folder change, and refresh. Invalid files are reported without hiding readable saves. Library reads never replace the currently open card.
- Save snapshots: manual Sync archives existing and incoming versions before updating the latest dump. Distinct game filename and save payload combinations are preserved; identical contents reuse a snapshot even across card layouts. Local backups exposes version history and capture sources. Sources identify loaded card images, not physical cards. Multiple playthroughs under the same filename remain separate snapshots; automatic playthrough identity and stable card IDs are deferred.
- Whole-card backup to a file; compose a new raw `.mcr` from selected master saves (rejects selections over 15 blocks).
- Adaptor **read** only. No hardware write is implemented. The adaptor has one physical slot; the sidebar lists Slot 1 only when that adaptor is present, with no second-slot row.
- Parser ported from MemcardRex Core by Shendo. License: GPL-3.0-or-later.
- Inspector already exposes title, region, product code, identifier, and linked slot chain.

Confirmed product direction (not all shipping yet):

- Three-layer manager: physical cards, named virtual cards, and a flat save library.
- Automatic **backup** of changed saves at save granularity when a physical card is connected. Manual Sync means card-to-directory save dumps; automatic backup and future two-way sync remain separate capabilities.
- The 15-block map is the card view: every block is a tile; linked continuation blocks stay visible as part of the save (e.g. 2 of 3), including non-contiguous chains.
- Card operations: open a card file, import a save onto a card, backup, create a new (virtual) memory card.
- Save operations: inspect metadata, export a single save (`.mcs` in the vision mock), duplicate, delete. Multi-select saves on a card.
- Two-way sync with a connected physical card, including write-back onto hardware — later chapter, still part of the job.
- Cloud as a library bucket beside local backups — later chapter, not the next build. Provider and mechanism are undecided.

Undecided:

- Display name beyond the repo and window title `memcard-viewer`. The vision mock’s subtitle “PlayStation Memory Card Manager” describes the job; it is not a rename and not a Sony-licensed brand.
- Where virtual cards and the save library live on disk, and how they are named/organized.
- Whether “Open card” from an arbitrary file stays first-class once virtual cards exist, or becomes import-into-library.
- Whether save titles are user-editable, or inspect-only.
- Cloud provider, account model, and what “connected” means for that bucket.

## Brand Commitments

- Product name in the app and bundle: `memcard-viewer`.
- GPL-3.0-or-later. MemcardRex Core (Shendo) attribution must remain with the parser.
- Top-left wordmark: the PlayStation monogram beside `memcard-viewer`. That monogram appears only there — do not add a “PlayStation Memory Card Manager” lockup, Sony wordmarks on the card plate, or PlayStation red as an accent.

## Evidence on Hand

- Working desktop app: gallery, inspector, adaptor probe/read, file open, backup, compose.
- README states v1 formats and the adaptor read path.
- Hardware tests in `src-tauri/src/usb.rs` (ignored by default; require a real adaptor and card).
- README mentions `blue.mcr` at the project root as a sample; it was not present in the workspace during init.
- Vision mock (IA and jobs, not visual law): `ChatGPT Image Sep 15, 2026, 07_41_27 AM.png`. An earlier variant at `ChatGPT Image Sep 15, 2026, 07_24_28 AM.png` added per-save screenshot previews and checksum/CRC; those are not confirmed product.
- Do not invent testimonials, user counts, benchmarks, or adaptor compatibility beyond `054C:02EA`.
- Do not treat mock copy such as “15 saves” on a 15-block card that also holds a 3-block save as a spec.

## Product Principles

1. The unit of backup is a save; the unit of layout is a 15-block card. Both are real; neither replaces the other.
2. Physical cards and the PS3 adaptor are the live usage scene. Virtual cards and the save library are how the collection persists when the adaptor is unplugged.
3. PS1 slot geometry (15 blocks, links, icons) is the product's truth. Linked blocks stay visible; do not pretend other consoles.
4. Auto-backup of changed saves is the near-term hardware loop. Two-way sync and cloud are later chapters; dumps-only is the current implementation, not the destination.
5. Copyleft and parser credit travel together: GPL-3.0-or-later and MemcardRex attribution stay in force.
