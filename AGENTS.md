# AGENTS.md

## Project

`memcard-viewer` — desktop PS1 memory card manager (Tauri 2 + React/Vite + Rust). Icon-first 15-block gallery, whole-card backup, compose-a-card from saves, PS3 Memory Card Adaptor read.

- **Product source of truth:** `PRODUCT.md` (scope, three-layer direction, brand commitments). **Visual source of truth:** `DESIGN.md` (tokens in frontmatter + Do's/Don'ts). Read both before UI or scope changes.
- **License:** GPL-3.0-or-later. Parser ported from MemcardRex Core by Shendo — keep that attribution with the parser.

## Commands

```bash
npm install
npm test        # cargo tests (src-tauri) + vitest
npm run tauri   # desktop app (Vite + Rust)
npm run dev     # web-only Vite dev (port 1420, strict)
npm run build   # tsc --noEmit && vite build
```

- Requires Rust (`rustup.rs`) and libusb (`brew install libusb` on macOS).
- Sample card: `blue.mcr` at project root. Open it via file-open or adaptor **Read** path.
- Hardware tests in `src-tauri/src/usb.rs` are `#[ignore]` — they need a real adaptor + card. Don't run them in CI-style verification.

## Layout

- `src/` — React UI. `src/card/` (engine, api, demo), `src/components/` (`PixelIcon.tsx`), `App.tsx`, `icons.tsx`, `index.css`, `App.css`.
- `src-tauri/` — Rust backend: `src/lib.rs`, `src/main.rs`, `src/usb.rs` (adaptor), `src/card/` (parser). `tauri.conf.json` (window title `memcard-viewer`, macOS 13+), `capabilities/`.
- `src-tauri/target/`, `node_modules/`, `dist/` — build output, don't edit.
- Tests: `src/card/engine.test.ts` (vitest, node env); Rust unit tests alongside parser modules.
- Assets: `assets/plates/`, `design/` (vision mocks — IA/jobs reference, not visual law).

## Product constraints (from PRODUCT.md)

- PS1 only: 128 KB, 15 slots/blocks. No PS2, no other consoles.
- Formats: raw `.mcr` family, DexDrive `.gme`, VGS `.vgs`/`.mem`. No VMP/MCX.
- Adaptor `054C:02EA` **read only** in v1 — no hardware write. One physical slot; sidebar shows Slot 1 only when adaptor present.
- Save = unit of backup; 15-block card = unit of layout. Linked continuation blocks stay visible (`n of m`), including non-contiguous chains.
- Near-term loop is **auto-backup** of changed saves, not sync. Two-way sync (write-back) and cloud are later chapters — don't describe current behavior as sync.
- Don't invent testimonials, user counts, benchmarks, or adaptor compatibility beyond `054C:02EA`. Don't treat mock copy (e.g. "15 saves") as spec.

## Design constraints (from DESIGN.md)

- Canvas Cool Paper `#dde4ed`, frost/white panels, soft navy-tinted shadows. No charcoal slabs, CRT, or XMB.
- One blue `#1e71ef` for selection, occupancy-used, progress, primary action. No PlayStation red. Occupancy Gold is for focused-chain cells only, not warnings.
- Every open card renders **fifteen numbered tiles**; empties are dashed tiles, never omitted. Save icons 16×16, `pixelated`, on near-black well — never smoothed.
- Wordmark: PlayStation monogram + `memcard-viewer` in Encode Sans Expanded 18px/600, top-left only. No "PlayStation Memory Card Manager" lockup, no Sony marks on the card plate.
- Unbuilt actions stay visible in the same chrome, disabled + Quiet Slate with `title="{feature} is not available yet"` — don't hide them.
- Token values live in the `DESIGN.md` frontmatter; follow the named shadow/radius vocabulary there.

## Code conventions

- TypeScript `strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`. `tsc --noEmit` must pass (`npm run build` runs it).
- Frontend tests: colocated `*.test.ts` run under vitest (`test.environment: node`). Keep parser/engine tests in `src/card/`.
- Rust: edition 2021. New Tauri commands go in `lib.rs` with serde-derived types; USB/adaptor code stays in `usb.rs`.
- Don't wrap MemcardRex UI; the port is parser-level (`ps1card`, `PS3MemCardAdaptor`, `HardwareCardTransfer` semantics).
