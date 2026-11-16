"""Contract of analysis.json: limits, rounding, serialization, token budget (ANA-13, D-018)."""

from __future__ import annotations

import json
import math
import re
from typing import Any, get_args

import pytest
from pydantic import ValidationError

from stemwise.analysis.masking import Collision
from stemwise.analysis.mix import MixMeasures
from stemwise.analysis.model import (
    Analysis,
    BandLabel,
    StemInput,
    build_analysis,
    dumps_analysis,
    estimated_tokens,
)
from stemwise.analysis.reference import Target
from stemwise.analysis.spectral import BAND_LABELS
from stemwise.analysis.stem import Hygiene, StemMeasures, StereoMeasures
from stemwise.schema_export import CONTRACTS

# Worst-case values: as many characters as the contract allows per number.
_BANDS = [-119.94, -24.12, -12.31, -8.04, -5.27, -9.93, -15.44, -22.06]


def _stem(**overrides: Any) -> StemMeasures:
    values: dict[str, Any] = {
        "channels": 2,
        "lufs": -21.437,
        "peak_db": -4.123,
        "true_peak_db": -3.812,
        "rms_db": -18.456,
        "crest_db": 14.217,
        "dyn_range_lu": 9.333,
        "active_pct": 62.468,
        "bands_db": list(_BANDS),
        "centroid_hz": 18_500.456,
        "onsets_per_s": 12.149,
        "stereo": StereoMeasures(correlation=-0.8234, side_mid_db=-11.049),
        "hygiene": Hygiene(dc_offset_pct=0.0412, clipped_samples=123_456, noise_floor_db=-72.051),
    }
    values.update(overrides)
    return StemMeasures(**values)


def _mix(**overrides: Any) -> MixMeasures:
    values: dict[str, Any] = {
        "lufs": -19.843,
        "peak_db": -3.512,
        "true_peak_db": -3.014,
        "rms_db": -16.444,
        "crest_db": 12.932,
        "dyn_range_lu": 8.149,
        "bands_db": list(_BANDS),
        "stereo": StereoMeasures(correlation=0.8234, side_mid_db=-11.049),
        "low_corr": 0.9712,
    }
    values.update(overrides)
    return MixMeasures(**values)


def _analysis(n_stems: int = 12, id_len: int = 32, **stem_overrides: Any) -> Analysis:
    ids = [f"{i:02d}".ljust(id_len, "x") for i in range(n_stems)]
    return build_analysis(
        sample_rate=48_000,
        duration_s=214.337,
        stems={sid: StemInput("backing_vocal", _stem(**stem_overrides)) for sid in ids},
        mix=_mix(),
        band_share={sid: [12.345] * 8 for sid in ids},
        collisions=[Collision(ids[0], ids[1], band, 0.71234) for band in range(8)]
        + [Collision(ids[-2], ids[-1], band, 0.6) for band in range(2)],
        references=[_mix(lufs=-23.0) for _ in range(3)],
        target=Target(n_references=3, bands_db=list(_BANDS), side_mid_db=-9.51, dyn_range_lu=6.84),
    )


@pytest.mark.req("ANA-13")
def test_token_budget_12_stems() -> None:
    text = dumps_analysis(_analysis())
    # 12 stereo stems with 32-character ids, 3 references, all fields filled: worst case.
    assert estimated_tokens(text) < 8000


_RATIO_KEYS = {"correlation", "low_corr", "score"}  # two decimals (D-021)


def _keyed_numbers(value: Any, key: str = "") -> list[tuple[str, float]]:
    if isinstance(value, dict):
        return [n for k, v in value.items() for n in _keyed_numbers(v, k)]
    if isinstance(value, list):
        return [n for v in value for n in _keyed_numbers(v, key)]
    return [(key, value)] if isinstance(value, float) else []


