# Canvas session prompt — logo "D · Two rooms"

To paste into a new Claude Desktop session (canvas). The prompt is self-contained: the session has no access to the
repository. You may also attach [directions/logo-directions.html](directions/logo-directions.html) so that it sees the
four starting proposals, but it is not necessary: the code of D is already in the prompt.

When the session delivers, the files go in `design/brand/final/` and the next step is the adoption in the app
(A-075).

````text
You are the designer of the Stemwise mark. In this session you refine the logo chosen by the author, direction
"D · Two rooms", into a mark ready for adoption: mark, macOS app icon, small mark for the app bar, favicon and
lockup with the name. Everything you need is in this message. Communicate with me in Italian; the texts inside the
boards that imitate the app stay in English, like the interface.

The work has two phases and a single pause.

PHASE 1 — Three variants within D
Propose three variants of direction D, clearly different from one another (not three retouches of the same shape): one
that evolves the starting proposal by correcting its flaws (section 5) and two that express the "two rooms" in another
way (section 6 has some hints, not mandatory). For each, show on the canvas:
- a name and two lines on the idea;
- the large mark;
- the macOS icon at 1024, 128, 32 and 16 px at real size, and in a Dock next to neutral icons;
- the mark at 20 px in the app's top bar, once in Mix and once in Master (section 4);
- the monochrome version;
- in three lines: what it tells, the risk, the closest existing brand.
Close with a comparison of the three at 128, 32 and 16 px. Then stop and ask me which variant I choose. It is the only
question you may ask me in the whole session.

PHASE 2 — Full refinement, without interruptions
After my choice, refine the variant up to the delivery of section 8, without stopping. If a point is ambiguous, decide
yourself in the way most consistent with this message and note the decision in the final README. Before delivering,
check the variant against the criteria of section 9 and fix what does not hold up.

────────────────────────────────────────

1. THE PRODUCT

Stemwise is a desktop app for macOS for mixing and mastering songs that are already recorded. It does not record and
does not compose: you start from the stems, the separate tracks of a song (vocals, drums, bass, guitars…), and you end
up with the master ready to publish.
- An AI assistant proposes, it does not drive: each of its choices is numbered, explained in one sentence and undoable
  in a single step. The user listens, corrects, decides. The model never receives the audio, only measurements.
- Two phases, two rooms. First the Mix (many tracks that become a song), then the Master (the whole song polished for
  release). They are two distinct workspaces, with separate histories, and are told apart at a glance by color: lime
  on aubergine for the Mix, lilac on indigo for the Master. The passage from Mix to Master is an explicit moment, a
  dialog that produces the mixdown; from the Master you can always go back to the Mix.
- User: someone who writes and records well but is not a sound engineer, and wants a credible result without learning
  a trade. Tone of the visual direction, called "Atelier": competent, calm, craftsmanlike. Never technical for show,
  never "magic", never aggressive.
- The name: stem (the separate tracks, but also the stem of a plant) + wise (sage, "who knows how").
- Start screen tagline: "Mix and master the songs you have already recorded. An assistant proposes,
  you listen and decide."

2. VISUAL PRINCIPLES OF THE INTERFACE

1. Two phases, two rooms: lime on aubergine for the Mix, lilac on indigo for the Master.
2. The assistant has a single color, sky blue: everything the AI wrote is sky blue. For this reason sky blue is not
   the product's color and does not enter the logo.
3. Color indicates sound or state, never decoration.
4. Readable by non-engineers.
5. Restful in the dark: no pure white, no pure black.
6. Every state also has a non-chromatic signal (shape, letter, outline, position), not only a color.

3. PALETTE, TYPOGRAPHY, SHAPES

Palette (OKLCH is the reference, the hex the sRGB conversion):
- Mix window background, aubergine: oklch(0.195 0.02 320) · #19121B
- Mix panels: oklch(0.24 0.022 320) · #241C26
- Wells, darker: oklch(0.16 0.018 320) · #110B12
- Lines: oklch(0.36 0.026 320) · #433946
- Master window background, indigo: oklch(0.19 0.03 290) · #141120
- Master panels: oklch(0.235 0.032 290) · #1E1B2C
- Main text: oklch(0.96 0.01 320) · #F5F0F6
- Muted text: oklch(0.75 0.018 320) · #B3AAB5
- Mix accent, lime: oklch(0.88 0.17 118) · #CEE654
- Master accent, lilac: oklch(0.81 0.12 300) · #CDAFFF
- AI sky blue (not to be used in the logo): oklch(0.85 0.09 215) · #85DDF3
In the logo: aubergine background (also with a light transition toward indigo), lime and lilac as the only two colors
of the mark, text #F5F0F6. Never pure black or white. Gradients only light and only for the depth of the macOS icon;
the flat mark must work without them.

