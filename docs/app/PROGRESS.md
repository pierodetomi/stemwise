# Product progress

Specification: [FOUNDATION.md](../FOUNDATION.md). Decisions: [DECISIONS.md](DECISIONS.md).
Legend: ⬜ not started · 🟦 in progress · ✅ closed (gate green) · ⚠️ closed with caveats

| Step | Status | Notes |
| --- | --- | --- |
| Foundation document and rules for agents | ✅ | 2026-11-10 |
| Design session (FOUNDATION §10) | ✅ | 2026-11-10 · direction "E · Atelier" · delivered in `design/` (see `design/README.md`) |
| F0 Foundations | ✅ | 2026-11-11 · Cargo workspace, Tauri app that opens (`make dev`), UI on the fake backend, tokens and components imported from `design/`, license gate, `make reqs`/`oracle`/`disk`/`clean-stale` · disk 5.7 GB (of which `local/` 4.0 GB) |
| F1 Core | ✅ | 2026-11-11 · document, commands with inverse, mix/master histories, gestures, acyclic routing, editing of clips/sections/tracks, automation, package with autosave and recovery, WAV/RF64/AIFF import with conversion, pyramids, snapshots, generated schemas and TS types · 40 tests (property tests on commands and routing) · disk 7.1 GB |
| F2 DSP | ✅ | 2026-11-11 · biquad, pan, compressor with sidechain, 4x saturation, reverb, delay, true peak limiter, dither, match EQ, width, meters, loudness (offline and real time), true peak · parity with the oracle (biquad, pan, LUFS, LRA, true peak) · no allocation in `process` · `docs/app/dsp.md` · disk 7.4 GB |
| F3 Engine | ✅ | 2026-11-11 · level-based graph with nested groups, returns, compensated sidechain, automation on a grid of 32, deterministic offline render (same hash with blocks 64/1000/4096), streaming with a lock-free seek protocol, cpal output with resampling, auxiliary threads (A-024), vertical slice proven in the real app · NFR-1 p99 14%, NFR-4 36× · independent review: 2 blockers and 9 findings fixed · disk ≈ 9 GB |
| F4 Analysis | ✅ | 2026-11-11 · ANA-1…12 measurements ported from the PoC with parity to the oracle (stem, mix, masking, shares, references, targets, librosa onsets), per-section measurements, families, `analysis` v2 document with neutral ids and under the token budget, cache tied to the clips · ANA-17: 16 stems of 4 min in 8.6 s |
| F5 AI | ⚠️ | 2026-11-11 · `mix_plan` v2 and `master_plan` v1 contracts with per-project schema and all limits, fake decider, prompts, OpenRouter client tested against a fake server (JSON mode fallback, retries with the wrong response, cancellation), verification loop with stop rules and best plan, revision from the current state (per-section lanes and manual values kept), lock, scope, log and costs also of failed or cancelled calls, registry across projects, master AI, CLI · independent review: 3 blockers and 14 findings, fixed (A-031, A-034…A-037) · caveats: strict schema and costs to verify with the real provider, master AI measurement in F8 (A-033) |
| F6 Mix UI | ✅ | 2026-11-11 · screens 1, 2, 3, 6, 7: welcome, new project with roles and sample-rate choice, workspace with timeline (ruler, loop, sections, clips with waveforms, drag with snap, trim, fade, cut, box selection, context menus, collapsible nested groups, automation lanes with pen, ranges, copy and paste), mixer, inspector (chain with bypass, EQ editor with live spectrum, compressor with curve and sidechain, saturation, sends, output), returns, history · `Edit` contract in the core (A-038), spectrum from the engine (A-039) · 11 Playwright tests on the flows, 8 captures for the visual comparison, self-test in the real app with the real engine |
| F7 AI UI | ✅ | 2026-11-11 · screens 4, 5, 12 (and 3c): request with scope and cost estimate, spending cap with confirmation, progress per phase and pass with cancellation, proposal under review with A/B, apply all or the chosen tracks or discard, AI usage with summary per type and model, filters, readable detail and raw JSON, CSV/JSON export, total across all projects, snapshots with A/B, brief and references, key in the keychain or offline demo · calls in a thread of the Tauri session (A-041) · 7 Playwright tests on the AI, full flow in the real app with the fake decider (self-test) |
| F8 Master | ✅ | 2026-11-11 · mixdown with trim at −3 dBTP and mix hash, master graph with MAS-1 chain on the bus and live parameters, target loudness computed in the background, loudness-matched A/B, meters and spectrum with the references profile, master AI measured before application and applicable per module, WAV/FLAC/AAC export with own FLAC encoder, system AAC and tags (`stemwise-export`), ceiling guaranteed on the decoded file · screens 8–10 · self-test in the real app (also A/B on a 44.1 kHz AAC reference) · independent review: 3 blockers and 9 findings fixed (A-043: chain without steps, ceiling on re-read files, AAC ≤ 48 kHz, converted references, A/B without clips, mixdown outside undo, jobs tied to the project, ISRC and limits in the core) |
| F9 Polish | ✅ | 2026-11-11 · single shortcut table (UI-3), Settings (11: Audio, AI, Appearance, Shortcuts, SET-1), error and empty states (13a–13f: missing files with relinking, disconnected device, no network, empty project, waveforms rebuilt on opening, UI-6), accessibility with axe-core and keyboard (UI-7), 60 fps with 32 tracks, import 16×4 min in 1.7 s, cold start 0.5 s in development (NFR-3), memory peak 1.0 GB with 32×5 min after streaming analysis (NFR-5) · A-044, A-045 · 43 Playwright tests, self-test in the real app · disk 11.3 GB after `clean-stale` (of which `local/` 4.0 GB) |
| F10 Delivery | ✅ | 2026-11-11 · `make build` produces `Stemwise.app` and `Stemwise_0.1.0_aarch64.dmg` (A-046), full self-test in the packaged app with a cold start of 969 ms, NFR-7 and NFR-8 verified by tests on the code, product README, full VALIDATION with recommended order · final independent review: 2 blockers and 6 findings fixed (A-047) · `make reqs`: no gaps · disk 13.7 GB after `clean-stale` (of which `local/` 4.0 GB, `target/debug` 7.7 GB) |
| F11 Reverb and parallel compression | ✅ | 2026-11-11 · P-011 · reverb in three algorithms (Dattorro plate, room and hall on a network of 16 delays) in place of the comb one, with decay in seconds, size, damping, deterministic modulation and low-pass; mix (dry/wet) of the track and group compressor; project schema 2 with migration; `mix_plan` v3 contract for reverb and mix; boards 7a and 7e and their implementation · FX-9: decay within 5%, tail without a repeated period (0.04–0.10 against 0.14–0.20 of the old one), same hash at every block · NFR-1 p99 17.9% with 4 more reverbs, NFR-4 31× · A-058 · disk 16.7 GB after `clean-stale` (of which `local/` 4.0 GB, `target/debug` 9.6 GB) |
| F12 Structural AI mix | ✅ | 2026-11-11 · P-011 · the plan creates returns and groups (`structure`, placeholders `n1`–`n6`), actually rendered once in the working copy and stable across iterations, recreated on application with one checkbox each; real doubles measured by pairs of the same role; techniques in the prompt and in the fake decider (doubles to the sides, plate and slapback for the voice, delay throws, drum bus in parallel); "Allow structural changes" checkbox; cards for the new elements in the proposal (boards 3a, 4a, 4b) · 5 flow tests, full flow from `stemwise-cli`, 60 Playwright tests, self-test in the packaged app (46 steps, cold start 1.4 s) · independent review: 2 blockers and 4 findings fixed (A-060) · `make reqs`: no gaps · A-059 · disk 17.48 GB after `clean-stale` |
| F13 Mix state at a glance | ✅ | 2026-11-13 · P-013 · design approved by the author (A-066), then implemented: badges `HP LP EQ CMP SAT` in three states on headers and strips (`CMP` with the sidechain color and the parallel bars), send dots and automation mark, label column with one send per return and bus pinned to the right, list of who sends in the return lane, selection connections, Overview view with one-click bypass; tokens made effective (header 344 px, strip 40, mixer 296) · UI-8 → UI-11 covered (8 Vitest tests, 6 Playwright, captures 3e, 3f, 3g, 7f–7h) · 60 fps with 32 tracks even with the mixer open with 4 returns (NFR-3) · A-067 · disk 18.07 GB |
| F14 Free master chain | ✅ | 2026-11-14 · P-017 · boards 15a–15j approved by the author, then implemented: chain as a list of repeatable blocks in free order (fixed input, limiter and dither), clipper, multiband from two to six bands, mix and filter on the compressor detector, per-section offsets; project schema 3 with a migration that preserves the sound; processor with constant latency and structure changes without glitches; `master_plan` v2 with per-block evaluation, proposal applicable item by item; rack next to the editor, warning on a target that is too high; roles `piano`, `woodwinds`, `choir` · MAS-11 → MAS-15 covered (engine, core, AI, 22 Playwright tests of the Master, 86 in all) · not run: self-test in the packaged app and `make build` · A-082 · disk 11.3 GB after a `cargo clean` |
| F15 Block-based track chain | ✅ | 2026-11-14 · P-018 · boards 16a–16u, 4c, 4d approved by the author (A-086), then implemented: track and group chain as a list of blocks in free order (at most 12, two per type), return chain (at most 6) with sidechain for ducking, resonant filters, bypass with a 10 ms fade, constant latency per class, mono that becomes stereo at the first stereo block; project schema 4 with migration at an **identical hash** (proven on projects written by the schema 3 build) and mixdown that stays up to date; rack in the inspector with menu per family, reorder, duplicate, reset; marks (four and `+n`) and Overview with one row per type · fixed the glitch after a graph swap (the fade now starts after the latency) · FX-10 → FX-12, UI-12 covered · A-087 · disk 11.9 GB |
| F16 New effects | ✅ | 2026-11-14 · P-018 · gate and expander, de-esser with listening to the detected signal alone (engine-only state), dynamic EQ with per-band sidechain, transient shaper, track clipper and multiband, saturation characters (`soft` unchanged), exciter, bass enhancer, width, modulation (chorus, flanger, phaser) and tremolo/auto-pan with phase tied to the position in the song, harmonizer (compensated constant latency) and doubler, `shimmer` reverb, ping-pong and tape delay; one editor per block (boards 16d–16s), automation of every parameter · FX-13 → FX-24 covered · FX-24 benchmark: p99 27% (threshold 50%), NFR-1 p99 12.8%, NFR-4 30× · schema 3 hash unchanged · processors written by six subagents and integrated (A-088) · disk 10.0 GB after `clean-stale` |
| F17 Creative ideas | ✅ | 2026-11-14 · P-018 · ANA-19 measurements (sibilance, crest, transients, noise floor) and `analysis` v4 with the non-classic blocks, `mix_plan` v4 with blocks per track, group and return, resonances, characters, filter offsets in octaves and blocks on per section; up to three ideas decided at the first iteration, kept out of the evaluation and given as context to the following ones, applied only if checked in the same step; fake decider and prompt with the new techniques; Creative ideas subsection with checkbox and listening to the idea alone (boards 4c, 4d), "new block" rows in the diff, "Allow creative ideas" switch, idea name in the track note; `stemwise-cli automix --ideas` · ANA-19, AI-25 → AI-27, UI-13 covered (`ideas_flow`, client against the fake server with the v4 schema, flow from the CLI, Playwright with captures 4c and 4d) · package rebuilt, self-test in the packaged app (49 steps, ideas listened to and applied, cold start 1.2 s) · independent review: no blockers, 7 defects fixed · A-089 · disk 12.75 GB after `clean-stale` |
| F18 Explain this mix | ⚠️ | 2026-11-15 · P-019 · boards 17a–17j approved by the author (A-092), then implemented: `Explain` view in the Mix (⌘E) with the signal map (tracks by family → groups → buses, sends, sidechain, collapsible groups, section strip) and the same mix in words, for the whole mix, per element and per section; rule-based text, glossary, copy as text; **no model**; levels per family and per section measured by the core in the background (`mix_levels`) · ANA-20, UI-14 → UI-16 covered (1 Rust test, 15 Vitest, 6 Playwright; 103 in all) · caveats: not tried in the Tauri app nor in the package · A-093 · disk 12.28 GB |
| F19 Guided tour | ✅ | 2026-11-15 · P-020, issue #5 · boards 18a–18w and C8 approved by the author (A-095), a single app menu with Help (UI-20, A-096), then implemented: demo song built and synthesized by the core (`stemwise_core::demo`, 12 stems, 68 s, three sections, same document and same hash at every build), ephemeral session in the app cache (no recents, autosave, save, export; deleted at the end and at startup), fake decider proposal in the background even with a key and outside the cost registry, "shown" mark in the app data, tour with spotlight and callout and the fifteen steps, link in the Welcome, reduced system menu and ⌘N/⌘O in the app · PRJ-9, UI-17 → UI-20 covered (7 Rust tests, 6 Vitest, 9 Playwright on the tour; 113 Playwright in all) · timings: song open and playable in 1.1 s (threshold 3 s), fake decider proposal 36 s in the background, mixdown and master 5 s, cold start 0.6–1.6 s · package rebuilt, self-test in the packaged app 59/59 · fixed three store updates during rendering (track and Master rack, new project references, A-098) · A-097 · disk 10.5 GB after `clean-stale` |