@pytest.mark.req("ANA-13")
def test_rounding_one_decimal() -> None:
    text = dumps_analysis(_analysis())
    numbers = _keyed_numbers(json.loads(text))
    assert numbers
    for key, n in numbers:
        assert round(n, 2 if key in _RATIO_KEYS else 1) == n, (key, n)
    assert not re.search(r"\d\.\d\d\d", text)
    doc = json.loads(text)
    assert doc["mix"]["low_corr"] == 0.97
    assert doc["mix"]["stereo"]["correlation"] == 0.82
    assert doc["masking_top"][0]["score"] == 0.71
    assert doc["mix"]["lufs"] == -19.8  # everything else: one decimal
    assert '"clipped_samples": 123456' in text  # integers stay integers
    assert "-0.0" not in text


@pytest.mark.req("ANA-13")
def test_no_non_finite() -> None:
    analysis = _analysis(
        lufs=-math.inf,
        bands_db=[-math.inf, 0.0, -math.inf, -math.inf, -math.inf, -math.inf, -math.inf, -200.0],
        stereo=StereoMeasures(correlation=1.0, side_mid_db=-math.inf),
        hygiene=Hygiene(dc_offset_pct=0.0, clipped_samples=0, noise_floor_db=-math.inf),
    )
    text = dumps_analysis(analysis)
    assert "Infinity" not in text
    assert "NaN" not in text
    stem = next(iter(json.loads(text)["stems"].values()))
    assert stem["lufs"] == -120.0
    assert stem["bands_db"] == [-120.0, 0.0, -120.0, -120.0, -120.0, -120.0, -120.0, -120.0]
    assert stem["stereo"]["side_mid_db"] == -120.0
    anti = _analysis(stereo=StereoMeasures(correlation=-1.0, side_mid_db=math.inf))
    anti_stereo = next(iter(anti.stems.values())).stereo
    assert anti_stereo is not None
    assert anti_stereo.side_mid_db == 120.0


@pytest.mark.req("ANA-13")
def test_round_trip_and_validation() -> None:
    analysis = _analysis()
    assert Analysis.model_validate(json.loads(dumps_analysis(analysis))) == analysis
    doc = json.loads(dumps_analysis(analysis))
    doc["mix"]["lufs"] = float("inf")
    with pytest.raises(ValidationError):
        Analysis.model_validate(doc)
    doc = json.loads(dumps_analysis(analysis))
    doc["unexpected"] = 1
    with pytest.raises(ValidationError):
        Analysis.model_validate(doc)


@pytest.mark.req("ANA-13")
def test_nulls_for_inactive_stem() -> None:
    silent = _stem(
        channels=1,
        rms_db=None,
        crest_db=None,
        dyn_range_lu=None,
        active_pct=0.0,
        bands_db=None,
        centroid_hz=None,
        onsets_per_s=0.0,
        stereo=None,
    )
    stem = next(iter(json.loads(dumps_analysis(_analysis(**silent.__dict__)))["stems"].values()))
    for key in ("rms_db", "crest_db", "dyn_range_lu", "bands_db", "centroid_hz", "stereo"):
        assert stem[key] is None


@pytest.mark.req("ANA-13")
def test_committed_schema_registered() -> None:
    # The drift check itself is tests/test_project.py::test_committed_schema_matches_model.
    assert CONTRACTS["analysis.schema.json"] is Analysis


@pytest.mark.req("ANA-13")
def test_no_library_names() -> None:
    text = dumps_analysis(_analysis()).lower()
    for name in ("numpy", "scipy", "librosa", "pyloudnorm", "pydantic", "soundfile", "pedalboard"):
        assert name not in text


def test_band_labels_match_contract() -> None:
    assert get_args(BandLabel) == BAND_LABELS


def test_layout_lists_of_numbers_on_one_line() -> None:
    text = dumps_analysis(_analysis(n_stems=2))
    assert '"bands_db": [-119.9, -24.1, -12.3, -8.0, -5.3, -9.9, -15.4, -22.1]' in text
    assert text.startswith('{\n  "version": 1,\n  "project": {')
    assert text.endswith("}\n")
