"""Shared pytest configuration.

Network guard: PRD §9 forbids tests that reach the network. Every test not marked
`@pytest.mark.llm` gets socket connections (except local AF_UNIX sockets) replaced by an error.
"""

from __future__ import annotations

import socket

import pytest


class NetworkAccessError(RuntimeError):
    """Raised when a test without the `llm` marker tries to open a network connection."""


@pytest.fixture(autouse=True)
def _block_network(request: pytest.FixtureRequest, monkeypatch: pytest.MonkeyPatch) -> None:
    if request.node.get_closest_marker("llm") is not None:
        return

    real_connect = socket.socket.connect

    def guarded_connect(self: socket.socket, address, *args, **kwargs):
        if getattr(socket, "AF_UNIX", None) is not None and self.family == socket.AF_UNIX:
            return real_connect(self, address, *args, **kwargs)
        raise NetworkAccessError(
            f"Network access is forbidden in tests (PRD §9): {address!r}. "
            "Mark real-model tests with @pytest.mark.llm."
        )

    def guarded_create_connection(address, *args, **kwargs):
        raise NetworkAccessError(f"Network access is forbidden in tests (PRD §9): {address!r}.")

    monkeypatch.setattr(socket.socket, "connect", guarded_connect)
    monkeypatch.setattr(socket, "create_connection", guarded_create_connection)
