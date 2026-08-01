from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kyuubiki_sdk import (
    MODEL_COLLABORATION_SCHEMA_VERSION,
    MODEL_PLAN_APPROVAL_SCHEMA_VERSION,
    MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    KyuubikiSession,
    ModelResearchExecutionError,
    SessionModelActionDispatcher,
    build_model_headless_plan,
    execute_model_headless_plan,
)


class FakeDispatcher:
    def __init__(self, fail_on: str | None = None) -> None:
        self.fail_on = fail_on
        self.seen: list[str] = []

    def dispatch_model_action(self, action: str, _payload: dict[str, Any]) -> dict[str, Any]:
        self.seen.append(action)
        if action == self.fail_on:
            raise RuntimeError("injected dispatcher failure")
        return {"authority": "test-dispatcher", "output": {"action": action, "ok": True}}


class FakeControlPlane:
    def __init__(self) -> None:
        self.seen: list[tuple[str, Any]] = []

    def health(self) -> dict[str, Any]:
        self.seen.append(("health", None))
        return {"status": "ok"}

    def submit_workflow_catalog_job(
        self, workflow_id: str, input_artifacts: dict[str, Any]
    ) -> dict[str, Any]:
        self.seen.append(("submit_workflow_catalog_job", workflow_id))
        return {"job": {"job_id": "job-python-research", "status": "queued"}}


class ModelResearchExecutionTests(unittest.TestCase):
    def test_rejects_unapproved_plan_before_dispatch(self) -> None:
        plan = build_model_headless_plan(_session(), _proposal())
        dispatcher = FakeDispatcher()

        with self.assertRaisesRegex(ModelResearchExecutionError, "exact caller-issued approval"):
            execute_model_headless_plan(dispatcher, plan, None, lambda _plan, _approval: True)
        self.assertEqual(dispatcher.seen, [])

    def test_rejects_unverified_approval_before_dispatch(self) -> None:
        plan = build_model_headless_plan(_session(), _proposal())
        dispatcher = FakeDispatcher()

        with self.assertRaisesRegex(ModelResearchExecutionError, "verifier rejected approval"):
            execute_model_headless_plan(
                dispatcher,
                plan,
                _approval(plan),
                lambda _plan, _approval: False,
            )
        self.assertEqual(dispatcher.seen, [])

    def test_executes_approved_plan_and_retains_authority(self) -> None:
        plan = build_model_headless_plan(_session(), _proposal())
        dispatcher = FakeDispatcher()

        receipt = execute_model_headless_plan(
            dispatcher,
            plan,
            _approval(plan),
            lambda _plan, _approval: True,
        )
        self.assertEqual(receipt["status"], "completed")
        self.assertEqual(receipt["completed_steps"], 2)
        self.assertEqual(receipt["records"][1]["authority"], "test-dispatcher")

    def test_retains_partial_failure(self) -> None:
        proposal = _proposal()
        proposal["calls"] = [
            {"action": "service_health", "payload": {}},
            {"action": "protocol_describe", "payload": {}},
        ]
        plan = build_model_headless_plan(_session(), proposal)
        dispatcher = FakeDispatcher("protocol_describe")

        receipt = execute_model_headless_plan(
            dispatcher, plan, None, lambda _plan, _approval: True
        )
        self.assertEqual(receipt["status"], "failed")
        self.assertEqual(receipt["failed_step"], 2)
        self.assertEqual(receipt["completed_steps"], 1)
        self.assertIn("injected dispatcher failure", receipt["records"][1]["error"])

    def test_failure_receipt_preserves_valid_utf8_with_byte_bound(self) -> None:
        proposal = _proposal()
        proposal["calls"] = [{"action": "service_health", "payload": {}}]
        plan = build_model_headless_plan(_session(), proposal)

        class UnicodeFailureDispatcher:
            def dispatch_model_action(
                self, _action: str, _payload: dict[str, Any]
            ) -> dict[str, Any]:
                raise RuntimeError("错" * 1_000)

        receipt = execute_model_headless_plan(
            UnicodeFailureDispatcher(),
            plan,
            None,
            lambda _plan, _approval: True,
        )
        error = receipt["records"][0]["error"]
        self.assertLessEqual(len(error.encode("utf-8")), 2_051)
        self.assertTrue(error.endswith("..."))

    def test_plan_rejects_malformed_payload_types(self) -> None:
        proposal = _proposal()
        proposal["calls"][1]["payload"]["workflow_id"] = 42
        proposal["calls"][1]["payload"]["input_artifacts"] = []

        plan = build_model_headless_plan(_session(), proposal)
        self.assertFalse(plan["ok"])
        self.assertTrue(any("non-empty string" in issue for issue in plan["issues"]))
        self.assertTrue(any("JSON object" in issue for issue in plan["issues"]))

    def test_session_dispatcher_uses_existing_client_routes(self) -> None:
        session = _session()
        plan = build_model_headless_plan(session, _proposal())
        control_plane = FakeControlPlane()
        dispatcher = SessionModelActionDispatcher(KyuubikiSession(control_plane=control_plane))

        receipt = execute_model_headless_plan(
            dispatcher,
            plan,
            _approval(plan),
            lambda _plan, _approval: True,
        )
        self.assertEqual(receipt["status"], "completed")
        self.assertEqual(
            control_plane.seen,
            [
                ("health", None),
                (
                    "submit_workflow_catalog_job",
                    "workflow.material-study-envelope-ranking-json",
                ),
            ],
        )

    def test_repository_fixtures_reach_approved_execution(self) -> None:
        schemas = Path(__file__).resolve().parents[3] / "schemas"
        session = json.loads(
            (schemas / "examples.model-collaboration-session.json").read_text()
        )
        proposal = json.loads((schemas / "examples.model-workflow-proposal.json").read_text())
        approval = json.loads((schemas / "examples.model-plan-approval.json").read_text())
        plan = build_model_headless_plan(session, proposal)

        receipt = execute_model_headless_plan(
            FakeDispatcher(), plan, approval, lambda _plan, _approval: True
        )
        self.assertEqual(receipt["status"], "completed")
        self.assertEqual(receipt["completed_steps"], len(proposal["calls"]))

    def test_execution_receipt_retains_narrow_job_binding(self) -> None:
        plan = {
            "schema_version": "kyuubiki.model-headless-plan/v1",
            "session_id": "python-research-session",
            "workflow_id": "workflow.material",
            "ok": True,
            "ready_without_confirmation": True,
            "issues": [],
            "steps": [
                {
                    "index": 1,
                    "action": "job_wait",
                    "category": "observation",
                    "risk": "normal",
                    "payload": {"job_id": "job-bound-001"},
                    "requires_confirmation": False,
                    "confirmation_reason": None,
                    "output_keys": ["job"],
                }
            ],
        }
        receipt = execute_model_headless_plan(
            FakeDispatcher(), plan, None, lambda _plan, _approval: True
        )
        self.assertEqual(receipt["records"][0]["job_id"], "job-bound-001")


