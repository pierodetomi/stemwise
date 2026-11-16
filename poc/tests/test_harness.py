"""Sanity checks of the test harness itself (not of the engine)."""

import socket

import pytest

import stemwise
from tests.conftest import NetworkAccessError


def test_package_importable() -> None:
    assert stemwise.__version__


def test_network_is_blocked() -> None:
    with pytest.raises(NetworkAccessError):
        socket.create_connection(("example.com", 443), timeout=1)
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s, pytest.raises(NetworkAccessError):
        s.connect(("127.0.0.1", 9))
