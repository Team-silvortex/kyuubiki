import test from "node:test";
import assert from "node:assert/strict";

import { pickConnectedPorts } from "../../src/components/workbench/workflow/workbench-workflow-topology-connection.ts";

test("pickConnectedPorts prefers dataset matches before artifact-only matches", () => {
  const sourceNode = {
    id: "source",
    outputs: [
      { id: "out_a", artifact_type: "artifact/json", dataset_value: "left_bundle" },
      { id: "out_b", artifact_type: "artifact/json", dataset_value: "other_bundle" },
    ],
  };
  const targetNode = {
    id: "target",
    inputs: [
      { id: "in_x", artifact_type: "artifact/json", dataset_value: "left_bundle" },
      { id: "in_y", artifact_type: "artifact/json", dataset_value: "fallback_bundle" },
    ],
  };

  const selected = pickConnectedPorts(sourceNode as never, targetNode as never);

  assert.equal(selected.sourcePort?.id, "out_a");
  assert.equal(selected.targetPort?.id, "in_x");
});

test("pickConnectedPorts falls back to artifact matches when dataset values differ", () => {
  const sourceNode = {
    id: "source",
    outputs: [{ id: "out_a", artifact_type: "artifact/json", dataset_value: "left_bundle" }],
  };
  const targetNode = {
    id: "target",
    inputs: [
      { id: "in_x", artifact_type: "artifact/csv", dataset_value: "csv_bundle" },
      { id: "in_y", artifact_type: "artifact/json", dataset_value: "different_bundle" },
    ],
  };

  const selected = pickConnectedPorts(sourceNode as never, targetNode as never);

  assert.equal(selected.sourcePort?.id, "out_a");
  assert.equal(selected.targetPort?.id, "in_y");
});
