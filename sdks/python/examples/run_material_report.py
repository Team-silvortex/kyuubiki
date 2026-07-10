from __future__ import annotations

from kyuubiki_sdk import build_material_report


def main() -> None:
    report = build_material_report(
        "composite-thermo-electric-panel",
        [
            {
                "electrostatic": {"max_electric_field": 45.0e6},
                "heat": {"max_temperature": 120.0},
                "thermal": {"max_stress": 180.0e6},
            },
            {
                "electrostatic": {"max_electric_field": 52.0e6},
                "heat": {"max_temperature": 98.0},
                "thermal": {"max_stress": 140.0e6},
            },
            {
                "electrostatic": {"max_electric_field": 58.0e6},
                "heat": {"max_temperature": 132.0},
                "thermal": {"max_stress": 210.0e6},
            },
        ],
    )
    print(f"study={report['study']}")
    print(f"winner={report['winner_candidate_id']}")
    print(f"reliability={report['reliability']['summary']['decision']}")


if __name__ == "__main__":
    main()
