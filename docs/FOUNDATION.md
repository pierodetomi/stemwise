# Stemwise — product foundation document

November 10, 2026 · author: Piero De Tomi · status: **approved as the basis for design and implementation** ·
extended on November 11, 2026 with the structural AI mix and the new reverb (P-011, phases F11 and F12) ·
extended on November 13, 2026 with mix state at a glance (P-013, phase F13), persistent proposals (P-014)
and guided assistant setup (P-015) ·
extended on November 14, 2026 with the measured AI master (P-016) and the free master chain, in design (P-017) ·
extended on November 15, 2026 with the mix explanation (P-019, phase F18) and the guided tour (P-020, phase F19)

This document is the source of truth for the Stemwise product: what it does, how it is built, with which stack, in
which order it is built, and by which rules the agents work. It replaces the PoC PRD ([PRD.md](../poc/docs/PRD.md)) as
the reference for new work; the PRD and the Python code remain as an archive and as the numeric oracle (§5.7).

The work happens in **two sessions**, each started with a ready-made prompt:

1. **Design session** (Claude Desktop, canvas) — [§10](#10-design-session-prompt).
2. **Implementation session** (Claude Code, in this repository) — [§11](#11-implementation-session-prompt).

In case of conflict: this document wins over `CLAUDE.md`, which wins over the design, which wins over the PoC PRD.
A conflict found during implementation is resolved in this order and recorded in
`docs/app/DECISIONS.md`, without stopping.

---

## 1. Vision

Stemwise is a **multitrack DAW for mixing and mastering already-recorded songs**, in which a language model acts as
a mixing assistant: it proposes reasoned parameters, the user listens, corrects by hand or in words, and stays in
control. It is not a DAW for recording or composing.

**User.** Someone who writes and records well but is not strong at mixing and mastering (the author first of all),
and wants a credible result without learning a trade or paying for subscriptions. Second profile: someone who knows
how to mix and wants a quick, explained starting point.

**Product principles** (inherited from the PoC and confirmed by its outcome, [README](../poc/README.md#poc-outcome)):

- **P1. The model never receives audio.** It receives numeric measurements, roles, track names, briefs and requests;
  it returns parameters. All audio stays on the machine. The user can always see what is sent.
- **P2. The project document is the single source of truth.** Same project ⇒ same offline render (same hash). The AI
  has no privileged channel: it makes the same changes the user could make by hand.
- **P3. Every AI choice is explained** (per-track rationale, plan summary) and **reversible** (an applied proposal is
  a single undo step).
- **P4. Nothing is destructive.** Imported files are never modified; every operation is an undoable command.
- **P5. Mix and master are two distinct phases**, with separate workspaces, tools and histories.
- **P6. No compromise on the audio engine**: native real time, with no allocations or locks in the audio thread.

### 1.1 Decisions made with the author (2026-11-10)

| Id | Decision |
| --- | --- |
| P-001 | **Distributable product**: only permissive-license dependencies; no GPL/LGPL library. The PoC's `pedalboard` and `matchering` are not used: the DSP is rewritten. |
| P-002 | **One codebase for macOS, Windows and Linux**: cross-platform stack; v1 is verified and packaged on macOS (Apple Silicon and Intel). Since 2026-11-16 (A-104) also on Windows 10/11 x64, with an NSIS installer; Linux is the next step. Porting a system is build and testing work, not a rewrite. |
| P-003 | In v1: **free automation curves**, **clip editing**, **built-in delay and saturation**. |
| P-004 | **Out of v1: third-party AU/VST3 plugins.** |
| P-005 | **Model access with the user's own key via OpenRouter**, stored in the system keychain. No backend. |
| P-006 | Stack: **Tauri 2 + Rust engine + React/TypeScript UI** (§4). |
| P-007 | Two autonomous sessions (design, implementation); delegation to subagents only where it reduces costs (§9). |
| P-008 | In v1 (author's review): **multiple returns, sidechain and structured routing** (nested groups), **automation of all continuous chain parameters**, **FLAC and AAC export with metadata and ISRC**. Out: arbitrary routing graph, DDP and MP3 (LGPL encoder, in conflict with P-001). |
| P-009 | **"AI usage" panel** in the app: models, calls, costs and a readable history of every call (AI-14). |
| P-010 | **Disk space cap** on the repository during development, with periodic cleanup (§9, rules 18–20). |
| P-011 | (2026-11-11, after the delivery of F10) **Structural AI mix and new reverb.** The AI can propose changes to the mix structure that do not alter the song: **dedicated returns** and **new groups** (AI-17 → AI-20); it recognizes **true doubles** and opens them to the sides (ANA-18, AI-21); it uses slapback, delay "throws" and **parallel compression** (compressor dry/wet, FX-3). The comb reverb is **replaced** by three higher-quality algorithms (FX-5, FX-9), **with no render backward compatibility**: no real projects exist yet. Out: the fake double from a single take, per-track width, ping-pong delay, creation or duplication of audio tracks by the AI. Built in phases F11 and F12 (§8.1). |
| P-012 | (2026-11-13, from the author's test on a 30-stem song) **Balance and effects of the AI mix.** The proposed mixes left the drums ahead of everything, guitars and vocals behind, backing vocals almost absent and reverbs inaudible: the only measured goal was the reference's spectrum. The model now receives levels per family and per return (AI-22), the loop runs at least two iterations and keeps the last valid plan (AI-1), a revision has no step limits and starts from the same button as the auto mix (AI-3). |
| P-013 | (2026-11-13, from the author's test on a 30-stem song) **Mix state at a glance.** To know which tracks had effects, sends or sidechain, you had to select them one at a time: the state lived only in the inspector, and the strips showed two unnamed sends. Every header and every strip now carries the **marks** of what the track has switched on (UI-8), sends are **all visible and named** (UI-9), the selection shows **what it is connected to** (UI-10) and the mixer has a grid **Overview** view (UI-11). UI only: no change to the project document, the engine or the AI contracts. Boards 3e–3g, 7f–7h, C7; built in phase F13 (§8.1). |
| P-015 | (2026-11-13, ahead of opening the repository) **Guided assistant setup.** Anyone who does not know what OpenRouter is had to get a key and paste it. The app now **connects the account from the browser** (the provider's OAuth flow with PKCE): one button, approval on the site, and the key reaches the app by itself (AI-24). Credit and the manually pasted key are guided steps; running out of credit has its own state. The key lives in the keychain only where the app can read it **without asking for the password** (build signed with an Apple certificate); otherwise in a private app file (AI-11). P-005 stands: the user's key, no backend. Boards 14a–14k. |
| P-014 | (2026-11-13, issue #2) **Persistent proposals.** An assistant proposal could only be applied on the spot: once discarded it was lost, and once applied it could not be reapplied after changing parameters by hand. Every proposal received stays in the project, in Mix and in Master: you review when and what it was, listen again against the current state, and apply after an explicit confirmation (AI-23). |
| P-016 | (2026-11-14, issue #1, from the author's test) **Measured AI master.** On loud targets the AI master left all the work to the limiter and, when asked for more dynamics, lowered the loudness: it decided blind, in a single call, on measurements that did not say how far the peaks stand above the body of the mix. It now receives that distance, how much the target removes and how hard the limiter works with the current chain; every plan is **rendered and measured** and the next decision corrects it (MAS-6). The target stays the author's: it changes only if the request asks for it in words. Clipper, chain order and per-section master are the next step, to be designed. |
| P-017 | (2026-11-14, issue #1, decided by the author) **Free master chain.** The multiband master **enters the scope**. The chain goes to ten modules: Input first, Limiter and Dither last, and in between EQ, Match EQ, Compressor, Multiband, Width, Saturation and Clipper in **free order**, by hand and from the AI. New: clipper (how many dB to remove from the highest peaks), multiband from two to six bands, parallel mix and filter on the compressor detector, **per-section offsets** on a few parameters, the proposal with before and after measurements, the return to the mix when the target asks too much. The chain is a **list of blocks** shown as a rack next to the editor: EQ, Compressor, Multiband, Saturation and Clipper can appear twice, twelve blocks in all (A-080, A-081). Boards 15a–15j (A-079, A-081), approved by the author on November 14, 2026 and implemented in phase F14 (MAS-11 → MAS-15, project schema 3, `master_plan` v2). |
| P-018 | (2026-11-14, decided by the author after an analysis of possible effects) **Mix effects and creative ideas.** The track and group chain becomes a **list of blocks in free order**, like the master's (P-017): the five existing blocks stay as the starting point, and thirteen are added. **Correctives**: gate and expander, de-esser, dynamic EQ with sidechain, transient shaper. **Dynamics and color**: clipper and multiband also on tracks and groups, saturation characters (tape, tube, transistor), exciter, bass enhancer. **Space**: per-track width, a block chain also on returns (ducking, return EQ, gated reverb), ping-pong and tape delay, `shimmer` reverb. **Movement**: modulation (chorus, flanger, phaser), tempo-synced tremolo and auto-pan, resonant filters. **Pitch**: harmonizer and doubler as effects on the track. The AI's per-section offsets extend to filter frequencies and blocks switched on only in some sections. The mix plan adds and adjusts blocks (AI-25) and proposes up to three **creative ideas** in a sub-section of the proposal, each with its own checkbox, off by default (AI-26, AI-27). Out: the fake double from a single take on another track, reverse reverb and delay, freeze, lo-fi and bitcrusher, EQ with more bands and narrow notches, pitch correction. Designed and built in phases F15 → F17 (§8.1), with approval of the boards before implementation: boards 16a–16u, 4c and 4d (A-086), approved by the author on November 14, 2026. |
| P-019 | (2026-11-15, requested by the author) **Explain this mix.** The interface shows the state of the mix to those who already know how to read it; what is missing is the flow (who goes where) and the intent (who does what and why), and anyone who is not an engineer gets lost. A Mix view, **Explain**, puts the **signal map** on the left (tracks by family → groups → bus, sends, sidechain: UI-15) and **the same mix in words** on the right, for the whole mix, per element and per section (UI-16). It is built **by rules** from the project document and from render measurements (ANA-20): **no model**, no call, no cost; the reasons are only those already written by the assistant. Read-only: no change to the document, the engine or the AI contracts. Narration written by a model, local or remote, is postponed. Boards 17a–17j; phase F18 (§8.1). |
| P-020 | (2026-11-15, issue #5, decided with the author) **Guided tour.** Anyone opening the app for the first time does not know how it is laid out or where to start. At the **very first launch** a tour starts by itself that takes you around the app on a **built-in demo project** (PRJ-9): it is not a user project, the document is built by code and the audio is **synthesized** from closed formulas, so no audio file enters the repository or the package; it opens at the start of the tour and disappears at the end. The tour is a **complete round** in fifteen steps (transport, timeline, tracks, mixer, inspector, assistant, proposal under review, history, Explain, Master, export), **driven with Next**: spotlight on the area being explained, speech bubble with Back, Next and Skip; the tour opens panels and views, and the lit area stays usable (UI-17 → UI-19). It can be skipped at any moment and repeated from **Help › Take the Tour** in the app menu (UI-20) and from the Welcome screen. **No model**: the proposal shown comes from the fake decider, with no network and no costs. No change to the project schema, the engine or the AI contracts. Out: steps that advance only when the user performs an action, separate chapters, the demo project openable outside the tour, real stems in the package. Boards of screen 18, to be approved before implementation; phase F19 (§8.1). |

## 2. v1 scope

| In v1 | Out of v1 |
| --- | --- |
| Import of WAV/FLAC/AIFF stems onto separate tracks | Audio recording, MIDI, virtual instruments |
| Timeline with clips: cut, move, trim, fade, gain, mute | Time-stretch, pitch correction, warp, comping |
| Internal channel strip: trim, HPF, LPF, 4-band EQ, compressor, saturation, fader, pan, sends; with P-018 a free-order block chain with gate, de-esser, dynamic EQ, transient shaper, clipper, multiband, exciter, bass enhancer, width, modulation, tremolo and auto-pan, harmonizer and doubler | Third-party AU/VST3/CLAP plugins |
| Multiple returns (reverbs and delays) with sends, nested groups, compressor sidechain, mix bus | Arbitrary routing graph (track to track, multiple outputs, pre-fader sends) |
| Song sections and automation of all continuous chain parameters, plus mute and bypass | Real-time automation recording from external controllers |
| AI mix (verification loop), revision in words, track locks, AI usage and cost panel | Audio sent to the model, automatic instrument recognition from audio, automatic section detection |
| Structural AI mix (P-011): dedicated returns and groups created by the AI, true doubles recognized, parallel compression | Audio tracks created or duplicated by the AI, fake double from a single take (phase-shifted, panned copy on another track) |
| Reverb in four algorithms (plate, room, hall, shimmer), stereo, ping-pong and tape delay, block chain on returns (P-018) | Convolution reverb and impulse responses; reverse reverb and delay, freeze |
| AI creative ideas (P-018): blocks, sends, returns and per-section offsets proposed separately, each with its own checkbox | Lo-fi and bitcrusher; track EQ with more than 4 bands or narrow notches |
| Unlimited undo/redo, named snapshots, A/B comparison | Collaboration, cloud, accounts |
| Mixdown and separate master phase, AI master, references, multiband master (P-017) | Stem separation of the references |
| WAV, FLAC and AAC export with metadata (title, artist, album, year, ISRC, cover art) | MP3 (LGPL-licensed encoder), DDP and album assembly |
| macOS 13+, `.app`/`.dmg` package; Windows 10/11 x64, NSIS installer (A-104) | Verified builds for Linux, signing and notarization (they require the author's Apple account and an Authenticode certificate), automatic update |
| English interface, centralized strings | Other interface languages |
| Guided tour at first launch and on request, on a built-in demo project with synthesized audio (P-020) | Task-based tour (steps that wait for an action), separate chapters, video, online help; demo project openable outside the tour; real stems in the package |

Rule: what is on the right is **not implemented**, not even in part. If needed, it is noted in
`docs/app/DECISIONS.md` under "Flagged out of scope" and work continues.

## 3. Functional requirements

Every requirement has a stable id. Every id must have at least one automated test that cites it (§8.3) or a written
justification in `docs/app/REQS-JUSTIFIED.md`.

### 3.1 Project and files (PRJ)

- **PRJ-1.** A project is a package folder `<Name>.stemwise/` (§5.2) with a versioned `project.json`.
- **PRJ-2.** New, open, save, save as, recent projects. Autosave every 30 s and at every AI transaction;
  recovery after a crash.
- **PRJ-3.** Import **copies** the files into the package (`audio/`); the originals are never touched or
  referenced. The files in `audio/` are immutable: no command rewrites them.
- **PRJ-4.** Import by drag and drop or from a dialog, several files at once: each file becomes a track
  with a clip starting at zero. Formats: WAV (PCM 16/24, float 32, also EXTENSIBLE/RF64), FLAC, AIFF; mono or
  stereo.
- **PRJ-5.** The project rate is that of the first import. A file at a different rate is not silently
  resampled: the app asks whether to convert it (converted copy in the package) or cancel.
- **PRJ-6.** Every track is assigned a **role** (the PoC's list, PRD §6.1, plus `room`, `strings`, `brass` and, for
  acoustic and classical music, `piano`, `woodwinds`, `choir`: A-080).
  The role is suggested from the file name with simple rules and is always editable.
- **PRJ-7.** Song brief: genre, mood, free notes. Reference tracks (0–3; WAV/FLAC/AIFF/MP3/AAC),
  copied into the package and used only for analysis and comparison listening.
- **PRJ-8.** `project.json` is valid against a JSON schema exported to `app/schemas/`; a project of an
  earlier schema version is migrated on opening.
- **PRJ-9.** **Built-in demo project** (P-020). The app contains a demo project that serves only the
  tour (UI-17) and is **not a user project**. The core builds it from code: a document valid for the current
  schema (no new schema version) and audio **synthesized** from closed formulas with a fixed seed,
  never from files included in the repository or the package.
  - **Content**: a 60–75 s song at 48 kHz with 8–12 mono and stereo stems, with the names and roles of the song in
    the boards where possible (drums, bass, guitars or keyboards, vocals, backing vocals), enough to show everything
    the tour names: at least two groups, two returns (a reverb and a delay) with sends, a sidechain, three
    named sections, an automation lane, chains with the classic blocks and a few non-neutral values, brief
    filled in, no references.
  - **Deterministic**: two builds give the same document and the same render hash.
  - **Ephemeral**: when the tour starts the project is materialized in a folder in the app cache (never among the
    user's documents) and is deleted when the tour ends, however it ends; a leftover from a crash is deleted at
    the next launch. It does not appear among the recents, has no autosave or recovery, cannot be saved
    (Save and Save As do nothing), exports no files. Changes made during the tour are lost: every tour starts from
    the same state.
  - **No model**: the assistant's proposal and the mixdown the tour shows are computed on the computer,
    the proposal **always with the fake decider**, even if a key is configured; no network call (NFR-7),
    nothing in the cross-project cost log.
  - **Timing** (Apple Silicon): the first step of the tour is on screen with no wait; the project is open and playable
    within 3 s of the start of the tour; proposal and mixdown are prepared in the background and a step that needs
    them before they are ready says so and waits. The cold start of the app stays under 2 s (NFR-3).
  - That the open project is the demo one is an attribute of the **session**, exposed to the interface: it is not
    written in the document.

### 3.2 Transport and timeline (TRN, CLP, SEC)

- **TRN-1.** Play, stop, pause, return to start, positioning by clicking on the ruler, loop of a region,
  scrolling that follows the cursor.
- **TRN-2.** Ruler in minutes:seconds; optional bar grid if the user sets BPM and meter (display and snap
  only).
- **TRN-3.** Horizontal and vertical zoom, smooth scrolling, waveforms drawn from precomputed peak
  pyramids.
- **CLP-1.** Clip operations: select (also multiple), move in time and between tracks of the same
  format, split at the cursor, trim the edges, duplicate, delete, mute.
- **CLP-2.** Fade in and fade out per clip (linear or constant power) and clip gain (−24…+24 dB).
- **CLP-3.** Snap to grid, clip edges, cursor and section boundaries; can be disabled.
- **CLP-4.** The clips of a track do not overlap: moving a clip over another trims the one underneath
  (undoable as a single step).
- **SEC-1.** Song sections: named intervals (intro, verse, chorus…) with a color on a dedicated lane;
  create, rename, move the boundaries, delete. Sections do not overlap.
- **SEC-2.** Sections are the grid on which the AI decides different parameters (AI-6) and on which the analysis
  measures over time (ANA-15).

### 3.3 Tracks, mixer and chain (TRK, FX)

- **TRK-1.** Track types: audio, group, return (reverb or delay), mix bus. Returns are freely created and
  deleted (up to 8); a new project has one of each type. Reorder, rename,
  color, delete, duplicate.
- **TRK-2.** Mute, solo (solo-in-place; returns follow), fader (−∞…+12 dB), pan (constant power for mono,
  balance for stereo), peak and RMS meter per track, true peak and LUFS on the mix bus.
- **TRK-3.** Structured routing: the output of an audio track, a group or a return goes to the mix bus
  or to a group. Groups nest (up to 4 levels) and have the same chain as a track. The core
  rejects any assignment that would create a cycle.
- **TRK-5.** Post-fader sends from audio tracks and groups to any return, each with its own level.
- **TRK-4.** Mixer view (vertical strips) and track headers in the timeline show the same state.
- **FX-1.** Chain per track and per group: trim → blocks (FX-10) → fader → pan → sends (post-fader) to the
  returns. A new track or group has the five **classic** blocks in the starting order: high-pass →
  low-pass → EQ (4 bands: peak, low shelf, high shelf) → compressor → saturation. Every block has a bypass.
- **FX-2.** Filters and EQs are RBJ biquads as in the PoC ([analysis.md](../poc/docs/analysis.md), "Render"); same parameter
  limits (§5.5).
- **FX-3.** Feed-forward compressor with **stereo-linked** detector (closes the PoC's D-034): threshold,
  ratio, attack, release, knee, makeup; gain reduction meter. **Sidechain**: the detector can
  listen to another track or a group (post-fader signal), with a high-pass on the detector; the core rejects
  sources that would create a cycle. **Mix** (dry/wet, 0–100%, default 100%): blends the compressed signal with
  the uncompressed one, sample-aligned; it is the **parallel compression** of tracks and groups (P-011) and is
  automated like the other parameters (AUT-1). The master compressor does not have it.
- **FX-4.** Saturation: soft waveshaper with 4x oversampling, parameters `drive_db` and `mix`, automatic level
  compensation.
- **FX-5.** Reverb-type return (algorithmic, own implementation) in **three algorithms** to choose from:
  `plate` (Dattorro plate: dense and bright, no early reflections, meant for vocals), `room` and
  `hall` (delay network with feedback of at least 8 lines, with early reflections; `hall` longer and more
  diffuse). Parameters: algorithm, decay time (0.1–10 s), size, pre-delay (0–200 ms), high-frequency damping,
  modulation, high-pass, low-pass and return level. It replaces the comb reverb
  of the first delivery, which is **removed** (P-011): an earlier project opens with the reverb converted
  to `room` (PRJ-8), with no guarantee of an identical render. With P-018 a fourth algorithm is added, `shimmer`
  (FX-22).
- **FX-6.** Stereo delay-type return: time (ms, or musical value if there is a BPM), feedback, high-pass and
  low-pass in the feedback, return level; with P-018 also the ping-pong mode and the tape character (FX-23).
- **FX-7.** Graphical EQ editor (curve with draggable handles and real-time track spectrum) and
  compressor panel with transfer curve.
- **FX-8.** Mix bus: fader and meter. No processor on the bus in the mix phase: the final dynamics belong to the
  master.
- **FX-9.** Reverb quality, verified by property tests for each of the three algorithms (thresholds
  motivated in the test and reported in `docs/app/dsp.md`):
  - the decay time measured on an impulse (RT60 between 500 Hz and 2 kHz) is within ±10% of the set value;
  - the tail is dense: the normalized autocorrelation of the tail has no isolated peaks (no repeated echo or
    dominant metallic resonance);
  - with mono input, left and right of the tail are decorrelated (|correlation| < 0.3);
  - with damping the highs decay before the mids; the pre-delay is respected; the tail ends in exact silence;
  - modulation is deterministic: oscillators with a fixed initial phase, reset at the start of every render,
    no random generator at run time; the result does not depend on the block size (ENG-4);
  - no allocation in `process`, stability with parameters changing every 32 samples (AUT-6);
  - NFR-1 is still met with 4 reverb returns added to the benchmark.
- **FX-10.** **Block chain** (P-018). Between trim and fader the chain of tracks and groups is a **list** of blocks in
  free order, like the master's (MAS-15). Types: the classic `hpf`, `lpf`, `eq`, `compressor`, `saturation` and the
  new `gate`, `deesser`, `dynamic_eq`, `transient`, `clipper`, `multiband`, `exciter`, `bass_enhancer`, `width`,
  `modulation`, `tremolo`, `harmonizer`, `doubler` (FX-12 → FX-21). Each type at most **twice**, at most **12
  blocks** per chain. Adding, removing, moving, duplicating and resetting a block to its initial values are one
  undo step each; removing a classic block is allowed and it can be put back. Common requirements:
  - a structure change during playback produces no clicks; switching a bypass on or off, by hand or from
    automation, goes through a fade of about 10 ms;
  - the **latency** of a track does not depend on its chain: it is the same for every track, group and return, and
    adding a block never shifts a track relative to the others (ENG-5);
  - a mono track stays mono until the first **stereo** block (`width`, `modulation`, `tremolo` in auto-pan,
    `harmonizer`, `doubler`); from there the chain is stereo and the pan acts as balance (TRK-2). A stereo block that
    receives a mono signal says so in its panel when it has no effect (`width`);
  - a schema 3 project opens with the five classic blocks in the usual order and **the same render**
    (same hash);
  - the chain marks (UI-8) and the Overview view (UI-11) follow the list (UI-12).
- **FX-11.** **Return chain** (P-018). Every return has, after its processor, a list of blocks of the same
  types as FX-10, empty by default, at most 6. It serves **ducking** (compressor with sidechain from the track
  playing over the effect), return EQ and **gated reverb** (gate with sidechain from the snare). Sidechains
  follow FX-3: the core rejects sources that would create a cycle. A return still does not send to
  other returns (TRK-5).
- **FX-12.** **Resonant filters**: high-pass and low-pass have a `resonance` (Q from 0.5 to 10; 0.707, today's
  behavior, is the default), automatable like the frequency. With per-section offsets (AI-6)
  and automation they make filters that open and close between sections.
- **FX-13.** **Gate / expander**: threshold, range (0–80 dB: how much it lowers below the threshold), ratio (1–20; at
  20 it is a gate), attack, hold, release, high-pass and low-pass on the detector, optional sidechain from another
  track or a group (FX-3). Opening and closing without clicks.
- **FX-14.** **De-esser**: frequency (3–12 kHz), threshold, maximum reduction (0–12 dB), `split` mode (lowers only what
  is above the frequency) or `wide` (lowers the whole signal); listening to the detected signal alone, which enters
  neither the project nor the export.
- **FX-15.** **Dynamic EQ**: one or two bands (peak, low shelf, high shelf) with frequency, Q, threshold, maximum
  gain (−12…+6 dB: negative cuts, positive boosts when the band's signal exceeds the threshold), attack and
  release. The detector listens to the same band or, with the sidechain, to the same band of another track or of a
  group (FX-3): so the guitars drop at 3 kHz **only when the voice sings**.
- **FX-16.** **Transient shaper**: attack (±12 dB), sustain (±12 dB), speed (0–1: how short the attack part is),
  mix. It has no threshold: the result does not depend on the input level.
- **FX-17.** **Clipper and multiband on tracks and groups**: the same processor as MAS-12 for the multiband (2–6
  bands); the clipper is the one from MAS-11 with an **absolute threshold** (−24…0 dBFS) in place of the threshold
  measured on the song, because on a track its input changes with every preceding parameter.
- **FX-18.** **Color**: saturation has a `character`: `soft` (that of FX-4, default and the value of existing
  projects), `tape` (rounded peaks and a slight loss of highs), `tube` (asymmetric, even harmonics),
  `transistor` (harder, odd harmonics). **Exciter**: frequency (1–12 kHz), drive, amount: it saturates only what
  is above the frequency and mixes it back in. **Bass enhancer**: frequency (40–160 Hz), harmonics (0–1), mix: it
  generates harmonics of the low band so the bass reads on small speakers too; it does not generate octaves below.
  All with 4x oversampling and automatic level compensation, like FX-4.
- **FX-19.** **Width** per track, group and return: width (0–200%, gain of the side) and mono below a
  frequency (0–250 Hz), like the master's Width block (MAS-1).
- **FX-20.** **Movement**. **Modulation**: mode `chorus`, `flanger` or `phaser`; rate (0.02–10 Hz, or musical value if
  there is a BPM), depth, feedback (flanger and phaser, −0.9…0.9), voices (chorus, 1–3), stages (phaser: 4,
  6, 8, 12), stereo spread (0–180°), mix. **Tremolo**: mode `tremolo` or `autopan`, rate (Hz or musical value),
  depth, shape (sine, triangle, smoothed square), stereo phase offset for the tremolo. For both the
  **phase depends only on the position in the song**: the same sample of the song has the same phase in playback from
  any point and in the offline render; with BPM the phase is locked to bar 1.
- **FX-21.** **Pitch**. **Harmonizer**: one or two voices, each with interval (−12…+12 semitones, plus ±50 cents),
  level and pan, plus the mix. **Doubler**: one or two voices with a small pitch offset (0–30 cents), a
  delay (5–40 ms) that moves slowly, stereo spread and mix. Pitch shifter written by us, deterministic, with
  constant compensated latency (FX-10). The doubler is an effect **inside the track**: it creates no tracks or copies
  of the clip (§2). No pitch correction.
- **FX-22.** **`shimmer` reverb**: a fourth algorithm of FX-5, with a pitch transposition in the recirculation:
  interval (+5, +7 or +12 semitones, default +12) and amount (0–1). With amount 0 all of FX-9 holds; with amount
  above 0 the tail stays bounded, ends in exact silence and does not depend on the block size.
- **FX-23.** **Ping-pong and tape delay**: mode `stereo` (that of FX-6, default) or `ping_pong` (input summed to
  mono, repeats alternating between left and right); character `clean` (default) or `tape`, with wow and flutter
  (0–1, phase tied to the position in the song like FX-20) and drive in the feedback (0–12 dB) with the level compensated.
- **FX-24.** **Quality of the new blocks**, verified by property tests for each block (thresholds motivated in the
  test and reported in `docs/app/dsp.md`):
  - in the neutral position (mix at 0, amount or depth at 0, gains at 0 dB) the block leaves the signal as it is
    (difference ≤ 1e-6, apart from the compensated latency);
  - no allocation in `process`, result independent of the block size (ENG-4), stability with
    parameters changing every 32 samples (AUT-6), bounded output with full-scale noise;
  - gate: below the threshold the signal drops by the set range (±0.5 dB); de-esser in `split`: below the frequency
    the signal does not change; dynamic EQ: the band moves only when the detector exceeds the threshold; transient
    shaper: a steady sine does not change level; harmonizer: interval measured on a sine within ±5 cents;
    modulation and tremolo: the render starting at 30 s gives the same samples as the full render from 30 s on;
    ping-pong: the repeats alternate between channels;
  - NFR-1 is met with the classic chain; with 8 of the 32 tracks also carrying a harmonizer, a four-band
    multiband and a chorus the callback stays under 50% of the period.

### 3.4 Automation (AUT)

- **AUT-1.** Automation lanes for **every continuous parameter** of audio tracks, groups and returns: fader,
  pan, sends, trim, filter frequencies, frequency/gain/Q of every EQ band, threshold/ratio/attack/
  release/makeup of the compressor, drive and mix of the saturation, return parameters and, with P-018, every
  continuous parameter of the blocks of FX-10 → FX-23 (also in the return chains); plus mute and block bypass. Points with linear interpolation; mute and bypass are stepped.
- **AUT-6.** The automated parameters of filters, EQ and compressor are updated at intervals of at most 32
  samples with interpolated coefficients: no click or instability, even with steep curves (verified by a
  test on fast sweeps).
- **AUT-2.** Editing: add, move, delete points; range selection; freehand drawing
  simplified into points; copy/paste of a range.
- **AUT-3.** With an active lane, the corresponding control shows the automated value; moving it with the
  transport stopped shifts the whole curve (offset), it does not write points.
- **AUT-4.** The render applies the curves with a per-sample ramp (no audible step) and is identical between
  playback and offline render.
- **AUT-5.** The AI's per-section decisions (AI-6) are written **as automation points** in the same
  lanes, with a ramp at section boundaries (default 80 ms): they remain editable by hand.

### 3.5 History and non-destructiveness (HIS)

- **HIS-1.** Every change to the project is a **command** with its inverse. Unlimited undo and redo in the
  session, from menu, shortcut and history panel (readable list of steps, click to return to a point).
- **HIS-2.** Continuous gestures (dragging a fader, a clip, a point) produce a single step.
- **HIS-3.** The application of an AI proposal is **a single transaction**, with a label and a reference to the call
  log.
- **HIS-4.** Named mix snapshots, persistent in the project: save, restore (undoable), A/B compare
  two states during listening without interrupting playback.
- **HIS-5.** Mix and master have separate histories.
- **HIS-6.** Mute/solo, zoom, selection and cursor position do not enter the history (view state).

### 3.6 Audio engine (ENG)

- **ENG-1.** Real-time playback of the whole graph, with streaming reads from disk (read thread
  and per-track ring buffers). The audio thread does not allocate, does not block, does no I/O.
- **ENG-2.** Choice of output device and buffer size; if the device does not support the
  project rate, resampling on output.
- **ENG-3.** Every parameter change reaches the engine within one buffer, with smoothing (no clicks).
- **ENG-4.** **Offline render** with the same code as the real-time graph, faster than real time,
  deterministic: same project ⇒ same file hash. The result does not depend on the block size
  (difference ≤ 1e-6 of peak between blocks of 64 and 4096 samples).
- **ENG-5.** Compensation of the delays introduced by the saturation oversampling and the limiter lookahead.
- **ENG-6.** Meters and playback position sent to the interface at at least 30 Hz, without going through the audio
  thread in a blocking way.
- **ENG-7.** A headless CLI (`stemwise-cli`) exposes analysis, render, AI mix with the fake decider,
  mixdown and master on a project: it serves the full-flow tests and autonomous verification.
### 3.7 Analysis (ANA)

- **ANA-1 → ANA-12.** The PoC measurements (PRD §5.2), with the formulas of [analysis.md](../poc/docs/analysis.md),
  rewritten in Rust and verified against the oracle (§5.7).
- **ANA-13.** The analysis JSON sent to the model stays under 12,000 estimated tokens for 24 tracks and 8 sections.
- **ANA-14.** Measurements are computed on the track content **after clip editing and before the
  chain**. The cache is invalidated when the clips change, not when the parameters change.
- **ANA-15.** Compact measurements over time: loudness of each track per section and percentage of active time per
  section; band profile of the mix per section.
- **ANA-16.** Families: stems with drum roles (or in the same group) are also reported as a family
  (loudness and bands of the sum); masking **within** a family does not enter the main collisions.
- **ANA-17.** Analysis in parallel across tracks, with visible progress; 16 stems of 4 minutes in under 30 s
  on Apple Silicon.
- **ANA-18.** **True doubles** (P-011): for every pair of audio tracks with the same role the analysis measures how
  much they are two performances of the same part: correlation of the level envelopes, similarity of the band
  profile, overlap of active time, and the frame-by-frame difference of the spectral shape to tell two
  performances (a few dB) from two copies of the same signal (a fraction of a dB, even at different levels or with
  a few milliseconds of delay: marked `identical`, opening them to the sides is pointless). Roles considered: guitars,
  keyboards, synth, pads, strings, brass and vocals. Pairs
  above the threshold enter the analysis document as `doubles` (ids of the two tracks, score, `identical`).
  Initial thresholds motivated in the test and to be tuned in the author's validation; the ANA-13 cap is still met.
- **ANA-19.** **Measurements for the new blocks** (P-018), computed like ANA-14 and under the ANA-13 cap: for tracks
  with vocal roles the **sibilance** (by how much short peaks between 5 and 10 kHz exceed the body of the voice, and
  the frequency where they fall); for every track the **crest** (peak over short RMS) and the **transients** (by how
  much the attacks detected by the onsets stand above the level that follows them); for tracks with drum roles the
  **floor** (level between one hit and the next relative to the hits, which says how much bleed there is); for stereo
  tracks the correlation and the side share; the tempo and meter of the project, if known. For every track, group and
  return the analysis also lists the blocks that are not classic (id, type, state, position).

- **ANA-20.** **Levels for the explanation** (P-019). For the Explain view the core measures, from the render of the
  mix as it is (the same renders as AI-22): the loudness of every family and every return relative to the mix, over
  the whole song and per section, and the loudness of the whole mix and of every section. The measurement runs in the
  background, does not block the interface, is cancelled if the mix changes and stays in memory until the mix
  changes. No data leaves the computer.

### 3.8 AI-assisted mix (AI)

- **AI-1.** **Auto mix**: from brief, analysis and reference profile, the model returns a complete plan
  (§5.5). Verification loop as in the PoC (offline render, evaluation, up to 3 iterations, stop criteria and
  best plan: PRD §5.5), with two differences (P-012): an auto mix runs **at least two iterations**, because only
  the second knows the balance of a rendered plan (AI-22); the plan kept is the **last valid one**,
  unless it moves away from the reference by more than 1 dB compared with an earlier one.
- **AI-2.** The plan is not applied by itself: it becomes a **proposal** under review, with summary, per-track
  rationale, parameter diff against the current state, evaluation (distance from the reference before/after,
  technical constraints) and cost. The user listens to the proposal in A/B with the current state, then **applies all**,
  **applies only the chosen tracks** or **discards**.
- **AI-3.** **Revision in words**: a natural-language request ("vocals more forward, less reverb on the
  guitars") produces a single-iteration proposal that starts from the current state, manual changes
  included. The variation limits between iterations (AI-8) **do not apply** to a revision: it is a single step,
  requested by the author and judged by ear (P-012). In the interface request and auto mix start from the
  **same button**: with text it is a revision, without it is an auto mix; Enter in the field inserts a new line.
- **AI-4.** **Lock**: a locked track (or group) is not modified by the AI; the model receives it as
  context and validation restores its values.
- **AI-5.** Scope: the user can limit a request to the selected tracks; the others behave as
  locked for that request.
- **AI-6.** **Per-section parameters**: if the project has sections, the plan can contain for every track
  per-section offsets of gain, sends and EQ band gains (D-048 of the PoC), applied as automation (AUT-5).
  With P-018 also the high-pass and low-pass frequency (in octaves) and the **blocks switched on only in some sections**
  (bypassed in the others, with the fade of FX-10).
- **AI-7.** Input normalization: the auto mix sets the **trim** of every track toward −23 LUFS (REN-1
  of the PoC), as a visible part of the proposal; can be disabled.
- **AI-8.** Strict validation of the plan against schema and limits; up to 2 new attempts with the error
  sent back to the model; variation limits between the iterations of an auto mix (PRD DEC-5). If the provider rejects the `strict`
  schema, an automatic fallback to JSON mode plus validation (Q-40 of the PoC, option b).
- **AI-9.** `Decider` interface with two implementations: OpenRouter and a deterministic **fake decider** (the PoC
  rules extended to sections, delay, saturation and, with P-011, to structure, reverb algorithms, parallel
  compression and doubles). The fake decider can be selected in the settings ("Offline demo") and is the only one used
  in the tests.
- **AI-10.** Transparency: for every call the request, response, tokens, cost and duration are saved in the project;
  the detail can be consulted in the "AI usage" panel (AI-14). The model never receives audio, paths, file
  names or the song title.
- **AI-11.** OpenRouter key **in the system keychain when the app can read it without the system asking for
  the password** (on macOS: build signed with a certificate issued by Apple, whose identity does not change from one build
  to the next); **otherwise in a file in the app data folder readable only by the user** (permissions 600; on Windows the data folder in the user's profile), never
  in the project, in the logs or in the settings file. The interface always says where the key is. Model, reasoning effort and temperature
  configurable; no model hard-wired in the code (configuration default: `openai/gpt-6.1-sol`, medium
  effort, D-047). Clear state when the key or the network is missing.
- **AI-12.** Calls do not block the interface, show progress by phase (analysis, decision, render,
  evaluation) and can be cancelled.
- **AI-13.** `rationale` and `summary` in the language of the brief.
- **AI-14.** **"AI usage" panel**, reachable from both phases:
  - project summary: number of calls, input and output tokens, total cost, total duration,
    broken down by type (auto mix, revision, master) and **by model used**;
  - chronological history of calls: date, type, model and effort, attempts, tokens, cost, duration, outcome
    (applied, partly applied, discarded, failed, cancelled) and link to the history step;
  - detail of a call in readable form: request rendered as a brief, user request, tracks with
    roles and locks, measurement tables; response rendered as summary, per-track changes and rationales; the raw
    JSON of request and response remains viewable and copyable;
  - export of the history as CSV and JSON;
  - total of costs **across all projects** in the AI settings (log in the app data area).
- **AI-15.** The cost is the one reported by the provider in the response; if missing, it is estimated from the tokens and the
  model's price list and marked as an estimate. Failed or cancelled calls count for what they spent.
- **AI-16.** Before a call the app shows an estimate of tokens and cost; an optional spending cap per project
  asks for confirmation when it would be exceeded.
- **AI-17.** **Structural changes** (P-011): the plan can, in the `structure` section (§5.5):
  - **create returns** (reverb, with its algorithm, or delay), up to 3 per proposal and within the cap of 8
    (TRK-1), to give a family of tracks its own space (for example a plate for the vocals, a short delay for
    slapback);
  - **create groups**, up to 3 per proposal and within the 4 levels (TRK-3), and move existing audio tracks or groups into them:
    at least two members, all with the same output; the group takes that output and the position of the
    first member. Sends and parameters of the members do not change as a result of the move.

  The plan does **not** create, duplicate or delete audio tracks, does not delete returns or groups, does not move tracks between
  existing groups and does not touch clips and sections: the song stays the same. Every structural change is a change
  the user can make by hand with the same commands (P2).
- **AI-18.** Structural changes can be **disabled** from the AI panel ("Allow structural changes",
  on by default, remembered in the project): when disabled, the schema sent does not contain `structure`. A
  request limited to some tracks (AI-5) carries no structure: a return is shared and a group changes the
  path of its members. A locked track (AI-4) is not moved into a new group.
- **AI-19.** **Stable structure in the loop**: the structure is decided at the first iteration; the following
  iterations receive it as given and adjust only parameters (their schema does not contain `structure`). A
  revision in words (AI-3) can propose structure in its single iteration. Validation (AI-8) discards with
  a warning, without failing the plan, a new return that no track sends to and a new group with
  fewer than two valid members.
- **AI-20.** **Proposal and application** (extends AI-2, HIS-3): new returns and groups appear in the proposal
  as rows of their own, with name, rationale and parameters; A/B listening includes them. "Apply all" creates them
  in the same transaction as the other changes: a single undo step removes them. Every new element has its
  **own checkbox**: when unchecked, it is not created, and the sends toward it or the move into the group are dropped, while
  the other track changes stay. With "apply only the chosen tracks" a checked new return is created
  if at least one chosen track sends to it, and a checked new group is created with only the chosen members,
  if there are at least two; otherwise the proposal says why it is not created. "Discard" creates nothing.
- **AI-21.** **Mixing techniques in the prompt and in the fake decider** (P-011):
  - **true doubles**: a pair reported by ANA-18 and not `identical` is treated as a pair: opposite and
    symmetric pans, aligned levels, same processing;
  - **slapback**: a dedicated delay with a short time (80–140 ms) and zero feedback, to which the vocals send;
  - **delay "throw"**: delay send raised only in some sections (AI-6);
  - **parallel compression**: compressor decided with `mix` below 1 on drums, vocals or their groups;
  - **distinct spaces**: reverb algorithm and decay chosen per family (short plate for the vocals,
    room or hall for the rest).
  - with P-018, among the corrections: **de-esser** on vocals with high sibilance; **dynamic EQ** with sidechain in
    place of a fixed cut when two tracks mask each other only while playing together; **gate** on drums
    with a high floor; **transient shaper** to add or remove punch; **ducking** of returns from the vocals; **sidechain
    pumping** (kick to bass or pad) where the genre asks for it and never as a default; width on pads and
    stereo keyboards, never on bass and kick; saturation character chosen by role and brief;
  - with P-018, among the creative ideas (AI-26): modulation, tremolo and auto-pan, harmonizer, doubler, shimmer,
    ping-pong, tape delay, gated reverb, filters that open between sections, throw.

  The prompt describes each technique with the case in which it is needed and the one in which it should be avoided; the fake decider
  applies each with a deterministic rule, verified by the full flow of `stemwise-cli`.
- **AI-22.** **Measured balance** (P-012): the model receives how loud every **family** of tracks (lead vocal,
  backing vocals, drums, percussion, bass, guitars, keyboards, other) and every **return** is relative to the whole
  mix, in LU, over the song and per section. It receives it before deciding (auto mix: with all faders at 0 dB on the
  normalized trims; revision: on the current mix) and in the evaluation of every iteration, measured on the render of the
  plan. Every level is a solo render of the project: the family without the returns, the return alone. The
  prompt gives the starting ratios between the families and the threshold below which a return is not heard, says that the
  reference profile describes tone and not balance, and that brief and requests come first.
- **AI-23.** **Persistent proposals** (P-014). Every call that comes back with a proposal, of mix or master,
  keeps it in its project folder, whatever became of it (applied, partly applied,
  discarded). The assistant lists them in Mix and in Master, most recent first, with when it arrived, the request
  and the outcome. A proposal from the list comes back **under review on the current state**: the differences are recomputed
  on the mix (or the master) of now, tracks that no longer exist or are locked stay out and this is stated,
  returns and groups already created are not created twice, A/B works as for a new proposal. Applying it
  asks for an **explicit confirmation** saying that the current mix will change, and is a single history step
  (AI-2, HIS-3); closing it changes nothing and does not alter its recorded outcome. Reopening a proposal does not
  call the model and costs nothing.
- **AI-24.** **Guided assistant setup** (P-015). A three-step dialog, the same from first launch,
  from the Settings and from the assistant column with no key. **Connection**: the app opens OpenRouter's
  authorization page in the browser (OAuth with PKCE S256 and `state`), waits for the return on `localhost`,
  exchanges the code for a key of the user and stores it (AI-11); the key never passes through the browser or
  the interface. Refusal, expiry (ten minutes), absent network and cancellation each have their own outcome, and
  leaving the dialog stops the wait. **Credit**: how much a mix and a revision cost at the prices of the chosen
  model, the link to the credits page, and a plain statement that the app cannot read the balance.
  **Manually pasted key** as a secondary route, in three numbered steps. When the provider rejects a call
  the assistant tells **credit exhausted**, **key limit** and **key no longer valid** apart, each with its
  own way out; a rejected call costs nothing and does not change the mix. The offline demo is always one
  click away.
- **AI-25.** **Blocks in the plan** (P-018): the plan can add blocks to the chains of tracks, groups and returns
  (FX-10, FX-11) and adjust those that exist, within the limits of §5.5; at most 3 new blocks per chain in a
  proposal. The model does not move existing blocks and does not remove them: it turns them off with the bypass. A new block goes in
  a named position (`first`, `before_compressor`, `after_compressor`, `last`); by hand it is moved like any
  other. Lock (AI-4) and scope (AI-5) also apply to chains: a return is shared, so a request
  limited to some tracks does not change its chain. In the proposal a new block is a row marked as new
  in the diff of its track.
- **AI-26.** **Creative ideas** (P-018): the plan can propose up to **3 ideas**, each with an id (`i1`–`i3`), a name (in
  English, at most 32 characters), a rationale (in the language of the brief) and, if needed, the sections in which it plays. The
  elements of an idea are marked with its id and can only be **additions**: new blocks, new sends,
  new returns (AI-17, if structural changes are allowed) and per-section offsets. An idea does not change the
  parameters of what exists: validation (AI-8) discards with a warning an element that does, without failing the
  plan. Removing an idea leaves the rest of the plan identical. Ideas come from the brief, the roles and the sections,
  not from the measurements: the prompt asks for them optional, different from each other and not corrective. They are decided at the first
  iteration and stay fixed in the loop (like AI-19); the loop renders and evaluates the plan **without** ideas. A
  revision in words can propose them in its single iteration ("something bolder in the bridge"). The fake
  decider proposes them with deterministic rules on roles and sections.
- **AI-27.** **Ideas in the proposal** (P-018): the proposal has a **Creative ideas** sub-section, one card per idea
  with name, rationale, sections and elements. Every idea has its **own checkbox, off by default**. A/B plays the
  proposal with the checked ideas; every card has a listen of the proposal with that idea alone. "Apply all"
  applies the checked ideas in the same transaction (one undo step, HIS-3); with "apply only the chosen tracks"
  the elements of the chosen tracks of an idea are applied, and a new return of the idea follows AI-20. An
  "Allow creative ideas" switch in the AI panel, on by default and remembered in the project, removes `ideas` from the schema.
  A request limited to some tracks (AI-5) can propose ideas only on those and without new returns. Persistent
  proposals (AI-23) also reopen their ideas.

### 3.9 From mix to master (MDN)

- **MDN-1.** The move to the master phase is an explicit action ("Bounce & go to Master"): offline render
  of the mix into `mixdowns/` (WAV float 32), with a trim in attenuation only to stay within −3 dBTP.
- **MDN-2.** The master works **on the frozen mixdown**, not on the live mix. If the mix changes after the mixdown, the
  master phase flags it and offers to redo it keeping the master settings.
- **MDN-3.** You can return to the mix at any time; the master state stays in the project.
- **MDN-4.** Every mixdown records the hash of the mix state it comes from.

### 3.10 Master (MAS)

- **MAS-1.** The master chain (P-017): input gain → the **free blocks** in the chosen order → loudness
  gain found by the app → true peak limiter with lookahead → dither (16 bit only). The free blocks are parametric
  EQ (6 bands), match EQ toward the reference profile (8 bands, amount 0–100%), linked compressor,
  multiband compressor (MAS-12), stereo width (gain of the side, mono below a frequency), saturation and clipper
  (MAS-11). Input, limiter and dither stay in their place: the limiter closes the chain because the ceiling (MAS-3) holds
  only if nothing follows it. Every block has a bypass.
- **MAS-2.** Predefined and customizable targets: Streaming (−14 LUFS, −1 dBTP), Loud (−9 LUFS, −1 dBTP),
  CD (−10 LUFS, −0.3 dBTP).
- **MAS-3.** The limiter **guarantees** the true peak ceiling (measured with 4x oversampling on the exported file):
  closes D-074 of the PoC.
- **MAS-4.** Meters: momentary, short-term and integrated LUFS, LRA, true peak, correlation, spectrum with the reference
  profile overlaid.
- **MAS-5.** **Loudness-matched** A/B comparison between master, unprocessed mixdown and reference.
- **MAS-6.** **AI master**: the model receives the measurements of the mixdown, the profile and loudness of the references
  (optional), brief and target, and returns a reasoned master plan (§5.5), as a proposal (same rules
  as AI-2, AI-8, AI-10). Revision in words here too (D-077 of the PoC). **It is a measured loop** (P-016): before the
  decision the current master is rendered at the target loudness and measured; every proposed plan is in
  turn, and the next decision receives plan and measurement (loudness reached, reduction of the limiter and of the
  compressor in depth and in time, punch lost, distance from the reference). At most three plans; the loop closes
  when the model confirms the plan just measured, and the proposal is the last plan. The **target belongs to
  the author**: without a request the plan keeps it; with a request it changes only if the request asks in words
  for a different loudness, and asking for more dynamics is not that. A request on a master never touched asks for a whole
  master, not a revision.
- **MAS-7.** Export: WAV (16/24 bit, float 32), FLAC (16/24 bit) and AAC in an M4A container (bitrate of choice);
  44.1/48/88.2/96 kHz with quality conversion; TPDF dither at 16 bit; several formats in one export; export
  report (LUFS, true peak, LRA, distance from the reference).
- **MAS-9.** Metadata: title, artist, album, year, genre, track number, ISRC (format-validated),
  cover art. Written in the native tags of each format (`LIST/INFO`, `bext` and `ID3` chunks for WAV; Vorbis comments
  and picture for FLAC; atoms for M4A) and kept in the project.
- **MAS-10.** AAC uses the operating system's encoder behind an isolated interface (AudioToolbox on macOS, Media Foundation on Windows);
  where the encoder is missing, the format is not offered. For lossy formats, the true peak is measured on the decoded
  file and, if it exceeds the ceiling, the export is repeated with the necessary margin.
- **MAS-8.** The master is deterministic: same mixdown and same plan ⇒ same file.
- **MAS-11.** **Clipper**: removes up to N dB from the highest peaks that reach it (0–12 dB), with 4x oversampling
  and an adjustable softness; below the threshold it touches nothing. The threshold is the peak measured at its input
  over the whole song minus N dB: it holds in every position of the chain and with every target. Until that peak is
  measured the clipper lets everything through.
- **MAS-12.** **Multiband compressor**: two to six bands separated by fourth-order Linkwitz-Riley crossovers
  (the sum of the bands is flat), each with its own linked compressor (threshold, ratio, attack, release,
  gain) and its own bypass; crossovers at least a third of an octave apart. The number of bands is changed by
  hand and from the AI; listening to a band in solo does not enter the project or the export.
- **MAS-13.** The master compressor has a **mix** (parallel compression) and a high-pass filter on the **detector**
  ("Listens above", 0 = off): what is below that frequency does not make it work.
- **MAS-14.** **Per-section offsets**: every section of the song can have, on top of the whole-song master and only
  while it plays, a level offset toward the limiter (±3 dB), a side gain offset (±3 dB, with the Width block
  on), a limiter release offset (±200 ms) and offsets of the band gains of an EQ (±3 dB). They enter and leave
  with a ramp of about a tenth of a second. The loudness of the target stays measured on the whole song.
- **MAS-15.** **Repeatable blocks and free order**: the chain is a list. EQ, compressor, multiband, saturation
  and clipper can appear **twice**, match EQ and width once; at most nine free blocks (twelve with input,
  limiter and dither). Adding, removing, moving and resetting a block to its initial values are one undo step
  each. In listening a structure change produces no clicks; the latency is the same for every chain.

### 3.11 Interface and settings (UI, SET)

- **UI-1.** Two workspaces, **Mix** and **Master**, with an always-visible phase selector and a visual identity that
  tells them apart at a glance.
- **UI-2.** Screens and states listed in §6, faithful to the delivered design.
- **UI-3.** Keyboard shortcuts for transport, editing, undo/redo, zoom, tools; a browsable list.
- **UI-4.** Minimum window 1280×720; fluid layout up to 4K; resizable and collapsible panels.
- **UI-5.** Every continuous control: drag, wheel, double-click for the default, numeric entry, modifier for fine
  adjustment.
- **UI-6.** Explicit, recoverable errors and empty states (missing files, different sample rates, missing key, no
  network, disconnected audio device).
- **UI-7.** Basic accessibility: keyboard navigation of the controls, AA contrast for text and states, screen-reader
  labels on non-text controls.
- **UI-8.** **Chain marks** (P-013). Every track and group header in the timeline and every mixer strip shows, without
  selecting anything, five marks in signal order (high-pass, low-pass, EQ, compressor, saturation), **each with its
  badge** (`HP`, `LP`, `EQ`, `CMP`, `SAT`), so that you can read which block it is without remembering its position, and
  in one of three states: **off** (bypassed), **on but neutral** (not bypassed, but does not change the sound: EQ with
  all active bands at 0 dB or with no active bands, saturation with mix at 0), **working**. A compressor with a
  sidechain takes the color of the track it listens to; one with mix below 100% is marked as parallel. A separate mark
  says the track has automation, in the assistant's color if the assistant wrote the lanes. The marks stay at every lane
  height and at the minimum window size, have a worded label (tooltip and screen reader) and are derived from the
  document: no state of their own. In the strips the badges sit in a column, as wide as the strip: there a click on a
  badge toggles the block's bypass and a double click opens it in the inspector, with the same command as the
  inspector (one undo step). In the headers the badges are read-only: a click selects the track. With P-018 the marks
  follow the block-based chain (UI-12).
- **UI-9.** **Readable sends** (P-013). Every strip shows a send for **every** return in the project (up to 8), in the
  same order, with its level; a label column fixed at the left of the mixer names each row. Every header shows one dot
  per return, lit where the track sends. A return's lane always lists who sends to it, with color, name and level.
- **UI-10.** **Selection links** (P-013). Selecting a track, a group or a return marks in the timeline and in the mixer
  what it is connected to, giving the reason in words: the group that takes its output, the returns it sends to, the
  tracks that send to it (with the level), the tracks whose compressor listens to it. The link marks do not change the
  document and do not enter the history.
- **UI-11.** **Mixer Overview** (P-013). The mixer has two views on the same columns, chosen by a selector: the
  faders, and a grid with one row per chain block, one per return, the output and the automation. Every cell states the
  block's state and main value (frequency, number of bands, ratio, drive, send level). A click on a cell toggles the
  bypass (on a missing send it creates it at the default level); a double click opens the block in the inspector. Every
  change is the same command as the inspector: one undo step.
- **UI-12.** **Block chain in the interface** (P-018). The inspector shows the chain as a list, like the master rack
  (screen 15): a menu to add a block divided by family (Dynamics, Tone, Space, Movement, Pitch), drag to reorder,
  duplicate, reset to initial values, bypass; every new block has its own editor, with curves where they are needed
  (de-esser and dynamic EQ reduction, gate curve, modulation shape). A return shows its list under the processor. The
  marks of UI-8 become one per block, in chain order, each with its badge (`HP`, `LP`, `EQ`, `CMP`, `SAT`, `GT`, `DS`,
  `DEQ`, `TR`, `CLP`, `MB`, `EXC`, `BAS`, `WID`, `MOD`, `TRM`, `HRM`, `DBL`); when they do not fit, the first ones are
  shown plus a `+n`. The Overview view (UI-11) has one row for every block type present in the project.
- **UI-13.** **Creative ideas in the interface** (P-018): the Creative ideas sub-section in the proposal (screen 4),
  with a checkbox and listening per idea (AI-27); the "Allow creative ideas" switch in the AI panel; the elements of an
  applied idea carry the idea's name in the tooltip and in the track's rationale.
- **UI-14.** **Explain view** (P-019, screen 17). In Mix, an `Explain` button in the top bar (⌘E; `Esc` closes)
  replaces timeline, mixer and panels with the map (UI-15) and the text (UI-16). The top bar stays live: transport,
  loop and undo work. The view is **read-only**: it creates no history steps and does not modify the project. It calls
  no model and uses no network; given the same project and measurements it shows the same page. It follows every
  change to the project while it is open.
- **UI-15.** **Mix map** (P-019). Columns from left to right: audio tracks grouped by family (the families of AI-22),
  groups next to their members (one column per nesting level), mix bus; the returns ("shared effects") below the
  groups. Solid line: output, thicker the higher the fader; dashed: send; dotted with an arrow in the source's color:
  sidechain. On the boxes: automation diamond (assistant or hand), lock, number of the assistant's note. A click selects
  an element: its path to the bus, its sends with the level, what feeds it and the sidechains stay lit; the rest is
  dimmed. Double click: closes the view and selects the element in the workspace. A group folds into one row with the
  track count. A section strip chooses the whole song or one section (sections with the same name count as one): tracks
  that do not play there are dimmed, those that play only there or have different values carry a label.
- **UI-16.** **The mix in words** (P-019). Text in English built from fixed sentences filled with the project's values,
  in parts that always come in the same order (a part with no content does not appear): in brief; structure (groups,
  returns, tracks going straight to the bus); who is in front (ANA-20; until the measurement exists the part says so);
  left to right (pan); space (sends per return); who reacts to whom (sidechain); what changes through the song (clips
  present per section, automation); reasons (the assistant's `summary` and notes, **quoted**). With an element selected
  the text is its card: where it goes, where it sits, the lit chain blocks in signal order with a sentence on what each
  does, the assistant's reason if there is one. With a section: what plays, what is different, the loudness compared
  with the others. Names are chips with the element's color and select it in the map; technical terms have a
  one-sentence explanation from a fixed glossary. The text describes and does not judge: no advice. `Copy as text`
  copies the plain text.
- **UI-17.** **Tour: start and repeat** (P-020, screen 18). The app remembers in its data folder (not in the project)
  whether the tour has already been shown. If it never has, on first launch the tour **starts by itself** as soon as
  the interface is ready, even if recent projects exist; the "shown" mark is written when the first step appears, so an
  interrupted tour or an app closed midway does not make it start again by itself. On request it starts from the
  **Help › Take the Tour** entry of the **app menu** (UI-20) and from a link in the Welcome screen. With a project open
  the tour first asks what closing the project asks (save, discard, cancel), then closes it. The menu entry is disabled
  while the assistant is working or a bounce or an export is in progress. The tour never starts by itself in the app's
  self-test or on the fake backend unless explicitly requested: the existing tests do not change.
- **UI-18.** **Tour: the steps** (P-020). Every step has a **spotlight** on the area it talks about (the rest is
  dimmed) and a **bubble** next to it: title, one or two sentences in English, counter (`3 of 15`), `Back`, `Next`
  (`Done` on the last), `Skip tour`. The first and last steps are cards in the center, without a spotlight.
  - **The tour drives the interface.** Every step declares the state it needs (phase, open panels, view, selection,
    proposal under review or applied) and establishes it by itself, going forward as well as back: a step can be
    reached from any other and always shows the same thing. If the user changed something in the lit area, the step that
    depends on it restores it.
  - **The lit area stays usable**: you can press Play, move a fader, open a block. Outside the spotlight clicks do
    nothing. To advance, `Next` is enough: no step waits for an action.
  - **Keyboard**: `→` or `Enter` forward, `←` back, `Esc` skips the tour; the space bar stays with the transport. The
    app's other shortcuts are suspended while the tour is active. The app menu's project commands (New, Open, Close),
    reachable when the top bar is lit, close the tour first; Save and Save as do nothing (PRJ-9).
  - **Steps, in this order** (the final texts are in the boards): 1 opening card (what Stemwise is, what will be seen,
    about three minutes, `Start` and `Skip tour`); 2 transport and top bar (Play plays the demo song); 3 timeline,
    clips and sections (every imported file is a track, nothing is destructive); 4 track header (role, mute, solo,
    lock, chain marks); 5 mixer (strips, faders, sends, groups, returns, Overview view); 6 inspector and block chain
    of the selected track; 7 assistant (brief, Auto mix and revision in words, **the model never receives audio**, cost
    estimate before sending); 8 proposal under review (summary, diff and reasons per track, A/B, apply all or the
    chosen tracks or discard); 9 history (the applied proposal is a single undo step; snapshots); 10 Explain; 11 phase
    selector and mixdown (mix and master are two phases); 12 Master: chain and target; 13 Master: meters and A/B; 14
    export (formats and metadata); 15 closing card (where to start: New project, connecting the assistant; the tour can be
    redone from Help).
  - **Accessibility** (UI-7): the bubble is a labeled dialog, focus stays between the bubble and the lit area, the step
    change is announced to screen readers, AA contrast, no animation with "reduce motion". At the minimum window size
    (UI-4) the bubble always stays inside the window and the lit area is brought into view.
- **UI-19.** **Tour: exit** (P-020). `Done`, `Skip tour`, `Esc` and closing the app end the tour the same way:
  playback stops, the demo project is closed and deleted (PRJ-9), the interface returns to the Welcome screen and the
  user's preferences (appearance, panels, widths) are what they were before the tour. Whoever skips gets a notification
  saying where to find it again. On first launch, once the tour is over, the Welcome screen shows as usual the invitation
  to set up the assistant (screen 1b). Without an audio device the tour goes on without sound, with the state of UI-6.
  If the demo project cannot be prepared, the tour says so, ends and the app stays usable.
- **UI-20.** **A single app menu** (2026-11-15, decided by the author with P-020). All of Stemwise's commands are in the
  menu that opens from the project name in the top bar, in Mix and in Master: `New project…` (⌘N), `Open…` (⌘O);
  `Project settings…`, `Save` (⌘S), `Save as…` (⇧⌘S), `Close project`; `Settings…` (⌘,); the `Help` submenu with
  `Take the Tour`. The macOS menu bar keeps only what macOS provides: the Stemwise menu with About, Services, Hide,
  Hide Others, Show All and Quit; Edit, which text fields need; Window. On Windows and Linux the window has no menu bar
  (A-104). The shortcuts stay the same and are handled by the interface (UI-3); outside macOS they are read with Ctrl,
  Alt and Shift. On the Welcome screen, where there is no project, the same commands are its buttons and links.
- **SET-1.** Settings: audio device and buffer; AI (key, model, effort, fake decider); appearance; shortcuts
  (consultation only in v1).

### 3.12 Non-functional requirements (NFR)

- **NFR-1.** 32 stereo tracks at 48 kHz with the full chain, 512 buffer: the audio callback uses at most 30% of the
  buffer period on Apple Silicon (automatic benchmark in release).
- **NFR-2.** No allocation in the audio callback (verified by a test with an allocator that rejects it).
- **NFR-3.** Interface at 60 fps during playback with 32 visible tracks; cold start under 2 s; import of 16 stems of 4
  minutes under 15 s.
- **NFR-4.** Offline render of 16 tracks at least 20 times faster than real time.
- **NFR-5.** Memory: audio is not loaded entirely into RAM; a project of 32 stems of 5 minutes stays under 1.5 GB.
- **NFR-6.** Permissive licenses only (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode), verified automatically on
  every `make check`. Fonts and icons with a license that allows including them (OFL, MIT, ISC).
- **NFR-7.** No network calls beyond OpenRouter, and only on user action. No telemetry. Connecting the account (AI-24)
  adds a **temporary local listener**: only on loopback interfaces, only while the user is connecting, for a single
  response and for at most ten minutes. The app opens only OpenRouter pages in the browser, from a fixed list.
- **NFR-8.** The code contains nothing specific to an operating system outside modules isolated behind an interface
  that is the same on all systems (A-104).
- **NFR-9.** During development the repository, build artifacts included, stays under **20 GB** (§9, rules 18–20).

## 4. Technology stack

### 4.1 Choice

| Area | Choice | Notes |
| --- | --- | --- |
| Desktop shell | **Tauri 2** | System webview (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux), small package, Rust backend in the same process |
| Engine, DSP, analysis, AI, data model | **Rust** (Cargo workspace) | Native real time; compiler and `cargo test` as strict verification for the agents |
| Audio output | `cpal` | CoreAudio / WASAPI / ALSA with the same code |
| Decoding | `symphonia` (MPL-2.0) | WAV, FLAC, AIFF; MP3/AAC for references |
| WAV writing | `hound` | |
| FLAC writing | `flacenc` (or a pure-Rust equivalent) | |
| AAC writing | System encoder (AudioToolbox on macOS, Media Foundation on Windows) behind an interface | Platform-specific module, with signing (A-072) and the menu bar (A-104) |
| Metadata | `lofty` (or equivalent) | WAV, FLAC, M4A tags |
| Resampling | `rubato` | Import, references, output, export |
| FFT | `realfft` / `rustfft` | |
| Lock-free queues | `rtrb` (or equivalent) | Parameters and meters between the audio thread and the rest |
| HTTP | `reqwest` with `rustls` | OpenRouter only |
| Keychain | `keyring`; `security-framework` to know whether the build is signed | Keychain / Credential Manager / Secret Service; private file where the keychain would ask for the password (AI-11) |
| Contracts | `serde` + `schemars`; generated TypeScript types (`tauri-specta` or `ts-rs`) | A single definition of the types, in Rust |
| Interface | **React + TypeScript + Vite** | |
| UI state | `zustand` | Mirror of the core's state, never a source of truth |
| Style | CSS with the design tokens as variables (Tailwind 4 allowed) | |
| Accessible primitives | Radix UI | Menus, dialogs, popovers, tooltips |
| Dense graphics | Canvas 2D; WebGL only if needed for 60 fps | Timeline, waveforms, meters, spectrum, curves |
| Tests | `cargo test`, `proptest`, `criterion`; `vitest`; Playwright | |
| License gate | `cargo-deny`, npm license check | |

Versions are pinned at the start of implementation (latest stable of each), with lockfiles committed.

### 4.2 Why this stack

The criterion was twofold: maximum effectiveness of agentic development, no evident compromise on performance,
interface and hardware.

| Alternative | Audio engine | UI and design | Agentic development | Portability | Outcome |
| --- | --- | --- | --- | --- | --- |
| **Tauri 2 + Rust + React** | Native, safe by construction | The canvas design is already HTML/CSS: it translates almost one to one | Very high: strict compiler, fast tests, UI verifiable in the browser | Same code on the three systems | **Chosen** |
| JUCE (C++) | Industry standard, plugin hosting ready | Imperative UI, slow to iterate, far from the web design | Medium: slow builds, runtime memory errors, UI not automatically verifiable | Good | Discarded: its main advantage (plugins) is out of scope; commercial license |
| Swift + AVAudioEngine | Excellent on Apple | SwiftUI struggles with dense timelines; design to be retranslated | Medium: UI verification is hard for an agent | None | Discarded: shuts out Windows/Linux (P-002) |
| Electron + Python sidecar (PoC) | Python cannot sustain real time; GPL | Same as Tauri | High on the UI, low on the engine | Good | Discarded: P-001 and P6 |
| Electron + native addon | Native | Same as Tauri | High | Good | Discarded: same design as Tauri with a heavier package and one more bridge |

Points that make agentic development effective with the chosen stack:

- **Verification without ears.** The engine is a pure library: the offline render is the same code as playback, so
  tests on synthetic signals cover what will be heard.
- **Numeric oracle.** The PoC's measurements produce expected values to compare the Rust port against (§5.7).
- **UI verifiable in the browser.** The interface talks to the backend through a `Backend` interface with two
  implementations: Tauri and an **in-memory fake backend**. With the second the whole UI runs in a browser: Playwright
  tests it and the agent compares it visually with the design.
- **A single language for contracts.** Types are born in Rust and generated for TypeScript: a mismatch is a compile
  error, not a runtime bug.

Known limits and countermeasures:

- **The webview is not a native toolkit.** Real time never crosses it: audio lives in Rust, the UI receives only meters
  and position at 30–60 Hz over a binary channel and draws on canvas. Waveforms arrive as precomputed peak pyramids, not
  as audio.
- **Linux uses WebKitGTK**, which is weaker: a risk of future work, not of v1.
- **Own compressor and reverb** will not sound like the PoC's: the port is validated with measurements and with the
  author's final listening (§8.5), as foreseen by PRD §7.1.

## 5. Architecture

### 5.1 Repository structure

The PoC is **frozen** and confined to `poc/` (A-068). The product lives in `app/`. The product website, when there is
one, will have its own `web/` folder.

```text
stemwise/
  CLAUDE.md                 # rules for the agents
  docs/
    FOUNDATION.md           # this document
    app/                    # PROGRESS, DECISIONS, REQS-JUSTIFIED, VALIDATION, dsp.md
  design/                   # hand-off of the design session (§7)
  poc/                      # Python PoC: read-only, archive and oracle
    src/, tests/, schemas/, scripts/, pyproject.toml, Makefile
    docs/                   # PRD.md, analysis.md, DECISIONS.md, PROGRESS.md, milestones/
    POC-RULES.md            # working rules of the PoC, archived
  app/
    Makefile                # setup, check, test, bench, reqs, oracle, dev, build
    Cargo.toml              # workspace
    crates/
      stemwise-core/        # project document, commands, undo/redo, persistence, schemas
      stemwise-dsp/         # biquad, compressor, saturation, reverb, delay, limiter, pan, meter
      stemwise-analysis/    # measurements, masking, target profile, evaluation
      stemwise-engine/      # graph, transport, streaming, automation, offline render, cpal output
      stemwise-ai/          # Decider, prompt, OpenRouter, fake decider, validation, loop
      stemwise-cli/         # commands without an interface
    src-tauri/              # Tauri commands and channels: thin shell over the crates
    ui/                     # React + TypeScript
      src/backend/          # Backend interface: tauri.ts, mock.ts
    schemas/                # JSON Schemas exported from the Rust types
    fixtures/oracle/        # expected values generated by the PoC (JSON, never audio)
    scripts/                # reqs, oracle, licenses
```

Crate dependencies: `dsp` and `core` depend on no other crate; `analysis` → `dsp`; `engine` → `core`, `dsp`; `ai` →
`core`, `analysis`, `engine` (offline render of the loop); `cli` and `src-tauri` → all. No crate below `src-tauri`
knows Tauri.

### 5.2 Project package

```text
MySong.stemwise/
  project.json          # document: schema version, tracks, clips, chains, automation, sections, brief, master
  audio/                # imported files (immutable), name = content hash
  references/
  cache/                # peaks, analysis (regenerable, deletable)
  ai/                   # one folder per call: request.json, response.json, plan.json, evaluation.json
  snapshots/
  mixdowns/             # mixdown-<n>.wav + mixdown-<n>.json (hash of the mix state, trim, measurements)
  masters/              # exported files + report
  autosave/
```

`project.json` contains only paths relative to the package. Saving is atomic (write to a temporary file and rename).

### 5.3 State, commands and history

- The **project document** lives in the Rust core and is the only source of truth. The UI keeps a read-only copy,
  updated by events.
- Every mutation is a `Command` (data, not a closure) with `apply` and `invert`. A `Transaction` groups several
  commands into one history step. Continuous gestures open a transaction on press and close it on release; during the
  gesture the engine receives the values as a preview without touching the history.
- View state (zoom, selection, mute/solo, open panels) lives in the UI and, where needed, in a separate section of the
  project that generates no history.
- The audio engine receives an immutable snapshot of the graph at every structural change (atomic pointer swap, the
  old one released off the audio thread) and parameter changes on a lock-free queue.

### 5.4 Audio engine

```text
reader thread ──ring buffers──▶ audio thread (cpal) ──▶ device
                                           │   ▲
                                 meter/pos │   │ parameters, graph snapshots
                                           ▼   │
                                        control thread ◀──▶ UI (Tauri channels)
```

- **Structured** acyclic graph: clip → track (chain) → nested groups (chain) → mix bus; post-fader sends to the
  returns; returns → group or mix bus; compressor sidechain inputs. At every structural change the core recomputes the
  topological order and rejects cycles; the audio thread receives the order already computed.
- The **offline render** calls the same `process` function as the callback, with synchronous file reading.
- Determinism holds for the same build and the same architecture; bit-for-bit identity across different systems is
  neither guaranteed nor required.
- The master phase uses a second, simpler graph (mixdown → master chain), with the same transport.

### 5.5 AI contracts

The contracts are Rust types exported to `app/schemas/`. No library names in the JSON. The examples are illustrative;
the limits are normative.

**`mix_plan` (version 3)** — extends the PoC's (PRD §6.3); version 3 adds `structure`, the reverb algorithms and the
compressor's `mix` (P-011):

```json
{
  "version": 3,
  "iteration": 1,
  "summary": "Vocal forward in its own plate, double-tracked guitars wide on the sides, tighter drums, fuller chorus.",
  "structure": {
    "returns": {
      "n1": { "type": "reverb", "name": "Vocal plate", "rationale": "A short, bright space for the vocal only." },
      "n2": { "type": "delay", "name": "Slap", "rationale": "Slapback to give the vocal body without a tail." }
    },
    "groups": {
      "n4": { "name": "Drums", "members": ["t1", "t3"], "rationale": "A bus to compress the drums in parallel." }
    }
  },
  "returns": {
    "r1": { "type": "reverb", "algorithm": "hall", "decay_s": 2.2, "size": 0.6, "damping": 0.5, "modulation": 0.2, "predelay_ms": 20, "return_hpf_hz": 250, "return_lpf_hz": 9000, "return_gain_db": -6.0 },
    "r2": { "type": "delay", "time_ms": 375, "feedback": 0.25, "feedback_hpf_hz": 200, "feedback_lpf_hz": 6000, "return_gain_db": -9.0 },
    "n1": { "type": "reverb", "algorithm": "plate", "decay_s": 1.1, "size": 0.4, "damping": 0.3, "modulation": 0.3, "predelay_ms": 40, "return_hpf_hz": 300, "return_lpf_hz": 12000, "return_gain_db": -8.0 },
    "n2": { "type": "delay", "time_ms": 110, "feedback": 0.0, "feedback_hpf_hz": 200, "feedback_lpf_hz": 5000, "return_gain_db": -12.0 }
  },
  "tracks": {
    "t4": {
      "gain_db": 2.0, "pan": 0.0, "hpf_hz": 90, "lpf_hz": null,
      "eq": [{ "type": "peak", "freq_hz": 300, "gain_db": -2.5, "q": 1.4 }],
      "compressor": { "threshold_db": -18.0, "ratio": 3.0, "attack_ms": 10.0, "release_ms": 120.0, "knee_db": 6.0, "makeup_db": 3.0, "mix": 1.0, "sidechain": null },
      "saturation": null,
      "sends_db": { "n1": -14.0, "n2": -18.0, "r2": -24.0 },
      "sections": { "s3": { "gain_offset_db": 1.5, "send_offsets_db": { "r2": 6.0 }, "eq_gain_offsets_db": [1.0] } },
      "rationale": "Cut at 300 Hz to remove the overlap with the guitars; +1.5 dB and a delay throw in the chorus."
    },
    "t2": {
      "gain_db": 0.0, "pan": 0.0, "hpf_hz": 40, "lpf_hz": null, "eq": [],
      "compressor": { "threshold_db": -24.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 90.0, "knee_db": 3.0, "makeup_db": 0.0, "mix": 1.0, "sidechain": "t1" },
      "saturation": null, "sends_db": {}, "sections": {},
      "rationale": "The bass dips slightly on every kick hit (sidechain from t1) to clear the low end."
    }
  },
  "groups": {
    "g1": { "gain_db": -1.0, "eq": [], "compressor": null, "saturation": null, "sends_db": {}, "rationale": "Guitars 1 dB down to make room for the vocal." },
    "n4": {
      "gain_db": 0.0, "eq": [],
      "compressor": { "threshold_db": -30.0, "ratio": 6.0, "attack_ms": 3.0, "release_ms": 80.0, "knee_db": 3.0, "makeup_db": 8.0, "mix": 0.35, "sidechain": null },
      "saturation": null, "sends_db": {},
      "rationale": "Parallel compression: 35% heavily compressed under the natural drums makes them denser."
    }
  }
}
```

**Structure** (AI-17 → AI-20). `structure` lists what the plan creates: returns (`type`, `name`, `rationale`) and
groups (`name`, `members`, `rationale`). The new elements have placeholder ids already listed in the project's schema
(`n1` … `n3` for returns, `n4` … `n6` for groups; `null` where they are not needed), and are used in the rest of the plan like
the existing ones: `returns.<n>` and `groups.<n>` for the parameters, `sends_db.<n>` for the sends. On application the
core assigns the real ids. `name` is in English (the interface language), at most 24 characters. The model does not
create, duplicate or delete audio tracks, does not delete returns or groups, and does not change a track's output
except by putting it in a new group. `structure` is missing from the schema when structural changes are disabled
(AI-18) and in the iterations after the first (AI-19), which receive the already-decided structure as context.

Auto mix guarantees that at least one reverb return and one delay return exist (created, if missing, as part of the
proposal). `sidechain` is the id of a track or a group, or `null`; validation rejects sources that would create a
cycle.

Limits: those of the table in PRD §6.3 for the parameters already present (except the reverb's `room_size`, which no
longer exists), plus:

| Parameter | Minimum | Maximum |
| --- | --- | --- |
| Compressor `mix` | 0.1 | 1 |
| Reverb `algorithm` | `plate`, `room`, `hall` | |
| Reverb `decay_s` | 0.3 | 6 |
| Reverb `size` | 0 | 1 |
| Reverb `damping` | 0 | 1 |
| Reverb `modulation` | 0 | 1 |
| Reverb `return_lpf_hz` | 2,000 | 20,000 |
| New returns per proposal | 0 | 3 |
| New groups per proposal | 0 | 3 |
| Members of a new group | 2 | — |
| Compressor `knee_db` | 0 | 12 |
| Saturation `drive_db` | 0 | 18 |
| Saturation `mix` | 0 | 1 |
| `sends_db.<return>` | −60 | −6 |
| Reverb `predelay_ms` | 0 | 100 |
| Delay `time_ms` | 20 | 1,000 |
| Delay `feedback` | 0 | 0.7 |
| Delay `feedback_hpf_hz` | 20 | 800 |
| Delay `feedback_lpf_hz` | 1,000 | 20,000 |
| Delay `return_gain_db` | −24 | 0 |
| Section `gain_offset_db` | −6 | +6 |
| Section `send_offsets_db.<return>` | −12 | +12 |
| Section `eq_gain_offsets_db[]` (per band position) | −4 | +4 |

The trim (AI-7) is not in the plan: the engine computes it from the loudness measurement and adds it to the proposal.
Locked tracks appear in the message as context, with `"locked": true`, and not in the returned plan.

**`mix_plan` (version 4)** (P-018) — version 3 plus the new blocks, ideas and offsets. Only the additions are shown:

```json
{
  "version": 4,
  "ideas": {
    "i1": { "name": "Wide chorus on the keys", "sections": ["s3", "s5"], "rationale": "The keys open up in the choruses and leave the center to the vocal." },
    "i2": { "name": "Filter build into the last chorus", "sections": ["s4"], "rationale": "The pad starts closed and opens up until the last chorus." }
  },
  "tracks": {
    "t4": {
      "hpf_resonance": null, "lpf_resonance": null,
      "blocks": [
        { "id": "k1", "type": "deesser", "position": "after_compressor", "freq_hz": 6500, "threshold_db": -28.0, "max_reduction_db": 5.0, "mode": "split" }
      ],
      "rationale": "High sibilance (+7 dB at 7 kHz): de-esser after the compressor."
    },
    "t5": {
      "blocks": [
        { "id": "k2", "type": "dynamic_eq", "position": "before_compressor", "bands": [
          { "type": "peak", "freq_hz": 3000, "q": 1.2, "threshold_db": -30.0, "range_db": -3.0, "attack_ms": 10.0, "release_ms": 120.0, "sidechain": "t4" }] }
      ],
      "rationale": "The guitars dip at 3 kHz only while the vocal sings, instead of a fixed cut."
    },
    "t7": {
      "blocks": [
        { "id": "k3", "idea": "i1", "type": "modulation", "position": "last", "mode": "chorus", "rate": "1/4", "rate_hz": null, "depth": 0.4, "feedback": 0.0, "voices": 2, "stages": null, "spread_deg": 120, "mix": 0.35 }
      ],
      "sections": { "s3": { "blocks_on": ["k3"] }, "s5": { "blocks_on": ["k3"] } }
    },
    "t8": {
      "lpf_hz": 18000, "lpf_resonance": 2.0,
      "sections": { "s4": { "idea": "i2", "lpf_offset_oct": -3.0 } }
    }
  },
  "returns": {
    "r1": {
      "blocks": [
        { "id": "k4", "type": "compressor", "position": "last", "threshold_db": -30.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 250.0, "knee_db": 6.0, "makeup_db": 0.0, "mix": 1.0, "sidechain": "t4" }
      ]
    }
  }
}
```

**Blocks.** `blocks` lists only the blocks the plan **adds or changes**: a block that is not there stays as it is
(unlike the master, because tracks are many and the response must stay short, A-062). `id` is the id of an existing
block (the analysis lists them, ANA-19) or a new word for a new block; a new block has its own `position` (AI-25);
`"bypass": true` turns off an existing block. The classic blocks stay in the version 3 fields (`hpf_hz`, `eq`,
`compressor`, `saturation`), to which `hpf_resonance`, `lpf_resonance` and `saturation.character` are added. The
per-project schema has an `anyOf` on the block types, like `master_plan` v2. In returns, `blocks` is allowed only if
structural changes are permitted (a return is shared, AI-25); the reverb also has `shimmer_semitones` and
`shimmer_amount`, the delay `mode`, `character`, `wow` and `drive_db`.

**Ideas.** `ideas` is missing from the schema when ideas are disabled (AI-27) and in the iterations after the first,
which receive the already-decided ideas as context. An element belongs to an idea with `"idea": "<id>"`: a block, a
track section (all its offsets), an entry of `structure`. An idea's send is written in the track's
`idea_sends_db.<idea>.<return>`, separate from `sends_db`.

Limits added by version 4:

| Parameter | Minimum | Maximum |
| --- | --- | --- |
| Ideas per proposal | 0 | 3 |
| New blocks per chain in a proposal | 0 | 3 |
| `hpf_resonance`, `lpf_resonance` | 0.5 | 4 |
| Section `hpf_offset_oct`, `lpf_offset_oct` | −3 | +3 |
| Gate `threshold_db` / `range_db` / `ratio` | −70 / 0 / 1 | −10 / 40 / 20 |
| Gate `attack_ms` / `hold_ms` / `release_ms` | 0.1 / 0 / 10 | 20 / 200 / 1,000 |
| De-esser `freq_hz` / `max_reduction_db` | 3,000 / 0 | 12,000 / 10 |
| Dynamic EQ: bands / `range_db` / `q` | 1 / −9 / 0.3 | 2 / +4 / 6 |
| Transient `attack_db` / `sustain_db` | −9 | +9 |
| Track clipper `threshold_db` | −18 | 0 |
| Track multiband: bands / band gain | 2 / −6 | 4 / +6 |
| Exciter `freq_hz` / `amount` | 1,500 / 0 | 12,000 / 0.6 |
| Bass enhancer `freq_hz` / `harmonics` | 40 / 0 | 160 / 0.8 |
| Width `width` / `mono_below_hz` | 0 / 0 | 1.6 / 250 |
| Modulation `rate_hz` / `depth` / `feedback` / `mix` | 0.02 / 0 / −0.8 / 0 | 8 / 1 / 0.8 / 0.6 |
| Tremolo `rate_hz` / `depth` | 0.1 / 0 | 12 / 0.8 |
| Harmonizer: voices / interval / mix | 1 / −12 / 0 | 2 / +12 / 0.5 |
| Doubler `detune_cents` / `delay_ms` / `mix` | 0 / 5 / 0 | 25 / 40 / 0.6 |
| Reverb `shimmer_amount` | 0 | 0.6 |
| Delay `wow` / `drive_db` | 0 / 0 | 1 / 9 |

Musical values (`rate`, the delay's `time`) are allowed only if the project has a tempo. The other parameters of the new
blocks have in the plan the same limits as the interface (FX-13 → FX-23). Version 3 plans kept in projects can still be
read (AI-23).

**`analysis` (version 3)** — the PoC's (PRD §6.2) plus: `sections` (id, name, start, end), `tracks.<id>.section_lufs`,
`tracks.<id>.section_active_pct`, `mix.section_bands_db`, `families` (ANA-16), `returns` (id, type and, for reverbs,
algorithm), `doubles` (ANA-18: pairs of tracks, score, `identical`) and, for each track, `name`, `group`, `locked`. The
ids sent are neutral (`t1`, `g1`, `s1`). **Version 4** (P-018) adds the measurements of ANA-19 (`sibilance_db` and
`sibilance_hz` for vocals, `crest_db`, `transient_db`, `floor_db` for drums, `stereo` for stereo tracks), the project's
`tempo` and `meter` if known, and `blocks` for every track, group and return (neutral id `k…`, type, bypass, position
of the non-classic blocks).

**`master_plan` (version 2)** (P-017):

```json
{
  "version": 2,
  "summary": "Same loudness with a third of the limiter work: the peaks come down earlier.",
  "target": { "lufs": -9.6, "true_peak_db": -1.0 },
  "input_gain_db": 0.0,
  "chain": [
    { "type": "compressor", "id": "b3", "threshold_db": -9.0, "ratio": 3.0, "attack_ms": 3.0, "release_ms": 90.0, "knee_db": 4.0, "makeup_db": 0.0, "mix": 0.6, "listen_above_hz": 120, "why": "…" },
    { "type": "eq", "id": "b1", "bands": [{ "type": "low_shelf", "freq_hz": 45, "gain_db": -1.5, "q": 0.7 }], "why": "…" },
    { "type": "multiband", "id": "b4", "crossovers_hz": [120, 4000], "bands": [
      { "threshold_db": -14.0, "ratio": 2.5, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0 },
      { "threshold_db": 0.0, "ratio": 1.0, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0 },
      { "threshold_db": 0.0, "ratio": 1.0, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0 }], "why": "…" },
    { "type": "eq", "id": "tone", "bands": [{ "type": "high_shelf", "freq_hz": 9000, "gain_db": 1.0, "q": 0.7 }], "why": "…" },
    { "type": "clipper", "id": "b7", "take_off_db": 4.0, "softness": 0.4, "why": "…" }
  ],
  "limiter": { "release_ms": 80.0 },
  "sections": [{ "section": "s3", "level_db": 0.5, "side_gain_db": 0.0, "limiter_release_ms": 0.0, "eq": [{ "block": "tone", "gains_db": [0.6] }] }],
  "rationale": { "order": "…", "limiter": "…", "sections": "…" }
}
```

`chain` lists only the blocks that work, in the order they work: a master block that the plan does not list is turned
off and stays at the end with its settings. `id` is the id of a block the master already has (the analysis lists them
in `current_chain`) or any word for a new block. Types: `eq`, `match_eq`, `compressor`, `multiband`, `width`,
`saturation`, `clipper`. Every block carries its reasoning in `why`; `rationale` has only `order`, `limiter` and
`sections`. The proposal is applied whole or **item by item** (each block, the order, the limiter, the sections, the
input, the target). Version 1 plans kept in projects can still be read (AI-23).

**Master analysis (version 3)** — what the model reads before deciding a master (MAS-6): `mixdown` (`lufs`,
`true_peak_db`, `peak_to_loudness_db`, `short_crest_db`: median crest over 400 ms windows, `peak_share_pct`: share of
samples within 3, 6 and 9 dB of the peak, `loudness_range_lu`, `bands_db`, `stereo`, `low_corr`), `target` (average
profile of the references), `references` (loudness and peak-to-loudness distance of each, without names),
`band_delta_db`, `destination` (with `peak_to_loudness_db` and `peak_reduction_needed_db`: by how much the peaks must
come down overall) and `current_master`, the **evaluation** of the current master. An evaluation has `lufs`,
`on_target`, `loudness_gain_db`, `peak_to_loudness_db`, `short_crest_db`, `short_crest_lost_db`, `peak_share_pct`,
`blocks` (for every compressor, multiband and clipper that is on, with its id) and `limiter` (`max_db`, `mean_db`,
percentage of time with any reduction and over 1, 3 and 6 dB), `spectral_distance_db`, `band_delta_db`. From version 3
the analysis also carries `sections` (neutral id, name, start, end and the mixdown's loudness in each) and
`current_chain` (the master's blocks with id, type and state). The same evaluation accompanies every plan in the
iteration history.

Master limits: at most 9 blocks, two per type (one for `match_eq` and `width`); EQ up to 6 bands, gain ±6 dB;
`match_eq.amount` 0–1; ratio 1–4; `compressor.mix` 0.2–1; `compressor.listen_above_hz` 0–300; multiband with 2–6 bands,
crossover 40–12,000 Hz at least a third of an octave apart, band gain ±6 dB; `side_gain_db` ±6; `mono_below_hz` 0–250;
`clipper.take_off_db` 0–6; `target.lufs` −24…−6; `target.true_peak_db` −3…−0.1; `limiter.release_ms` 20–500; section
offsets: level ±2 dB, sides ±3 dB, release ±100 ms, EQ ±2 dB.

The system prompt starts from the PoC's (`poc/src/stemwise/decision/prompt.py`), with the limits table **generated from
the schema**, never copied by hand.

### 5.6 Interface between UI and core

- Commands (request/response) for mutations and queries; events for state changes (document patches); binary channel
  for meters, position and spectra; dedicated protocol for the peak pyramids.
- The UI contains no domain logic: no dB, timing or validation calculations duplicated from the core, except for
  drawing.
- `ui/src/backend/mock.ts` implements the same interface with an in-memory demo project (tracks, synthetic waveforms,
  simulated meters, a sample AI proposal): it is what Playwright and visual verification use.

### 5.7 The PoC as oracle

`poc/src/stemwise/` is not modified. `make oracle` (in `app/`) runs a script that generates synthetic signals with a
fixed seed, measures them with the PoC's functions and writes to `app/fixtures/oracle/` **only JSON** (signal
parameters and expected values). The Rust tests regenerate the same signals and compare:

| Area | Tolerance against the oracle |
| --- | --- |
| Integrated loudness, LRA | ±0.1 LU |
| True peak, peak, RMS, crest | ±0.1 dB |
| Energy per band, shares, centroid | ±0.2 dB · ±1 percentage point · ±2% |
| Masking, correlations | ±0.02 |
| Biquad (frequency response), pan, gain | ±0.01 dB |

Compressor, saturation, reverb, delay and limiter have no oracle: they are verified by properties (static gain, time
constants, ceiling respected, stability, absence of NaN and denormals) documented in `docs/app/dsp.md`. The signal
generators must be defined identically in the two languages (closed formulas or samples written in the JSON for short
signals), never with the libraries' random generators.

## 6. Interface inventory

Binding list for design and implementation. Reference size 1440×900; minimum 1280×720.

**Screens**

1. **Welcome** — recent projects, new project, open; first launch with an invitation to set up the AI (skippable).
2. **New project / Import** — drop area, file list with format, channels, duration, proposed and editable role;
   warning for a different sample rate; brief (genre, mood, notes); references.
3. **Mix workspace** — the main screen:
   - top bar: project name, **Mix | Master** phase selector, transport, position, loop, undo/redo, bus meters (true
     peak, LUFS), engine load;
   - timeline: ruler, section lane, track headers (name, role, color, mute/solo, lock, mini fader), clips with
     waveforms, expandable automation lanes, cursor, loop region;
   - mixer (bottom panel or alternate view): strips with meter, fader, pan, sends, gain reduction; groups, returns,
     mix buses; with P-013: chain marks on every header and strip (UI-8), one send per return with the label column
     (UI-9), grid **Overview** view (UI-11), also on a large project with short lanes;
   - inspector of the selected track: chain with bypass, EQ editor, compressor, saturation, sends, the **AI
     rationale** for that track;
   - AI panel: brief, references, Auto mix, revision bar in words, "Allow structural changes" (AI-18) and "Allow
     creative ideas" (AI-27) switches, cost estimate before sending, project cost so far with access to "AI usage";
   - history panel: undo steps, snapshots, A/B comparison.
4. **Mix workspace, proposal under review** — summary, track list with diff and rationale, **new returns and groups
   as rows of their own** (marked as new, with name, rationale, parameters and, for groups, the members; AI-20),
   before/after evaluation, A/B by ear, apply all / apply selected / discard; with "apply selected" you see which
   new elements will be created; with P-018 the new blocks in the diff and the **Creative ideas** subsection (one
   card per idea, check off by default, listening to the idea alone).
5. **Mix workspace, AI at work** — progress per phase and per iteration, cancel.
6. **Editing** — tools (select, cut, automation), drag states, fade, snap, multiple selection, clip and track
   context menus.
7. **Returns & routing** — return panels (reverb with algorithm selector and the parameters of FX-5, delay),
   creating and deleting a return, a track's output selector, nested groups in the mixer and in the timeline,
   sidechain source selector and **mix** control in the compressor (FX-3), choice of the parameter to automate in a
   lane; with P-013: the selection's connections (who sends to a return, where a track goes, who listens to it in
   sidechain; UI-10) and the list of who sends in the return's lane.
8. **Bounce to Master** — confirmation, progress, outcome with the mixdown measurements.
9. **Master workspace** — mixdown waveform, modular master chain, loudness meters and spectrum with reference
   profile, target, master AI panel, loudness-matched A/B (master / mixdown / reference), outdated mixdown warning.
10. **Export** — formats (WAV, FLAC, AAC, also together), sample rate, bits, dither, metadata (with ISRC and cover
    art), folder; progress; final report.
11. **Settings** — Audio, AI, Appearance, Shortcuts.
12. **AI usage** — summary (calls, tokens, cost, by type and by model), call history with filters, readable detail
    of a call (request and response, with the raw JSON on request), export, empty state; confirmation of exceeding
    the spending cap.
13. **Cross-cutting states** — empty, loading, errors of §3.11 UI-6, destructive confirmations, notifications.
14. **Assistant setup** (P-015) — connecting the OpenRouter account from the browser, waiting, page shown in the
    browser, credit, ready, negative outcomes, key pasted by hand, and the states of exhausted credit, key limit and
    key no longer valid in the assistant column.
15. **Master chain** (P-017) — ten-module chain with free order in the middle, clipper, compressor with mix and
    detector filter, multiband, per-section offsets, measured proposal, target too high for the mix.
16. **Track chain** (P-018) — the block-based chain in the track, group and return inspector (UI-12): menu per
    family, order, drag states, one editor for each new block of FX-12 → FX-23, marks and Overview with long
    chains, delay and reverb with the new modes.

17. **Explain this mix** (P-019) — the view that explains the mix (UI-14 → UI-16): the whole mix, a track, a return,
    a sidechain with a term explained, a section, a large project with collapsed groups, a mix never touched, the
    minimum window, the full text.

18. **Guided tour** (P-020) — the tour (UI-17 → UI-19): opening card on the Welcome, one step with the spotlight
    for each area (top bar, timeline, header, mixer, inspector, assistant, proposal, history, Explain, phase
    selector, Master, export), closing card, a step that waits for the proposal or the mixdown, the minimum window,
    the link in the Welcome and the Help item of the app menu (UI-20), the notification after `Skip tour`.

**Components** (each with its states: normal, hover, active, focus, disabled, automated, locked where it makes
sense): fader, knob, pan, meter (peak/RMS, gain reduction, LUFS), buttons and toggles (mute, solo, lock, bypass),
numeric field, menu, tooltip, dialog, toast, tab, clip, fade handles, automation point, section, channel strip,
chain module, EQ curve, transfer curve, ruler, playback cursor, progress bar, proposal card, diff row, AI call row,
measurements table, routing selector, sidechain indicator; with P-018: track chain block, block menu, idea card;
with P-013: chain marks, send dots, automation mark, Overview cell, mixer label column; with P-020: tour spotlight
and callout.

## 7. From design to implementation

The design session delivers, in the repository's `design/` folder:

- `design/README.md` — index: chosen direction, principles, list of screens and files, notes for implementers
  (behaviors that cannot be deduced from the boards: resize, density, animations, states).
- `design/tokens.css` — all tokens as CSS variables (colors, typography, spacing, radii, shadows, durations, track
  colors, colors of the two phases).
- `design/screens/` — one board per screen and state of §6 (self-contained HTML/CSS, or a more specific measured
  image).
- `design/components/` — the component sheet with all states.

If the design session cannot write to the repository, the author exports the boards and copies them into `design/`
before starting the implementation. The implementation does not start without `design/README.md` and
`design/tokens.css`.

During implementation the design is normative for look and layout; this document is normative for behavior. Where
the design lacks a state, the agent derives it from the existing tokens and components and records it in
`docs/app/DECISIONS.md`.

## 8. Implementation plan

### 8.1 Phases

They run in order, **with no pauses between one and the next**. Each phase closes when its gate is green; status is
updated in `docs/app/PROGRESS.md`.

| Phase | Content | Gate (besides `make check` green) |
| --- | --- | --- |
| **F0 Foundations** | Cargo workspace, Tauri app that starts, UI with fake backend, design tokens imported, `Makefile`, license gate, requirements script, `make oracle`, `make disk` and `make clean-stale` with the build profiles of rule 18 | The app opens with `make dev`; the UI runs in the browser with the fake backend; `make oracle` writes the fixtures; `make disk` reports usage per folder |
| **F1 Core** | Project document, schemas, commands and undo/redo, package, autosave, file import and copy, peak pyramids | PRJ, HIS covered; routing without cycles (TRK-3); property tests: every random sequence of commands followed by the undos returns to the initial document |
| **F2 DSP** | All FX and MAS processors, meters, loudness, true peak | Parity with the oracle (§5.7); processor properties; no allocation in `process` |
| **F3 Engine** | Structured graph (nested groups, multiple returns, sidechain), clips, automation of all parameters, offline render, streaming, `cpal` output, transport; **vertical slice**: open a project, play, move a fader from the UI | ENG covered; stable render hash; block-size invariance; NFR-1 and NFR-4 benchmarks |
| **F4 Analysis** | Port of the measurements, per-section measurements, families, evaluation | ANA covered; parity with the oracle; ANA-17 |
| **F5 AI** | Contracts, prompts, fake decider, OpenRouter, validation and retries, loop, revision, lock, log and cost registry, master AI | Full offline flow from `stemwise-cli` with the fake decider; HTTP client tested against a fake local server |
| **F6 Mix UI** | Timeline, mixer, inspector, clip editing, automation, sections, returns, history | UI-2 for screens 1–3, 6, 7; Playwright on the main flows; visual comparison with the design |
| **F7 AI UI** | Brief, references, auto mix, proposal under review with A/B, revision in words, AI usage panel, snapshots | Screens 4, 5, 12; AI-14 → AI-16; full flow in the app with the fake decider |
| **F8 Master** | Mixdown, master graph, meters, targets, master AI, loudness-matched A/B, export in the three formats with metadata | MDN, MAS covered; export true peak within the ceiling on test signals; screens 8–10 |
| **F9 Polish** | Settings, errors and empty states, shortcuts, accessibility, UI performance | UI-3 → UI-7, SET-1, NFR-3, NFR-5; screens 11, 13 |
| **F10 Delivery** | `.app`/`.dmg` package, final independent review, README, validation guide | `make build` produces the package; `make reqs` with no gaps; `docs/app/VALIDATION.md` complete |
| **F11 Reverb and parallel compression** (P-011) | Three reverb algorithms in place of the comb one, new parameters in the document with schema migration, compressor `mix`, engine and automation, updated design boards (screen 7, compressor panel) and their implementation, `docs/app/dsp.md` | FX-3, FX-5, FX-9 covered; stable render hash and block invariance with the new reverbs; `make bench` (NFR-1 with 4 reverbs); visual comparison with the design |
| **F12 Structural AI mix** (P-011) | Measurement of doubles, `mix_plan` and `analysis` contracts version 3, validation and application of the structure, loop with stable structure, prompt and fake decider with the techniques, updated design boards (screens 3 and 4) and their implementation, package rebuilt, VALIDATION updated | ANA-18, AI-17 → AI-21 covered; full flow from `stemwise-cli` and in the app with the fake decider, with returns and groups created and undone in one step; client tested against the fake server with the v3 schema; independent review; `make reqs` with no gaps |
| **F13 Mix state at a glance** (P-013) | Design boards (3e–3g, 7f–7h, C7, and the existing boards regenerated with the marks), then, **after the author approves the boards**: marks on headers and strips, sends per return with the label column, selection connections, Overview view; no change to core, engine and contracts | UI-8 → UI-11 covered; no timeline redraw tied to the playhead (A-064, A-065) and 60 fps with 32 tracks (NFR-3); visual comparison with the design; `make reqs` with no gaps |
| **F14 Free master chain** (P-017) | Chain as a list of repeatable blocks with free order, clipper, multiband from two to six bands, mix and filter on the compressor detector, per-section offsets, project schema 3 with migration, `master_plan` v2 with per-block evaluation, rack and editor (boards 15a–15j), measured proposal applicable item by item, warning on a target that is too high | MAS-11 → MAS-15 covered; render identical at every processing block size; schema 2 projects opened with the same sound; flow with the fake decider; `make reqs` with no gaps |
| **F15 Block-based track chain** (P-018) | Design boards for **all** the phases of P-018 (screen 16, updates to 3, 4 and 7, components), then, **after the author's approval**: track and group chain as a list, return chain, resonant filters, bypass fade, constant latency, project schema 4 with migration, rack inspector, marks and Overview on the list | FX-10 → FX-12, UI-12 covered; schema 3 projects opened with the same hash; render identical at every processing block size; no glitch when changing structure; visual comparison with the design |
| **F16 New effects** (P-018) | The processors and editors of FX-13 → FX-23 (gate, de-esser, dynamic EQ, transient shaper, track clipper and multiband, saturation characters, exciter, bass enhancer, width, modulation, tremolo, harmonizer, doubler, shimmer, ping-pong and tape), automation of their parameters, `docs/app/dsp.md` | FX-13 → FX-24 covered; `make bench` with the FX-24 case; visual comparison with the design |
| **F17 Creative ideas** (P-018) | ANA-19 measurements, `analysis` v4 and `mix_plan` v4, new blocks and offsets in the plan, ideas with validation and application, loop with ideas held fixed, prompt and fake decider with the new techniques, proposal with Creative ideas and switch, package rebuilt, VALIDATION updated | ANA-19, AI-25 → AI-27, UI-13 covered; full flow from `stemwise-cli` and in the app with the fake decider, with ideas applied and undone in one step; client tested against the fake server with the v4 schema; independent review; `make reqs` with no gaps |
| **F18 Explain this mix** (P-019) | Boards of screen 17, then, **after the author's approval**: background measurement of levels (ANA-20), Explain view with map and rule-based text, glossary, text copy, ⌘E shortcut | ANA-20, UI-14 → UI-16 covered; Playwright flow on the fake backend; visual comparison with boards 17a–17j |
| **F19 Guided tour** (P-020) | Boards of screen 18 (and the callout component), then, **after the author's approval**: demo project built and synthesized by the core, ephemeral session, tour commands between UI and core with the generated types, "shown" mark in the app data, tour with spotlight and callout and the fifteen steps, single app menu with Help (UI-20) and link in the Welcome, fake backend, self-test, VALIDATION | PRJ-9, UI-17 → UI-20 covered; same hash at every build of the demo project and no file left after closing; no network call and no cost recorded; Playwright on the fake backend (first launch, all steps forward and back, skip, repeat, minimum window, axe); tour self-test in the real app; PRJ-9 timings and cold start measured; visual comparison with the boards; `make reqs` with no gaps |

F11 and F12 run after F10, in order, with the same rules of §9 (even where §9 and §11 speak of "F10" as the last
phase). The boards in `design/` are updated at the start of each phase, before the interface. F13 (P-013) and F15
(P-018) are the phases that **stop halfway**: once the boards are delivered, the implementation waits for the author
to approve them. The boards of F15 also cover F16 and F17; once approved, F15 → F17 run in sequence. F18 (P-019) and
F19 (P-020) also stop halfway for the approval of the boards.

### 8.2 Common gate: `make check` (in `app/`)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test --workspace` · license gate ·
TypeScript type check · lint · `vitest` · Playwright on the fake backend · `make reqs` for closed phases · space
cap check (NFR-9). No commit with the gate red. Benchmarks run with `make bench` at the gate of the phases that cite
them.

### 8.3 Requirements traceability

Every test cites the ids it verifies: in Rust with a documentation comment `/// req: ENG-4, AUT-4` above the test,
in TypeScript with `// req: UI-5` above the case. `make reqs` lists the ids of §3 without tests; every uncovered id
must appear in `docs/app/REQS-JUSTIFIED.md` with the reason (for example: verifiable only by ear).

### 8.4 Test rules

- No test uses the network: the OpenRouter client is tested against a fake local HTTP server.
- No audio file in the repository: synthetic signals with a fixed seed, test projects in temporary folders.
- No test opens the real audio device, except a smoke test marked `#[ignore]`, run by hand at the F3 gate.
- Tolerances always explicit and motivated.

### 8.5 What is left to the author after the implementation

The agent cannot listen or use a real key. `docs/app/VALIDATION.md` lists, with executable steps: listening to a
real song (clicks, glitches, quality of compressor, reverb, delay, saturation, limiter); auto mix and revision with
the real model and reading the costs; master against a reference; signing and notarization; a counter-test on a
song other than "Confidence" (PoC limitation, M7). With P-011 are added: listening to the three reverb algorithms
on voice and drums (metallic tails, audible modulation); A/B listening of a proposal with structure (dedicated
returns, group with parallel compression, open doubles) against the same without; tuning of the doubles thresholds
on a song with really doubled guitars. With P-018: listening to every new block on real material (de-esser and gate
above all, harmonizer, doubler and shimmer for pitch artifacts), and creative ideas proposed by the real model on
songs of different genres.

## 9. Rules for agents

They apply to the implementation session and are reproduced in `CLAUDE.md`.

**Autonomy**

1. After the preliminary check (§11), the agent **does not stop and does not ask questions** until F10 is complete.
2. An ambiguity is resolved by choosing the option most consistent with this document, in this order: principles
   (§1), requirements (§3), architecture (§5), design. The choice is recorded in `docs/app/DECISIONS.md` (id
   `A-nnn`, context, alternatives, choice, reason) and work continues.
3. A technical obstacle (a library that does not do what is needed, a test that cannot be made green) is tackled
   with at most three different approaches; then either the fallback that preserves the requirement is chosen, or
   the requirement is degraded to the useful minimum, recorded among the "Caveats" in `docs/app/PROGRESS.md`, and
   work continues. A gate is never bypassed by weakening a test.
4. The only reasons to stop: a destructive or irreversible action outside the repository, the need for a secret, or
   a conflict with principles P1–P6 that no alternative resolves.

**Boundaries**

5. Nothing out of scope (§2, right column).
6. The PoC is read-only: what lies under `poc/` is not modified. It is run only to generate the oracle.
7. New dependencies are allowed without asking, if the license is among those of NFR-6 and the library is
   maintained; each one is noted in `docs/app/DECISIONS.md`. No GPL/LGPL/AGPL dependency, not even a development one
   if it ends up in the package.
8. Secrets: the agent does not read `.env`, does not ask for or use keys, does not make calls to real models.
9. Git: work on the `product/v1` branch; small commits with the phase prefix (`F3: add automation ramps (AUT-4)`);
   one local tag per closed phase (`f3`). Never `push`, `merge` into `main`, `reset --hard`, `rebase`.
10. Language: everything in the repository is in English — code, comments, messages, interface, documents in `docs/`,
    `design/` and `poc/`, decisions, progress, commits, issues and pull requests (A-106). Only `rationale` and
    `summary` follow the language of the brief (AI-13).

**Coordination and subagents**

11. The main agent is the orchestrator: it owns architecture, contracts between modules, integration, gates and
    decisions. It organizes the work as it sees fit.
12. **Delegate to reduce costs, not on principle.** It pays to delegate to a subagent on a cheaper model
    (`app-implementer`) when the task is all of: bounded (one module or a few files), specified by an already
    written contract (signature, requirements, expected tests or fixtures) and verifiable by a command. Examples:
    the port of a measurement with its oracle fixture; a DSP processor with its properties; an interface component
    from its board; a series of tests from a table of cases.
13. **Do not delegate** when: the brief would cost as much as the work; architecture decisions or broad context are
    needed; the task touches files that others are modifying; module integration; fewer than a handful of tool
    calls. For broad searches in the code, use the cheaper read-only subagent.
14. Work in parallel only on disjoint sets of files; two agents never write the same files. Every brief names
    files, contract, verification command and what must not be touched.
15. A subagent's result is not proof: the orchestrator reruns the verification command and reads the diff before
    integrating. A subagent that fails the same task twice is not relaunched: the work returns to the orchestrator.
16. An **independent review** (`app-reviewer`, fresh context, read-only) is done at the close of F3, F5, F8 and F10,
    not at every phase: it serves quality, and its cost is justified only on what is risky (real time, AI
    contracts, master, delivery). Well-founded findings are fixed before closing the phase.
17. At every phase close the orchestrator updates `docs/app/PROGRESS.md` (status, gate, caveats) so that the work
    can be resumed from a new session without any other memory.

**Disk space**

18. **Prevention, set up in F0.** A single build folder for the whole workspace (`app/target`), used also by
    subagents and by the Tauri build: no alternative `CARGO_TARGET_DIR`, no copy of the repository or worktree with
    its own `target`. Development and test profiles with reduced debug information
    (`debug = "line-tables-only"`) and without incremental compilation for dependencies; a single `node_modules`
    (pnpm store outside the repository); tests write only to temporary folders that they delete at the end, even
    when they fail; no rendered audio file remains on disk after a test.
19. **Measurement.** `make disk` reports the usage of the repository and its main folders (`app/target` per
    profile, `node_modules`, produced packages, test reports, test temporary folders). It is run at every phase
    close, after every `make build` or `make bench`, and in any case at least every two hours of work; the value is
    noted in `docs/app/PROGRESS.md`.
20. **Cleanup.** Attention threshold **10 GB**: `make clean-stale` deletes what is no longer needed (incremental
    caches, artifacts of unused profiles, builds of superseded dependency versions, previous `.app`/`.dmg`
    packages, Playwright reports and traces, leftover temporary folders, worktrees and local branches of subagents
    already integrated). Cap **20 GB** (NFR-9): full `cargo clean` and rebuild of only the profile in use;
    `make check` fails above the cap. Cleanup touches **only git-ignored files inside the repository** and the
    temporary folders created by tests: never tracked files, never `local/`, never anything outside the repository.
    These deletions are pre-authorized and need no confirmation.

---

## 10. Design session prompt

To paste into a new Claude Desktop session (canvas), **attaching this file** (`FOUNDATION.md`).

````text
You are the product designer of Stemwise, a desktop DAW for mixing and mastering already-recorded songs with the
assistance of an AI model. The attached document (FOUNDATION.md) is the specification: read sections 1, 2, 3 and 6
in full before designing. Section 6 is the binding inventory of screens, states and components; section 7
describes what you must deliver.

The work has two phases and a single pause.

PHASE 1 — Four visual directions
Propose 4 visual and creative directions that are clearly different from one another (not four color variants of
the same idea). For each one show on the canvas:
- a name and three lines on the concept and on which user it makes feel at home;
- the complete "Mix workspace" screen at 1440×900 with realistic content (12 tracks with real roles, waveforms,
  song sections, one open automation lane, mixer, inspector with EQ, AI panel with a rationale);
- an excerpt of the "Master workspace" showing how the direction distinguishes the two phases;
- palette, typography and the key components (fader, knob, meter, clip, primary button).
Then stop and ask me which direction I choose. It is the only question you may ask me in the whole session.

PHASE 2 — Everything else, without interruptions
After my choice, design all the screens, states and components of section 6, in the order in which they are listed,
without stopping and without asking me anything else. If a point is ambiguous, decide yourself in the way most
consistent with the chosen direction and with the specification, and note the decision in the final index. Do not
consider the work finished until every item in section 6 has its board: at the end check the list item by item.

Product constraints
- Mix and Master are two distinct phases: they must be recognizable at a glance (accent color, layout, density),
  with a phase selector always visible. The passage from mix to master is an explicit moment.
- The AI is an assistant, not an autopilot: its proposals are read, listened to in A/B, accepted in whole or in
  part, and every choice has a visible rationale. No generic chatbot aesthetic: requests in words live inside the
  instrument.
- The person using the app is not a sound engineer: density must stay readable, advanced controls must not
  intimidate. But it must look like a serious audio instrument, not a toy.
- People work for hours, often in dimly lit environments: contrast must be restful, meters and playback state
  readable out of the corner of the eye. Text and states with at least AA contrast.
- Every control has its states: normal, hover, active, keyboard focus, disabled and, where it makes sense,
  automated and locked by the user (lock for the AI).
- Track colors are a palette of at least 12 distinguishable hues that works on clips, strips and headers.
- Interface in English. No imitation of the look of existing DAWs: the identity must be its own.

Implementation constraints (the interface will be React + CSS inside a webview; timeline, waveforms, meters and
curves will be drawn on canvas)
- Minimum window 1280×720, reference 1440×900, fluid up to 4K: state what resizes, what collapses and in which
  order.
- Only fonts whose license allows inclusion in a distributed app (OFL or equivalent), at most two families, with
  tabular figures for numeric values. Icons from a set with an MIT/ISC license or drawn by you as SVG.
- Effects achievable in CSS without continuous rendering costs: no animated blurs or heavy shadows on elements that
  move at 60 fps.
- All measurements derive from tokens: no isolated values in the boards.

Delivery (section 7 of the document)
- tokens.css: all tokens as CSS variables (colors, typography, spacing, radii, shadows, durations and animation
  curves, track palette, accents of the two phases).
- One board for every screen and state of section 6, in self-contained HTML/CSS that uses those tokens.
- The component sheet with all states.
- README.md: chosen direction, principles, index of the boards with the name of every file, behaviors that cannot be
  deduced from the boards (resize, animations, drags, density), decisions taken autonomously.
Organize the files as design/README.md, design/tokens.css, design/screens/, design/components/. If you have access
to the project folder, write them there; otherwise make them downloadable with those names.
````

## 11. Implementation session prompt

To paste into a new Claude Code session opened on this repository, **after** `design/` has been populated.
`.claude/settings.json` already authorizes the development commands (`make`, `cargo`, `pnpm`, `node`, `uv run`, the
working git commands); for cases not covered it is advisable to start it with a permissions mode that does not ask
for confirmation on every command.

````text
You must implement Stemwise from start to finish, autonomously, following docs/FOUNDATION.md, which is the source of
truth of the product. Read it in full before anything else, then read CLAUDE.md, docs/app/PROGRESS.md and
docs/app/DECISIONS.md, and design/README.md with design/tokens.css.

PRELIMINARY CHECK — the only moment in which you may ask me questions
Before writing code verify and report to me in a single message:
1. that design/README.md and design/tokens.css exist and that every screen of section 6 of FOUNDATION.md has its
   board (list the missing ones);
2. that the necessary tools are installed (rustc, cargo, node, pnpm, Xcode command line tools, uv for the PoC
   oracle) and what, if anything, is missing;
3. the contradictions or gaps you found between FOUNDATION.md and the design that you cannot resolve with the rules
   of section 9: for each one propose your choice.
If nothing blocks, say so and start right away without waiting for an answer. If something really blocks (design
missing, a missing tool you cannot install), stop here and ask: it is the only pause allowed.

EXECUTION — without interruptions until the end of F10
- Work on the product/v1 branch. Execute phases F0 → F10 of section 8.1 in order, one after the other, without
  asking me for confirmations, approvals or clarifications between one phase and the next. A phase is closed only
  when its gate is green.
- Apply the rules of section 9. In particular: you resolve ambiguities yourself and record them in
  docs/app/DECISIONS.md; you tackle obstacles with alternative approaches and, if needed, with a fallback recorded
  among the caveats; you never weaken a test to pass a gate.
- Coordinate the work as you see fit. Delegate to subagents only when it reduces costs (rules 11–17): bounded
  tasks, with contract and verification already written, to the app-implementer subagent; broad searches to the
  read-only subagent; independent review with app-reviewer at the close of F3, F5, F8 and F10. Always verify a
  subagent's result yourself before integrating it.
- At every closed phase: update docs/app/PROGRESS.md, commit, put the local tag of the phase, and proceed with the
  next one in the same turn. State must live in the files, not in the conversation: if the context is compacted,
  reread FOUNDATION.md, PROGRESS.md and DECISIONS.md and resume from the first unfinished step.
- Keep disk space under control (rules 18–20): a single build folder, make disk at every phase and after every
  build or benchmark, make clean-stale above 10 GB, never beyond 20 GB.
- Do not use the network except to download dependencies. Do not read .env, do not use keys, do not call real
  models: the AI is developed and tested with the fake decider and with a fake local HTTP server.
- Do not push, do not merge into main, do not modify the PoC.

VERIFICATION OF WHAT YOU BUILD
- Engine, DSP, analysis and AI: automated tests on synthetic signals, parity with the PoC oracle, full flows from
  stemwise-cli.
- Interface: Playwright tests on the fake backend and visual comparison of every screen with its board in
  design/screens (open the UI in the browser, look at the result, fix the differences). Also start the real Tauri
  app and verify that the main flows work with the real engine.
- Do not declare finished something you have not executed. What you cannot verify (listening, the real model,
  signing) must be listed in docs/app/VALIDATION.md with the steps I will have to perform.

END
You are done when F10 is closed: make check green, make reqs with no unjustified gaps, .app/.dmg package produced,
final review done and findings fixed, PROGRESS.md and VALIDATION.md complete. Only then write me a final report:
what works and how you verified it, the caveats, the most relevant decisions taken autonomously, what I must
validate and in which order.
````

## 12. Risks

| Risk | Effect | Countermeasure |
| --- | --- | --- |
| Own DSP of lower quality than the PoC (compressor, reverb, limiter) | Less polished mix and master | Properties verified by tests, known algorithms documented in `docs/app/dsp.md`, the author's final listening as the first item of `VALIDATION.md` |
| Very long autonomous session: drift or loss of context | Inconsistent or repeated work | State in files (rule 17), a gate per phase, a tag per phase, independent review at the risky points |
| Audio glitches not detectable by tests | Clicks or dropouts when listening | Same code for playback and render, allocation test, benchmarks, vertical slice already in F3 |
| Fidelity to the design not verifiable to the pixel | Approximate interface | Shared tokens, fake backend in the browser, visual comparison screen by screen |
| The prompt with sections and families exceeds the token budget or confuses the model | Cost or worse plans | ANA-13 ceiling verified by a test; measurement of real costs in the author's validation |
| The PoC was validated on a single song | Generality not demonstrated | Counter-test on a different song in the author's validation (§8.5) |
| Build and test artifacts make the repository grow without limit | Disk full, session blocked | Rules 18–20, `make disk` and `make clean-stale` from F0, cap verified by `make check` |
| Structured routing and extended automation widen the engine | F3 longer and more fragile | Acyclic graph computed off the audio thread, property tests on routing, independent review at F3 |
| WebView performance with many tracks | Stutter during playback | Canvas, peak pyramids, binary channel; NFR-3 measured in F9, WebGL as a fallback |
| The new reverb meets the properties but does not convince by ear | More returns without a real gain | Published and known algorithms (Dattorro, FDN), FX-9 properties, the author's listening before closing the judgment on F12 |
| The model abuses structure (useless returns and groups) or the loop cannot judge it: spectral distance does not see it | More complicated, not better, mixes | Caps per proposal, structure decided only once (AI-19), useless elements discarded by validation, AI-18 switch, rows of their own in the proposal with A/B |
| False doubles: two different parts with the same role mistaken for a pair | Wrong pans | Several measurements together in ANA-18, cautious thresholds tuned by the author; it remains a proposal, visible and discardable per track |
