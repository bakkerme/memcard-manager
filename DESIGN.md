---
name: memcard-viewer
description: A cool paper desktop for fifteen physical memory card blocks.
colors:
  bg: "#dde4ed"
  bg-accent: "#e7edf4"
  panel: "#e1e8f1"
  panel-strong: "#e7edf4"
  line: "#bdcddd"
  line-strong: "#c3cedc"
  text: "#18375d"
  text-muted: "#4d6788"
  text-faint: "#7488a2"
  accent: "#1e71ef"
  accent-soft: "#cddff7"
  good: "#1aa65f"
  occupancy-gold: "#e8b931"
  danger: "#d4536c"
  danger-soft: "#fdecef"
  pixel-well: "#0f141c"
  placeholder: "#edf2f8"
  white: "#fff"
  accent-hover: "#1964d8"
typography:
  display:
    fontFamily: "\"Encode Sans Expanded\", \"Encode Sans\", \"Helvetica Neue\", Helvetica, sans-serif"
    fontSize: "18px"
    fontWeight: 600
    letterSpacing: "-0.03em"
  title:
    fontFamily: "\"Helvetica Neue\", Helvetica, Arial, sans-serif"
    fontSize: "18px"
    fontWeight: 600
  inspector-title:
    fontFamily: "\"Helvetica Neue\", Helvetica, Arial, sans-serif"
    fontSize: "16px"
    fontWeight: 500
    lineHeight: 1.3
  body:
    fontFamily: "\"Helvetica Neue\", Helvetica, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.3
  navigation:
    fontFamily: "\"Helvetica Neue\", Helvetica, Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 400
  label:
    fontFamily: "\"Helvetica Neue\", Helvetica, Arial, sans-serif"
    fontSize: "12px"
    fontWeight: 400
  button:
    fontFamily: "\"Helvetica Neue\", Helvetica, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1
rounded:
  xl: "12px"
  lg: "10px"
  md: "8px"
  sm: "8px"
  well: "6px"
  control: "6px"
  navigation: "7px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  gutter: "14px"
  lg: "16px"
  xl: "20px"
  section: "24px"
components:
  button-primary:
    textColor: "{colors.white}"
    rounded: "{rounded.control}"
    typography: "{typography.button}"
    padding: "8px 14px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
    textColor: "{colors.white}"
  button-secondary:
    backgroundColor: "{colors.panel-strong}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "8px 14px"
  button-danger:
    textColor: "{colors.danger}"
    rounded: "{rounded.control}"
    padding: "8px"
  navigation-selected:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.text}"
    rounded: "{rounded.navigation}"
    padding: "10px 6px"
  chip-block:
    backgroundColor: "{colors.panel-strong}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "8px 14px"
  card-tile:
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
  workspace:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.xl}"
---
# Design System: memcard-viewer

## Overview

**Creative North Star: "The Cool Paper Desk"**

A light desktop manager built around the physical geometry of a PS1 memory card. Cool paper, frost surfaces, fine navy rules, and small inset highlights make the controls feel tangible without overpowering the save icons. The existing world remains The Cool Paper Desk.

The final mock supplies the grouped source sidebar, inset card summary, five-column block gallery, adjacent inspector, and quiet footer. The implementation retains that material and spacing hierarchy while following PRODUCT.md for real card contents and available operations. System UI type keeps metadata compact; Encode Sans Expanded identifies the product only in the top-left wordmark.

**Key Characteristics:**

- Cool paper and layered frost materials.
- Fifteen numbered blocks with crisp pixel wells.
- One action blue with small semantic status accents.
- Compact metadata and restrained, tactile controls.

Evidence: [final mock](design/design-mock-final.png), [desktop](.impeccable/review/design-pass-desktop.png), [compact](.impeccable/review/design-pass-compact.png), and [mobile](.impeccable/review/design-pass-mobile.png). Tokens and states are extracted from `src/index.css`, `src/App.css`, `src/App.tsx`, and `src/components/PixelIcon.tsx`; screenshots illustrate the result, while the final source cascade owns values.

## Colors

The palette is blue-gray throughout, with a saturated blue reserved for interaction and occupancy.

### Primary

- **Manager Blue** (`accent`) carries primary actions, selected outlines, focus, occupied cells, and progress. **Blue Wash** (`accent-soft`) fills selected navigation and informational surfaces.
- The primary button uses a light-to-deep blue gradient; its hover becomes the solid `accent-hover`. This material is carried in the sidecar, because gradients are not color primitives.

### Secondary

- **Occupancy Gold** is confined to focused-chain occupancy cells.
- **Connected Green** marks the adaptor’s connected lamp. **Danger Rose** and its pale wash identify errors and destructive actions, not brand identity.

