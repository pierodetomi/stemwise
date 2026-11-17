# CLAUDE.md — stemwise

Stemwise is a desktop DAW for mixing and mastering already-recorded songs with the help of an AI model
(Tauri 2 + Rust engine + React/TypeScript UI, in `app/`). The specification is
[docs/FOUNDATION.md](docs/FOUNDATION.md): **it is the source of truth** for requirements, architecture, stack, phases
and rules. Read it in full at the start of each session. In case of conflict: FOUNDATION wins over this file, which
wins over the design (`design/`), which wins over the PoC PRD.

Current state (always read it at the start of a session):

@docs/app/PROGRESS.md

Decisions made autonomously and caveats: [docs/app/DECISIONS.md](docs/app/DECISIONS.md).

## The repository is public

The code is under the MIT license ([LICENSE](LICENSE)); the PoC in `poc/` is under GPL-3.0-or-later because it uses
two GPL libraries; the Stemwise name and logo are reserved to the author ([TRADEMARKS.md](TRADEMARKS.md)). Anything
that goes into a commit can be read by anyone: no personal data, paths from the author's machine, names of real
files or songs, server addresses, recorded model responses on real material. The libraries shipped in the app are
listed in `app/src-tauri/resources/THIRD-PARTY-LICENSES.txt`: after every dependency change, regenerate it with
`make third-party` (the license gate checks it).

## Language: English everywhere

Everything written into the repository is in **English**: code, comments, messages, the interface, documents
(`docs/`, `design/`, `poc/`), decisions, progress notes, validation steps, agent and command definitions, commit
messages, tags, issues and pull requests. This applies to new text and to every edit of existing text (A-105). The
only exception is product data: the model's `rationale` and `summary` are written in the language of the brief, and
test fixtures may contain other languages on purpose. Conversation with the author in chat follows the author's
language; anything that lands in a file does not.

## The PoC is frozen

The repository also contains the Python PoC, closed with a go outcome, confined to `poc/` (code in
`poc/src/stemwise/`, `poc/tests/`, `poc/schemas/`, with its own `pyproject.toml` and `Makefile`; documents in
`poc/docs/`: `PRD.md`, `analysis.md`, `DECISIONS.md`, `PROGRESS.md`, `milestones/`). It is **read-only**: it serves
as an archive and as a numeric oracle (FOUNDATION §5.7). It is not modified (its documents were translated to English
once, A-105); it is run only to generate the oracle fixtures. Its working rules are archived in
[poc/POC-RULES.md](poc/POC-RULES.md) and **do not apply to the product**: in particular "one milestone at a time with
approval", "ambiguity ⇒ stop and ask" and the `/milestone-start` and `/milestone-close` commands do not apply.

## Non-negotiable principles (FOUNDATION §1)

- **The model never receives audio**: only measurements, roles, track names, brief, requests. Never paths, file
  names or the song title.
- **The project document is the single source of truth**: same project ⇒ same offline render (same hash). The AI
  produces the same edits the user could make by hand.
- **Every AI choice is explained and reversible**: an applied proposal is a single undo step.
- **Nothing is destructive**: imported files are immutable; every mutation is a command with its inverse.
- **Mix and master are two distinct phases**, with separate workspaces and histories.
- **Audio thread**: no allocation, no locks, no I/O. Playback and offline render use the same code.
- **Permissive licenses only** (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode). No GPL/LGPL/AGPL
  dependencies: the PoC's `pedalboard` and `matchering` are not used.
- **Secrets**: the agent does not read `.env`, does not ask for or use keys, does not call real models. The AI is
  developed with the fake decider and a fake local HTTP server.
- **Language**: English everywhere in the repository (see above); `rationale` and `summary` in the language of the
  brief.

## Way of working: autonomous session (FOUNDATION §9)

1. **Do not stop and do not ask questions** after the preliminary check, until the last open phase is closed. The
   phases of FOUNDATION §8.1 are executed in order, one after the other, without waiting for approvals. Phases that
   start from design boards stop for the author's approval.
2. **Ambiguity ⇒ decide and record.** Pick the option most consistent with FOUNDATION (principles, then
   requirements, then architecture, then design), record it in `docs/app/DECISIONS.md` (`A-nnn`: context,
   alternatives, choice, reason) and go on.
3. **Obstacle ⇒ up to three different approaches**, then a fallback that keeps the requirement, or the requirement
   reduced to its useful minimum and recorded among the "Caveats" of `docs/app/PROGRESS.md`. Never weaken a test to
   pass a gate.
