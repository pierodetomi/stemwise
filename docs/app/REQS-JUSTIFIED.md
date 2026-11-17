# Requirements without a dedicated automated test

Every id in FOUNDATION §3 must have a test that cites it (§8.3) or an entry here, with the reason. `make reqs` reads
the lines of the form `- **ID** — reason`.

## Verified by a gate step, not by a test

- **NFR-6** — The license gate is a step of `make check`: `cargo deny check licenses` (allowed list in
  `app/deny.toml`) and `app/scripts/licenses-npm.mjs` (permissive production dependencies, no copyleft even among the
  development ones). A test that reran the same tools would add nothing.
- **NFR-9** — `app/scripts/disk.sh --check` is the last step of `make check` and fails above 20 GB; `make
  disk` reports the usage per folder.

## Verified by the benchmark (`make bench`, release profile)

- **NFR-1** — `app/crates/stemwise-engine/benches/engine.rs`: 32 stereo tracks at 48 kHz with a full chain (filters,
  4 EQ bands, compressor, saturation, sends to two returns, one group), blocks of 512 samples; fails if the average
  callback time exceeds 30% of the period. A debug test would measure nothing meaningful.
- **NFR-4** — same benchmark: offline render of 16 tracks of 60 s with a full chain; fails below 20× real time.

Note: the ANA-17 time (16 stems of 4 minutes in under 30 s) is measured by `crates/stemwise-ai/benches/analysis.rs`
(`make bench`); the test `cache_follows_clip_edits_not_parameters_and_progress_is_reported` verifies the parallelism
and the visible progress.
