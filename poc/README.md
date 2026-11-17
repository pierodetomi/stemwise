# stemwise

> **In English.** This is the proof of concept that came before the Stemwise app: a Python command-line tool that
> tested whether a language model can make good mixing decisions from measurements alone. It ended with a "go" and
> is now frozen: it is kept as an archive and as the numeric oracle for the app's tests. Its documents are in
> Italian. It is licensed under [GPL-3.0-or-later](LICENSE), unlike the rest of the repository, which is MIT. The
> app is in [`../app`](../app/README.md).

Proof of concept of an AI-assisted mixing and mastering engine: a language model receives **only numeric
measurements** of the stems (never audio) and returns a mix plan in JSON, which the engine applies locally. Goal: a
go/no-go decision on building the full tool.

> **The product.** The PoC closed with a go. The product, Stemwise (a desktop DAW for AI-assisted mixing and
> mastering), is in [app/](../app/README.md): specification in [docs/FOUNDATION.md](../docs/FOUNDATION.md), progress in
> [docs/app/PROGRESS.md](../docs/app/PROGRESS.md). What follows describes the PoC, which is now frozen.

- Specification: [docs/PRD.md](docs/PRD.md)
- Progress: [docs/PROGRESS.md](docs/PROGRESS.md)
- Decisions and open questions: [docs/DECISIONS.md](docs/DECISIONS.md)
- Instructions for the agent: [POC-RULES.md](POC-RULES.md)

## Requirements

