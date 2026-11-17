# Brief — four logo proposals for Stemwise

Self-contained prompt for an AI design agent: it holds everything needed and does not require access to the app's
code. Paste it as it is.

---

## The task

Design **four logo proposals** for Stemwise, each in a **different visual direction** (section 5). Each proposal
includes the mark, the macOS app icon, the small version for the app bar and a lockup with the name. All four must stay
inside the app's visual language, described in sections 2–4: palette, typography and shapes are not to be reinvented.

Today the icon is made of **five vertical lime bars** of different heights (tallest in the middle) on a rounded
aubergine square. It works but it is anonymous: they are the bars of any equalizer, they could be the logo of any audio
or podcast app. The problem to solve is this: **a mark that is recognizably Stemwise**, not just "audio".

## 1. The product

Stemwise is a desktop app for macOS for **mixing and mastering songs that are already recorded**. It does not record
and does not compose: you start from the **stems**, the separate tracks of a song (vocals, drums, bass, guitars…), and
you end up with the master ready to publish.

- **An AI assistant proposes, it does not drive.** On request it proposes the mix parameters (levels, equalization,
  compression, reverbs…). Each of its choices is numbered, explained in one sentence and undoable in a single step.
  The user listens, corrects by hand or in words, and decides. The model never receives the audio, only numeric
  measurements: the audio stays on the computer.
- **Two phases, two rooms.** First the **Mix** (many tracks that become a song), then the **Master** (the whole song
  polished for release). The two phases have distinct workspaces and are told apart at a glance by color: **lime on
  aubergine** for the Mix, **lilac on indigo** for the Master.
- **User.** Someone who writes and records well but is not a sound engineer, and wants a credible result without
  learning a trade or paying subscriptions. Second, someone who knows how to mix and wants a quick, explained starting
  point.
- **Tone.** Competent, calm, craftsmanlike: the app's visual direction is called **"Atelier"**. Never technical for
  show, never "magic", never aggressive.
- **The name.** *Stem* (the separate tracks, but also the stem of a plant) + *wise* (sage, aware, "who knows how").
  Both meanings are legitimate material; the botanical pun is possible, not mandatory.
- **Tagline on the start screen:** "Mix and master the songs you have already recorded. An assistant proposes,
  you listen and decide." (the interface is in English).

## 2. Visual principles of the interface

1. **Two phases, two rooms**: lime on aubergine for the Mix, lilac on indigo for the Master.
2. **The assistant has a single color**, sky blue: everything the AI wrote or proposed is sky blue. For this reason sky
   blue is **not** the product's color.
3. **Color indicates sound or state, never decoration.** Each track has its own hue; the meters are mint green;
   automation is pink; the rest is neutral.
4. **Readable by non-engineers**: labels in plain words, no showy jargon.
5. **Restful in the dark**: no pure white, no pure black, comfortable contrasts for hours of work in a dim room.
6. **Every state also has a non-chromatic signal** (shape, letter, outline, position), not only a color.

## 3. Palette

Use these exact values (OKLCH is the reference; the hex is the sRGB conversion).

| Role | OKLCH | Hex |
| --- | --- | --- |
| Mix window background (aubergine) | `oklch(0.195 0.02 320)` | `#19121B` |
| Mix panels | `oklch(0.24 0.022 320)` | `#241C26` |
| Wells and charts (darker) | `oklch(0.16 0.018 320)` | `#110B12` |
| Lines and borders | `oklch(0.36 0.026 320)` | `#433946` |
| Master window background (indigo) | `oklch(0.19 0.03 290)` | `#141120` |
| Master panels | `oklch(0.235 0.032 290)` | `#1E1B2C` |
| Main text (never pure white) | `oklch(0.96 0.01 320)` | `#F5F0F6` |
| Muted text | `oklch(0.75 0.018 320)` | `#B3AAB5` |
| **Mix accent (lime)** | `oklch(0.88 0.17 118)` | `#CEE654` |
| Text on lime | `oklch(0.2 0.04 118)` | — (almost black green) |
| **Master accent (lilac)** | `oklch(0.81 0.12 300)` | `#CDAFFF` |
| AI assistant (sky blue) | `oklch(0.85 0.09 215)` | `#85DDF3` |
| Meter (mint) | `oklch(0.82 0.13 160)` | `#6FDEA7` |
| Automation (pink) | `oklch(0.8 0.12 345)` | `#F59ECF` |

The **12 track hues** (alternating lightness, so two neighboring tracks also differ in brightness):
`#F8A4A3` `#DE9871` `#ECC980` `#A2BC75` `#99E0BA` `#6BBBB6` `#98DAEA` `#79B1E0` `#BAC4F6` `#B99BDE` `#EFBBE3`
`#E48DA8`.

Color rules for the logo:

