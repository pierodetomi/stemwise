# Decisions and open questions

Log of the decisions made during development and of the questions that block or condition a milestone.
Each entry: id, date, context, decision/question, reason, milestones affected. Decisions are never deleted: if one
changes, a new entry is added that replaces the previous one ("Replaces D-00x").

## Open questions

From PRD §11 ("To decide before starting") and from reading the PRD during setup.

| Id | Question | Blocks | Notes |
| --- | --- | --- | --- |
| Q-7 | PRD §4 cites a "engine pipeline · 6 steps, 1 cycle" diagram that is not in the exported file. | None | The text of §4 is enough; if the diagram adds constraints, they must be carried over into the PRD. |
| Q-40 | If a provider declares `structured_outputs` but rejects the `strict` schema (for example `minimum`/`maximum`), or if `require_parameters` with `seed`/`temperature`/`reasoning` finds no endpoint, the call fails with HTTP 400/404 and there is no caveat: D-045 chooses the mode only from `supported_parameters` (which on `/models` is the union across providers). | None today; to decide if it happens in the manual validation of M3 | Findings 2 and 11 of the M3 close review. (a) **Recommended, only if the case arises**: variable `STEMWISE_OUTPUT_MODE=json_object` to force JSON mode + validation on that model, and do not send `seed` (marginal benefit) when it narrows routing. (b) Automatic fallback: if the first attempt with `json_schema` gets a 400 on the schema, retry once with `json_object` (one extra attempt outside the DEC-4 count). (c) Leave as is and discard the models that do not handle the schema. |

## Closed questions

- Q-1 (raw or balanced stems) → D-025.
- Q-2 (default model) → D-047 (option c, chosen by the author after the M3 comparison).
- Q-4 (package name) → D-006.
- Q-8 (trim) → D-007. Q-9 (mono in the raw sum) → D-008. Q-10 (true peak filter) → D-009.
- Q-11 (`project.yaml` validation) → D-010. Q-12 (formats) → D-011 (integrated by D-013). Q-13 (resampling of references) → D-012.
- Q-14 → D-014, Q-15 → D-015, Q-16 → D-016, Q-17 → D-017, Q-18 → D-018, Q-19 → D-019 (M1, recommended options).
- Q-20 (decimals for correlations and scores) → D-021.
- Q-22 (full-band activity) → D-022, Q-21 (onsets in pauses) → D-023, Q-23 (reference names) → D-024
  (findings of the M1 close review, recommended options).
- Q-5 → D-026, Q-6 → D-027, Q-24 → D-028, Q-25 → D-029, Q-26 → D-030, Q-27 → D-031, Q-28 → D-032,
  Q-29 → D-033 (M2, recommended options, approved by the author with the plan).
- Q-30 → D-036, Q-31 → D-037, Q-32 → D-038, Q-33 → D-039, Q-34 → D-040, Q-35 → D-041, Q-36 → D-042,
  Q-37 → D-043, Q-38 → D-044, Q-39 → D-045 (M3, recommended options, approved by the author with the plan).
- Q-41 → D-049, Q-42 → D-050, Q-43 → D-051, Q-44 → D-052, Q-45 → D-053, Q-46 → D-054, Q-47 → D-055,
  Q-48 → D-056, Q-49 → D-057 (M4, recommended options, approved by the author with the plan).
- Q-50 → D-059, Q-51 → D-060, Q-52 → D-061, Q-53 → D-062, Q-54 → D-063, Q-55 → D-064, Q-56 → D-065,
  Q-57 → D-066 (M5, recommended options, approved by the author with the plan).
- Q-58 → D-069, Q-59 → D-070, Q-60 → D-071, Q-61 → D-072, Q-62 → D-073, Q-63 → D-074, Q-64 → D-075,
  Q-65 → D-076 (M6, recommended options, approved by the author with the plan).
- Q-3 (the 3 tracks for listening) and Q-66 → Q-72 (`ab` command, scorecard, hypothesis count): superseded by D-079,
  with no decision on the merits (M7 reduced, protocol §10.2 not run). The text of the questions remains in the git
  history (commit `M7: plan`).

## Decisions

### D-001 — Toolchain and repository structure (2026-11-09, setup)

- Python pinned to 3.12 (`.python-version`), `requires-python = ">=3.11,<3.14"`: the PRD asks for ≥ 3.11 and the
  upper bound avoids versions not yet supported by `numba`/`librosa`.
- `uv` project, `src/` layout, build backend `hatchling`. Quality: `ruff` (format + lint) and `pytest`.
  No type checker for now: the audio libraries have incomplete stubs; can be evaluated later.
- Runtime dependencies added milestone by milestone, only when needed (in M0: `typer`, `pydantic`,
  `pyyaml`, `numpy`, `soundfile`, plus what the true peak needs).
- `Makefile` as the single entry point (`make check` is the gate).

### D-002 — Automatic guarantees on the PRD rules (2026-11-09, setup)

- `tests/conftest.py` blocks network connections in every test not marked `llm` (PRD §9).
- `pytest` excludes `llm` tests by default (`-m "not llm"`), `--strict-markers` prevents badly written markers.
- Traceability: markers `req("ID")` and `acceptance("Mx")`; `scripts/req_coverage.py` compares the requirement ids in the
  PRD with those cited in the tests.
- `.gitignore` excludes audio (`*.wav`, `*.flac`, …), `runs/`, `local/`, `.env`.

### D-003 — Git flow (2026-11-09, setup)

Branch per milestone `milestone/mx-<slug>`, small commits with green tests, `--no-ff` merge into `main` and tag
`mx` only after the explicit approval of the author. No remote configured at the moment.

### D-004 — Development machine (2026-11-09, setup) — to be confirmed

Detected: macOS (Darwin 24.6), Apple M2, arm64. Answers the question in PRD §11 about the operating system,
unless the author indicates otherwise. Implication: dependencies must have `macosx_arm64` wheels.

### D-005 — PRD renamed (2026-11-09, setup)

`docs/PRD — PoC engine di mixing e mastering assistiti da AI.md` → `docs/PRD.md`, to reference it from `CLAUDE.md` and the
commands without spaces and special characters in the path. Content unchanged.

