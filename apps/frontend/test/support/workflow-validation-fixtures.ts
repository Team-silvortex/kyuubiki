export function buildWorkflowBridgeNormalizationGraph() {
  return {
    nodes: [
      {
        id: "bridge_a",
        config: {
          contract_normalization: [
            { field: "target.field", previous: "temp_peak", next: "thermal_temperature_max" },
            { field: "source.field", previous: "field_peak", next: "electrostatic_field_peak_magnitude" },
          ],
        },
      },
      {
        id: "bridge_b",
        config: {
          contract_normalization: [
            { field: "transform.reduction", previous: "max", next: "mean" },
          ],
        },
      },
    ],
  };
}

export function buildWorkflowValidationFixGraph() {
  return {
    nodes: [
      {
        id: "guard",
        inputs: [{ id: "bundle", artifact_type: "artifact/json", dataset_value: "diagnostics_bundle" }],
        outputs: [{ id: "result", artifact_type: "artifact/json", dataset_value: "guard_result" }],
      },
    ],
    edges: [
      {
        id: "edge_1",
        artifact_type: "artifact/json",
        dataset_value: "diagnostics_bundle",
      },
    ],
    entry_inputs: [
      { node_id: "guard", artifact_type: "artifact/json" },
    ],
    output_artifacts: [
      { node_id: "guard", artifact_type: "artifact/json" },
    ],
  };
}
