# PRD — PoC engine for AI-assisted mixing and mastering

Nov 9, 2026 · @Piero

## 1. Context and goal

The PoC must answer a single question: can a language model that receives only numeric measurements of the stems decide
mix parameters that sound better than the raw sum?

The user is one person, the author of the songs. The author writes and records well, but is not strong at mixing and
mastering. The author wants a personal tool, without subscriptions or expensive software.

Three principles guide the whole project:

- **The model never receives audio.** It receives JSON with measurements and returns JSON with parameters. All audio
  processing runs locally with free libraries.
- **The mix plan is the only source of truth.** The same plan and the same stems produce the same audio file.
- **Every choice is explained.** The model justifies each intervention, so the author understands it and can correct it.

The PoC has no graphical interface: it is a Python engine driven from the command line. The expected outcome is a
go/no-go decision on building the full tool.

This document is written to be executed by Claude Code, one milestone at a time (section 9).

## 2. Hypotheses and success criteria

The PoC succeeds if it confirms the five hypotheses below, each with a verifiable threshold.

| Hypothesis | How it is measured | Threshold |
| --- | --- | --- |
| H1. Measurements are enough for the model to decide | Plans valid against the schema, after at most 2 attempts | 100% of calls |
| H2. The verification loop converges | Spectral distance from the reference between the first and the best iteration | Decreasing on at least 2 tracks out of 3 |
| H3. The result sounds better than the raw sum | Blind listening at loudness-matched levels (section 10) | AI mix preferred on at least 2 tracks out of 3 |
| H4. Requests in words produce the requested change | Revision requests judged "moved in the right direction" | At least 4 out of 5 |
| H5. The cost is negligible | Cost of model calls per track, from the logs | Below 0.20 USD per track |

Two technical constraints always apply, regardless of the hypotheses:

- The mix comes out with a true peak no higher than −3 dBTP, without a limiter.
- Rendering is deterministic: the same plan and the same stems give a file with the same hash.

The most honest comparison remains the one with a mix made by hand by the author, when one exists. It is not a success
threshold, but it must be noted.

## 3. Scope

The PoC covers the minimal path from stems to master, with static parameters for the whole track.

| Inside the PoC | Outside the PoC |
| --- | --- |
| Python engine driven from the command line | Graphical interface, plugins, real time |
| WAV or FLAC stems already aligned, from 2 to 16 | Editing, pitch correction, timing, noise cleanup |
| Role of each stem declared in the project file | Automatic recognition of instruments |
| Brief: genre, mood, free notes | Audio sent to the model |
| From 0 to 3 reference tracks, analyzed as stereo mixes | Separation into stems of the reference tracks |
| Per stem: gain, pan, filters, 4-band EQ, compressor, send to the reverb | Automation over time and differences between sections |
| One shared reverb | Delay, saturation, modulation, sidechain |
| Automatic verification loop, up to 3 iterations | Third-party VST plugins |
| Revision with a request in natural language | Export of a DAW project |
| Master for comparison with a reference | Master guided by requests in words |
| Full log and report for each run | Distribution, installer, multi-user |

If during development something from the right-hand column is needed, it must be flagged and not implemented.

## 4. Architecture

The engine is a four-step loop around a single call to the model: analysis, decision, render, evaluation.

&#91;embedded content: engine pipeline · 6 steps, 1 loop\]

The analysis measures stems, mix and references. The model receives only those numbers and returns a plan. The render
applies it, the evaluation measures the result and, if that is not enough, sends back to the model. The master starts
from the mix of the best plan.

## 5. Functional requirements

Each module reads and writes JSON files, and can be run on its own from the command line.

### 5.1 Ingest (ING)

- **ING-1.** Loads and validates `project.yaml` (section 6). Clear errors for missing fields or nonexistent files.
- **ING-2.** Accepts WAV or FLAC stems, mono or stereo, at 16, 24 or 32-bit float.
- **ING-3.** All stems must have the same sample rate. If they differ, error: no silent resampling.
- **ING-4.** If the durations differ, pads with silence at the end and writes a warning.
- **ING-5.** Reference tracks are resampled to the project sample rate only for analysis.
- **ING-6.** Internal processing in float32. Export to 24-bit WAV.

### 5.2 Analysis (ANA)

For each stem, computed only on the active parts (above −50 dBFS):

