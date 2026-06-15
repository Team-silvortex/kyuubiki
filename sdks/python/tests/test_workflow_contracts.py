from __future__ import annotations

import json
import pathlib
import unittest

from kyuubiki_sdk import (
    WORKFLOW_DATASET_SCHEMA_VERSION,
    WORKFLOW_GRAPH_SCHEMA_VERSION,
    WorkflowContractValidationError,
    validate_workflow_dataset_contract,
    validate_workflow_graph,
)


SCHEMAS_DIR = pathlib.Path(__file__).resolve().parents[3] / "schemas"


def load_json(filename: str) -> dict:
    return json.loads((SCHEMAS_DIR / filename).read_text(encoding="utf-8"))


class WorkflowContractValidationTest(unittest.TestCase):
    def test_validates_reference_dataset_example(self) -> None:
        contract = load_json("examples.workflow-dataset.json")
        validated = validate_workflow_dataset_contract(contract)
        self.assertEqual(validated["schema_version"], WORKFLOW_DATASET_SCHEMA_VERSION)

    def test_validates_reference_graph_example(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        validated = validate_workflow_graph(graph)
        self.assertEqual(validated["schema_version"], WORKFLOW_GRAPH_SCHEMA_VERSION)

    def test_rejects_unknown_dataset_value_reference(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        graph["edges"][0]["dataset_value"] = "missing_value"
        with self.assertRaises(WorkflowContractValidationError) as context:
            validate_workflow_graph(graph)
        self.assertIn("missing_value", str(context.exception))

    def test_rejects_duplicate_dataset_value_ids(self) -> None:
        contract = load_json("examples.workflow-dataset.json")
        contract["values"].append(dict(contract["values"][0]))
        with self.assertRaises(WorkflowContractValidationError) as context:
            validate_workflow_dataset_contract(contract)
        self.assertIn("duplicate id", str(context.exception))

    def test_validates_execution_hints(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        graph["dispatch_policy"] = "central_fetch"
        graph["placement_tags"] = ["mesh-enabled"]
        graph["required_capabilities"] = ["artifact-cache"]
        graph["defaults"] = {
            "cache_policy": "cached",
            "orchestrated": False,
            "dispatch_policy": "central_fetch",
            "placement_tags": ["cpu"],
            "required_capabilities": ["solver.thermal"],
        }
        graph["operator_fetch_plan"] = [
            {
                "node_id": "thermal_solve",
                "operator_id": "solve.thermal.steady_state",
                "package_ref": "kyuubiki://operators/solve.thermal.steady_state",
                "version": "1.0.0",
                "integrity": "sha256:demo",
                "cache_scope": "agent",
            }
        ]
        graph["nodes"][1]["placement_tags"] = ["gpu-preferred"]
        graph["nodes"][1]["required_capabilities"] = ["solver.thermal"]
        validated = validate_workflow_graph(graph)
        self.assertEqual(validated["dispatch_policy"], "central_fetch")

    def test_rejects_invalid_dispatch_policy(self) -> None:
        graph = load_json("examples.workflow-graph.json")
        graph["dispatch_policy"] = "mystery_mode"
        with self.assertRaises(WorkflowContractValidationError) as context:
            validate_workflow_graph(graph)
        self.assertIn("dispatch_policy", str(context.exception))


if __name__ == "__main__":
    unittest.main()
