# Stemwise — "D · Two rooms" mark, refined

Delivery of the refinement of direction D (A-075, A-076). The page to open first is
[presentation.html](presentation.html): a single file, it opens with a double click and loads its fonts from Google
Fonts.

## The idea

The two trajectories of proposal D are the **Mix**: many parts that come closer and join, in lime.
Beyond a gap is the **Master**: a single solid shape, the finished song, in lilac.
First the lines, then the room: the sequence reads left to right, even without color.

## Construction

100 × 100 grid (`mark.svg`).

| Element | Measures |
| --- | --- |
| Mix lines | stroke 9, rounded ends and joins; they start at y = 20 and y = 80 from x = 9, and arrive at y = 45.5 and y = 54.5 at x = 53, where they touch without overlapping |
| Curve | the one from proposal D: horizontal run to x = 23, quadratic curve (control at x = 31), straight diagonal, quadratic curve (control at x = 45), horizontal run; continuous tangents, the diagonal takes the central 40 % between the two controls |
| Gap | 7.5 between the end of the lines (with the rounded end) and the room |
| Master room | solid rectangle x 65–95, y 37–63 (30 × 26), radius 10 |
| Footprint | x 4.5–95, y 15.5–84.5: centered in the square |

Optical versions, redrawn on the pixel grid:

| File | Grid | Stroke | Gap | Room |
| --- | --- | --- | --- | --- |
| `mark-20.svg` | 20 | 2 px, runs on whole rows (y 4 and 16) | 1.5 px | 6.25 × 6, radius 2 |
| `favicon-32.svg` | 32 | 2 px | 2 px | 7 × 6, radius 2.5 |
| `favicon-16.svg` | 16 | 1.5 px | 1.5 px | 3.75 × 4, radius 1.5 |

