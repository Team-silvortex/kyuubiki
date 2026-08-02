from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from kyuubiki_sdk import (
    ModelResearchExecutionError,
    build_model_headless_plan,
    build_model_plan_approval_request,
    compute_model_headless_plan_digest,
)


class ModelPlanApprovalTests(unittest.TestCase):
    def test_shared_plan_builds_digest_bound_request(self) -> None:
        schemas = Path(__file__).resolve().parents[3] / "schemas"
        session = _read(schemas / "examples.model-collaboration-session.json")
        proposal = _read(schemas / "examples.model-workflow-proposal.json")
        expected = _read(schemas / "examples.model-plan-approval-request.json")
        expected.pop("$schema")
        plan = build_model_headless_plan(session, proposal)

        self.assertEqual(build_model_plan_approval_request(plan), expected)
        self.assertEqual(
            compute_model_headless_plan_digest(plan),
            "sha256:22e040653a1fc2274201a86f3ffaff67e896cedb5754e6fee01fb0528704d18d",
        )

    def test_nested_payload_changes_digest(self) -> None:
        schemas = Path(__file__).resolve().parents[3] / "schemas"
        session = _read(schemas / "examples.model-collaboration-session.json")
        proposal = _read(schemas / "examples.model-workflow-proposal.json")
        plan = build_model_headless_plan(session, proposal)
        changed = copy.deepcopy(plan)
        changed["steps"][1]["payload"]["input_artifacts"]["material_rows"]["rows"][0][
            "case_id"
        ] = "changed"

        self.assertNotEqual(
            compute_model_headless_plan_digest(plan),
            compute_model_headless_plan_digest(changed),
        )

    def test_approval_request_rejects_inconsistent_gated_risk(self) -> None:
        schemas = Path(__file__).resolve().parents[3] / "schemas"
        session = _read(schemas / "examples.model-collaboration-session.json")
        proposal = _read(schemas / "examples.model-workflow-proposal.json")
        plan = build_model_headless_plan(session, proposal)
        plan["steps"][1]["risk"] = "normal"

        with self.assertRaisesRegex(ModelResearchExecutionError, "has invalid risk"):
            build_model_plan_approval_request(plan)


def _read(path: Path) -> dict:
    return json.loads(path.read_text())


if __name__ == "__main__":
    unittest.main()