### Neutral

- **Cool Paper** (`bg`) is the canvas, with **Mist Wash** (`bg-accent`) and **Frost Panel** (`panel`, `panel-strong`) separating surfaces.
- **Ink Navy**, **Slate Caption**, and **Quiet Slate** (`text`, `text-muted`, `text-faint`) establish text hierarchy. Quiet Slate also signals unavailable controls; it is not a substitute for readable active body copy.
- **Pale Rule** and **Steel Rule** (`line`, `line-strong`) define fine boundaries. Near-black is restricted to the pixel well.

**The Blue Selection Rule.** Selection, chain focus, occupied blocks, progress, and the primary action share Manager Blue. Occupancy Gold identifies only the focused chain in the occupancy strip.

## Typography

**Display Font:** Encode Sans Expanded, Encode Sans, Helvetica Neue, Helvetica, sans-serif.
**Body Font:** Helvetica Neue, Helvetica, Arial, sans-serif.

Encode Sans Expanded 600 is bundled locally in `src/assets/fonts/` with its OFL license; the app does not depend on a remote font request. The monogram uses the Simple Icons CC0 silhouette, attributed in `src/icons.tsx`.

The wordmark supplies the expanded letterforms; task text remains compact and familiar. There is no large editorial display role or independent monospace family. Block numbers and counts use tabular numerals.

### Hierarchy

- **Display:** the wordmark; compact layouts reduce it to 16px at 1050px.
- **Title:** the card name; reduced to 16px at 1350px.
- **Inspector title:** uses 16px medium weight; short titles fit beside the icon on wide screens and wrap naturally in compact layouts.
- **Body:** save titles, with 12px metadata and 14px navigation names. Tile titles use 13px on wide layouts and 12px on compact layouts, wrap without a line limit, and never use ellipses; compact block captions use 11px. Product codes appear in the inspector only.
- **Label:** supporting occupancy and save metadata. Sidebar category labels identify actual navigation groups; inspector metadata labels identify data fields. These do not establish a decorative eyebrow style.

**The Compact Wordmark Rule.** Reserve Encode Sans Expanded for the top-left memcard-viewer wordmark; use the body stack for task headings and metadata.

## Layout

The desktop shell uses a sidebar of `clamp(210px, 20.7vw, 318px)` beside the flexible workspace, with a 12px column gap. Its outer inset is 16px 14px 10px; header, content, and footer rows are a content-sized minimum of 56px, flexible, and 40px. The header contains the wordmark and Import save, Backup, and Sync actions. Its action group wraps when needed. Card loading lives in the scrolling source sidebar. macOS reserves a 32px native drag strip above the header. The source sidebar, summary, grid, and inspector repeat the mock’s nested panel hierarchy. Above 760px the shell fits 100dvh with no content-driven minimum height; the source sidebar fills its workspace row and scrolls internally when needed. At 760px and below the page becomes content-height and source navigation is capped at 320px with internal scrolling.

The summary is inset 26px 18px 22px with 12px 20px internal padding. The workspace body has 18px horizontal inset, 16px bottom inset, and a 16px gap between the gallery and an inspector sized `minmax(250px, 31%)`. The gallery is five columns by three rows, with 14px row gaps, 8px column gaps, and content-sized row minimums. Tile contents use 12px 8px 14px padding. Inspector padding is 22px 20px 72px with 24px section gaps. These measured intervals, rather than a fabricated geometric scale, describe the shipped rhythm.

At 1350px, the source sidebar is 210px, header row a content-sized minimum of 48px, and inspector 240px. The summary uses 8px 12px margins, 8px 12px padding, and a 72px minimum height. The workspace uses 12px side and bottom insets with a 12px gallery-to-inspector gap. Tiles have 12px top and 8px side/bottom padding, 6px content gaps, a 150px minimum height, and 48px pixel wells. Slot numbers sit beside the icon rather than taking a separate row. Per-slot menus and selection lamps are omitted; the blue outline indicates selection. Compact navigation names use 13px with tighter section spacing. Inspector padding is 12px, section gaps are 16px, and its pixel well is 48px. Rows grow for full titles; the workspace body scrolls when the contents exceed available height. The default window remains 1100 × 760. At 1050px, the wordmark remains on one line and cannot shrink; compact buttons use 12px text and 8px 10px padding. The sidebar remains 210px, the inspector moves below the board into two columns, occupancy moves to a second summary row, while the shell remains window-height with independently scrolling source navigation and workspace content. At 760px, the workspace precedes a wrapping source sidebar, the board has three columns, and the inspector stacks. At 400px, the board has two columns. Every layout retains all fifteen physical positions.