Icon: continuous-corner squircle (the Bézier approximation of Apple's continuous corners), 824 × 824 body centered in
the 1024 square, nominal radius 185.4; mark at scale 5.8 (stroke 52 px) centered on the body; soft shadow in the
margin (blur 14, offset 12, opacity 45 %); 2 px inner hairline at 7 % of the primary text color, to separate the edge
from the dark Dock.

## Colors

| Use | OKLCH | Hex |
| --- | --- | --- |
| Mix, lime (lines) | oklch(0.88 0.17 118) | `#CEE654` |
| Master, lilac (room) | oklch(0.81 0.12 300) | `#CDAFFF` |
| Background, aubergine | oklch(0.195 0.02 320) | `#19121B` |
| Lockup text, light monochrome | oklch(0.96 0.01 320) | `#F5F0F6` |
| Icon background gradient | from `#271E2A` (top left) to `#19121B` to `#171223` (bottom right, toward indigo) | |

The assistant's sky blue does not enter the mark. No pure white or black.

## Clear space and minimum sizes

- **Clear space**: x = half the height of the Master room (13/100 of the mark's side), measured from the visible
  shape. In the lockup the same x applies, measured on the mark.
- **Mark**: minimum 16 px. Below 24 px the optical drawings are used (`mark-20.svg`, `favicon-32.svg`,
  `favicon-16.svg`); from 24 px up, `mark.svg`.
- **Lockup**: minimum width 110 px. Smaller than that, only the mark.
- **Icon**: all macOS sizes from the same `icon-1024.svg`.

## Files

| File | Content |
| --- | --- |
| `mark.svg` | color mark, viewBox 100 × 100, groups `room-mix` and `room-master` |
| `mark-mono.svg` | mark in `currentColor` |
| `mark-20.svg` | mark drawn for 20 px, groups `room-mix` and `room-master` in `currentColor` |
| `mono-light.svg`, `mono-dark.svg` | mark in `#F5F0F6` and in `#19121B` |
| `favicon-32.svg`, `favicon-16.svg` | favicons with background, drawn for those sizes |
| `icon-1024.svg` | complete flat icon: squircle, gradient, shadow, mark |
| `icon-layers/background.svg` | the squircle with the background, 1024 × 1024 |
| `icon-layers/background-fullbleed.svg` | the same background full-bleed, for Icon Composer (see below) |
| `icon-layers/room-mix.svg`, `icon-layers/room-master.svg` | the two rooms, 1024 × 1024, aligned to the background |
| `lockup.svg` | mark + "Stemwise" as paths, for the aubergine |
| `lockup-mono-light.svg`, `lockup-mono-dark.svg` | lockup in `#F5F0F6` and in `#19121B` |
| `presentation.html` | the complete presentation |

All the SVGs contain only paths: no live text, no embedded images. They are regenerated with
`python3 design/brand/build-final.py` (requires `fonttools` and `uharfbuzz`, MIT and Apache-2.0 licenses, used only
to convert the font); the presentation is built by `design/brand/presentation.py`.

## Decisions made in pass 2

1. **Faithful refinement, not a new shape.** Pass 1 proposed three variants (Threshold, Plan, Ess); the author
   discarded them all and asked to refine the delivered proposal D. The refinement keeps the two trajectories with
   their curve (run, curve, diagonal, curve, run), the rounded stroke, the lime and lilac on aubergine, the
   left-to-right reading. The discarded variants were not kept.
2. **The two lines are both the Mix.** In the proposal one line was the Mix and the other the Master, and they
   arrived together (defect 2). Now they are two parts of the same Mix that rejoin; the Master comes after, and it is
   a single thing.
3. **The arrow becomes the Master room.** The arrowhead (defect 3) is replaced by a solid rounded rectangle,
   separated by a gap. It is solid so that in monochrome the two rooms stay different in shape (lines against
   surface, defect 5), and it is rounded with radius 10/30 to resemble a room and the app's cards more than a pill.
4. **The lines touch without merging.** They arrive at 45.5 and 54.5 with stroke 9: they touch at the center but
   remain two. There is no Y-shaped meeting point, which was the reason for the "merge" reading (defect 1).
5. **Proportions.** The room is 26 tall: more than the two lines side by side (18), so that the Master weighs as
   much as the Mix at a glance; with 18 it shrank to a dot at 16–32 px. The mark occupies two thirds of the square
   (defect 4).
6. **Top bar: lit room.** It holds up at 20 px, so it is adopted: the app colors the group of the current phase with
   its accent and brings the other to 40 % (in Mix the lime lines and the room at 40 %; in Master the lilac room and
   the lines at 40 %). To prevent overlapping parts from showing through, the 40 % must be rendered as a color mixed
   with the background (`color-mix(in srgb, <accent> 40%, <background>)`) and not as opacity; in the 20 px drawing the
   two rooms do not overlap anyway. The whole mark in the phase accent remains the fallback.
7. **Font.** The name is converted to paths from the same file the app includes
   (`@fontsource/bricolage-grotesque`, SemiBold 600, OFL-1.1), with HarfBuzz kerning and tracking −0.01 em; no glyph
   touched up.
8. **Icon Composer.** Icon Composer applies its own mask: for it the right background is
   `background-fullbleed.svg`, while `background.svg` (the squircle) serves for a classic `.icns` and as an alignment
   reference. The two rooms are separate layers, so Icon Composer can give each its own glass and depth.
9. **Shadow in the flat icon.** `icon-1024.svg` carries a soft shadow in the margin, like macOS icons up to Sequoia;
   the layers for Icon Composer have none, because the system adds it.
10. **Small icon.** There are no optical versions of the app icon at 16 and 32 px: on Retina Macs the Finder uses the
    double representation, and the one scaled down from the 1024 icon stays legible. Optical drawings exist for the
    mark without background (bar) and for the favicons.
11. **Clear space and minimums** as above: x = half a room, mark from 16 px, lockup from 110 px.
