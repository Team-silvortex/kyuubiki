from __future__ import annotations

import json
import socket
import struct
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

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
    workflow_catalog_job_payload = {
        "job": {
            "job_id": "workflow-catalog-job",
            "status": "completed",
            "progress": 1.0,
            "current_node": "output",
            "completed_nodes": ["input", "output"],
            "progress_events": [{"node_id": "output", "status": "completed"}],
        }
    }
    workflow_graph_job_payload = {
        "job": {
            "job_id": "workflow-graph-job",
            "status": "completed",
            "progress": 1.0,
            "current_node": "output",
            "completed_nodes": ["input", "solve", "output"],
            "progress_events": [{"node_id": "solve", "status": "completed"}],
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
        if self.path == "/api/v1/fem/axial-bar/jobs":
            self._respond(202, {"job": {"job_id": "job-axial", "status": "queued"}})
            return
        if self.path == "/api/v1/fem/thermal-frame-3d/jobs":
            self._respond(202, {"job": {"job_id": "job-thermal-frame-3d", "status": "queued"}})
            return
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
        parsed = urlparse(self.path)

        if parsed.path == "/api/v1/workflows/catalog":
            self._respond(200, self.workflow_catalog_payload)
            return
        if parsed.path == "/api/v1/workflows/catalog/workflow.test-graph":
            self._respond(
                200,
                {
                    "workflow": {
                        "id": "workflow.test-graph",
                        "graph": {
                            "schema_version": "kyuubiki.workflow-graph/v1",
                            "id": "workflow.test-graph",
                            "name": "Test Graph",
                            "version": "1.0.0",
                            "entry_nodes": ["input"],
                            "output_nodes": ["output"],
                            "nodes": [
                                {"id": "input", "kind": "input", "inputs": [], "outputs": [{"id": "mesh", "artifact_type": "mesh.input"}]},
                                {"id": "output", "kind": "output", "inputs": [{"id": "mesh_result", "artifact_type": "mesh.result"}], "outputs": []},
                            ],
                            "edges": [],
                        },
                    }
                },
            )
            return
        if parsed.path == "/api/v1/operators":
            query = parse_qs(parsed.query)
            if query.get("domain") == ["structural"]:
                self._respond(200, self.operator_catalog_payload)
            else:
                self._respond(200, self.operator_catalog_payload)
            return
        if parsed.path == "/api/v1/operators/solver.truss_2d":
            self._respond(200, {"operator": self.operator_catalog_payload["operators"][0]})
            return
        if parsed.path == "/api/v1/jobs/job-smoke":
            self._respond(200, self.job_payload)
            return
        if parsed.path == "/api/v1/jobs/workflow-catalog-job":
            self._respond(200, self.workflow_catalog_job_payload)
            return
        if parsed.path == "/api/v1/jobs/workflow-graph-job":
            self._respond(200, self.workflow_graph_job_payload)
            return
        if parsed.path == "/api/v1/results/job-smoke":
            self._respond(200, self.result_payload)
            return
        if parsed.path == "/api/v1/results/workflow-catalog-job":
            self._respond(
                200,
                {
                    "job_id": "workflow-catalog-job",
                    "result": {
                        "workflow_id": "workflow.test-graph",
                        "run_id": "run-workflow-catalog",
                        "status": "completed",
                        "current_node": "output",
                        "completed_nodes": ["input", "output"],
                        "progress_events": [{"node_id": "output", "status": "completed"}],
                        "artifacts": {
                            "mesh.result": {"artifact_id": "artifact.catalog.result"},
                        }
                    },
                },
            )
            return
        if parsed.path == "/api/v1/results/workflow-graph-job":
            self._respond(
                200,
                {
                    "job_id": "workflow-graph-job",
                    "result": {
                        "workflow_id": "workflow.test-inline",
                        "run_id": "run-workflow-graph",
                        "status": "completed",
                        "current_node": "output",
                        "completed_nodes": ["input", "solve", "output"],
                        "progress_events": [{"node_id": "solve", "status": "completed"}],
                        "artifacts": {
                            "mesh.result": {"artifact_id": "artifact.graph.result"},
                        }
                    },
                },
            )
            return
        if parsed.path == "/api/v1/results/job-smoke/chunks/nodes":
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
        filtered_operators = session.control_plane.list_workflow_operators({"domain": "structural"})
        self.assertEqual(filtered_operators["operators"][0]["family"], "solver")
        operator = session.control_plane.fetch_workflow_operator("solver.truss_2d")
        self.assertEqual(operator["operator"]["kind"], "solver")

        catalog_job = session.control_plane.submit_workflow_catalog_job("workflow.test-graph", {"mesh": {"project_id": "demo"}})
        self.assertEqual(catalog_job["job"]["job_id"], "workflow-catalog-job")
        catalog_descriptor = session.control_plane.fetch_workflow_catalog_workflow("workflow.test-graph")
        self.assertEqual(catalog_descriptor["workflow"]["graph"]["id"], "workflow.test-graph")

        graph_job = session.control_plane.submit_workflow_graph_job(
            {
                "schema_version": "kyuubiki.workflow-graph/v1",
                "id": "workflow.test-inline",
                "nodes": [{"id": "solve", "kind": "solver", "operator_id": "solver.truss_2d"}],
            },
            {"mesh": {"project_id": "demo"}},
        )
        self.assertEqual(graph_job["job"]["job_id"], "workflow-graph-job")

    def test_agent_client_runs_workflow_jobs(self) -> None:
        session = KyuubikiSession.from_control_plane(self.base_url)
        agent = KyuubikiAgentClient(session)

        catalog_outcome = agent.run_workflow_catalog(
            "workflow.test-graph",
            {"mesh": {"project_id": "demo"}},
            timeout_s=5.0,
        )
        self.assertEqual(catalog_outcome["terminal"]["job"]["status"], "completed")
        self.assertEqual(
            catalog_outcome["result"]["result"]["artifacts"]["mesh.result"]["artifact_id"],
            "artifact.catalog.result",
        )
        self.assertEqual(
            catalog_outcome["validated_outputs"]["artifacts"]["output.mesh_result"]["artifact_id"],
            "artifact.catalog.result",
        )
        self.assertEqual(catalog_outcome["workflow_runtime"]["run_id"], "run-workflow-catalog")
        self.assertEqual(catalog_outcome["workflow_progression"]["snapshots"][0]["current_node"], "output")

        graph_definition = {
            "schema_version": "kyuubiki.workflow-graph/v1",
            "id": "workflow.test-inline",
            "name": "Inline Graph",
            "version": "1.0.0",
            "entry_nodes": ["input"],
            "output_nodes": ["output"],
            "nodes": [
                {"id": "input", "kind": "input", "inputs": [], "outputs": [{"id": "mesh", "artifact_type": "mesh.input"}]},
                {"id": "solve", "kind": "solve", "operator_id": "solver.truss_2d", "inputs": [], "outputs": []},
                {"id": "output", "kind": "output", "inputs": [{"id": "mesh_result", "artifact_type": "mesh.result"}], "outputs": []},
            ],
            "edges": [],
        }
        graph_outcome = agent.run_workflow_graph(
            graph_definition,
            {"mesh": {"project_id": "demo"}},
            timeout_s=5.0,
        )
        self.assertEqual(graph_outcome["terminal"]["job"]["status"], "completed")
        self.assertEqual(
            graph_outcome["result"]["result"]["artifacts"]["mesh.result"]["artifact_id"],
            "artifact.graph.result",
        )
        self.assertEqual(graph_outcome["output_manifest"]["graph_id"], "workflow.test-inline")
        self.assertEqual(
            graph_outcome["validated_outputs"]["artifacts"]["output.mesh_result"]["artifact_id"],
            "artifact.graph.result",
        )
        self.assertEqual(graph_outcome["workflow_runtime"]["current_node"], "output")
        self.assertEqual(graph_outcome["workflow_progression"]["latest"]["run_id"], "run-workflow-graph")

    def test_session_supports_expanded_solve_kinds(self) -> None:
        session = KyuubikiSession.from_control_plane(self.base_url)

        axial = session.submit_job("axial_bar_1d", {"nodes": [], "elements": []})
        self.assertEqual(axial["job"]["job_id"], "job-axial")

        thermal_frame = session.submit_job("thermal_frame_3d", {"nodes": [], "elements": []})
        self.assertEqual(thermal_frame["job"]["job_id"], "job-thermal-frame-3d")

    def test_session_supports_direct_rpc_for_advanced_solve_kinds(self) -> None:
        listener = socket.create_server(("127.0.0.1", 0))
        host, port = listener.getsockname()
        expected_methods = [
            "solve_modal_frame_2d",
            "solve_nonlinear_spring_1d",
            "solve_contact_gap_1d",
            "solve_acoustic_bar_1d",
            "solve_stokes_flow_plane_quad_2d",
        ]

        def serve_once() -> None:
            for expected_method in expected_methods:
                conn, _addr = listener.accept()
                with conn:
                    size = struct.unpack(">I", _recv_exact(conn, 4))[0]
                    payload = json.loads(_recv_exact(conn, size).decode("utf-8"))
                    self.assertEqual(payload["method"], expected_method)
                    frame = json.dumps(
                        {
                            "ok": True,
                            "result": {
                                "solver": expected_method.removeprefix("solve_"),
                                "input": payload["params"],
                            },
                        }
                    ).encode("utf-8")
                    conn.sendall(struct.pack(">I", len(frame)) + frame)

        thread = threading.Thread(target=serve_once, daemon=True)
        thread.start()
        try:
            session = KyuubikiSession.from_endpoints(
                self.base_url,
                rpc_host=host,
                rpc_port=port,
            )
            modal = session.solve_direct("modal_frame_2d", {"nodes": [], "elements": []})
            nonlinear = session.solve_direct("nonlinear_spring_1d", {"nodes": [], "elements": []})
            contact = session.solve_direct("contact_gap_1d", {"nodes": [], "elements": [], "contacts": []})
            acoustic = session.solve_direct("acoustic_bar_1d", {"nodes": [], "elements": []})
            stokes = session.solve_direct("stokes_flow_quad_2d", {"nodes": [], "elements": []})
            self.assertEqual(modal["solver"], "modal_frame_2d")
            self.assertEqual(nonlinear["solver"], "nonlinear_spring_1d")
            self.assertEqual(contact["solver"], "contact_gap_1d")
            self.assertEqual(acoustic["solver"], "acoustic_bar_1d")
            self.assertEqual(stokes["solver"], "stokes_flow_plane_quad_2d")
        finally:
            listener.close()
            thread.join(timeout=1)


def _recv_exact(sock: socket.socket, size: int) -> bytes:
    chunks = bytearray()
    while len(chunks) < size:
        chunk = sock.recv(size - len(chunks))
        if not chunk:
            raise RuntimeError("rpc connection closed before frame completed")
        chunks.extend(chunk)
    return bytes(chunks)


if __name__ == "__main__":
    unittest.main()