- macOS or Linux, Python 3.11–3.13, [uv](https://docs.astral.sh/uv/)
- Only for the real decider: an OpenRouter key in `OPENROUTER_API_KEY` (see `.env.example`)

## Setup

```bash
make setup
make check
```

## Usage

A track is a folder with `project.yaml` (format in [PRD §6.1](docs/PRD.md); schema in
[schemas/project.schema.json](schemas/project.schema.json)), the stems and any references. WAV stems (PCM 16/24, float 32)
or FLAC (16/24), mono or stereo, all at the same sample rate; `sample_rate: auto` derives it from the stems.

```bash
uv run stemwise mix <cartella-del-brano> --baseline-only
```

Writes the raw sum of the stems (`baseline.wav`, 24-bit stereo WAV, true peak ≤ −3 dBTP) into a new folder
`runs/<data-ora>/` of the track.

```bash
uv run stemwise analyze <cartella-del-brano>
```

Measures the stems, the raw sum and the references, and writes `analysis.json` into a new folder `runs/<data-ora>/`:
loudness, peaks, dynamics, energy in 8 bands, centroid, transients, stereo, hygiene for each stem; the same measures on
the sum; band shares, the 10 strongest masking collisions and the average profile of the references. Contract in
[schemas/analysis.schema.json](schemas/analysis.schema.json), formulas in [docs/analysis.md](docs/analysis.md).

```bash
uv run stemwise mix <cartella-del-brano> [--decider llm|mock] [--max-iter 1-3] [--model <id>] \
    [--temperature <t>] [--reasoning-effort low|medium|high|xhigh] [--export-stems]
```

The verification loop: analysis and raw sum, then up to 3 decision iterations, render and evaluation (`--max-iter` takes
precedence over `options.max_iterations` in `project.yaml`). Writes into a new folder `runs/<data-ora>/`:

- `analysis.json`, `baseline.wav` and `baseline_evaluation.json` (the raw sum evaluated as a comparison term);
- for each iteration, `iter_NN/` with `plan.json`, `mix.wav`, `evaluation.json` (spectral distance from the reference
  profile, per-band, stereo-width and dynamics deviations, residual collisions, technical constraints, measures of the
  mix), `llm_request.json` (everything that was sent, attempt by attempt) and `llm_response.json` (responses, tokens,
  cost, duration);
- `best.json` (the best iteration: minimum distance among those that meet the constraints) and `report.md` (table of the
  raw sum and of each iteration, reasons for the best plan, collisions, cost).

The loop stops when the distance falls below 1.5 dB, when it improves by less than 0.2 dB over the previous iteration, or
at the maximum number of iterations; without references, at the first iteration that meets the constraints. From the
second iteration the model receives the last 2 iterations with their evaluations; gain and EQ bands cannot change by more
than ±4 and ±3 dB from one iteration to the next. The model receives only measurements, ids, roles and notes of the stems,
and the brief: no audio, file names or title. If a plan is invalid, it is sent back to the model with the errors up to 2
times; if it also fails that way at the first iteration, the command exits with an error and the run folder keeps the
logs; at a later iteration the loop stops and keeps the result. If no iteration meets the technical constraints, the
command exits with an error (the files remain). Contracts in [schemas/evaluation.schema.json](schemas/evaluation.schema.json)
and [schemas/best.schema.json](schemas/best.schema.json), formulas in [docs/analysis.md](docs/analysis.md), section
"Evaluation".

- `--decider llm` (default): the real model via OpenRouter. Configuration from environment variables or from `.env` in the
  current folder (model in `.env.example`): `OPENROUTER_API_KEY`, `STEMWISE_MODEL` (required, no default model in the
  code), `STEMWISE_REASONING_EFFORT`, `STEMWISE_TEMPERATURE` (default 0.2), `STEMWISE_BASE_URL`. Command-line options take
  precedence. Temperature, seed and reasoning effort are sent only if the model accepts them (otherwise a warning).
- `--decider mock`: fixed rules per role, no network, deterministic; from the second iteration it corrects, with an EQ, the
  band furthest from the target. It still writes `llm_request.json` with the messages the model would receive, so the
  prompt can be read for free.

```bash
uv run stemwise mix <cartella-del-brano> --plan <mix_plan.json> [--export-stems]
```

Applies and evaluates a hand-written mix plan, without the model (format in [PRD §6.3](docs/PRD.md); schema in
[schemas/mix_plan.schema.json](schemas/mix_plan.schema.json)): the same files as `mix`, with a single iteration `iter_00/`.
With `--export-stems`, also the processed stems in `iter_NN/stems/` and the reverb return in `iter_NN/reverb_return.wav`.
Chain and formulas in [docs/analysis.md](docs/analysis.md), section "Render".

```bash
uv run stemwise report <cartella-del-brano> [--run <cartella-del-run>]
```

Regenerates `report.md` from the run files alone, including those of a revision: the last run with `best.json`, or the one
indicated (the name of the folder in `runs/` or a path).

```bash
uv run stemwise revise <cartella-del-brano> "<richiesta>" [--run <cartella-del-run>] [--decider llm|mock] \
    [--model <id>] [--temperature <t>] [--reasoning-effort low|medium|high|xhigh] [--export-stems]
```

A revision in words (for example `"voce più avanti, meno riverbero sulle chitarre"`, from 1 to 1,000 characters): a single
iteration starting from the best plan of the last run (the one in `best.json`; after a `mix`, the best plan of that mix)
or of the run indicated with `--run`, with the request as the priority. Writes a new run and does not touch the starting
one:

- `revision.json` (starting run and plan, request), `source_plan.json` and `source_evaluation.json` (copies of the starting
  plan and of its evaluation);
- `analysis.json`, `baseline.wav` and `baseline_evaluation.json` copied from the starting run: the analysis is not redone,
  only the references are re-measured. If in the meantime stems or references have changed (ids, roles, channels,
  duration, peak, reference profile), the command stops and asks for a new `mix`. The `options` of `project.yaml`
  (`pre_normalize`, `mix_true_peak_db`) are not saved in the run and are not checked: if you change them, redo `mix` before
  revising, otherwise the diff does not describe the whole change;
- `iter_00/` as for `mix`, and `best.json` and `report.md` with the comparison raw sum · starting plan · revised plan, the
  **parameter diff** and the reasons for the changed stems.

The model receives the starting plan with its evaluation and the request; the variation limits (±4 dB of gain, ±3 dB per EQ
band) apply relative to the starting plan, so a large change may need two revisions. There is no rollback: the revised plan
is the "best" of the new run whatever its distance from the reference, provided it meets the technical constraints; if it
violates one, the command exits with an error and the files remain for listening. Since the revision becomes the last run,
a `revise` after another one starts from the already revised plan; to start from an earlier plan, use `--run`. With the fake
decider, the request is interpreted through a few keywords (subject: voce [vocal], basso [bass], chitarre [guitars],
batteria [drums]…; direction: più avanti/indietro [further forward/back], alza/abbassa [raise/lower], più/meno riverbero
[more/less reverb]). Contract in [schemas/revision.schema.json](schemas/revision.schema.json).

```bash
uv run stemwise master <cartella-del-brano> [--run <cartella-del-run>] [--mix <file>] \
    [--reference ref1|ref2|ref3|<file>] [--pcm16]
```

The master for comparison with a reference (mandatory). The mix is brought to the loudness and average spectrum of the
reference with Matchering, then passes through its limiter. It does not use the model. By default it takes the `mix.wav` of
the best plan of the last run (including a revision) and the first reference of `project.yaml`; `--run` chooses another
run, `--reference` another reference (`ref2`, `ref3` or a file), and `--mix` any file (for example a mix you made by hand) at
the project's sample rate. Writes:

- `master/master.wav` (24-bit WAV, at the project's sample rate) inside the run; a second master of the same run goes into
  `master_02/`, then `master_03/`…: no existing file changes. With `--mix`, a new run is created that contains only
  `master/` and that does not become the "last run" for `report` and `revise`;
- with `--pcm16`, also `master_16.wav` (16 bit, no dither);
- `master_report.json`: loudness and true peak of the master, of the starting mix and of the reference, and the spectral
  distance from the reference used before and after the master. Contract in
  [schemas/master_report.schema.json](schemas/master_report.schema.json).

If the true peak exceeds −1 dBTP, the command flags it with a warning but does not correct it: Matchering's limiter works on
the sample peak, so with strong commercial references the warning is normal. The master serves to listen to the mix against
the reference at similar level and tonality; it is not a final master.

The `ab` command (blind listening, REP-2) has not been implemented: the author closed the PoC without the blind listening
protocol (D-079). The command exists and replies "not implemented".

## Development with Claude Code

Development proceeds by milestone (PRD §9), one at a time, each with two checkpoints with the author:

1. `/milestone-start M0`: the agent reads the PRD, writes the plan in `docs/milestones/M0.md` and stops for the OK and for
   the open questions.
2. Development in small steps, each with tests and `make check` green.
3. `/milestone-close`: automated checks, independent review (subagent `milestone-reviewer`), report with instructions for
   manual validation. The agent stops.
4. The author validates, records the outcome in the report and approves: merge into `main` and the milestone tag.

It is advisable to use a new Claude Code session for each milestone: the state lives in the files under `docs/`, not in the
chat.

The real audio material for manual validation goes in `local/` (ignored by git).

## Licenses

The PoC is distributed under [GPL-3.0-or-later](LICENSE), because it depends on `pedalboard` and `matchering`, which are
GPLv3. The rest of the repository, product included, is under the MIT license and uses neither the PoC nor these two
libraries: from the PoC it takes only the oracle numbers (`app/fixtures/oracle`), computed with NumPy, SciPy and librosa.

## PoC outcome

**Decision: go** (2026-11-10, D-080). Development of the product proceeds.

PoC question: can a model that receives only numeric measurements of the stems decide mix parameters that sound better than
the raw sum? On the material tested, yes. The default model is `openai/gpt-6.1-sol` at medium reasoning effort (D-047).

### Hypotheses (PRD §2)

| Hypothesis | Threshold | Measured | Outcome |
| --- | --- | --- | --- |
| H1. Measurements are enough for the model to decide | 100% of valid plans within 2 new attempts | 8 real calls on "Confidence" (13 stems, 3 models) and the synthetic `llm` tests: all valid on the first attempt | ✅ |
| H2. The verification loop converges | Distance decreasing between the first and the best iteration on ≥ 2 tracks out of 3 | "Confidence": Sol 2.1 → 0.7 dB, Terra 1.7 → 1.2 dB (raw sum 3.7 dB) | ✅ on 1 track |
| H3. The result sounds better than the raw sum | AI mix preferred on ≥ 2 tracks out of 3, blind listening at loudness-matched levels | "Confidence": the best mix stands out better than the raw sum and the first iteration; master judged without problems | ✅ according to the author, non-blind listening |
| H4. Requests in words produce the requested change | ≥ 4 out of 5 in the right direction | 3 out of 3 (one synthetic, two real on "Confidence") | ✅ on 3 requests |
| H5. The cost is negligible | < 0.20 USD per track | ~0.22 USD: `mix` at 2 iterations 0.11, two `revise` 0.06 + 0.05 | ✅ with the deviation accepted (D-068) |

Technical constraints: the mixes stay within −3 dBTP without a limiter, and the render is deterministic (same plan and same
stems ⇒ same hash); both are covered by automated tests.

### Limits of the verification

The PoC closed without the listening protocol of PRD §10.2 (D-079): one track instead of three, non-blind listening and not
loudness-matched, 3 requests in words instead of 5. The level imbalance worked against the AI mix (the raw sum was about
4 dB louder, −17.3 against −21.2 LUFS), and the AI mix was still preferred. The comparison with a mix made by hand by the
author was not done.

### Guidance for the product

- **Parameters per section (automation): must-have** (D-048). With static parameters, the distorted guitars of the chorus of
  "Confidence" remain barely audible. Together with the sections, it is advisable to give the model compact measures over
  time, for example the loudness of each stem per section (never audio).
- **Master with parameters chosen and justified by the model** (D-077): how much to imitate the reference or how to weigh
  more than one, loudness and true peak according to the destination, width and compression according to genre and mood.
  Expected value lower than in the mix. Matchering's limiter looks at the sample peak, so the master often exceeds −1 dBTP
  (D-074).
- **Spectral distance does not measure quality**: the fake decider obtains smaller distances than the model's first plan. It
  is fine as a guide for the loop, not as a judgment.
- **Drums in many stems**: the strongest collisions are almost always between the microphones of the same drum kit, and they
  cover the useful ones. A grouping by instrument family is needed in the measures and in the prompt.
- **Compressor per channel** (D-034): on unbalanced stereo stems the image can shift. A linked compressor is needed, outside
  `pedalboard`.
- **Compute time**: 3–7 minutes for a `mix` of 13 stems, of which ~1.5 are model calls. To optimize (spectra computed only
  once, parallelism).
- **Small things**: the warning about an unsupported `temperature` repeats on each call; there is no fallback if a provider
  rejects the `strict` schema (Q-40); an AI mix should be compared with a manual mix, at loudness-matched levels.
- **Portability** (PRD §7.1): the contracts in `schemas/` and the formulas in `docs/analysis.md` survive a change of language;
  a port of the render chain must be re-validated by ear.