- **Background**: aubergine `#19121B` (or a very close tone of the same family). Never pure black, never pure white.
- **Main color of the mark**: lime `#CEE654`. Lilac `#CDAFFF` is allowed as a second color, especially in direction D.
- The other hues (sky blue, mint, pink, track hues) have a precise meaning in the app: use them only if a direction
  makes them an explicit point, never as decoration, and never sky blue as the dominant color.
- At most **three colors** in the icon, background included. Gradients only light and only for the depth of the macOS
  icon; the flat mark must work without them.

## 4. Typography and shapes

### Typography

- Titles and name: **Bricolage Grotesque** (weights 500 and 600). Everything else in the interface: **Figtree**. Both
  licensed under the SIL Open Font License 1.1, on Google Fonts.
- The **wordmark** "Stemwise" uses Bricolage Grotesque 600 with tracking close to `-0.01em`. You may adjust spacing
  and individual glyphs, but the family must stay recognizable. In the final files the text must be converted to
  outlines.
- No text inside the app icon (at most a single letter, in direction B).

### Shapes

The app has its own icon set: **16 × 16 grid, 1.5 px stroke, rounded caps and joins**
(`stroke-linecap: round; stroke-linejoin: round; fill: none`), filled shapes only for small details. Some real
examples, `viewBox="0 0 16 16"`:

```svg
<!-- loop -->
<path d="M11 2.5l2 2-2 2M13 4.5H5.5a3 3 0 0 0-3 3M5 13.5l-2-2 2-2M3 11.5h7.5a3 3 0 0 0 3-3"/>
<!-- faders: three lanes with filled caps -->
<path d="M4 2.5v11M8 2.5v11M12 2.5v11"/>
<rect x="2.5" y="8.5" width="3" height="2.5" rx=".8"/><rect x="6.5" y="4" width="3" height="2.5" rx=".8"/><rect x="10.5" y="7" width="3" height="2.5" rx=".8"/>
<!-- group: one track that gathers the others -->
<rect x="2" y="3" width="12" height="4" rx="1"/><path d="M4.5 7v5.5h9M4.5 9.8h9"/>
<!-- ear: listening -->
<path d="M2.5 9V8a5.5 5.5 0 0 1 11 0v1"/><rect x="2" y="9" width="3" height="4.5" rx="1"/><rect x="11" y="9" width="3" height="4.5" rx="1"/>
```

The current mark in the app bar, `viewBox="0 0 18 18"`, filled with the phase's accent color:

```svg
<rect x="1" y="7" width="3" height="4" rx="1.5"/><rect x="5.5" y="3" width="3" height="12" rx="1.5"/>
<rect x="10" y="5" width="3" height="8" rx="1.5"/><rect x="14.5" y="8" width="3" height="2" rx="1"/>
```

The rest of the interface: soft radii everywhere (cards and dialogs 14 px, buttons 10 px, fields 8 px, audio clips
6 px), wide rounded fader caps, knobs with an arc showing the value, audio clips with a name band in the track's full
color, tinted body and light waveform, automation curves made of points joined by segments. No aggressive sharp
corners, no dramatic shadows. The mark must look like part of the same family as these shapes, not a foreign object.

## 4bis. Where the logo appears in the app

- **Top bar** (52 px high, on the window background): at the far left the mark at **20 × 20 px**, monochrome in the
  phase's accent color (lime in Mix, lilac in Master), followed by the project name in Bricolage Grotesque and the
  "Mix | Master" phase selector.
- **Start screen**: on the left, vertically centered, the mark at **48 px**, below it the name "Stemwise" in
  Bricolage Grotesque at 40 px and the tagline in Figtree 14 px muted text; on the right the list of recent projects.
- **AI connection page** shown in the browser: small mark + "Stemwise" in the header of a card.
- In the future: a product website, the README, the installer disk image.

## 5. The four directions

Each starts from a different idea; all stay within the palette and shapes of sections 3–4. The indications are a
starting point: within each direction you are free, as long as the result is clearly different from the other three.

### A · Convergence — "many stems, one song"

The product's idea in one gesture: several parallel lines (the stems) that flow into a single one. **Stroke** language,
related to the icon set (rounded caps, constant thickness), like a routing diagram reduced to the essentials. The lines
can form an S, a fan or a gathering node. Optional: the starting lines take some of the track hues and the arriving one
is lime, *only* if it stays readable at 20 px (otherwise all in one color).
Avoid: a circuit diagram, a network or fintech logo.

### B · Monogram and wordmark — "the S of Stemwise"

Typographic direction: an **S** built from Bricolage Grotesque and transformed by an element from the app's world: the
curve of an automation with its points, the profile of a clip with its fade handles, a continuous waveform that draws
the letter. The full wordmark is crafted with the same care as the mark: it is the direction where the name matters
most.
Avoid: a generic startup S, or an already-seen wave-S. Look for a distinctive detail.

### C · The instrument's object — "the fader you touch"

