import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkflowIntegrityReport,
} from "../../src/components/workbench/workflow/workbench-workflow-integrity.ts";
import type {
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "../../src/lib/api/index.ts";

function graphWithOperator(operatorId = "solve.heat_plane_quad_2d"): WorkflowGraphDefinition {
  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.integrity",
    dataset_contract: {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: "dataset.integrity",
      version: "1.0.0",
      values: [
        {
          id: "result",
          data_class: "result",
          element_type: "json_object",
          shape: {},
        },
      ],
    },
    nodes: [
      {
        id: "solve",
        kind: "solve",
        operator_id: operatorId,
        outputs: [{ id: "result", artifact_type: "artifact/result", dataset_value: "result" }],
      },
    ],
    edges: [],
  };
}

function workflow(graph = graphWithOperator()): WorkflowCatalogEntry {
  return {
    id: "workflow.integrity",
    name: "Integrity workflow",
    version: "1.0.0",
    summary: "Integrity test fixture",
    graph,
    entry_inputs: [],
    output_artifacts: [],
  };
}

test("workflow integrity does not invent missing operators while the catalog is unavailable", () => {
  const report = buildWorkflowIntegrityReport(workflow(), []);

  assert.equal(
    report.issues.some((issue) => issue.id.startsWith("operator:missing-descriptor:")),
    false,
  );
});

test("workflow integrity reports an operator absent from a loaded catalog", () => {
  const unrelatedDescriptor = { id: "solve.other" } as WorkflowOperatorDescriptor;
  const report = buildWorkflowIntegrityReport(workflow(), [unrelatedDescriptor]);

  assert.ok(report.issues.some((issue) => issue.id === "operator:missing-descriptor:solve"));
});

test("workflow integrity can inspect the live draft instead of stale catalog graph state", () => {
  const draft = graphWithOperator();
  draft.nodes.push({ ...draft.nodes[0] });
  draft.schema_version = "";

  const report = buildWorkflowIntegrityReport(workflow(), [], draft);

  assert.ok(report.issues.some((issue) => issue.id === "graph:duplicate-node:solve"));
  assert.ok(report.issues.some((issue) => issue.id === "graph:missing-schema-version"));
});
