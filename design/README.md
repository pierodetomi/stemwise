# Stemwise — design

Delivery of the design session (FOUNDATION §7 and §10), November 10, 2026. Direction chosen by the author:
**E · Atelier**.

During implementation this design is normative for **look and layout**; `docs/FOUNDATION.md` is normative for
**behavior**. Where a state is missing, derive it from existing tokens and components and note it in
`docs/app/DECISIONS.md`.

## Contents

| File | What it contains |
| --- | --- |
| [tokens.css](tokens.css) | All the tokens as CSS variables: colors, typography, spacing, radii, shadows, durations and curves, track palette, accents of the two phases, control and shell measures, gesture thresholds. |
| [components/components.css](components/components.css) | The styles of every component, with the state classes. Uses tokens only. |
| [components/index.html](components/index.html) | The component sheet with all states (boards C1–C6). |
| [screens/screens.css](screens/screens.css) | Board frame and screen-level layouts. Uses tokens only. |
| `screens/NN-*.html` | One page for each screen of FOUNDATION §6; each page contains one board per state. |
| `tools/` | The board generator (sample data, waveforms, shared parts). Not part of the product. |
| [brand/](brand/README.md) | The logo work: brief, the four proposed directions, the choice (D · Two rooms, A-075) and the prompt of the canvas session that refines it. |

The boards are **static HTML**: they open in the browser with a double click, with no server and no script. Each board
is an app window at 1440×900 (1280×720 where indicated), with a label (`3a`, `7c`…) and a line saying what it shows.

To regenerate them after a change to the generator (needs Node 20 or newer):

```bash
node design/tools/build.mjs
```

`design/tools/shot.sh <page.html> <output.png> [width] [height]` produces an image of a page with headless Chrome; it
is the tool used to check the boards.

## Chosen direction: E · Atelier

A combination the author asked for after seeing the four initial directions:

- **structure** of direction A: timeline at the top, mixer below, inspector and assistant as two columns on the right;
- **palette and controls** of direction D: aubergine background, lime for the Mix, lilac for the Master, rounded cards,
  wide fader caps, plain-language labels;
- **numbered notes** of direction B: the same number on the track header, in the inspector and in the assistant's list
  of notes;
- **clips** of direction B brought to the classic region shape: name band in the solid track color, tinted body, light
  waveform.