Phases F0 → F19 closed. Repository public since 2026-11-16 (A-101). Windows is the second system (A-104, branch
`feature/multi-platform-build`). Next step: the author's validation.

## Where to resume

- Last step concluded: **F19** (tag `f19`): guided tour (P-020) and a single app menu (UI-20). To be tried in the
  real app on clean data: VALIDATION 57–61. Before that, F18 (tag `f18`): Explain this mix (P-019), to be tried on a
  real song (VALIDATION 55–56). Before that, F17 (tag `f17`): P-018, to be tried by ear and with the real model
  (VALIDATION 47–54).
- After F10, from the author's validation: new project with the files chosen before it exists (A-048); eight defects
  from the manual test fixed (A-049); master ready in 0.4 s on a reopened project and in 8 s after a bounce, with 13
  real tracks at 96 kHz, instead of 40 s and 29 s (A-050); master preparation overlay designed (screens 9d–9k) and
  implemented (A-051) · disk 15.0 GB after `clean-stale`.
- Then: keychain read once (A-052), save from the interface (A-053), playhead scrub (A-054), loudness search that
  reaches loud targets (A-055), listening at real level with loudness matching on request and a warning on a target
  far from the references, designed (boards 9l–9u, 10d–10e) and implemented (A-056).
- After F12, from the author's test: a new target in Master waits under an overlay until the level is in place,
  instead of the `Measuring…` label alone (board 9v, A-061).
