# Mx — <milestone name>

- Status: 🟦 in progress | 🟨 in author review | ✅ approved
- Branch: `milestone/mx-<slug>`
- PRD requirements: <e.g. ANA-1 → ANA-13>
- PRD sections read: <e.g. §5.2, §6.2, §9, §10.1>

## Plan (written at the start, approved by the author before writing code)

### Requirements → planned tests

| Id | What to verify | Planned test | Tolerance |
| --- | --- | --- | --- |
| ANA-1 | … | `tests/analysis/test_stem.py::test_…` | ±0.1 LU |

### Acceptance criteria → tests

| Criterion (PRD §9) | Test |
| --- | --- |
| … | `…` (`@pytest.mark.acceptance("Mx")`) |

### Steps

1. …

### Ambiguities and questions

- None / Q-x in [DECISIONS.md](../DECISIONS.md)

## Closing

### Done

- …

### Decided

- D-xxx … (details in DECISIONS.md)

### Doubts and risks

- …

### Flagged out of scope

- None / …

### Automated check

```text
<brief output of `make check` and `make reqs M=Mx`>
```

Requirements without an automatic test and the reason: …

Outcome of `milestone-reviewer`: <summary of the findings and how they were handled>

### Manual validation (for the author)

Prerequisites: …

1. Command:
   ```bash
   uv run stemwise …
   ```
   What to check / listen to: …
   Expected outcome: …

### Author review outcome

_Filled in by the author._

- Date:
- Manual validation: ok / problems: …
- Decision: approved / to revise: …
