"""The `Decider` interface (PRD §7) and the data it exchanges (DEC-1).

A decider turns a `DecisionInput` (brief, stems with id/role/notes, the analysis, the history of
the last iterations, an optional request of the author) into a complete, validated `MixPlan`.
Two implementations: `MockDecider` (offline, deterministic, DEC-7) and `LlmDecider` (OpenRouter).
Everything here is plain data: no audio, no file paths, no song title (PRD §7, D-041).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Protocol, runtime_checkable

from stemwise.analysis.model import Analysis
from stemwise.decision.schema import MixPlan
from stemwise.project import Brief, Project

HISTORY_LENGTH = 2  # DEC-1: the last 2 iterations, with their evaluations


@dataclass(frozen=True)
class StemInfo:
    id: str
    role: str
    notes: str | None = None


@dataclass(frozen=True)
class HistoryEntry:
    """A previous iteration: its plan and, from M4, its evaluation (`evaluation.json`)."""

    plan: MixPlan
    evaluation: dict[str, Any] | None = None


@dataclass(frozen=True)
class DecisionInput:
    brief: Brief
    stems: tuple[StemInfo, ...]
    analysis: Analysis
    analysis_json: str  # the exact text of analysis.json, sent as is
    iteration: int = 0
    history: tuple[HistoryEntry, ...] = ()
    request: str | None = None
    previous_plan: MixPlan | None = None  # reference of the change limits (DEC-5)
    pre_normalize: bool = True  # REN-1: stems at -23 LUFS before the plan
    true_peak_ceiling_db: float = -3.0  # REN-5: trim of the mix bus

    @property
    def sample_rate(self) -> int:
        return self.analysis.project.sample_rate

    @property
    def stem_ids(self) -> list[str]:
        return [s.id for s in self.stems]

    @property
    def recent_history(self) -> tuple[HistoryEntry, ...]:
        return self.history[-HISTORY_LENGTH:]


@dataclass(frozen=True)
class Decision:
    """A validated plan, the warnings raised on the way and the logs of the calls (DEC-8)."""

    plan: MixPlan
    warnings: tuple[str, ...] = ()
    request_log: dict[str, Any] = field(default_factory=dict)
    response_log: dict[str, Any] | None = None


@runtime_checkable
class Decider(Protocol):
    name: str

    def decide(self, decision_input: DecisionInput) -> Decision: ...


def decision_input(
    project: Project,
    analysis: Analysis,
    analysis_json: str,
    *,
    iteration: int = 0,
    history: tuple[HistoryEntry, ...] = (),
    request: str | None = None,
    previous_plan: MixPlan | None = None,
) -> DecisionInput:
    """Build the input from a loaded project: only brief, ids, roles, notes and options are kept."""
    options = project.spec.options
    return DecisionInput(
        brief=project.spec.brief,
        stems=tuple(StemInfo(s.id, s.role, s.notes) for s in project.spec.stems),
        analysis=analysis,
        analysis_json=analysis_json,
        iteration=iteration,
        history=history,
        request=request,
        previous_plan=previous_plan,
        pre_normalize=options.pre_normalize,
        true_peak_ceiling_db=options.mix_true_peak_db,
    )
