import test from "node:test";
import assert from "node:assert/strict";

import { validateEdgeAndDatasetReferences } from "../../src/components/workbench/workflow/workbench-workflow-validation-edge.ts";

test("validateEdgeAndDatasetReferences reports edge artifact mismatches and missing dataset ids", () => {
  const graph = {
    dataset_contract: { values: [{ id: "known_value" }] },
    nodes: [
      {
        id: "source",
        outputs: [{ id: "result", artifact_type: "artifact/json", dataset_value: "known_value" }],
      },
      {
        id: "target",
        inputs: [{ id: "result", artifact_type: "artifact/json", dataset_value: "missing_value" }],
      },
    ],
    edges: [
      {
        id: "edge_1",
        from: { node: "source", port: "result" },
        to: { node: "target", port: "result" },
        artifact_type: "artifact/csv",
        dataset_value: "missing_value",
      },
    ],
  };

  const issues = validateEdgeAndDatasetReferences(graph as never);

  assert.ok(issues.some((issue) => issue.id === "edge-artifact-from:edge_1"));
  assert.ok(issues.some((issue) => issue.id === "edge-artifact-to:edge_1"));
  assert.ok(issues.some((issue) => issue.id === "edge-dataset:edge_1:missing_value"));
  assert.ok(issues.some((issue) => issue.id === "port-dataset:target:result:missing_value"));
});
