---
description: Close the active milestone (checks, independent review, report, stop for the author)
---

Close the active milestone indicated in `poc/docs/PROGRESS.md` (status 🟦). Indicate it below as `Mx`.

1. **Automatic gate.** Run `make -C poc check`. If it fails, fix and repeat: do not close with red tests.
2. **Requirement coverage.** Run `make -C poc reqs M=Mx`. Every requirement found must be covered by a test or justified in
   writing in the report (e.g. verifiable only with the real model ⇒ `llm` test). Verify that every acceptance criterion of
   PRD §9 has a `@pytest.mark.acceptance("Mx")` test.
3. **Contracts and documents.** If the milestone touched Pydantic models, regenerate and commit `poc/schemas/`; if it touched
   measures, update `poc/docs/analysis.md` with the exact formulas and parameters.
4. **Independent review.** Launch the subagent `milestone-reviewer`, passing it only the milestone id and the branch name (not
   your conclusions). For each finding: fix it, or note in the report why it should not be fixed. After the fixes, repeat
   point 1.
5. **Report.** Fill in the "Closing" section of `poc/docs/milestones/Mx.md`:
   - Done, Decided (with the entries added to `poc/docs/DECISIONS.md`), Doubts and risks, Flagged out of scope;
   - Automated check: brief output of `make -C poc check` and `make -C poc reqs`;
   - **Manual validation**: numbered steps the author can run by copying the commands, on a real track in `local/` or on a
     synthetic project, with what to listen to or check and the expected outcome. If the validation requires the network or
     costs money, say so explicitly.
   - Leave "Author review outcome" empty.
6. Update `poc/docs/PROGRESS.md`: status 🟨.
7. Commit: `Mx: close milestone (report)`.
8. **Stop.** Summarize for the author: what was done, what was decided, what remains doubtful, and the manual
   validation instructions. Do not start the next milestone.

When the author explicitly approves in chat: record the outcome in the report if the author did not, set the status to ✅ in
PROGRESS.md, commit, then `git switch main && git merge --no-ff milestone/<branch> && git tag mx`.
