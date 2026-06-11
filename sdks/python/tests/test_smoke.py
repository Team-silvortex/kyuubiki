from __future__ import annotations

import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from kyuubiki_sdk import KyuubikiAgentClient, KyuubikiSession


class _SmokeHandler(BaseHTTPRequestHandler):
    workflow_catalog_payload = {
        "workflows": [
            {
                "id": "workflow.test-graph",
                "name": "Test Graph",
                "version": "1.0.0",
                "summary": "Smoke workflow",
                "entry_inputs": [{"node_id": "input", "artifact_type": "mesh.input", "description": "Mesh input"}],
                "output_artifacts": [{"node_id": "solve", "artifact_type": "mesh.result", "description": "Solve result"}],
            }
        ]
    }
    operator_catalog_payload = {
        "operators": [
            {
                "id": "solver.truss_2d",
                "version": "1.0.0",
                "domain": "structural",
                "family": "solver",
                "kind": "solver",
                "summary": "Smoke operator",
                "capability_tags": ["smoke"],
                "origin": "built_in",
                "input_schema": {"schema": "kyuubiki.study.truss_2d", "version": "v1"},
                "output_schema": {"schema": "kyuubiki.result.truss_2d", "version": "v1"},
                "inputs": [],
                "outputs": [],
                "validation": {"baseline_status": "verified", "baseline_cases": [], "smoke_paths": []},
            }
        ]
    }
    job_payload = {
        "job": {
            "job_id": "job-smoke",
            "status": "completed",
            "progress": 1.0,
        }
    }
    result_payload = {
        "job_id": "job-smoke",
        "result": {
            "nodes": [
                {"index": 0, "id": "n0"},
                {"index": 1, "id": "n1"},
                {"index": 2, "id": "n2"},
            ],
            "elements": [{"index": 0, "id": "e0"}],
            "max_displacement": 1.0e-6,
            "max_stress": 7.0e4,
        },
    }

    def do_POST(self) -> None:
        if self.path == "/api/v1/fem/truss-2d/jobs":
            self._respond(202, {"job": {"job_id": "job-smoke", "status": "queued"}})
            return
        if self.path == "/api/v1/workflows/catalog/workflow.test-graph/jobs":
            self._respond(202, {"job": {"job_id": "workflow-catalog-job", "status": "queued"}})
            return
        if self.path == "/api/v1/workflows/graph/jobs":
            self._respond(202, {"job": {"job_id": "workflow-graph-job", "status": "queued"}})
            return
        self._respond(404, {"error": "not_found"})

    def do_GET(self) -> None:
        if self.path == "/api/v1/workflows/catalog":
            self._respond(200, self.workflow_catalog_payload)
            return
        if self.path == "/api/v1/operators":
            self._respond(200, self.operator_catalog_payload)
            return
        if self.path == "/api/v1/jobs/job-smoke":
            self._respond(200, self.job_payload)
            return
        if self.path == "/api/v1/results/job-smoke":
            self._respond(200, self.result_payload)
            return
        if self.path.startswith("/api/v1/results/job-smoke/chunks/nodes"):
            self._respond(
                200,
                {
                    "job_id": "job-smoke",
                    "kind": "nodes",
                    "offset": 0,
                    "limit": 2,
                    "returned": 2,
                    "total": 3,
                    "items": self.result_payload["result"]["nodes"][:2],
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


class SmokeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), _SmokeHandler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        host, port = cls.server.server_address
        cls.base_url = f"http://{host}:{port}"

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=1)

    def test_agent_client_run_study_and_chunk_browse(self) -> None:
        session = KyuubikiSession.from_control_plane(self.base_url)
        agent = KyuubikiAgentClient(session)

        outcome = agent.run_study("truss_2d", {"nodes": [], "elements": []}, timeout_s=5.0)
        self.assertEqual(outcome["terminal"]["job"]["status"], "completed")
        self.assertIsNotNone(outcome["result"])

        page = agent.browse_result_chunks("job-smoke", "nodes", offset=0, limit=2)
        self.assertEqual(page["returned"], 2)
        self.assertEqual(page["total"], 3)

    def test_control_plane_workflow_endpoints(self) -> None:
        session = KyuubikiSession.from_control_plane(self.base_url)
        catalog = session.control_plane.list_workflow_catalog()
        self.assertEqual(catalog["workflows"][0]["id"], "workflow.test-graph")

        operators = session.control_plane.list_workflow_operators()
        self.assertEqual(operators["operators"][0]["id"], "solver.truss_2d")

        catalog_job = session.control_plane.submit_workflow_catalog_job("workflow.test-graph", {"mesh": {"project_id": "demo"}})
        self.assertEqual(catalog_job["job"]["job_id"], "workflow-catalog-job")

        graph_job = session.control_plane.submit_workflow_graph_job(
            {
                "schema_version": "kyuubiki.workflow-graph/v1",
                "id": "workflow.test-inline",
                "nodes": [{"id": "solve", "kind": "solver", "operator_id": "solver.truss_2d"}],
            },
            {"mesh": {"project_id": "demo"}},
        )
        self.assertEqual(graph_job["job"]["job_id"], "workflow-graph-job")


if __name__ == "__main__":
    unittest.main()
