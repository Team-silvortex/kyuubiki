from __future__ import annotations

import json

from kyuubiki_sdk import material_study_execution_plan_example


def main() -> None:
    print(json.dumps(material_study_execution_plan_example(), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