The five explored directions remain in the session canvas
(<https://claude.ai/artifact/KU16jqpeLSrN9Jpfrb29gp>, private to the author). They are not needed for implementation.

## Principles

1. **Two phases, two rooms.** Mix is lime on aubergine, Master is lilac on indigo; the layout changes too (many lanes
   versus a single waveform and a chain). The phase selector is always at the top left and always shows both phases.
   The switch is an explicit dialog (boards 8a–8c).
2. **The assistant proposes, it does not lead.** Each of its choices has a number, a sentence and a diff. Everything it
   has written or that is off limits to it uses a single color, the sky blue `--ai`: notes, automation lanes written
   by it, locks, call progress. There is no chat transcript: the request in words is a field inside the tool, with
   scope and estimated cost next to it.
3. **Color indicates sound or state, never decoration.** Track tints live on clip, strip, header and inspector; meters
   are mint green; the rest is neutral.
4. **Readable by non-engineers.** Labels in words ("Mud", "Presence", "evens out 3 dB"), units always written out,
   advanced controls present but ordered following the signal chain.
5. **Restful in the dark.** No solid white and no solid black; text at least 5.4:1 on the lightest background;
   playhead and meters readable from the corner of the eye because they are the brightest elements of their box.
6. **Every state also has a non-chromatic signal**: icon, letter, outline or position.

## Board index

| Screen of §6 | File | Boards |
| --- | --- | --- |
| 1 · Welcome | [screens/01-welcome.html](screens/01-welcome.html) | 1a recents, new, open · 1b first launch with an invitation to set up the AI (skippable, with offline demo) · with P-020 both carry the `Take the tour` link |
| 2 · New project / Import | [screens/02-new-project-import.html](screens/02-new-project-import.html) | 2a empty · 2b files dragged over · 2c list with format, sample rate, channels, duration, role; sample-rate warning; brief; references · 2d choice on the different sample rate |
| 3 · Mix workspace | [screens/03-mix-workspace.html](screens/03-mix-workspace.html) | 3a main screen · 3b history panel · 3c snapshots and A/B · 3d minimum window 1280×720 · with P-013: 3e mixer Overview · 3f large project with short lanes · 3g the same in Overview; all boards carry the marks of the chain, the sends and the automation · With P-011: under the request field, the "Allow structural changes" checkbox (new returns and groups). With P-018: two checkboxes under the request field, "structural changes" and "creative ideas". |
| 4 · Proposal in review | [screens/04-mix-proposal.html](screens/04-mix-proposal.html) | 4a summary, evaluation, tracks with diff and rationale, A/B, apply all / selected / discard · 4b partial application With P-011: 4a and 4b first show the returns and groups the plan creates ("New in this proposal"), each with its own checkbox, type, reason and who uses it; in 4b one is excluded by hand and one is not created because fewer than two of its tracks remain selected. · with P-018: 4c the creative ideas (checkbox off by default, new blocks in the track diff) · 4d listening to a single idea |
| 5 · AI at work | [screens/05-mix-ai-working.html](screens/05-mix-ai-working.html) | 5a progress by phase and by pass, cancel · 5b cancelled |
| 6 · Editing | [screens/06-editing.html](screens/06-editing.html) | 6a multiple selection · 6b dragging with snap · 6c cut and trim · 6d fades, clip gain, muted clips · 6e clip and track context menus · 6f automation tool and section editing |
| 7 · Returns & routing | [screens/07-returns-routing.html](screens/07-returns-routing.html) | 7a reverb return with algorithm selector (plate, room, hall), decay, size, pre-delay, damping, modulation and filters · 7b delay return and creating a return · 7c output selector and nested groups in timeline and mixer · 7d sidechain source and parameter to automate · 7e parallel compression (compressor Mix below 100 %) · with P-013: 7f who sends to a return · 7g where a track goes · 7h who listens to a track in sidechain |
| 8 · Bounce to Master | [screens/08-bounce-to-master.html](screens/08-bounce-to-master.html) | 8a confirmation · 8b progress · 8c result with the mixdown measurements |
| 9 · Master workspace | [screens/09-master-workspace.html](screens/09-master-workspace.html) | 9a main screen · 9b mixdown out of date · 9c master proposal in review |
| 9 · Master workspace, preparation | [screens/09-master-preparing.html](screens/09-master-preparing.html) | 9d preparation after a bounce · 9e reopened project, saved measurements, no references · 9f back to Mix during preparation · 9g ready while in Mix · 9h unreadable reference · 9i Master opened with a reference excluded · 9j unreadable mixdown · 9k new measurement after a change (no overlay) · 9v new target (overlay) |
| 9 · Master workspace, listening level and target warning | [screens/09-master-listening.html](screens/09-master-listening.html) | 9l real level (default) · 9m same loudness · 9n same loudness with anti-clip lowering · 9o master proposal loudness-matched · 9p no references · 9q target much lower than the reference · 9r multiple references · 9s target menu · 9t after "Keep" · 9u target much higher than the reference |
| 10 · Export | [screens/10-export.html](screens/10-export.html) | 10a formats, sample rate, bits, dither, metadata with ISRC and cover art, folder · 10b progress · 10c report · 10d export with the target warning · 10e report after an export with the discrepancy |
| 11 · Settings | [screens/11-settings.html](screens/11-settings.html) | 11a Audio · 11b AI · 11c Appearance · 11d Shortcuts |
| 12 · AI usage | [screens/12-ai-usage.html](screens/12-ai-usage.html) | 12a summary by type and by model, history with filters, export menu · 12b readable detail · 12c raw JSON · 12d empty · 12e spending cap exceeded |
| 13 · Cross-cutting states | [screens/13-states.html](screens/13-states.html) | 13a empty project · 13b small empty states · 13c loading · 13d missing files and disconnected device · 13e missing key · 13f no network · 13g destructive confirmations and crash recovery · 13h notifications |
| 14 · Assistant setup (P-015, A-071) | [screens/14-assistant-setup.html](screens/14-assistant-setup.html) | 14a connection with OpenRouter · 14b waiting for the browser · 14c page shown in the browser · 14d credit and spending cap · 14e ready · 14f not approved · 14g key pasted by hand, in three steps · 14h key entered · 14i credit exhausted · 14j key limit reached · 14k key no longer valid; updated 1b, 11b and 13e, which open the dialog |
| 15 · Master chain (P-017, A-079, A-081; approved and implemented in F14, A-082) | [screens/15-master-chain.html](screens/15-master-chain.html) | The chain as a rack next to the editor, with repeatable blocks: 15a rack, clipper open with "Take off up to" · 15b reordering · 15i adding a block · 15j block menu (move, bypass, reset, remove) · 15c compressor with Mix and detector filter · 15d multiband, from two to six bands · 15h the same with five · 15e per-section offsets · 15f proposal with measurements before → after, order, new blocks and sections · 15g the target asks too much of this mix. Replaces the module row of 9a–9c; the rest of screen 9 stays. |
| 15 · Master chain, exploration (A-080; choice: R, A-081) | [screens/15-master-chain-directions.html](screens/15-master-chain-directions.html) | The same master with eleven blocks in three layouts: G grid with an "Add module" tile · R vertical rack next to the editor · P path with the peak bar above the body after each block. Kept as a record: boards 15 are redrawn in the rack. |
| 16 · Track chain (P-018, A-086; approved by the author on 2026-11-14) | [screens/16-track-chain.html](screens/16-track-chain.html) | 16a the chain in the inspector, De-esser open · 16b block menu by family · 16c reordering · 16d Gate · 16e dynamic EQ with sidechain · 16f Transient · 16g saturation characters · 16h Bass enhancer · 16i Exciter · 16j Width · 16k Modulation · 16l Tremolo · 16m Harmonizer · 16n Doubler · 16o resonant filter that opens between sections · 16p chain of a return: ducking · 16q gated reverb · 16r ping-pong and tape delay · 16s shimmer reverb · 16t marks with long chains · 16u Overview with long chains |
| 17 · Explain this mix (P-019, A-092; approved by the author on 2026-11-15, implemented in F18, A-093) | [screens/17-mix-explain.html](screens/17-mix-explain.html) | 17a where it opens (`Explain` in the top bar, ⌘E) · 17b the whole mix: signal map and text · 17c a track · 17d a shared effect (return) · 17e who listens to whom (sidechain) and an explained term · 17f a section of the song · 17g large project with folded groups · 17h never-touched mix · 17i minimum window · 17j the full text |
| 18 · Guided tour (P-020, A-095, A-096; approved by the author on 2026-11-15, implemented in F19, A-097) | [screens/18-tour.html](screens/18-tour.html) | the fifteen steps: 18a opening card on the Welcome · 18b top bar and transport · 18c timeline · 18d track header · 18e mixer · 18f inspector · 18g assistant · 18h proposal in review · 18i history · 18j Explain · 18k phase selector and mixdown · 18l Master chain · 18m loudness and A/B · 18n export · 18o closing card; then 18p step waiting for the proposal · 18q step waiting for the mixdown · 18r minimum window · 18s link in the Welcome · 18t app menu with Help › Take the Tour (A-096) · 18u tour with a project open · 18v notification after `Skip tour` · 18w demo project that cannot be prepared |

Component sheet, [components/index.html](components/index.html):

| Board | Components of §6 |
| --- | --- |
| C1 · Controls | fader, knob, pan, meter (peak/RMS, gain reduction, LUFS), buttons, toggles (mute, solo, lock, bypass), segmented control, numeric field, other fields, tag |
| C2 · Overlays and feedback | menu, tooltip, tab, dialog, toast, progress bar, banner; Master listening level, Listening section, target warning and note |
| C3 · Timeline | clip, fade handles, automation point, section, ruler, playback cursor, track header |
| C4 · Mixer and inspector | channel strip, chain module, EQ curve, transfer curve, routing selector, sidechain indicator |
| C5 · Assistant and AI usage | proposal card, diff row, note, AI call row, measurement table, request field, progress steps |
| C6 · Foundations | palette of the two phases, state colors, 12 track tints, typography, spacing, radii, animation, icons |
| C7 · Glance marks (P-013) | badges of the five chain blocks (HP LP EQ CMP SAT) in the three states, parallel and sidechain compressor, send dots, automation mark, header with the marks, Overview cells, strip with the label column |
| C8 · Guided tour (P-020) | spotlight; bubble of a step (arrow, focus on Next), waiting, inside the lit area; opening and closing card |

States shown for each control: normal, hover, active, keyboard focus, disabled and, where meaningful, automated and
locked for the AI. The classes `is-hover`, `is-active`, `is-focus`, `is-dis`, `is-auto`, `is-lock`, `is-sel` exist to
show them statically; in the app they correspond to `:hover`, `:active`, `:focus-visible`, `[disabled]` and to state
attributes.

## Behaviors not deducible from the boards

### Resizing

Reference 1440×900, minimum 1280×720, fluid up to 4K.

- **Fixed:** top bar (`--top-h`), side columns (`--col-side`), track header (`--head-w`), gutter (`--gap`).
- **Fluid:** the lane area takes all the width that remains; the mixer strips share the width between `--strip-w` and
  `--strip-w-max`; the timeline takes the height that remains above the mixer.
- **Below 1360 px of width (compact layout, board 3d):** side columns at `--col-side-min`, headers at `--head-w-min`,
  strips at `--strip-w-min`, header mini fader hidden, compressor transfer curve hidden, rationale card hidden in the
  inspector (it stays in the assistant).
- **Below 800 px of height:** lanes go to `--lane-s`; at that height the clip loses the name band and the header the
  role row.
- **Mixer:** height `--mixer-h`, resizable by the user between `--mixer-h-min` and `--mixer-h-max` by dragging the top
  edge. Below the minimum strip width it scrolls horizontally and the mix bus stays pinned to the right.
- **Collapsible panels, in this priority order** (the first is the one the app offers to close first when space is
  short, and each has its own shortcut): assistant (`⌘J`) → inspector (`⌘I`) → mixer (`⌘M`). A closed column becomes a
  bar of `--rail`; the closed mixer becomes a meter-only strip of height `--bridge-h`. The timeline never closes.
- **Above 1920 px:** the side columns grow up to `--col-side-max`; the rest goes to the timeline. The interface does
  not scale: on 4K you see more tracks and more time.
- **Master:** same rule for the columns; waveform and chain take the width, the module editor the height that remains.

### Density

Three lane heights: `--lane-s` (26), `--lane` (38, default), `--lane-l` (60). Vertical zoom moves from one to the
other; boards 6a–6e use the tall one. The automation lane is always `--auto-h` tall. Master deliberately has fewer
elements and larger numbers (`--fs-6`).

### Animations

Only `opacity` and `transform` are animated. No shadow and no blur on moving elements.

| What | Duration | Curve |
| --- | --- | --- |
| hover, press | `--dur-1` | `--ease-out` |
| toggle, tab, menu opening | `--dur-2` | `--ease-out` (menus close immediately) |
| panels opening or closing, dialogs | `--dur-3` | `--ease-in-out` |
| Mix ↔ Master phase change | `--dur-4` | `--ease-in-out`: cross-fade of the two workspaces, the accent changes halfway |
| indeterminate bar | 1.2 s in a loop | `--ease-linear` |

Playhead, meters, waveforms, EQ curves and automation are drawn on canvas and use no CSS transitions: the playhead
moves by `transform`, meters rise immediately and fall at 20 dB/s, the peak holds for 2 s. With "Reduce motion" on,
only the cross-fades at `--dur-1` remain.

### Drags and gestures

- A click becomes a drag after `--drag-threshold`. `Esc` cancels the gesture in progress and leaves no undo steps; each
  completed gesture is a single step.
- **Continuous controls** (fader, knob, pan, numeric fields): vertical drag, full travel in `--knob-travel`; `⇧`
  multiplies by `--fine`; wheel by steps; double click for the default value; click on the value to type it, `↩`
  confirms, `Esc` cancels. An out-of-range value colors the border red and is not applied.
- **Automated control:** the cap or arc turns pink (`--auto`) and follows the curve; moved with the transport stopped,
  it shifts the whole curve.
- **Track locked for the AI:** blue lock on, clips dimmed; the controls remain the user's.
- **Clip:** dragging the body moves it (also between tracks of the same format; the destination lane gets an accent
  outline); the edges trim; the fade handles appear on hover at the top corners; dragging with `⌥` duplicates. Snap
  attaches within `--snap-dist` to grid, clip edges, cursor and section boundaries, shows a blue line and a label, and
  is suspended by holding `⌘`. A clip dropped over another trims the one below.
- **Selection:** click, `⇧`-click to add or remove, box by dragging on an empty lane.
- **Automation:** with the automation tool, clicking the curve adds a point, dragging moves it, dragging on empty space
  draws freehand (simplified to points on release), box to select a range, `⌫` deletes. The lanes written by the
  assistant are blue; as soon as the user touches a point the lane turns lime.
- **Sections:** dragging on the empty lane creates one; a boundary is dragged by moving both neighbors; double click
  renames.
- **EQ:** the handles move in frequency and gain; the wheel changes Q; double click resets the band.

### Assistant

- Notes are numbered in the order in which the plan returns them; the number stays the same as long as the proposal (or
  its applied result) is the current one. Clicking a note selects the track; clicking the number in the header opens
  the note.
- During a call the workspace stays usable; the request field is replaced by "Cancel".
- The proposal in review takes the place of inspector and assistant; timeline and mixer stay live for listening. The
  A/B does not stop playback and does not enter the history.
- The **Master** selector in the phase selector opens dialog 8a if there is no up-to-date mixdown; otherwise it
  changes phase immediately (with the preparation 9d–9e if the master measurements are not ready).

### Notifications, tooltips, menus, dialogs

Toasts at the bottom right, stacked upward, gone after `--toast-life` if they have no action and are not errors.
Tooltips after `--tip-delay` on hover, immediately with keyboard focus. In dialogs focus is trapped, `Esc` cancels,
`↩` runs the primary action unless it is destructive.

### Master preparation (9d–9k)

Added on November 11, 2026, at the author's request. When entering Master, the mixdown and the references are read and
measured and the master is brought to the target loudness; until this work is finished the Master workspace is not
usable (empty waveform, unknown loudness) and must not be shown half-done.

**When the overlay appears.** Every time the master cache must be (re)built: "Open Master" after a bounce (8c), the
Master selector on a reopened project that already has a mixdown, "Bounce again" from 9b, a reference added, removed
or replaced while in Master. The work starts as soon as the mixdown is written (already while 8c is being read); on a
reopened project it starts only when entering Master. The overlay appears only if the wait exceeds `--wait-show`
(250 ms); once shown it stays at least `--wait-min` (600 ms), so it does not flash. If everything is already saved in
the project, Master opens without an overlay.

**What it covers.** Everything under the top bar: the workspace below is drawn only as empty shells (tab and module
names, no values) behind the scrim. The top bar stays: name, status `Mixdown N · preparing`, phase selector active;
transport, position, undo/redo, A/B and Export disabled (`--dis`). Space, transport keys and `⌘Z`/`⇧⌘Z` do nothing.

**Panel** (dialog component of size m, but `role="status"` and not modal):

| Element | Text |
| --- | --- |
| Title | `Preparing the master` |
| Subtitle | `Mixdown 3 · 3:42 · 48 kHz · 2 references` (`1 reference`, `no references`) |
| Under the bar | on the left `38 %`; on the right `0:06 elapsed · about 12 s left`. The remaining time appears after the first measured step (before that only `0:06 elapsed`); under one second it becomes `almost done` |
| Note (with references) | `The master opens when every measure is ready, so the first thing you hear is already at the target loudness. Saved in the project, so this runs once per mixdown and reference.` |
| Note (without references) | `No references in this project, so there is nothing to convert or compare. Add one in Project settings.` |
| Footer | `Going back to Mix keeps this going.` and the `Back to Mix` button with `Esc` |

The bar is determinate, in the phase accent (lilac, not the sky blue `--ai`: it is not the assistant), weighted by the
estimated work of each step; the backend computes the fraction.

**Steps**, in this order. A step that is not needed (no reference, reference already at the project sample rate or
with the converted copy already in the project) is **not listed**. A step whose result is already ready is listed as
done immediately, with the provenance in place of the duration.

| Step | Label | Secondary row | On the right |
| --- | --- | --- | --- |
| read | `Reading Mixdown 3` | — | `1.6 s` · `From the bounce` |
| mixdown measurement | `Measuring the mixdown` | `Loudness, peaks, spectrum, waveform` | `2.1 s` · `Saved in the project` |
| conversion (one per reference) | `Converting Reference A to 48 kHz` | `From 44.1 kHz · the copy stays in the project` | `46 %` |
| reference measurement | `Measuring the references` | the names: `Reference A, Reference B` | `52 %` · `Saved in the project` |
| loudness | `Bringing the master to the target loudness` | `Streaming · −14 LUFS · −1 dBTP` | `70 %` |

Step states: waiting (circle with the number, `Waiting`), running (number on accent, bold, percentage), done (check on
green, duration or provenance), skipped with warning (triangle on amber, `Skipped`), failed (cross on red, `Failed`),
not started after an error (dimmed, `Not started`). Every state also has the icon or the number, not only the color.

**End.** At the last step the bar reaches 100 %, the overlay fades in `--dur-3` and underneath is already the complete
workspace (waveform, measurements, loudness, chain): everything appears at once, with no progressive filling. Focus
returns to the Master segment of the selector. With "Reduce motion" the fade is `--dur-1`.

**Esc and return to Mix (9f, 9g).** `Esc`, "Back to Mix" and the Mix segment do the same thing: you return to Mix
**without cancelling**. Preparation continues on the auxiliary threads; the Master segment shows a ring with the
progress and the tooltip `Preparing the master · 64 % · keeps going while you mix`. If it finishes while you are in Mix,
the toast `Master is ready` / `Mixdown 3 is measured and at −14 LUFS for Streaming.` appears with the action `Open
Master` (it stays until closed, because it has an action); at that point Master opens immediately. If you return to
Master before the end, the overlay resumes from the current state. If the cache becomes useless (new bounce, references
changed) the work restarts and the steps that are still valid stay done. Closing the project abandons it.

**No "Cancel".** The work does not change the project (it writes only cache), it is needed anyway to use the Master and
stopping it would mean redoing it from scratch on return. The only exit is "Back to Mix", which throws nothing away.

**Errors.**

- *Unreadable mixdown* (missing file, truncated, unexpected format) — the only blocking error (9j). Title `The master
  can’t be prepared`; subtitle `Mixdown 3 could not be read: the file is missing from the project or damaged.`; the
  failed step carries the cause (`The file ends early (2:18 of 3:42)`, `The file is missing`, `The file is not a WAV
  file`); note `Your mix and your master settings are safe. Bouncing again writes a new mixdown from the current mix
  and keeps every master setting.`; footer `Copy details` · `Back to Mix` · `Bounce again` (primary, leads to 8b and
  then to 9d). No bar.
- *Unexpected error in a later step* (memory, internal error) — same pattern, subtitle `Something went wrong while
  measuring the references.` (the step in lowercase), primary `Try again`.
- *Unreadable reference* — does not block (9h): the step becomes `Reference B can’t be read` / `The file is missing or
  damaged · skipped`, preparation continues, and Master opens with the banner (9i) `Reference B can’t be read` / `The
  comparison, the match EQ and the A/B use Reference A only until you replace or remove it.` with `Project
  settings…` and `Not now`.
- An error while in Mix arrives as an error toast `The master can’t be prepared` with the action `Show`, which opens
  Master on 9j.

**New measurements after a change (9k): no overlay.** Every change to the master (a knob, a bypass, undo/redo in
Master; not the target, see 9v) reaches the chain immediately and is heard; in the background only the loudness
search is redone. Covering the workspace at every gesture would break the "touch and listen" loop, so the state is
inline, in the Loudness card: label `Measuring…` (clock, accent) in place of `On target`, indeterminate bar, row `Your
change is already playing. The level match to −14 LUFS follows when the measure is done.`, integrated loudness and
true peak in `--tx-3` with the row `before the change`. The real-time meters continue. Here too the state appears only
after `--wait-show`. While measuring: A/B active (uses the last measurement), "Master with AI" disabled with tooltip
`Waiting for the loudness measure`, Export openable but with the primary `Measuring…` disabled until the measurement
arrives.

**New target (9v): overlay.** Added on November 11, 2026, at the author's request (A-061). Changing the loudness or
the ceiling of the target (a preset, the Custom values, `Match the reference`, undo or redo of one of these, a master
proposal that sets it) is the only change that is **not heard immediately**: the new level arrives only when the gain
search is finished. With the inline state of 9k the author kept working believing the change had already happened. For
this reason here the workspace waits under the same panel as the preparation, above the workspace **as it is** (not the
shells: the values behind the scrim are those of the previous target).

| Element | Text |
| --- | --- |
| Title | `Bringing the master to the new target` |
| Subtitle | `Custom · −9.6 LUFS · −0.3 dBTP · was −14 LUFS, −1 dBTP` |
| Under the bar | on the left `42 %`; on the right `0:04 elapsed` (no estimate: the number of passes is not known beforehand) |
| Step 1 | `Finding the gain for −9.6 LUFS` (if only the ceiling changes: `Finding the gain under the −0.3 dBTP ceiling`) / `The whole master is rendered and measured until it is on target` / percentage |
| Step 2 | `Measuring the result` / `Loudness, true peak, range` / `Waiting`, then `Working…` |
| Note | `Until this is done the master still plays at the level it had. The workspace comes back when the new level is in place, so what you hear and what you read are the new target.` |
| Footer | `Going back to Mix keeps this going.` and `Back to Mix` with `Esc` |

Rules: it appears after `--wait-show` and stays at least `--wait-min`, like 9d (a target already measured returns
immediately, with no overlay). While dragging a Custom value it does not appear: it starts on release. Top bar: phase
selector and **transport active** (you can stop or listen: it plays the master at the previous level); undo/redo, A/B
and Export disabled; `⌘Z` and the other keys do nothing, `Esc` returns to Mix without stopping the work and the Master
segment shows the ring as in 9f. No "Cancel": when the work is finished the change is an undo step, and the previous
target is already measured, so going back to it is immediate. At the end focus returns to the target selector. If the
change happens inside the export dialog (10d) the overlay does not appear: there the primary button waits. Backend
field: `MasterStatus.retarget` (`from_lufs`, `from_true_peak_db`, `fraction`, `elapsed_ms`), present only during the
wait.

**Accessibility.** The panel is `role="status"` with `aria-live="polite"`: the step that begins is announced, not the
percentages. The bar has `role="progressbar"` with the value. On opening, focus goes to "Back to Mix"; `Tab` also
reaches the phase selector.

**Field required from the backend** (today there is only `MasterStatus.analysing`). Proposal: `MasterStatus.prep:
MasterPrep | null`, present only during cache construction, with `mixdown` (number), `fraction` (0–1, weighted),
`elapsed_ms`, `eta_ms | null`, `error | null` (`kind`: `mixdown_unreadable` or `internal`, `step`, `detail`) and
`steps: PrepStep[]`; each `PrepStep` has `kind` (`read_mixdown`, `measure_mixdown`, `convert_reference`,
`measure_references`, `loudness`), `state` (`waiting`, `running`, `done`, `skipped`, `failed`, `not_started`),
`source` (`bounce`, `saved` or none), `fraction`, `ms`, `reference` (id and name) and `from_rate` where needed. The UI
composes the texts from the `kind`. Rule: `prep` present ⇒ overlay; `prep` absent and `analysing` true ⇒ inline state
of 9k.

### Listening level and target warning (9l–9u, 10d–10e)

Added on November 11, 2026, at the author's request. Before, the Master always played loudness-matched (MAS-5): the
true level of the master was never heard, and a Custom target at −18 LUFS against a reference at −9.6 LUFS went
unnoticed until listening to the exported file.

**Two listening modes.** In the Master bar, before the A/B source selector, a `quiet` segmented control of the same
height: `Real level | Same loudness` (the second with the `level` icon, "=").

- **Real level (default).** The master plays exactly like the exported file; mixdown and references at their real
  level, with no gain or lowering.
- **Same loudness (on request).** The previous MAS-5 behavior: mixdown and references brought to the measured loudness
  of the master; if a peak then exceeds −1 dBTP, all sources, master included, drop by `ab_trim_db`.
- The state reads at a glance: the lit segment (in Same loudness with accent background and outline and "="), the
  header of the Listening section and the verb of the rows (`quieter`/`louder` versus `raised`/`lowered`).
- **Shortcut `⇧B`** (`Same loudness on / off`); `B` stays with the proposal and snapshot A/B (the `B` rule must exclude
  `⇧`). The segment tooltip: `Compare at the same loudness: quieter sources are raised, louder ones lowered. ⇧B`.
- The mode is not in the project and is not an undo step: every time the project is opened it starts from Real level;
  it stays what was chosen as long as the project is open, even when passing through Mix. Changing mode does not stop
  playback (like the A/B, with no click).
- During preparation (9d) the control is disabled along with the rest of the bar.
- With two or more references the source segments become `Ref A`, `Ref B` (full name in the tooltip and in the
  Listening section).

**Listening section of the Loudness card**, under integrated loudness and true peak: header `Listening` · `Real level`
(or `= Same loudness` in accent) and on the right `⇧B same loudness` (`⇧B real level`). One row per source: name,
measured integrated loudness, discrepancy in words. The row being listened to has the speaker icon and the `--bg-2`
background.

| Mode | Master | Mixdown, references |
| --- | --- | --- |
| Real level | `as exported` | discrepancy in LU against the master: `4.5 LU quieter`, `1.2 LU louder` |
| Same loudness | `unchanged`, or `lowered 2.5 dB` with the lowering | gain with which it plays: `raised 2.8 dB`, `lowered 1.2 dB` (lowering included) |

With the lowering active, under the rows: `Raising the mixdown 4.5 dB would push its peaks over −1 dBTP, so everything
plays 2.5 dB lower. The export is not affected.` (the source that causes it, with its gain). During the review of a
master proposal, when listening to the proposal the first row is `Proposal`.

**Proposal in review (9c, 9o).** The `Current master | Proposal` selector of the Assistant column follows the same
global mode and has no switch of its own. Below, at real level: `Real level` / `The proposal is 0.1 LU louder: −14.0
against −14.1 LUFS. B switches · ⇧B same loudness`; loudness-matched: `= Same loudness` / `The proposal plays 0.1 dB
lower to match the current master. B switches · ⇧B real level`.

**Target warning.** The references (the measured ones; the excluded ones of 9h do not) define an integrated loudness
range, from lowest to highest. The **loudness of the target** is compared (not the measurement of the master), so the
warning is immediate and does not wait for the loudness search.

- Target **more than 3 LU below the lowest reference** → amber warning (`banner stack`): `Much quieter than your
  reference` / `At −18 LUFS this master will sound 8.4 LU quieter than Reference A (−9.6 LUFS): clearly softer when the
  two play one after the other.` With multiple references: `Much quieter than your references` / `At −18 LUFS this
  master will sound quieter than both: 6.8 LU below Reference B (−11.2 LUFS) and 8.4 LU below Reference A (−9.6
  LUFS).` (with three or more: `than all of them`).
- Target **more than 3 LU above the highest reference** → neutral note (`banner stack info`): `Much louder than your
  reference` / `At −9 LUFS this master will be 5.2 LU louder than Reference A (−14.2 LUFS). Getting there takes more
  limiting, which can flatten the punch, and streaming services turn loud songs down anyway.` Only in the Loudness
  card and in the menu: too loud is heard immediately at real level and is seen in the limiter's reduction.
- Within the range, within 3 LU of the edges or without references (9p): nothing.
- **Actions:** `Match the reference(s)` (primary) brings the target to the nearest edge of the range (the lowest
  reference if below, the highest if above), as Custom and **with the previous ceiling**; a single undo step (`Target
  matched to the references`). Row below: `Sets the target to −9.6 LUFS and keeps the −1 dBTP ceiling · one undo
  step`. `Keep −18 LUFS` (secondary) keeps the target.
- **Where it appears:** under the Target selector of the Loudness card, with the selector with an amber border and the
  triangle (9q, 9r); the reference row in the Listening section in amber with the triangle; in the target menu (9s)
  each entry says where it stands (`2.8 LU quieter`, `0.6 LU louder`, `Between your references`, in amber with the
  triangle beyond 3 LU) and there is the entry `Match the references · −11.2 LUFS` (`Keeps −1 dBTP`); in the export
  dialog (10d) as a banner above the formats, with the same actions: Export stays available, and "Match the
  reference" there brings the primary to `Measuring…` as in 9k; in the report (10e) an amber row `8.4 LU quieter than
  Reference A. … If that wasn’t intended, choose “Match the reference” in the Loudness card and export again.`
- **Keep** applies to the project: saved in the project, it is an undo step of the Master history (`Kept the target`).
  From that moment no warning or note for the target, anywhere; under the selector the discreet row `8.4 LU quieter
  than Reference A · kept on purpose. Warn me again` (9t) stays as long as the discrepancy exceeds the threshold, and
  the export report writes in neutral `Exported at −18 LUFS on purpose, 8.4 LU quieter than Reference A.` "Warn me
  again" removes the Keep (also an undo step).

**To decide in implementation.** Whether the meters of the Loudness card measure before or after the listening gain;
proposal: before, that is always the real level of the source, because the numbers of the card speak about the file.
Fields required from the backend: the listening mode (`MasterStatus.listen_same: bool` and the command to change it),
the loudness of the master (already in `measures`), of the references (already in `references`) and of the mixdown
(already in `mixdown_measures`), the gain of each source when loudness-matched (today deduced from `ab_trim_db`), and
in the project the Keep (`master.target_kept: bool`).

## Fonts and icons

- **Bricolage Grotesque** (headings) and **Figtree** (everything else): both SIL Open Font License 1.1, so they can be
  included in the app. The boards load them from Google Fonts; **the app must include their files** (weights 500 and
  600 of Bricolage Grotesque; 400, 500 and 600 of Figtree) with the license texts. Tabular figures everywhere.
- The raw JSON uses the system monospace font, which is not distributed.
- **Icons:** a dedicated set, drawn for this project (16 px grid, 1.5 px stroke, rounded ends). The paths are in
  `tools/lib.mjs` (object `IC`) and all the icons are in C6.

## Decisions made autonomously

| No. | Decision | Reason |
| --- | --- | --- |
| D1 | The mixer is a bottom panel, not an alternative view. | It keeps the same state visible together with timeline and inspector (TRK-4). |
| D2 | The proposal in review replaces inspector and assistant with a single wide panel. | Summary, evaluation and 15 readable cards are needed; timeline and mixer must remain for listening. |
| D3 | The lanes written by the assistant are blue, those drawn by hand take the phase accent. | It makes visible who wrote what without text. |
| D4 | The phase selector says "Mix" and "Master", without numbers. | Author's request. |
| D5 | The track EQ bands have a name in words (Mud, Presence, Harsh, Air) derived from the frequency; the numeric values remain. | The user is not an engineer. |
| D6 | The chain in the inspector is a grid of 8 blocks in the fixed order of FX-1 (fader and pan in a single block); the output is a selector in the header. | In a single row the values were cut off. |
| D7 | In Master the target is in the Loudness card, not in the top bar. | The bar did not hold A/B, target and export together at 1440 px. |
| D8 | The master chain is a numbered 4×2 grid 1–8. | Eight boxes in a row cut off the names. |
| D9 | In the Master the waveform shows the master (lit) above the mixdown (gray). | It shows at a glance how hard the chain works. |
| D10 | The clip name is at 9 px bold. | Author's request; it is the only text under 11 px and it is redundant with the track header. |
| D11 | Below 32 px of lane the clip loses the name band. | There would be no space left for the waveform. |
| D12 | The channel meters are solid color: mint, amber above −6 dBFS, red cap on the over. | No gradient on elements that change at 30 Hz. |
| D13 | Compact layout below 1360 px and panel closing order (assistant, inspector, mixer). | See "Resizing". |
| D14 | The offline demo is offered already at first launch. | AI-9: the fake decider is selectable; it avoids a dead end for those without a key. |
| D15 | The crash recovery dialog and the relinking of missing files are drawn in 13g. | PRJ-2 and UI-6 require them, §6 does not list them as screens. |
| D16 | The bounce result (8c) already uses the Master accent on the primary button. | It marks the moment of the phase change. |
| D17 | Added tokens for the gestures (drag threshold, snap distance, knob travel, fine multiplier, delays). | They are measures the implementation must share. |
| D18 | The boards are generated by a script and delivered as static HTML. | Forty-nine boards with realistic waveforms are not maintainable by hand; the resulting HTML has no dependencies. |
| D19 | The master preparation covers the body of the workspace, not the top bar. | The phase selector must stay usable to return to Mix; the rest of the bar is disabled. |
| D20 | No "Cancel" during preparation: "Back to Mix" (and `Esc`) returns to Mix and the work continues. | The work does not touch the project and is needed anyway; cancelling it would only make it be redone. |
| D21 | New measurements after a master change do not use the overlay but an inline state in the Loudness card. | The chain already plays the change; blocking at every gesture would make the Master unusable. |
| D22 | An unreadable reference is a warning and is skipped; only the unreadable mixdown blocks. | Without references the master works; without a mixdown it does not. |
| D23 | Steps that are not needed are not listed; those already ready appear done, with the provenance. | The list describes what really happens and explains why it is fast. |
| D24 | The running step and the preparation bar use the phase accent, not the sky blue `--ai`. | The sky blue is reserved for what the assistant does. |
| D25 | Tokens `--wait-show` and `--wait-min` for background waits. | They prevent overlay and inline state from flashing; they are values the implementation must share. |
| D26 | The Master plays at the real level by default; loudness matching (MAS-5) is an on-request mode, `⇧B`. | The user must hear the master as the exported file; the loudness-matched comparison remains for judging the sound. |
| D27 | The listening mode and the source are two segmented controls of the same height in the bar; the discrepancies are in the Listening section of the Loudness card. | With the discrepancies in the segments the bar did not hold two references at 1440 px; the Loudness card is where levels are discussed and it says them in words. |
| D28 | A single listening mode for sources and proposal; the proposal selector has no switch of its own. | One place for one state. |
| D29 | The warning compares the target with the range of the references: more than 3 LU below the lowest a warning, more than 3 LU above the highest a neutral note; the fix goes to the nearest edge and keeps the ceiling. | It warns only when the target is outside all the references; too loud is heard immediately and is not needed for export. |
| D30 | "Keep" applies to the whole project and is an undo step; a discreet row with "Warn me again" remains. | No insistence, but the discrepancy stays visible and tracked in the export report. |
| D31 | With two or more references the segments shorten to `Ref A`, `Ref B`. | Bar space; the full name is in the tooltip and in the Listening section. |

Names, titles, paths and values in the boards are example content.

## Inventory check (FOUNDATION §6)

Screens: 1 → 1a–1b · 2 → 2a–2d · 3 → 3a–3d · 4 → 4a–4b · 5 → 5a–5b · 6 → 6a–6f · 7 → 7a–7e · 8 → 8a–8c ·
9 → 9a–9u · 10 → 10a–10e · 11 → 11a–11d · 12 → 12a–12e · 13 → 13a–13h. Total 69 screen boards and 6 component boards.

Details of §6 and where they are:

- top bar (name, phase selector, transport, position, loop, undo/redo, bus meter, load): 3a;
- timeline (ruler, sections, headers, clips, automation, cursor, loop): 3a, C3;
- mixer (meters, fader, pan, sends, gain reduction; groups, returns, bus): 3a, 7c, C4;
- inspector (chain with bypass, EQ, compressor, saturation, sends, rationale): 3a;
- AI panel (brief, references, Auto mix, request in words, estimate, cost with access to AI usage): 3a;
- history panel (steps, snapshots, A/B): 3b, 3c;
- creating and deleting a return: 7b (menu), 7a (button), 13g (confirmation);
- UI-6 errors: missing files 13d and 13g · different sample rates 2c and 2d · missing key 13e · no network 13f ·
  disconnected device 13d.

Components: all 29 listed in §6 are in C1–C5 (table above).

## Known limits

- The boards were checked by rendering them with Chrome at 1440 px of width; they were not tried in other browsers or
  in the Tauri webview.
- Contrast was computed from the tokens (text from 5.4:1 to 16.5:1 on the four backgrounds of both phases; dark text on
  accents, states and tints from 6.3:1 to 13.2:1). Disabled states and muted clips are deliberately below threshold.
- The positions of clips, points and menus in the boards are example coordinates written inline: they are not design
  measures. The measures are only in the tokens.
- The hover, active and focus states are static: the transitions described above are not demonstrated.
- In the mixer at 1440 px the long strip names are truncated; the full name is in the tooltip and in the track header.

## Explain this mix (screen 17, P-019)

Added on November 15, 2026, requested by the author: the interface shows the state of the mix but not the flow or the
intent, and a non-engineer does not understand who goes where and why. Approved on 2026-11-15 and implemented in F18;
the differences from the implementation are in A-093.

**What it is.** A Mix view, opened by `Explain` in the top bar (⌘E, `Esc` to close), that takes the place of timeline,
mixer and panels. The top bar stays live: you listen while you read. On the left the **map**, on the right **the same
mix in words**. Everything is read from the project document and the analysis already in cache: **no model**, no call,
no cost; same project, same page. It is read-only: it creates no undo steps.

**Map.** Three columns, from left to right: tracks grouped by family, groups (next to their members), mix bus; shared
effects (returns) sit under the groups. One more level of group nesting adds a column.

| Mark | Meaning |
| --- | --- |
| Solid line in the track color | Output. The thickness follows the level (2–9 px). |
| Dashed line | Send: a copy toward a shared effect. |
| Dotted line with arrow, in the source color | Sidechain: who listens to whom. |
| Blue / lime diamond | Automation written by the assistant / by hand (as UI-8). |
| Blue number | Assistant note, the same number as headers and inspector. |
| Row with border and `›` | Folded group: its tracks in one row, with the count. |

A click on a box selects it: its path to the bus stays lit, the sends with the level written on them and the sidechains;
the rest is dimmed. Double click: closes the view and opens the element in the inspector. The strip above the map
chooses the whole song or a section (repetitions count as one): tracks that do not play there are dimmed, those that
play only there or have different values carry a label.

**Text.** Always in the same order; a part without content does not appear.

| Part | Where it comes from |
| --- | --- |
| In short | Document counts; loudest track and family from the analysis; pan; differences between sections. |
| How it is built | Groups with members and the blocks on at group level; returns with type, parameters and who sends; tracks going directly to the bus. |
| Who is in front | Loudness per family (AI-22, already computed). |
| Left to right | Pan of every track. |
| Space | Sends per return, from highest to lowest. |
| Who reacts to whom | Sidechains of the document; average reduction measured by the engine. |
| What changes along the song | Clips present per section, automation and per-section offsets. |
| Reasons | `summary` and `rationale` of the last applied proposal, **quoted** as they are (in the language of the brief). |

Selecting a track, a group or a return, the text becomes its card: where it goes, its place in the mix, the blocks of the
chain **in signal order** (badge, name, value, a sentence on what it does), the assistant's reason and what was
changed by hand afterward. Selecting a section: what plays, what is different, loudness compared.

**Text rules.** Every sentence is a fixed template filled with project values: one sentence per block type (for
example compressor: `Turns the loudest {words|hits|notes} down…: about {GR} dB on the peaks`, with the noun chosen from
the track's role). Track, group and return names are **chips** with the color of the element: a click selects it in
the map. Technical terms (`group`, `shared effect`, `sidechain`, `compressor`, `parallel compressor`…) have a dotted
underline and a one-sentence tooltip, from a fixed glossary. The text **describes and does not judge**: no advice, no
quality adjectives. The reasons are only those the assistant wrote; a change made by hand has no reason. `Copy as text`
copies the plain text.

**Measures.** Text column 496 px (`--col-explain`, 432 at the minimum); a track row in the map 30 px (`--map-node`), 26
with many tracks, 22 in the minimum window; beyond that, the map scrolls vertically.

**Outside these boards.** The narration written by a model (local or remote) on top of these facts: decision deferred
by the author. The Master does not yet have its version of the view.

## Guided tour (screen 18, P-020)

Added on November 15, 2026 (issue #5, specification in FOUNDATION: P-020, PRJ-9, UI-17 → UI-19). Boards 18a–18w and
component C8; choices in A-095 and A-096. Approved by the author on 2026-11-15 and implemented in F19; the
differences from the implementation are in A-097 (the "Try it" of step 8 no longer names the B key, suspended during
the tour).

**What it is.** A tour of the app in fifteen steps on a **demo song** that the app builds and synthesizes by itself
(PRJ-9): it is not a user project, it is not saved, it disappears at the end. It starts by itself at first launch (card
18a, on the Welcome) and on request from `Help › Take the Tour` (18t) and from the Welcome link (18s). Each step has a
spotlight on the area it talks about and a bubble next to it; you go forward with `Next`, no step waits for a gesture.
The final texts of the steps are in `tools/s-tour.mjs` (`STEPS`), to be copied as they are into the app strings.

**The song of the boards.** The boards use the twelve tracks of the sample song (`lib.mjs`) over 68 s with three
sections (Verse, Chorus, Outro), two groups, two returns, the bass sidechain on the kick and the hand-drawn automation
lane of Lead Vox: it is what PRJ-9 asks to show. Project name `Demo song`, status `Tour · nothing is saved`, no
references, no cost (`Offline demo · no cost`). The Master and Explain boards take up the rest of the sample song
(chain, measurements, text): they are example content.

**The steps and the state they establish.** Each step declares the state it needs and sets it on entering, both going
forward and going back (UI-18). The target is found by **role and accessible name** (`aria-label` of the areas, role
`dialog`, the Play button), never by id.

| Step | Board | Target | State the step sets | Bubble side |
| --- | --- | --- | --- | --- |
| 1 Welcome to Stemwise | 18a | none: card in the center | Welcome; the song prepares in the background | — |
| 2 Play the song | 18b | top bar, arrow on Play | Mix, demo project open, playhead at the start of the Chorus | below |
| 3 Tracks on a timeline | 18c | `Timeline` | panels open, full zoom | right |
| 4 A track | 18d | Lead Vox header | Lead Vox selected | right |
| 5 The mixer | 18e | `Mixer` | mixer open in Faders view | right |
| 6 The inspector | 18f | `Inspector` | inspector open on Lead Vox, EQ open | left |
| 7 The assistant | 18g | `Assistant`, arrow on the request field | Notes tab, no proposal | left |
| 8 A proposal to review | 18h (18p waiting) | `Proposal` | fake decider proposal in review, A/B on the proposal | left |
| 9 One step to undo | 18i | `Assistant` | proposal applied (one step), History tab | left |
| 10 Explain this mix | 18j | everything under the bar | Explain view on the whole mix | inside |
| 11 Mix, then Master | 18k (18q waiting) | phase selector | Mix, Explain closed, mixdown done or in progress | below |
| 12 The master chain | 18l | rack and editor | Master, Clipper open | right |
| 13 Loudness and A/B | 18m | `Loudness` card | Master, A/B on the master | left |
| 14 Export | 18n | export dialog | dialog open and moved to the right, Export off | left |
| 15 Ready for your own songs | 18o | none: card in the center | Master under the dimming; `Done` closes | — |

Going back one step undoes only what the next step had added and the previous one does not declare: from 9 to 8 the
proposal returns to review (undo of the application); from 12 to 11 you return to Mix; from 14 to 13 the dialog closes.
What the user changed inside the lit area (a fader, a block turned off) stays until a step declares otherwise;
everything is lost at the end.

**Spotlight.** A layer above the whole window (`--z-tour`, above dialogs and toasts). The hole is the target enlarged by
`--tour-pad` (4 px), with the card radius (`--r-3`), a 2 px ring in the **phase accent** (lime in Mix, lilac in
Master: the tour is not the assistant, so no `--ai`) and around it the dimming `--tour-dim`, lighter than the dialog
scrim so the context stays readable. The hole does not leave the window: on an area as wide as the window (the top bar)
it is pulled in just enough to show the ring. Inside the hole the app works (Play, a fader, a block); outside, clicks
do nothing. The tour does not use the dialog scrim: a lit dialog (export) loses its own and moves to the right edge, at
`--gap` from the edge, to leave room for the bubble (18n). The opening and closing cards dim the whole window.

**Bubble: position.** Width `--tour-w` (340). One side after another is tried and the first where the bubble fits with
`--tour-gap` (12) from the hole and at least `--tour-edge` (16) from the window is taken: first the side preferred by
the step (below for the top bar targets), then **right, left, below, above**. On the chosen side the bubble is centered
on the target (or on its anchor point, for example Play) and slides along the side until it is entirely inside the
window. The arrow (square of `--tour-arrow` rotated) sits on the side facing the target and points to the anchor, never
closer than 24 px to the corners of the bubble. If no side suffices (Explain, 18j) the bubble sits **inside** the hole,
at the bottom left 16 px from its edges, with no arrow. The height is the real height of the bubble, measured before
positioning it. At the minimum window (18r) the target is first brought into view (scrolling of the timeline or the
mixer), then the bubble is positioned with the same rule; on every window resize it is recomputed.

**Bubble: content.** Counter `3 of 15` with a bar of fifteen marks (done ones in accent, the current one light); title
in the heading typeface; one or two sentences; an optional `Try it: …` row when the area invites a gesture (steps 2, 5,
8); at the bottom `Skip tour`, `Back`, `Next` (`Done` on the last, without `Skip tour`). The opening card has `Skip
tour` and `Start`, the line about the synthetic sound and the tour keys.

**Waiting steps (18p, 18q).** Proposal and mixdown are prepared in the background from the start of the tour. If a step
arrives before they are ready, the bubble adds a status row (`Preparing the proposal…` in blue, because the assistant
does it; `Bouncing the mix to the mixdown…` with a determinate bar in the phase accent) and `Next` is off; as soon as the
result is there the step shows it by itself and `Next` turns on. `Back` and `Skip tour` remain. Step 8 while waiting
lights up the assistant at work (same view as 5a); the Master at step 12 may still show the preparation of 9d.

**Animations.** Only `opacity` and `transform`, as in the rest of the app. On a step change the bubble and the ring fade
in `--dur-1`; the hole moves and resizes with a `transform` (translation and scale) in `--dur-3` `--ease-in-out`; the
bubble reappears in its place in `--dur-2` `--ease-out`. The phase change (steps 11 → 12) is the usual cross-fade of
`--dur-4`: the bubble appears when it is over. Tour appearance and exit: fade of the dimming in `--dur-3`. With **"Reduce
motion"** no movements or scales: hole, ring and bubble swap with a fade of `--dur-1`.

**Keyboard and focus.** `→` and `Enter` forward, `←` back, `Esc` skips the tour; the space bar stays with the transport
(also with focus on the bubble). The other app shortcuts are suspended; the project commands of the menu bar (New,
Open, Close) first close the tour. At each step focus goes to `Next` (to `Start` and `Done` in the cards); `Tab` cycles
between the bubble and the controls of the lit area, and only there. The bubble is a `dialog` with the title as its
label, non-modal; the step change is announced by an `aria-live="polite"` region (`Step 3 of 15: Tracks on a
timeline`). Contrast like the rest of the interface (text on `--bg-2` at least 5.4:1).

**Exit (UI-19).** `Done`, `Skip tour`, `Esc` and closing the app do the same thing: playback stops, the demo project
closes and is deleted, the Welcome returns (1b at first launch, with the invitation to the assistant), the preferences
stay as before. After `Skip tour` or `Esc` the toast of 18v appears (`Tour skipped`), with no action, gone after
`--toast-life`. Without an audio device the tour continues without sound, with the state of 13d in the bar. If the song
cannot be prepared the opening card says so (18w) and the tour ends.

**Start on request.** The Welcome link sits under `New project` and `Open project…`, with the duration; the tooltip
says what it is (18s). With a project open the entry is in the **app menu**, which opens from the project name and now
collects all the Stemwise commands (UI-20, A-096): `New project…` ⌘N, `Open…` ⌘O · `Project settings…`, `Save` ⌘S,
`Save as…` ⇧⌘S, `Close project` · `Settings…` ⌘, · `Help ›` `Take the Tour`. The macOS menu bar keeps only the system
Stemwise menu, Edit and Window. `Take the Tour` is off while the assistant is working, during a bounce or an export
(18t). With a project open it goes through the close confirmation, with a row saying why (18u); `Cancel` leaves
everything as it was.

**Measures.** `--tour-w` 340, `--tour-card-w` 480, `--tour-pad` 4, `--tour-gap` 12, `--tour-edge` 16, `--tour-arrow` 12,
`--tour-dim`, `--z-tour` 45. The spotlight positions in the boards are measured on the boards themselves (`R` in
`s-tour.mjs`); in the app they come from the real target.

**Generator.** `s-tour.mjs`; to reuse the parts of the other screens `timeline`, `top`, `wave`, `autoGeo`, the Explain
section strip and the Master waveform accept a different song (`T`, `sections`), and `proposal`, `explain`,
`masterChain` are exported. The other boards stay identical, except 1a, 1b and 14 which show the link to the tour.