- **ANA-1.** Integrated loudness in LUFS (BS.1770), peak, true peak with 4x oversampling, RMS, crest factor.
- **ANA-2.** Dynamic range: difference between the 95th and the 10th percentile of the short-term loudness.
- **ANA-3.** Energy in 8 fixed bands, in dB relative to the total of the stem: 20–60, 60–120, 120–250, 250–500, 500–2k,
  2k–4k, 4k–8k, 8k–20k Hz.
- **ANA-4.** Spectral centroid, transient density (onsets per second), percentage of active time.
- **ANA-5.** For stereo stems: L/R correlation and side/mid ratio.
- **ANA-6.** Hygiene checks: DC offset, clipped samples, noise floor estimate.

On the mix and on the interactions between stems:

- **ANA-7.** The same measures as ANA-1, ANA-2, ANA-3 and ANA-5, on the sum.
- **ANA-8.** Share of each stem in the energy of the mix, per band, as a percentage.
- **ANA-9.** Masking: for each pair of stems and each band, percentage of frames in which both are active in that band,
  weighted by the lower energy of the two. The JSON contains only the 10 strongest collisions.
- **ANA-10.** Mono compatibility of the low end: L/R correlation below 120 Hz.

On the reference tracks:

- **ANA-11.** The measures of ANA-7, computed after normalizing the loudness, so the spectral comparison does not depend
  on the volume.
- **ANA-12.** Target profile = average of the references. Without references the profile is absent and the evaluator
  uses only the technical constraints.

Cross-cutting constraint:

- **ANA-13.** `analysis.json` for 12 stems stays under 8,000 estimated tokens (characters divided by 4). Values rounded
  to one decimal.

### 5.3 Decision (DEC)

- **DEC-1.** The model receives: a system prompt with the role and the parameter limits, the brief, the analysis, the
  target profile, the history of the last 2 iterations with their evaluations, and any request from the author.
- **DEC-2.** The model returns a complete `mix_plan.json`, never a diff. Use structured output with JSON Schema if the
  model supports it, otherwise JSON mode plus validation.
- **DEC-3.** Each stem has a `rationale` field of one or two sentences. The plan has a `summary` field.
- **DEC-4.** Validation with schema and limits (section 6). If the plan is invalid, up to 2 new attempts, sending the
  error back to the model. Then the command fails with a clear message.
- **DEC-5.** Limit on variation between one iteration and the next: gain ±4 dB, each EQ band ±3 dB. Values out of limit
  are brought back to the limit, with a warning.
- **DEC-6.** Model, temperature (default 0.2) and base URL are configurable. No model hard-wired in the code.
- **DEC-7.** There is a fake decider, deterministic and without network, with simple rules. It serves the tests and
  offline development.
- **DEC-8.** Each call is saved in full: request, response, tokens, cost if the response reports it, duration.

### 5.4 Render (REN)

- **REN-1.** Pre-gain: each stem is normalized to −23 LUFS before the plan, so the plan's gains are comparable.
  Can be disabled from `project.yaml`.
- **REN-2.** Chain per stem, in fixed order: pre-gain, high-pass, low-pass, EQ, compressor, makeup, gain, pan, send to
  the reverb.
- **REN-3.** Constant-power pan (−3 dB at the center) for mono stems, balance for stereo stems. Implemented in numpy.
- **REN-4.** One shared reverb, 100% wet, with global parameters and a high-pass on the return.
- **REN-5.** Mix bus: summing and automatic trim to stay within −3 dBTP. No other processing.
- **REN-6.** Writes `mix.wav`. With `--export-stems` it also writes the processed stems.
- **REN-7.** Always also writes the raw sum (`baseline.wav`): the original stems summed, with only the trim.

### 5.5 Evaluation and loop (EVA)

- **EVA-1.** Re-analyzes the rendered mix with the measures of ANA-7, ANA-9 and ANA-10. Does the same on the raw sum, as
  a comparison term.
- **EVA-2.** Spectral distance: mean absolute error in dB across the 8 bands, after aligning the average level to that
  of the target.
- **EVA-3.** Also reports the stereo width difference, the dynamic range difference and the residual collisions.
- **EVA-4.** Technical constraints: true peak, correlation below 120 Hz greater than 0, negligible DC offset.
- **EVA-5.** The loop stops when the distance falls below 1.5 dB, or improves by less than 0.2 dB, or the maximum number
  of iterations is reached (default 3).
