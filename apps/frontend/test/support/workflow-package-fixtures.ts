export function buildImportedWorkflowGraph() {
  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.test",
    name: "workflow test",
    version: "1.12.0",
    nodes: [
      { id: "bundle", kind: "transform", operator_id: "transform.compose_diagnostics_bundle" },
      { id: "guard", kind: "transform", operator_id: "transform.evaluate_diagnostics_bundle_guard" },
    ],
    edges: [],
    entry_inputs: [
      { node_id: "electrostatic_input", artifact_type: "artifact/json" },
    ],
    output_artifacts: [
      { node_id: "markdown_output", artifact_type: "export/markdown" },
    ],
    dataset_contract: {
      id: "workflow.test.dataset",
      values: [{ id: "diagnostics_bundle" }, { id: "markdown_report" }],
    },
  };
}

export function buildImportedWorkflowPackage() {
  return {
    runtime_manifest: {
      required_operator_ids: [
        "transform.compose_diagnostics_bundle",
        "transform.evaluate_diagnostics_bundle_guard",
      ],
      sample_input_node_ids: ["electrostatic_input"],
      bridge_seed_summaries: [],
    },
    contract_manifest: {
      dataset_contract_id: "workflow.test.dataset",
      dataset_value_ids: ["diagnostics_bundle", "markdown_report"],
      entry_contracts: [
        {
          node_id: "electrostatic_input",
          artifact_type: "artifact/json",
          dataset_value: "diagnostics_bundle",
        },
      ],
      output_contracts: [
        {
          node_id: "markdown_output",
          artifact_type: "export/markdown",
          dataset_value: "markdown_report",
        },
      ],
    },
    workflow: {
      input_artifact_contract_warnings: {},
    },
  };
}