### D-006 — Package name: `stemwise` (2026-11-09, setup)

Closes Q-4. Python package, module (`src/stemwise/`) and CLI command are called `stemwise`, in place of the provisional
`mixpoc` of the PRD. The PRD is not modified: where it writes `mixpoc`, read `stemwise`. The repository folder remains
`mixpoc`. The name applies to the PoC; that of a possible product is decided after the go (PRD §7.1).

### D-007 — Bus trim: attenuation only, target limit − 0.01 dB (2026-11-09, M0)

Closes Q-8 (option b, chosen by the author). The bus is attenuated only if the true peak exceeds
`mix_true_peak_db`; in that case the gain brings the true peak to `mix_true_peak_db − 0.01 dB`, so the 24-bit file
read back stays within the limit despite rounding. A sum already within the limit is not touched.
Clarification in development: the implemented rule is `g_dB = min(0, (C − 0.01) − TP)`, so even a true peak
in the band (C − 0.01; C] is attenuated by the little that is needed; otherwise a file just below C could
exceed it after quantization (formula in `docs/analysis.md`).

### D-008 — Bus always stereo, mono stems in the center at constant power (2026-11-09, M0)

Closes Q-9 (option a, choice delegated to the agent). The raw sum is always stereo; a mono stem enters the center with
gain √½ (−3.01 dB) per channel, the same law as the pan of REN-3. Reason: a neutral plan (gain 0, pan 0, no block,
`pre_normalize: false`) reproduces the baseline, so the comparison between raw sum and AI mix measures only the choices
of the plan and not a different treatment of mono stems.

### D-009 — True peak: 4x oversampling with `scipy.signal.resample_poly` (2026-11-09, M0)

Closes Q-10 (option a, choice delegated to the agent). Default filter of `resample_poly(x, 4, 1)`: Kaiser FIR
β = 5.0, 81 taps, cutoff at the original Nyquist. Reason: it is a longer and flatter interpolator than the 48-tap
reference FIR of BS.1770-4, so it underestimates inter-sample peaks less, and it can be described in full with explicit
parameters in `docs/analysis.md`. If in M1 it is necessary to align with commercial meters, it can be compared with the
standard's filter.

### D-010 — Strict validation of `project.yaml`, with `sample_rate: auto` (2026-11-09, M0)

Closes Q-11 (option a, with an addition by the author). `name`, `sample_rate`, `brief`, `stems` mandatory;
2–16 stems and 0–3 references; unique ids in the format `^[a-z0-9][a-z0-9_]*$`; unknown fields rejected;
`options` optional with the PRD defaults, `mix_true_peak_db` ≤ −3.0, `max_iterations` 1–3.
`sample_rate` accepts an integer or `"auto"`: with `auto` the frequency is derived from the stems and is accepted if
all stems have only one; otherwise an error (ING-3). With an integer, all stems must match.
References do not count: they are resampled to the project frequency (ING-5).
Clarifications after the M0 close review: `mix_true_peak_db` is limited to [−24; −3] and rejects `inf`/`NaN`
(with `-.inf` the trim would have produced silence); stem ids have at most 32 characters.

### D-011 — Accepted formats: WAV PCM 16/24, float 32, FLAC 16/24 (2026-11-09, M0)

Closes Q-12 (option a, chosen by the author). Any other format or subtype (32-bit integer PCM, float 64, 8 bit)
and more than 2 channels give a clear error with the file path.

### D-012 — Reference resampling with `scipy.signal.resample_poly` (2026-11-09, M0)

Closes Q-13 (option a, choice delegated to the agent). Exact rational ratio (e.g. 160/147 for 44.1→48 kHz),
default filter (Kaiser FIR β = 5.0). Reason: the references serve only the 8-band analysis and the normalized loudness,
where the difference with a higher-quality resampler (soxr) is negligible; it avoids anticipating `librosa` in M0 and
remains documentable. In M0 there is only the loading function: `mix
--baseline-only` checks that the references exist without loading them.

### D-013 — WAV also with WAVE_FORMAT_EXTENSIBLE header and RF64 (2026-11-09, M0)

Integrates D-011, on a finding of the M0 close review. libsndfile classifies as a separate format (`WAVEX`) every WAV
with a WAVE_FORMAT_EXTENSIBLE header, common in 24-bit and float exports (ffmpeg, sox, various DAWs); RF64 is
the EBU extension of WAV beyond 4 GB, with the same `.wav` extension. Both are "WAV" for ING-2 and accept the
same encodings (PCM 16, PCM 24, float 32). Other containers remain excluded, for example W64 (Sony Wave64,
extension `.w64`). To be reviewed if the author prefers to exclude RF64.

### D-014 — Active parts: STFT frames above −50 dBFS (2026-11-10, M1)

Closes Q-14 (option a, approved by the author with the M1 plan). Grid: frames of the analysis STFT (Hann,
≈170 ms, hop ¼). Frame level = Hann-weighted mean square, averaged over channels; active if > −50 dBFS. On active
frames: RMS, crest, bands, centroid, stereo, `active_pct`. On the full signal: peaks, true peak, DC, clipping. Integrated
LUFS with BS.1770 gating only. Onsets counted on the full signal and divided by the active seconds. Noise floor
from the inactive frames.

### D-015 — Dynamic range = LRA of EBU Tech 3342 (2026-11-10, M1)

Closes Q-15 (option a). Short-term 3 s with a 100 ms step, absolute gate −70 LUFS and relative −20 LU, difference
between the 95th and 10th percentile. K-weighting of the short-term with the BS.1770 formulas, verified against `pyloudnorm`.
Clarification at step 1: `pyloudnorm` 0.2.0 already implements the LRA of Tech 3342 (`Meter.loudness_range`), so there is
no need to write the K-weighting separately; the short-term step is 90 ms (97% overlap) instead of 100 ms,
still compliant with the standard (at least 10 values per second). The K-weighting consistency test planned in
the plan lapses; the four cases of Tech 3342 remain.

### D-016 — Masking: band activity at −50 dBFS, min/max score frame by frame (2026-11-10, M1)

