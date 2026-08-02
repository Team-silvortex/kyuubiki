from __future__ import annotations

import json
import unittest
from pathlib import Path

from kyuubiki_sdk import (
    MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION,
    ModelResearchExecutionError,
    advance_model_research_frontier,
    build_model_research_frontier_proposal,
    start_model_research_frontier,
)


class ModelResearchFrontierTests(unittest.TestCase):
    def test_verified_submission_binds_real_job_id_into_next_proposal(self) -> None:
        frontier = start_model_research_frontier(
            receipt(
                "workflow_submit_catalog",
                output={"job": {"job_id": "job-real-001", "status": "queued"}},
            ),
            lambda _receipt: True,
        )
        self.assertEqual(frontier["stage"], "waiting_for_job")
        self.assertEqual(frontier["job_id"], "job-real-001")
        proposal = build_model_research_frontier_proposal(frontier, lambda _frontier: True)
        self.assertEqual(proposal["calls"][0]["action"], "job_wait")
        self.assertEqual(proposal["calls"][0]["payload"]["job_id"], "job-real-001")

    def test_unverified_receipt_cannot_create_frontier(self) -> None:
        submitted = receipt(
            "workflow_submit_graph", output={"job": {"job_id": "job-real-002"}}
        )
        with self.assertRaisesRegex(ModelResearchExecutionError, "receipt verifier rejected"):
            start_model_research_frontier(submitted, lambda _receipt: False)

    def test_wait_and_fetch_advance_without_guessing_ids(self) -> None:
        waiting = start_model_research_frontier(
            receipt("fem_submit", output={"job": {"job_id": "job-real-003"}}),
            lambda _receipt: True,
        )
        fetch = advance_model_research_frontier(
            waiting,
            receipt(
                "job_wait",
                job_id="job-real-003",
                output={
                    "terminal": {
                        "job": {"job_id": "job-real-003", "status": "completed"}
                    },
                    "history": [],
                },
            ),
            lambda _frontier: True,
            lambda _receipt: True,
        )
        self.assertEqual(fetch["stage"], "ready_to_fetch_result")
        proposal = build_model_research_frontier_proposal(fetch, lambda _frontier: True)
        self.assertEqual(proposal["calls"][0]["action"], "result_fetch")
        self.assertEqual(proposal["calls"][0]["payload"]["job_id"], "job-real-003")

        validate = advance_model_research_frontier(
            fetch,
            receipt(
                "result_fetch",
                job_id="job-real-003",
                output={"result": {"artifacts": []}},
            ),
            lambda _frontier: True,
            lambda _receipt: True,
        )
        self.assertEqual(validate["stage"], "ready_to_validate")
        self.assertIsNone(validate["next_action"])

    def test_mismatched_job_binding_is_rejected(self) -> None:
        waiting = start_model_research_frontier(
            receipt(
                "workflow_submit_catalog",
                output={"job": {"job_id": "job-real-004"}},
            ),
            lambda _receipt: True,
        )
        with self.assertRaisesRegex(ModelResearchExecutionError, "job_id does not match"):
            advance_model_research_frontier(
                waiting,
                receipt(
                    "job_wait",
                    job_id="job-guessed",
                    output={"terminal": {"job": {"status": "completed"}}},
                ),
                lambda _frontier: True,
                lambda _receipt: True,
            )

    def test_failures_block_progression(self) -> None:
        waiting = start_model_research_frontier(
            receipt(
                "workflow_submit_catalog",
                output={"job": {"job_id": "job-real-005"}},
            ),
            lambda _receipt: True,
        )
        blocked = advance_model_research_frontier(
            waiting,
            receipt(
                "job_wait",
                job_id="job-real-005",
                output={"terminal": {"job": {"status": "cancelled"}}},
            ),
            lambda _frontier: True,
            lambda _receipt: True,
        )
        self.assertEqual(blocked["stage"], "blocked")
        self.assertEqual(
            blocked["blocking_reason"], "job reached terminal status cancelled"
        )

        initial_failure = start_model_research_frontier(
            receipt(
                "workflow_submit_catalog",
                output=None,
                status="failed",
                error="control plane unavailable",
            ),
            lambda _receipt: True,
        )
        self.assertEqual(initial_failure["stage"], "blocked")
        self.assertEqual(initial_failure["blocking_reason"], "control plane unavailable")

    def test_repository_frontier_fixture_matches_sdk_contract(self) -> None:
        path = (
            Path(__file__).resolve().parents[3]
            / "schemas/examples.model-research-frontier.json"
        )
        frontier = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(
            frontier["schema_version"], MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION
        )
        proposal = build_model_research_frontier_proposal(frontier, lambda _frontier: True)
        self.assertEqual(
            proposal["calls"][0]["payload"]["job_id"],
            "job-material-envelope-001",
        )

    def test_inconsistent_frontier_state_is_rejected(self) -> None:
        frontier = start_model_research_frontier(
            receipt(
                "workflow_submit_catalog",
                output={"job": {"job_id": "job-real-006"}},
            ),
            lambda _receipt: True,
        )
        frontier["next_action"] = "result_fetch"
        with self.assertRaisesRegex(ModelResearchExecutionError, "stage and next action"):
            build_model_research_frontier_proposal(frontier, lambda _frontier: True)

    def test_unverified_frontier_cannot_generate_proposal(self) -> None:
        frontier = start_model_research_frontier(
            receipt(
                "workflow_submit_catalog",
                output={"job": {"job_id": "job-real-007"}},
            ),
            lambda _receipt: True,
        )
        with self.assertRaisesRegex(ModelResearchExecutionError, "frontier verifier rejected"):
            build_model_research_frontier_proposal(frontier, lambda _frontier: False)


def receipt(
    action: str,
    *,
    output: object,
    job_id: str | None = None,
    status: str = "completed",
    error: str | None = None,
) -> dict[str, object]:
    return {
        "schema_version": "kyuubiki.model-research-execution-receipt/v2",
        "plan_schema_version": "kyuubiki.model-headless-plan/v1",
        "session_id": "research-session",
        "workflow_id": "workflow.material",
        "plan_digest": "sha256:" + "0" * 64,
        "status": status,
        "execution_authority": "kyuubiki-headless-sdk",
        "approval_id": "approval-test",
        "completed_steps": 0 if error else 1,
        "failed_step": 1 if error else None,
        "records": [
            {
                "index": 1,
                "action": action,
                "job_id": job_id,
                "authority": None if error else "control_plane",
                "output": output,
                "error": error,
            }
        ],
    }


if __name__ == "__main__":
    unittest.main()
