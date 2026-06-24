from __future__ import annotations

import unittest

from kyuubiki_sdk import (
    build_material_report,
    build_material_report_from_payload,
    describe_material_study,
    extract_material_result_payloads,
    material_study_catalog,
)


class MaterialReportsTest(unittest.TestCase):
    def test_catalog_exposes_material_studies(self) -> None:
        catalog = material_study_catalog()
        self.assertEqual(len(catalog), 4)
        dielectric = describe_material_study("dielectric-screening")
        self.assertEqual(dielectric["id"], "material_dielectric_screening")
        self.assertEqual(dielectric["domain"], "electromagnetic")
        self.assertTrue(dielectric["metric_specs"])

    def test_extracts_successful_headless_result_fetch_payloads(self) -> None:
        payloads = extract_material_result_payloads(
            {
                "schema_version": "kyuubiki.headless-execution-run/v1",
                "steps": [
                    {
                        "action": "result_fetch",
                        "status": "executed",
                        "result_preview": {"result": {"max_electric_field": 42.0e6}},
                    },
                    {
                        "action": "result_fetch",
                        "status": "failed",
                        "result_preview": {"result": {"max_electric_field": 1.0}},
                    },
                ],
            }
        )
        self.assertEqual(payloads, [{"max_electric_field": 42.0e6}])

    def test_builds_dielectric_report_from_alias(self) -> None:
        report = build_material_report(
            "dielectric-screening",
            [
                {"result": {"max_electric_field": 42.0e6, "max_flux_density": 1.2e-3}},
                {"result": {"max_electric_field": 38.0e6, "max_flux_density": 3.3e-3}},
                {"max_electric_field": 48.0e6, "max_flux_density": 0.9e-3},
            ],
        )
        self.assertEqual(report["schema_version"], "kyuubiki.dielectric-material-report/v1")
        self.assertEqual(report["winner_candidate_id"], "polyimide_film")
        self.assertEqual(report["candidates"][0]["rank"], 1)

    def test_builds_report_directly_from_payload_wrapper(self) -> None:
        report = build_material_report_from_payload(
            "structural-panel",
            {
                "result_payloads": [
                    {"result": {"max_stress": 240.0e6, "max_displacement": 0.001}},
                    {"result": {"max_stress": 180.0e6, "max_displacement": 0.0008}},
                    {"result": {"max_stress": 210.0e6, "max_displacement": 0.0012}},
                ]
            },
        )
        self.assertEqual(report["study"], "material.structural_panel_screening.v1")
        self.assertTrue(report["winner_candidate_id"])


if __name__ == "__main__":
    unittest.main()
