# Stemwise: the app

This folder holds the product: a [Tauri 2](https://tauri.app) app with an engine in Rust and an interface in React
and TypeScript. What the app does, how to install it and the rules of the code are in the
[README at the root](../README.md); this page is the map for working here.

## Requirements

macOS 13 or later on Apple Silicon, the Xcode Command Line Tools, stable Rust, Node 22 or later with pnpm, and
`cargo-deny` for the licence gate. Regenerating the numeric oracle also needs the proof of concept in
[`../poc`](../poc/README.md) (Python, with uv).

## Commands

```bash
make setup        # toolchain components and dependencies
make dev          # the app, in development
make ui           # the interface alone in a browser, on the mock backend
make check        # fmt, Clippy, Rust tests, licences, types, lint, Vitest, Playwright, requirements, disk space
make test         # only the tests
make bench        # NFR-1, NFR-3 (import), NFR-4, NFR-5, ANA-17, in release mode
make build        # Stemwise.app and the .dmg, in target/release/bundle
make schemas      # JSON Schemas and TypeScript types, from the Rust types
make third-party  # the list of third-party licences that ships inside the app
make reqs         # requirements of the specification without a test or a justification
make oracle       # regenerates fixtures/oracle from the proof of concept
make disk         # disk space used by the repository; make clean-stale to clean up
```

`make check` must pass before every commit.

## Layout

| Folder | What is in it |
| --- | --- |
| `crates/stemwise-core` | The project document, commands with their inverse, undo histories, the project package, import. |
| `crates/stemwise-dsp` | The processors: filters, dynamics, saturation, reverbs, delays, limiter, loudness and true-peak meters. |
| `crates/stemwise-engine` | The audio graph, real-time and offline engine, the master chain, the mixdown. |
| `crates/stemwise-analysis` | The measurements for the assistant, in numeric parity with the proof of concept's oracle. |
| `crates/stemwise-ai` | The contracts of the plans, the stand-in decider, the OpenRouter client, the verify-and-correct loop. |
| `crates/stemwise-export` | FLAC encoder, AAC through the system, WAV with `bext`, tags, the export pipeline. |
| `crates/stemwise-cli` | Command-line tool: demo project, import, analysis, auto mix, schemas, offline render. |
| `src-tauri` | The Tauri shell: session, commands, the threads of long jobs. |
| `ui` | The interface (React 19, Zustand), the mock backend, the Vitest and Playwright tests. |
| `schemas`, `ui/src/bindings` | JSON Schemas and TypeScript types generated from the Rust types. Never edited by hand. |
| `fixtures/oracle` | Reference numbers computed by the proof of concept, which the Rust tests must match. |

## Conventions

- **Types are born in Rust.** The interface uses the generated types and holds no domain logic of its own.
- **Every test names the requirement it verifies**: `/// req: ENG-4` in Rust, `// req: UI-5` in TypeScript. The
  requirements are in [docs/FOUNDATION.md](../docs/FOUNDATION.md), section 3.
- **Tests use synthetic signals with a fixed seed** and temporary folders. No test uses the network or a real audio
  device, and no audio file is committed.
- **A processor without an oracle is documented** in [docs/app/dsp.md](../docs/app/dsp.md): algorithm, parameters,
  the properties that are tested.
- **A new dependency needs a permissive licence** (the gate checks it) and a run of `make third-party`.

## How it is verified

- **Rust tests**: numeric parity with the oracle, properties of commands and routing, no allocation in the audio
  callback, deterministic render.
- **Playwright** on the mock backend: mix, assistant and master flows, error states, accessibility (axe-core),
  60 fps with 32 tracks, plus a capture of every screen to compare with the boards in `design/screens`.
- **Self-test in the real app.** With `STEMWISE_SELFTEST_PROJECT` and `STEMWISE_SELFTEST_OUT` set, the interface
  runs the main flows on the real engine and audio device and writes a report.
- **What needs ears, a real key or a signature** is in [docs/app/VALIDATION.md](../docs/app/VALIDATION.md).

The specification, the decisions and the progress log are in [`../docs`](../docs/FOUNDATION.md), in Italian.