The implementation deliberately preserves physical order instead of the mock’s rearranged chain, displays one hardware slot only when present, and derives counts from real data. Virtual Cards lists every loaded file or card backup by name, with a separate X button to close each card. Opening a backup again focuses its existing row. Open card and New card from selection sit below these rows. Unavailable Import save, Backup, and Sync remain in the top bar; Backup and Sync are disabled while a collection source is selected. The header has no connection pill. An absent adaptor is an ordinary sidebar state; attachment automatically reads Slot 1, and its refresh control explicitly rereads it. Auto-backup and cloud remain visible future functions. Sync uses secondary button chrome; completion feedback uses the shared informational banner.

## Elevation & Depth

Depth comes from cool translucent gradients, fine borders, inset white highlights, and very shallow navy shadows. Panels are material layers rather than white cards floating high above the canvas. Faint, rotated corner frames sit behind the shell without competing with controls.

### Shadow Vocabulary

- **Panel** (`--shadow`): `inset 0 1px 0 #ffffff99, 0 8px 24px #1c304e06`; workspace, sidebar, menus.
- **Tile** (`--shadow-tile`): `inset 0 1px 0 #ffffffb3, 0 2px 4px #1c304e0a`; summary, save tiles, inspector, and block chips.
- **Pixel Well:** `0 0 0 2px #f5f8fb99, 0 2px 5px #1c304e26`; the frame surrounding the pixel image.
- **Primary Action:** `inset 0 1px 2px #ffffff99, 0 3px 8px #1e71ef29`; the blue compose action.

**The Soft Depth Rule.** Use the named inset-highlight and diffuse navy shadow vocabulary; do not introduce hard offset shadows.

## Shapes

The named radius vocabulary is `xl` for the outer workspace, `lg` for the sidebar and menus, `md` for tiles, summary, inspector and chips, and `well` for pixel frames. `sm` currently aliases `md`; retain the source value rather than inventing another step. Controls use the `control` radius, navigation uses `navigation`, and small occupancy cells use 4px corners. Empty tiles use dashed boundaries; selected and focused-chain tiles use the blue border plus a one-pixel outer ring.

## Components

### Buttons

Tactile, compact controls with restrained highlights. Standard buttons use the secondary surface, a fine line border, and a minimum height of 35px. Primary buttons use `linear-gradient(#378bff, #1267e6)` and white text. Hover on secondary buttons changes text and border toward blue; primary hover uses `accent-hover`. Disabled secondary actions use Quiet Slate and lose shadows. The disabled primary retains its pale blue treatment and reduced opacity. Inspector controls have a 40px minimum height; its unavailable delete action keeps quiet chrome.

Global keyboard focus is a two-pixel blue outline with a two-pixel offset; tile hit targets bring the outline inward by three pixels. State transitions run for 160ms with ease. The searching lamp pulses over 1.2s; reduced-motion preference removes transitions and animations.

### Chips

Linked-block chips are small raised frost rectangles with tabular numbers, a blue status dot, and line borders. They show chain order in the inspector, including non-contiguous physical positions. Arrow separators express the actual linkage.

### Cards / Containers

The workspace encloses the summary, board, and inspector. The summary uses a translucent cool gradient, a neutral unbranded card plate, and fifteen numbered occupancy cells. Used cells are blue; the complete focused chain becomes gold. Counts distinguish saves from occupied blocks.

### Navigation

Three source groups share line icons, compact names, secondary context, count badges, and thin separators. The selected row combines Blue Wash with a three-pixel inset blue edge. Hover brightens enabled rows. Local backups is an enabled source row with an actual save count; unavailable cloud and automatic backup actions retain disabled semantics and availability titles.

### Fifteen-block gallery

Each tile has its physical number, pixel well, full wrapping title and block count. The native save frame is 16×16; tile wells display at 80px on wide screens and 48px in compact layouts. Inspector wells display at 104px wide and 48px compact. The near-black well and `image-rendering: pixelated` preserve the frame’s edges. Continuations repeat the save image, reserve title space without duplicating visible text, and show n of m. Selection and focused-chain outlines span every related tile. Empty positions remain numbered, dashed, and visible. Deleted saves are dimmed.

The development screenshot fixture contains synthetic icons. They demonstrate pixel rendering and geometry only; production icons must come from parsed card frames.

### Local save library

Local saves shares the workspace, summary, pixel tiles, blue selection, and inspector vocabulary. A compact heading names the library; the configured folder and its controls live in Settings. A compact catalog shows one tile per save, with no physical slot numbers, continuation tiles, occupancy strip, or artificial empty slots. Its flexible grid fills available width with columns of at least 125px (110px at the mobile breakpoint), 8px gaps, and 160px tile minimum height. Each tile carries the full save title, game product code in Slate Caption, and block count.

