# PoC progress

Status legend: ⬜ not started · 🟦 in progress · 🟨 in author review · ✅ approved

| Milestone | Requirements | Status | Report | Blockers |
| --- | --- | --- | --- | --- |
| Repository setup and agent workflow | — | ✅ | — | — |
| M0 Foundations | ING-1→6, REN-5 (trim), REN-7 | ✅ | [M0](milestones/M0.md) | — |
| M1 Analysis | ANA-1→13 | ✅ | [M1](milestones/M1.md) | — |
| M2 Render | REN-1→7 | ✅ | [M2](milestones/M2.md) | — |
| M3 Decision | DEC-1→8 | ✅ | [M3](milestones/M3.md) | — |
| M4 Verification loop | EVA-1→6, REP-1 | ✅ | [M4](milestones/M4.md) | H5: `mix` 0.11 USD measured, decision with the costs of `revise` (D-047) |
| M5 Revision in words | REV-1→3 | ✅ | [M5](milestones/M5.md) | H5: ~0.22 USD per track, accepted (D-068) |
| M6 Master | MAS-1→3 | ✅ | [M6](milestones/M6.md) | — |
| M7 Listening (reduced: PoC outcome) | ~~REP-2 + §10 protocol~~ (D-079) | ✅ | [M7](milestones/M7.md) | — |

Active milestone: **none**. PoC closed with outcome **go** (D-080, [README](../README.md#poc-outcome)).

Open questions and decisions: [DECISIONS.md](DECISIONS.md). Report per milestone: `docs/milestones/Mx.md` (template in
[milestones/_TEMPLATE.md](milestones/_TEMPLATE.md)).

## Acceptance criteria and expected manual validation

Each automatic criterion becomes a test `@pytest.mark.acceptance("Mx")`. The manual validation is what the author checks by ear
or by eye before approving; the exact instructions go in the report.

- **M0**: Auto: `stemwise mix <prog> --baseline-only` on synthetic stems writes `baseline.wav` within −3 dBTP; different
  durations ⇒ padding + warning; different sample rates ⇒ error; a run does not overwrite previous runs.
  Manual: `--baseline-only` on a real track in `local/`, listening to `baseline.wav` (no clipping, correct duration).
- **M1**: Auto: 1 kHz sine in the 500–2k band; pink noise with similar energy in bands of equal octave width; two identical
  stems ⇒ maximum masking; `analysis.json` of 12 stems < 8,000 estimated tokens.
  Manual: `stemwise analyze` on a real track, reading `analysis.json` for plausibility (vocal vs bass, etc.).
- **M2**: Auto: +6 dB of gain ⇒ +6 LU ±0.1; pan −1 zeroes the right channel; the high-pass attenuates below the cutoff; two
  renders of the same plan ⇒ same hash. Manual: render of a hand-written plan on a real track, A/B listening with
  `baseline.wav`.
- **M3**: Auto: full flow offline with the fake decider; invalid plans ⇒ retry up to 2, then a clear error; non-JSON responses
  handled. Manual (network, costs money): one real call, valid plan, complete logs, reading the reasons.
- **M4**: Auto: the report compares the raw-sum distance with each iteration; the best plan has the minimum distance among the
  valid ones; `report.md` generated. Manual: reading the report of a real run.
- **M5**: Auto: "voce più avanti" ("vocal further forward") with the fake decider ⇒ diff on the parameters of the vocal or of
  the stems that cover it. Manual: two real requests, listening, is the direction the one requested?
- **M6**: Auto: `master.wav` produced; `master_report.json` with LUFS, true peak, distance from the reference.
  Manual: listening to the master against the reference.
- **M7**: Reduced (D-079): no `ab` and no §10.2 protocol. Originally planned: Auto, `ab` produces files of equal duration and
  loudness with random names and a separate key; Manual, §10.2 protocol on 3 tracks and a completed sheet. Still open:
  go/no-go noted in the README, with the data from M3–M6.
