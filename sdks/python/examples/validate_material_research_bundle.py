from __future__ import annotations

import json
import pathlib

from kyuubiki_sdk import validate_material_research_bundle


def main() -> None:
    repo_root = pathlib.Path(__file__).resolve().parents[3]
    fixture = repo_root / "schemas" / "examples.material-research-bundle.json"
    bundle = validate_material_research_bundle(json.loads(fixture.read_text()))
    print(f"schema={bundle['schema_version']}")
    print(f"study={bundle['study']}")
    print(f"winner={bundle['summary']['winner_candidate_id']}")
    print(f"reliability={bundle['summary']['reliability_decision']}")


if __name__ == "__main__":
    main()