- **EVA-6.** Keeps track of the best plan: the minimum distance among those that meet the constraints. If an iteration
  gets worse, the best remains the previous one.

### 5.6 Revision in words (REV)

- **REV-1.** `revise` runs a single iteration starting from the best plan, with the author's request as the priority.
- **REV-2.** In revision there is no automatic rollback: the author asked for the change and judges it by ear.
- **REV-3.** The report shows the parameter diff against the starting plan.

### 5.7 Master (MAS)

- **MAS-1.** Input: a `mix.wav` and a reference track, mandatory in the PoC.
- **MAS-2.** Uses Matchering to align the mix to the reference. Output at 24 bit, optionally 16 bit.
- **MAS-3.** After the master, measures integrated LUFS, true peak and spectral distance from the reference. Warning if the
  true peak exceeds −1 dBTP.

### 5.8 Report (REP)

- **REP-1.** Each run produces a `report.md`: table of the iterations with distance, LUFS and true peak, reasons per
  stem, main collisions, total cost.
- **REP-2.** `ab` prepares the files for blind listening: same duration, same loudness, random names, key in a separate
  file.

## 6. Data contracts

The four files below are the interfaces between the modules. The schemas are defined once, as Pydantic models, and the
JSON Schema passed to the model is generated from them. The values in the examples are illustrative.

### 6.1 `project.yaml`, written by the author

```yaml
name: "Titolo del brano"
sample_rate: 48000
brief:
  genre: "indie rock"
  mood: "caldo, intimo, un po' sporco"
  notes: "La voce deve stare davanti. Batteria asciutta."
stems:
  - id: kick
    file: stems/kick.wav
    role: kick
  - id: bass
    file: stems/bass.wav
    role: bass
  - id: gtr_l
    file: stems/guitar_left.wav
    role: electric_guitar
    notes: "ritmica, doppiata con gtr_r"
  - id: lead_vox
    file: stems/lead_vocal.wav
    role: lead_vocal
references:
  - file: references/ref1.wav
options:
  pre_normalize: true
  max_iterations: 3
  mix_true_peak_db: -3.0
```

Allowed roles: `kick`, `snare`, `toms`, `overheads`, `drums`, `percussion`, `bass`, `electric_guitar`, `acoustic_guitar`,
`keys`, `synth`, `pad`, `lead_vocal`, `backing_vocal`, `fx`, `other`.

### 6.2 `analysis.json`, produced by the analysis

```json
{
  "project": { "sample_rate": 48000, "duration_s": 214.3 },
  "stems": {
    "lead_vox": {
      "role": "lead_vocal",
      "channels": 1,
      "lufs": -21.4,
      "peak_db": -4.1,
      "true_peak_db": -3.8,
      "crest_db": 14.2,
      "dyn_range_lu": 9.3,
      "active_pct": 62,
      "bands_db": [-38.0, -24.1, -12.3, -8.0, -5.2, -9.9, -15.4, -22.0],
      "centroid_hz": 1850,
      "onsets_per_s": 2.1,
      "stereo": null,
      "hygiene": { "dc_offset": 0.0, "clipped_samples": 0, "noise_floor_db": -72.0 }
    }
  },
  "mix": {
    "lufs": -19.8,
    "true_peak_db": -3.0,
    "dyn_range_lu": 8.1,
    "bands_db": [-14.0, -9.5, -8.8, -9.9, -7.1, -12.4, -16.0, -21.5],
    "stereo": { "correlation": 0.82, "side_mid_db": -11.0 },
    "low_corr": 0.97
  },
  "band_share_pct": { "lead_vox": [0, 1, 12, 30, 41, 38, 22, 15] },
  "masking_top": [
    { "a": "gtr_l", "b": "lead_vox", "band": "2k-4k", "score": 0.71 }
  ],
  "target": {
    "n_references": 1,
    "bands_db": [-12.5, -9.0, -9.6, -10.8, -7.5, -11.0, -14.2, -19.0],
    "side_mid_db": -9.5,
    "dyn_range_lu": 6.8
  }
}
```

### 6.3 `mix_plan.json`, decided by the model

