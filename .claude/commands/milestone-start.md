---
description: Start a PoC milestone (plan + checkpoint with the author, no code)
argument-hint: <M0..M7>
---

Start the milestone **$ARGUMENTS**. In this phase you **do not write production code**: you produce the plan and stop for
the author's approval.

1. **Preconditions.** Read `poc/docs/PROGRESS.md`. Verify that all previous milestones are ✅ and that none is 🟦. Verify
   that `git status` is clean. If a condition does not hold, stop and explain.
2. **Reading the specification.** Read in `poc/docs/PRD.md`: §1–§3 (principles and scope), the section with the requirements of
   the milestone (§5.x), the data contracts involved (§6), §7, the milestone entry in §9 and §10.1. Read `poc/docs/DECISIONS.md`
   and the reports of the previous milestones in `poc/docs/milestones/` (especially "Doubts and risks" and "Author review
   outcome").
3. **Blocking questions.** Check in DECISIONS.md the open questions that block $ARGUMENTS. List each new ambiguity found
   (a requirement that can be read in more than one way, a library that may not do what is needed, a conflict with the scope).
   Add them as `Q-n` in DECISIONS.md with the possible alternatives.
4. **Branch.** `git switch -c milestone/<mx>-<slug>` (e.g. `milestone/m1-analysis`).
5. **Plan.** Create `poc/docs/milestones/$ARGUMENTS.md` from the template `poc/docs/milestones/_TEMPLATE.md` and fill in only the
   "Plan" section:
   - table of requirements → planned tests, with explicit and justified tolerances;
   - table of acceptance criteria → `acceptance` tests;
   - small, ordered steps, each one closable with `make -C poc check` green and a commit;
   - any new dependencies (only the PRD §7 stack) and documents to update (`poc/docs/analysis.md`, `poc/schemas/`).
6. Update `poc/docs/PROGRESS.md`: status 🟦, link to the report, active milestone.
7. Commit: `$ARGUMENTS: plan`.
8. **Checkpoint.** Present to the author, concisely: steps, key tests, open questions with the alternatives and
   your recommendation. **Stop and wait** for the OK or the answers. Do not start the first step in the same turn.

After the OK: carry out the steps one at a time (tests first, implementation, `make -C poc check` green, commit
`$ARGUMENTS: <what> (<requirement IDs>)`). If during development a new ambiguity emerges, record it and ask instead of working
around it. When all the steps are closed, use `/milestone-close`.
