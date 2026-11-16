"""Pan laws (REN-3, D-008, D-030)."""

from __future__ import annotations

import math

import numpy as np
import pytest

from stemwise.render.mixbus import to_stereo
from stemwise.render.pan import pan
from tests.helpers.signals import noise, sine


@pytest.mark.req("REN-3")
@pytest.mark.parametrize("position", [-0.75, -0.5, -0.25, 0.25, 0.5, 0.75])
def test_mono_pan_constant_power(position: float) -> None:
    x = sine(440.0)
    y = pan(x, position).astype(np.float64)
    power = y[:, 0] ** 2 + y[:, 1] ** 2
    np.testing.assert_allclose(power, x[:, 0].astype(np.float64) ** 2, rtol=1e-6, atol=1e-12)
    theta = (position + 1.0) * math.pi / 4.0
    np.testing.assert_allclose(y[:, 0], x[:, 0] * math.cos(theta), rtol=1e-6, atol=1e-9)
    louder, softer = (1, 0) if position > 0 else (0, 1)
    assert (y[:, louder] ** 2).sum() > (y[:, softer] ** 2).sum()


@pytest.mark.req("REN-3")
def test_mono_hard_left_zeroes_right() -> None:
    x = sine(440.0)
    y = pan(x, -1.0)
    assert np.all(y[:, 1] == 0.0)
    np.testing.assert_array_equal(y[:, 0], x[:, 0])
    y = pan(x, 1.0)
    assert np.all(y[:, 0] == 0.0)
    np.testing.assert_array_equal(y[:, 1], x[:, 0])


@pytest.mark.req("REN-3")
def test_mono_centre_is_the_raw_sum_law() -> None:
    x = sine(440.0)
    np.testing.assert_array_equal(pan(x, 0.0), to_stereo(x))  # -3.01 dB per channel (D-008)


@pytest.mark.req("REN-3")
@pytest.mark.parametrize(
    ("position", "left", "right"),
    [(0.0, 1.0, 1.0), (-1.0, 1.0, 0.0), (1.0, 0.0, 1.0), (0.5, 0.5, 1.0), (-0.25, 1.0, 0.75)],
)
def test_stereo_balance(position: float, left: float, right: float) -> None:
    x = noise(channels=2, seed=3)
    y = pan(x, position)
    if position == 0.0:
        np.testing.assert_array_equal(y, x)  # unchanged, sample for sample
    np.testing.assert_allclose(y[:, 0], x[:, 0] * left, rtol=1e-6)
    np.testing.assert_allclose(y[:, 1], x[:, 1] * right, rtol=1e-6)
    if right == 0.0:
        assert np.all(y[:, 1] == 0.0)
    if left == 0.0:
        assert np.all(y[:, 0] == 0.0)


def test_pan_output_is_stereo_float32() -> None:
    for x in (sine(440.0), noise(channels=2)):
        y = pan(x, 0.3)
        assert y.shape == (x.shape[0], 2)
        assert y.dtype == np.float32