Closes Q-16 (options i and A). A stem is active in a band in a frame if the band level exceeds −50 dBFS.
Score `Σ_{t: both active} min(E_a,E_b) / Σ_{t: at least one active} max(E_a,E_b)`, in [0, 1]. In the JSON, the 10
highest scores greater than 0, with ties in project order and band order.

### D-017 — Per-band shares on the sum of the stems; analysis mix = `baseline.wav` (2026-11-10, M1)

Closes Q-17 (option a). The share of a stem in a band is its energy on the bus divided by the sum of the energies
of all stems in that band (the shares add up to 100%). The `mix` block of `analysis.json` measures the raw sum
with the trim, i.e. the signal of `baseline.wav`.

### D-018 — JSON: dB limited to ±120, DC offset as a percentage (2026-11-10, M1)

Closes Q-18 (options a, a). In the JSON, levels in dB are limited to [−120, +120] and `inf` or `NaN` never appear;
measures that require active parts are `null` on a fully inactive stem. The DC offset is a percentage of full scale,
with one decimal.

### D-019 — `analyze` writes to a new run folder (2026-11-10, M1)

Closes Q-19 (option a). `stemwise analyze <project>` creates `runs/<date-time>/analysis.json` with the same rules as the
M0 runs (no overwriting, folder created only after validation).

### D-020 — Onsets: absolute threshold on the non-normalized envelope (2026-11-10, M1)

Emerged at step 4. The plan foresaw `librosa.onset.onset_detect` with default parameters. But the default normalizes the
envelope to its maximum and uses `delta = 0.07`: on sustained signals, without attacks, the maximum is
small and the fluctuations become onsets. Measured: stationary sine 26 onsets/s, white noise 10/s, pink noise
11/s. A pad or a held bass would come out as "percussive" in the eyes of the model. Choice: `normalize=False` and
`delta = 2.0`, i.e. a peak must exceed the local mean of the positive mel spectral flux by 2 dB (units of
`librosa.onset.onset_strength`, independent of level). The defaults remain for the rest (`hop 512`, `n_fft 2048`,
128 mel bands, `pre_max 30 ms`, `post_max 0`, `pre_avg`/`post_avg 100 ms`, `wait 30 ms`).
Measured with the new threshold: sine, white noise and pink noise 0–0.1 onsets/s; clicks, damped noise bursts
(even at −34 dBFS) and plucked notes 4/s counted exactly. It can be revised on real material in manual validation
(`onsets_per_s` of voice and pad against drums).
Confirmed by the author on 2026-11-10.

### D-021 — Two decimals for correlations and masking scores (2026-11-10, M1)

Closes Q-20 (option b, chosen by the author). In `analysis.json` `correlation`, `low_corr` and `score` (dimensionless
quantities in [−1, 1]) are rounded to two decimals, as in the example of PRD §6.2; all other real numbers to one (ANA-13).
Reason: with one decimal the masking scores flattened (different collisions all at `0.5`) and a `low_corr` of 0.04 would
have appeared as `0.0`, i.e. as an apparent violation of EVA-4. Cost: about 40 more characters across 12 stems.

### D-022 — Frame power, activity and RMS limited to 20 Hz–20 kHz (2026-11-10, M1)

Closes Q-22 (option a, chosen by the author). Modifies D-014. Finding 2 of the close review: the frame power included DC and
sub-20 Hz content, so a DC offset above 0.32% (or rumble) made every frame active (`active_pct` 100 instead of 30, RMS
−18.7 instead of −13.5 dBFS). Now the frame power, used for the −50 dBFS threshold, for the RMS and for the noise floor,
sums only the bins 20 Hz–20 kHz, the same as the bands. Peaks, true peak, DC offset and clipping remain on the full signal.

### D-023 — Onsets counted only in active frames, envelope per channel (2026-11-10, M1)

Closes Q-21 (option a, chosen by the author). Modifies D-014 and integrates D-020. Finding 1 of the review: transients
in the pauses below −50 dBFS (headphone bleed of the click) were counted and divided by the active seconds alone
(sustained voice + bleed ⇒ 11.95 onsets/s). Now an onset counts only if it falls in an active frame (the frame with the
center nearest to the onset). Moreover the envelope is the **mean** of the channel envelopes, not the envelope of the channel
mean, so that antiphase (L = −R) does not cancel the onsets; the mean, instead of the sum proposed in Q-21, leaves the
scale of the 2 dB threshold unchanged for dual-mono stems.

### D-024 — References identified by neutral labels in `analysis.json` (2026-11-10, M1)

Closes Q-23 (option a, chosen by the author). Finding 3 of the review: the `file` field carried into the JSON intended for
the model (DEC-1) the path written in `project.yaml`, even an absolute one (user name, artist, title), against the
privacy of PRD §7. Now each reference has only `id` = `ref1`, `ref2`, `ref3`, in the order of `project.yaml`; the path
remains only in the local error messages.

### D-025 — Raw recorded stems: `pre_normalize: true` remains the default (2026-11-10, M1)

Closes Q-1. The author confirms that the stems come out of the DAW with the raw recording levels, without the fader
balancing (in M1 validation: 26 dB between the lowest and the highest stem, voice much lower than the guitars). The
default `pre_normalize: true` of PRD §6.1 is therefore the right one for REN-1 in M2.

### D-026 — Hand-written plan: `mix --plan <file>` (2026-11-10, M2)

Closes Q-5 (option a, approved with the M2 plan). `stemwise mix <project> --plan <file.json> [--export-stems]`
skips the decision and writes a new run with `baseline.wav` and `iter_00/` (`plan.json` copied, `mix.wav`, exported stems).
It is an extra option beyond PRD §8, useful also after M3 to correct a plan by hand.
Clarification after the M2 close review: the `plan.json` written in `iter_00/` has `iteration: 0`, the number
of the folder, whatever value the input file carries.

### D-027 — Plan model brought forward into M2 (2026-11-10, M2)

Closes Q-6 (option a). `stemwise.decision.schema.MixPlan` with the limits of PRD §6.3 is created in M2 and exported to
`schemas/mix_plan.schema.json`. Prompts, deciders, retries (DEC-4) and variation limits (DEC-5) remain for M3.

