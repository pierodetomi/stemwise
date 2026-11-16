.PHONY: setup fmt lint test test-llm check reqs schemas

setup:
	uv sync

fmt:
	uv run ruff format .
	uv run ruff check --fix .

lint:
	uv run ruff format --check .
	uv run ruff check .

test:
	uv run pytest

# Real model over the network: needs OPENROUTER_API_KEY, costs money. Never part of `check`.
test-llm:
	uv run pytest -m llm

check: lint test

# Requirement coverage, e.g. `make reqs M=M1` (omit M for all milestones).
reqs:
	uv run python scripts/req_coverage.py $(if $(M),--milestone $(M),)

# Regenerate the JSON Schemas of the data contracts (PRD §7.1) after touching a Pydantic model.
schemas:
	uv run python scripts/export_schemas.py
