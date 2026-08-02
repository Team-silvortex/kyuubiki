from __future__ import annotations

import json
import pathlib
import unittest

from kyuubiki_sdk import (
    ModelResearchExecutionError,
    validate_model_research_frontier_result,
)


SCHEMAS = pathlib.Path(__file__).resolve().parents[3] / "schemas"


class ModelResearchValidationTests(unittest.TestCase):
    def test_validates_bound_result_without_overclaiming(self) -> None:
        report = validate_model_research_frontier_result(
            frontier(), receipt(result_payload()), graph(), None, allow, allow
        )
        self.assertEqual(report["stage"], "workflow_result_validated")
        self.assertEqual(report["claim_boundary"], "screening_only_not_qualification")
        self.assertTrue(report["external_validation_required"])
        self.assertEqual(report["workflow_result"]["artifact_keys"], ["thermo_summary.result"])

    def test_validates_retained_screening_bundle(self) -> None:
        bundle = load("examples.material-research-bundle.json")
        report = validate_model_research_frontier_result(
            frontier(), receipt(result_payload()), graph(), bundle, allow, allow
        )
        self.assertEqual(report["stage"], "screening_bundle_validated")
        self.assertEqual(report["material_bundle"]["bundle_id"], bundle["bundle_id"])
        self.assertIn("external_validation_required", report["next_actions"])

    def test_rejects_wrong_job_and_unverified_frontier(self) -> None:
        with self.assertRaisesRegex(ModelResearchExecutionError, "does not match"):
            validate_model_research_frontier_result(
                frontier(), receipt(result_payload(), "job-guessed"), graph(), None, allow, allow
            )
        with self.assertRaisesRegex(ModelResearchExecutionError, "verifier rejected"):
            validate_model_research_frontier_result(
                frontier(), receipt(result_payload()), graph(), None, deny, allow
            )

    def test_rejects_non_completed_runtime(self) -> None:
        with self.assertRaisesRegex(ModelResearchExecutionError, "status must be completed"):
            validate_model_research_frontier_result(
                frontier(), receipt(result_payload("running")), graph(), None, allow, allow
            )


def load(name: str) -> dict:
    return json.loads((SCHEMAS / name).read_text(encoding="utf-8"))


def graph() -> dict:
    return load("examples.workflow-graph.json")


def frontier() -> dict:
    return {
        "schema_version": "kyuubiki.model-research-frontier/v1",
        "session_id": "research-session",
        "workflow_id": "workflow.heat-to-thermo-quad-2d",
        "stage": "ready_to_validate",
        "job_id": "job-validation-001",
        "next_action": None,
        "transition_count": 3,
        "evidence": {},
        "blocking_reason": None,
    }


def result_payload(status: str = "completed") -> dict:
    return {
        "result": {
            "workflow_id": "workflow.heat-to-thermo-quad-2d",
            "run_id": "run-validation-001",
            "status": status,
            "artifacts": {
                "result/thermal_plane_quad_2d": {
                    "artifact_id": "artifact.thermo.result",
                    "artifact_type": "result/thermal_plane_quad_2d",
                    "dataset_value": "thermo_result",
                }
            },
        }
    }


def receipt(output: dict, job_id: str = "job-validation-001") -> dict:
    return {
        "schema_version": "kyuubiki.model-research-execution-receipt/v2",
        "plan_schema_version": "kyuubiki.model-headless-plan/v1",
        "session_id": "research-session",
        "workflow_id": "workflow.heat-to-thermo-quad-2d",
        "plan_digest": "sha256:" + "0" * 64,
        "status": "completed",
        "execution_authority": "kyuubiki-headless-sdk",
        "approval_id": "approval-test",
        "completed_steps": 1,
        "failed_step": None,
        "records": [{
            "index": 1,
            "action": "result_fetch",
            "job_id": job_id,
            "authority": "control_plane",
            "output": output,
            "error": None,
        }],
    }


def allow(_value: object) -> bool:
    return True


def deny(_value: object) -> bool:
    return False


if __name__ == "__main__":
    unittest.main()
