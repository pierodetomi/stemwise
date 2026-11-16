# Stemwise product (FOUNDATION §8.2). Run from app/.
SHELL := /bin/bash
UI := ui
TAURI := $(CURDIR)/$(UI)/node_modules/.bin/tauri
export CARGO_TERM_COLOR := never

.PHONY: setup dev ui check test test-rust test-ui e2e bench reqs oracle schemas build disk clean-stale fmt lint licenses third-party ci

setup:
	rustup component add rustfmt clippy
	cargo fetch
	cd $(UI) && pnpm install --frozen-lockfile
	cd $(UI) && pnpm exec playwright install webkit

dev:
	cd src-tauri && $(TAURI) dev

ui:
	cd $(UI) && pnpm exec vite --open

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cd $(UI) && pnpm exec eslint .

licenses:
	cargo deny check licenses
	node scripts/licenses-npm.mjs
	node scripts/third-party.mjs --check

# Rewrites the notices bundled into the app (src-tauri/resources/THIRD-PARTY-LICENSES.txt) after a dependency change.
third-party:
	node scripts/third-party.mjs

test-rust:
	cargo test --workspace

test-ui:
	cd $(UI) && pnpm exec vitest run

e2e:
	cd $(UI) && pnpm exec playwright test

test: test-rust test-ui e2e

# Exported JSON Schemas (app/schemas) and TypeScript bindings (ui/src/bindings) from the Rust types.
schemas:
	cargo run -q -p stemwise-cli -- export-schemas --schemas schemas --bindings $(UI)/src/bindings

check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(MAKE) --no-print-directory licenses
	cd $(UI) && pnpm exec tsc -b --noEmit
	cd $(UI) && pnpm exec eslint .
	cd $(UI) && pnpm exec vitest run
	cd $(UI) && pnpm exec playwright test
	node scripts/reqs.mjs --closed
	bash scripts/disk.sh --check

# The gate as the hosted CI runs it (.github/workflows/check.yml): `check` without the frame-time tests, which need
# a real GPU, and without the disk budget of a working copy.
ci:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(MAKE) --no-print-directory licenses
	cd $(UI) && pnpm exec tsc -b --noEmit
	cd $(UI) && pnpm exec eslint .
	cd $(UI) && pnpm exec vitest run
	cd $(UI) && pnpm exec playwright test --project=webkit
	node scripts/reqs.mjs --closed

bench:
	cargo bench -p stemwise-engine --bench engine
	cargo bench -p stemwise-ai --bench analysis
	cargo bench -p stemwise-ai --bench project
	bash scripts/disk.sh

reqs:
	node scripts/reqs.mjs

oracle:
	cd ../poc && uv run python ../app/scripts/oracle.py

build:
	# CI=true: the bundler skips the AppleScript that arranges the .dmg window in the Finder (it times out without
	# a logged-in Finder session; A-046). The image is the same, without the custom window layout.
	cd src-tauri && CI=true $(TAURI) build
	# The bundle must carry a valid signature (ad hoc, `signingIdentity: "-"`): with an invalid one macOS calls a
	# downloaded app "damaged" and offers no way to open it (A-103).
	codesign --verify --deep --strict target/release/bundle/macos/Stemwise.app
	bash scripts/disk.sh

disk:
	bash scripts/disk.sh

clean-stale:
	bash scripts/clean-stale.sh
