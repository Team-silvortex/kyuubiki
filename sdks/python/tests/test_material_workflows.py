from __future__ import annotations

import unittest

from kyuubiki_sdk import (
    MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID,
    material_study_envelope_catalog_request,
    material_study_envelope_input_artifacts,
    material_workflow_catalog,
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

        self.assertEqual(request["workflow_id"], MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID)
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


if __name__ == "__main__":
    unittest.main()