def _session() -> dict[str, Any]:
    return {
        "schema_version": MODEL_COLLABORATION_SCHEMA_VERSION,
        "session_id": "python-research-session",
        "workflow_id": "workflow.material-study-envelope-ranking-json",
        "objective": "Run one bounded material screening study.",
        "created_at": "2026-08-01T00:00:00Z",
        "policy": {
            "allowed_actions": [
                "service_health",
                "protocol_describe",
                "workflow_submit_catalog",
            ],
            "allow_sensitive": True,
        },
    }


def _proposal() -> dict[str, Any]:
    return {
        "schema_version": MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
        "session_id": "python-research-session",
        "calls": [
            {"action": "service_health", "payload": {}},
            {
                "action": "workflow_submit_catalog",
                "payload": {
                    "workflow_id": "workflow.material-study-envelope-ranking-json",
                    "input_artifacts": {"material_rows": {"rows": []}},
                },
            },
        ],
    }


def _approval(plan: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": MODEL_PLAN_APPROVAL_SCHEMA_VERSION,
        "approval_id": "approval-python-test",
        "session_id": plan["session_id"],
        "workflow_id": plan["workflow_id"],
        "authority": "python-integration-test",
        "issued_at": "2026-08-01T00:01:00Z",
        "approved_steps": [{"index": 2, "action": "workflow_submit_catalog"}],
    }


if __name__ == "__main__":
    unittest.main()
