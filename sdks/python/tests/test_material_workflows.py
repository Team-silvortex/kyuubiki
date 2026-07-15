from __future__ import annotations

import json
import unittest
from pathlib import Path

from kyuubiki_sdk import (
    MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID,
    MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION,
    material_study_envelope_catalog_request,
    material_study_envelope_input_artifacts,
    material_study_execution_plan_example,
    material_workflow_catalog,
)

FIXTURE_PATH = (
    Path(__file__).resolve().parents[3]
    / "schemas"
    / "examples.material-envelope-catalog-request.json"
)


class MaterialWorkflowTest(unittest.TestCase):
    def test_material_workflow_catalog_prefers_orchestra_catalog_path(self) -> None:
        catalog = material_workflow_catalog()

        self.assertEqual(catalog[0]["id"], "material_study_envelope_catalog")
        self.assertEqual(catalog[0]["workflow_kind"], "orchestra_catalog_job")
        self.assertEqual(catalog[0]["required_actions"][0], "workflow_submit_catalog")
        self.assertEqual(catalog[1]["workflow_kind"], "operator_graph")

    def test_material_envelope_catalog_request_uses_builtin_workflow_id(self) -> None:
        request = material_study_envelope_catalog_request()
        fixture = json.loads(FIXTURE_PATH.read_text())
        fixture.pop("$schema")

        self.assertEqual(request["workflow_id"], MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID)
        self.assertEqual(request, fixture)
        self.assertEqual(
            request["input_artifacts"]["material_rows"]["rows"][0]["case_id"],
            "cool_stiff",
        )

    def test_material_envelope_helpers_return_deep_copies(self) -> None:
        first = material_study_envelope_input_artifacts()
        first["material_rows"]["rows"][0]["case_id"] = "mutated"

        second = material_study_envelope_catalog_request()

        self.assertEqual(
            second["input_artifacts"]["material_rows"]["rows"][0]["case_id"],
            "cool_stiff",
        )

    def test_material_study_execution_plan_example_matches_shared_contract(self) -> None:
        plan = material_study_execution_plan_example()

        self.assertEqual(
            plan["schema_version"],
            MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION,
        )
        self.assertEqual(plan["study_id"], "material_heat_spreader_screening")
        self.assertEqual(plan["step_count"], len(plan["steps"]))
        self.assertEqual(plan["solve_step_count"], 3)
        self.assertEqual(plan["candidate_count"], 3)
        self.assertEqual(plan["material_card_contract_required"], True)
        self.assertEqual(
            plan["material_card_schema_version"],
            "kyuubiki.material-card/v1",
        )
        self.assertEqual(plan["material_card_ref_count"], 3)
        self.assertIn("copper_c110", plan["candidate_ids"])
        self.assertIn("heat-spreader", plan["recommended_command"])

    def test_material_study_execution_plan_example_returns_deep_copies(self) -> None:
        first = material_study_execution_plan_example()
        first["candidate_ids"][0] = "mutated"
        second = material_study_execution_plan_example()

        self.assertEqual(second["candidate_ids"][0], "aluminum_6061")


if __name__ == "__main__":
    unittest.main()
