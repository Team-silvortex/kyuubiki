from __future__ import annotations

import unittest

from kyuubiki_sdk import (
    WORKFLOW_DATASET_SCHEMA_VERSION,
    WORKFLOW_GRAPH_SCHEMA_VERSION,
    build_workflow_axis,
    build_workflow_dataset_contract,
    build_workflow_dataset_value,
    build_workflow_defaults,
    build_workflow_edge,
    build_workflow_graph,
    build_workflow_node,
    build_workflow_operator_fetch_entry,
    build_workflow_port,
    build_workflow_schema_ref,
    build_workflow_shape,
)


class WorkflowBuilderTest(unittest.TestCase):
    def test_builds_valid_dataset_contract(self) -> None:
        contract = build_workflow_dataset_contract(
            "dataset.demo/v1",
            version="1.0.0",
            values=[
                build_workflow_dataset_value(
                    "thermal_case",
                    data_class="study_model",
                    element_type="json_object",
                    shape=build_workflow_shape(axes=[build_workflow_axis("elements", semantic="mesh_element")]),
                    encoding="json",
                    schema_ref=build_workflow_schema_ref("kyuubiki.operator.demo.input", "1"),
                )
            ],
        )
        self.assertEqual(contract["schema_version"], WORKFLOW_DATASET_SCHEMA_VERSION)
        self.assertEqual(contract["values"][0]["id"], "thermal_case")

    def test_builds_valid_graph(self) -> None:
        dataset_contract = build_workflow_dataset_contract(
            "dataset.demo/v1",
            version="1.0.0",
            values=[
                build_workflow_dataset_value("thermal_case", data_class="study_model", element_type="json_object", shape={}),
                build_workflow_dataset_value("thermal_result", data_class="result", element_type="json_object", shape={}),
            ],
        )
        graph = build_workflow_graph(
            "workflow.demo",
            name="Demo workflow",
            version="1.0.0",
            entry_nodes=["input"],
            output_nodes=["output"],
            dataset_contract=dataset_contract,
            defaults=build_workflow_defaults(
                cache_policy="cached",
                orchestrated=False,
                dispatch_policy="central_fetch",
                placement_tags=["cpu"],
                required_capabilities=["solver.thermal"],
            ),
            dispatch_policy="central_fetch",
            operator_fetch_plan=[
                build_workflow_operator_fetch_entry(
                    "solve",
                    operator_id="solve.demo",
                    package_ref="kyuubiki://operators/solve.demo",
                    version="1.0.0",
                    integrity="sha256:demo",
                    cache_scope="agent",
                )
            ],
            placement_tags=["mesh-enabled"],
            required_capabilities=["artifact-cache"],
            nodes=[
                build_workflow_node(
                    "input",
                    kind="input",
                    inputs=[],
                    outputs=[build_workflow_port("case", artifact_type="study_model/demo", dataset_value="thermal_case")],
                ),
                build_workflow_node(
                    "solve",
                    kind="solve",
                    operator_id="solve.demo",
                    inputs=[build_workflow_port("case", artifact_type="study_model/demo", dataset_value="thermal_case")],
                    outputs=[build_workflow_port("result", artifact_type="result/demo", dataset_value="thermal_result")],
                    placement_tags=["gpu-preferred"],
                    required_capabilities=["solver.thermal"],
                ),
                build_workflow_node(
                    "output",
                    kind="output",
                    inputs=[build_workflow_port("result", artifact_type="result/demo", dataset_value="thermal_result")],
                    outputs=[],
                ),
            ],
            edges=[
                build_workflow_edge("edge-1", from_node="input", from_port="case", to_node="solve", to_port="case", artifact_type="study_model/demo", dataset_value="thermal_case"),
                build_workflow_edge("edge-2", from_node="solve", from_port="result", to_node="output", to_port="result", artifact_type="result/demo", dataset_value="thermal_result"),
            ],
        )
        self.assertEqual(graph["schema_version"], WORKFLOW_GRAPH_SCHEMA_VERSION)
        self.assertEqual(len(graph["edges"]), 2)
        self.assertEqual(graph["dispatch_policy"], "central_fetch")
        self.assertFalse(graph["defaults"]["orchestrated"])
        self.assertEqual(graph["operator_fetch_plan"][0]["node_id"], "solve")


if __name__ == "__main__":
    unittest.main()