### D-028 — Shares and masking on pre-normalized stems (2026-11-10, M2)

Closes Q-24 (option a). With `pre_normalize: true` the per-band shares (ANA-8) and masking (ANA-9) are computed on the stems
brought to −23 LUFS by the pre-gain of REN-1, i.e. on the starting point of the plan. The `mix` block remains the raw sum
(D-017); the per-stem measures remain on the raw levels.

### D-029 — Filters and EQ: RBJ biquads in `scipy`, 12 dB/octave (2026-11-10, M2)

Closes Q-25 (option a). High-pass, low-pass, EQ and high-pass on the reverb return are RBJ cookbook biquads (form with Q)
applied with `scipy.signal.sosfilt` in float64; high-pass and low-pass are second-order Butterworth (Q = 1/√2).
`pedalboard` is used only for the compressor and the reverb. Reason: the first-order filters of
`pedalboard` (6 dB/octave, without Q) are not the biquads of PRD §7.1.

### D-030 — Linear balance of stereo stems (2026-11-10, M2)

Closes Q-26 (option a). Stereo stem: `L·min(1, 1−p)`, `R·min(1, 1+p)`. At the center the stem remains identical.

### D-031 — The mix lasts as long as the stems (2026-11-10, M2)

Closes Q-27 (option a). The reverb tail beyond the end of the stems is cut off.

### D-032 — `--export-stems`: post-pan stems with trim, plus the reverb return (2026-11-10, M2)

Closes Q-28 (option a). In `iter_NN/stems/` one file per stem (stereo, after the chain up to the pan, with the bus trim)
and `reverb_return.wav`: their sum is `mix.wav`, except for the 24-bit rounding.
Clarification after the M2 close review (finding 1): the reverb return is written to
`iter_NN/reverb_return.wav`, outside `stems/`, because `reverb_return` is a valid stem id (D-010) and the two files would
overwrite each other. If an exported stem exceeds 0 dBFS (possible with antiphase stems that cancel in the
mix) the 24-bit file clips it and the sum no longer matches: in that case a warning says so (finding 3).

### D-033 — Edge cases of the render (2026-11-10, M2)

Closes Q-29 (option a). Muted stem or shorter than 400 ms ⇒ pre-gain 0 dB with a warning; no ceiling on the pre-gain, warning
above +24 dB; the plan must contain exactly the ids of the project; plan frequency ≥ `fs/2` ⇒ error.
Added after the M2 close review (finding 4): stems shorter than 3 samples ⇒ clear error (the `pedalboard` plugins
derive the channels from the smaller dimension of the array).

### D-034 — `pedalboard` compressor per channel, accepted (2026-11-10, M2)

Verified at step 4 of the plan: `pedalboard.Compressor` (JUCE) has a peak detector per channel, so on a stereo stem
the two channels are compressed independently (left at −10 dBFS, right 40 dB below, threshold −30,
ratio 4: left reduced by 14.2 dB, right unchanged). As proposed in the approved plan, it is accepted for the PoC and noted:
the stereo image of stems with very unbalanced channels may shift. A linked compressor would require leaving `pedalboard`.

### D-035 — Strict validation of the plan, without coercion (2026-11-10, M2)

Finding 2 of the M2 close review. The plan model accepted `"gain_db": "2"`, `"pan": true` or
`"iteration": "1"` converting them, while `schemas/mix_plan.schema.json` rejects them. Now the model is `strict`: it accepts
exactly what the exported schema accepts (an integer remains a valid number). The schema does not change.

### D-036 — HTTP client: `httpx` (2026-11-10, M3)

