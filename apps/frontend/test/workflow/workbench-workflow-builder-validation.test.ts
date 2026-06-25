import test from "node:test";
import assert from "node:assert/strict";

import { applyAllWorkflowValidationFixes } from "../../src/components/workbench/workflow/workbench-workflow-builder-validation.ts";

test("applyAllWorkflowValidationFixes repairs a simple edge artifact mismatch", () => {
  const graph = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.validation-fix",
    name: "workflow validation fix",
    version: "1.11.5",
    nodes: [
      {
        id: "input_node",
        kind: "input",
        outputs: [{ id: "result", artifact_type: "artifact/json" }],
      },
      {
        id: "output_node",
        kind: "output",
        inputs: [{ id: "result", artifact_type: "artifact/json" }],
        outputs: [],
      },
    ],
    edges: [
      {
        id: "edge_1",
        from: { node: "input_node", port: "result" },
        to: { node: "output_node", port: "result" },
        artifact_type: "artifact/csv",
      },
    ],
    entry_inputs: [],
    output_artifacts: [],
  };

  const result = applyAllWorkflowValidationFixes(graph as never, [], [], []);

  assert.ok(result.graph);
  assert.equal(result.appliedCount, 1);
  assert.equal(result.appliedIssues.length, 1);
  assert.equal(result.graph?.edges?.[0]?.artifact_type, "artifact/json");
});
