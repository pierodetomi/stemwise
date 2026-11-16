"""Loading and validation of `project.yaml` (ING-1, PRD §6.1, D-010).

`ProjectSpec` is the contract (exported to `schemas/project.schema.json`); `Project` adds the
project folder, so that file paths in the spec are resolved relative to it.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Annotated, Literal

import yaml
from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    StrictBool,
    StrictInt,
    ValidationError,
    field_validator,
    model_validator,
)

from stemwise.errors import ProjectError

PROJECT_FILE = "project.yaml"

Role = Literal[
    "kick",
    "snare",
    "toms",
    "overheads",
    "drums",
    "percussion",
    "bass",
    "electric_guitar",
    "acoustic_guitar",
    "keys",
    "synth",
    "pad",
    "lead_vocal",
    "backing_vocal",
    "fx",
    "other",
]

StemId = Annotated[
    str,
    Field(
        pattern=r"^[a-z0-9][a-z0-9_]*$",
        max_length=32,
        description="Stem identifier: lowercase letters, digits and underscores, at most 32.",
    ),
]


class _Strict(BaseModel):
    model_config = ConfigDict(extra="forbid")


class Brief(_Strict):
    """Creative brief written by the author (passed to the model as text)."""

    genre: str = Field("", description="Musical genre.")
    mood: str = Field("", description="Desired mood.")
    notes: str = Field("", description="Free notes on the desired result.")


class StemSpec(_Strict):
    id: StemId
    file: str = Field(description="WAV or FLAC file, relative to the project folder.")
    role: Role = Field(description="Declared role of the stem.")
    notes: str | None = Field(None, description="Optional notes about the stem.")


class ReferenceSpec(_Strict):
    file: str = Field(description="Reference mix (WAV or FLAC), relative to the project folder.")


class Options(_Strict):
    pre_normalize: StrictBool = Field(
        True, description="Normalise every stem to -23 LUFS before applying the plan."
    )
    max_iterations: StrictInt = Field(
        3, ge=1, le=3, description="Maximum iterations of the verification loop."
    )
    mix_true_peak_db: float = Field(
        -3.0,
        ge=-24.0,
        le=-3.0,
        allow_inf_nan=False,
        description="True-peak ceiling of the mix bus in dBTP (from -24 to -3).",
    )


class ProjectSpec(_Strict):
    """Content of project.yaml."""

    name: str = Field(min_length=1, description="Song title.")
    sample_rate: Annotated[StrictInt, Field(gt=0)] | Literal["auto"] = Field(
        description="Sample rate of every stem in Hz, or 'auto' for the one shared by all stems."
    )
    brief: Brief
    stems: list[StemSpec] = Field(min_length=2, max_length=16)
    references: list[ReferenceSpec] = Field(default_factory=list, max_length=3)
    options: Options = Field(default_factory=Options)

    @field_validator("sample_rate", mode="before")
    @classmethod
    def _sample_rate_shape(cls, value: object) -> object:
        valid_int = isinstance(value, int) and not isinstance(value, bool) and value > 0
        if not (valid_int or value == "auto"):
            raise ValueError(f"must be a positive integer (Hz) or 'auto', got {value!r}")
        return value

    @model_validator(mode="after")
    def _unique_ids(self) -> ProjectSpec:
        ids = [s.id for s in self.stems]
        duplicates = sorted({i for i in ids if ids.count(i) > 1})
        if duplicates:
            raise ValueError(f"duplicate stem ids: {', '.join(duplicates)}")
        return self


@dataclass(frozen=True)
class Project:
    """A validated project: the spec plus the folder its relative paths refer to."""

    root: Path
    spec: ProjectSpec

    def resolve(self, file: str) -> Path:
        return (self.root / file).resolve()

    def stem_path(self, stem: StemSpec) -> Path:
        return self.resolve(stem.file)

    def reference_path(self, reference: ReferenceSpec) -> Path:
        return self.resolve(reference.file)


def _format_validation_error(path: Path, error: ValidationError) -> str:
    lines = [f"invalid {PROJECT_FILE} ({path}):"]
    for item in error.errors():
        loc = ".".join(str(part) for part in item["loc"]) or "<root>"
        lines.append(f"  - {loc}: {item['msg']}")
    return "\n".join(lines)


def load_project(project_dir: Path) -> Project:
    """Read, validate and check the files of `project_dir/project.yaml`. Raises `ProjectError`."""
    root = Path(project_dir).resolve()
    path = root / PROJECT_FILE
    if not path.is_file():
        raise ProjectError(f"{PROJECT_FILE} not found in {root}")
    try:
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise ProjectError(f"cannot parse {PROJECT_FILE} ({path}): {exc}") from exc
    if not isinstance(doc, dict):
        raise ProjectError(f"invalid {PROJECT_FILE} ({path}): expected a mapping at the top level")
    try:
        spec = ProjectSpec.model_validate(doc)
    except ValidationError as exc:
        raise ProjectError(_format_validation_error(path, exc)) from exc

    project = Project(root=root, spec=spec)
    missing = [
        f"  - stem '{s.id}': file not found: {project.stem_path(s)}"
        for s in spec.stems
        if not project.stem_path(s).is_file()
    ] + [
        f"  - reference: file not found: {project.reference_path(r)}"
        for r in spec.references
        if not project.reference_path(r).is_file()
    ]
    if missing:
        raise ProjectError("\n".join([f"missing files in {PROJECT_FILE} ({path}):", *missing]))
    return project