```json
{
  "version": 1,
  "iteration": 1,
  "summary": "Voce avanti, chitarre aperte ai lati, basse ripulite.",
  "reverb": {
    "room_size": 0.35,
    "damping": 0.5,
    "return_hpf_hz": 250,
    "return_gain_db": -6.0
  },
  "stems": {
    "lead_vox": {
      "gain_db": 2.0,
      "pan": 0.0,
      "hpf_hz": 90,
      "lpf_hz": null,
      "eq": [
        { "type": "peak", "freq_hz": 300, "gain_db": -2.5, "q": 1.4 },
        { "type": "high_shelf", "freq_hz": 9000, "gain_db": 1.5, "q": 0.7 }
      ],
      "compressor": {
        "threshold_db": -18.0,
        "ratio": 3.0,
        "attack_ms": 10.0,
        "release_ms": 120.0,
        "makeup_db": 3.0
      },
      "reverb_send_db": -14.0,
      "rationale": "Taglio a 300 Hz per togliere la sovrapposizione con le chitarre."
    }
  }
}
```

Parameter limits, enforced by validation. `null` turns the block off.

| Parameter | Minimum | Maximum |
| --- | --- | --- |
| `gain_db` | −24 | +12 |
| `pan` | −1 (left) | +1 (right) |
| `hpf_hz` | 20 | 400 |
| `lpf_hz` | 2,000 | 20,000 |
| EQ, number of bands | 0 | 4 |
| EQ `type` | `peak`, `low_shelf`, `high_shelf` |  |
| EQ `freq_hz` | 20 | 20,000 |
| EQ `gain_db` | −9 | +9 |
| EQ `q` | 0.3 | 8 |
| Compressor `threshold_db` | −40 | 0 |
| Compressor `ratio` | 1 | 8 |
| Compressor `attack_ms` | 0.1 | 100 |
| Compressor `release_ms` | 20 | 1,000 |
| Compressor `makeup_db` | 0 | 12 |
| `reverb_send_db` | −60 | −6 |
| Reverb `room_size`, `damping` | 0 | 1 |
| Reverb `return_hpf_hz` | 20 | 800 |
| Reverb `return_gain_db` | −24 | 0 |

### 6.4 `evaluation.json`, produced by the evaluator

```json
{
  "iteration": 1,
  "spectral_distance_db": 1.7,
  "band_delta_db": [3.0, 1.4, -2.2, -3.0, -1.6, 0.4, 0.7, 1.3],
  "width_delta_db": -1.5,
  "dyn_delta_lu": 1.3,
  "masking_top": [
    { "a": "gtr_l", "b": "lead_vox", "band": "2k-4k", "score": 0.48 }
  ],
  "constraints": { "true_peak_ok": true, "low_corr_ok": true, "dc_ok": true },
  "is_best": true,
  "stop_reason": null
}
```

`band_delta_db` is target minus mix, after aligning the average level: a positive value means the mix lacks energy in that
band.

## 7. Stack and technical constraints

Everything runs locally in Python; the only network call is the one to the model through OpenRouter.

| Scope | Choice |
| --- | --- |
| Language | Python 3.11 or later, project managed with `uv` |
| Audio I/O | `soundfile` |
| Analysis | `numpy`, `scipy`, `librosa`, `pyloudnorm` |
| Processing | `pedalboard` for filters, EQ, compressor, reverb |
| Master | `matchering` |
| Schemas and validation | `pydantic` |
| Calls to the model | HTTP client for the OpenRouter chat completions endpoint, OpenAI-compatible |
| CLI | `typer` |
| Tests | `pytest` |

Constraints:

- **Secrets.** The key is in the environment variable `OPENROUTER_API_KEY`. The `.env` file does not enter the repository.
- **Privacy.** No audio leaves the machine. Only measurements, stem ids and roles, and the brief go to the model.
- **No GPU.** It must run on an ordinary laptop.
- **Decider abstraction.** The code depends on a `Decider` interface with two implementations: the real model and the fake
  decider.
- **Pure functions.** Analysis, render and evaluation are functions without global state, testable with arrays in memory.
- **Language.** Code, comments and CLI messages in English. The `rationale` field in the language of the brief.
- **Licenses.** `pedalboard` and `matchering` are GPLv3. For personal use they carry no obligations; this matters only in
  case of distribution.

The default model is not fixed here: it must be chosen among those that support structured output, starting from an
inexpensive one (section 11).