Typography: titles and name in Bricolage Grotesque (weights 500 and 600), the rest of the interface in Figtree; both
SIL Open Font License 1.1, on Google Fonts. The "Stemwise" wordmark is Bricolage Grotesque 600, tracking about
-0.01em; you may adjust spacing and individual glyphs, but the family must stay recognizable.

Shapes: the app's icons are on a 16 × 16 grid, 1.5 px stroke, rounded caps and joins, no fill. Two real examples
(viewBox 0 0 16 16):
  loop:   <path d="M11 2.5l2 2-2 2M13 4.5H5.5a3 3 0 0 0-3 3M5 13.5l-2-2 2-2M3 11.5h7.5a3 3 0 0 0 3-3"/>
  group:  <rect x="2" y="3" width="12" height="4" rx="1"/><path d="M4.5 7v5.5h9M4.5 9.8h9"/>
The rest of the interface: soft radii (cards 14 px, buttons 10 px, clips 6 px), wide rounded fader caps, no aggressive
sharp corners, no dramatic shadows. The mark must look like part of the same family.

4. WHERE THE LOGO LIVES

- macOS app icon: 1024 × 1024 with the Apple icon grid (body about 824 px centered, continuous-corner squircle, margin
  for the shadow). It reads in the Dock at 128 px and in the Finder at 32 and 16 px.
- App top bar: 52 px high, on the window background. At the far left the mark at 20 × 20 px, then the project name
  (Bricolage Grotesque, for example "Paper Lanterns") and the "Mix | Master" selector. Today the mark in the bar is
  monochrome in the phase's accent color: lime in Mix (on #19121B), lilac in Master (on #141120).
- Start screen: mark at 48 px, below it "Stemwise" in Bricolage Grotesque 600 at 40 px and the tagline in Figtree
  14 px #B3AAB5; on the right the list of recent projects.
- Browser page for the AI connection: small mark + "Stemwise" in the header of a card.
- In the future: product website, README, installer disk image.

5. THE STARTING PROPOSAL AND ITS FLAWS

Direction D as delivered by a first round of exploration (viewBox 0 0 100 100):

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <path d="M16 25H43Q50 25 55 32L64 43Q68 48 76 48H84" fill="none" stroke="#CEE654" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M16 75H43Q50 75 55 68L64 57Q68 52 76 52H84" fill="none" stroke="#CDAFFF" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M75 39L86 50L75 61" fill="none" stroke="#CDAFFF" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
</svg>

In the icon it sits on a #19121B rectangle of 824 px with rx 218, scaled 6 times and centered.

What works: it is the only one of the four directions that tells something of its own about the product (the two
phases with their two colors); the rounded stroke is of the same family as the icons; lime and lilac on aubergine sit
well together.

What does not work, and must be solved:
1. It reads as the generic "merge" icon (two roads that join, the merging of two branches): it is an interface symbol
   already seen everywhere, not a mark.
2. It tells the wrong thing: the two lines arrive together, in parallel, like two equal inputs. The phases instead come
   one after the other: first you mix, then you master; and the Mix is "many parts → one song", the Master is
   "one song → polished".
3. The arrowhead is a cliché (export, forward, send) and at the meeting point it overlaps the end of the lime line: at
   32 px it becomes a blotch.
4. The mark occupies half the height of its square: at 16–32 px it remains a tiny arrow.
5. In monochrome, that is exactly in the top bar, the two rooms disappear and only the "merge" remains.
6. The lockup lettering is not Bricolage Grotesque but a geometric stroke drawing.
7. The icon background is a rectangle with a simple radius, not the continuous-corner squircle of macOS.

6. HINTS FOR THE VARIANTS (optional)

