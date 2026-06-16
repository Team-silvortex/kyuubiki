import test from "node:test";
import assert from "node:assert/strict";

import { validateOperatorDescriptorContracts } from "../../src/components/workbench/workflow/workbench-workflow-validation-operator-descriptor.ts";

test("validateOperatorDescriptorContracts reports missing descriptor ports and mismatched datasets", () => {
  const graph = {
    nodes: [
      {
        id: "bundle",
        kind: "transform",
        operator_id: "transform.compose_diagnostics_bundle",
        inputs: [{ id: "electrostatic", artifact_type: "artifact/json", dataset_value: "wrong_value" }],
        outputs: [],
      },
    ],
  };
  const operatorDescriptors = [
    {
      id: "transform.compose_diagnostics_bundle",
      inputs: [
        { id: "electrostatic", artifact_type: "artifact/json", dataset_value: "electrostatic_diagnostics" },
        { id: "thermal", artifact_type: "artifact/json", dataset_value: "thermal_diagnostics" },
      ],
      outputs: [{ id: "result", artifact_type: "artifact/json", dataset_value: "diagnostics_bundle" }],
    },
  ];

  const issues = validateOperatorDescriptorContracts(graph as never, operatorDescriptors as never);

  assert.ok(issues.some((issue) => issue.id === "operator:dataset:bundle:inputs:electrostatic"));
  assert.ok(issues.some((issue) => issue.id === "operator:missing-port:bundle:inputs:thermal"));
  assert.ok(issues.some((issue) => issue.id === "operator:missing-port:bundle:outputs:result"));
});