4. **You stop only for**: a destructive or irreversible action outside the repository, the need for a secret, a
   conflict with the principles that no alternative resolves.
5. **Nothing out of scope** (FOUNDATION §2, right column): record it under "Flagged out of scope" in
   `docs/app/DECISIONS.md` and go on.
6. **New dependencies** are allowed without asking if the license is permissive and the library is maintained; each
   one is recorded in `docs/app/DECISIONS.md`. The license gate is part of `make check`.
7. **State lives in files.** At every closed phase update `docs/app/PROGRESS.md`, commit and tag. After a context
   compaction re-read FOUNDATION, PROGRESS and DECISIONS and resume from the first unfinished step.
8. **Do not declare done what you have not run.** What you cannot verify (listening, real model, signing) goes into
   `docs/app/VALIDATION.md` with the steps for the author.

## Coordination and subagents

The main agent is the orchestrator: it owns architecture, contracts between modules, integration, gates and
decisions, and organizes the work as it sees fit.

- **Delegate to cut costs, not on principle.** Delegate to the `app-implementer` subagent (cheaper model) a task that
  is at once **bounded** (one module or a few files), **specified** by an already written contract (signature,
  requirements, expected tests or fixtures) and **verifiable** by a command.
- **Do not delegate** when: the brief would cost as much as the work; architecture decisions or a lot of context are
  needed; the task touches files others are editing; it is integration between modules; a few tool calls suffice.
  For broad searches use the read-only `Explore` subagent.
- **Parallel only on disjoint files.** Every brief names files, contract, verification command and what must not be
  touched.
- **A subagent's result is not proof**: re-run the verification command and read the diff before integrating. After
  two failures on the same task, the work goes back to the orchestrator.
- **Independent review** with `app-reviewer` only at the close of F3, F5, F8, F10 and F12.

## Disk space (FOUNDATION §9, rules 18–20)

The repository, artifacts included, stays under **20 GB**; above **10 GB** it is cleaned.

- **A single build folder** (`app/target`) for orchestrator, subagents and Tauri: never an alternative
  `CARGO_TARGET_DIR`, never copies of the repository or worktrees with their own `target`. A single `node_modules`.
- **Measure** with `make disk` at every phase close, after every `make build` or `make bench`, and at least every two
  hours; record the value in `docs/app/PROGRESS.md`.
- **Clean** with `make clean-stale` above 10 GB (incremental caches, unused profiles, old packages and reports,
  leftover temporary folders, worktrees of already integrated subagents); full `cargo clean` at the ceiling.
- Tests delete their own temporary folders even when they fail; no rendered audio stays on disk.
- Cleaning touches **only git-ignored files inside the repository** and the tests' temporary folders: never tracked
  files, never `local/`, never anything outside the repository. It is pre-authorized.

## Commands (in `app/`, created in F0)

```bash
make setup     # toolchain and dependencies
make dev       # Tauri app in development
make ui        # UI in the browser with the fake backend
make check     # fmt, clippy -D warnings, Rust tests, licenses, types, lint, vitest, Playwright, reqs
make test      # tests only
make bench     # benchmarks (NFR-1, NFR-4)
make reqs      # requirements of FOUNDATION §3 without tests
make oracle    # regenerate app/fixtures/oracle from the PoC
make build     # system package: .app/.dmg on macOS, NSIS installer on Windows
make ci        # the gate as GitHub CI runs it: check without frame-time tests and without the disk ceiling
make third-party  # regenerate the third-party license list shipped in the app
make disk      # repository usage per folder
make clean-stale  # delete artifacts that are no longer needed
```

A green `make check` is mandatory before every commit.

## Conventions

- **Tests**: every test cites the requirements it verifies (`/// req: ENG-4` in Rust, `// req: UI-5` in
  TypeScript). Explicit, justified tolerances. Synthetic signals with a fixed seed, test projects in temporary
  folders. No test uses the network or the real audio device. No audio files in the repository.
- **Contracts**: types are born in Rust; JSON schemas exported to `app/schemas/` and TypeScript types generated on
  every change. The UI does not duplicate domain logic.
- **DSP**: every processor without an oracle is documented in `docs/app/dsp.md` (algorithm, parameters, verified
  properties).
- **Git**: work happens on the `product/v1` branch; `main` is the public branch and moves forward only at the
  author's request; small commits in English with the phase prefix (`F3: add automation ramps (AUT-4)`); a local tag
  per closed phase (`f3`). Never `push`, `merge` into `main`, `reset --hard`, `rebase` without the author's explicit
  request.
