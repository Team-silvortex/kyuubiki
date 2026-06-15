from __future__ import annotations

import json
import pathlib
import unittest

from kyuubiki_sdk import (
    WorkflowContractValidationError,
    build_workflow_output_manifest,
    normalize_workflow_progression,
    normalize_workflow_runtime,
    validate_workflow_result_against_graph,
)


SCHEMAS_DIR = pathlib.Path(__file__).resolve().parents[3] / "schemas"


def load_json(filename: str) -> dict:
    return json.loads((SCHEMAS_DIR / filename).read_text(encoding="utf-8"))


class WorkflowResultValidationTest(unittest.TestCase):
    def test_builds_output_manifest_from_graph(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        manifest = build_workflow_output_manifest(graph)
        self.assertEqual(manifest["graph_id"], "workflow.heat-to-thermo-quad-2d")
        self.assertEqual(manifest["outputs"][0]["key"], "thermo_summary.result")

    def test_validates_result_payload_with_artifact_type_fallback(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        validated = validate_workflow_result_against_graph(
            graph,
            {
                "result": {
                    "workflow_id": "workflow.demo",
                    "run_id": "run-1",
                    "status": "completed",
                    "current_node": "solve_thermo",
                    "completed_nodes": ["input", "solve_thermo"],
                    "progress_events": [{"node_id": "solve_thermo", "status": "completed"}],
                    "artifacts": {
                        "result/thermal_plane_quad_2d": {
                            "artifact_id": "artifact.thermo.result",
                            "artifact_type": "result/thermal_plane_quad_2d",
                            "dataset_value": "thermo_result",
                        }
                    }
                }
            },
        )
        self.assertEqual(
            validated["artifacts"]["thermo_summary.result"]["artifact_id"],
            "artifact.thermo.result",
        )
        self.assertEqual(validated["workflow_runtime"]["current_node"], "solve_thermo")

    def test_rejects_missing_required_output_artifact(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        with self.assertRaises(WorkflowContractValidationError) as context:
            validate_workflow_result_against_graph(graph, {"result": {"artifacts": {}}})
        self.assertIn("thermo_summary.result", str(context.exception))

    def test_normalizes_workflow_runtime(self) -> None:
        runtime = normalize_workflow_runtime(
            {
                "result": {
                    "workflow_id": "workflow.demo",
                    "run_id": "run-1",
                    "status": "running",
                    "current_node": "solve",
                    "completed_nodes": ["input"],
                    "progress_events": [{"node_id": "input", "status": "completed"}],
                }
            }
        )
        self.assertEqual(runtime["run_id"], "run-1")
        self.assertEqual(runtime["completed_nodes"], ["input"])

    def test_normalizes_workflow_progression(self) -> None:
        progression = normalize_workflow_progression(
            [
                {
                    "job": {
                        "job_id": "job-1",
                        "status": "running",
                        "progress": 0.5,
                        "current_node": "solve",
                        "completed_nodes": ["input"],
                        "progress_events": [{"node_id": "input", "status": "completed"}],
                    }
                }
            ],
            {
                "result": {
                    "workflow_id": "workflow.demo",
                    "run_id": "run-1",
                    "status": "completed",
                    "current_node": "output",
                    "completed_nodes": ["input", "solve", "output"],
                    "progress_events": [{"node_id": "output", "status": "completed"}],
                    "artifacts": {},
                }
            },
        )
        self.assertEqual(progression["snapshots"][0]["current_node"], "solve")
        self.assertEqual(progression["latest"]["run_id"], "run-1")
