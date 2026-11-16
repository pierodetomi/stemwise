"""JSON Schema export of the data contracts (PRD §6, §7.1) into `schemas/`."""

from __future__ import annotations

import json

from pydantic import BaseModel

from stemwise.analysis.model import Analysis
from stemwise.decision.schema import MixPlan
from stemwise.evaluate import Best, Evaluation
from stemwise.master import MasterReport
from stemwise.project import ProjectSpec
from stemwise.revise import Revision

JSON_SCHEMA_DIALECT = "https://json-schema.org/draft/2020-12/schema"

# File name in schemas/ -> model.
CONTRACTS: dict[str, type[BaseModel]] = {
    "project.schema.json": ProjectSpec,
    "analysis.schema.json": Analysis,
    "mix_plan.schema.json": MixPlan,
    "evaluation.schema.json": Evaluation,
    "best.schema.json": Best,
    "revision.schema.json": Revision,
    "master_report.schema.json": MasterReport,
}


def render_schema(model: type[BaseModel]) -> str:
    schema = {"$schema": JSON_SCHEMA_DIALECT, **model.model_json_schema()}
    return json.dumps(schema, indent=2, ensure_ascii=False, sort_keys=True) + "\n"
