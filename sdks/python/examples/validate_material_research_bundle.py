from __future__ import annotations

import json
import pathlib
import sys

from kyuubiki_sdk import validate_material_research_bundle


def main() -> None:
    repo_root = pathlib.Path(__file__).resolve().parents[3]
    default_fixture = repo_root / "schemas" / "examples.material-research-bundle.json"
    bundle_path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else default_fixture
    bundle = validate_material_research_bundle(json.loads(bundle_path.read_text()))
    print(f"schema={bundle['schema_version']}")
    print(f"study={bundle['study']}")
    print(f"winner={bundle['summary']['winner_candidate_id']}")
    print(f"reliability={bundle['summary']['reliability_decision']}")
    print(f"next_round={bundle['summary']['next_round_decision']}")
    print(f"next_iteration={bundle['summary'].get('next_iteration', 0)}")
    print(f"runnable_next_steps={bundle['summary'].get('runnable_next_step_count', 0)}")
    print(f"chain_stop={bundle['summary']['chain_stop_reason']}")


if __name__ == "__main__":
    main()
