# Product decisions

Log of the decisions made during design and implementation. The author's decisions have ids `P-nnn` and
live in [FOUNDATION §1.1](../FOUNDATION.md#11-decisions-made-with-the-author-2026-11-10); those the agent made
autonomously have ids `A-nnn` and live here. A decision is never deleted: if it changes, a new one is added
that supersedes it. The PoC decisions (`D-nnn`) remain in [poc/docs/DECISIONS.md](../../poc/docs/DECISIONS.md).

Format of an entry:

```markdown
### A-001 — Title (date, phase)
Context. Alternatives considered. **Choice** and reason. Requirements affected.
```

## Agent decisions

### A-001 — Fonts bundled from the `@fontsource` packages (2026-11-11, F0)
The design loads Bricolage Grotesque and Figtree from Google Fonts; the app must bundle them (NFR-6, design/README).
Alternatives: download the TTFs by hand; load them from the network (forbidden by NFR-7). **Choice**: `@fontsource/figtree`
(400/500/600) and `@fontsource/bricolage-grotesque` (500/600), OFL 1.1, packaged by Vite; license texts in
`app/ui/public/licenses/`. Requirements: NFR-6, NFR-7, UI-2.

### A-002 — The UI imports tokens and component styles directly from `design/` (2026-11-11, F0)
Alternatives: copy `tokens.css` and `components.css` into `app/ui`; rewrite them as CSS modules. **Choice**: Vite alias
`@design` → `design/`; `main.tsx` imports `@design/tokens.css` and `@design/components/components.css`, and
`ui/src/styles/` adds only what the app needs (fluid window, interactive `:hover`/`:focus-visible` states
equivalent to the `is-*` classes of the boards). Reason: the design stays the single source of the visual language and
visual comparison is direct. Requirements: UI-2.

### A-003 — TypeScript 6 instead of 7 (2026-11-11, F0)
`typescript-eslint` 8.72 rejects TypeScript 7.0. **Choice**: `typescript@^6`, the latest line supported by the lint
(§8.2 requires types and lint). We can move to 7 when the lint supports it.

### A-004 — List of allowed licenses (2026-11-11, F0)
NFR-6 lists MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode. Some transitive dependencies of Tauri and of the tools
use equivalent or more permissive variants. **Choice**: also allowed are `MIT-0`, `0BSD`, `CC0-1.0`,
`BSL-1.0`, `Apache-2.0 WITH LLVM-exception`, `Unicode-DFS-2016`, `CDLA-Permissive-2.0` (Rust, `app/deny.toml`) and,
for npm, `OFL-1.1` (fonts only, as NFR-6 provides), `BlueOak-1.0.0`, `Python-2.0`, `CC-BY-4.0` (development data,
not packaged). No copyleft license, not even among the development dependencies.

### A-005 — AAC encoder via AudioToolbox with our own FFI (2026-11-11, preliminary)
There is no AAC encoder in Rust with a permissive license. **Choice**: an isolated module behind a trait
(`AacEncoder`), implemented on macOS with AudioToolbox's `ExtAudioFile` via FFI; elsewhere the format is not offered
(MAS-10, NFR-8).

### A-006 — Contents of "Appearance" in settings (2026-11-11, preliminary)
FOUNDATION (SET-1) only says "appearance". **Choice**: the options of board 11c, saved in the app preferences
(not in the project, they are view state: HIS-6).

### A-007 — Requirement traceability per phase (2026-11-11, F0)
`make check` must run `make reqs` "for closed phases" (§8.2) but §8.1 does not assign every id to a phase.
**Choice**: `app/scripts/req-phases.json` assigns every id (by prefix, with exceptions) to the phase in which it must be covered;
`node scripts/reqs.mjs --closed` fails if an id of a closed phase (✅/⚠️ in PROGRESS) is neither tested nor
justified in `REQS-JUSTIFIED.md`. NFR-6 and NFR-9 are justified as gate steps.

### A-008 — Test temporary folders inside `app/target/tmp` (2026-11-11, F0)
Rule 20: cleanup touches only ignored files inside the repository and the tests' temporary folders. **Choice**: the
Rust tests create their folders (with `tempfile`, deleted even on failure) under `app/target/tmp`,
which `make disk` measures and `make clean-stale` empties.

### A-009 — Playwright on WebKit (2026-11-11, F0)
**Choice**: Playwright uses the WebKit project, the engine closest to Tauri's WKWebView on macOS.

### A-010 — Optimization in the development profiles (2026-11-11, F0)
Unoptimized DSP is 20–50 times slower and would make tests and `make dev` slow. **Choice**: `opt-level = 2`
for the dependencies and for the crates `stemwise-dsp`, `stemwise-engine`, `stemwise-analysis` in the `dev` profile too;
reduced debug info (`line-tables-only`, none for dependencies) and no incremental cache for dependencies
(rule 18).

### A-011 — Verifying the Tauri app without screen capture (2026-11-11, F0)
`screencapture` asks the author for a system permission: it is not used. **Choice**: the real app is verified from the logs and
from a self-test mode that runs the main flows through the same Tauri commands (introduced with
the vertical slice of F3); the look is compared with the boards in the browser, on the fake backend.

### A-012 — WAVs are read in place, other formats from a decoded cache (2026-11-11, F1)
PRJ-3 asks to copy files into the package; the engine must read them in streaming with random access (ENG-1).
Alternatives: convert everything to an internal format at import (doubles the space); read every format with
symphonia with seek. **Choice**: the copy in `audio/` is identical to the original (name = BLAKE3 hash, read-only); the
WAVs (PCM 16/24/32, float 32/64, EXTENSIBLE, RF64) are read directly with our own parser (`core::wav`);
FLAC and AIFF are decoded once into `cache/pcm/<id>.f32`, regenerable. The peak pyramids
(`cache/peaks/`) start from blocks of 64 samples with a factor of 4 between levels. Requirements: PRJ-3, PRJ-4, ENG-1, TRN-3.

### A-013 — Our own FLAC encoder instead of `flacenc` (2026-11-11, F1)
Files produced by `flacenc` 0.5.1 are read neither by symphonia nor by the macOS system decoder
(`ExtAudioFileRead` −50), while a FLAC produced by `afconvert` is readable. **Choice**: in F8 a FLAC encoder
is written in Rust (STREAMINFO, fixed blocks of 4096, fixed predictors of order 0–4, partitioned Rice coding, CRC-8
and CRC-16), verified by re-reading the files with symphonia. Until then the import test covers WAV and AIFF; the
FLAC part of PRJ-4 is added with the encoder. Requirements: MAS-7, PRJ-4.

### A-014 — Schema migration tested on a draft v0 (2026-11-11, F1)
v1 is the first public version of `project.json`: no older projects exist. To test the PRJ-8 mechanism
a draft "v0" is defined (without `ai` and `snapshots`, tempo as a flat `bpm` field) that
`package::migrate` brings to v1; a document newer than the current version is rejected with a clear message.

### A-015 — A parameter command changes a single value (2026-11-11, F1)
The property test on the history found that setting the gain of an EQ band enabled it as a side effect,
and the inverse did not undo it. **Choice**: `SetParam` changes exactly one value; enabling
or disabling a band is a `SetTrack`. When the user moves a disabled band, the UI sends both
changes in the same gesture. Requirements: HIS-1.

### A-016 — Mute and solo in the document, outside the history (2026-11-11, F1)
HIS-6 excludes mute and solo from the history, but P2 wants the same project to give the same render. **Choice**: mute
and solo live in the document (they are saved and rendered) and change with `Document::apply_view`, which creates no history
steps. Automated mute is a lane like the others.

### A-017 — Clip snapping is computed in the UI (2026-11-11, F1)
CLP-3 (snap to grid, edges, cursor, sections) is needed at 60 fps during dragging. **Choice**: it is interaction
geometry and lives in the UI (verified with vitest); the core receives only the final position of the gesture and applies
CLP-4. It is not duplicated domain logic: the core does no snapping. `req-phases.json` assigns CLP-3 to F6.

### A-018 — Short internal ids, never reused (2026-11-11, F1)
**Choice**: ids are a prefix plus a project counter (`t3`, `g7`, `r1`, `c12`, `s2`), never reused.
Renumbered neutral ids are still sent to the model (§5.5).

### A-019 — Own processors: chosen forms (2026-11-11, F2)
FOUNDATION forbids the GPL libraries of the PoC; compressor, reverb, delay, saturation and limiter are our own
(`docs/app/dsp.md`). **Choices**: feed-forward compressor with a peak detector linked across the two outputs, quadratic
knee (Giannoulis et al.) and ballistics in dB; Freeverb-type reverb (8 comb + 4 allpass per channel, known
tunings) calibrated on the return levels measured on the PoC (±1.5 dB); stereo delay with a gliding time (50 ms) and
fractional reading; saturation `2·tanh(u/2)` at 4x, because plain `tanh` aliased at −46 dB on the 15 kHz test;
257-tap Kaiser FIR oversampler with an integer latency of 64 samples (ENG-5); true peak limiter with the PoC's 4x
detector, hold and 2 ms moving average (the ceiling holds by construction) and an internal margin of 0.1 dB. Requirements: FX-3 … FX-6,
MAS-1, MAS-3.

### A-020 — 32-sample parameter grid anchored to time (2026-11-11, F2)
AUT-6 asks for updates at most every 32 samples with interpolated coefficients, ENG-4 for invariance from the block
size. **Choice**: the engine evaluates automation and smoothing on a 32-sample grid anchored to frame 0
of the timeline; gains follow a per-sample ramp inside the sub-block, filters and EQ recompute the
coefficients at every sub-block; processors with internal smoothing do it per sample. This way 64 and 4096 samples
per block give the same result.

### A-021 — Saturation performance risk (2026-11-11, F2)
The 257-tap oversampler costs about 500 multiplications per sample and channel. With 32 stereo tracks and saturation
active everywhere it could weigh on NFR-1. **Choice**: measure at the F3 benchmark; if needed, reduce the filter
(80 dB stopband with fewer taps) or process the graph on multiple threads.

### A-022 — Two-stage half-band oversampling and rational `tanh` (2026-11-11, F3; partly supersedes A-019)
The 257-tap FIR of A-019 cost 400 ns per sample (NFR-1 benchmark at 137%). **Choice**: two polyphase half-band 2x
stages in f32 (56 and 14 odd taps, Kaiser β 8.2 and 7.9, stopband ≥ 80 dB verified on the coefficients),
overall latency fixed at 64 samples (a multiple of the 32 grid, A-020) with one sample of delay; the
`2·tanh(u/2)` shaper uses Lambert's continued fraction (error ≤ 1e-4 up to |x| 4.97, then saturation at ±1)
applied to the four oversampled points together. Cost: about 65 ns per sample. Requirements: FX-4, ENG-5, NFR-1.

### A-023 — The NFR-1 gate is on the 99th percentile of block time (2026-11-11, F3)
FOUNDATION asks that the callback use "at most" 30% of the period. A benchmark on a non-real-time thread also measures
operating system preemptions in the maximum. **Choice**: the gate is the p99 of block time (≤ 30%); mean and
maximum are printed. The benchmark covers two scenarios: full chain on 32 stereo tracks with one group and sends to
two returns, and the same with fast automation of fader, pan, high-pass, frequency and gain of all the bands.
Current measurement: mean 9%, p99 14%, maximum 20%. Requirements: NFR-1.

### A-024 — Auxiliary threads for the real-time graph (2026-11-11, F3)
Even after A-022 the callback on a single thread remained at 28–31% of the period. **Choice**: the independent nodes of a
graph level are split between the audio thread and up to 3 auxiliary threads (`stemwise-engine::pool`). No
allocation or lock on the audio thread: work is assigned with a single atomic word (epoch and index) and a
compare-and-swap, the audio thread works too and then spin-waits; idle auxiliaries sleep and are
woken with `unpark`. Accumulations toward destinations stay serial and in fixed order: the result is identical
to the single-thread path (same hash). Residual risk: an auxiliary descheduled in the middle of a node delays the block;
recorded in `VALIDATION.md` (listening with many tracks). Requirements: NFR-1, NFR-2.

### A-025 — Mute, solo and bus fader without rebuilding the graph; fades on the transport (2026-11-11, F3)
The F3 review found that mute, solo and bus fader went through a graph rebuild (gap of
silence, cut tails). **Choice**: mute is the `mute` parameter (one-cell ramp); solo is a mask
computed on the control side and applied with a ramp (`SetSolo`, view state); the bus fader is a parameter.
True structural changes (adding a track, moving a clip) replace the graph with a 128-sample fade-out,
a short buffer-filling pause (a few tens of ms) and a fade-in;
the same for pause, locate and loop. Solo follows solo-in-place logic: returns and groups that receive them
stay audible; a soloed return keeps receiving sends; the sidechain key is taken before solo.
Requirements: ENG-3, TRK-2, FX-8, TRN-1.

### A-026 — Flush-to-zero on the audio thread (2026-11-11, F3)
**Choice**: the `cpal` callback sets FZ (aarch64) or FTZ/DAZ (x86_64) on its own thread at the first call, so
denormals cost nothing even on Intel Macs (P-002). Offline render does not set it: its determinism is per
build and thread, and the difference lies only in the denormal range (inaudible).

### A-027 — Project analysis lives in the `stemwise-ai` crate (2026-11-11, F4)
§5.1 wants `analysis` to depend only on `dsp`, but ANA-14 asks to measure track content after clip editing
(engine code) and ANA-15/16 use sections, groups and roles (core). **Choice**: `stemwise-analysis`
contains only the signal measurements (PoC port, parity with the oracle); the analysis of a project (reading the
tracks with `TrackStream`, cache, sections, families, document for the model with neutral ids) lives in
`stemwise-ai::analysis_doc`, which may depend on core, analysis and engine.

### A-028 — Analysis cache and analyzed mix (2026-11-11, F4)
**Choice**: every track has a key (BLAKE3 of clips, media, channels, sections, sample rate and song end): chain
parameters do not change it, clip editing does (ANA-14). The cached result includes the measurements,
the per-section measurements, the pre-gain toward −23 LUFS and the per-band powers of the frames (for shares and masking,
computed on the pre-normalized stems as in PoC D-028). The analyzed mix is the raw sum with the center law
and the trim to −3 dBTP (the PoC's `baseline`). Tracks are analyzed on at most 4 threads to keep
memory under NFR-5.

### A-029 — Onsets in f64 (2026-11-11, F4)
librosa computes STFT, mel bank and decibels in f32; the Rust port works in f64. On the 22 oracle cases the onset
density matches within ±0.05 onsets/s, so there is no need to emulate f32.

### A-030 — The fake decider (2026-11-11, F5)
AI-9 asks for a deterministic fake decider with the PoC rules extended. **Choice** (`stemwise-ai::decider`): the
PoC rules (gain, pan, high-pass per role, EQ cuts on collisions with priority tracks,
compressors on vocals and bass, correction toward the reference from the second iteration, word-based rules for
revision) extended with: bass sidechain from the kick, light saturation on bass and electric guitars, drum
group −1 dB, delay at a dotted eighth of the tempo (375 ms without a tempo), vocals +1.5 dB in choruses and more reverb
in the bridge, words for the delay. The rationales follow the language of the brief: Italian if the brief contains at least two
Italian function words, English otherwise (AI-13). The fake master proposes match EQ if there is a reference,
light shelves where the mixdown deviates from the profile, a compressor if the LRA exceeds 8 LU, mono below 120 Hz if the
bass correlation is below 0.6, and reacts to "louder", "brighter", "wider".

### A-031 — OpenRouter request (2026-11-11, F5)
**Choice**: `POST /chat/completions` with `response_format` `json_schema` and `strict: true` (automatic fallback to
`json_object` when the provider rejects the schema: 400/422 that mention schema or response_format), `reasoning.effort`
from the configuration, `usage.include` to get the cost in the response, header `X-Title: Stemwise` (no personal
data). The key lives only in the header and in the keychain; the body recorded in the log does not contain it. The
request runs on a separate thread: cancellation returns immediately (AI-12) and the abandoned call counts with the
cost estimated from the prompt tokens.

**Update after the F5 review.**
- The schema sent to the model is built per project (`plan::mix_plan_schema_for`): every editable id is a
  required property, sends and per-section offsets are listed per return and allow `null`, the type of
  each return is fixed, every limit in `LIMITS` is inside the schema. It is the "strict" subset of the
  OpenAI-compatible providers, like the PoC schema (D-039).
- The fallback to JSON mode stays remembered for the whole client session.
- On retry the wrong response comes back as an `assistant` message, followed by the errors (as in `llm.py`).
- Markdown fences around the JSON are stripped.
- `finish_reason: length` and an `error` inside a 200 response are errors, with their consumption.
- Calls cancelled during the request (`Abandoned`) and those dropped by a network error count the prompt
  tokens at list price, marked as an estimate.

### A-032 — Cost estimate and cap (2026-11-11, F5)
**Choice**: input tokens = message characters / 4 for every iteration, plus the previous plans in the history;
output tokens ≈ 180 per track or group plus 150 (400 for the master); cost from the model's configurable price list
(default $1.25 / $10 per million in/out tokens, to be updated by the author). An optional project spending cap
signals when spent + estimate would exceed it (AI-16): the UI asks for confirmation (board 12e).

### A-033 — AI master evaluation deferred to the master graph (2026-11-11, F5)
The F5 AI master decides on the mixdown measurements and proposes a validated plan, applicable and undoable; measuring the
resulting master (LUFS, true peak, distance from the reference) requires the F8 master graph and is added there
to the flow (MAS-6).

### A-034 — Scope, returns and locks in revision (2026-11-11, F5, after the review)
**Context**: AI-4 and AI-5 speak of tracks and groups; returns are shared by all tracks.

**Choices**:
- A request with a scope (some tracks) leaves all returns untouched: changing the reverb would also change the
  tracks outside the scope.
- A locked return is context only, like a locked track. The model sees `locked` on returns and on everything
  outside the scope, both in the analysis and in the track list.
- Locking a group protects the group's settings, not those of the tracks it contains: the lock is per
  channel, like the lock row in the header. Whoever wants to protect the tracks locks them one by one.
- Applying preserves the returns' bypass and the delay's musical sync if the tempo does not change.
- A value the author set outside the assistant's limits (a fader at −40 dB, a send at −3 dB) arrives
  exact in the starting plan. It is accepted if the model sends it back unchanged; limits apply to what changes.
- Per-section offsets already written by the assistant are rebuilt from its lanes (mid-section value
  minus the static value). This way the revision preserves them. A parameter without offsets does not receive flat lanes.

### A-035 — The AI master does not change the target (2026-11-11, F5, after the review)
The target (LUFS and true-peak ceiling) is the author's choice. The AI master proposes the chain; the plan's `target` is
reset to the current target, except in a revision in words, where the author may ask for another
level. The master's `input_gain_db` is limited to ±12 dB: FOUNDATION does not fix it, and ±12 covers the typical
difference between a −3 dBTP mixdown and any target, with the limiter guaranteeing the ceiling.

### A-036 — Track names toward the model (2026-11-11, F5, after the review)
FOUNDATION allows track names and forbids file names and title. A track's default name comes from the
file name, which often contains the title or a take number. **Choice**: before reaching the model every name
loses occurrences of the project name (if at least 3 characters long) and leading numbers; an empty name becomes
"Track" (`analysis_doc::model_name`). The name in the project does not change.

### A-037 — Exported AI contracts (2026-11-11, F5, after the review)
`app/schemas/` contains `mix_plan.schema.json` (generic form) and `master_plan.schema.json` (with the limits), plus
those of the core. The full schema of a project is generated at call time. The `analysis` v2 document remains
defined by the code that builds it (`analysis_doc::model_document`) and by the tests that check its fields, neutral ids
and token budget: it is a JSON object without a Rust type to derive a schema from, and duplicating it by hand would make it
diverge.

### A-038 — UI changes are data: `Edit` (2026-11-11, F6)
**Context**: §5.6 wants a UI without domain logic. The changes to clips, sections, tracks and automation
are many and must remain one step each.
**Choice**: an enumeration `stemwise_core::edit::Edit`, exported as a TypeScript type, and an `edit(edit,
label)` command: the core computes the commands (trimming under a moved clip, routing checks, lane limits,
freehand drawing simplification) and applies them as a transaction. For drags that must
sound while they move (moved curve, new send, bus fader) there is `gesture_edit`: the gesture returns to its initial
state and reapplies the edit (`Document::gesture_replace`), so the result does not accumulate errors. Clips
instead are dragged in preview in the UI and become a single `edit` on release. The fake backend has a
simplified copy of the same edits, only for Playwright and visual verification. Within a gesture an already
touched parameter is updated in place (frequency and gain of a band alternated remain one compact step). Calls to
Tauri are queued, one after the other, so the start, values and end of a gesture arrive in order.

### A-039 — Real-time spectrum of the selected track (2026-11-11, F6)
FX-7 asks for the track's spectrum in the EQ editor. **Choice**: the audio thread copies the post-fader output of the observed node
into a lock-free ring; the control side computes the 4096-point FFT of the `stemwise-dsp` analyzer
and reduces it to 96 logarithmic bands from 20 Hz to 20 kHz, sent with the meters (`MeterSnapshot.spectrum`).
Observation follows the selection (`watch_spectrum`). No FFT on the audio thread.

### A-040 — States and controls the design does not show (2026-11-11, F6)
- **Vertical zoom**: a "Track height" button next to the zoom cycles the three lane heights (TRN-3).
- **Ruler**: always in minutes:seconds (TRN-2); with a BPM, bars only give the snap grid step.
- **Automation lane**: in the header, besides the parameter selector, the current value and a button to
  hide it. The selector swaps the visible lane, without losing points; at the bottom there is "Delete this automation".
- **Automation with the mouse** (AUT-2): with the pen, click adds a point, dragging a point moves it, dragging
  on empty space draws. `⇧`+drag selects a range, `⌫` deletes it, `⌘C`/`⌘V` copy and
  paste it at the cursor.
- **AUT-3 during playback**: moving an automated control moves the curve even with the transport running.
  v1 has no write mode, so there is no alternative behavior to offer.
- **Compressor**: the five knobs of the design plus the knee in a numeric field below (FX-3 asks for it to be adjustable).
- **History**: at the bottom of the list a "Project opened" entry returns to the initial state.
- **Master button**: until F8 it shows a notice; dialog 8a arrives with the mixdown.
- **References and snapshots**: in screen 2 references are added from the brief after creation; the snapshots
  panel is completed in F7 with HIS-4.
- **Deleting a track or a return** asks for confirmation (13g) even though it is undoable, as the ellipsis of
  "Delete track…" in board 6e suggests.

### A-041 — The assistant in the app (2026-11-11, F7)
- **One call at a time.** While a proposal is under review another is not started: it must be applied or discarded.
  During the call the workspace stays usable (AI-12) and the call starts from the project state
  at the time of the request.
- **A/B.** The engine loads the version to listen to (proposal or snapshot) without touching the document and without a
  history step (AI-2, HIS-4). A structural change made during listening brings listening back to the current
  mix, because the engine plays the document again.
- **Applying.** The commands are recomputed on the project as it is at the time of applying (`apply_commands`), so
  changes made during listening remain. The step carries the call's id (HIS-3), and the call receives the
  outcome (applied, partially applied, discarded) and the step label. Every call, even failed or
  cancelled, enters the cost log of all projects, and the project saves automatically (PRJ-2).
- **Estimate before sending** (AI-16). The message size is estimated from the number of tracks and sections, without
  building the analysis: it gives an order of magnitude in real time, while the real call counts the real tokens. The
  cap is set in the AI usage panel, where it also carries "Change cap…" of dialog 12e.
- **Snapshots.** Saving a snapshot adds a name to the list without a history step (the list is not the
  mix); restoring it is a step; deleting it removes the name and leaves the file in the package.
- **Brief and references** are edited from "Edit brief" in the assistant column (PRJ-7): the brief is a history
  step; adding or removing a reference too.
- **Key** (AI-11): a minimal dialog (key in the keychain, or offline demo) until the full settings
  arrive in F9. The self-test uses the offline demo and an in-memory key; with `STEMWISE_DATA` it uses a
  temporary data folder.

### A-042 — The master phase in the app (2026-11-11, F8)
- **Master graph.** The engine loads a listening project with a single neutral stereo track that plays the
  mixdown (or a reference), and the master chain on the bus (`Graph::set_master_chain`). Parameters arrive on a
  queue of their own made of `Copy` values (`MasterParams`), applied without rebuilding the graph or allocating. Every
  configuration has the same latency: disabled blocks are replaced by delay lines (refined in A-043).
- **Target loudness** (MAS-2). A gain before the limiter, found offline on the mixdown: one pass
  without the limiter to measure, then corrective ones with the limiter (up to two; from A-055 up to six, guided by the slope). It is recomputed in a thread after every change,
  and an epoch discards superseded results. It is deterministic (MAS-8).
- **Limiter.** Its bypass applies only when listening: export always limits, so the ceiling is guaranteed on the file
  (MAS-3). The file is then re-decoded and measured with 4x true peak; an AAC above the ceiling is re-encoded with the
  margin needed (MAS-10).
- **Loudness-matched A/B** (MAS-5). The mixdown plays with the chain bypassed and the gain that brings it to the loudness
  of the master; a reference plays as a track of the listening project with the same criterion. The AI master
  proposal is listened to in the same way.
- **Mixdown** (MDN-1, MDN-4). It is a float WAV file in `mixdowns/`, with attenuation-only trim down to −3 dBTP and
  the mix hash. Recording it is not a history step: the file exists anyway, and redoing it creates the next
  number.
- **Deterministic exports** (MAS-8). The `bext` chunk receives the date from the caller; ID3 frames are sorted
  by identifier, because `lofty`'s generic conversion goes through a map. Two identical exports give the same
  bytes for WAV and FLAC; AAC depends on the system encoder.
- **Resampling** at export: before the chain, so the limiter works at the file's sample rate.
- **Master control limits in the UI.** EQ ±12 dB, ratio up to 10 and similar: the tighter limits of §5.5 apply to
  the AI plans. (ISRC: see A-043, now validated in the core.)
- **AI master partially applied**: per module (input, eq, match, compressor, width, saturation, limiter,
  target), as a single master history step.
- **Cover**: copied into the package (`masters/cover.jpg|png`) and written in every format.
- **Encoders and tags** are in the `stemwise-export` crate. Three subagents wrote them in parallel on
  separate files; the orchestrator reran the checks.

### A-043 — F8 review fixes (2026-11-11, F8)
The independent review found 3 blockers and 9 findings. Fixes:
- **No steps in the master** (B2, ENG-3).
  - Continuous elements track the new value every 32 samples, with a 10 ms constant. They are:
    - the gains: input, loudness, A/B, output;
    - the EQ bands, which pass through 0 dB to change type;
    - the match EQ and the width.
  - Turning a block on or off, and the A/B, is a crossfade.
  - Saturation and limiter also work when off and the delay lines always run. A fade therefore does not make
    old audio audible (N8) nor an empty look-ahead.
  - With constant parameters (render, export) the tracking has already arrived: nothing changes compared to before.
  - The test `master_changes_glide_instead_of_stepping` measures the maximum jump between samples after every kind of
    change.
- **Numbered parameters** (S5). Every `set_master` has a sequence number and every graph remembers the number it was
  built with. When a swap arrives with older parameters, the audio thread applies the most recent ones to it.
- **Ceiling on the written file** (S2, MAS-3, MAS-10).
  - WAV and FLAC are re-read with the decoder (symphonia for FLAC: an independent decoder re-reads our own
    encoder, cf. N4) and measured.
  - If they exceed the ceiling the floats are scaled by the excess plus one quantization step, then rewritten.
  - After 6 attempts the export fails rather than deliver a file over the ceiling. The same holds for AAC.
  - Reports are measured on the decoded files.
  - The test covers all blocks on, content with inter-sample peaks, ceilings at 0 and −0.1, sample rates up to
    96 kHz and 16, 24 and 32 bit.
- **AAC up to 48 kHz** (B3). The AudioToolbox encoder does not go beyond. Above that rate the dialog does not offer
  AAC (MAS-10) and the pipeline rejects the settings before writing anything (`pipeline::check`).
- **Export cancellation** (S3). The cancellation request goes through the progress callback; files already
  written by that export are removed.
- **References in the A/B** (B1).
  - At the first measurement every reference is converted once into `cache/listen-<id>-<rate>.wav` (stereo float at
    the project's sample rate); the imported file stays as it is. This way MP3, M4A and different rates sound right.
  - The master cache also depends on the list of references.
  - If the listening switch fails, it returns to the previous choice.
  - The self-test in the real app listens to a 44.1 kHz AAC reference in a 48 kHz project.
- **Loudness-matched A/B without clipping** (S7).
  - The gain that brings the mixdown and each reference to the master's loudness is computed. If with it one of them
    exceeds −1 dBTP, all the A/B sources, master included, play lower by the same amount
    (`ab_trim_db`, shown next to the selector).
  - The trim is applied after the limiter, so it does not change the master's sound.
  - The AI master proposal plays at its own loudness, measured together with the proposal.
- **Undo and mixdown** (S1, MDN-2).
  - `SetMaster` always preserves the current mixdown, both when applying and in the inverse. Only the new command
    `SetMixdown`, recorded outside the histories, changes it. Master undo and redo no longer bring back an
    old mixdown, and an old copy of the state cannot overwrite a bounce.
  - In the Master phase the engine switches immediately to the new mixdown.
- **Jobs tied to the project** (S6). The session has an `open_id` that changes at every open and close. Bounce,
  export and AI master remember it and discard the result if the project is no longer the same. The loudness epoch
  does not restart from zero: an old result never matches a new request.
- **Loudness thread** (S4). The decision to stop and the `loud_running` flag change under the same lock:
  a change that arrives later finds the flag off and starts a new thread. The gain computation happens only once
  per change (N2).
- **ISRC and chain validated in the core** (S8, S9).
  - `MasterState::check` enforces the control limits (`master_limits` table), the sample rates, the bit
    depths, the bit rates and the ISRC on every `SetMaster` from the UI.
  - The ISRC is valid in compact form or with all hyphens, and it is the same rule the tags use (moved to the core).
  - The dialog shows the state with the same rule and does not save an invalid ISRC.
- **The `stemwise-export` crate** (S9) does not appear in §5.1. It contains encoders, tags and the export pipeline. It depends on
  `engine`, which provides the chain and render, and on `ai`, for the DTOs of the master proposal shown next to the master
  state. The alternative, putting everything in `engine`, would have brought `lofty` and the AudioToolbox FFI into the
  audio engine. It is an addition to the architecture: the boundaries of §5.1 hold (no crate depends on the app, the core
  depends on none).
- **Empty name at export** (N9). The file is named `Untitled`. A file with the same name in the folder is
  replaced, as the hint on the field says.
- **They stay as they are, with reason:**
  - N1: `bext` date and time from the clock, already declared in A-042.
  - N5: a single bit depth for WAV and FLAC; FLAC stops at 24.
  - N6: the loudness gain is limited to ±24 dB; "off target" remains visible in the measurements.

### A-044 — Polish: shortcuts and settings (2026-11-11, F9)
- **A single shortcuts table** (`ui/src/shared/shortcuts.ts`, UI-3). It is used by the keyboard handlers, the
  tooltips and the list in Settings › Shortcuts (SET-1, read-only as v1 wants).
- **Differences from design 11d.**
  - ⌘S saves the project, because that is the habit in every macOS application. Snapshot moves to ⇧⌘S.
  - "Clip gain ⌘G" is not there: clip gain already has the context menu and dragging.
  - ⌘, is added for Settings.
- **Settings** (screen 11).
  - Audio: device, buffer, engine load, output sample-rate conversion. "Play test tone" is not there: the
    engine plays only projects and the test signal would require a dedicated path in the audio thread; it is verified
    by listening to the project (VALIDATION).
  - AI: decider, key in the keychain (never shown: only whether it exists is visible), model, effort, temperature, spending
    cap of the open project, what is sent, totals across all projects (with the number of projects from the registry).
  - Appearance: user preferences, not project ones, saved in the webview's `localStorage`, with default values if
    missing. They cover lane height, name and waveform on clips, gain badge, position
    in bars (only with a tempo), peak hold, numeric peak on the strips and reduced animations.
  - The ruler stays in minutes and seconds (TRN-2).
  - Settings are reached from the gear icon in the Mix and Master bars, from the welcome screen and with ⌘,.

### A-045 — Polish: states, accessibility, performance (2026-11-11, F9)
- **Missing files** (UI-6, 13d).
  - The core knows which files are missing from the package (`package::missing_media`); the project event carries them to the UI.
  - The engine plays clips without files as silence instead of rejecting the whole graph. The bounce, on the other hand, refuses
    while files are missing, because a mixdown with muted clips would be wrong without saying so.
  - "Locate…" (`media::relink`) imports the chosen file as at the first import. If the content is the same it goes back to
    its place and the document does not change. A different file with the same channels and the same duration becomes a new
    media for the clips (`RelinkMedia`, an undoable step). Anything else is rejected without leaving files behind.
- **Disconnected device** (13d). The UI polls the output state every second while a project is open
  (the device can disappear even when stopped). A banner under the bar offers another device and the settings.
- **No network** (13f). A "network unavailable" error from the client puts, in place of the key box,
  "No connection to OpenRouter" with "Try again" and "Use offline demo". The toast remains for the detail.
- **Empty states** (13a, 13b) as in the design: timeline with drop zone, mixer, inspector, assistant, brief,
  history, section lane, sends without returns.
- **Waveforms on open** (13c). If the cache is missing from the package (a copy does not carry `cache/` with it), a
  thread rebuilds it file by file; every clip is redrawn when its file is ready.
- **Accessibility** (UI-7). The test uses axe-core (WCAG 2 A and AA, serious and critical violations) on welcome, Mix,
  Settings, AI usage and Master, plus keyboard on the continuous controls. Three deviations from the design, because
  FOUNDATION wins:
  - a disabled chain block no longer lowers the text opacity, but has an outline and a struck-through value;
  - the red of the error tags is lightened toward the text color;
  - the master modules are buttons with a pressed state, not tabs: a tab cannot contain the LED.
  The rows of the AI usage table have cells with a role.
- **Memory** (NFR-5). The analysis for the assistant reached 2.5 GB on 32 five-minute stems. Now it is at 1.0 GB:
  - integrated loudness, LRA and true peak work in streaming on the signal, without `f64` copies: same blocks as
    pyloudnorm, values equal to within 1e-9 LU (test against direct computation, oracle parity unchanged);
  - the mix and family sums are in `f32`;
  - 2 stems are analyzed at a time instead of 4 (ANA-17 stays at 13 s on 30).
- **Performance** (NFR-3, NFR-5).
  - `make bench` measures the import (16 stems of 4 min, 1.7 s out of 15) and the memory of a project of 32 five-minute stems,
    imported, played for 20 s by the engine in real time and analyzed (1.0 GB out of 1.5).
  - A Playwright test measures the 60 fps of the timeline with 32 tracks visible during playback: 60 fps in Playwright's
    WebKit; the test tolerates at most 5% of frames over 33 ms (0–1 measured out of 180).
  - Cold start is the time from process start to the first UI paint (`ui_ready`). The self-test measures it in the
    app; under `tauri dev` it includes Vite and the debug build, so it is measured on the packaged app in
    F10.

### A-046 — Delivery: package and verified policies (2026-11-11, F10)
- **`.dmg` without the custom window.** Tauri's bundler opens the volume in the Finder via AppleScript to
  arrange the icons. Without a reachable Finder session the request times out and the `.dmg` is not created.
  - First attempt: `make build` as it was, failed with an AppleEvent timeout.
  - Fallback: `CI=true`, with which the bundler skips that step. The image content is the same, without the icon
    layout; `hdiutil verify` reports it valid.
- **Signing.** The app has only the linker's ad hoc signature: without certificates, the first launch requires right-click ›
  Open (VALIDATION 23). Signing and notarization are left to the author.
- **Cold start** (NFR-3) measured in the packaged app with the self-test: 969 ms from process start to the first
  paint of the interface. The full self-test passes in release too.
- **NFR-7 and NFR-8 as tests on the code.**
  - Network: only the OpenRouter client uses the network, and only `stemwise-ai` depends on an HTTP library. The client is created
    only where the user asks for the assistant. No telemetry.
  - Webview: no network API in the UI sources, and the CSP limits connections to the app and to IPC.
  - macOS: conditional compilation, frameworks and FFI live only in the AAC module, which has the same interface and a
    fallback elsewhere.

### A-047 — Final review fixes (2026-11-11, F10)
The final independent review found 2 blockers and 6 findings. Fixes:
- **Missing files never treated as silence outside listening** (B1).
  - Offline render (bounce, AI loop evaluation, CLI) and analysis refuse to work with missing files:
    `Missing` error, no analysis cache written.
  - The mix assistant refuses the request and the Auto mix button is disabled while the timeline banner
    reports missing files.
  - Only the live engine plays them as silence, with the clips hatched.
- **The title does not reach the model even in family names** (B2): `model_name` applies there too; the test
  uses a group that contains the title.
- **"Locate…" and snapshots** (C2).
  - Relinking to a different file is `AddMedia` plus `RelinkMedia { from, to }`, which only repoints the clips.
    The old entry stays in the project: a snapshot that names it finds it again and flags it as missing.
  - `missing_media` also flags clips that name a media not in the list; in the menu they appear as "Unknown
    file", not to be relinked.
  - `RelinkMedia` is in the property test strategy.
- **No silent conversion** (C3, PRJ-5). A file at another sample rate is accepted only if the missing file
  had also been converted at import. A rejection also removes the caches of the attempt, and the message gives the frames.
- **Waveforms after "Save as"** (C4): the copy rebuilds its caches as at open. Pending clips
  show the `is-loading` state of design 13c (C6).
- **Locate one file at a time** (C6, design 13g). The design shows a dialog with search in a folder and several files
  together. Here there is a menu of missing files and a choice per file: the operation is the same and the check of shape and
  content stays file by file. Search by folder is not in v1.
- **Caches written safely** (N1): temporary files have a name unique per process and call. Files
  converted at import also become read-only and do not overwrite an identical file already present (N7).
- **Accessibility** (N6): clips are reached with Tab and selected with Enter; axe also runs on the proposal under
  review and on the export dialog.
- **They stay, with reason:**
  - N2: the `f32` analysis sums depend on thread order at the 1e-7 level. The document for the
    model rounds to 0.1 dB or less, and the render is untouched.
  - N4: NFR-3 (import) and NFR-5 are verified with the benchmark (`make bench`), like NFR-1 and NFR-4. The memory benchmark
    does not cover the AI evaluation loop (three copies of the stereo mix: about 350 MB more over 5 minutes, below the limit).
  - N8: the rule "network only on user action" is verified at the client creation point, which is only one.

### A-048 — New project: files chosen before the project exists (2026-11-11, after F10, reported by the author)
- **Defect.** In the "New project" screen, adding files gave "no project is open": the command
  `import_candidates` required the open project, which is created only with "Create project".
- **Why the tests did not see it.**
  - Playwright's fake backend examined files even without an open project.
  - The self-test in the real app started from a package already created by the CLI.
  - The real creation path was covered by neither.
- **Fix.** The command receives `adding` and a reference sample rate:
  - for "Add tracks" it uses the open project's sample rate;
  - for a new project it uses that of the first file already in the list, even between one addition and the next (PRJ-5), or none.
  The fake backend now rejects "adding" without a project, like the shell.
- **Coverage.**
  - The self-test in the real app (packaged too) closes the project, lists the files in two groups, creates the
    project, imports, then adds a track to the open project.
  - A core test verifies the sample-rate comparison between groups.
- **60 fps test.** It failed once in `make check` (12 slow frames out of 180) because it measured in parallel with
  three other browsers. Now it runs alone, after all the others (Playwright project `perf`); the threshold is unchanged.

### A-049 — Fixes from the author's manual validation (2026-11-11, after F10, reported by the author)

**Context.** During manual testing the author reported eight defects.

1. In project creation, references could not be added.
2. Switching from Mix to Master always asked for a new mixdown.
3. In the master the space bar did not start playback.
4. Drags selected the interface text.
5. Tracks could not be dragged (reorder, into a group) nor grouped from a selection.
6. The settings icon looked like a sun.
7. There was no place to see and change the project settings (name, brief, references).
8. The bus meters at the top widened and narrowed during playback.

**Choices.**

1. **References at creation.** They are chosen (click) or dragged onto the dedicated box before the project exists. They are added right after the track import, because they are converted to the project's sample rate, which the first file fixes. To tell a drop on the references box from one on the tracks, the file drag event now carries the pointer position.
2. **Mix → Master.** If a mixdown exists the master is always entered already built. If the mixdown is out of date, the master says so with the existing banner and offers to redo it keeping the settings (MDN-2). The bounce dialog remains only for the first mixdown. In addition the mix fingerprint (`package::mix_hash`) ignores the name, tint and role of tracks and the original name of the files: renaming a track no longer makes the mixdown stale.
   - Side effect: the mixdowns of existing projects turn out stale once, because of the change in the fingerprint formula.
3. **Shortcuts in the master.** The master has its own handler for space, enter, Home, undo/redo (on the master history) and save. The other shortcuts in the table concern the mix.
4. **Text selection.** Disabled across the whole app (`user-select: none`). Fields, the raw JSON, the AI summary and rationales and the project path remain selectable.
5. **Tracks.**
   - Headers accept ⌘-click (adds or removes) and ⇧-click (range).
   - ⇧⌘G and the track menu group the whole selection.
   - Dragging the header moves the track or the selection. A line shows where it will go; on the center of a group header the track goes inside.
   - The core has a new edit, `Edit::PlaceTracks { tracks, before, output }`: reorder and routing in **a single undo step**, with the check that a group does not end up inside itself and that a return does not enter a group (also valid for `add_group`). The UI does not duplicate domain logic.
6. **Icon.** Gear instead of rays, in the design generator (`design/tools/lib.mjs`) and in `iconPaths.ts`.
7. **Project settings.** A "Project settings" dialog replaces the brief dialog and opens from the project name in the top bar, in Mix and in Master. It shows name, sample rate, tracks, position, brief and references (adding by click or drag, removal). The name is changed with the new `Edit::RenameProject` (`Command::SetName`, one undo step); the package folder does not change. References appear by name in the assistant column.
8. **Meters.** Every value in the bar (true peak, loudness, load) has a fixed space: bars and neighbors no longer move.
   - **Correction (the author's second report).** The first version gave the bars the class `bars`, which `design/screens/screens.css` already uses for vertical bars; so the horizontal meters had zero width and vanished. The test back then only verified that they did not move. Now the class is `bus-bars` and the meters live in a common component (`shared/BusMeters.tsx`), also present in the master bar (master output).
   - **Space in the master.** The label "Listen at the same loudness" becomes "A/B", with the explanation in the tooltip. Engine load stays only in the mix. Below 1360 px the master hides loudness and the mixdown label, which are already in the loudness panel and in the waveform card.
   - **Test.** Now the test verifies that the bars are at least 40 px wide and fill during playback, in Mix and in Master, at 1440 and 1280 px, and that the top bar does not overflow. Putting the wrong class back makes the test fail.

**Verification.**
- New tests: `dragged_tracks_move_in_one_step_into_and_out_of_groups` (core) and the rename check in the mixdown test (engine).
- Four new or extended Playwright tests: references at creation by click and drag and project settings; tracks dragged into and out of a group and group from selection; no text selection and still meters; Mix↔Master without bounce and space in both phases.
- In the real-app self-test: rename without a stale mixdown and re-entering the master without a bounce, mix change with the master opening anyway, reference and new name on the just-created project, tracks moved into and out of a group with playback.

### A-050 — Master ready quickly: measurements kept in the project, no repeated computation (2026-11-11, after F10, reported by the author)

**Context.** With a real project (13 stereo stems at 96 kHz, 3:04) the author saw the master take seconds to become usable. The phase switch itself takes 10–85 ms. What is slow are the background analyses:
- waveform and mixdown measurements: 9 s, 20 s with a 44.1 kHz reference;
- loudness on the target: another 20 s;
- everything redone at every project open.

Profile on the real mixdown:
- `measure_mix`: 3.1 s, run twice;
- master measurements: redid true peak (2.1 s), integrated loudness and range that `measure_mix` already had;
- one pass of the master chain: 3.8 s; the loudness search makes 2–3, plus a final pass equal to the last;
- reference conversion: 5.5 s every time, against 0.05 s to re-read the already converted copy;
- re-reading the mixdown after the bounce: 0.06 s.

**Choices.**
1. **Measurements kept in the package** (`cache/master/`): mixdown measurements (key: number, mix fingerprint, frames, sample rate), of every reference (key: file id and sample rate) and the last 16 loudness results. The loudness key includes chain, target, mixdown and band profiles. They are derived data: a file that does not match or cannot be read is recomputed, and deleting the folder costs only extra waiting. Files are written by rename, never half-written.
2. **Bounce samples reused.** The bounce passes its samples to the cache of the just-written mixdown, without re-reading the file. The gain is small (56 ms read), but the analysis starts immediately, while the author reads the bounce result.
3. **Converted copy of the reference.** If it already exists in `cache/listen-…wav` that one is re-read, without reconverting. Mixdown and references are measured in parallel.
4. **Loudness.**
   - The search (`loudness_search`) can start from the gain found for the previous state. This skips the measurement pass without the limiter, and a small change usually takes one pass instead of three.
   - The last pass becomes the result, without repeating it. To do this the gains are rounded to 0.01 dB before every pass, not after.
   - For the first state measured in a session it starts from an estimate: target loudness minus that of the mixdown, minus the input gain.
   - Export always searches from scratch: same project, same file (MAS-8).
5. **Measurements.** `measure_mix` computes spectrum, true peak of each channel, loudness and range on separate threads, with the same values as before. The master measurements use those of `measure_mix` instead of recomputing them.

**Side effect.** The rounding moved changes the export's loudness gain by a few hundredths of a dB compared to the previous version. The result remains deterministic.

**Dependencies.** `rayon`, already in the workspace, now enters the `src-tauri` shell too.

**Addendum.** The master preparation overlay, designed in screens 9d–9k, is still to be implemented.

### A-051 — Master preparation overlay (2026-11-11, after F10, requested by the author)

**Context.** The author asked that, on entering Master, work still in progress be covered by an overlay that explains what it is doing, and that the workspace appear only when it is really ready. The design is in screens 9d–9k (`design/screens/09-master-preparing.html`, spec in `design/README.md`).

**Choices.**
- **Contract.** `MasterStatus.prep: MasterPrep | null` as proposed by the design: number, duration and sample rate of the mixdown, how many references, weighted fraction, elapsed time, estimate of remaining time, any error and steps with state, origin ("bounce", "saved"), progress and duration. In addition `skipped_references` for banner 9i.
- **Steps.** The step plan is born in `Prep::plan` from the package: what is already saved counts as done immediately and weighs zero, and an unnecessary conversion is not listed.
  - `load_cache` reports each step as it goes.
  - The loudness search reports progress after every pass.
  - The weights are estimated on the times measured for A-050.
- **Errors.**
  - An unreadable mixdown (missing, truncated, unreadable file) stops the preparation with 9j and "Bounce again".
  - An internal error offers "Try again" (`master_prepare_retry`).
  - A failed preparation does not restart by itself at every change.
  - An unreadable reference is skipped (9h) and named in banner 9i.
- **Interface.**
  - Overlay after 250 ms and for at least 600 ms (`--wait-show`, `--wait-min`), above the empty shells of the workspace.
  - In the top bar only the phase selector stays active. Space, undo and the other keys do nothing; Esc and "Back to Mix" return to Mix without stopping the work.
  - In the Mix the Master segment shows a progress ring.
  - The "Master is ready" toast (or the error with "Show") has an action and stays until dismissed. Notifications now accept an action.
  - After a master change no overlay, but state 9k in the Loudness card. At that moment "Master with AI" and the export button wait for the measurement.
- **Tests.** Three Playwright tests (overlay, Esc and ring, toast; unreadable mixdown and skipped reference; state 9k) on the fake backend, which now simulates preparation with timed steps. The real-app self-test has a check that the preparation reports its steps and finishes.

### A-052 — Keychain read once, only when the key is needed (2026-11-11, after F10, reported by the author)

**Context.** At startup the app asked for the keychain password four times. Every assistant state sent to the interface (startup, project open, updates) read the key from the keychain just to know whether it existed (`has_key`). With the unsigned app macOS asks for the password at every read, and an "Always Allow" holds only for that build: the ad hoc signature changes with every build.

**Choice.** `LazyKeyStore` in `stemwise-ai::keys` wraps the system keychain.
- Whether the key exists is told by a small file `has-key.json` (`true` or `false`, no secret) in the app's data folder, updated when the key is saved or removed.
- The value is read only when a call to the provider starts, at most once per session, then it stays in memory.
- For a key saved by a previous version (file absent) the first state request reads once and writes the file.
- Result: startup, project opening and assistant state no longer touch the keychain. The password can be asked for at most once per session, at the first AI call with the real provider; never with the offline demo.

**Test.** `the_keychain_is_read_at_most_once_and_never_to_know_whether_a_key_is_saved` counts the reads on a fake keychain.

**Not verified.** It is not verified with the real keychain (the agent does not use keys): it stays in VALIDATION 7.

**Caveat.** The prompt at the first call disappears entirely only with a Developer ID signature (A-046).

### A-053 — Save, save as and close from the interface (2026-11-11, after F10, reported by the author)

**Context.** The author could not find how to save the project. The interface saved only with ⌘S and with autosave, with no visible command. "Save as" (PRJ-2) and project closing existed in the backend but not in the interface. The app had no menu bar of its own.

**Choices.**
- **Project menu.** The project name in the top bar, in Mix and in Master, opens a menu: "Project settings…", "Save" (⌘S), "Save as…" (⇧⌘S), "Close project".
- **macOS menu bar.** Stemwise menu (About, Settings ⌘,, Quit), File (New Project… ⌘N, Open… ⌘O, Close Project, Save ⌘S, Save As… ⇧⌘S), Edit for text fields, Window. The File commands reach the interface (`menu` event) and go through the same commands as the project menu (`project/commands.ts`). Keys reach the page first, so the interface shortcuts work as before.
- **Closing.** With unsaved changes, closing or opening another project asks "Save changes to …?" with Save, Don't save and Cancel. The confirmation dialog now accepts a third choice.
- **Save as.** Asks for folder and name with the system save panel. The copy becomes the open project and the original stays as it was.
- **Shortcuts.** ⇧⌘S is "Save as", as in Mac apps; snapshot moves from ⇧⌘S to ⌥⌘S. The single shortcuts table (UI-3) and Settings show this.

**Verification.**
- One Playwright test: save from the menu, ⇧⌘S, closing with the three choices, Open and New from the simulated menu bar.
- In the real-app self-test, "Save as".
- The native menu bar is not verified by a test: the app that starts in the self-test builds it, but clicks on the menus remain to be tried by hand (VALIDATION).

### A-054 — Playhead scrub in Mix and in Master (2026-11-11, after F10, requested by the author)

**Context.** The playhead could not be dragged, neither in the Mix nor in the Master. In the Mix dragging on the ruler already creates the loop region.

**Choices** (agreed with the author, as in Logic and similar tools).
- **Mix.**
  - Scrubbing is done by grabbing the playhead inside the ruler (within 7 px), or with ⌥ and dragging anywhere on the ruler.
  - A click elsewhere on the ruler moves the position, a drag creates the loop, as before.
  - Below the ruler the playhead line lets clicks through to clips and lanes and does not move it.
- **Master.** Dragging on the waveform scrubs; a click moves the position.
- **Behavior.** The playhead follows the pointer immediately. The position reaches the engine at most every 33 ms, plus the last one on release. During playback you hear from where you drag and resume from the last position; with the transport stopped only the position moves.
- **Master overlay.** While fading it lets clicks through: before, it blocked them for a few hundred milliseconds after the master was already ready.

**Verification.**
- Two Playwright tests: Mix (grab in the ruler, line that lets clicks through, ⌥ and drag) and Master (waveform).
- In the real-app self-test, 20 moves at 30 Hz during playback: playback continues from the last one, with signal on the bus.

### A-055 — Loudness search that also reaches loud targets (2026-11-11, after F10, reported by the author)

**Context.**
- On the real project, with the Loud target (−9 LUFS, −1 dBTP), the master stopped at −9.4 LUFS.
- The search corrected the gain as if every dB of gain gave a dB of loudness. Under heavy limiting it gives much less, and two corrective passes were not enough.
- The test tolerated 0.5 LU precisely on the loud targets, and so hid the defect.

**Choice.**
- After the first correction, the next one uses the slope measured between the last two passes. The slope is limited between 0.25 and 1, so a step never exceeds four times the error.
- The search stops within 0.1 LU (`ON_TARGET_LU`) or after six passes in all (`SEARCH_PASSES`).
- A target unreachable within ±24 dB of gain stays "Off target". On the real mixdown −7 LUFS stops at −7.8.
- The result remains deterministic (MAS-8).

**Verification.**
- The test now asks for 0.1 LU on −14, −9, −10, −8 and −7 LUFS.
- On the real 13-track mixdown: −9.06 LUFS with the Loud target (before −9.4), −14.03 with Streaming.

### A-056 — Listening at real level and target warning (2026-11-11, after F10, requested by the author)

**Context.**
- The Master always played loudness-matched (MAS-5), so the true level of the master was never heard.
- With a loud master, the master itself also played lowered to avoid clipping the other sources (`ab_trim_db`).
- A Custom target at −18 LUFS against a reference at −9.6 LUFS went unnoticed until listening to the exported file.
- The design is in boards 9l–9u and 10d–10e, decisions D26–D31 of `design/README.md`.

**Choices.**
- **Listening.**
  - "Real level" is the default mode: the master plays like the exported file, mixdown and references at their level, with no gain or lowering.
  - "Same loudness" is the previous MAS-5 behavior, on request (⇧B). The master proposal under review follows the same mode: when loudness-matched it plays at the loudness of the current master.
  - The mode lives in the session (`MasterStatus.listen_same`, command `master_listen_same`). It is not in the project and is not an undo step: every project opens at real level, and the mode stays when passing through the Mix.
- **Target warning.**
  - The target's loudness is compared with the range of the measured references.
  - More than 3 LU below the lowest there is a warning; more than 3 LU above the highest a neutral note.
  - "Match" brings the target to the nearest edge as Custom, with the same ceiling, in a single undo step.
  - "Keep" is `MasterState.target_kept` (with `serde(default)`, so existing projects open without migration), also an undo step.
  - The computation and the wording live in `ui/src/master/level.ts`. They are pure functions with unit tests, because they are also needed for targets not yet chosen (the menu) and for the report files. The core already exposes the loudness values needed (`references[].lufs`, now with `true_peak_db`).
- **Loudness card meters.** They measure what plays (the output bus), as before. At real level, the default mode, they coincide with the file. The design proposed measuring before the listening gain, but this would require a second measurement point in the engine for one case, loudness matching, which the numbers of that mode already explain.
- **Master bar.** The boards have no output meters, added after the design at the author's request (A-049). With the new controls the bar overflowed by 165 px already at 1440, so in the Master:
  - the meters keep the bars and the true peak, but not the loudness, which is large in the Loudness card;
  - spacing and padding of the segments are tighter;
  - below 1600 px the total duration next to the position and the "Reviewing a master proposal" label are hidden (it is already in the Assistant column);
  - in the narrow (compact) window the labels become "Real", "Same", "Ref A".

  The bar test verifies that it fits in 1440 and 1280 px, also with two references.
- The presets in the targets menu now use the true minus sign (−14), like the rest of the UI.

**Verification.**
- Playwright tests:
  - listening (9l–9n, ⇧B, the mode that stays between phases);
  - warning (9q–9t, menu, Keep with undo, Warn me again, export and report 10d–10e, Match);
  - note on the louder target (9u);
  - proposal A/B with B and ⇧B (9c, 9o).
- Unit tests of the logic in `level.test.ts`.
- Real-app self-test: the same mixdown passage, at real level and then loudness-matched, changes by the expected difference.

### A-057 — Focus stays where the user put it when the master overlay disappears (2026-11-11, after F10)

**Context.**
- The export test failed twice inside `make check`: the ISRC just typed disappeared from the field.
- Cause: when the preparation overlay finished fading, focus was returned to the Master segment (9d) in every case.
- While fading, the overlay lets clicks through (A-054). Whoever opened export right away and typed lost focus midway: the keys ended up on the segment and the field stayed empty.
- The case occurred in the real app as in the test under load.

**Choice.** Focus returns to the Master segment only if it had stayed on the overlay, that is, if it is now on no element (the page body or a removed element).

**Verification.** A Playwright test opens Export and types in the title while the overlay fades: the field keeps focus and holds the text. Before the fix it failed every time.

### A-058 — F11: three-algorithm reverb and compressor mix (2026-11-11, P-011)

**Context.** FOUNDATION asks for three reverb algorithms in place of the comb-based one (FX-5, FX-9) and the compressor's
dry/wet (FX-3). Choices made while building them.

**Choices.**

- **The algorithm is fixed at reverb construction.** Changing it is a structural change (`Edit::SetReverbAlgorithm`,
  the graph is rebuilt with the fade of A-024): no buffer to zero in the audio thread. The parameters stay
  as they were.
- **16-line network with a Hadamard matrix**, not 8: more density of echoes and modes at the same per-sample cost as
  one matrix multiplication (fast transform). *Bent* sign patterns at input and output: with a row
  of the matrix as the output pattern one line came back whole (peak 0.2 in the autocorrelation).
- **Plate: output taps moved outside the tank's allpasses.** It is the only deviation from Dattorro's paper:
  the inside of an allpass is a comb and its period could be heard in the tail.
- **The plate shrinks with short decays** (at least five tank round trips within the decay): below about 3 s
  the decay decides the size and `size` is only a ceiling. Below about 1 s the plate stays grainy by
  construction: the panel and the prompt say so, and direct to `room` for short spaces.
- **Density measurement (FX-9).** "No isolated peak" is rendered like this: the autocorrelation of the flattened tail must
  not return at a delay *and* at its double (2–150 ms), which is the fingerprint of a comb. The maximum peak alone did not
  distinguish: even a well-made plate has a single repetition between two taps of the same line. Threshold 0.11: the
  old reverb measures 0.14–0.20, the new ones 0.04–0.10. The plate is measured from 2 s up.
- **Decay time.** ±10% without damping and without modulation at 0.5, 1.5 and 4 s; ±15% at the default values. Below
  0.5 s hall and plate are up to 11% long: it stays in the notes of `dsp.md`.
- **Compressor mix on the gain**, not on two paths: `1 + mix·(g − 1)`. Dry and compressed are the same
  sample, so no comb filter. The `Compressor` type is shared with the master, where `mix` stays at 1 and
  is not exposed (FOUNDATION: the master compressor does not have it).
- **AI contract already at `mix_plan` v3 for reverb and mix.** Removing `room_size` from the document the AI crate
  no longer compiled: the new reverb fields (`algorithm`, `decay_s`, `size`, `modulation`, `return_lpf_hz`) and
  `compressor.mix` enter now, with limits, validation, prompt and fake decider (which for now leaves `mix` at 1 and
  the algorithm as it is). `structure`, doubles and techniques stay for F12.
- **Migration (PRJ-8).** Schema 2: the old reverb becomes `room` with the decay its `room_size` gave (feedback
  `0,28·room_size + 0,7` on lines of about 30 ms); compressors receive `mix` 1. The same
  function updates the snapshot files, which have no version. The render is not identical (P-011).
- **New unit `s`** in the parameter specs, for the decay.
- **Interface.** Boards 7a (Plate/Room/Hall selector, Decay, Size, Pre-delay, Damping, Modulation, filters) and 7e
  (parallel compression) designed in `design/` before the implementation. The Mix field sits in the Knee row;
  the text next to it says "Parallel" below 100%. "Reverb return" in the Add menu creates a hall: the algorithm is chosen
  in the panel.

**Verification.** 12 tests on the reverb and 2 on the mix in the DSP crate, allocation test on the three algorithms, engine tests
(same hash with blocks of 64, 1000 and 4096; mix 0 equal to the compressor excluded within 0.1 dB), migration, one
Playwright test (algorithm and mix, each one undo step), `make bench`: NFR-1 p99 17.9% with four more reverbs.

### A-059 — F12: structural AI mix (2026-11-11, P-011)

**Context.** FOUNDATION asks that the plan be able to create returns and groups (AI-17 → AI-20), the measurement of true
doubles (ANA-18), and mixing techniques in the prompt and in the fake decider (AI-21). Choices made while building them.

**Choices.**

- **The structure becomes real only once, in the working copy.** After the first iteration, what the plan creates is
  inserted into the working project (`structure::materialise`) and removed from the plan. From then on they are returns
  and groups like the others: later iterations find them in the schema and in the analysis without `structure` (AI-19),
  the plan is rendered and evaluated together with them, and the A/B listening includes them. The existing validation is
  unchanged: it works on a project that contains them.
- **Fixed placeholders**: `n1`–`n3` returns, `n4`–`n6` groups, all present in the strict schema and nullable. The strict
  schema needs every property listed and required, so there cannot be a free number of elements.
- **Ids of the new elements outside the project counter** (prefix plus the instant of the call): the project can change
  during the revision without an id colliding with a track added by the author.
- **On apply, the structure is recreated on the project as it is at that moment** (`structure::commands`), not by
  replaying saved commands: a saved command would carry the old state of a track. If a member is gone or has been
  routed elsewhere, the group is not created (`NoLongerFits`) and the rest is applied.
- **Own checkbox on the new elements** (AI-20, updated): `aiApply(only)` also receives their ids. For each one the core
  exposes the tracks it depends on and how many it needs (one sender for a return, two members for a group): the
  interface shows "not created" with the same criterion the core uses to decide. A send to a return that is not
  created is dropped; the other changes to the track remain.
- **No structure in requests limited to some tracks** (AI-18, clarified in FOUNDATION): returns were already outside a
  scope (A-034) and a group changes the path of its members.
- **What cannot be created is dropped with a warning, it does not fail the plan**: a return without sends, a group with
  fewer than two valid members, a locked member or one with a different output, too many returns, nesting beyond four
  levels, parameter type different from the declared one (the defaults apply). Only a group created without its chain in
  `groups` is an error, sent back to the model, as for every existing group.
- **True doubles (ANA-18): per-frame spectrum comparison, not waveform correlation.** The analysis cache keeps the power
  per band of every frame, not the samples: the frame-by-frame comparison of the spectral shape tells two performances
  (a few dB) from two copies (a fraction of a dB) without re-reading the audio, and also recognizes as a copy a "fake
  double" delayed by a few milliseconds, which zero-lag correlation would not see. FOUNDATION is updated accordingly.
  Initial thresholds: overlap ≥ 0.7, envelope correlation ≥ 0.6, profile difference ≤ 3 dB; copy below 0.5 dB per frame
  with envelopes correlated above 0.98. Each track is in at most one pair. To be tuned on real material (VALIDATION 28).
- **Fake decider (AI-21)**, deterministic rules:
  - non-`identical` double → pan ±0.7 and same level;
  - solo vocal → "Vocal plate" return (plate, 1.2 s) in place of the shared reverb, and "Slap" return (delay
    110 ms, feedback 0); if they already exist under that name it reuses them, so a second auto mix does not recreate
    them;
  - solo vocal in the choruses → send to the delay +6 dB (throw);
  - drums on at least two tracks that go straight to the mix bus → "Drums" group with parallel compression (mix
    35%); an already existing drums group receives the same compression.
- **Prompt**: "New returns and groups" section only when the structure is allowed; "Techniques" section always (doubles,
  slapback, throw, parallel compression, distinct spaces), each with the case in which to avoid it.
- **Command-line demo** made richer (snare, a guitar played twice, two sections), so that the full flow exercises all
  the techniques.

**Verification.** ANA-18 tests on synthetic signals (two performances, a copy, different rhythm, different color); four
flow tests with the fake decider and a "spy" decider (structure created once and stable across iterations, apply in one
step and undo, partial apply in the five cases, toggle, scope, lock, warnings and retry, revision with structure); full
flow from `stemwise-cli`; a Playwright test on the proposal (boards 4a and 4b) and on the toggle.

### A-060 — Fixes from the independent review of F11 and F12 (2026-11-11)

**Context.** Review of `f10..HEAD` with `app-reviewer`: 2 blockers, 4 findings to fix, 3 notes.

**Blockers, fixed.**

- **A new return fed only by a new group was never created**, not even with "Apply all": returns were created before
  groups, when their only sender did not exist yet. Now groups come before returns, both in the working copy and on
  apply; the list stays in the order of the proposal.
- **A sidechain to a new group that was not created made the whole apply fail**: the nonexistent source made the
  routing invalid. Now a source that is not in the project is dropped (the compressor listens to its own track) and the
  rest is applied; this also holds for a track deleted during the revision, a defect that predates F12.

**To fix, fixed.**

- **New return left without senders** (a later iteration that removes the sends, or the only sender is locked): before
  the proposal it is removed from the working project, the plan and the list, with a warning (`structure::prune`). The
  proposal's commands are built on the project with the creations already applied, as the apply will do later.
- **"No longer fits" visible in the interface.** Each new element carries `fits`, computed on the project as it is now;
  the proposal updates on every project change during the revision and says "The project changed since this
  proposal: it no longer fits".
- **Tests that did not prove what they cited.** The command-line test now verifies from the saved project every
  AI-21 technique and that the double is found and not `identical`; added a test on the copy delayed by 12 ms
  (ANA-18), one on the blocking cases and on the return without senders, and the density of `room` at 0.6 s (FX-9).
- **Unrecorded divergences.** Doubles also require the same channel count (a mono and a stereo track are not two
  performances on the same microphone; it stays so). The fake decider now also gives the two doubles the same
  processing (filters, EQ, compressor, saturation), as AI-21 asks.

**From the test on the delayed copy.** The rule for `identical` also required envelopes correlated above 0.98: a copy
delayed by 12 ms has the same color in every frame (0.13 dB) but a shifted envelope (0.91), so it was not
recognized. Now only the per-frame difference counts, below 0.5 dB. Two synthetic performances with the same envelope
measure 0.8 dB: the margin is narrow and the threshold must be tuned on real material (VALIDATION 28).

**Notes.** A sender locked during the revision no longer counts as a user of a new return. The "Allow
structural changes" checkbox no longer rebuilds the audio graph.

### A-061 — A new target waits under an overlay (2026-11-11, after F12, asked by the author)

**Context.**
- After a change of the target's loudness or ceiling, the Loudness tab showed only `Measuring…` and the line
  "Your change is already playing" (state 9k, A-051). The author kept working without noticing that the change had not
  happened yet.
- The line was also false for this case: a change to the chain is heard immediately, but the level of a new target
  arrives only when the gain search is finished (up to six passes over the whole master, A-055).

**Choice** (board 9v, spec in `design/README.md`).
- **Only the target blocks.** The other master changes stay inline as in 9k: covering the workspace at every knob
  would break the "touch and listen" loop.
- **Contract.** `MasterStatus.retarget` (`from_lufs`, `from_true_peak_db`, `fraction`, `elapsed_ms`), present
  while the project's target differs from the one the on-screen measurements were made for. The backend decides it,
  so it holds for every route: preset, Custom values, "Match the reference", undo and redo, an applied master
  proposal.
- **Overlay.** The same panel as the preparation, over the workspace as it is: new and previous target, bar, two steps
  (gain search, measurement of the result), a note on what is heard in the meantime. It appears after 250 ms: a target
  already measured (among the last 16 results kept in the project) comes back immediately, without overlay.
- **What stays active.** Phase selector and transport: it plays the master at the previous level, and stopping it must
  stay possible. Undo, A/B, Export and the other buttons wait. `Esc` and "Back to Mix" return to the Mix without
  stopping the work, with the ring on the Master segment.
- **No "Cancel".** When the work is done the change is an undo step, and going back to the previous target is
  immediate because it is already measured.
- **Dragging.** While dragging a Custom value the overlay does not appear; it starts on release.
- **Export dialog.** If the change happens there ("Match the reference", 10d) the overlay does not appear: it waits for
  the primary button, as before.
- **Progress.** The fraction counts the passes out of six possible, so a search that converges in two jumps
  from 28% to 85%. No estimate of the remaining time: the number of passes is not known beforehand.

**Verification.**
- A Playwright test on the fake backend: overlay with the texts, old values behind, transport active, undo disabled,
  `Esc` with the ring, end with the new level, immediate return with undo.
- In the real app's self-test: a target change is reported as waiting until the new level, and
  undo returns to the previous measurement.
- The fake backend now updates the master also after undo and redo, like the app.

### A-062 — Truncated provider response: explicit ceiling, retry, full log (2026-11-13, from the author's test)

**Context.**
- On a 30-stem project the second AI mix always failed with "the provider refused the request (200): the answer
  was cut off at the model's output limit". The two failed calls lasted 22 and 48 s, with zero tokens reported,
  and do not appear in OpenRouter's Activity; the first round had written about 8,800 tokens per iteration.
- The cause cannot be proven after the fact: of a failed call the log kept only the messages and the error, not the
  request body (schema included) nor the response. The request also carried no `max_tokens`, so the
  ceiling was the one chosen by the provider. The message talked of refusal, which is not what had happened.

**Alternatives.** (a) Only the message. (b) Fixed ceiling in the code. (c) Ceiling in the configuration, retry
in JSON mode and full log.

**Choice: (c).**
- **Log (AI-10).** Of every failed attempt `request-<it>-<n>.json` contains messages, error and every exchange with the
  provider (request sent, status, response); `response-<it>-<n>.json` contains the last raw response. The
  key travels only in the header and does not enter the log.
- **Ceiling.** `AiConfig.max_output_tokens` (32,000, reasoning included), sent as `max_tokens`; 0 sends
  nothing. A configuration saved earlier takes the default value. If the provider rejects the ceiling (400 naming it)
  the session continues without it. It is not exposed in Settings: it is changed in `ai-config.json`.
- **Retry.** A response with `finish_reason: "length"` under a strict schema is retried once in JSON
  mode, only for that call (a model can get stuck under a strict schema until the cut); the plan is still
  validated by us. The tokens of the truncated response are added to the call's cost (AI-15).
- **Message.** A dedicated error (`Truncated`) with the tokens written and the ceiling sent; the other provider errors
  say "answered with an error", not "refused".

**Reason.** Without the saved response a provider failure cannot be diagnosed; the explicit ceiling removes a
variable; the retry covers the most likely case without weakening the validation.

**Caveat.** The cause of the observed failure remains unknown and the fix is not proven with the real provider: if it
recurs, the call's log now contains the response (VALIDATION 7–11).

### A-063 — Balance and effects of the AI mix, sections beyond 120 s, a single button (2026-11-13, P-012)

**Context** (analysis of the Balance calls, 30 stems, and of Confidence).
- With the plan applied, measuring the stems with trim and fader: drums −18.6 LUFS, vocal −20.3, guitars −20.9, backing
  vocals −29.0. Vocal reverb about 20 dB below the signal, slapback about 30 dB below.
- The only measured goal of the loop was the spectral distance from the reference. The raw mix had 4 dB too much at
  250–500 Hz and 4 dB too little below 120 Hz: to get closer the model cut 330 Hz on all the guitars and lowered
  them. At 1.1 dB the loop stopped.
- The model decides one fader per track on trims normalized to −23 LUFS: fifteen drum microphones summed
  exceed a single vocal, and no number told it so.
- The prompt held back the effects ("sparingly", "at a low level", "create nothing unless it clearly helps").
- **Defect.** `r1`, the rounding of levels (limit ±120), was also used for seconds: song duration and
  section boundaries stopped at 120 s. In Balance (185 s) Solo and the last Chorus came out with zero duration, and the
  model wrote it in the summary.
- **Defect.** The "Auto mix" button ignored the text written in the field: the request started only with Enter or
  with the arrow. In the Balance log there is no mix revision at all.
- On Confidence, "backing vocals on par with the lead" produced exactly +4 dB (from −7 to −3 against +1): the DEC-5
  limit.

**Choices.**
- **Measured balance (AI-22, `balance.rs`).** For each family (by role) a solo render with the returns
  muted, for each return a render on its own, plus the mix: integrated and per-section loudness, reported as the
  difference from the mix in LU. Three renders at a time. Before the decision (`balance_at_unity` for the auto mix,
  `balance` of the starting plan for the revision) and in the evaluation of every iteration.
- **Loop.** An auto mix does not stop at the first iteration. The plan kept is the last valid one, unless it is
  farther from the reference by more than 1.0 dB (`SPECTRAL_TOLERANCE_DB`) than an earlier one; without references
  the last valid one. Discarded alternative: a balance score in the loop, because it would put a fixed taste
  above the brief.
- **Prompt.** New sections "Balance" (starting ratios between families), "Tone and the target" (the reference is
  tone, not balance; it is mastered) and "Space" (audible space, thresholds on `returns_lu`); removed the caution on
  effects and sections.
- **Revision without step limit.** `limit_changes` applies only between the iterations of an auto mix.
- **Seconds.** `secs1` for duration and section boundaries; `r1` stays for levels.
- **A single button** (asked by the author): arrow removed, Enter inserts a newline, the button says "Auto mix" without
  text and "Revise mix" with text, with a line explaining the difference. As already in the Master. It departs from
  board 3 of the design, which shows the arrow.

**Costs.** An auto mix without references goes from one to two model calls. Each evaluation adds a render
per family and per return (about ten on Balance).

**Caveats.** The starting ratios and the return thresholds are rules of thumb, not measurements: to be tuned by ear
(VALIDATION 31). The effect on the real model is not proven by the agent. The fake decider does not use `balance`.

### A-064 — Waveforms recomputed on every playhead update (2026-11-13, from the author's test)

**Context.** During playback of Balance (33 tracks, 185 s) the editor stuttered. The timeline
redraws on every playhead update and every clip rebuilt the path string of its waveform
(up to 2048 columns, two `toFixed` each): the web view's sampling showed the main thread always
busy in `toFixed`, `join` and garbage collection. The 60 fps test (NFR-3) did not see it: it uses narrow clips.

**Choice.** The path is computed only when the peaks or the height change (`useMemo`) and the `Waveform` component
is skipped on equal props (`memo`). After the fix the main thread is busy about 22% during
playback of the same project, in the app in development.

**Not done.** The timeline still redraws all the rows on every update: separating the playhead from the rest
would reduce the load further, but it touches the drag gestures and must be done separately.

### A-065 — Playhead out of the timeline redraw; Sends in their own section (2026-11-13, asked by the author)

**Playhead (NFR-3, follow-up to A-064).** The timeline read the playback position as state and redrew
entirely, rows and clips included, on every update. Now the position is read from the store when a gesture needs it
(snap, cut at the playhead, paste, grab on the ruler, clip menu); only the playhead line (`Playhead` component), the
current value of each automation lane (selector that changes only with the value) and auto-scroll (store
subscription, without render) follow it.

**Sends.** In the inspector the sends sat next to the saturation in the same section: the knobs
got confused. Now the saturation has its own row and the sends a section below, separated by a line, with the label
"post-fader · to the returns": they are not a block of the chain, they leave it after the fader. It departs from board 7
of the design, which puts them side by side.

### A-066 — Mix state at a glance: design choices (2026-11-13, P-013, asked by the author)

**Context.** On a 30-stem project the author could not tell which tracks had effects and sends without
selecting them one at a time. In the code: the header showed name, role, mute/solo/lock and mini fader; the strip
two unnamed send bars, only for the first two returns (`sendTargets.slice(0, 2)` in `Mixer.tsx`); the return's lane
only the number of senders. Everything else was in the inspector.

**Choices.**

- **Badges, not squares or icons.** Three rounds with the author. First five identical squares read by position: nobody
  remembers which position is which block. Then an icon per block (filter curves, bell, bent line, squared
  wave): hard to understand and not very standard; a family of solid symbols (sliders, arrows,
  flame) was also compared, in which however high-pass and low-pass stayed almost identical. Finally chosen the
  **badges** `HP LP EQ CMP SAT`, the ones printed on any mixer: filled when the block is working, outlined when it is on but
  neutral, dim in bypass; `CMP` in the color of the source with the sidechain and with two small bars when it is parallel. I withdraw
  the objection made to letters in the first version: they were single letters (H L E C S), these can be read.
- **In the strips the badges sit in a column**, one under the other at strip width and centered, like the insert slots
  of a console. Four layouts compared with the author: a single row requires 90 px strips (at 1440 there would be room for 7 instead of
  16); two rows force a 7 px character; the two-column grid cuts `CMP` at
  40 px. The column costs 54 px of height: the mixer's default height rises from 240 to 296 px
  (`--mixer-h-glance`) and the timeline scrolls above.
- **Three states, not two.** "On but neutral" exists because an EQ not in bypass with the bands at 0 dB does nothing: with
  only two states the mark would lie, or the click on the bypass would not change what is seen.
- **Click on the badges in the strips and in the Overview, not in the headers.** In the strips each badge is as wide as the
  strip (36 × 10 px) and in the Overview each cell is: there the click sets or clears the bypass and the double click opens the
  block in the inspector (confirmed by the author on 2026-11-13). In the headers the badges are small and in a row:
  they stay read-only and the click selects the track.
- **Label column in the mixer** (`--rail-w` 44 px, 72 in the Overview) instead of badges or colors for the returns:
  the returns stay neutral (design principle 3) and the row of each send has the name written. Cost: the strips
  go from 42 to 40 px minimum (`--strip-w-glance`) to fit 16 strips and the bus at 1440.
- **Header from 240 to 344 px** (from 200 to 304 at the minimum window; `--head-w-glance`, `--head-w-glance-min`): the
  marks cell is 94 px wide (the five badges in a row) and must not take space from the name. The lanes lose
  104 px.
- **The app does not change until F13 starts.** The interface imports tokens and styles directly from `design/` (A-002):
  for this reason the new measures are separate tokens, to replace `--head-w`, `--strip-w` and `--mixer-h` in F13, and the rules that
  touch up already existing markup (sends in the strip, bus pinned to the right) apply only where there is the label
  column (`.mixer:has(.mxrail)`).
- **Links in the accent color** (lime), not in the color of the selected track: the returns are neutral and
  their gray would not be visible.
- **Overview without the fader row and without the "→ group" row**: the output is already a row of the grid, and with 4
  returns the grid would not fit in the mixer's height.
- **All the Mix boards regenerated** with the marks, because the design is normative and two versions of the same
  header would be an ambiguity.

**To decide with the author on the boards.** Whether the marks cell is worth the 104 px taken from the lanes; whether the Overview must
remain a mixer view or become a panel of its own; whether the links should also dim the tracks not
linked.

### A-067 — Mix state at a glance in the app: choices and differences from the boards (2026-11-13, F13)

**Done.** Pure function `chainGlance` (`app/ui/src/mix/glance.ts`) that reads from the `Track` the state of the five
blocks with the UI-8 rule, the sidechain source, the parallel and the automation; `relations` gives, for the selection,
the reason for each link (UI-10). The marks (`Marks.tsx`) are memoized per track and do not depend on the playhead;
in the strip only the fader and meter part follows the meters. Mixer with label column, one send per return,
bus pinned to the right and Overview view (`mixerView`, view state). The block open in the inspector moves from local
state to the store (`inspBlock`) so that the double click on a badge or a cell opens it.

**Tokens.** The measures of A-066 become the base ones instead of remaining separate tokens: `--head-w` 344 px,
`--head-w-min` 304, `--strip-w` 40, `--mixer-h` 296; the `-glance` tokens go away. New `--mixer-h-short` (240 px,
window under 800 px height, like board 3d) and `--chain-col-h` (54 px, the badge column, which the label column
uses to align "Chain"). Boards regenerated: only the components sheet changes.

**Choices where the boards or FOUNDATION are silent.**
- **Click and double click.** A click on a badge or a cell waits 250 ms for a possible second click, so a double click
  opens the block without setting and clearing the bypass (two history steps). From the keyboard (Enter, Space) the bypass is
  immediate.
- **Existing send in the Overview.** A send has no bypass in the document: the click on an existing send does nothing
  (removing it would lose the level), the double click opens the sends in the inspector; on an absent one the click creates it
  at the default level of `send.<return>` (−12 dB).
- **Returns in the Overview.** The first two rows give the return's filters (for the delay those of the feedback), the third
  the processor (click = return bypass, `bypass.fx`), the fourth decay or time; the sends row says
  how many tracks send to it.
- **Automation.** The mark is in the assistant's color only if the assistant wrote all the lanes with points;
  a single manual lane is enough for it to be in the accent color. The Overview cell says the first parameter and "+n".
- **Links of a selected group.** The four UI-10 rules apply (the group that takes its output, the
  returns, who sends to it, who listens to it); the members are not marked: they are already indented under the group.
- **Neutral saturation** is written "dry" in the Overview cell.

**Intended differences from the boards.**
- **Contrast (UI-7).** The badges and cells in bypass are drawn in `--line-strong`, below AA for text
  (axe-core flags them). In the app they take `--tx-3`: they stay a step below the outlined ("neutral") badge and far
  from the filled one.
- **Rows aligned for returns and bus.** In board 3a the strips of the returns and the bus do not have the badge
  column, and their send rows rise by 54 px relative to the label column. In the app they have an
  invisible column of the same height: each row sits at the height of its label.
- **Order and rows of the strips as before F13.** The app keeps the timeline order (groups before their members)
  and the "→ group" row when the project has groups; the boards put the groups at the bottom and do not have the row.
- **Overview wider than the window.** With the wide Overview column (92 px) and 17 columns the mixer at 1440
  scrolls sideways; board 3e, with 16 narrow columns, fits.

**Verifications.** Vitest on the rules (all states, neutral EQ without bands, with bands at 0 dB, with bands turned off
with gain), Playwright on badges without selection and at all heights, click and double click on the badge with one undo
step, four returns with rows aligned to the column, list of senders, links of a return and of a
track outside the history, Overview with one-click bypass and created send; 60 fps with 32 tracks both with the mixer
closed and with the mixer open, marks and 4 returns (NFR-3). The fake backend has the states of the boards in the demo project and
`simulateReturns(n)` for more than two returns. The real app's self-test was not rerun: F13 does not touch core,
engine or contracts, and the check is left to the author (VALIDATION 33).

### A-068 — The PoC confined to `poc/` (2026-11-13, after F13, asked by the author, issue #3)

**Context.** At the root coexisted the PoC code (`src/`, `tests/`, `schemas/`, `scripts/`, `pyproject.toml`,
`Makefile`, `README.md`) and, in `docs/`, its documents next to those of the product (two `DECISIONS.md` and two
`PROGRESS.md`). A third strand is coming, the product website (issue #4).

**Alternatives.** (a) `poc/`, `app/` and in the future `web/`, with the root acting as an index; (b) `poc/`, `web/` and the
app code at the root.

**Choice: (a).** Three strands with different toolchains are better in three sibling folders: with (b) the Cargo workspace would
mix with `web/`, `design/` and `docs/`, and all the references to `app/` would have to be rewritten (FOUNDATION, rules,
commands, `app/target` as the single build folder) with no gain. `design/` stays at the root because it serves both the app
and the site; `docs/` holds only FOUNDATION and `docs/app/`. `web/` will be born with issue #4.

**Done.** Moved with `git mv` (history preserved): code, `pyproject.toml`, `uv.lock`, `.python-version`,
`Makefile`, `README.md` and `.env.example` into `poc/`; `PRD.md`, `analysis.md`, `DECISIONS.md`, `PROGRESS.md` and
`milestones/` into `poc/docs/`; the archived rules in `poc/POC-RULES.md`, where their relative links are valid
again. The PoC refers to itself only with relative paths, so its content does not change: the only
exception is four links of its README that pointed outside (product, FOUNDATION, rules). Updated:
`make oracle` and `oracle.py` (the Python environment is now `poc/.venv`), `disk.sh`, the formatting hook, the agents and the
`milestone-*` commands, the paths in FOUNDATION §5.1, §5.7 and §9 (rule 6), CLAUDE.md, the crate comments and
`dsp.md`. New root README acting as an index.

**Verifications.** `make oracle` regenerates fixtures identical to the tracked ones; from `poc/` 674 PoC tests pass
out of 675: `test_revise_is_deterministic` fails, but it already failed before the move (same outcome on a copy
of the previous commit with the old environment), so it does not depend on this change and, with the PoC frozen, it is not
fixed. The product's `make check` green.

### A-069 — One toggle per panel in the Mix top bar (2026-11-13, after F13, asked by the author, issue #7)

**Context.** Mixer, inspector and assistant already closed each on their own (UI-4), but only from the keyboard
(⌘M, ⌘I, ⌘J): on screen there was only the way to reopen them (the `Mixer` strip and the two collapsed columns). Whoever
does not know the shortcuts had no way to free up space.

**Alternatives.** (a) Three toggles in the top bar, always in the same place; (b) a close button
in the header of each panel, leaving reopening to the strips; (c) a menu item.

**Choice: (a).** A single place that shows the state of the three panels and does not move when a panel disappears; (b)
scatters the controls and makes them disappear along with the panel. `Panels` group before the settings, with the icons already
used by the collapsed columns (`faders`, `eye`, `spark`): fixed name (`Mixer panel`…) and state in `aria-pressed`, the
shortcut in the tooltip. The state is the same as the shortcuts and the strips, which stay. While a proposal occupies the
right columns, the inspector and assistant toggles leave the bar (they would have nothing to act on, and at
that moment at the minimum window there is no room); the mixer one stays.

**Consequence.** At the minimum window (1280 px) the bar no longer had room with the text `Unsaved changes ·
autosaved`: now it is that text that gives way, with ellipsis and the full tooltip, and the bus meters stay in the window.

**Difference from the boards.** Boards 3a–3g do not have these toggles: the design was not updated (its
generator was not touched). Only the Mix workspace: in the Master the columns do not close.

**Verifications.** Playwright: each toggle acts on its own, follows shortcut and strip, the bar fits in 1280 px with
unsaved changes; with a proposal under review the two side ones are not there. UI-4, UI-3.

### A-070 — Proposals that persist and can be reapplied (2026-11-13, after F13, P-014, issue #2)

**Context.** A proposal lived only in memory while under review: applied or discarded, the call's log (AI usage)
remained but there was no way to get it back.

**Choices.**
- **What is kept.** For the mix a `proposal.json` file in the call's folder: plan, id map,
  new elements, trim, scope, evaluations, warnings. Not the commands nor the diff: they are recomputed on the project as it is
  when the proposal is reviewed, with the same code as every apply (`apply_commands`). For the master the
  `plan-0.json` that the call already wrote is enough. The call's log says whether the proposal exists (`stored`, absent
  in old projects = no): the calls made before this change stay in the list but cannot be reopened.
- **Review instead of blind apply.** "Review" puts the proposal back on the same review screen as
  a new one (A/B, apply all or a part), with a note on when it arrived. Discarded alternatives: a direct
  "Apply" button from the list (it would apply without seeing or hearing what changes today) and a list in AI usage
  (far from the work; the author asked for the assistant).
- **Confirmation.** Only for reopened proposals, before applying: it says when it arrived, that the mix may have
  changed and that the current settings of the touched tracks will be replaced; one undo step.
- **Outcome.** Applying a reopened proposal updates the outcome of its call; closing it ("Close review") leaves it
  as it was. No new call, no cost.
- **Measurements.** In the mix the evaluations shown are those taken at receipt (remeasuring would mean two renders
  at every reopening); the note says so. In the master the proposal is remeasured on the current mixdown, like a new one.
- **Where.** `Proposals` tab in the Mix and Master assistant. With four tabs the Mix selector goes on a
  row of its own: next to the title it did not fit in the 288 px column.
- **Top bar.** The project name now truncates after the save state, when the bar is full (a
  proposal under review at the minimum window).
- **Fake backend.** Undo no longer rolls back the AI call log, as in the app.

**Difference from the boards.** The tab, the note and the confirmation are not in the delivered design.

**Verifications.** Rust: a discarded proposal comes back identical on an unchanged project; after a fader moved by hand and a
deleted track the diff is against today's state, the missing track is declared, the returns are not duplicated,
the apply is a single step; the same for the master. Playwright: list, review, confirmation with cancel, updated
outcome, close without effects, in Mix and in Master. Not tried in the real app nor with proposals from a real
model (VALIDATION 35).

### A-071 — Guided assistant setup: design choices (2026-11-13, asked by the author, approved on 2026-11-13)

**Context.** With the repository opening in view, the author wants someone who does not know what OpenRouter is to still
end up with a working assistant. Today the first launch (1b) and Settings (11b) ask to paste a key.

**Verified on the OpenRouter documentation (2026-11-13).** There is an OAuth flow with PKCE: the app opens
`https://openrouter.ai/auth?callback_url=…&code_challenge=…&code_challenge_method=S256` (plus `key_label` and `state`),
the user signs in or registers and approves, the browser returns to the `callback_url` with a `code` valid for 10 minutes, the app
exchanges it with `POST /api/v1/auth/keys` and receives `key`, a normal key of the user. `http://localhost` is allowed on
any port, without registering the app. The following do **not** appear to exist: custom URL schemes, a spending limit settable
from the flow, reading the balance with a normal key (`GET /api/v1/credits` wants a management key;
`GET /api/v1/key` gives only usage and limit of the key). Without credit a request returns 402, with `limit_source`
`openrouter_credits` (balance) or `openrouter_key_limit` (key limit).

**Choices.**

- **A dialog in three steps** (`Connect OpenRouter`, `Add credit`, `Ready`), the same from the three entry points: first
  launch (1b), Settings › AI (11b), assistant column without a key (13e). Boards 14a–14k.
- **OAuth as the main route, pasted key as the secondary route** (14g, 14h): three numbered steps, each with the
  link to the right page. Discarded alternative: only the step-by-step guide, which leaves the copy and paste.
- **Return on `localhost`**: the app listens on `127.0.0.1`, port chosen by the system, for a single request and at most
  10 minutes; `state` and PKCE S256. The page the browser shows (14c) is served by the app. To be reconciled with NFR-7 at
  implementation time: the network stays OpenRouter only and only on user action, but a temporary
  local listener is added.
- **The credit step does not verify the balance**, because the app cannot read it: it says so plainly (14d), offers the
  link to the credits page and closes with "I have added credit" or "Skip for now". The lack of credit
  is discovered at the first request and has its own state in the assistant column (14i), distinct from the key
  limit (14j) and from the key no longer valid (14k).
- **The per-project spending cap is already set here** (14d): it is the only protection the app controls.
- **The amounts in 14d are examples.** In the app they must be computed from the cost log; until a
  history exists, from values measured by the author with the default model (VALIDATION).
- **Offline demo always one click away**, in 1b and in the dialog: the app can be tried before opening an account.

**Fixed along the way.** The board captions were not visible on any page: the fader's `.cap`
(`components.css`) covered the board's `.cap`. The rule in `screens.css` is now `.board>.cap` and resets the
inherited properties.

**Implemented** (P-015, AI-24). `stemwise-ai::connect` (listening on `127.0.0.1` and `::1`, PKCE, code exchange),
commands `ai_connect_start`/`cancel`/`reopen` and `ai_open_page` (fixed list of OpenRouter pages), dialog
`ui/src/ai/Setup.tsx`. Differences from the boards:

- **Spending cap in 14d only with a project open**: the cap belongs to the project. From the first launch the step says where
  to set it.
- **Amounts in 14d computed**, not examples: the app's estimate on a typical song (12 tracks, 8 sections) at the prices
  of the chosen model (`budget::typical_costs`). They remain estimates: to be compared with the real costs (VALIDATION 36).
- **"Open OpenRouter" in place of "Open sign-up"** in 14g: the documentation does not give the sign-up address;
  the main page is opened.
- **A refusal without `code`** is treated as "not approved": the documentation does not describe the return after a
  refusal. If the provider does not return at all, the wait ends at expiry or with Cancel.
- **1b was designed but not built**: now the first launch shows the invitation in place of the recent projects.
- The policy tests (NFR-7, NFR-8) admit `connect.rs` as the second network file and `signing.rs` as the second
  macOS-specific module, and verify that the listener is local only and that the app opens only provider pages.

**Not verified.** The flow with the real provider (the agent does not use accounts or keys): everything is proven against the fake
server. VALIDATION 36–38.

### A-072 — Where the key lives: keychain if silent, otherwise private file (2026-11-13, decided by the author)

**Context.** On first use of the key macOS asked for the keychain password twice. The code reads
once only (A-052); the two prompts come from macOS, which for an app not signed with a stable identity checks both
access to the item and its partition list, and fails both at every build.

**Alternatives.** Keychain only with the author's signature (the prompts remain in `make dev` and for those who build from
source); always file (the keychain is given up even in the signed package); automatic.

**The author's choice: automatic.** `signing::keychain_is_silent()` asks the system whether the running code
satisfies `anchor apple generic` (any certificate issued by Apple; never an ad hoc signature). If yes: keychain,
as before (`LazyKeyStore`). If not: `FileKeyStore`, file `openrouter.key` in the app's data folder, created with
permissions 600 and replaced by rename. Outside macOS: keychain. `STEMWISE_KEY_STORE=keychain|file` forces the
choice. `AiStatus.key_place` carries the choice to the interface, which names it wherever it speaks of the key.

**Consequences.** With an unsigned build a key already saved in the keychain is not read (reading it
would bring back the prompts): the assistant is reconnected once. The file protects from other users of the computer, not
from programs running as the same user; the keychain remains the choice where it costs no friction. Signing the
package remains with the author (A-046): with the signature the app will use the keychain by itself.

**Not verified.** That the keychain is really silent in the signed build (VALIDATION 38).

### A-073 — Wider mixer strips (2026-11-13, the author's choice)

**Context.** At 40 px (A-067) the strips cut 9 names out of 17 in the demo project and the author found them narrow.

**Alternatives shown** on the real mixer at 1440 px with the side panels open: 52 px (3 names cut, 12 strips
visible), 64 px (none, 10 visible), 64 px with larger controls.

**The author's choice: the third.** `--strip-w` 64 px (52 in the compact window, `--strip-w-min`); in the strips the
fader and its cap go from 22 to 30 px (`--strip-fader-w`, `--strip-cap-w`) and the chain badges from 8 to 9.5 px
(`--fs-sig-strip`); track headers, inspector and horizontal sliders stay as they were. The badge column
grows by 9 px: the mixer's default height rises to 304 px and to 250 in the short window, so the fader's
travel does not change. The mixer scrolls horizontally sooner: 10 strips plus the bus instead of 17 at 1440 px.

**Not touched.** In the minimum window (1280×720) the fader travel was and is 30 px: little, but it is a limit
that predates this change.

### A-074 — Mixer label column: a single width (2026-11-13, reported by the author)

**Context.** The label column was 44 px wide in the Faders view and 92 in the Overview (A-067): switching from
one view to the other the strips shifted by 48 px, and in Faders the longer return names were cut off.

**Choice.** `--rail-w` is 92 px, like `--rail-w-wide`: the column has the Overview's width in both
views. The strips do not move and the labels fit in full. It costs the strips 48 px in the Faders view. Playwright
test: same column width, same strip positions, no label cut off.

### A-075 — Logo: direction "D · Two rooms", to be refined in canvas (2026-11-13, the author's choice)

**Context.** The app icon (five lime bars on aubergine) and the mark in the top bar (four bars) are
correct but anonymous: they are the bars of any equalizer. The author had four directions drawn by an external
agent on a standalone brief (`design/brand/BRIEF-logo.md`); the proposals are archived in
`design/brand/directions/`.

**Alternatives.** A · Convergence (four stems gathering into one output), B · Monogram (an S as an automation
curve), C · The fader (a cap resting at the right spot), D · Two rooms (a lime and a lilac trajectory that
meet).

**The author's choice: D.** It is the only one that tells a feature specific to the product, the two phases Mix and Master
with their two accents. The delivered proposal is a starting point, not the final logo: two strokes that join
under an arrowhead read as the generic "merge" icon; the two lines arrive together, whereas the phases
come one after the other; the mark takes up half the height and at 16–32 px it shrinks to an arrow; in monochrome (the top
bar, `currentColor`) only the "merge" is left; the lockup's lettering is not Bricolage Grotesque; the icon's
background is not a continuous-corner squircle.

**Next step.** A Claude Desktop session in canvas with the prompt `design/brand/PROMPT-canvas-D.md`: three
variants within D, one chosen by the author, then the full refinement with delivery in `design/brand/final/`.
Adoption in the app (Tauri icons from `icon-1024`, mark in `app/ui/src/shared/Icon.tsx` and `mark()` of the boards,
welcome and AI connection page) is a later step, with its own visual checks.

### A-076 — Logo: the refined D (2026-11-14, on the author's direction)

**Context.** The prompt `design/brand/PROMPT-canvas-D.md` asked for three variants within D and then the refinement of
the chosen one. The three variants (Threshold: three lines and a ring; Plan: two rooms in plan view; Ess: a broken S) were
all discarded by the author: the delivered D proposal was to be refined, not new shapes drawn.

**Alternatives.** Keep D as it was, removing only the arrow (it stays a Y-shaped "merge", the two lines stay two
equal inputs); a scale that lines up the two curves (loses the shape of D); the two curves of D as the Mix
and, in place of the arrow, a second shape for the Master.

**Choice.** The last one. The two trajectories of D, with their curve, are both lime: the Mix, many parts that
come together and touch without fusing. Beyond a gap, in place of the arrow, the Master's room: a filled
rounded rectangle in lilac. Lines against surface keep the two rooms distinct even in monochrome; in the top
bar the room of the current phase lights up and the other goes to 40% (as a color mixed with the background, not as
opacity). The name is Bricolage Grotesque 600 converted to paths from the file the app already includes.

**Reason.** It solves the seven defects listed in A-075 while changing as little as possible of the approved shape. Delivered in
`design/brand/final/` with README and `presentation.html`; regenerated with `design/brand/build-final.py`. Adoption
in the app remains the next step.

### A-077 — Measured AI master: diagnosis and loop (2026-11-14, issue #1, P-016)

**Context.** The author: the AI master often gives a choked limiter that cuts the dynamics, and when asked to
preserve them it lowers the LUFS target. The question was whether the limit was in the tools or in the model.

**What was found** (six real calls saved in the author's projects, all `gpt-6.1-sol`, and the Balance mixdown
measured with a script):
- The mixdown has 17 dB between true peak and loudness (−20.1 LUFS, peak at −3 dBFS); at −9.6 LUFS with ceiling at −1 less
  than 9 remain. Only 0.7% of the samples are within 9 dB of the peak: a few isolated hits. First-order estimate: at −14 LUFS
  the limiter works 2% of the time, at −9.6 49%, with 8.5 dB on the peaks.
- The model received `dyn_range_lu` (3–4 LU, the loudness variation between passages) and concluded "dynamics already
  contained, no compression" in four answers out of six. The peak-to-loudness distance did not reach it.
- A single call, without knowing how hard the limiter would work either before or after; the measurement of the proposed master
  (A-033) went only to the interface.
- A ten-line prompt, without technique. Every call was a revision of an empty chain with "change what is
  asked and keep the rest".
- Of the limiter the plan adjusts only the release. With these levers lowering the target was the effective move.

**Choices.**
- **Measurements** (`mastering::dynamics`, master analysis version 2): `peak_to_loudness_db`, `short_crest_db`,
  `peak_share_pct`, `destination.peak_reduction_needed_db`, loudness of the references; `dyn_range_lu` becomes
  `loudness_range_lu` and the prompt says what it does not mean.
- **Loop** (`run_master`): the current master and every plan are rendered at the target's loudness
  (`evaluate_master`: gain search, then a render with the reduction statistics,
  `engine::master::render_metered`, 10 ms windows) and measured. At most three plans (`MASTER_ITERATIONS`); the loop
  stops when the model returns the same settings as the plan just measured, without another render. The last plan is kept:
  there is no single measurement to optimize like the mix's spectral distance, and the model sees the whole
  history. A later iteration without a valid plan leaves the one already measured.
- **Prompt**: the chain described for what it really does (saturation as a soft clipper with the dB taken off the peaks
  at each drive, the loudness gain found by the app, the limiter as the only stage after), how to read the measurements,
  how to distribute the peak reduction among the stages, practical thresholds on the limiter (above 3 dB for more than a few
  percent of the time it can be heard).
- **Target** (clarifies A-035): with a request the model can still change it, but the prompt admits it only
  if the request asks in words for a different loudness. It is not enforced by the code: recognizing "more volume" in every language
  is not reliable. If the target cannot be reached cleanly, the `summary` says so and indicates by how much to lower it or
  to tame the peaks in the mix.
- **First request**: on a chain equal to that of a new project the request asks for a whole master (no
  starting plan); the call type stays `MasterRevision` because it carries a request.
- **Call files**: `plan-<n>.json` and `evaluation-<n>.json` per iteration, `plan.json` the plan kept;
  `restore_master` reads `plan.json` and, for earlier calls, `plan-0.json`.
- The measurement for the interface (`measure_master`) starts from the gain already found in the call.

**Discarded alternatives.** Keeping the "best" plan according to a score (which one? loudness, punch and timbre pull in
different directions); enforcing the target from the code even in revisions (removes a legitimate request).

**Not done here** (next step, touches MAS-1, project schema and design): clipper before the limiter, parallel mix
and filter on the master compressor's detector (today the detector also hears the bass: the 100 Hz filter
applies only with an external sidechain), chosen chain order, per-section offsets, return to the mix when
the peaks are too many, the evaluation shown in the proposal. The multiband master stays out of scope (FOUNDATION §2).

**Measurement with the engine on the real mixdown** (Balance, 187 s, temporary probe not kept; limiter: time over 3 dB
of reduction and maximum reduction):

| Target | Limiter only | Saturation 9 dB | Saturation 12 dB | Fast compressor + saturation 9 dB |
| --- | --- | --- | --- | --- |
| −14 LUFS | 0.0% · 4.2 dB | 0.0% · 1.9 dB | 0.0% · 0.4 dB | 0.0% · 1.4 dB |
| −12.5 LUFS | 0.4% · 5.9 dB | 0.0% · 3.5 dB | 0.0% · 1.9 dB | 0.0% · 2.9 dB |
| −9.6 LUFS | 21.4% · 10.7 dB | 12.7% · 8.0 dB | 5.5% · 6.0 dB | 10.0% · 7.2 dB |

At −9.6 the limiter alone works over 1 dB for 61% of the time. The stages already present lighten it a lot, but
the short crest lost stays 2.9 dB in every variant (0.1 at −14): that loss is decided by the target, not by the
tool. The tool decides **how** it is lost (rounded peaks or the whole mix ducking after each hit).
To keep more punch at that loudness lower peaks are needed already in the mix.

**Verification.** `the_master_loop_measures_each_plan_and_the_next_decision_corrects_it`,
`a_request_on_an_untouched_master_asks_for_a_whole_master_and_on_a_worked_one_for_a_change`,
`the_metered_render_is_the_same_audio_and_reports_how_hard_the_limiter_works`. With the real model: VALIDATION 39.

### A-078 — The new mark in the app (2026-11-14, the author's request)

**Context.** The delivery of A-076 (`design/brand/final/`) is to be adopted: app icon, mark in the top bars,
start screen, end-of-connection page in the browser, design boards.

**Choice.** The `Mark` component (`app/ui/src/shared/Icon.tsx`) and `mark()` of the boards (`design/tools/lib.mjs`)
draw the 20 px version; with `large` the one on the 100 grid (start screen, 48 px). The two rooms
are two paths (`mark-mix`, `mark-master`) colored by `components.css`: the room of the current phase
in the accent, the other at 40% mixed with the background (`color-mix`), according to `data-phase`; on screens outside a
project the Mix applies. `brand` keeps lime and lilac: start screen and browser page (`connect.rs`, board 14c),
which has no phases. The Tauri icons are generated from `icon-1024.svg` rasterized at 1024 px (`source.png`) with
`tauri icon`, keeping only the files listed in `tauri.conf.json`. The interface in the browser (`make ui`) has the
32 px favicon.

**Verification.** `make check`; boards regenerated with `node design/tools/build.mjs`; icon in the package: VALIDATION 40.

### A-079 — Free master chain: boards 15a–15h, awaiting approval (2026-11-14, issue #1, P-017)

**Context.** After A-077 the author asked to design the next step and posed two questions: whether the chain order
should stay fixed and whether the master is also worked per section. He also decided that the **multiband
master enters the scope** (it was in the right column of FOUNDATION §2).

**Design choices** (`design/screens/15-master-chain.html`, generator `design/tools/s-chain.mjs`):
- **Ten modules.** Input first; Limiter and Dither last, fixed; in between EQ, Match EQ, Compressor,
  Multiband, Width, Saturation and Clipper, **in free order**. Fixed are only those that are also fixed in mastering
  practice: the true peak limiter closes the chain because the ceiling (MAS-3) holds only if nothing follows it, and the dither
  comes after by definition; Input is an input gain, not an effect. Two rows of five; padlock on the
  fixed ones, handle on the movable ones; drag, ⌥← and ⌥→, module menu, "Reset order". A reorder is an undo step.
- **Movable clipper** (revised with the author's opinion: in the first draft it was fixed before the limiter, but that is
  a custom, not a rule: some clip before the compressor or between two stages). It starts before the limiter.
  Two controls: **"Take off up to"**, how many dB to take off the highest peaks that reach it, and the softness; a
  histogram says where the peaks are and how many it cuts. The threshold is relative to the peak measured at the clipper's
  input (the app renders the master anyway to find the loudness gain), so it holds in every position and
  with every target; the first draft counted it from the ceiling, which makes sense only right before the limiter.
- **Compressor**: Mix (parallel, as on the tracks, 7e) and "Listens above" (filter on the detector).
- **Multiband from two to six bands** (decided by the author: three is the start, not the limit, also and above all for the
  model). It opens with three (Low, Mid, High); − and + change the number, a double click on the graph splits a band,
  dragging a crossover onto the neighbor merges two. Per band bypass, solo, reduction and the five controls; from four
  bands up the cards show name, range and reduction (15h). Six like the EQ's bands.
- **Per-section offsets**, not different chains: level, sides, limiter release and EQ gains, added to the
  master of the whole song while the section plays, with ramps of a tenth of a second. A lane under the
  waveform shows them; selecting a section opens its editor in place of a module. "All 3 choruses" writes the
  same values onto the sections with the same name.
- **Measured proposal**: before the module list, five before → after rows (limiter over 3 dB, maximum
  reduction, punch, loudness, distance from the reference); order and sections are cards with their own checkbox.
- **Return to the mix**: when, everything possible done, the limiter still works too hard, a warning gives three exits: keep,
  lower the target to a computed value, or go to the Mix with the request already written in the assistant
  (nothing sent without confirmation).
- The Loudness tab gains the clipper meter and "Limiter over 3 dB".

**Discarded alternatives.** Completely free order, limiter included (breaks MAS-3); different chains per section (a
different product: timbre jumps at the boundaries, unreadable proposal; confirmed by the author); clipper threshold in dBFS
or counted from the ceiling.

**To decide with the author before implementing**: the layout of these boards. Then: new MAS requirements in
FOUNDATION, project schema 3 with migration, `master_plan` v2 (order, clipper, multiband, mix, sections), DSP
(clipper with the measurement of the peak at its input, Linkwitz-Riley crossover from two to six bands, multiband) with `docs/app/dsp.md`, engine with variable order and
ramped offsets, panel.

### A-080 — No genre as the norm, repeatable blocks, three ways to show the chain (2026-11-14, issue #1, P-017)

**Context.** The author asked whether the app is genre-neutral (the discussion on loudness and punch came from
his rock mix) and whether more blocks of the same type can be had in the master chain.

**Neutrality: what was found.** Engine, measurements, loop and targets do not presuppose a genre. The fixed texts do: the A-077
master prompt talked of "snare and kick" and "drum hits"; the mix balance (A-063) took for granted a band with a singer and
cited rock and metal; "a mix without audible space sounds unfinished" was a taste. The
track roles are those of a band.

**Choices on the prompts** (made):
- **Master**: measurements and functions, never instruments ("short transients", "the tracks that carry them"). A new paragraph,
  "What this music wants": the genre comes only from brief, references and target; high peaks and wide loudness range
  are not flaws in themselves; where the dynamics are the performance the light chain is the right choice. A test forbids
  instrument names in the prompt and genre names outside that paragraph
  (`the_master_prompt_names_no_instrument_and_takes_no_genre_as_the_norm`).
- **Mix**: the balance starts from deciding what the song is built on. The A-063 ratios stay, unchanged
  in the numbers, for voice-led music; next to them: instrumental with a lead, music built on rhythm and bass,
  music recorded as a performance (the balance of the take is preserved), everything else. Space depends
  on the music and the brief. In the mix prompt the instrument names stay: there the roles are project data.
- **Not done**: new roles (woodwinds, choir, piano, sub, loops): they touch schema, per-family analysis and interface;
  they go with the implementation of P-017. The warning of board 15g will name the family that really carries the peaks.

**Repeatable blocks** (decided with the author, to be implemented with P-017): the master chain is a **list of
blocks**, not a fixed set. Repeatable EQ, Compressor, Multiband, Saturation, Clipper; unique Input, Match EQ,
Width, Limiter, Dither. At most two per type and twelve in total. Adding a block must have an **always
visible** command, not only a menu item (the author's request).

**Three ways to show the chain** (`design/screens/15-master-chain-directions.html`, exploration):
- **G · Grid**: the cards of 15a with a third row and an "Add module" tile. Familiar; takes height from the editor and
  the order reads in two directions.
- **R · Rack**: the chain is a column next to the editor, top to bottom, one row per block with value,
  reduction and switch; "Add module" is a row of its own. Twelve rows without scrolling; narrower editor.
- **P · Path**: one column per block with a bar that says how far the peaks stand above the body after that block and
  how much the block removed; a "+" between each pair and an "Add module" column. It shows who does the work; names and values
  tight at twelve blocks.

**Pending**: the author's choice among G, R and P; then boards 15a–15h are redrawn in that one.

### A-081 — The master chain is a rack: boards 15 redrawn (2026-11-14, the author's choice, P-017)

**The author's choice** among the three layouts of A-080: **R · Rack**. The boards of `design/screens/15-master-chain.html`
are redrawn that way, with repeatable blocks; the page of the three directions stays as a record.

- **The rack**: a column `--rack-w` wide (288 px, new token) next to the editor, top to bottom. One row per
  block: handle or padlock, order number, name, value, small bar of the reduction it applies, number of the assistant's
  note, indicator. Repeated blocks are called "EQ 1", "EQ 2". The editor of the chosen block is on the right, narrower
  than before (456 px usable): compressor and sections put the controls on several rows; the multiband has one row
  per band at any number of bands, in place of the cards.
- **Adding** (15i): "Add module" is an always visible row at the point where the new block lands (before the
  limiter), with the count "11 of 12"; it opens the list of types that can still be added and says why the others cannot.
  In addition a "+" at the junction between two rows. The new block is born off and opens in the editor.
- **Block menu** (15j): move, bypass, reset, remove; the fixed ones have no "Remove".
- **Reorder** (15b): drag with a line where it lands, ⌥↑ and ⌥↓.
- Board 15g names the group of tracks the peaks come from (A-080).

List: 15a rack · 15b reorder · 15i add · 15j block menu · 15c compressor · 15d multiband · 15h five
bands · 15e sections · 15f proposal · 15g target too high. **Awaiting the author's approval**; then
the implementation (list in A-079, with the chain as a list of blocks and the new roles of A-080).

### A-082 — F14: the free master chain, how it is built (2026-11-14, P-017, boards approved by the author)

**Data.** `MasterChain` = input gain, `blocks` (list of `MasterBlock { id, block }`, with `Block` tagged
by type), limiter, dither; `MasterState.sections` carries the offsets. Project schema **3**: the migration
puts the five blocks of schema 2 in the same order and with the same settings, plus multiband and clipper
off, so the render does not change. The master compressor has its own type (`MasterComp`: mix and `listen_above_hz`,
no sidechain). The structure changes are `ChainOp` in the core (`MasterState::chain_edit`: add, remove,
move, bands, split, merge, reset, initial order): the interface asks, the core does.

**Engine.** The processor keeps two processors ready per type (one for match EQ and width) and receives the chain as a
`Copy` value with nine slots; which processor is used by whom is decided by the block id, so moving it does not change its
state. **Constant latency**: the latency that the oversampled blocks present do not take is taken by a delay line
(256 samples plus the limiter, against the 64 plus the limiter of before). **Structure change while listening**: the output
drops to silence in a few milliseconds, the new chain enters with clean state, it waits for tails and limiter to
empty and rises again (about 25 ms in all): no click, verified by the changes test. The section offsets
follow the position on the timeline with a constant of 25 ms (settled in about a tenth of a second).

**Clipper.** The threshold is the peak at its input minus `take_off_db`. That peak is measured offline before the
loudness search (`clip_peaks`: one pass per active clipper, the second measured with the first at work) and
travels with the master's measurements (`MasterMeasures.clip_peak`): while listening, between an upstream change and the new
measurement, the clipper uses the previous peak. Deterministic (MAS-8).

**Measurements.** `compute_loudness` makes one more pass to know how hard each block works (`render_metered`):
`MasterMeasures` carries limiter and blocks (maximum, average, active time, time over 3 dB), the peak-to-loudness distance
and the short crest. From here the "Limiter over 3 dB" row and the 15g warning: above 5% of the song (`STRAIN_PCT`), with a
lower loudness estimated from the limiter's maximum reduction (not searched with a render).

**AI.** `master_plan` v2 (FOUNDATION §5.5). A block that the plan does not list is turned off and stays at the end; an EQ
without bands in use counts as off. `MasterChange` gives the proposal item by item and `master_partial` applies
some of them. The evaluation carries the reduction of each block by id. Stored v1 plans are converted on reading.
The strict schema uses `anyOf` on the seven block types: proven only with the fake server. The prompt describes the blocks,
the order as a decision and the sections, without instrument names (A-080).

**Interface: differences from the boards.**
- With a proposal under review the rack shows the proposed chain read-only and in place of the editor there is a
  panel that says so (board 15f kept the editor open): editing a chain that is not the project's
  would have been confusing.
- The numbers of the assistant's notes on the rack rows are not there: the master keeps no numbered notes.
- The clipper histogram is drawn from the mixdown's peaks per column (an approximation for the eye), not from the
  signal at its input.
- The 15g warning does not name the family of tracks the peaks come from: it would need a stem analysis against the
  mixdown that does not exist (caveat).
- "Reset" in the rack header returns the blocks to the order of a new project.

**Roles** (A-080): `piano`, `woodwinds`, `choir`, with their rules on file names; in the balance
families piano and woodwinds go with the keyboards, the choir with the backing vocals. There are no families of their own for orchestral music
(caveat).

**Verification.** Engine: order, two blocks of the same type, equal latency, clipper, multiband, sections, render
equal at blocks of 64, 1000 and 4096, no click at the changes. Core: migration, limits, `ChainOp`. AI: plan ↔ state,
items, partial apply, v1 plan, loop that adds the clipper. Interface: 22 Master Playwright tests.
Not run: the self-test in the packaged app and `make build`.

### A-083 — The master waveform: peaks and body (2026-11-14, reported by the author)

**Context.** On a loud master the waveform was a solid purple rectangle. The app drew the mixdown
multiplied by the gain and clipped at the ceiling, column by column: each column of a couple of pixels contains
thousands of samples, at least one reaches the ceiling, and all come out full. More height would have changed nothing.

**Choice.**
- The master is drawn from what was **really measured**: at the end of the loudness search the session
  keeps the envelope of the rendered master, 2048 columns of (min, max, RMS), also in the
  `cache/master/loudness.json` file (version 2), so a reopened project has it right away. Command `master_wave`, type
  `MasterWave`.
- Three layers: the **body** (RMS) in solid color, the **peaks** as a thin line (in a first draft they were a solid
  band, which on a loud master filled the whole height again and the author did not understand it), the **mixdown** body
  solid and dark inside it (also tried as a line only: the author prefers it solid). On a limited master the peaks line is almost flat, and that is right; the body keeps
  verses, choruses and pauses.
- The height is the square root of the level: the body does not squash to the center and the quiet passages do not disappear.
- Before the first measurement only the mixdown is seen, in gray.

**Discarded alternatives.** More height for the tab (same rectangle, taller, and less space for the rack); dB scale of
the peaks only (stays flat where the peaks are at the ceiling).

**Boards.** Boards 9 and 15 still show full master and gray mixdown: not regenerated.

**Verification.** `the_envelope_keeps_peaks_and_body_apart_and_narrows_without_losing_either`; Playwright "the waveform of
a loud master keeps its shape". On the real song: VALIDATION 46.

### A-084 — Wider track headers on request (2026-11-14, issue #9)

**Context.** The header column in the timeline was fixed at 344 px (304 in the compact layout): with long names
or many chain marks there was no way to give the names more space. The issue asks for horizontal resizing
with the current width as the minimum.

**Choice.**
- A thin handle on the right edge of the column (`col-resize` cursor, accent line on hover, while dragging
  and with focus), over the whole height of the timeline: ruler, sections and rows widen together.
- **Minimum** the layout's width (344 px, 304 in the compact one); **maximum** 640 px and in any case such as to leave
  at least 320 px to the lanes. If the window narrows, the column shrinks down to the minimum and returns to the requested
  width when the window widens again.
- It is a **user preference** (`headW` among the interface preferences, in the webview's storage), not of the
  project: it does not touch the document, so no undo and no effect on the render.
- Double click: returns to the layout's width. From the keyboard (UI-7) the handle is a focusable `separator`:
  arrows ±16 px (±64 with Shift), Home and End for minimum and maximum.

**Discarded alternatives.** Width in the project (it is a view choice, not of the mix); resizing from the mixer
(the mixer's label column is another thing, A-074).

**Boards.** Not drawn: the handle is invisible at rest, the boards stay valid at the minimum width.

**Verification.** Vitest `the track header widens on request…` (`geometry.test.ts`); Playwright "the track headers widen
by dragging their edge…" (`mix.spec.ts`), also after a reload.

### A-085 — Mix effects and creative ideas: how the specification is written (2026-11-14, P-018, decided by the author)

**Context.** After an analysis of the effects that could be added to the Mix, the author chose to include them
all except three: EQ with more bands and narrow notches, reverse and freeze reverb and delay, lo-fi and bitcrusher. He also
included shimmer, harmonizer and doubler, which FOUNDATION §2 excluded, and left out only the fake double.
He asked to write the specification first and then the boards, to be approved before the implementation.

**Choices made autonomously** (to be confirmed with the boards):
- **Block chain in free order, like the master** (FX-10), instead of "free slots" after the fixed chain. The
  gate goes before everything, the de-esser after the compressor and the modulation at the end: with fixed slots half of the blocks
  would be in the wrong place. The architecture of P-017 is reused (list, constant latency, changes without clicks). The
  five classics remain the starting point and the migration from schema 3 preserves the hash. Ceilings: 2 per type and 12
  blocks per track, 6 per return.
- **Doubler and fake double.** The doubler is an effect inside the track, with slightly detuned and delayed voices
  mixed with the signal. The excluded fake double is a copy on another track, out of phase and panned, and it is still
  out because it creates a track (AI-17).
- **Track clipper with absolute threshold** (FX-17): the song-measured threshold of the master clipper (MAS-11) on a
  track would have to be remeasured at every preceding parameter.
- **Equal latency for all tracks** and independent of the chain: adding a harmonizer must not shift
  a track relative to the others. It costs a fixed playback delay of a few milliseconds.
- **Mono until the first stereo block**: after a chorus or an auto-pan the pan of a mono track becomes
  balance. Making all mono tracks stereo would cost CPU while giving nothing.
- **Oscillator phase tied to the position in the song** (FX-20, FX-23), not to the start of the render as in the reverb
  modulation: a tempo-synced tremolo must sit on the beat even when starting from mid-song.
- **Compact v4 contract.** The plan carries only the blocks it adds or changes (the master lists them all), because
  with 30 tracks the response would get too long (A-062). The model does not move or remove blocks: it adds at a
  named position and turns off with the bypass. AI limits tighter than those of the interface.
- **Creative ideas are additions only**, so that removing the checkmark of an idea leaves the rest of the plan identical,
  like the new elements of AI-20. Checkboxes **off by default**: a creative idea is a matter of taste and the author makes it.
  The loop evaluates the plan without ideas, because no measurement says whether a chorus succeeded.
- **Return chain only with structural changes allowed**: a return is shared between tracks.
- **Three phases** (F15 foundations, F16 processors, F17 AI). The boards of all three are drawn at the start of F15 and
  the implementation awaits approval, as in F13.

**Discarded alternatives.** A generic "FX" block with a menu of effects inside: it would read worse in the marks and
in the Overview. Creative ideas evaluated in the loop: there is no measurement. Ideas on by default: the A/B would play
something the author has not chosen.

### A-086 — Mix effects and creative ideas: boards 16a–16u, 4c, 4d, approved (2026-11-14, P-018)

**Context.** P-018 asks for the boards of all three phases before the implementation (FOUNDATION §8.1, F15).

**Design choices.**
- **The chain lives in the inspector**, as a compact list (24 px rows) above the editor of the chosen block. There are no
  new columns nor a wide panel like the master rack: a track's chain is viewed next to the
  timeline. Trim at the top and Fader · Pan at the bottom have the padlock; the sends go down to a single row at the bottom.
  It replaces the chain's button grid (3a), which would not fit with twelve blocks.
- **Each row has its own badge**, the same as in the headers and strips (UI-8): whoever reads a chain in the mixer
  finds it the same in the inspector. New badges: `GT DS DEQ TR CLP MB EXC BAS WID MOD TRM HRM DBL`. A block with sidechain
  takes the color of the track it listens to, like today's compressor.
- **The "Add block" menu sits next to the inspector**, with four columns (Dynamics, Tone, Space & movement, Pitch). Each
  block says in words what it does, and what cannot be added says why. Space and Movement are in a single column
  because Space has only one block.
- **Each editor says what it does to the sound**, besides the parameters: where the de-esser works, when the dynamic EQ works,
  where the gate closes, the hit before and after, the added harmonics, the harmonizer's scale. The harmonizer warns
  that the intervals are fixed and do not follow the key.
- **Blocks that sound only in some sections** carry the section name in the row (`Choruses`, `Bridge`).
  Those added by a proposal carry `New` until applied.
- **The marks with long chains** show the first four badges and a `+n`; the tooltip names the others. In the Overview there is
  a row for every block type present in the project, and with many types the grid scrolls.
- **The creative ideas** are a sub-section of the proposal, three cards side by side, each with checkbox, rationale,
  elements, sections and "Listen". Board 4c shows one checked to show the state, but the default stays off
  (AI-27).
- **The assistant's two checkboxes** sit on one row: "Allow · structural changes · creative ideas".

**Open for the author.** The layout in the inspector versus a wide rack like the master's. The number of
blocks to show in the marks before the `+n`. Whether ideas should also have a listen once the proposal is applied.

**Approval.** The author approved the boards on 2026-11-14 as they are: the chain stays in the inspector, the
marks show four badges before the `+n`, listening to the idea alone exists only with the proposal under review.
F15 → F17 are executed in sequence.

### A-087 — F15: the block-based track chain, how it is built (2026-11-14, P-018)

**Data.** `Chain` = trim, `blocks` (list of `FxBlock { id, fx }`, `Fx` tagged by type), fader, pan; a return
uses the same list after its processor (FX-11, at most 6). **A block's id starts with the key of its type**
(`hpf`, `eq`, `comp2`, `gate`, `deq`…): the migration from schema 3 gives the five classics the ids `hpf`, `lpf`, `eq`,
`comp`, `sat`, so the parameter ids (`eq.2.gain`, `bypass.comp`) and the automation lanes stay valid without
rewriting them, and from a parameter's id alone one reads the type and therefore its range. A block parameter is
`<id>.<key>`; a band or voice index is `<id>.<n>.<key>`; the bypass `bypass.<id>`. Project schema **4**.
`Filter` has the `resonance` (default 1/√2, the Butterworth of before); `Saturation` the `character`; the reverb
`shimmer_*`, the delay `mode`, `character`, `wow`, `drive_db` (with serde defaults, used by F16). The structure
changes are `FxOp` (`Edit::Chain`): add, remove, move, duplicate, reset to initial values, bands or voices;
removing a block removes its lanes. The non-continuous values of a block (modes, character, voices, sync) go through
`Edit::SetBlock`, which receives the whole block. An added block arrives **in bypass** and before the fader (board 16b).

**Identical hash.** Schema 3 projects render with the same hash: verified by a test (`schema3.rs`) on two
projects written and rendered by the schema 3 build (`app/fixtures/schema3/`, full chain with automation,
sidechain, parallel group, nested returns: a delay that goes into a group that sends to a plate), at blocks of
4096, 64 and 777 samples. To get there the engine runs the classic blocks with the same computation as before: a block
fully off is skipped, one fully on is not blended, the saturation always runs with the mix at zero when it is
off. The **mixdown** also stays up to date: if the recorded mix hash matches the one schema 3
computed (redone with a copy of the types of that time), after the migration it is replaced with the new hash.

**Constant latency, per class.** All the track and group chains take the latency of the slowest of them, all
the returns that of the slowest return; the graph aligns the classes with each other as it did (ENG-5). Adding a block
with latency never shifts a track relative to the others. Discarded alternative: a single latency also for the
returns. Giving the returns the tracks' latency shifts the reverbs' input in render time, and their
modulation, which starts at the beginning of the render, would change phase: goodbye identical hash. FOUNDATION FX-10 says "the same
for every track, group and return": it is true within each class, and between the classes nobody shifts. The overall
latency changes when the slowest chain changes: the graph is rebuilt anyway.

**Bypass with fade.** The bypass value slides linearly over 10 ms per sample, from whoever it comes (hand or
automation); a still bypass gives the computation of before. The only difference from schema 3: the bypass lanes pass in
10 ms instead of 32 samples (FX-10 asks for it).

**Structure changes while listening.** A structure change rebuilds the graph and swaps it with the fade-out
and fade-in that the engine already had. **Defect found and fixed**: after a swap the fade-in
started immediately, but the first samples of a zeroed graph are the silence of its latency: the signal entered
mid-ramp, with a click of about 0.1 on a 0.3 sine. Now the ramp starts after the latency (test
`structural_changes_while_playing_do_not_click`).

**Mono and stereo.** A mono chain becomes stereo at the first stereo block (width, modulation, auto-pan tremolo,
harmonizer, doubler), whether it is on or off (it is structure, not state). From there the pan acts as balance with the
mono pan law applied to each channel: a neutral stereo block leaves the track identical, also in the level at
center (verified).

**Sidechain.** Compressor, gate and the dynamic EQ's bands listen to another track or a group, also from the chain
of a return (ducking). Each key is aligned to the block that uses it (latency of the blocks before it); the core
rejects as before sources that are returns and cycles.

**Catalog.** The core exports `fx_catalog.json` (badge, name, family, new block, path of every parameter
inside the block, found by trying to change it): the interface and the fake backend use it for menus, badges, values and
parameters, without repeating them.

**Interface: differences from the boards.**
- Rack rows: the gain reduction bar per block is not there (the engine measures the reduction per track,
  not per block).
- In the Overview view a block type that a track does not have reads "off" as in board 16u, but is not
  clickable; a return puts filters, state and processor time in the rows that its chain leaves empty.
- Two blocks of the same type sit in a single cell, with the first and "×2".
- The editors of the new blocks arrive in F16; in F15 a new block shows one knob per parameter.

**Verification.** Core: id, keys, limits, `FxOp`, catalog, migration and updated mixdown. Engine: schema 3 hash,
10 ms fade, latency that does not shift the tracks, resonance (Q 8 = +18 dB at the frequency), ducking from the chain
of a return, mono turned stereo, free order independent of the compute block, no click when changing
structure during playback. Interface: 5 Playwright tests (`chain.spec.ts`), Vitest on marks, names and sections; captures
16a, 16b, 16c, 16p, 16t, 16u.

### A-088 — F16: the new blocks, how they are built (2026-11-14, P-018)

**Who wrote them.** The processors of FX-13 → FX-23 were written by six subagents on disjoint files of the DSP crate
(gate, de-esser and transient; dynamic EQ; saturation characters, exciter and bass enhancer; modulation, tremolo and
width; pitch shifter, harmonizer, doubler and shimmer; ping-pong and tape delay), starting from a written contract
(signature, parameters, latency, FX-24 properties); the orchestrator reran the tests, read the code and did
the integration in the engine. Each algorithm is described in `docs/app/dsp.md`.

**Choices in the processors** (to be heard in the validation):
- Dynamic EQ: the band reaches its full range at 6 dB above the threshold; below the threshold it is exactly off.
- De-esser: fixed ratio 10; `split` lowers `x − LP(x)` (exact complement): below the frequency the signal does not
  change (±0.1 dB at 500 Hz with 6.5 kHz). The **listen to the detected signal alone** is an engine-only state
  (`<id>.listen`, command `listen_block`): it is not a document parameter, so the offline render and the export cannot
  have it; a rebuilt graph restarts without it, and closing the editor turns it off.
- Transient: fixed dead zones (1.5 and 0.5 dB) and ramps (10 and 6 dB) on the ratio between fast and slow envelope; below
  100 Hz a steady tone may move slightly.
- Saturation: `tape` = the soft curve and a 12 kHz low-pass in the saturated branch; `tube` = bias and
  10 Hz high-pass; `transistor` = harder knee. `soft` is the code of before, unchanged (identical hash).
  Changing character has no fade of its own: it is a structure change (graph swap with its fade).
- Pitch shifter: two taps on a delay line with cosine fade; to stay within ±5 cents, when a tap
  jumps it aligns by correlation to the other (a search bounded at every jump: a peaky but bounded cost, measured
  by the benchmark). Harmonizer: 35 ms window, constant latency 864 samples at 48 kHz (18 ms); the presence of a
  harmonizer brings the whole track class to that latency (A-087).
- Doubler: the moving delay follows the position in the song; the pitch shift does not (its pitch shifter
  starts from the start of the render). FX-20 asks it for modulation and tremolo, and they respect it.
- LFO phase: anchored to the position at the first block and accumulated per sample: a render starting mid-song
  differs from the full one by some 1e-5 (below −80 dB), because of the rounding of the two sums.
- Delay: the drive in the feedback exists only in the `tape` character; `stereo` + `clean` is the code of before, unchanged.
- A block with latency that is off does not run: on turning it back on the 10 ms fade covers its old state.

**Interface.** One editor for each new block (boards 16d–16s): a drawing computed from the settings (gate
curve, de-esser zone, dynamic EQ band at rest and at maximum, hit before and after, harmonics, stereo image,
LFO, harmonizer scale), the controls, a line that says what it does. Differences: the drawings do not show the live
reduction (the engine measures the reduction per track, A-087); the delay and shimmer live in the return's panel, with the
`SetReturn` edit for mode, character and interval.

**Verification.** 166 tests of the DSP crate (FX-24 properties for every block), `no_alloc` with all the blocks; engine: neutral
blocks that leave the render identical, chain with all blocks at work independent of the compute block,
modulation, tremolo and tape tied to the position, de-esser listen; schema 3 hash unchanged; FX-24 benchmark:
p99 27% of the period (threshold 50%); NFR-1 p99 12.8%; NFR-4 30×. Interface: 3 Playwright tests
(`effects.spec.ts`) and captures 16d–16s.

### A-089 — F17: creative ideas and blocks in the plan, how they are built (2026-11-14, P-018)

**Measurements (ANA-19).** `crates/stemwise-analysis/src/character.rs`: vocal sibilance, short crest, drum transients
and noise floor, computed with the other measurements and cached. Sibilance compares the 5–10 kHz band in 5 ms frames
with the **vocal body below 5 kHz** over the surrounding 300 ms: the first version, which compared it with the whole
voice over 50 ms, could not exceed 0 dB (the body already contained the band) and the flow test showed it. Expected
values: about −20 dB for a soft voice, 0 and above for a sibilant one; the prompt and the fake decider suggest the
de-esser above about −3 dB. Initial threshold, to be tuned (VALIDATION 52). The analysis lists only the non-classic
blocks (neutral ids `k…`): the classic ones live in the v3 fields.

**Plan v4.** Blocks per track, group and return (`blocks`, with `$defs` per type and `anyOf` in the strict schema),
resonances, saturation character, new reverb and delay fields, per-section offsets of the octave filters and
`blocks_on`. A filter offset acts on the **first enabled filter** of that type, not on the first one overall: this way
an idea can add its own resonant low-pass (the classic one stays off) and move it between sections without changing
what already exists (AI-26).

**Ideas.** `ideas.rs`: `take` removes them from the plan at the first iteration, the loop evaluates the plan without
ideas, `merge` puts them back at the end; `strip` keeps only the ones ticked at the time of applying and listening. A
new return belonging to an idea is created only with the idea ticked. UI-13: the idea's name (with its rationale) is
added to the **note of the track** it touches, which is the rationale text and the tooltip of the number in the header;
blocks do not carry a tooltip of their own with the idea's name (it would take a document field for a single label).
The history step names the applied ideas.

**Fake decider.** De-esser, gate, dynamic EQ keyed by the voice, width, ducking on the shared reverb; ideas `i1` chorus
on the keyboards in the choruses, `i2` filter that opens toward the last chorus (now with its own resonant low-pass: the
first version required an already enabled low-pass and never fired), `i3` tape ping-pong delay on a new return.

**Interface.** "Creative ideas" sub-section (boards 4c, 4d) with a tick, listening to the idea alone (which counts as
listening to the proposal in the A/B) and sections; "new block" rows in the diff; "Allow · structural changes ·
creative ideas" as in board 3a. CLI: `automix --ideas i1,i3|all`.

**Independent review** (`app-reviewer` on F15–F17): no blockers, seven defects fixed.
1. A revision lost the rise of the filter of an applied idea: re-reading looked for the lane on the first filter,
   writing on the first enabled one. Now both use the same filter (`playing_filter`); covered in `ideas_flow`.
2. A `null` field of a block now keeps the value also for bands, voices, sidechain and rate (before, it emptied the
   bands and made the plan fail in strict). In JSON mode the number of bands and voices is also checked (§5.5).
3. Iterations after the first receive the ideas decided in the document (`decided_ideas`) and a line in the prompt. On
   merge nothing in the kept plan is replaced: an already used block id is renamed, an idea's offsets for a section
   that the plan uses stay apart (`idea_sections`, outside the schema) and are added only with the idea ticked, a block
   beyond the three new ones per chain stays out with a warning.
4. An idea's sends, also from groups, go only to returns and within the limits.
5. An idea's new return no longer appears among the "will be created" elements: it is in the idea's card and follows
   its tick (`NewElementView.idea`).
6. In an idea's section `blocks_on` accepts only the idea's blocks; the others are removed with a warning.
7. Engine: a parameter moved while the old graph was fading out went to the new graph's slots but on the old one. Now
   `Msg::Param` carries the graph id and goes to the one it was resolved for (even if it is still pending).
New blocks at the same position also keep the plan's order. Left among the Caveats: FX-24 verified at 1e-4 and the
doubler not tied to position; UI-13 without a per-block tooltip.

**Verification.** `tests/ideas_flow.rs` (ideas decided once, outside the evaluation, applied only if ticked, undone in
one step; switch off), client test against the fake server with the v4 schema, full flow from the CLI, Playwright
`creative ideas` with captures 4c and 4d, self-test in the packaged app (49 steps: listening to a single idea, idea
applied in the same step; cold start 1.2 s). The self-test must be run with the window visible: with the screen off,
drawing stops and the cold start comes out at minutes.

### A-090 — Verification of P-018 after closing: three blockers and eleven defects fixed (2026-11-14)

**Context.** With the phases closed and the gate green, a verification with three independent reads (core and engine,
DSP, AI and interface) found defects the tests did not see. The first was reproduced before fixing it; each fix has its
own test.

**Blockers.**
1. **Render dependent on the block after a bypass switched off and on again** (ENG-4, FX-10). The crossfade ends in the
   middle of a 32-sample cell and the processor fell asleep "from the next piece": how many samples it had heard
   depended on the compute block boundary (differences up to 2e-3 with blocks from 12,481 to 12,511). Now sleep is
   decided once per cell, the mix is exact at the two extremes and a block that turns back on restarts clean (before,
   it fed back the audio from when it had turned off). Alternative discarded: making the crossfade end on a cell
   boundary (it changes the duration with the sample rate and does not remove the old state).
2. **Modulation and tremolo phase with the rate automated** (FX-20). After a seek the phase was `rate × position`, in
   continuous playback the sum of the increments: with a lane on the rate, listening from mid-song and export
   differed. Now the engine gives the block the cycles elapsed since the start of the song, computed from the lane.
   Alternatives discarded: removing rate automation (against AUT-1); recomputing `rate × position` at every cell (a
   moving rate would make the phase jump by whole cycles). **Caveat**: a rate changed by hand while playing continues
   from the accumulated phase and realigns at the next start.
3. **An idea's return did not stay still in the loop** (AI-26). From the second iteration it was in the schema like any
   other return: the model had to rewrite it and at merge its parameters won over the idea's. Now later iterations
   receive schema, document and validation without that return.

**Defects.**
- Core: a chain's limits (FX-10, FX-11) also apply to `SetTrack`, `InsertTrack` and at load: a hand-written
  `project.json` with more than eight keys or an inconsistent multiband led to a panic, the first on the audio thread.
  `SetBlock` removes the band and voice lanes the block no longer has.
- AI: a resonance or a block set by hand beyond the plan's limits (the interface goes further) no longer makes a
  revision fail if it leaves them as they are (AI-3, like the classic fields); reapplying a proposal does not
  duplicate the new blocks (AI-23); an idea's section offset may be on the send the idea itself adds (the "throw" the
  prompt suggests); an idea block that no longer has room in the chain stays out of the proposal with a warning
  instead of being listed and then skipped; two blocks with the same id in a plan chain are an error.
- DSP: gate and dynamic EQ on a mono chain read all the key's channels; the de-esser's detector has its own high-pass;
  the exciter no longer digs at the cutoff frequency (band-pass of the dry, `docs/app/dsp.md`); the pitch shifter's
  search has a cost that does not grow with the sample rate.
- Gate: `fx_catalog.json` and `param_specs.json` have a freshness test (`stemwise-cli/tests/bindings.rs`).

**Left as is, among the caveats.** The **bass enhancer** has no oversampling or level compensation, against the letter
of FX-18: it generates harmonics with polynomials up to the fourth degree from a band below 160 Hz, so nothing above
640 Hz that a 4x could protect, and adding harmonics to the dry raises the level by construction. The **transient
shaper** on a steady tone below 100 Hz can move the gain (already in A-088). In **split** the de-esser still lowers by
about 0.7 dB one octave below the frequency at 12 dB of reduction (6 dB per octave complement). The benchmark runs
only at 48 kHz. The chain limits are repeated in `ui/src/mix/fx.ts`; the Playwright tests of the ideas test the fake
backend.

### A-091 — Auto mix with ideas: the `null` placeholders of `ideas` and `idea_sends_db` are read as absent (2026-11-14, from the author's test)

**Context.** On AndrewCole_DeadRoses_Full (33 tracks, `openai/gpt-6.1-sol`) the auto mix with ideas failed after three
attempts with "invalid type: null, expected struct IdeaPlan" ($0.52, 6 minutes). The three responses were complete
(`finish_reason: stop`) and respected the strict schema, which lists `i1`–`i3` and allows `null` for an unused idea
and for an absent send. The parser accepted `null` in the top-level maps (`nonnull_map`), but not in `ideas` and not in
the inner map of `idea_sends_db`: the first two attempts failed on `"i1": {"n1": null, …}`, the third (the model had
replaced those `null`s with −60) on `"i2": null`. The error message sent back to the model talked about a `null` that
the schema declares valid, so the retries could not succeed. The tests did not see it because fake decider and fake
server only sent `i1`.

**Alternatives.** (a) Remove `null` from the ideas schema: with strict every key is required, and the model would have
to invent three ideas. (b) Ideas map as an array: changes the `mix_plan` v4 contract. (c) Read the `null`s as absent,
as for the other maps.

**Choice: (c).** `ideas` uses `nonnull_opt_map` (a map with no values left is equivalent to no ideas, so the warning
about disallowed ideas does not appear); `idea_sends_db` uses `nonnull_nested_map`, which removes the `null`s at both
levels. Tests: `unused_idea_ids_written_as_null_are_absent`, `idea_sends_written_as_null_are_absent_at_both_levels` and
the client test against the fake server, now with `"i2": null, "i3": null`. Replayed in the real loop, on a copy of the
project, the three recorded responses each become a proposal at the first attempt (57 commands, idea "Soft Tape
Echo"). For the third, four warnings remain: the idea's sends at −60 touch already existing sends and are discarded, as
foreseen by AI-26.

### A-092 — Explain this mix: signal map and rule-based explanation, boards 17a–17j (2026-11-15, P-019, requested by the author)

**Context.** The author: the app is still hard to read, for a newcomer and for himself; it is not clear what goes where,
who does what and why. The marks of F13 (UI-8 → UI-11) show the **state** to those who already know how to read it;
missing are the **flow** (routing is reconstructed one track at a time) and the **intent** (the `rationale`s stay in
the proposal). The author proposed an "explain this mix" button, thinking of an embedded local model.

**Alternatives.** (a) Only touch-ups to what exists (richer tooltips, a one-sentence path in the inspector): useful but
they do not give the overview. (b) Explanation written by a model, local or via OpenRouter: better prose, but download
and memory (local) or cost and a connected account (remote), and the risk that a small model gets the routing wrong.
(c) Signal map plus **rule-based** text on the data the project already has.

**Choice.** (c), decided with the author: for now **no model**. The proposal is **P-019**: an "Explain this mix" view
with the map on the left and the text on the right, both derived from the project document and the analysis. Boards in
`design/screens/17-mix-explain.html`, rules in `design/README.md` ("Explain this mix" section). Design choices:

- **View and not modal**: it takes the place of the workspace but leaves the top bar alive, so one can listen while
  reading. `⌘E` was free in the shortcuts table.
- **"Shared effects" next to "returns"**: in the text and in the map the returns are called with the word a newcomer
  understands; "returns" remains as a secondary label.
- **Reasons are quotes**: the rule-based text describes; the why is only what the assistant wrote in the applied
  proposal. No generated advice.
- **Map by families** (those of AI-22) and collapsible groups, to hold up 30-track projects.

**Reason.** 80% of the explanation is facts from the document: rule-based it is exact, instant, offline, the same for
the same project, and it respects the principles by itself (no audio or file names to a model, no secrets). It also
becomes the fact base for an eventual narration with a model, postponed.

**Status.** Boards approved by the author on 2026-11-15 and implemented in F18 (A-093); FOUNDATION updated (P-019,
ANA-20, UI-14 → UI-16). Data the implementation will have to expose and that the UI does not receive today in this
form: loudness per family and per section, average reduction of each compressor, clips present per section.

### A-093 — Explain this mix: implementation choices (2026-11-15, F18, P-019)

**Context.** Boards 17a–17j approved by the author on 2026-11-15. What remained to decide was where to compute the
explanation and where to take the levels from.

**Choices.**

- **Map and text derived in the interface** from the project document (`ui/src/mix/explain/`: `model.ts`,
  `layout.ts`, `text.ts`), like the marks of P-013: they are readings for drawing, not domain logic, and this way the
  fake backend does not duplicate them. Families come from the core (`role_families.json`, generated from
  `family_of`).
- **Levels measured by the core** (ANA-20): `measure_levels` reuses the AI-22 balance renders and adds the loudness of
  the mix and of the sections. `mix_levels` command: answers immediately if the mix is already measured, otherwise
  measures in a thread and announces with the `mix-levels` event; a measurement on a superseded mix is canceled. In
  memory, not in the project. A failed measurement means "no levels": the "Who is in front" part does not appear.
- **By family, not by track.** The boards said "the loudest track, 1.4 LU above Kick": it would take one render per
  track (30 renders on a big song). The text uses families, already measured.
- **No measured reduction of the compressors.** The boards said "about 3 dB on the peaks": the measurement does not
  exist outside the master. The sentence gives ratio and threshold.
- **`Esc` in two steps**: first it goes back to the whole mix, then it closes. The boards only said "closes".
- **Group sending to a return**: dashed line from the group's foot to the return below; from a deeper column the line
  comes back. It was not in the boards.
- **Block turned on by a bypass lane**: counts as on in the sections where it plays, even if at rest it is off.
- **Map measurements**: rows of 30 px as long as the map fits in the available height, then 26; 22 in the compact
  window. Gaps between families reduced compared with the board (14 px) because the AI-22 families are six, not five.

**Verification.** 15 Vitest tests (map, layout, sections, text), 6 Playwright (view, map, folds and sections, text and
levels, card and copy, accessibility and minimum window), 1 Rust test on the levels. **Not run**: the view in the Tauri
app with the real measurement, `make build` and the packaged self-test (VALIDATION 55–56).

### A-094 — Guided tour: specification of P-020 (2026-11-15, issue #5, decided with the author)

**Context.** Issue #5: at the very first launch a guided tour must start, skippable and repeatable from the menu, on a
project prepared for the purpose, embedded in the app and not saved to the user's disk. Analysis done in a session
separate from the implementation; the specification is in FOUNDATION (P-020, PRJ-9, UI-17 → UI-19, screen 18, phase
F19).

**The author's choices** (four questions, 2026-11-15).

- **Audio synthesized** by the code, not real stems in the package nor a tour without sound: no audio file in the
  repository, no rights to manage, the real engine plays. It sounds synthetic: the author judges it.
- **Full round** (Mix, assistant, proposal, Explain, Master, export) and not only the Mix nor separate chapters.
- **Guided with Next**, with the lit area usable, and not steps that wait for an action.
- **Boards and approval** before implementation, as for F13, F15 and F18.

**The agent's choices**, to be reviewed with the boards.

- The engine reads clips from disk: "not saved to the user's disk" becomes a package in the **app cache**, deleted at
  the end of the tour and, after a crash, at the next launch. Out of recents, autosave, saving and export.
- "Starts automatically" is the **opening card** shown by itself, with `Start` and `Skip tour`: the project prepares
  while it is being read and the cold start does not change.
- The tour starts by itself when the **"shown" mark is missing**, even if there are recent projects: a single rule, and
  the only existing installation is the author's. The mark is written at the first step.
- On request: new **Help** menu in the menu bar and link in the Welcome. With a project open the tour closes it (with
  the usual confirmation) and at the end returns to the Welcome, without reopening it.
- The proposal shown is **always from the fake decider**, even with a key configured, and does not enter the cost
  log: the tour does not spend and does not use the network.
- The demo project is built in the **core** (the UI does not duplicate domain logic); on the fake backend the tour uses
  the board song that already exists. The steps find their targets by role and type, not by id.
- Out of scope (FOUNDATION §2): the demo project openable outside the tour, to keep the issue as written ("closed
  automatically at the end").
- The new requirements are assigned to F19 in `app/scripts/req-phases.json`, so `make check` stays green while the
  phase is open.

### A-095 — Guided tour: the boards of screen 18 (2026-11-15, F19, to be approved)

**Context.** First half of F19: boards of screen 18 and the speech bubble component (FOUNDATION §6, UI-17 → UI-19),
before any app code. The specification is silent on appearance, positioning and several states; the agent's choices, to
be reviewed with the boards.

- **Boards 18a–18w and C8** in `design/screens/18-tour.html` and `design/components/index.html`, generated by
  `design/tools/s-tour.mjs`. Beyond the list in §6: a second waiting step (mixdown, 18q, besides the proposal, 18p), the
  tour requested with a project open (18u) and the demo song that cannot be prepared (18w), because UI-17 and UI-19
  describe them.
- **Final texts** of the fifteen steps in `STEPS` of `s-tour.mjs` (title, one or two sentences, an optional `Try it`
  line on steps 2, 5 and 8). Alternative discarded: longer texts with lists; the tour must stay within the promised
  three minutes.
- **The boards' song** is the example one cut to 68 s with three sections (Verse, Chorus, Outro) and the twelve tracks:
  within the 8–12 stems of PRJ-9 and with everything the tour names. The real content is decided by the core in
  implementation.
- **Ring in the phase accent**, not in `--ai` blue: the tour is not the assistant (design principle 2). `--tour-dim`
  dimming lighter than the dialogs' scrim: the context around the area stays readable.
- **Bubble position computed by a rule** (preferred side, then right, left, below, above; inside the area when no side
  is enough) and not written step by step: it holds the same at the minimum window and above 1920 px.
- **Export dialog moved to the right edge** during step 14, without its scrim: centered it leaves no room for the
  bubble (340 px on the left at 1440, 260 at 1280). Alternatives discarded: bubble over the dialog (covers its
  content), lighting only the formats (the step also talks about the metadata).
- **Explain (step 10) lit in full** with the bubble inside, bottom left: map and text together are the point of the
  step.
- **Steps 12 and 13 split chain and target**: the target is in the Loudness card (D7) and a single box including both
  would cover almost all of the Master. Step 12 names it, step 13 lights it.
- **A step sets only the state it declares**; what the user changes in the lit area stays until the end of the tour.
  Going back undoes only what the next step had added (from 9 to 8 the proposal goes back into review with an undo).
- **Tour requested with a project open**: the same close confirmation (`Save changes to “…”?`) with a line on why;
  `Cancel` does not start the tour.
- **Link in the Welcome** under the two buttons, with the duration, on all variants (1a, 1b, 14). The `Take the Tour`
  item is off while the assistant works, during a bounce or an export; where it lives is said by A-096.
- **Notification after `Skip tour` and `Esc`**, not after `Done`: whoever finishes the round has already read, in the
  closing card, where to find it again.
- New tokens in `design/tokens.css`: `--tour-w`, `--tour-card-w`, `--tour-pad`, `--tour-gap`, `--tour-edge`,
  `--tour-arrow`, `--tour-dim` (in the two phases), `--z-tour`.
- The board generator accepts a different song in the timeline, top bar, Explain section strip and Master waveform; the
  existing boards stayed identical, except 1a, 1b and 14 which now show the link to the tour.

### A-096 — A single app menu (2026-11-15, F19, requested by the author)

**Context.** Reviewing board 18t (new Help menu in the macOS menu bar) the author noticed that the commands are split
between the macOS menu bar and the menu that opens from the project name (A-053), and asked to gather them in the app
menu, with Help as a new item and with all the app's items that today are in the system bar. Check done on `app_menu`
in `app/src-tauri/src/main.rs` and on `project/ProjectMenu.tsx`: in the system bar there were, besides the macOS
items, `Settings…` (⌘,) in the Stemwise menu and all of File (New Project… ⌘N, Open… ⌘O, Close Project, Save ⌘S, Save
As… ⇧⌘S); in the app menu only Project settings, Save, Save as and Close project.

**Choice (UI-20).** The app menu contains, in this order: `New project…` ⌘N, `Open…` ⌘O · `Project settings…`,
`Save` ⌘S, `Save as…` ⇧⌘S, `Close project` · `Settings…` ⌘, · `Help ›` with `Take the Tour`. The macOS bar keeps only
the system Stemwise menu (About, Services, Hide, Hide Others, Show All, Quit), Edit (without it, copy and paste in text
fields do not work in the webview) and Window. Board 18t redrawn.

**Consequences for the implementation (second half of F19).**
- ⌘N and ⌘O today go through the system bar: they enter the single shortcuts table (UI-3), like ⌘S, ⇧⌘S and ⌘, which
  are already there. The app menu shows them next to the items.
- `Settings…` leaves the system Stemwise menu, where macOS usually puts it: the author's choice, ⌘, stays.
- The A-053 Playwright test that opens and creates projects "from the simulated menu bar" moves to the app menu; the
  VALIDATION item on the native bar becomes the check that only the system items remain there.
- On the Welcome there is no project name: New project, Open project, Settings and the tour are already buttons and
  links on the screen.

### A-097 — Guided tour: the implementation (2026-11-15, F19)

**Context.** Second half of F19, after approval of boards 18a–18w and of A-096. The agent's choices where FOUNDATION
and the boards are silent.

**Core (PRJ-9).**
- The demo song lives in `stemwise_core::demo`: twelve stems synthesized from closed formulas (chords A, F♯m, D, E at
  120 BPM, SplitMix64 noise with fixed seeds, phase accumulators and smoothed levels so that no note change clicks),
  written at 16 bits in the package and **adopted** as imported media with `media::adopt` (moved, not copied, named by
  the hash); the document is built with the same `Edit`s and `Command`s as the app. The twelve syntheses run in
  parallel: in release the build takes under a second (before the parallelization 4 s, almost all for acoustic guitar
  and Rhodes, now computed with one exponential and one `sin_cos` per note).
- The CLI's `demo` command stays as it was (its tests depend on it): the tour song is another thing.
- Content beyond PRJ-9: tempo 120/4, sections Verse 0–24 s, Chorus 24–52 s, Outro 52–68 s; Gtr Left and Gtr Right are
  two different performances of the same part (true double for ANA-18); an "s" at the start of each vocal phrase gives
  the de-esser work.

**Session.**
- The package lives in `<app cache>/tour/Demo song.stemwise` (`~/Library/Caches/app.stemwise.desktop`; with
  `STEMWISE_DATA`, in `<data>/cache`). It is deleted at every close of the demo project (also when opening another
  project, at the end of the session and on app exit with `RunEvent::Exit`) and at launch.
- The tour does not use `ai_start`: the fake decider's proposal is prepared on its own thread as soon as the song is
  open and stays aside, so steps 2–7 show the assistant without a proposal. Step 8 puts it into review
  (`tour_proposal`); if it is not ready the assistant shows the work (18p) and the proposal enters by itself. A copy
  stays, so going back from step 9 to 8 puts it back into review after the undo.
- A tour finished while the song is being built throws it away (`epoch` counter): no project opening after Skip.
- The "shown" mark is `tour.json` in the data folder; the tour starts by itself if it is missing and if it is not the
  self-test.

**Interface.**
- The tour lives in `app/ui/src/tour/` (texts in `text.ts`, step machine in `machine.ts`, positioning in `place.ts`,
  the rest in `Tour.tsx`). Targets are found by accessible name and role (`aria-label` of the areas, `role="dialog"`,
  `role="radiogroup"`), clipped to the visible part of the scrolling panels.
- Clicks outside the spotlight stop on four blocks around the hole; shortcuts stop in a capturing `keydown` listener.
  In the text fields of the lit area (the request to the assistant) the keys remain the user's, except `Esc`.
- The "Try it" text of step 8 said "press B": `B` is a shortcut, suspended during the tour (UI-18). Now it says to
  switch between Current mix and Proposal; boards updated.
- Notifications are not shown during the tour (they sit under its layer and would talk about the demo song, which
  disappears): they are discarded. After `Skip tour` and `Esc` the toast of 18v, of a new `info` type.
- The system menu loses File and Settings (UI-20): ⌘N and ⌘O enter the shortcuts table ("Project" group) and work on
  every screen; the backend's `menu` event and `onMenu` are removed.
- The fake backend has the same interface (`tourInfo`, `tourMarkShown`, `tourStart`, `tourProposal`, `tourEnd`) on the
  usual song, renamed "Demo song" and without proposals; the tour starts by itself only with `?tour`.

**Verification.** 7 Rust tests (`demo` in the core: same document and same files; content; render with the same hash
in the engine; in the session: folder in the cache, no recents, autosave, saving, export, files left behind; leftover
deleted at launch; tour finished during the build; "shown" mark; fake decider's proposal even with a key and without
recorded costs), 6 Vitest (steps, keys, positioning also at 1280 × 720), 9 Playwright (first launch, no relaunch, all
steps forward and back, Skip and Esc, shortcuts and outside clicks, app menu with the save question, minimum window,
axe and focus, song that cannot be prepared, with captures), 10 steps in the real app's self-test.

### A-098 — No changes inside a React state update (2026-11-15, F19, reported by the author)

**Context.** The gate printed `Cannot update a component (Root) while rendering a different component (ChainRack)`
from before F19. Cause: drag reordering in the track rack (16c) and in the Master's (15b) sent the `move` command from
inside the function passed to `setDrag`, which React runs during rendering; the command updated the store while the
rack was drawing, and in StrictMode it could fire twice. The New project screen did the same with the notification
about the maximum number of references.

**Choice.** The drag's drop point lives in a gesture variable; when the gesture ends the state is reset and the command
fires once, outside React. The references notification is given before updating the state.

**Verification.** The Playwright test of reordering in the track rack and a new one for dragging in the Master rack
fail if that error appears in the console; without the fix they fail, with the fix they pass.

### A-099 — Guided tour: verification after phase close (2026-11-15, F19, requested by the author)

**Context.** F19's code re-read and the tour tried in the browser on the fake backend. Core and ephemeral session were
correct; five defects in the tour's interface, which the tests did not see.

**Defects and fixes.**
- **Step 8 got stuck** after `Apply all` or `Discard` in the lit area: "Preparing the proposal…" came back with `Next`
  off forever. Now the step (and step 9) waits only while the assistant is actually working; what the user does with
  the proposal does not hold `Next` (UI-18: no step waits for an action). Step 9 puts it back into review and applies
  it if needed.
- **`Cancel` during the wait canceled the tour's work**, because the assistant showed the cancel flag of the work
  itself: the proposal no longer arrived. Now `Cancel` stops only the wait (`tour_stop_waiting`): the work continues
  and a later step requests it again. The interface's wait ends as soon as the assistant stops working without a
  proposal, instead of staying busy up to 120 s.
- **Enter on `Back` or `Skip tour` went forward.** Enter on a focused button presses that button; arrows on a fader or
  on a choice in the lit area adjust it. The space bar with focus on the bubble goes to the transport (before it
  pressed `Next`, because the bubble is a dialog and the app's shortcuts ignore dialogs).
- **The `Discard` confirmation dialog ended up under the tour**, dimmed and partly covered by the bubble. Now it sits
  above the tour and has the keyboard while it is open (`Esc` closes the dialog, not the tour).
- **The app's shortcuts were not suspended by a text field** (⌘, opened Settings under the tour). In a field ⌘ applies
  only to text editing (⌘A, ⌘C, ⌘V, ⌘X, ⌘Z); the others wait.

**Two other fixes.** Step 8 brings the mix back to its initial state before showing the proposal (a fader moved in the
earlier steps no longer falsifies it, UI-18). Building the song is one at a time: a tour restarted while the previous
song is still being built no longer writes into the same folder.

**Verification.** One Rust test (`cancel_while_the_tour_waits_stops_the_waiting_and_not_the_proposal`), one Vitest test
on the keys, three Playwright tests (step 8 with apply and discard, `Cancel` during the wait, keys on the focused
control). `make check` green: 116 Playwright, 45 Vitest. Package rebuilt (`make build`) and self-test in the packaged
app: 59 steps out of 59, tour song open and playable in 1.1 s, fake decider's proposal in 37 s, cold start 1.4 s. The
self-test goes through commands, not clicks: the five defects in the real app remain to be tried by hand (VALIDATION
60).


### A-100 — Product landing page, in a separate repository (2026-11-16, issue #4, requested by the author)

**Context.** The author chose in canvas the direction "A · One song, start to finish" among four proposals
(<https://claude.ai/artifact/ErqoyT53SxMCznsQsnYbq3>) and approved the hero film (65 seconds, twelve scenes, with one
of his tracks). The landing was built on 2026-11-15 in `web/`; the next day the author decided that the promotional
site must not live in the app's public repository.

**Choice.** The site lives in the private repository `pierodetomi/stemwise-promo` (local folder `../stemwise-promo`):
static page, film, nginx image and Traefik configuration for the VPS, with its own README. `web/`, the `.gitignore`
exception for the track and the README line have been removed from this repository; the rule "no audio file in the
repository" stays without exceptions.

**What stays here.** MIT `LICENSE` at the root, as issue #4 asked. The PoC in `poc/` depends on GPL libraries
(`pedalboard`, `matchering`) without including their code: decided in A-101. The landing points to this repository
(code, releases, license on the `main` branch): the links work only since it is public.


### A-101 — Opening the repository: plan and the author's choices (2026-11-16)

**Context.** The repository is to be made public. State found: `main` stuck at the PoC alone (November 10), the product
on `product/v1`; no audio files or `.env` in the history; Cargo metadata still `UNLICENSED`; no CI or files for
contributors; the PoC imports `pedalboard` and `matchering` (GPLv3) and its `pyproject.toml` already declares
`GPL-3.0-or-later`. Issue #4 was deleted by the author because it concerned the site, now in another repository.

**The author's choices.**

1. **PoC**: stays in `poc/` under GPL-3.0-or-later, with its own `LICENSE` and a note in the README; MIT for
   everything else. The oracle script does not use the two libraries and the fixtures are only numbers. Alternative
   discarded: a separate repository (the oracle would need two checkouts).
2. **Email in the commits**: stays the one that is there; the history is not rewritten.
3. **Language**: README, contributor guide and architecture page in English; FOUNDATION, DECISIONS, PROGRESS and
   VALIDATION stay in Italian, declared as the project diary.
4. **Signing**: the first release ships unsigned, with first-launch instructions in the README.

**Plan.** (1) `product/v1` onto `main` and pending changes committed; (2) licenses: MIT in the metadata, the PoC's
`LICENSE`, list of third-party licenses in the package; (3) complete secret scan, re-read for personal data, `CLAUDE.md`
updated; (4) public README in English with screenshots; (5) `CONTRIBUTING`, `SECURITY`, templates, CI, existing issues
translated into English; (6) `v0.1.0` release with `.dmg` and checksum; (7) repository settings, opening, check of the
landing's links.

**Step 2, licenses (done).** MIT metadata (`app/Cargo.toml`, `ui/package.json`, `tauri.conf.json`); `poc/LICENSE` with
the GPLv3 text and a note in the two READMEs (the only changes to the frozen PoC, wanted by the author: no code
touched); `app/scripts/third-party.mjs` writes `src-tauri/resources/THIRD-PARTY-LICENSES.txt`, the list of the 357
included libraries (355 since A-103) (crates linked for `aarch64-apple-darwin`, production npm packages, fonts) with
the license texts, copied into the package together with `LICENSE`. `make licenses` fails if the list is not up to
date; it is regenerated with `make third-party`. No new tool: the script reads `cargo metadata` and `pnpm licenses`.
Twenty-three libraries do not have a license file of their own in the published package: the list names them with
their license. There is no menu item that opens the list (UI-20 fixes the menu): the file is in the app's resources and
in the repository.

**Name and logo reserved (the author's choice).** `TRADEMARKS.md` at the root removes from the MIT license the name
Stemwise, the mark, the lockup and the icon, with the files containing them (`design/brand/`,
`app/src-tauri/icons/`, the favicon, the `Mark` drawing): it says what can be done without asking, what cannot, and
what a fork's publisher must change. `LICENSE` stays the pure MIT text, so that GitHub recognizes it. The reservation
holds from before the opening: nobody received the repository with the mark under MIT. It is not a registration:
registering a trademark is a step only the author can take.

**Step 3, cleanup and checks (done).** Whole history (all branches, every diff) searched for OpenRouter, OpenAI,
Anthropic, AWS, GitHub, Slack, Google keys, JWTs, private keys and `api_key`/`token`/`password` assignments: a single
result, the test value `sk-or-test-…` in the tests. No dedicated scanner installed (`gitleaks`, `trufflehog`): at
opening GitHub's secret scanning must be turned on. Tracked files and history: no path of the author's machine (only
the fake backend's `/Users/demo/…`), no IP or server addresses, the only emails are those of the authors in the
third-party licenses. What remains, by a choice to be confirmed, are the titles of two of the author's songs in the
documents ("Confidence" in the PoC, "Balance" in PROGRESS and DECISIONS) and two links to private claude.ai
artifacts. `CLAUDE.md` updated: "The repository is public" section, language of the files for outsiders, `main` as the
public branch.

**Step 4, public README (done).** `README.md` at the root rewritten in English: what it does (mixing bench, assistant,
Explain, master, tour), the rule "the model receives no audio", installation with the first launch of an unsigned
build, OpenRouter and costs, what it does not do, building from source, code map and rules, tests, how it was built,
licenses, no warranty, author. Five screenshots in `docs/images/` (JPEG at 1600 px, 1.6 MB in all), taken from the
Playwright captures on the fake backend: no material from the author. `app/README.md` rewritten in English as a map
for developers; `poc/README.md` has a note in English at the top. The README assumes a published release (releases
page, checksum): it becomes true at step 6. The link to `CONTRIBUTING.md` is added at step 5.

**Step 5, community and CI (done, CI not yet run).** `CONTRIBUTING.md` (what to expect, bugs, proposals, code, the
rules a change must respect, license of contributions, conduct), `SECURITY.md` (private report through GitHub, what is
covered), templates for bugs and proposals, pull request template. `README.md` states at the opening that the project
is independent and non-commercial, with no support or guaranteed response times (requested by the author). CI:
`.github/workflows/check.yml` runs `make ci` on `macos-15` at every push to `main` and every pull request. `make ci`
is `make check` without the frame-time tests (`@perf`: a hosted runner has no real GPU) and without the disk-space
cap. **Not tried**: it runs only after a push to `main`; while the repository is private it consumes the macOS minutes
of the free plan (counted ten times). The eight existing issues and their comments are translated into English on
GitHub; the title of one of the author's songs and the reference to the deleted issue #4 were removed from the
translation. At opening, private vulnerability reporting and secret scanning must be turned on in the settings.

**Step 6, first release (package ready, release to publish).** `make build` from commit `d783166`:
`Stemwise_0.1.0_aarch64.dmg`, 10.6 MB, SHA-256 `dc1ca447…5118ed3b`. The package contains `LICENSE` and
`THIRD-PARTY-LICENSES.txt` (`Contents/Resources`), version 0.1.0 and a copyright note. Linker ad hoc signature, no
Developer ID. Self-test in the packaged app on a demo project and a temporary data folder: 59 steps out of 59, cold
start 1.4 s. The `v0.1.0` release (tag on the built commit, `.dmg`, checksum in the notes) is created on GitHub after
the author's consent.

**Release published.** `v0.1.0` on GitHub on 2026-11-16, with the `.dmg` (the checksum reported by GitHub matches the
local one); in the landing the download button points to the file.

**Step 7, opening (prepared; the repository is still private).** Site: <https://stemwise.pierodetomi.dev>. Done:
description, site address and fourteen repository topics; wiki and projects turned off; links to site, download and
issues at the top of the README; 1280×640 preview image in `docs/images/social-preview.png` (must be uploaded by hand
in Settings › Social preview: the API does not allow it). **To do right after opening**, because the free plan grants
them only to public repositories: protection of `main`, private vulnerability reporting, secret scanning; then first CI
run, check of the landing's links, deploy.

**Repository opened (2026-11-16, at the author's explicit request, after his test and the publication of the site).**
`pierodetomi/stemwise` is public. Turned on right after: private vulnerability reporting, secret scanning with push
protection, dependency alerts; `main` protected against force pushes and deletion, also for the administrator (no pull
request required: the author works alone). Verified from outside, without access: the site
<https://stemwise.pierodetomi.dev> (page, `healthz`, preview image), the repository, `LICENSE` on `main`, the releases
page, the `.dmg` (10,602,241 bytes, like the built file) and the reporting page. Left to the author: uploading the
preview image in Settings › Social preview.

### A-102 — Two flaky tests under load (2026-11-16)

**Context.** During the opening of the repository `make check` failed three times out of five, each time for a single
Playwright test, without interface changes.

**Tour (UI-18).** The step's `busy` was set to true by an effect, so one render after the step change: for one frame
`Next` looked ready, and a key pressed at that instant then found the step busy and was ignored (seen between step 14
and 15). Now the state is reset during the render that shows the step: a step with a preparation is busy from the first
frame. A user could not notice it, except for an arrow that did nothing.

**Master preparation overlay (UI-7, MAS-7).** The test waited at intervals for the `is-leaving` class, which lasts 220
ms: under load it missed it. Now an observer in the page, at the instant the overlay starts leaving, clicks what is over
the Export button (the overlay, if it still took the pointer) and the test verifies that it was the button. The test
asks the same thing as before, without depending on the control's timing.

### A-103 — The downloaded 0.1.0 appeared "damaged": package signature, 0.1.1 (2026-11-16)

**Context.** After the opening the author downloaded the `.dmg` of 0.1.0 from the release: macOS answered
«"Stemwise.app" is damaged and can't be opened», without offering "Open anyway". The release was public.

**Cause.** The package had only the signature the linker puts on the executable, not a bundle signature
(`codesign --verify --deep --strict`: "code has no resources but signature indicates they must be present"). For an
app in quarantine an invalid signature counts as a damaged file. Neither the self-test nor `make build` saw it: an app
built locally does not go through Gatekeeper. The signature had never been verified, not even in F10.

**Choice.** `signingIdentity: "-"` in `tauri.conf.json`: ad hoc signature of the whole bundle, with the resources
sealed. `make build` now runs `codesign --verify --deep --strict` on the package and fails if the signature is not
valid. Version 0.1.1; 0.1.0 must be withdrawn. Verified: valid signature; a copy marked as downloaded was opened by the
author with "Open anyway". The ad hoc signature carries the hardened runtime with it (`flags=adhoc,runtime`).

**Self-test in the 0.1.1 package.** Five runs: three complete (59 out of 59, cold start 1.4 s), one with a failed step
("master plays through the chain": no signal on the bus), one that stayed stuck without finishing (process waiting, no
engine thread blocked). **Cause not established.** The Mac was in use by the author during the tests; the hypothesis,
not verified, is that the covered window or the busy audio device stop the interface's timings. Before today the
self-test had never given different outcomes. To be repeated with the machine idle (VALIDATION 63).

**Repeated with the Mac idle (same day, load 1.7).** Three consecutive runs on the same 0.1.1 package: 59 out of 59
every time, in 87–88 s, with signal on the master bus (−6.3…−7.3 dBFS) and cold start of 0.6–1.2 s. It is consistent
with the hypothesis of the Mac being in use during the earlier tests, but does not prove it: the two anomalous
outcomes were not reproduced, so the cause remains not established.

**CI, first run.** On `macos-15`: formatting, Clippy and all Rust tests pass; the gate stopped at the license list,
sorted with a comparison that depends on the Node version (26 locally, 22 on the CI: names with `_` and `-` changed
place). Now the order is by code unit, the same everywhere, and the check prints the differing lines. The steps after
licenses (types, lint, Vitest, Playwright) have still never run on the CI.

**CI, second run (0.1.1 commit).** Order fixed; a content difference remained: `pnpm licenses` listed locally two more
packages (`@types/react` and `csstype`, optional "peer" dependencies: type declarations, not included in the app), and
on the CI no, with a different pnpm version. The script no longer asks pnpm for the list: it starts from the UI's
`dependencies` and follows them in `node_modules`, without peers. 355 libraries, the same number the CI computed. The
already published 0.1.1 package contains the previous list, with the two extra entries: harmless, it aligns at the next
release.

**CI green (commit `8738e13`, 2026-11-16).** The third run was canceled by GitHub for lack of Apple Silicon macOS
runners; rerun, it passed in 25 minutes: Rust tests, licenses (355 libraries), types, lint, Vitest
(8 files), Playwright (114 tests, 6.3 minutes against 1.7 locally, none flaky), requirements without gaps. Left out of
the CI, by choice, are the two frame-time tests and the disk cap.

**Tour, the real cause of the flaky test (corrects A-102).** The key trace added to the test showed where the arrow
ended up: on a radio button. When the Master's preparation overlay goes away and focus is on nothing, the Master puts it
back on the Mix/Master selector; between two tour steps focus is on nothing for an instant, and the next arrow went to
that selector instead of the tour. Now, with the tour running, the Master does not take focus. It was a real defect: it
also happened by hand, changing step while the master was finishing preparing. A-102's fix (step busy from the first
frame) stays, but it was not the cause. After: the three test files eight times in a row under load, no tour hang. In
the same round, a Master test looked for "Back to Mix" without saying in which overlay: made precise.

### A-104 — The two Dependabot alerts (2026-11-16)

**Context.** At opening GitHub flagged two "moderate" vulnerabilities in the dependencies. Neither is in the
distributed package.

**`minimatch` < 11.0.1 (npm).** Dependency of the linter only (eslint and typescript-eslint). Fixed: eslint 10.15.0,
which already uses 11, and an override in `app/ui/pnpm-workspace.yaml` (pnpm 11 no longer reads the `pnpm` field of
`package.json`) that brings typescript-eslint, stuck at 10, to 11 too. A single version in the lockfile, 11.0.8; lint
and types pass.

**`glib` < 0.20 (Rust).** Comes from Tauri only for Linux (`gtk` 0.18, which pins `glib` 0.18): on macOS it does not
enter the compilation (`cargo tree -i glib` without `--target all` finds nothing). It cannot be updated unless Tauri
moves to another GTK version. Not fixed: the alert is to be closed on GitHub as "vulnerable code not used", and
reopened the day a Linux build exists.

### A-104 — Windows as the second system: build, gate and package (2026-11-16, requested by the author)

**Context.** The app was built and packaged only on macOS. The author asked to make it cross-platform, starting from
Windows, which is the machine in use. The stack was already portable (P-002): `cpal`, `keyring`, `symphonia` and Tauri
work on the three systems, and `Cargo.lock` already contained the packages for Windows and Linux. The inventory found
what tied the app to macOS: the build, the gate, a few runtime defects and the interface's texts.

**Real defects found on Windows.**
- *Atomic save broken*: `package::write_atomic` reopened the temporary file read-only for `sync_all`. Windows refuses
  to flush to disk a file opened for reading (`FlushFileBuffers`), so every project creation and save failed. Now the
  same handle that writes it flushes it.
- *Save As*: the interface split paths only on `/` (`tauri.ts`, `NewProject.tsx`, `ExportDialog.tsx`), and with
  `C:\…` paths got name and folder wrong. Now it uses `shared/path.ts` (`baseName`, `parentDir`), with a test.
- *Snap*: in the Timeline snap was disabled only with `metaKey`, which outside macOS is the Windows key. Now Ctrl
  counts too, as for the other shortcuts.
- *CLI*: the data folder was `~/Library/Application Support/…` written by hand. Now it is `dirs::data_dir()` plus the
  identifier, the same folder as Tauri's `app_data_dir` on every system.

**Choices.**
- *Menu bar*: only on macOS. On Windows and Linux a bar inside the window would be a second app menu, against UI-20;
  text fields already have copy, paste and undo from the webview. The code lives in `src-tauri/src/platform.rs`, the
  third isolated module of NFR-8, which now applies to every system and not only macOS. The policy test normalizes
  path separators.
- *AAC on Windows* with Media Foundation (sink writer, MPEG-4 container), behind the same interface as AudioToolbox;
  the format is offered only if the system has an AAC encoder (the "N" editions without the Media Feature Pack do not).
  The encoder wants 16-bit PCM. The documentation says up to 192 kbit/s; Windows 11 really encodes at 256 and 320
  (measured from the file sizes: 258 and 322 kbit/s). The choices stay the same on all systems; where the encoder
  refuses a rate above 192, the export falls back to 192 and the report says so (`encode_aac` returns the rate used).
  Linux stays without AAC: there is no system encoder, and the available ones do not have permissive licenses (A-005).
- *Shortcuts*: the table stays written the Mac way. `keysInText` (`shared/keys.ts`) shows it as `Ctrl+Shift+S` outside
  macOS, in menus, tooltips and Settings. The texts that said "this Mac" and "macOS keychain" use
  `shared/platform.ts` ("this computer", "Windows Credential Manager"). The words depend on the system, the behavior
  does not. The Playwright tests run with the Safari-for-Mac user agent and stay unchanged.
- *Package*: `tauri.windows.conf.json` produces an NSIS installer for the current user (without administrator rights),
  with the WebView2 bootstrapper. The `.ico` icon already existed and was generated from the same `source.png`.
  `make build` verifies the signature with `codesign` only on macOS. The Windows package **is not signed**:
  SmartScreen warns at first launch, until the author has an Authenticode certificate (same situation as 0.1.0 on
  macOS).
- *Licenses*: `make third-party` lists the union of the libraries of the three packaged targets
  (`aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`): 497 libraries instead of 355, all
  with permissive licenses (`cargo deny` green). Each package may name a few libraries it does not contain, never omit
  one it contains. New dependencies: `windows` 0.62 (Windows only, already in the lockfile through Tauri),
  `windows-sys` 0.61 (development only, benchmark memory peak), `dirs` 7.0 (already in the lockfile through Tauri).
- *Render across systems*: on a single system the render stays deterministic, but it is not bit-identical between macOS
  and Windows. `sin` and `exp` come from the system math library and may differ in the last bit (about −140 dB): even
  the tests' own test signals come out different. We do not switch to a math library of our own: it would change the
  hashes on macOS and break the schema 3 migration test (A-087), whose fixtures come from a build that no longer
  exists. That test therefore runs only on macOS. A project opened on another system needs its mixdown redone, because
  the mix hash changes: it recomputes by itself.
- *Gate on Windows*: `make` is GNU make (here Strawberry Perl's `gmake`) inside a bash shell (Git Bash or Cygwin); the
  Makefile does not change. On Windows Playwright uses **Edge** (`channel: "msedge"`) instead of its WebKit, with the
  Safari-for-Mac user agent: WebView2, with which the app reaches the user, is Edge's engine. Playwright's WebKit for
  Windows draws in software: it measured 8 fps on the NFR-3 tests, against 145 and 117 fps on Edge, and under the
  suite's load it intermittently lost clicks on the tour bubble while it moved (two runs out of three, at 8 as at 4
  workers). With Edge the suite passed three times out of three, 116 tests in 1.2 minutes. On macOS and on the CI
  WebKit stays; the project is still called `webkit`.
- *ARIA defect found in passing*: the list of recent projects in the Welcome had `aria-label` on a `div` without a
  role, an attribute forbidden by ARIA. axe saw it only when the list was already loaded at scan time, that is,
  intermittently. Now it has `role="group"`.
- *Self-test*: it depended on the song's duration. After the auto mix, on a short song the playhead was already past the
  end, and two steps ("reverb algorithm changed while playing", "master plays through the chain") measured silence on
  the bus. Now those steps bring the playhead back to the start. The app's behavior does not change. `clean-stale`
  recognizes `.exe` executables with their `.pdb`s and the NSIS installer.

**Verified on Windows 11 x64 (2026-11-16).** `make check` green (116 Playwright tests, 162 requirements without gaps).
`make build` produces `Stemwise_0.1.1_x64-setup.exe` (7 MB). Self-test in the release executable with the real audio
device (WASAPI, USB headphones at 48 kHz): **59 out of 59**, cold start 0.8 s, load 3%. In the same self-test Save As,
WAV, FLAC and AAC export at 256 kbit/s under the ceiling, and the tour with the synthetic song pass. The AAC module is
checked with Clippy also for `aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`. The rest of the workspace for
macOS could not be compiled from here (some dependencies want a C compiler for macOS): the CI on `macos-15` confirms
it.

**CI on Windows (same day).** A second job, `make ci (Windows)` on `windows-latest`: GNU make from Chocolatey if
missing, the steps in Git Bash, Playwright on Edge, already present on the runner. It is a job of its own and not a
matrix, so that the `make ci` check on macOS keeps the name that `main`'s protection may require. The new
`.gitattributes` (`* text=auto eol=lf`) prevents the Windows checkout from converting files to CRLF, which would break
the byte-for-byte comparisons of generated files. In the index everything was already LF: no file changes. `make ci`
tried locally in Git Bash with a clean PATH: green, 114 Playwright tests. The job runs at the branch's first pull
request.

**Not done and not verified.** Linux (packages, fallback of the key to a file when Secret Service is missing); manual
test on a clean Windows (installer, listening, file drag with scaling, minimum window of 1280×720 on a laptop at 150%:
VALIDATION 64–66); benchmark on Windows; Authenticode signing.

### A-105 — Release 0.1.2 with the packages built by CI (2026-11-17, requested by the author)

**Context.** The author retried the build on macOS after the Windows changes and asked to close the
`feature/multi-platform-build` branch into `main` and to publish 0.1.2 for macOS and Windows, with the site aligned. A
single computer cannot build the packages of both systems: the `.dmg` needs macOS, the NSIS installer needs Windows.

**Alternatives.** (a) A draft release with the `.dmg` only and the installer built and uploaded by hand by the author;
(b) the packages built by GitHub CI; (c) 0.1.2 for macOS only. **The author's choice: (b).**

**How.** `.github/workflows/release.yml` starts on the push of a `vX.Y.Z` tag (or by hand on an existing tag). A first
job checks that the tag is the version in `tauri.conf.json` and creates the release **as a draft**; two jobs run
`make build` from the tagged commit on `macos-15` (Apple Silicon, ad hoc signature verified by `codesign`, A-103) and on
`windows-latest` (NSIS installer for the current user, unsigned) and attach the package; the last one downloads the two
packages and attaches `SHA256SUMS.txt`. The draft is published by hand, after writing the notes and trying the
downloaded packages: a package built by CI has not been through the self-test. The branch enters `main` as a
fast-forward (`main` was the direct ancestor).

**First run (tag `v0.1.2`).** Windows built and attached (7.4 MB NSIS installer); the macOS job cancelled by GitHub
after 15 minutes without a free Apple Silicon runner, rerun and passed. Packages downloaded: the sums in
`SHA256SUMS.txt` match, the `.dmg` has a valid ad hoc signature (`adhoc,runtime`), version 0.1.2, `LICENSE` and the
third-party licenses inside, `arm64` executable. Self-test in the CI `.dmg` on a demo project from the CLI, with the Mac
in use (load 5, a virtual machine running): 58 out of 59 twice, with signal on the bus; only the cold start fails,
8.5 s on the first launch of the freshly copied binary and 3.9 s on the second, against the 2 s threshold. A first
attempt had stopped after the export (as in A-103); the second restarted from that project with the leftover lock. Whether
the slow start depends on the machine or on the package is not established: to be repeated with the Mac idle
(VALIDATION 63).

**Gate of `main`.** On the macOS runner the accessibility test (axe-core on every screen) exceeded its 30 s timeout: it
took 19 s on the same CI at the previous commit and 25 s on Windows. It now has a 90 s timeout, like the tour tests; the
checks do not change.

### A-106 — English everywhere in the repository (2026-11-17, requested by the author)
With the repository public and the app open source, the documentation written in Italian (FOUNDATION, DECISIONS,
PROGRESS, VALIDATION, dsp.md, the design and brand documents, the PoC documents) shut out anyone arriving from
outside. Alternatives: keep Italian for the working log and English only for what newcomers read (the rule until now);
keep both languages side by side (twice the upkeep, and the two drift apart). **Choice**: everything in the repository
is in English — code, documents, decisions, progress, validation steps, agent and command definitions, commits, issues
and pull requests — for new text and for every edit. The existing documents were translated once, in full; ids,
numbering of requirements and validation items, code and contracts are unchanged. The frozen PoC got its documents
translated too (its code, tests and oracle are untouched). The headings that `app/scripts/reqs.mjs` looks for changed
(`## 3. Functional requirements`, `## 4. Technology stack`); the brand direction folders got English names. The only
exception is product data: `rationale` and `summary` stay in the language of the brief (AI-13), and fixtures may
hold other languages on purpose. Requirements: none (FOUNDATION §9, rule 10; CLAUDE.md).

## Added dependencies

| Library | Version | License | Why | Phase |
| --- | --- | --- | --- | --- |
| tauri, tauri-build | 2.12 / 2.7 | MIT/Apache-2.0 | Desktop shell (P-006) | F0 |
| tauri-plugin-dialog | 2.8 | MIT/Apache-2.0 | Open and save windows | F0 |
| serde, serde_json | 1.0 | MIT/Apache-2.0 | Contract serialization | F0 |
| react, react-dom | 19.3 | MIT | Interface (§4.1) | F0 |
| zustand | 5.1 | MIT | Mirror of the core's state | F0 |
| @tauri-apps/api, @tauri-apps/plugin-dialog | 2.12 / 2.8 | MIT/Apache-2.0 | Bridge to the core | F0 |
| @fontsource/figtree, @fontsource/bricolage-grotesque | 5.3 | OFL-1.1 | Bundled fonts (A-001) | F0 |
| schemars | 1.2 | MIT | JSON Schema of the contracts (§5.5) | F1 |
| ts-rs | 12.0 | MIT | TypeScript types generated from the Rust types | F1 |
| thiserror | 2.0 | MIT/Apache-2.0 | Typed errors | F1 |
| blake3 | 1.8 | CC0-1.0/Apache-2.0 | Hash of imported files and of the mix state | F1 |
| symphonia (wav, flac, aiff, pcm, mp3, aac, isomp4, alac) | 0.6 | MPL-2.0 | Decoding (§4.1) | F1 |
| rubato, audioadapter-buffers | 5.0 / 5.2 | MIT | Sample-rate conversion (PRJ-5) | F1 |
| clap, anyhow | 4 / 1 | MIT/Apache-2.0 | `stemwise-cli` | F1 |
| proptest, tempfile (development) | 1.12 / 3.27 | MIT/Apache-2.0 | Property tests, temporary folders | F1 |
| realfft | 3 | MIT/Apache-2.0 | Spectrum analyzer (FX-7, MAS-4) | F2 |
| cpal | 0.18 | Apache-2.0 | Audio output (ENG-2) | F3 |
| rtrb | 0.4 | MIT/Apache-2.0 | Lock-free queues between threads (ENG-1) | F3 |
| rayon | 1.12 | MIT/Apache-2.0 | Parallel offline render and analysis (NFR-4, ANA-17) | F3 |
| reqwest (blocking, json, rustls) | 0.13 | MIT/Apache-2.0 | OpenRouter client (AI-9) | F5 |
| keyring | 4.2 | MIT/Apache-2.0 | Key in the system keychain (AI-11) | F5 |
| vite, @vitejs/plugin-react, typescript, eslint, typescript-eslint, vitest, jsdom, @testing-library/*, @playwright/test, @tauri-apps/cli | current | MIT/Apache-2.0/BSD | Development and testing (not packaged) | F0 |
| lofty | 0.25 | MIT/Apache-2.0 | WAV (RIFF INFO, ID3v2), FLAC (Vorbis, picture), M4A (atoms) tags (MAS-9) | F8 |
| @axe-core/playwright (and axe-core) | 4.14 | MPL-2.0 | Accessibility tests (UI-7), development only | F9 |
| libc (development) | 0.2 | MIT/Apache-2.0 | Memory peak in the benchmark (NFR-5) | F9 |
| sha2 | 0.10 | MIT/Apache-2.0 | PKCE challenge of the OpenRouter connection (AI-24) | P-015 |
| getrandom | 0.3 | MIT/Apache-2.0 | Random PKCE verifier and `state` (AI-24) | P-015 |
| security-framework (macOS only) | 3.7 | MIT/Apache-2.0 | Knowing whether the build is signed with an Apple certificate (AI-11, A-072); already present as a dependency of `keyring` | P-015 |
| tauri-plugin-opener | 2.7 | MIT/Apache-2.0 | Opening OpenRouter pages in the browser (AI-24) | P-015 |
| windows (Windows only; Media Foundation, COM) | 0.62 | MIT/Apache-2.0 | System AAC encoder on Windows (MAS-10, A-104); already in the lockfile through Tauri | A-104 |
| windows-sys (development, Windows only) | 0.61 | MIT/Apache-2.0 | Benchmark memory peak on Windows (NFR-5, A-104) | A-104 |
| dirs | 7.0 | MIT/Apache-2.0 | CLI data folder equal to the app's on every system (A-104); already in the lockfile through Tauri | A-104 |

## Flagged out of scope

_Things needed or requested by the design that sit in the right column of FOUNDATION §2: noted, not implemented._
