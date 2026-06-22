import test from "node:test";
import assert from "node:assert/strict";

import {
  analyzeWorkflowGraphTopology,
  buildWorkflowGraphKernelIndex,
  selectFrontendKernelBackend,
} from "../../src/lib/workbench/frontend-kernel.ts";

test("selectFrontendKernelBackend protects cold starts and only uses wasm for warm hot paths", () => {
  assert.equal(
    selectFrontendKernelBackend({ workflowNodeCount: 16, workflowEdgeCount: 24 }).backend,
    "typescript",
  );

  const coldDecision = selectFrontendKernelBackend({
    workflowNodeCount: 1200,
    workflowEdgeCount: 2400,
    wasmReady: true,
    wasmWarmed: false,
  });
  assert.equal(coldDecision.backend, "typescript");
  assert.equal(coldDecision.coldStartProtected, true);

  const warmDecision = selectFrontendKernelBackend({
    workflowNodeCount: 1200,
    workflowEdgeCount: 2400,
    wasmReady: true,
    wasmWarmed: true,
  });
  assert.equal(warmDecision.backend, "wasm");
});

test("buildWorkflowGraphKernelIndex reports duplicate nodes and missing edge endpoints", () => {
  const index = buildWorkflowGraphKernelIndex({
    nodes: [{ id: "a", kind: "source" }, { id: "a", kind: "duplicate" }],
    edges: [
      {
        id: "edge.missing",
        from: { node: "a", port: "out" },
        to: { node: "missing", port: "in" },
        artifact_type: "artifact/json",
      },
    ],
  });

  assert.deepEqual(index.duplicateNodeIds, ["a"]);
  assert.deepEqual(index.missingEdgeNodeIds, ["missing"]);
  assert.equal(index.nodeById.get("a")?.kind, "source");
});

test("analyzeWorkflowGraphTopology detects cycles while preserving DAG order", () => {
  const dag = analyzeWorkflowGraphTopology({
    nodes: [
      { id: "a", kind: "source" },
      { id: "b", kind: "solve" },
      { id: "c", kind: "export" },
    ],
    edges: [
      {
        id: "a-b",
        from: { node: "a", port: "out" },
        to: { node: "b", port: "in" },
        artifact_type: "artifact/json",
      },
      {
        id: "b-c",
        from: { node: "b", port: "out" },
        to: { node: "c", port: "in" },
        artifact_type: "artifact/json",
      },
    ],
  });
  assert.equal(dag.hasCycle, false);
  assert.deepEqual(dag.topologicalNodeIds, ["a", "b", "c"]);

  const cycle = analyzeWorkflowGraphTopology({
    nodes: [{ id: "a", kind: "source" }, { id: "b", kind: "solve" }],
    edges: [
      {
        id: "a-b",
        from: { node: "a", port: "out" },
        to: { node: "b", port: "in" },
        artifact_type: "artifact/json",
      },
      {
        id: "b-a",
        from: { node: "b", port: "out" },
        to: { node: "a", port: "in" },
        artifact_type: "artifact/json",
      },
    ],
  });
  assert.equal(cycle.hasCycle, true);
  assert.deepEqual(cycle.cycleNodeIds, ["a", "b"]);
});
