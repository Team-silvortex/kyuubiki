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
        self.assertEqual(len(catalog), 5)
        dielectric = describe_material_study("dielectric-screening")
        self.assertEqual(dielectric["id"], "material_dielectric_screening")
        self.assertEqual(dielectric["domain"], "electromagnetic")
        self.assertTrue(dielectric["metric_specs"])
        composite = describe_material_study("composite-thermo-electric-panel")
        self.assertEqual(composite["domain"], "multiphysics_materials")

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
        self.assertEqual(
            report["reliability"]["summary"]["decision"],
            "ready_for_next_round",
        )
        self.assertEqual(report["reliability"]["summary"]["violation_count"], 0)

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

    def test_reliability_summary_blocks_missing_required_metrics(self) -> None:
        report = build_material_report(
            "dielectric-screening",
            [
                {"result": {"max_flux_density": 1.2e-3}},
                {"result": {"max_electric_field": 38.0e6, "max_flux_density": 3.3e-3}},
                {"max_electric_field": 48.0e6, "max_flux_density": 0.9e-3},
            ],
        )

        summary = report["reliability"]["summary"]
        self.assertEqual(summary["decision"], "blocked_by_quality_gates")
        self.assertEqual(summary["blocking_gate_ids"], ["gate.result_completeness"])
        self.assertEqual(
            report["reliability"]["quality_gates"][0]["status"],
            "violate",
        )

    def test_builds_composite_thermo_electric_report(self) -> None:
        report = build_material_report(
            "composite-thermo-electric-panel",
            [
                {"electrostatic": {"max_electric_field": 45.0e6}, "heat": {"max_temperature": 120.0}, "thermal": {"max_stress": 180.0e6}},
                {"electrostatic": {"max_electric_field": 52.0e6}, "heat": {"max_temperature": 98.0}, "thermal": {"max_stress": 140.0e6}},
                {"electrostatic": {"max_electric_field": 58.0e6}, "heat": {"max_temperature": 132.0}, "thermal": {"max_stress": 210.0e6}},
            ],
        )

        self.assertEqual(report["schema_version"], "kyuubiki.composite-panel-report/v1")
        self.assertEqual(report["study"], "material.composite_thermo_electric_panel.v1")
        self.assertEqual(len(report["candidates"]), 3)
        self.assertIn(
            report["reliability"]["summary"]["decision"],
            {"ready_for_next_round", "blocked_by_quality_gates"},
        )


if __name__ == "__main__":
    unittest.main()