The catalog toolbar keeps search, Game and Sort selects, result count, and Clear filters above the scrolling results. Search matches title, product code, identifier, region, and relative filename; Game filters by product code, including unknown games. Sort offers Save title, Game code, and Latest capture (newest known snapshot time). Results use 24-save pages with Previous / Next controls and visible range and page counts below the scrolling area. Changing search, game, or sort returns to the first page and scrolls results to the top. Selection and snapshot inspection remain available across pages and filters. A no-match state offers recovery through the filters.

Desktop catalog results and inspector scroll independently within the window-height workspace; the catalog toolbar and pagination remain visible. The inspector uses a flexible column of at least 200px. At 760px and below it stacks beneath the catalog, and each scrolling region is capped at 60dvh.

The inspector shows region, product, identifier, relative filename, and save size, with a Reveal in Finder action. File paths wrap anywhere. Empty and unconfigured states use the incumbent empty-board text treatment; unreadable files appear in an expandable informational banner, and directory failures use the error banner. Card actions stay disabled while this source is selected. Unconfigured empty states link to Settings; folder controls there remain visible but disabled in the web preview.

Save history appears below the inspector metadata, separated by a one-pixel Line border and 16px top padding. A labelled native select lists the latest dump and dated snapshots with short content fingerprints. Selecting a snapshot changes the displayed metadata, icon, and reveal destination. Capture sources use a collapsed disclosure; copy explains that versions sharing a game filename may represent different playthroughs. No physical card ID is displayed. The select uses the frost panel, well radius, 12px body type, and existing focus treatment.

### Collection settings

Settings stays visible at the bottom of the sidebar, beneath its independently scrolling source groups, and preserves loaded card sessions. Its workspace uses the compact collection heading and two plain sections: Collection folder and Refresh collection. The folder path wraps inside one frost field; Choose / Change folder and Reveal in Finder sit below it. Folder migration retains the existing copy-and-preserve behavior. Refresh collection remains available here for external file changes; opening Local saves or Card backups refreshes their contents automatically. Those collection views have compact headings with no folder path, folder picker, or refresh toolbar. Card actions stay disabled while Settings is selected.

Backup reuses the persisted collection folder, including one selected in the earlier save-backup flow. Its form explains that card images go automatically into `card-backups/` within that folder. Only users with no saved folder see the first-use picker; an unavailable folder or migration conflict shows an error instead of silently changing the destination.

### Operation banners

Backup and Sync results appear above the gallery in a Blue Wash banner, with Ink Navy copy and a blue Reveal in Finder text action. The banner uses a 12px radius, 10px 12px padding, a 12px gap, and wrapping content. Sync reports written, unchanged, and new snapshot counts alongside the destination path. Errors use the same geometry with the Danger Wash background and deep rose text (`#8d2740`); live status and alert semantics announce results to assistive technology.

Sync retains the standard secondary button and refresh icon. Its busy label is “Syncing…”, and card actions are disabled during the operation. The web preview keeps Sync visible but disabled with “Sync requires the desktop app”. The native directory picker belongs to the operating system rather than a custom app dialog.

### Read progress dialog

A compact centered native modal shows only a blue determinate bar based on received frames. Accessible labels identify the read and expose its percentage without visible copy. Its frost panel uses the xl radius, the progress track uses the well radius, and a quiet navy backdrop separates it from the workspace. The dialog holds keyboard focus and closes when the read succeeds or fails. The bar uses a 160ms linear transform transition and respects reduced motion. Failures use the error banner with retry guidance.

## Do's and Don'ts

### Do:

- **Do** keep all fifteen numbered tiles in physical slot order, including empty tiles and non-contiguous continuation blocks.
- **Do** retain the same save icon and an n of m caption on continuation tiles.
- **Do** render real 16×16 save frames with pixelated scaling in the near-black well.
- **Do** use actual save and occupied-block counts; a multi-block save is one save.
- **Do** keep unavailable actions visible, disabled, and titled “{feature} is not available yet”.
- **Do** preserve the top-left PlayStation monogram beside memcard-viewer as the only brand lockup.

### Don't:

- **Don’t** use charcoal slabs, CRT or XMB styling, or PlayStation red as the action accent.
- **Don’t** copy the mock’s second hardware slot, fictitious collection counts, or two-way sync claims; hardware exposes one slot and auto-backup remains unavailable. Manual Sync is card-to-directory only.
- **Don’t** add Sony branding to the card plate or a second PlayStation Memory Card Manager lockup.
- **Don’t** treat synthetic development fixture icons as production artwork or substitute mock illustrations for parsed save icons.
- **Don’t** hide empty or continuation blocks, rearrange physical slots to group a chain, or smooth pixel art.