A tactile pictogram taken from the app's controls: the **wide fader cap** on its lane, a knob with the value arc, or
the combination of the two. Soft, almost a physical object. The "wise" character can emerge in the detail: the fader
set at a precise point, not at the end of its travel (the right mix, not the loudest).
Avoid: the icon of a generic mixer or of a settings app.

### D · Two rooms — "first the mix, then the master"

The mark tells the **two phases**: two shapes that complete each other, one lime and one lilac, on the aubergine
background (or with a transition from aubergine to indigo). For example two arcs or two offset half-shapes, a shape that
goes from "many parts" to "one surface", a threshold between two environments. It is the only direction in which two
accents coexist as protagonists. The monochrome version for the bar must stay sensible even without the color contrast
(principle 6).
Avoid: a yin-yang symbol, a flag, an abstract "duality" logo with no link to audio.

## 6. Requirements for each proposal

Each proposal must work in **all** these uses:

1. **macOS app icon**, 1024 × 1024, with the squircle shape and the Apple icon grid (icon body about 824 px centered,
   continuous corners, margin for the shadow). It must read in the Dock at 128 px and in the Finder at 32 and 16 px.
   Prepare the design **in layers** (background + one or two foreground layers) so that it can be brought into
   Icon Composer for the glass effects of recent macOS versions; also deliver the flat version.
2. **Bar mark**, 20 × 20 px, monochrome in `currentColor` (in the app it becomes lime or lilac depending on the phase).
   It is the strictest use: at 20 px the mark must stay crisp, with no details under 1.5 px. If the full mark does not
   hold up, design a declared simplified variant.
3. **Horizontal lockup** mark + wordmark, on the aubergine background.
4. **Monochrome versions**: light (`#F5F0F6`) on a dark background and dark (`#19121B`) on a light background.
5. **Favicon** at 32 and 16 px.

## 7. What to avoid in all proposals

- Clichés: musical notes, headphones, microphones, vinyl records, full sine waves, equalizer bars (this is exactly the
  flaw of the current icon), brains, sparks, little stars or "magic" for the AI, owls for "wise".
- Resemblances to well-known audio and music brands: Logic Pro, GarageBand, Ableton, FL Studio, Reaper, Audacity,
  iZotope, Spotify, SoundCloud, Splice, LANDR. For each proposal state in one line the closest existing brand and why it
  is not confused with it.
- Details that disappear under 32 px, text in the icon, heavy 3D effects, photorealism.
- Fonts or resources with licenses other than SIL OFL, MIT, Apache-2.0, BSD or ISC. No raster images generated by image
  models as the final deliverable: the logo is vector, drawn with paths.

## 8. Delivery

1. **One HTML presentation page**, self-contained (a single file, opens with a double click, no server; fonts from
   Google Fonts), on the aubergine background and with the palette of section 3. One board per proposal (`A`, `B`, `C`,
   `D`) and a final comparison board. Each board shows:
   - the large mark, with one line on the idea and one on the construction (grid, proportions);
   - the macOS icon at 1024, 512, 128, 32 and 16 px **at real size**, and a Dock simulation next to other neutral
     icons;
   - the mark at 20 px inside a reproduction of the top bar described in 4bis, once in Mix (lime on `#19121B`) and once
     in Master (lilac on `#141120`), with an example project name next to it ("Paper Lanterns") and the "Mix | Master"
     selector;
   - the lockup on the start screen described in 4bis (mark at 48 px, "Stemwise" at 40 px, tagline);
   - the light and dark monochrome versions;
   - in a few lines: strengths, risks, closest existing brand.

   The comparison board puts the four icons side by side at 128, 32 and 16 px and the four marks in the bar.
2. **For each proposal, the SVG files**: `mark.svg` (full), `mark-20.svg` (bar version, `fill`/`stroke` in
   `currentColor`, square `viewBox`), `icon-1024.svg` (flat), the icon layers (`icon-layer-bg.svg`,
   `icon-layer-fg.svg`, optional second layer), `lockup.svg`, `mono-light.svg`, `mono-dark.svg`. Clean SVGs: paths only,
   no live text, no embedded images, colors taken from section 3.
3. **A short text** (in Italian) with the four ideas in one line each and the list of files.

## 9. Criteria the author will use to choose

1. **Recognizability**: is it remembered after seeing it once? At 16 px in the Finder is it still "the Stemwise one"?
2. **Relevance**: does it say something about *this* product (stems that become a song, two phases, the assistant that
   proposes, the craft) and not about audio in general?
3. **Consistency**: does it sit next to the app's icons and controls as if it were born with them?
4. **Calm**: is it suited to a tool in which you spend hours in the dark?
5. **Durability**: in two years will it still look right, or is it a fashion?

Before delivering, check each proposal against these five criteria and against section 7, and fix what does not hold
up. If a direction does not give a result up to standard, say so on its board instead of filling it.