Closes Q-30 (option a, revised at the author's request). `httpx` 0.28.1 for the call to OpenRouter:
`httpx.MockTransport` simulates OpenRouter in tests without network, separate timeouts for connection and read, distinct
exceptions for timeout, connection and HTTP status, readable body of error responses for the DEC-8 logs. `urllib` was discarded
(same result with more hand-written code) and the `openai` SDK (automatic retries, objects instead of the raw
body, OpenRouter fields via `extra_body`).

### D-037 — Decider configuration from environment and `.env` (2026-11-10, M3)

Closes Q-31 (option a). `STEMWISE_MODEL` (mandatory with `--decider llm`, no default in the code),
`STEMWISE_TEMPERATURE` (default 0.2), `STEMWISE_BASE_URL` (default `https://openrouter.ai/api/v1`),
`STEMWISE_REASONING_EFFORT` (optional, `low`/`medium`/`high`/`xhigh`, sent as `reasoning.effort`), plus
`OPENROUTER_API_KEY`. Read from the environment and from `.env` in the current folder (lines `KEY=value`; the environment
wins); `--model`, `--temperature` and `--reasoning-effort` override everything. `project.yaml` unchanged.

### D-038 — `mix` in M3: one decision and one render (2026-11-10, M3)

Closes Q-32 (option a). `mix` writes `iter_00/` with `plan.json`, `mix.wav`, `llm_request.json` and (with the real model)
`llm_response.json`, without `evaluation.json`. The history of the last 2 iterations (DEC-1) and the variation limits
(DEC-5) are pure functions tested on synthetic data, connected to the loop in M4; `--max-iter` arrives in M4.

### D-039 — Structured output with a schema per project (2026-11-10, M3)

Closes Q-33 (option a). Schema derived from the Pydantic model with `stems` as an object whose properties are exactly
the ids of the project (all required, `additionalProperties: false`), `response_format: json_schema`,
`strict: true`, `provider.require_parameters: true`. From the schema sent only the keywords outside the `strict`
subset are removed; the local validation (D-035, `check_plan_fits`) remains the only authority. The sent schema is
saved in `llm_request.json`, not in `schemas/`. The engine enforces `iteration`. A single ```` ```json ```` block
is accepted by removing the backticks. The choice between `json_schema` and `json_object` follows D-045 (no
`STEMWISE_OUTPUT_MODE`).

### D-040 — Retries and call log (2026-11-10, M3)

Closes Q-34 (option a). The retry continues the conversation (invalid answer as `assistant`, errors
as `user`). They count as an invalid plan: non-JSON text, empty or truncated answer, violated schema, plan that does not
fit the project; network and HTTP errors are not retried. `llm_request.json` and `llm_response.json` contain
`{"attempts": [...]}` aligned by index (body sent without headers or key; body received, status,
duration, tokens, cost `usage.cost` or `null`, validation outcome). If the decision fails the run folder
remains, with a message that indicates it. With the fake decider `llm_request.json` is written with the messages that would
be sent.
Clarification in development (step 7): the folder remains only if the model was called at least once; an error
before the first call (for example model not available on OpenRouter) costs nothing and the folder is removed as in M0–M2.
Empty values in `.env` (as in a copied `.env.example`) count as unset.

### D-041 — Prompt content (2026-11-10, M3)

Closes Q-35 (option a). System prompt in English with role, chain, units, limits generated from the model and the warnings of M1–M2
(pre-gain at −23 LUFS, instruments split into several stems, collisions between microphones of the same
drum kit, scores below 0.05 negligible, level of the reverb return, trim at −3 dBTP); `rationale` and
`summary` in the language of the brief. User message: brief, stems with id, role and `notes`, `analysis.json`, history,
request. The title of the track is not sent; the references remain `ref1…` (D-024).

### D-042 — Variation limits: EQ bands by position (2026-11-10, M3)

Closes Q-36 (option a). DEC-5 limits the stem `gain_db` to ±4 dB and the `gain_db` of each EQ band to ±3 dB relative to the
previous iteration. Bands paired by position; different type or new band ⇒ previous gain 0 dB;
frequency and Q free; removing a band is allowed; no limit on the first iteration. Each correction gives a warning
and an entry in the log.

### D-043 — Rules of the fake decider (2026-11-10, M3)

Closes Q-37 (option a). Pure function of roles and analysis: gain per role, drum family with n > 1 stems at
−10·log₁₀ n dB, mono stems of the same role alternated ±0.5, high-pass per role, a `peak` −2 dB on the other stem of the
strongest collisions with voice, bass or kick (at most 2 bands per stem), fixed compressor on voice and bass,
fixed reverb with sends per role. Texts in English; the author's request is ignored until M5. The exact values
are documented in the module `decision/mock.py`.

### D-044 — Default of `--decider`: `llm` (2026-11-10, M3)

Closes Q-38 (option a). Key and model are checked before any work.

### D-045 — Request parameters according to the model (2026-11-10, M3)

Closes Q-39 (option a). Before the decision the decider reads the `supported_parameters` of the model (OpenRouter's `GET /models`).
`temperature` and `seed: 0` are sent only if supported, otherwise a warning and a note in the log;
`json_schema` if the model supports `structured_outputs`, otherwise `json_object` + validation (DEC-2);
`reasoning.effort` if configured.

### D-046 — Findings of the M3 close review (2026-11-10, M3)

Integrates D-039, D-040, D-043 and D-044, on findings of the independent review.
- After an empty answer the retry sends back the `assistant` message with the text `(empty answer)`, because
  some providers reject empty messages (finding 1).
- The model is looked up on `GET /models` before the analysis (`preflight`): a wrong id fails immediately, without a
  run folder (finding 7, D-044).
- If the decision is interrupted after calls already made for a cause other than `DecisionError` (Ctrl-C, unexpected
  error), the folder remains and the call logs are written; if the render fails after a decision with the model, the
  message indicates the kept folder (finding 8, D-040).
- Every plan, from any decider, goes back through `MixPlan` and `check_plan_fits` before the render (finding 5).
- Values brought back to the DEC-5 limits are rounded to 6 decimals (finding 6); a ```` ```json ```` block
  with CRLF line endings is accepted (finding 10).
- `--decider`, `--model`, `--temperature` and `--reasoning-effort` with `--plan` or `--baseline-only` give an error
  instead of being ignored (finding 9).

### D-047 — Default model: GPT-6.1 Sol, medium effort; H5 to be reviewed with real costs (2026-11-10, M3)

Closes Q-2 (option c, chosen by the author). Comparison in the M3 manual validation on the track "Confidence" (13 stems),
one call per model, all valid on the first attempt:

| Model (effort) | Tokens in + out | Cost | Duration | Author's listening |
| --- | --- | --- | --- | --- |
| `openai/gpt-6.1-terra` (high) | 6,727 + 3,545 | 0.026 USD | 47 s | decent |
| `openai/gpt-6.1-sol` (medium) | 6,727 + 3,451 | 0.051 USD | 47 s | decent, slightly preferred |
| `anthropic/claude-haiku-5.5` (xhigh) | 10,002 + 22,768 | 0.012 USD | 101 s | discarded |

Haiku does not compensate for the drums split into 7 stems (group estimated about 5 dB above the voice, against about 1 dB for Terra
and parity for Sol). Default to set in `.env` and `.env.example`: `STEMWISE_MODEL=openai/gpt-6.1-sol`,
`STEMWISE_REASONING_EFFORT=medium` (no default in the code, DEC-6; `.env.example` updated by the author, because
the agent is denied access to the file). **H5 remains to be verified**: with about 5 calls per track (`mix` up to 3 + 2
`revise` of protocol §10.2) Sol is estimated at ~0.26 USD, above the threshold of 0.20; Terra at ~0.13. The decision on H5 (threshold,
count of revisions or change of default) is taken in M4/M5 with the measured costs of a whole track.

### D-048 — Parameters per section (automation): product must-have (2026-11-10, after M3)

Indication of the author after the M3 validation (distorted guitars of the chorus barely audible with static parameters).
In the PoC they remain out of scope (PRD §3, right column; §11, "deferred until after a possible go"). If the outcome is go or go
with caveats, the product **must** include different parameters for each section of the track (automations). With them it also becomes
useful to give the model compact measures over time, for example the loudness of each stem per section (not the waveform, which is
audio: PRD §1). To be reported in the PoC outcome in the README (M7).

### D-049 — Contract of `evaluation.json` and the loop files (2026-11-10, M4)

Closes Q-41 (option a, approved with the M4 plan). `evaluation.json` has the fields of PRD §6.4 plus a `mix` block
with the measures of ANA-7 and ANA-10 of the rendered mix, in the same format as the `mix` block of `analysis.json` (needed
by REP-1). The raw sum is evaluated in `baseline_evaluation.json` at the root of the run, same schema, `iteration:
null`. `is_best` says whether the iteration is the best at the moment it is evaluated: each `evaluation.json` is written
only once. The final authority is `best.json` (`iteration`, `spectral_distance_db`, `stop_reason`, path of the plan),
updated after each iteration. Schemas in `schemas/evaluation.schema.json` and `schemas/best.schema.json`.

### D-050 — Formulas and signs of the evaluation (2026-11-10, M4)

Closes Q-42 (option a). `offset = mean(target) − mean(mix)` (arithmetic mean in dB of the 8 bands),
`band_delta_i = target_i − (mix_i + offset)`, distance = mean of `|band_delta_i|`. `width_delta_db =
target.side_mid_db − mix.side_mid_db`, `dyn_delta_lu = target.dyn_range_lu − mix.dyn_range_lu`: positive = the mix lacks it. `null` if
one of the two values is missing. Calculation at full precision (levels limited to ±120 dB), rounding to one decimal only in the file.

### D-051 — Stop criteria and best plan (2026-11-10, M4)

Closes Q-43 (option a). Improvement = distance of the previous iteration − distance of the current; iteration
0 is compared only with the 1.5 dB threshold; an iteration that violates the constraints does not stop the loop for the threshold and
cannot be the best. Order of the reasons: `below_threshold`, `no_improvement`, `max_iterations`. The decisions use the
values written in `evaluation.json` (one decimal), compared in integer tenths; on a tie the oldest iteration wins.
DEC-5 remains relative to the previous iteration. `--max-iter N` (1–3) overrides `options.max_iterations`.

### D-052 — Loop without references (2026-11-10, M4)

Closes Q-44 (option a). Without targets distance and delta are `null`; the loop stops at the first iteration that
respects the constraints (`stop_reason: no_target`), otherwise it continues up to the maximum; best = the first valid one.

### D-053 — Technical constraints (2026-11-10, M4)

Closes Q-45 (option a). `true_peak_ok`: true peak of the trimmed mix in memory ≤ `mix_true_peak_db`. `low_corr_ok`:
`low_corr > 0`; `null` (no active frame below 120 Hz) counts as ok. `dc_ok`: mean of each channel ≤ 0.1% of
full scale (−60 dBFS), the measure of ANA-6.

### D-054 — Residual collisions (2026-11-10, M4)

Closes Q-46 (option a). Formula of ANA-9 (`masking v2`), top 10, on the processed stems as they arrive at the bus (after the pan,
before the trim, without the reverb return); for the raw sum on the original stems of `baseline.wav`,
reusing the spectra of the analysis. The spectra of the processed stems are computed one at a time during the render.

### D-055 — Errors midway through the loop (2026-11-10, M4)

Closes Q-47 (option a). Error at iteration 0 ⇒ as M3 (D-040, D-046). At k ≥ 1 ⇒ stop with `stop_reason:
decision_failed`, iterations done kept, `best.json` and `report.md` written, warning, exit 0. No valid iteration ⇒ `best.json` with `iteration: null`, report that says so, exit 1. Interruption ⇒ folder kept if the
model was called; `best.json` updated after each iteration.

### D-056 — `report.md` and the `report` command (2026-11-10, M4)

Closes Q-48 (option a). Fixed text in English; summaries and reasons as written by the decider. Content:
decider and model, brief, table with the raw sum and each iteration, reason for stop, summaries, DEC-5 corrections, main
parameters and per-stem reasoning of the best plan, 5 main collisions of the raw sum and the residual ones of the best, total
cost and tokens. The `mix` runs (decider and `--plan`) write it. `report` regenerates it from the run files only; "last run" = the most recent with `best.json`; `--run <folder>` for another.

### D-057 — Feedback of the fake decider (2026-11-10, M4)

Closes Q-49 (option a). Integrates D-043. From iteration 1 the fake starts from the previous plan and, in the band with the
largest `|band_delta_db|` (if ≥ 1 dB), corrects the stem with the largest share in that band with a `peak` (Q 1, at the
geometric center of the band) of gain `band_delta_db` limited to ±3 dB: it adjusts the `peak` already present at that
frequency or adds one if the stem has fewer than 4 bands. Without target, no correction.

### D-058 — Findings of the M4 close review (2026-11-10, M4)

Integrates D-050, D-054 and D-056, on findings of the independent review.
- Collisions of the raw sum (finding 2): mono stems are measured centered on two channels at `√½`, as they enter the bus of
  `baseline.wav` and as a mono stem rendered with the pan at center. Before, the activity threshold (mean over channels) was 3 dB
  stricter in the mix than in the raw sum, and a neutral plan "reduced" the collisions without
  changing anything. Cost: one extra STFT for each mono stem, once per run.
- Deviations limited to ±120 dB before entering the contract (finding 3): with extreme bands the formula could exceed
  the limits of the schema and interrupt the loop.
- `report` (finding 1): a malformed run file gives a clear error instead of a traceback. Collisions that
  round to 0.00 do not appear in the report (finding 6). `--run` first looks for a name in `runs/` of the track, then
  a path (finding 8).

### D-059 — `revise` writes a new run (2026-11-10, M5)

Closes Q-50 (option a, approved with the M5 plan). A revision creates `runs/<date-time>/` with `revision.json`
(starting run and iteration, path of the plan, request), `source_plan.json` and `source_evaluation.json` (copies of the
starting plan and its evaluation), `analysis.json`, `baseline.wav`, `baseline_evaluation.json` (copied from the
starting run), `iter_00/` (plan, `mix.wav`, evaluation, decider log), `best.json`, `report.md`. The starting run is not
touched. The revision becomes the "last run" (D-056); a `revise` after another starts from the revised plan. `revision.json` has the
schema `schemas/revision.schema.json`; `stop_reason` gains the value `revision`.

### D-060 — Starting and best plan of a revision (2026-11-10, M5)

Closes Q-51 (option a). `revise` starts from the plan indicated by `best.json` of the chosen run (last with `best.json`
or `--run`): after a `mix`, the best plan of that mix. `best.json` with `iteration: null` ⇒ clear error suggesting
`--run`. In the revision run `best.json` indicates `iter_00` whatever the distance (no rollback,
REV-2), provided it respects the technical constraints (PRD §2); if it violates one, `best.json` has `iteration: null`, the files remain
for listening and the exit is 1. The variant "automatically skip invalid revisions" is discarded because it is an automatic rollback.

### D-061 — Revision: analysis reused, references re-measured (2026-11-10, M5)

Closes Q-52 (option a). `analysis.json`, `baseline_evaluation.json` and `baseline.wav` are copied from the
starting run; only the references are re-measured, so that the evaluation target is at full precision as in the
starting run. First a consistency check: ids, roles, channels, duration and frequency of the project stems equal
to those of `analysis.json`, re-measured target equal (rounded) to the saved one; otherwise an error that indicates the
run and suggests `mix`.
Clarification in development (step 3): the check also compares the sample peak of each stem (`peak_db`, one
decimal), which costs one read and catches a stem replaced with the same duration. The `options` of
`project.yaml` (`pre_normalize`, `mix_true_peak_db`) are not saved in the starting run and therefore cannot be compared: if they
change between `mix` and `revise`, the revision uses the new ones without warning.

### D-062 — Model input in revision (2026-11-10, M5)

Closes Q-53 (option a). History = only the starting plan with its evaluation; request in the section "Author's
request (top priority)"; "Revision" section in the system prompt, present only with a request. The requests of the previous
revisions are not sent. The revised plan has `iteration: 0` (the folder, as D-026).

### D-063 — DEC-5 also applies in revision (2026-11-10, M5)

Closes Q-54 (option a). Gain ±4 dB and EQ bands ±3 dB relative to the starting plan (D-042), with warning and entry in the
report.

### D-064 — Plan diff (2026-11-10, M5)

Closes Q-55 (option a). Pure function `plan_diff(before, after)` on each field of the plan; EQ bands by position
as D-042; exact equality of the written values. The diff appears only in the report (regenerable from `source_plan.json` and
`iter_00/plan.json`), with only the changed lines, the unchanged stems and the new reasons of the changed stems.

### D-065 — The fake decider reads the request (2026-11-10, M5)

Closes Q-56 (option a). Integrates D-043. Italian and English keywords: subject (stem id or role by synonym,
plus reverb), direction (up ⇒ +2 dB of gain, down ⇒ −2 dB; with the reverb ±3 dB on the sends). Unrecognized request ⇒ starting plan
unchanged with a `summary` that says so. The exact rules are in the module `decision/mock.py`.

### D-066 — `revise` options and revision report (2026-11-10, M5)

Closes Q-57 (option a). `stemwise revise <track> "<request>" [--run] [--decider] [--model] [--temperature]
[--reasoning-effort] [--export-stems]`, without `--max-iter`; `--run` as for `report`; empty request or over 1,000
characters ⇒ error. Revision `report.md` in English: request, starting run, decider and cost, raw sum · starting · revised table, summary, diff, warnings, collisions before and after. `stemwise report` recognizes it from
`revision.json` and regenerates it identically.

### D-067 — H5: acceptable limited overrun (2026-11-10, M5) — to be confirmed with the measured cost

Indication of the author at the start of M5: if the cost per track slightly exceeds the threshold of 0.20 USD (estimate with the 2
`revise` of protocol §10.2: ~0.23–0.30), the overrun is acceptable. It is confirmed at the close of M5 with the
measured cost of two `revise` on the real track; if the cost comes out much higher it will be discussed again.

### D-068 — H5 accepted with limited overrun (2026-11-10, M5)

Confirms D-067 with the cost measured in the M5 manual validation on "Confidence" (Sol, medium effort): `mix` with 2
iterations 0.1116 USD, two `revise` 0.0589 and 0.0532 USD, i.e. ~0.22 USD per track with protocol §10.2 (one `mix` and
two `revise`). The threshold of 0.20 USD of PRD §2 is exceeded by about 10%: the author judges it acceptable. In M7
H5 is evaluated with this tolerance; if a track with 3 iterations of `mix` greatly exceeded ~0.25 USD, it will be discussed again.

### D-069 — Dependency `matchering==2.0.6` (2026-11-10, M6)

Closes Q-58 (option a, approved with the M6 plan). `uv add matchering==2.0.6` (pinned version, last published, October 2022), authorized by the author. It brings `resampy` and `statsmodels`, and with them `pandas`, `patsy`,
`formulaic`, `interface-meta`, `wrapt`, `python-dateutil` and `tzdata` (completed after the close review). Facts
verified on the source: deterministic processing (the only `random` names the ffmpeg temporary files), default internal
frequency 44,100 Hz, final sample-peak limiter with threshold `(2^15 − 61)/2^15` ≈ −0.016 dBFS, mono
duplicated to stereo, duration between `fft_size` samples and 15 minutes, error if mix and reference coincide.

### D-070 — Master at the project frequency (2026-11-10, M6)

Closes Q-59 (option a). `matchering.Config(internal_sample_rate=project fs)`: the mix is not resampled, the
master comes out at the project frequency, the reference is resampled as in ING-5 (D-012). The library warns that values other than 44,100 are not "tested properly"; the source scales FFT, windows and limiter with the frequency.

### D-071 — `matchering` called in memory (2026-11-10, M6)

Closes Q-60 (option a). Reading (formats of D-011), resampling and writing with `audio_io`; `matchering` receives
arrays (`matchering.stages.main`). The checks of `matchering.checker` that are needed (duration, mono → stereo, mix equal to the
reference) are redone in `master.py`. A test compares the output with that of `mg.process` at 44.1 kHz.

### D-072 — Where `master` writes (2026-11-10, M6)

Closes Q-61 (option a). Inside the chosen run: `master/` if it does not exist, otherwise `master_02/`, `master_03/`…; no existing
file changes. With `--mix` a new run `runs/<date-time>/master/` without `best.json`, which therefore does not become the "last run" (D-056).

### D-073 — Inputs and options of `master` (2026-11-10, M6)

Closes Q-62 (option a). `stemwise master <track> [--run] [--mix <file>] [--reference <id|file>] [--pcm16]`. Mix =
`iter_NN/mix.wav` indicated by `best.json` of the last run (or of `--run`); `best.json` with `iteration: null` ⇒ error
suggesting `--run` or `--mix`; `--run` with `--mix` ⇒ error. Reference = `ref1`; `--reference` accepts
`ref1…ref3` or a path; no reference ⇒ error (MAS-1). Mix at a frequency different from the project ⇒ error (as
ING-3); reference resampled (ING-5).

### D-074 — True peak of the master: warning only (2026-11-10, M6)

Closes Q-63 (option a). `matchering` limiter with the default settings; true peak measured and warning above
−1 dBTP (MAS-3), no correction. Expected: with strong commercial references the warning will be frequent.

### D-075 — 16 bit in addition (2026-11-10, M6)

Closes Q-64 (option a). `master.wav` always at 24 bit; `--pcm16` also writes `master_16.wav` from the same signal,
without dither. The MAS-3 measures are on the 24-bit file.

### D-076 — Content of `master_report.json` (2026-11-10, M6)

Closes Q-65 (option a). Model `MasterReport` → `schemas/master_report.schema.json`: origin of the mix, reference,
frequency, files written; `lufs`, `true_peak_db`, `true_peak_ok` (≤ −1 dBTP); `spectral_distance_db` and `band_delta_db`
with the formula of EVA-2 between the bands of the master (ANA-7) and those of the reference used (ANA-11); for comparison the
distance of the mix before the master and the LUFS/true peak of the reference; `warnings`. No library name. Measures on the
24-bit file read back. `report.md` unchanged.

### D-077 — Master decided by the model: idea for the product (2026-11-10, M6)

Question of the author at the start of M6: could the model make a creative contribution also in the master? In the PoC no:
the master serves the comparison with a reference (PRD §3, MAS-2 "use Matchering"), the master guided by words is out of scope (§3, §11), and a master decided by the model would confuse the reading of H3 (the blind listening compares the mixes at equal loudness). With `matchering` the creative choice lies entirely in the choice of the reference. If the outcome is go, for the product a master with parameters chosen and motivated by the model is evaluated: how much to imitate the reference or how to weigh more than one, loudness and true peak according to the destination, stereo width and compression according to genre and mood,
revision in words. Expected value lower than in the mix (few parameters, most of what is measurable already covered by the imitation of the reference). To be reported in the PoC outcome in the README (M7), together with D-048.

### D-078 — Findings of the M6 close review (2026-11-10, M6)

Integrates D-071, D-074 and D-076, on findings of the independent review.
- Mono inputs (finding 1): loudness, true peak and bands of mix and reference are measured on the stereo version that
  enters the master (mono duplicated), otherwise a mono reference came out 3 LU below a master at the same level.
- True peak (finding 2): `true_peak_ok` and the warning compare the full-precision value with −1 dBTP, as
  EVA-4 (D-053); the file reports the rounded value, so −0.96 dBTP reads −1.0 with `true_peak_ok: false`.
- Import (finding 3): `matchering` is imported only when a master is made; the other commands no longer depend on the library nor on its import of the deprecated `scipy.ndimage.filters`.
- An input shorter than 0.4 s (one loudness block) gives "too short", not "silent" (finding 4).
- Tolerances of the approach test tightened from 1.0 dB / ±1 LU to 0.3 dB / ±0.3 LU, about ten times the measured values (finding 6).

### D-079 — Reduced M7: no `ab` nor blind listening protocol (2026-11-10, M7)

Choice of the author after the M7 plan (option (a) proposed in chat). REP-2 (`ab`) and the protocol of PRD §10.2 on 3
tracks **are not run**: the author considers the listenings of the manual validations of M3–M6 on "Confidence" sufficient and does not find the
blind listening procedure useful. M7 closes with the outcome of the PoC in the README (criterion
of PRD §9), compiled with the data already measured. The `ab` command remains in the CLI and answers "not implemented" (exit code 3) with the reference to
this decision. Q-3 and Q-66 → Q-72 are superseded. Limits to keep in mind when reading the
outcome: one track instead of three, listening not blind and not at equal loudness (in M4 the raw sum was ~4 dB louder than the AI
mix, so the imbalance worked **against** the AI mix, which was nevertheless preferred), requests in words 3 instead of 5.
The author also discarded an exploratory milestone on section automations within the PoC: one moves on to the product
(D-080).

### D-080 — PoC outcome: go (2026-11-10, M7)

Decision of the author: **go**, one moves on to the development of the product. Hypotheses of PRD §2 with the data of M3–M6 (detail
in the README, "PoC outcome"):

| Hypothesis | Threshold | Measured | Outcome |
| --- | --- | --- | --- |
| H1 valid plans | 100% within 2 new attempts | 8 real calls on "Confidence" (3 models) and the 3 synthetic `llm` tests: all valid on the **first** attempt | ✅ |
| H2 the loop converges | decreasing on ≥ 2 tracks out of 3 | 1 track: Sol 2.1 → 0.7 dB, Terra 1.7 → 1.2 dB (raw sum 3.7) | ✅ on 1 track out of 1 |
| H3 better than the raw sum | preferred on ≥ 2 tracks out of 3, blind at equal loudness | 1 track, listening not blind: best mix preferred to raw sum and iter_00 (M4) | ✅ according to the author, not blind |
| H4 requests in words | ≥ 4 out of 5 | 3 out of 3 in the requested direction (M5: synthetic test and two real requests) | ✅ on 3 |
| H5 cost | < 0.20 USD per track | ~0.22 USD (`mix` 0.1116 + `revise` 0.0589 + 0.0532), accepted (D-068) | ✅ with tolerance |

Notes for the product, to be reported in the README: parameters per section (D-048), master with parameters chosen by the model
(D-077), and the limits observed in the validations (collisions dominated by drum microphones, per-channel compressor
D-034, master limiter on the sample peak D-074, 3–7 minutes for a `mix` of 13 stems, warning on `temperature`
repeated at every call, Q-7 and Q-40 left open).
