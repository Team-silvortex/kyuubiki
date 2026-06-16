import test from "node:test";
import assert from "node:assert/strict";

import { applyWorkflowGraphFix } from "../../src/components/workbench/workflow/workbench-workflow-validation-fix.ts";
import { buildWorkflowValidationFixGraph } from "../support/workflow-validation-fixtures.ts";

test("applyWorkflowGraphFix updates edge artifact type and clears edge dataset values", () => {
  const graph = buildWorkflowValidationFixGraph();
  const artifactFixed = applyWorkflowGraphFix(graph as never, {
    kind: "set_edge_artifact_type_from_source",
    edgeId: "edge_1",
    artifactType: "export/markdown",
  });
  const cleared = applyWorkflowGraphFix(artifactFixed as never, {
    kind: "clear_edge_dataset_value",
    edgeId: "edge_1",
  });

  assert.equal(cleared?.edges?.[0]?.artifact_type, "export/markdown");
  assert.equal(cleared?.edges?.[0]?.dataset_value, undefined);
});

test("applyWorkflowGraphFix updates node ports and catalog artifact types", () => {
  const graph = buildWorkflowValidationFixGraph();
  const nextGraph = applyWorkflowGraphFix(graph as never, {
    kind: "set_node_port_dataset_value_from_operator",
    nodeId: "guard",
    portId: "bundle",
    direction: "inputs",
    datasetValue: "report_payload",
  });
  const finalGraph = applyWorkflowGraphFix(nextGraph as never, {
    kind: "set_catalog_artifact_type",
    mode: "entry",
    nodeId: "guard",
    currentArtifactType: "artifact/json",
    artifactType: "export/markdown",
  });

  assert.equal(finalGraph?.nodes[0]?.inputs?.[0]?.dataset_value, "report_payload");
  assert.equal(finalGraph?.entry_inputs?.[0]?.artifact_type, "export/markdown");
});