- The threshold: many thin lines (the stems) enter the first room and become one; that single line crosses a threshold
  and enters the second room, where it settles or closes. First lime, then lilac: the sequence reads from left to
  right.
- The floor plan: two rooms seen from above, as in an architect's plan (the Atelier), contiguous and with an opening
  between them. The first is divided into strips (the tracks), the second is full or has a single shape (the song). Even
  in monochrome there remain two rooms, because the geometry says so.
- The continuous passage: a single stroke that changes color halfway, lime then lilac, and along its path draws an S
  (Stemwise) or a shape that goes from "open" to "closed".
Whatever you choose, the two rooms must be recognizable even without color (principle 6).

An opportunity to evaluate for the top bar: if the mark is made of two separable parts, the app could light the room
of the current phase in its accent and dim the other (for example to 40 %), instead of coloring the whole mark. The
mark would then also say which phase you are in, by position as well as by color. Show it in the bar boards; if it does
not hold up at 20 px, the monochrome mark remains.

7. WHAT TO AVOID

- Arrows, "merge", "play", "fast forward" symbols, road signs.
- Audio and AI clichés: notes, headphones, microphones, vinyl records, sine waves, equalizer bars, brains, sparks,
  little stars, owls.
- Yin-yang, flags, abstract "duality" symbols with no link to the product.
- Resemblances to well-known brands: Logic Pro, GarageBand, Ableton, FL Studio, Reaper, Audacity, iZotope, Spotify,
  SoundCloud, Splice, LANDR; and to the Git and GitHub "merge" icons.
- Details under 1.5 px at 20 px, text in the icon, heavy 3D effects, photorealism, generated raster images: the logo is
  vector, drawn with paths.
- Fonts or resources with licenses other than SIL OFL, MIT, Apache-2.0, BSD or ISC.

8. DELIVERY (PHASE 2)

A downloadable "final" folder with these files. Clean SVGs: paths only, no live text, no embedded images, colors from
section 3.
- mark.svg — the color mark, square viewBox.
- mark-mono.svg — monochrome in currentColor.
- mark-20.svg — designed for 20 px (thicknesses and positions aligned to the 20 grid, optical variants if needed), in
  currentColor. If you adopt the lit-room idea (section 6), the two rooms are two groups
  <g id="room-mix"> and <g id="room-master"> in currentColor, so that the app can color and dim them from CSS.
- favicon-32.svg and favicon-16.svg — with the background, designed for those sizes.
- icon-1024.svg — the complete flat icon, with the continuous-corner squircle.
- icon-layers/ — the layers for Icon Composer: background.svg (the squircle with the background), and one file for each
  foreground element (for example room-mix.svg and room-master.svg), all at 1024 × 1024 and aligned.
- lockup.svg — mark + "Stemwise" in Bricolage Grotesque 600 converted to outlines, on a transparent background,
  meant for the aubergine; lockup-mono-light.svg (#F5F0F6) and lockup-mono-dark.svg (#19121B).
- mono-light.svg and mono-dark.svg — the mark in #F5F0F6 and in #19121B.
- presentation.html — a self-contained page (a single file, opens with a double click, fonts from Google Fonts) with:
  the mark and its construction grid; the icon at 1024, 512, 128, 32 and 16 px at real size and in the Dock; the top
  bar in Mix and in Master; the start screen with the lockup; the monochromes on light and on dark; clear space and
  minimum size; correct and incorrect uses (do not rotate, do not recolor with other hues, do not stretch, do not put
  on light colored backgrounds).
- README.md, in Italian: the idea in three lines, the construction (grid, thicknesses, radii), the colors, the clear
  space, the minimum sizes, the list of files, the decisions made in phase 2.
If you cannot convert Bricolage Grotesque to outlines in the canvas, deliver the lockup with live text and the font
declared, and say so clearly in the README: I will convert it myself. Do not replace it with an invented lettering.

9. VERIFICATION CRITERIA

1. Recognizability: is it remembered after seeing it once? At 16 px in the Finder is it still "the Stemwise one"?
2. Relevance: does it tell the two phases, one after the other, and not audio in general nor any merge?
3. Without color: in monochrome do two rooms remain?
4. Consistency: does it sit next to the app's icons and controls as if it were born with them?
5. Calm: is it suited to a tool in which you spend hours in the dark?
6. Durability: in two years will it still look right?
````
