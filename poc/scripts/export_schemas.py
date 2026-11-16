"""Write the JSON Schemas of the data contracts into schemas/ (run via `make schemas`)."""

from __future__ import annotations

from pathlib import Path

from stemwise.schema_export import CONTRACTS, render_schema

SCHEMAS = Path(__file__).resolve().parent.parent / "schemas"


def main() -> None:
    SCHEMAS.mkdir(exist_ok=True)
    for name, model in CONTRACTS.items():
        (SCHEMAS / name).write_text(render_schema(model), encoding="utf-8")
        print(f"wrote schemas/{name}")


if __name__ == "__main__":
    main()
