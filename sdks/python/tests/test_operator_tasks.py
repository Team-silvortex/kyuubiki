from __future__ import annotations

import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from kyuubiki_sdk import ControlPlaneClient


class _OperatorTaskHandler(BaseHTTPRequestHandler):
    observed_payload: dict | None = None

    def do_POST(self) -> None:
        content_length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(content_length).decode("utf-8")
        payload = json.loads(raw_body)
        type(self).observed_payload = payload

        if self.path == "/api/v1/operator-tasks/prepare-batch":
            self._respond(
                200,
                {
                    "status": "verified",
                    "operator_task_batch_preparation_contract": "kyuubiki.operator_task_batch_preparation/v1",
                    "task_count": 1,
                    "verified_count": 1,
                    "error_count": 0,
                    "summaries": [{"case_id": "case-a", "status": "verified"}],
                },
            )
            return

        if self.path == "/api/v1/operator-tasks/execute-batch":
            self._respond(
                200,
                {
                    "status": "executed",
                    "operator_task_batch_execution_contract": "kyuubiki.operator_task_batch_execution/v1",
                    "task_count": 1,
                    "ok_count": 1,
                    "error_count": 0,
                    "results": [
                        {
                            "case_id": "case-a",
                            "task_id": "task-a",
                            "status": "ok",
                            "result": {"material_thermal_shock_status": "pass"},
                        }
                    ],
                },
            )
            return

        if self.path == "/api/v1/operator-tasks/checkpoint-batch":
            self._respond(
                200,
                {
                    "operator_task_batch_checkpoint_contract": "kyuubiki.operator_task_batch_checkpoint/v1",
                    "batch_digest": "a" * 64,
                    "checkpoint_digest": "b" * 64,
                    "resume_policy": {"status": "prepared", "next_action": "execute"},
                    "case_index": [{"case_id": "case-a"}],
                },
            )
            return

        if self.path == "/api/v1/operator-tasks/verify-checkpoint-batch":
            self._respond(
                200,
                {
                    "operator_task_batch_checkpoint_verification_contract": "kyuubiki.operator_task_batch_checkpoint_verification/v1",
                    "status": "verified",
                    "batch_digest": "a" * 64,
                    "checkpoint_digest": "b" * 64,
                    "resume_policy": {"status": "prepared", "next_action": "execute"},
                },
            )
            return

        if self.path == "/api/v1/operator-tasks/resume-plan-batch":
            self._respond(
                200,
                {
                    "operator_task_batch_resume_plan_contract": "kyuubiki.operator_task_batch_resume_plan/v1",
                    "next_action": "execute",
                    "target_case_ids": ["case-a"],
                    "blocked_case_ids": [],
                },
            )
            return

        self._respond(404, {"error": "not_found"})

    def log_message(self, format: str, *args) -> None:
        return

    def _respond(self, status: int, payload: dict) -> None:
        encoded = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)


class OperatorTaskClientTest(unittest.TestCase):
    def setUp(self) -> None:
        _OperatorTaskHandler.observed_payload = None
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), _OperatorTaskHandler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        host, port = self.server.server_address
        self.base_url = f"http://{host}:{port}"

    def tearDown(self) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=1)

    def test_execute_operator_task_batch(self) -> None:
        client = ControlPlaneClient(self.base_url)
        batch = {
            "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
            "tasks": [
                {
                    "case_id": "case-a",
                    "task_ir": {
                        "schema_version": "kyuubiki.operator-task-ir/v1",
                        "task_id": "task-a",
                    },
                }
            ],
        }

        result = client.execute_operator_task_batch(batch)

        self.assertEqual(result["status"], "executed")
        self.assertEqual(result["ok_count"], 1)
        self.assertEqual(
            result["results"][0]["result"]["material_thermal_shock_status"],
            "pass",
        )
        self.assertEqual(
            _OperatorTaskHandler.observed_payload,
            {"batch": batch},
        )

    def test_checkpoint_operator_task_batch(self) -> None:
        client = ControlPlaneClient(self.base_url)
        batch = {
            "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
            "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}],
        }
        preparation = {"run_id": "prepare-run", "batch_digest": "a" * 64}

        result = client.checkpoint_operator_task_batch(batch, preparation=preparation)

        self.assertEqual(
            result["operator_task_batch_checkpoint_contract"],
            "kyuubiki.operator_task_batch_checkpoint/v1",
        )
        self.assertEqual(result["resume_policy"]["next_action"], "execute")
        self.assertEqual(
            _OperatorTaskHandler.observed_payload,
            {"batch": batch, "preparation": preparation},
        )

    def test_verify_operator_task_batch_checkpoint(self) -> None:
        client = ControlPlaneClient(self.base_url)
        batch = {
            "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
            "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}],
        }
        checkpoint = {
            "operator_task_batch_checkpoint_contract": "kyuubiki.operator_task_batch_checkpoint/v1",
            "batch_digest": "a" * 64,
            "checkpoint_digest": "b" * 64,
        }

        result = client.verify_operator_task_batch_checkpoint(batch, checkpoint)

        self.assertEqual(result["status"], "verified")
        self.assertEqual(
            result["operator_task_batch_checkpoint_verification_contract"],
            "kyuubiki.operator_task_batch_checkpoint_verification/v1",
        )
        self.assertEqual(
            _OperatorTaskHandler.observed_payload,
            {"batch": batch, "checkpoint": checkpoint},
        )

    def test_plan_operator_task_batch_resume(self) -> None:
        client = ControlPlaneClient(self.base_url)
        batch = {
            "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
            "tasks": [{"case_id": "case-a", "task_ir": {"task_id": "task-a"}}],
        }
        checkpoint = {
            "operator_task_batch_checkpoint_contract": "kyuubiki.operator_task_batch_checkpoint/v1",
            "batch_digest": "a" * 64,
            "checkpoint_digest": "b" * 64,
        }

        result = client.plan_operator_task_batch_resume(batch, checkpoint)

        self.assertEqual(
            result["operator_task_batch_resume_plan_contract"],
            "kyuubiki.operator_task_batch_resume_plan/v1",
        )
        self.assertEqual(result["target_case_ids"], ["case-a"])
        self.assertEqual(
            _OperatorTaskHandler.observed_payload,
            {"batch": batch, "checkpoint": checkpoint},
        )

    def test_prepare_operator_task_batch(self) -> None:
        client = ControlPlaneClient(self.base_url)
        batch = {
            "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
            "tasks": [
                {
                    "case_id": "case-a",
                    "task_ir": {
                        "schema_version": "kyuubiki.operator-task-ir/v1",
                        "task_id": "task-a",
                    },
                }
            ],
        }

        result = client.prepare_operator_task_batch(batch)

        self.assertEqual(result["status"], "verified")
        self.assertEqual(result["verified_count"], 1)
        self.assertEqual(result["summaries"][0]["case_id"], "case-a")
        self.assertEqual(
            _OperatorTaskHandler.observed_payload,
            {"batch": batch},
        )


if __name__ == "__main__":
    unittest.main()