- From the author's test on 30 stems: a response truncated by the provider is retried in JSON mode, the request
  carries an explicit output ceiling and a failed call saves the complete request and response (A-062). The cause of
  the observed failure is still to be established with the real provider.
- From the analysis of the mixes proposed on Balance (P-012, A-063): the model receives the levels per family and per
  return (AI-22), the loop does at least two iterations and keeps the last valid plan, the prompt gives balance
  ratios and audibility thresholds of the effects, a revision has no step limit, sections over 120 s have their real
  timings, request and auto mix start from the same button.
- Glitches in playback on a large project: waveforms are no longer recomputed at every playhead update (A-064); the
  timeline is no longer redrawn while the playhead advances and sends have a section of their own in the inspector
  (A-065).
- From the author's test (P-013, A-066): the state of the tracks could only be read in the inspector. Designed the
  marks on headers and strips, the named sends, the selection connections and the mixer Overview view. The boards to
  look at: `design/screens/03-mix-workspace.html` (3a, 3e, 3f, 3g), `design/screens/07-returns-routing.html` (7f,
  7g, 7h), `design/components/index.html` (C7). Approved on 2026-11-13 and implemented in F13 (A-067); to be tried on
  a real project (VALIDATION 33–34).
- Repository structure (issue #3, A-068): the PoC is confined in `poc/` with its own code, documents (`poc/docs/`)
  and Python environment (`poc/.venv`); at the root remain `app/`, `design/`, `docs/` (product only) and a README
  that serves as an index. `make oracle` regenerates identical fixtures. The product site (issue #4) will have its
  own `web/`.
- Request button outside the panel (issue #6): with a selected track with a long name the scope selector pushed
  `Revise mix` beyond the assistant's edge. Now the name is truncated with an ellipsis (in full in the tooltip) and
  the button does not move; Playwright regression test.
- On-demand panels (issue #7, A-069): three switches in the Mix top bar (mixer, inspector, assistant), independent,
  on the same state as ⌘M, ⌘I, ⌘J and the reopening strips. They are not in the design boards.
- Persistent proposals (issue #2, P-014, AI-23, A-070): every proposal received is kept in the project; the
  `Proposals` tab of the assistant, in Mix and in Master, lists them and puts them back under review on the current
  state, with confirmation before applying. Proposals received before this change stay in the list but cannot be
  reopened. To be tried in the real app (VALIDATION 35).
- Guided assistant setup (P-015, AI-24, A-071): a three-step dialog connects the OpenRouter account from the browser
  (OAuth with PKCE, return to `localhost`), guides to the credit and keeps the pasted key as a secondary route;
  exhausted credit, key limit and key no longer valid have their own state. The first launch (1b) now exists. Tested
  against the fake server; with the real provider in VALIDATION 36–37.
- Where the key lives (A-072, author's choice): in the keychain only if the build is signed with an Apple
  certificate, otherwise in a private app file. No password prompt in any case. With an unsigned build the assistant
  must be reconnected once (VALIDATION 38). · disk 18.93 GB after `clean-stale` (of which `target/debug` 11.9 GB,
  `target/release` 2.3 GB, `local/` 4.0 GB): close to the cap, the next step starts with a `cargo clean`.
- Wider mixer strips (A-073, author's choice among three captures): 64 px, larger fader and badges in the strips
  only, mixer 304 px high.
- Mixer label column equally wide in both views (A-074): strips no longer shift when switching from Faders to
  Overview and return names are not cut off.
- Logo (A-075, A-076): four directions designed by an external agent (`design/brand/directions/`), chosen **D · Two
  rooms**, refined on 2026-11-14 while staying faithful to the proposal: the two trajectories are the Mix, the arrow
  becomes the full room of the Master. Delivered in `design/brand/final/` (`presentation.html`). Adopted in the app
  (A-078): icon, mark in the bars with the room of the lit phase, start screen, connection page, boards; to be seen in
  the package (VALIDATION 40). Index in `design/brand/README.md`.
- Measured AI master (issue #1, P-016, A-077): the AI master decided blindly in a single call, on measurements that
  did not say how far the peaks stand above the body of the mix, and on loud targets it left everything to the
  limiter. Now it receives peak-to-loudness distance, the reduction required by the target and the measured current
  master; every plan is rendered and measured (limiter and compressor work, lost punch) and the next decision
  corrects it, up to three plans. Prompt with the technique; the target changes only if asked in words. On the real
  Balance mixdown at −9.6 LUFS the limiter alone works over 3 dB for 21% of the time, with the existing stages 5–10%.
  To be tried with the real model (VALIDATION 39). **Open**: clipper, parallel compressor and detector filter, chain
  order, per-section master, evaluation shown in the proposal: to be designed (they touch MAS-1 and the panel). ·
  disk 19.46 GB after `clean-stale`: the next step starts with a `cargo clean`.
- Free master chain (issue #1, P-017, A-079, A-081, A-082): designed, approved and **implemented in F14**. To be
  tried by ear and with the real model (VALIDATION 41–45).
- No genre as the norm (A-080): the fixed prompts no longer presuppose a genre. The master talks about measurements
  and not instruments (a test forbids it); the mix balance starts from the type of music and keeps the ratios of
  A-063 for voice-led music. To be tried with the real model on non-rock songs (VALIDATION 41). Decided with the
  author: the master chain is a list of repeatable blocks (two per type, twelve in all). The author chose the **rack**
  among three layouts (A-081).
- Master waveform (A-083): on a loud master it was a solid rectangle. Now it is drawn from the measured master, with
  the body in solid color, the peaks as a thin line and the mixdown dark inside it (VALIDATION 46). Issue #1 closed.
- Resizable track headers (issue #9, A-084): a handle on the right edge of the column widens it from 344 px (304 in
  compact, which remain the minimum) up to 640, always leaving 320 px to the lanes; the width is a user preference,
  double-click resets it to the minimum.
- Mix effects and creative ideas (P-018, A-085): chosen by the author after an analysis of the possible effects.
  Specification in FOUNDATION (FX-10 → FX-24, ANA-19, AI-25 → AI-27, UI-12, UI-13, `mix_plan` v4, phases F15 → F17).
  Next step: the boards, delivered on 2026-11-14 (A-086): `design/screens/16-track-chain.html` (16a–16u) and
  `design/screens/04-mix-proposal.html` (4c, 4d). **Approved by the author on 2026-11-14**; F15 → F17 in sequence.
- Verification of P-018 with phases closed (A-090): three blockers and eleven defects that the tests did not see,
  each fixed with its own test. Render identical at every block after a bypass turned off and on; modulation and
  tremolo phase tied to the position even with the rate automated; an idea's return stays still in the loop; chain
  limits verified at load and in every command; revisions that do not fail on manually set values; proposals reopened
  without duplicate blocks; exciter without a hole at the cutoff frequency, de-esser that does not trigger on vowels.
  After the fixes: `make bench` (NFR-1 p99 12.1%, FX-24 p99 26.9% on a threshold of 50%, NFR-4 31×, ANA-17 12.4 s,
  NFR-5 peak 1.20 GB), package rebuilt and self-test in the packaged app (49 steps out of 49, cold start 1.4 s) ·
  disk 6.8 GB after a `cargo clean` of the development profile and `clean-stale`.
- From the author's test on 33 stems (A-091): auto mix with ideas failed on every response of a real model, because
  the parser rejected the `null`s that the strict schema requires for unused ideas and their sends. Now they are read
  as absent; the three recorded responses, replayed in the loop, become a proposal.
- Explain this mix (P-019, A-092, requested by the author on 2026-11-15): a view that explains the mix without a
  model, with the signal map (tracks → groups → buses, sends, sidechain) and the same mix in words, per element and
  per section. Boards `design/screens/17-mix-explain.html` (17a–17j) approved the same day and **implemented in
  F18** (A-093). The narration with a local model is postponed.
- Guided tour (issue #5, P-020, A-094): specified with the author on 2026-11-15, **not yet started**. Synthesized
  audio, full run in fifteen steps, guided with Next, boards to be approved before implementation. F19 starts from the
  boards of screen 18 (`design/screens/18-tour.html`) and stops for approval. **Boards delivered on 2026-11-15**
  (A-095): 18a–18o the fifteen steps with the final texts, 18p–18q steps that wait for proposal and mixdown, 18r
  minimum window, 18s–18u start from the Welcome, from the app menu and with a project open, 18v notification after
  `Skip tour`, 18w song that cannot be prepared; C8 spotlight and callout; notes for implementers in
  `design/README.md` ("Guided tour"). Approved and **implemented in F19** (A-097, tag `f19`); to be tried in the real
  app on clean data, with the texts and the sound of the synthetic song (VALIDATION 57–61).
- Verification of F19 with the phase closed (A-099, requested by the author): five tour defects that the tests did
  not see, each fixed with its own test: step 8 stuck after `Apply all` or `Discard`, `Cancel` that cancelled the
  tour's proposal, Enter on `Back` that went forward, confirmation of `Discard` under the callout, shortcuts active
  from a text field. `make check` green (116 Playwright); package rebuilt, self-test in the packaged app 59/59 (cold
  start 1.4 s) · disk 10.57 GB after `clean-stale`.
- Landing page (issue #4, A-100): four directions on canvas, chosen **A · One song, start to finish**; hero film of 65
  seconds in twelve scenes, with the author's track. By the author's choice the site **does not live in this
  repository**: it lives in the private repository `stemwise-promo` (`../stemwise-promo`), with nginx image and
  Traefik configuration for the VPS. Here remains the MIT `LICENSE` at the root. To do after opening the repository:
  VALIDATION 62.
- Opening the repository (A-101, 2026-11-16): plan in seven steps and four choices by the author (PoC in `poc/` under
  GPL, commit email unchanged, project documents in Italian with README in English, first release unsigned). **Steps
  1–5 done**: `main` brought to `product/v1`; MIT metadata, `poc/LICENSE` GPL, third-party license list in the
  package (`make third-party`); name and logo reserved (`TRADEMARKS.md`); history searched for secrets and personal
  data, `CLAUDE.md` updated (step 3); public README in English with five screenshots (step 4); `CONTRIBUTING`,
  `SECURITY`, templates, CI with `make ci` (not yet run), issues translated (step 5). Step 6: package 0.1.0 built
  from `d783166`, licenses inside, self-test in the package 59/59 (cold start 1.4 s); release `v0.1.0` published on
  GitHub. Step 7 prepared (description, topics, site <https://stemwise.pierodetomi.dev>, preview image): **only making
  the repository public is missing**, and right after that protection of `main`, vulnerability reporting, secret
  scanning, first CI run · disk 10.89 GB.
- Two Playwright tests flaky under load, which emerged when rerunning `make check` (three failures out of five runs,
  A-102): in the tour `Next` seemed ready for one frame before the step started preparing, and the key pressed at
  that instant was lost (fixed in the code); the Master overlay test looked at intervals for a state that lasts 220
  ms (now the page catches the instant). After that: the two files three times in a row, 96 out of 96.
- **Repository public since 2026-11-16** (A-101): security and protection of `main` turned on, site and links
  verified from outside. To do: watch the first CI run, upload the preview image (author).
- **0.1.0 withdrawn, 0.1.1 ready** (A-103): the downloaded 0.1.0 appeared as "damaged" because the package did not
  have a valid signature. Now ad hoc signing of the bundle, verified by `make build`. Also fixed the order of the
  license list (it failed on CI) and the true cause of the flaky tour test (the Master took focus between two steps).
  `make check` green · disk 12.5 GB.
- 0.1.1 published and verified by downloading it from outside, 0.1.0 deleted, landing updated (A-103). CI: second run
  still stuck on the license list, for two packages that pnpm listed only locally; the list is now derived from
  `node_modules` without pnpm (355 libraries).
- **CI green** on `8738e13` (A-103): the whole gate on the GitHub runner, 114 Playwright tests. The macOS runner
  queues may cause a run to be cancelled: in that case rerun.
- **Windows (A-104, 2026-11-16, requested by the author).** Gate, package and real app on Windows 11 x64:
  `make check` green (Playwright on Edge, the WebView2 engine), NSIS installer from `make build`, self-test in the release
  executable 59 out of 59 with WASAPI audio (cold start 0.8 s). Defects found and fixed: atomic save that failed on
  Windows, Save As with `\` paths, snap that read only the ⌘ key, CLI data folder. AAC with Media Foundation, menu
  bar on macOS only, shortcuts and texts per system, license list for three targets (497 libraries). CI also on
  `windows-latest` (job `make ci (Windows)`, never run yet: it starts at the first PR). To be tried by hand:
  VALIDATION 64–66. Linux and Authenticode signing remain · disk 7.85 GB.
- **English everywhere (A-105, 2026-11-17, requested by the author).** All the documentation (FOUNDATION, DECISIONS,
  PROGRESS, VALIDATION, dsp.md, design, brand, PoC) translated from Italian; the language rule in CLAUDE.md and
  FOUNDATION §9 now asks for English in every file, commit, issue and pull request. Branch `feature/english-docs`.
- Next step: in parallel the author's validation (docs/app/VALIDATION.md, in the recommended order), at least one auto
  mix with the real model and the package on a clean Mac before opening.
- State verification: `cd app && make check` (green), `make reqs` for the remaining gaps, `git tag` for the phases.

## Caveats

_Requirements reduced or not verified, with reason and reference to the decision (FOUNDATION §9, rule 3)._

- **AI with the real provider (F5)** — per-project strict schema, reported costs, keychain: verifiable only with the
  author's key (VALIDATION 7–11). Lacking strict, the app falls back on its own to JSON mode (A-031).
- ~~Master AI measurement (MAS-6)~~ — closed in F8: the master proposal is measured before application (A-033,
  A-042).
- **Measured AI master with the real model (MAS-6)** — the loop is tested with the fake decider and measurements on
  the real mixdown; whether a real model uses the stages before the limiter and holds the target is told only by the
  author's test (A-077, VALIDATION 39). The target in a revision is held by the prompt, not by the code.
- **Free master chain by ear and with the real model (F14)** — clipper, multiband, sections and order changes are
  measured on synthetic signals: the sound, and the use a real model makes of them, are judged by the author (A-082,
  VALIDATION 41–45). The strict schema with `anyOf` on the block types is tested only with the fake server.
- **Where the peaks come from (15g)** — the warning on a target that is too high does not name the track family: the
  analysis of stems against the mixdown is missing. The lower loudness it proposes is an estimate (A-082).
- **Families for orchestral music (AI-22)** — the new roles (`piano`, `woodwinds`, `choir`) sit in the existing
  families; balancing by orchestra sections is not there (A-080, A-082).
- **Unsigned package (F10)** — the agent does not have a Developer ID: signing and notarization remain with the
  author; the first launch asks for right-click › Open (A-046, VALIDATION 23). The `.dmg` has no custom window layout
  (A-046).
- **Plate under 1 s (FX-9)** — the plate is grainy by construction with decays under about 1 s: density is verified
  from 2 s upward; panel and prompt direct to `room` for short spaces (A-058, VALIDATION 25).
- **Structural AI mix by ear and with the real model (F12)** — whether the structure really improves the mix, and how
  much of it a real model proposes, is judged by the author (VALIDATION 27, 29). The strict schema with nullable
  placeholders (`anyOf`) is tested only against the fake server.
- **Doubles thresholds (ANA-18)** — initial, tested on synthetic signals: the margin between a copy (0.13 dB per
  frame) and two synthetic performances (0.8 dB) is narrow around the threshold of 0.5 dB; to be tuned on real
  material (A-060, VALIDATION 28).
- **Reverbs and parallel compression by ear (F11)** — the properties are measured, the sound is judged by the author
  (VALIDATION 25–26).
- **Mix AI balance (AI-22)** — the ratios between families and the return thresholds in the prompt are rules of thumb:
  whether the mixes improve is judged by the author with the real model (A-063, VALIDATION 31–32).
- **Connection to OpenRouter (AI-24)** — tested only against the fake server: the provider's behavior after a
  refusal and the registration inside the authorization page are not documented (A-071, VALIDATION 36).
- **Silent keychain in the signed build (AI-11)** — depends on the author's signing: not verifiable by the agent
  (A-072, VALIDATION 38).
- **New blocks and ideas by ear and with the real model (F15 → F17)** — the block properties are measured on
  synthetic signals; the sound, the sibilance threshold (−3 dB, A-089) and the quality of a real model's ideas are
  judged by the author (VALIDATION 47–54). The strict schema with blocks in `anyOf` is tested only with the fake
  server.
- **Position in the song (FX-20, FX-24)** — modulation and tremolo restart from the same phase at every position,
  even with the rate automated, verified with tolerance 1e-4 in the engine and 1e-5 in the processors (not bit by
  bit, because of floating-point accumulation). A rate changed by hand during playback realigns at the next start.
  The doubler drift is tied to the position, the phase of its pitch shifter (and of harmonizer and shimmer) restarts
  from where playback starts (A-088, A-090).
- **Bass enhancer (FX-18)** — without 4x oversampling and without level compensation: they are not needed by a
  harmonic generator below 640 Hz that is added to the dry (A-090).
- **Transient shaper below 100 Hz (FX-16)** — a very low steady tone can move the gain (A-088).
- **De-esser in split (FX-14)** — an octave below the frequency the level still drops by about 0.7 dB at 12 dB of
  reduction (A-090).
- **Benchmark at 48 kHz (NFR-1, FX-24)** — at 96 kHz the load is not measured; the pitch shifter search now does not
  grow with the frequency (A-090).
- **Idea name in the tooltip (UI-13)** — the idea name sits in the track note (rationale and tooltip of the number),
  not in a tooltip of every block (A-089).
- **Explain in the real app (F18)** — the view is tested on the fake backend and the level measurement by a Rust
  test; the `mix_levels` command in the Tauri app, the package and the self-test were not run (A-093, VALIDATION
  55–56).
- **Explain: per family, without measured reduction (UI-16)** — the text says who is in front per family and not per
  track, and describes compressors from ratio and threshold: the per-track render and the measurement of the
  reduction are missing (A-093). The Master does not have its view yet.
- **Self-test in the 0.1.1 package (A-103)** — with the Mac in use: three complete runs out of five, one with a step
  without signal on the bus, one that stayed stuck. With the Mac idle: three out of three, 59 out of 59. The two
  anomalous outcomes were not reproduced and the cause is not established: if they happen again with the machine
  idle, it is a defect to look for in the self-test or in the audio startup.
- **Windows not tried by hand (A-104)** — gate and self-test run on the development machine; installer on a clean
  Windows, listening, dragging with scaling and minimum window on a laptop at 150% are judged by the author
  (VALIDATION 64–66). Unsigned package: SmartScreen warns. The CI on Windows has never run yet. The render is not
  bit-identical across systems (system math library): the schema 3 test runs only on macOS.
- **Locate one file at a time (UI-6)** — the dialog with search by folder of design 13g is replaced by a menu of the
  missing files (A-047).
