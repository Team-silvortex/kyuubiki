import test from "node:test";
import assert from "node:assert/strict";

import {
  countWorkflowBridgeNormalizationAdjustments,
  readBridgeNormalizationEntries,
} from "../../src/components/workbench/workflow/workbench-workflow-bridge-normalization.ts";
import { buildWorkflowBridgeNormalizationGraph } from "../support/workflow-validation-fixtures.ts";

test("readBridgeNormalizationEntries keeps only valid normalization records", () => {
  const graph = buildWorkflowBridgeNormalizationGraph();
  const entries = readBridgeNormalizationEntries(graph.nodes[0] as never);

  assert.equal(entries.length, 2);
  assert.equal(entries[0]?.field, "target.field");
});

test("countWorkflowBridgeNormalizationAdjustments totals normalization entries across graph", () => {
  const graph = buildWorkflowBridgeNormalizationGraph();

  assert.equal(countWorkflowBridgeNormalizationAdjustments(graph as never), 3);
});
