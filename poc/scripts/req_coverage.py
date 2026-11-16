"""Report which PRD requirements are referenced by tests via `@pytest.mark.req(...)`.

Requirement ids are read from docs/PRD.md (lines like `- **ANA-3.** ...`). Test references are read
statically (AST) from tests/**/*.py: any call `<x>.mark.req("ID", ...)`, as a decorator or in
`pytestmark`. Acceptance tests are calls `<x>.mark.acceptance("Mx")`.

Usage:
    uv run python scripts/req_coverage.py                 # all milestones
    uv run python scripts/req_coverage.py --milestone M1  # one milestone; exit 1 if gaps
"""

from __future__ import annotations

import argparse
import ast
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PRD = ROOT / "docs" / "PRD.md"
TESTS = ROOT / "tests"

# Requirement ids owned by each milestone (PRD §9). M0 also builds the trim (REN-5) and the
# baseline (REN-7), which M2 completes; they are listed under M2 to avoid double counting.
MILESTONES: dict[str, list[str]] = {
    "M0": ["ING-1", "ING-2", "ING-3", "ING-4", "ING-5", "ING-6"],
    "M1": [f"ANA-{i}" for i in range(1, 14)],
    "M2": [f"REN-{i}" for i in range(1, 8)],
    "M3": [f"DEC-{i}" for i in range(1, 9)],
    "M4": [f"EVA-{i}" for i in range(1, 7)] + ["REP-1"],
    "M5": ["REV-1", "REV-2", "REV-3"],
    "M6": ["MAS-1", "MAS-2", "MAS-3"],
    "M7": ["REP-2"],
}

REQ_RE = re.compile(r"\*\*([A-Z]{3}-\d+)\.\*\*")


def prd_requirements() -> list[str]:
    return REQ_RE.findall(PRD.read_text(encoding="utf-8"))


def _mark_name(func: ast.expr) -> str | None:
    """Return 'req' for `pytest.mark.req` / `mark.req`, else None."""
    if (
        isinstance(func, ast.Attribute)
        and isinstance(func.value, ast.Attribute)
        and func.value.attr == "mark"
    ):
        return func.attr
    return None


def scan_tests() -> tuple[dict[str, list[str]], dict[str, list[str]]]:
    """Return (requirement id -> test locations, milestone -> acceptance test locations)."""
    reqs: dict[str, list[str]] = defaultdict(list)
    acceptance: dict[str, list[str]] = defaultdict(list)
    for path in sorted(TESTS.rglob("*.py")):
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        rel = path.relative_to(ROOT)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            name = _mark_name(node.func)
            if name not in {"req", "acceptance"}:
                continue
            args = [a.value for a in node.args if isinstance(a, ast.Constant)]
            target = reqs if name == "req" else acceptance
            for arg in args:
                target[str(arg)].append(f"{rel}:{node.lineno}")
    return reqs, acceptance


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--milestone", "-m", choices=sorted(MILESTONES), help="limit to one milestone"
    )
    args = parser.parse_args()

    known = prd_requirements()
    reqs, acceptance = scan_tests()

    problems = 0
    unknown = sorted(set(reqs) - set(known))
    if unknown:
        problems += len(unknown)
        print(f"Unknown requirement ids referenced by tests (not in PRD): {', '.join(unknown)}")

    selected = [args.milestone] if args.milestone else list(MILESTONES)
    for ms in selected:
        ids = MILESTONES[ms]
        covered = [i for i in ids if i in reqs]
        print(f"\n{ms}: {len(covered)}/{len(ids)} requirements referenced by tests")
        for rid in ids:
            where = reqs.get(rid)
            mark = "ok " if where else "-- "
            print(f"  {mark}{rid:<7} {', '.join(where) if where else 'NOT COVERED'}")
        acc = acceptance.get(ms, [])
        print(f"  acceptance tests: {len(acc)}" + (f" ({', '.join(acc)})" if acc else ""))
        if args.milestone:
            problems += len(ids) - len(covered) + (0 if acc else 1)

    if args.milestone and problems:
        print(
            f"\n{problems} gap(s): cover them with tests or justify them in the milestone report."
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
