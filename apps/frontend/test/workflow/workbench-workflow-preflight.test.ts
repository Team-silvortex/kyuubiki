import test from "node:test";
import assert from "node:assert/strict";

import { buildWorkbenchWorkflowPreflightReport } from "../../src/components/workbench/workflow/workbench-workflow-preflight.ts";

const actionContract = {
  id: "transform.fixture",
  risk: "normal" as const,
  summary: { en: "Fixture transform" },
  payloadExample: {},
  inputSchema: [],
  outputSchema: [{ key: "result", label: "Result" }],
};

test("buildWorkbenchWorkflowPreflightReport accepts valid headless-ready workflow", () => {
  const workflow = {
    id: "workflow.preflight-ready",
    name: "Preflight ready",
    version: "2.0.0",
    summary: "ready",
    entry_inputs: [],
    output_artifacts: [],
    graph: {
      schema_version: "kyuubiki.workflow-graph/v1",
      id: "workflow.preflight-ready",
      version: "2.0.0",
      dispatch_policy: "inline_graph",
      nodes: [
        {
          id: "node_1",
          kind: "transform",
          operator_id: "transform.fixture",
          inputs: [],
          outputs: [{ id: "result", artifact_type: "artifact/json" }],
        },
      ],
      edges: [],
    },
  };

  const report = buildWorkbenchWorkflowPreflightReport({
    actionMap: new Map([[actionContract.id, actionContract]]),
    workflow: workflow as never,
  });

  assert.equal(report.ok, true);
  assert.equal(report.status, "ready");
  assert.equal(report.headless_ready, true);
  assert.equal(report.headless_step_count, 1);
  assert.equal(report.headless_batch?.steps[0]?.action, "transform.fixture");
});

test("buildWorkbenchWorkflowPreflightReport blocks ui-only workflow", () => {
  const workflow = {
    id: "workflow.ui-only",
    name: "UI only",
    version: "2.0.0",
    summary: "blocked",
    entry_inputs: [],
    output_artifacts: [],
    graph: {
      schema_version: "kyuubiki.workflow-graph/v1",
      id: "workflow.ui-only",
      version: "2.0.0",
      dispatch_policy: "ui_only",
      nodes: [],
      edges: [],
    },
  };

  const report = buildWorkbenchWorkflowPreflightReport({ workflow: workflow as never });

  assert.equal(report.ok, false);
  assert.equal(report.status, "blocked");
  assert.equal(report.headless_ready, false);
  assert.equal(report.blocking_issue_count, 1);
  assert.ok(report.issues.some((issue) => issue.id === "runtime:dispatch:ui-only"));
});

test("buildWorkbenchWorkflowPreflightReport reports missing graph", () => {
  const report = buildWorkbenchWorkflowPreflightReport({
    workflow: {
      id: "workflow.missing-graph",
      name: "Missing graph",
      version: "2.0.0",
      summary: "missing",
      entry_inputs: [],
      output_artifacts: [],
    } as never,
  });

  assert.equal(report.ok, false);
  assert.equal(report.blocking_issue_count, 1);
  assert.equal(report.headless_step_count, 0);
  assert.ok(report.issues.some((issue) => issue.id === "workflow:graph:missing"));
});
