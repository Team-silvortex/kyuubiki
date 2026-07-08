from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path
from typing import Any

MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID = "workflow.material-study-envelope-ranking-json"
MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION = "kyuubiki.material-study-execution-plan/v1"
_REPO_ROOT = Path(__file__).resolve().parents[3]
_MATERIAL_STUDY_EXECUTION_PLAN_EXAMPLE = (
    _REPO_ROOT / "schemas" / "examples.material-study-execution-plan.json"
)


def material_study_envelope_input_artifacts() -> dict[str, Any]:
    return deepcopy(
        {
            "material_rows": {
                "rows": [
                    {
                        "case_id": "cool_stiff",
                        "summaries": {
                            "thermal": {"max_temperature": 82.0},
                            "structural": {"max_stress": 100.0},
                        },
                    },
                    {
                        "case_id": "warm_safe",
                        "summaries": {
                            "thermal": {"max_temperature": 90.0},
                            "structural": {"max_stress": 120.0},
                        },
                    },
                    {
                        "case_id": "hot_light",
                        "summaries": {
                            "thermal": {"max_temperature": 140.0},
                            "structural": {"max_stress": 110.0},
                        },
                    },
                ]
            }
        }
    )


def material_study_envelope_catalog_request(
    input_artifacts: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "workflow_id": MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID,
        "input_artifacts": deepcopy(input_artifacts)
        if input_artifacts is not None
        else material_study_envelope_input_artifacts(),
    }


def material_workflow_catalog() -> list[dict[str, Any]]:
    return [
        {
            "id": "material_study_envelope_catalog",
            "title": "Material Study Envelope Catalog Job",
            "domain": "multi_physics_materials",
            "objective": "submit the built-in material envelope workflow from the Orchestra catalog",
            "template_id": "material_study_envelope_catalog",
            "workflow_kind": "orchestra_catalog_job",
            "required_actions": ["workflow_submit_catalog", "job_wait", "result_fetch"],
            "aliases": ["material-envelope-catalog", "material_envelope_catalog"],
        },
        {
            "id": "material_study_envelope_ranking",
            "title": "Material Study Envelope Ranking",
            "domain": "multi_physics_materials",
            "objective": "compose material envelopes, rank candidates, and extract a Pareto frontier",
            "template_id": "material_study_envelope_ranking",
            "workflow_kind": "operator_graph",
            "required_actions": ["workflow_submit_graph", "job_wait", "result_fetch"],
            "aliases": [
                "material-envelope",
                "material_envelope",
                "material.pareto_ranking.v1",
            ],
        },
    ]


def material_study_execution_plan_example() -> dict[str, Any]:
    return deepcopy(json.loads(_MATERIAL_STUDY_EXECUTION_PLAN_EXAMPLE.read_text(encoding="utf-8")))