### 7.1 Portability

Python is the choice for the PoC, not necessarily for the eventual application: the stack of that application will be
decided only after a go, with no constraints taken now. What must survive a change of language are the contracts, the
formulas and the results, not the code.

- **Schemas as a neutral specification.** The JSON Schemas generated from the Pydantic models are saved in the repository
  in `schemas/` and updated on every change. They are the definition of the contracts; the Python code implements them.
- **Explicit formulas.** Every analysis measure is documented in `docs/analysis.md` with its formula and exact parameters
  (window, STFT size and hop, band limits, activity threshold, loudness algorithm version). Filters and EQ are defined as
  standard biquads following the RBJ cookbook formulas, with coefficients derivable from the plan's parameters alone.
- **Engine without a terminal.** The engine communicates only through files and JSON and never reads from interactive
  input. The CLI is a thin wrapper over functions that accept paths and return data. This way another process, in any
  language, can invoke it as an executable or rewrite it piece by piece.
- **No library details in the contracts.** The exchange files contain no library names, serialized objects or values
  that depend on the implementation.

A warning for the future: a rewrite of the render chain will not produce identical samples, because the compressor and
the reverb are implementations specific to the library. A port must be re-validated by ear with the protocol in section
10, not only with tests. This is an accepted cost, deferred until after the go/no-go decision.

## 8. CLI, repository and output

Six commands cover the whole flow. `mixpoc` is a provisional name.

```bash
mixpoc analyze <progetto>                      # writes analysis.json
mixpoc mix <progetto> [--max-iter 3] [--decider llm|mock] [--export-stems] [--baseline-only]
mixpoc revise <progetto> "voce più avanti, meno riverbero sulle chitarre"
mixpoc master <progetto> [--mix <file>] [--reference <file>]
mixpoc report <progetto>                       # regenerates report.md of the last run
mixpoc ab <progetto>                           # prepares the files for blind listening
```

`<progetto>` is the folder that contains `project.yaml`. Every command exits with a non-zero code on error.

`revise`, `master`, `report` and `ab` work on the last run, unless otherwise indicated with `--run <cartella>`.

Repository structure:

```text
mixpoc/
  pyproject.toml
  README.md
  src/mixpoc/
    cli.py
    project.py          # loading and validation of project.yaml
    audio_io.py
    analysis/
      stem.py
      mix.py
      masking.py
      reference.py
    decision/
      schema.py         # Pydantic models of the plan
      prompt.py
      llm.py            # real decider via OpenRouter
      mock.py           # fake decider
      validate.py
    render/
      chain.py          # chain per stem
      mixbus.py
    evaluate.py
    loop.py             # decision, render, evaluation loop
    master.py
    report.py
  tests/
```

Folder of a track and output of a run:

```text
mio_brano/
  project.yaml
  stems/
  references/
  runs/
    2026-11-09T21-30-00/
      analysis.json
      baseline.wav
      iter_00/
        plan.json
        mix.wav
        evaluation.json
        llm_request.json
        llm_response.json
      iter_01/
      best.json           # points to the best iteration
      master/
        master.wav
        master_report.json
      report.md
```

A run never modifies the files in `stems/` and `references/`, and never overwrites earlier runs.

## 9. Milestones

Eight milestones in sequence: each one closes with green tests and a pause for the author's review before the next one
starts.

1. **M0, foundations.** Repository, empty CLI, loading of `project.yaml`, audio I/O, raw sum.
   - Acceptance: `mixpoc mix --baseline-only` on synthetic stems, without any plan yet, writes `baseline.wav` within −3 dBTP.
2. **M1, analysis.** Complete `analysis.json` for stems, mix and references (ANA-1 → ANA-13).
   - Acceptance: a 1 kHz sine falls in the 500–2k band; pink noise gives similar energy in bands of equal octave width; two
     identical stems give maximum masking.
3. **M2, render.** Applies a hand-written plan (REN-1 → REN-7).
   - Acceptance: +6 dB of gain give +6 measured LU, ±0.1; a pan at −1 zeroes the right channel; the high-pass attenuates
     below the cutoff; two renders of the same plan have the same hash.
4. **M3, decision.** Schema, prompt, fake decider, real decider, validation and retries (DEC-1 → DEC-8).
   - Acceptance: the full flow runs offline with the fake decider; with the real model a valid plan comes out and the logs
     are complete.
