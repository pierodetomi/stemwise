# CLAUDE.md — stemwise

PoC of a Python engine, driven from the CLI, in which a language model decides the mix parameters starting **only from
numeric measurements** of the stems. The complete specification is in [docs/PRD.md](docs/PRD.md): it is the source of truth
for requirements, data contracts, milestones and acceptance criteria. In case of conflict between this file and the PRD,
the PRD wins; report the conflict to the author.

Package and command name: **`stemwise`** (D-006). Where the PRD writes `mixpoc` (provisional name), read `stemwise`: module
`src/stemwise/`, command `stemwise <analyze|mix|revise|master|report|ab>`.

Current milestone status (always read it at the start of a session):

@docs/PROGRESS.md

## Non-negotiable principles (PRD §1, §7)

- **The model never receives audio.** Only measurements, stem ids and roles, and the brief go to the model.
- **The mix plan is the only source of truth.** Same plan + same stems ⇒ same file (same hash).
- **Every choice is explained** (`rationale` per stem, `summary` for the plan).
- **Pure functions**: analysis, render and evaluation without global state, testable with numpy arrays in memory.
- **Engine without a terminal**: no `input()` and no interactive prompts; the CLI (`typer`) is a thin wrapper over functions
  that accept paths and return data.
- **Neutral contracts**: the exchange JSON contains no library names and no serialized objects. The schemas originate as
  Pydantic models and are exported to `schemas/` on every change.
- **Explicit formulas**: every analysis measure is documented in `docs/analysis.md` (formula, window, STFT size/hop, band
  limits, thresholds, algorithm version).
- **Determinism**: no unseeded randomness, no dependence on the clock in the content of audio files.
- **Technical constraints always valid**: mix ≤ −3 dBTP without a limiter; deterministic render.
- **Language**: code, comments, docstrings and CLI messages in **English**. The `rationale` field in the language of the brief.
  Process documentation (`docs/`, milestone reports) and communication with the author in **Italian** (the documents
  were translated to English after the repository became public, A-106).
- **Secrets**: the key is in `OPENROUTER_API_KEY` (env / `.env`, never committed, never read by the agent).
- A run never modifies `stems/` and `references/` and never overwrites previous runs.

## Working rules (PRD §9)

1. **One milestone at a time**, in the order of `docs/PROGRESS.md`. Do not start the next one without the author's explicit
   approval.
2. **No out-of-scope work** (right column of PRD §3), not even "while we're at it". If needed, note it in the milestone
   report under "Flagged out of scope" and proceed without implementing it.
3. **Ambiguity ⇒ stop and ask.** If a requirement is ambiguous or a library does not do what is needed, do not work around
   the problem: record the question in `docs/DECISIONS.md` (section "Open questions") and ask.
4. **No test calls the network.** `tests/conftest.py` blocks sockets; tests with the real model have `@pytest.mark.llm` and
   do not run by default.
5. **No audio file in the repository.** Tests generate synthetic signals (sines, pink noise, impulses, silence). The author's
   real material stays in `local/` (ignored by git) or outside the repo.
6. **Dependencies**: only the stack of PRD §7. A new library requires the author's OK (`uv add` is not pre-authorized).

## Milestone flow

The operational detail is in the commands `/milestone-start` and `/milestone-close` (`.claude/commands/`).

1. **Start**: `/milestone-start Mx`: branch `milestone/mx-<slug>`, reading of the relevant PRD sections, plan in
   `docs/milestones/Mx.md` (requirements → planned tests, steps, ambiguities). **Checkpoint**: present the plan and wait for
   the author's OK (or the answers to the open questions) before writing code.
2. **Development in small steps**: for each step, tests first (known expected values + explicit tolerances), implementation,
   `make check` green, commit. Never commit with red tests.
3. **Close**: `/milestone-close`: `make check`, `make reqs M=Mx`, independent review with the subagent `milestone-reviewer`,
   closing report in `docs/milestones/Mx.md` (done, decided, doubts, out of scope, **manual validation instructions**),
   update of `docs/PROGRESS.md`. Commit. **Stop**.
4. **Author review**: the author runs the manual validation and records the outcome in the report. Only after an explicit
   approval in chat: `git merge --no-ff` into `main` and tag `mx`.

### Definition of Done of a milestone

- All requirements of the milestone implemented, each with at least one test marked `@pytest.mark.req("ID")` (or a
  justification written in the report, e.g. verifiable only with the real model).
- Each acceptance criterion of the PRD covered by a test `@pytest.mark.acceptance("Mx")`.
- `make check` green (format, lint, tests). `make reqs M=Mx` without unjustified gaps.
- `schemas/` and `docs/analysis.md` updated if contracts or measures were touched.
- Complete milestone report, with manual validation instructions executable by copying the commands.

## Commands

```bash
make setup            # uv sync (creates .venv with the dev dependencies)
make fmt              # ruff format + automatic fixes
make check            # format check + lint + test (mandatory gate before each commit)
make test             # pytest only (excludes the llm-marked tests)
make test-llm         # tests with the real model: requires OPENROUTER_API_KEY, uses the network, costs money
make reqs M=M1        # coverage of the PRD requirements of the milestone in the tests
uv run pytest tests/test_x.py -k nome   # single test
```

## Test conventions

- One test file per module (`tests/analysis/test_stem.py` ↔ `src/stemwise/analysis/stem.py`).
- Mandatory markers: `@pytest.mark.req("ANA-3")` links the test to the requirement (also several ids);
  `@pytest.mark.acceptance("M1")` for the acceptance criteria of the milestone.
- Tolerances always explicit and justified (`np.testing.assert_allclose(..., atol=0.1)  # ±0.1 LU from PRD M2`).
- Synthetic signals and test projects generated in `tmp_path`, with fixed seeds. Shared helpers in `tests/helpers/`
  (signal generators, synthetic project).
- The fake decider (`decision/mock.py`) is the only decider used in the full-flow tests.

## Architecture (PRD §4, §8)

Loop: **analysis → decision → render → evaluation** (up to 3 iterations), then the master from the best plan. Module structure
in `src/stemwise/` as in PRD §8; each module reads/writes JSON and can be run on its own. The code depends on the `Decider`
interface (implementations: LLM via OpenRouter, deterministic mock).

## Git

- `main` contains only approved milestones. Work happens on `milestone/mx-<slug>`.
- Small commits, message in English, prefixed with the milestone: `M1: add band energy measure (ANA-3)`.
- Never `push`, `merge`, `reset --hard`, or `rebase` without an explicit request from the author.
