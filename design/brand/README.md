# Stemwise — brand mark

Work on the app logo: the mark, the macOS icon, the mark in the top bar and the lockup with the name. The decisions
are in [A-075 and A-076](../../docs/app/DECISIONS.md).

> **Rights reserved.** The Stemwise name and logo, and everything in this folder, are not covered by the repository's
> MIT license. What you may and may not do: [TRADEMARKS.md](../../TRADEMARKS.md).

## Status

| Step | Status | Where |
| --- | --- | --- |
| Brief for four directions | ✅ 2026-11-13 | [BRIEF-logo.md](BRIEF-logo.md) |
| Four directions (external agent) | ✅ 2026-11-13 | [directions/](directions/) |
| The author's choice: **D · Two rooms** | ✅ 2026-11-13 | A-075 |
| Refinement of D | ✅ 2026-11-14 | [final/](final/) (open `final/presentation.html`), A-076; regenerated with `build-final.py` |
| Adoption in the app (Tauri icons, mark in the bar, design boards) | ✅ 2026-11-14 | A-078 |

In the app the mark lives in `app/ui/src/shared/Icon.tsx` (`Mark`), in the boards in `mark()` of
`design/tools/lib.mjs`, with the colors in `design/components/components.css`; the icons in `app/src-tauri/icons/` are
generated from `final/icon-1024.svg`.

## The four directions

Drawn by an external agent from [BRIEF-logo.md](BRIEF-logo.md). They are an archive and a reference: they are not
part of the product.

| Direction | Idea | Folder |
| --- | --- | --- |
| A · Convergence | four stems come in distinct and gather into a single output | [directions/A-convergence](directions/A-convergence) |
| B · Monogram | an S built as an automation curve, with three points | [directions/B-monogram](directions/B-monogram) |
| C · The fader | a fader cap resting at the right spot, not at the end of travel | [directions/C-fader](directions/C-fader) |
| **D · Two rooms** (chosen) | two trajectories, lime (Mix) and lilac (Master), meeting in one output | [directions/D-two-rooms](directions/D-two-rooms) |

[directions/logo-directions.html](directions/logo-directions.html) shows the four proposals: icon from 1024 down to
16 px, Dock, top bar in Mix and Master, lockup on the welcome screen, monochrome versions and a final comparison. It
opens with a double click and loads its fonts from Google Fonts.

Each folder contains `mark.svg`, `mark-20.svg` (in `currentColor`), `icon-1024.svg`, `icon-layer-bg.svg`,
`icon-layer-fg.svg`, `lockup.svg`, `mono-light.svg` and `mono-dark.svg`.

Limits stated by the agent and observed here:

- the lettering of the lockups is a geometric stroke drawing, **not** Bricolage Grotesque 600: it must be redone;
- the icon background is a rectangle with a simple radius (`rx="218"`), not the continuous-corner squircle of macOS;
- the layers for Icon Composer are only split into two files, not assembled;
- in `C-fader/mono-dark.svg` the inner details have the same color as the cap and disappear (does not concern D).