5. **M4, verification loop.** Evaluation, stop criteria, best plan, report (EVA-1 → EVA-6, REP-1).
   - Acceptance: the report compares the raw-sum distance with that of each iteration; the plan marked as best has the
     minimum distance among the valid ones; `report.md` is generated.
6. **M5, revision in words.** The `revise` command (REV-1 → REV-3).
   - Acceptance: with the request "voce più avanti" ("vocal further forward"), the report shows a diff on the parameters of
     the vocal or of the stems that cover it.
7. **M6, master.** The `master` command (MAS-1 → MAS-3).
   - Acceptance: `master.wav` produced; `master_report.json` reports LUFS, true peak and distance from the reference.
8. **M7, listening.** The `ab` command (REP-2) and the protocol of section 10 on 3 real tracks.
   - Acceptance: listening sheet completed and the go/no-go decision noted in the README.

Working rules for Claude Code:

- One milestone at a time. At the end of a milestone: a summary of what was done, what was decided and what remains doubtful.
- No out-of-scope features, not even "while we're at it".
- If a requirement is ambiguous or a library does not do what is needed, stop and ask instead of working around the problem.
- No test calls the network. Tests with the real model sit behind a marker and do not run by default.

## 10. Tests and evaluation

The automated tests verify that the engine does what it declares; only listening says whether the result is good.

### 10.1 Automated tests

- **Synthetic signals generated in the tests:** sines, pink noise, impulses, silence. No audio file in the repository.
- **Analysis:** known expected values for each measure, with explicit tolerances.
- **Render:** one test per block of the chain and a determinism test on the hash.
- **Schema:** valid plans accepted, out-of-limit plans corrected or rejected, non-JSON responses handled.
- **Full flow:** `analyze`, `mix`, `revise`, `master` with the fake decider, on a synthetic project created in a temporary folder.

### 10.2 Listening protocol

1. Choose 3 of the author's tracks, different in arrangement, from 4 to 12 stems each, with at least one reference each.
2. For each track run `mix` and then `ab`. The files to compare are: the raw sum, the best AI mix and, if it exists, the
   author's manual mix.
3. `ab` brings all the files to the same loudness and renames them at random. Without this step the louder file always wins.
4. Listen on at least two systems, for example headphones and speakers.
5. For each file, note a score from 1 to 5 on balance, clarity, low end and space, plus any obvious defects. Then indicate
   the preferred one.
6. Open the key only after voting.
7. On each track, try at least two requests with `revise` and note whether the change goes in the requested direction.

### 10.3 Decision

- **Go:** the thresholds H1 → H5 of section 2 are met.
- **Go with caveat:** H3 fails, but the noted defects point to a precise cause, for example a missing measure.
- **No-go:** the AI mix does not beat the raw sum, and the model's reasons do not explain the defects.

## 11. Risks and open questions

The main risk is that the measures do not capture what makes a mix good: in that case the model decides well on data that
does not matter.

| Risk | Effect | Countermeasure in the PoC |
| --- | --- | --- |
| The measures do not describe what the ear judges | Plausible plans but mediocre mixes | Reasons per stem, blind listening, a measure set easy to extend |
| Chasing the reference spectrum is not the same as a good mix | Mix forced toward a different arrangement | The target guides the evaluator, but the H3 threshold is listening |
| Static parameters, without automation | Tracks with strong differences between verse and chorus stay unbalanced | Out of scope: note the defect, do not solve it |
| The model oscillates or overdoes it | Iterations that get worse | Variation limits (DEC-5) and best plan (EVA-6) |
| Basic compressor and reverb | Result less refined than a DAW | Accepted for the PoC |
| Analysis and history make tokens grow | Rising cost and latency | ANA-13 cap, history limited to the last 2 iterations |

To decide before starting:

- [ ] Do the stems come out of the DAW raw or with the fader balance? Decides the default of `pre_normalize`.
- [ ] Which model to use as the default on OpenRouter.
- [ ] Operating system and machine on which the PoC runs.
- [ ] The 3 tracks for listening, and for which of them a manual mix exists.
- [ ] Final package name in place of `mixpoc`.

Deferred until after a possible go: per-section automation, sidechain between kick and bass, balances taken from the
references by separating them into stems, VST plugins, DAW project export, master guided by words.
