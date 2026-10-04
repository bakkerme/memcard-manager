# Design pass — 2026-09-20

Reference: `design/design-mock-final.png`. Current evidence: `design-pass-desktop.png` (1536×1024), `design-pass-compact.png` (1100×760), and `design-pass-mobile.png` (390×844 viewport).

The independent finish review requested fixes to the monogram, font delivery and control weight, inspector title/actions, corner framing, and documentation. Its final follow-up and the documenter's final response were interrupted by agent usage limits. The parent completed the scoped verdict and reconciled the documents directly; this is not a second independent approval.

## Verdict

- Resolved: accurate Simple Icons monogram replaces the approximate P/bar geometry.
- Resolved: Encode Sans Expanded 600 and its OFL license are bundled locally. Toolbar weight is reduced to 400; the wordmark retains the required 18px/600 treatment.
- Resolved: the wide inspector title fits beside its icon, and 72px bottom padding restores the action group's spacing. Compact title wrapping remains intentional.
- Resolved: faint corner geometry restores the reference's framing.
- Resolved: DESIGN.md and its sidecar describe the current implementation. This report and the named captures record current evidence separately from historical build gates.

## Remaining

No open items in the scored visual fix list. Historical surface/build artifacts retain pre-existing drift (including obsolete brand wording); they are not evidence for this pass and were not silently rewritten. The pass is a functional adaptation of the supplied mock, not a pixel-identical reproduction. Real frame artwork, source counts, physical block order, one adaptor slot, and disabled future actions intentionally differ from its illustrative content.

Disposition: ship for the scored fixes.

## Verification

- Production build and TypeScript checks pass.
- 14 Rust tests and 1 frontend test pass; 2 hardware-only tests remain ignored.
- Mechanical design detector: no findings.
- Browser checks: fifteen tiles, linked-save selection, additive selection (four selected blocks for two saves), no horizontal overflow at 390px or 1100px, and rendered error/read states.
- Physical adaptor operations were not exercised.
