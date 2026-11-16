# Contributing to Stemwise

Thank you for taking the time. This page says what to expect and how to make a contribution easy to accept.

## Before anything else

Stemwise is an independent, non-commercial project that one person maintains in his spare time. Issues and pull
requests are read with care, but replies can take a while, and not every proposal can be taken on. A slow answer is
not a lack of interest.

## Reporting a bug

Open an [issue](https://github.com/pierodetomi/stemwise/issues/new/choose) with the bug template. A report is most
useful when it says:

- what you did, step by step, and what you expected instead;
- the version of Stemwise (in the app's menu, **Stemwise › About**, or the name of the `.dmg`) and of macOS;
- whether it happens every time;
- a screenshot, when the problem is something you see.

Please **do not attach audio you are not free to share**, and never post an API key. For a problem with the
assistant, the request and the response of the call are in **AI usage › Details**: they contain measurements and
track names, no audio. Read them before pasting, and remove what you prefer to keep private.

Security problems do not go in a public issue: see [SECURITY.md](SECURITY.md).

## Proposing a feature

Open an issue with the feature template and describe the problem before the solution: what you were trying to do
and where the app got in the way. The list of what version 1 deliberately leaves out is in the
[README](README.md#what-it-does-not-do); a request in that list is not unwelcome, but it is unlikely to be picked
up soon.

## Sending code

1. **For anything larger than a fix, open an issue first.** It avoids work on something that would not be merged.
2. Fork the repository and branch from `main`.
3. Set up and run the app as the [README](README.md#build-from-source) describes. `make ui` runs the interface in
   a browser on a mock backend, with no Rust build: it is the quickest way into the UI code.
4. Make the change, with its tests.
5. Run `make check` from `app/`. It must pass.
6. Open a pull request that says what changes and why, and how you verified it.

### What a change must respect

These rules hold the app together. A pull request that breaks one will not be merged, however good the rest.

- **The model never receives audio**, file names, paths or the song's title.
- **The project document is the only source of truth**: the same project always renders to the same audio.
- **Nothing is destructive**: imported files are never modified, and every change is a command with its inverse.
- **The audio thread never allocates, locks or does I/O.**
- **Permissive licences only** for dependencies (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 and the like). No GPL,
  LGPL or AGPL. After adding or updating a dependency, run `make third-party` and commit the result.
- **No secrets and no real model calls in tests.** Tests use the built-in stand-in decider and a fake local
  server; none of them touches the network or a real audio device.
- **No audio files in the repository.** Tests synthesise their signals, with a fixed seed.

### Conventions

- Code, comments, commit messages and the interface are in English.
- Types are defined in Rust; the JSON Schemas and the TypeScript types are generated with `make schemas`, never
  edited by hand. The interface holds no domain logic of its own.
- Every test names the requirement it verifies (`/// req: ENG-4` in Rust, `// req: UI-5` in TypeScript). The
  requirements are numbered in [docs/FOUNDATION.md](docs/FOUNDATION.md), section 3. That document is in Italian:
  if you cannot find the right one, say so in the pull request and it will be sorted out in review.
- Never weaken a test to make it pass. If a test is wrong, say why in the pull request.
- Small commits, each with a message that says what changed.

### About the Italian documents

The specification and the project log in `docs/` are in Italian, the author's language. You do not need to read
them to fix a bug, and you are not expected to write in Italian. If a change needs the specification updated, the
maintainer will do that part.

## Licence of contributions

By sending a contribution you agree that it is released under the [MIT licence](LICENSE), like the rest of the
code. The Stemwise name and logo are not part of that licence: see [TRADEMARKS.md](TRADEMARKS.md).

## Conduct

Be kind and be specific. Criticise the code, not the person. Harassment and personal attacks are not accepted, and
the maintainer may remove comments or block accounts that bring them.
