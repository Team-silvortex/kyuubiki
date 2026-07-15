import test from "node:test";
import assert from "node:assert/strict";

import { createWorkflowBackendService } from "../../src/lib/workbench/workflow-backend-service-core.ts";
import type { JobEnvelope } from "../../src/lib/api/fem-shared.ts";
import type {
  WorkflowCatalogPayload,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
  WorkflowOperatorCatalogPayload,
} from "../../src/lib/api/workflow-types.ts";

function jobEnvelope(jobId: string): JobEnvelope<WorkflowGraphJobResult> {
  return {
    job: {
      created_at: "2026-06-29T00:00:00.000Z",
      job_id: jobId,
      progress: 0,
      status: "queued",
      updated_at: "2026-06-29T00:00:00.000Z",
      worker_id: null,
    },
  };
}

test("orchestrated workflow backend submits catalog workflows through catalog transport", async () => {
  const calls: string[] = [];
  const service = createWorkflowBackendService({
    fetchJob: async <TResult>(jobId: string) => jobEnvelope(jobId) as JobEnvelope<TResult>,
    fetchCatalog: async () => ({ workflows: [] }) satisfies WorkflowCatalogPayload,
    fetchOperators: async () => ({ modules: [], operators: [] }) satisfies WorkflowOperatorCatalogPayload,
    submitCatalogJob: async (workflowId, inputArtifacts) => {
      calls.push(`catalog:${workflowId}:${Object.keys(inputArtifacts).join(",")}`);
      return jobEnvelope("catalog-job");
    },
    submitGraphJob: async () => {
      calls.push("graph");
      return jobEnvelope("graph-job");
    },
  });

  const result = await service.submitWorkflow({
    inputArtifacts: { model: { id: "demo" } },
    workflowId: "thermal.catalog",
  });

  assert.equal(result.job.job_id, "catalog-job");
  assert.deepEqual(calls, ["catalog:thermal.catalog:model"]);
});

test("orchestrated workflow backend submits drafts through graph transport", async () => {
  const calls: string[] = [];
  const graph: WorkflowGraphDefinition = {
    edges: [],
    id: "draft.flow",
    nodes: [],
    schema_version: "kyuubiki.workflow-graph/v1",
  };
  const service = createWorkflowBackendService({
    fetchJob: async <TResult>(jobId: string) => jobEnvelope(jobId) as JobEnvelope<TResult>,
    fetchCatalog: async () => ({ workflows: [] }) satisfies WorkflowCatalogPayload,
    fetchOperators: async () => ({ modules: [], operators: [] }) satisfies WorkflowOperatorCatalogPayload,
    submitCatalogJob: async () => {
      calls.push("catalog");
      return jobEnvelope("catalog-job");
    },
    submitGraphJob: async (submittedGraph, inputArtifacts) => {
      calls.push(`graph:${submittedGraph.id}:${Object.keys(inputArtifacts).join(",")}`);
      return jobEnvelope("graph-job");
    },
  });

  const result = await service.submitWorkflow({
    graph,
    inputArtifacts: { heat_model: { id: "hm" } },
    workflowId: "fallback.workflow",
  });

  assert.equal(result.job.job_id, "graph-job");
  assert.deepEqual(calls, ["graph:draft.flow:heat_model"]);
});

test("orchestrated workflow backend keeps catalog/operator/job reads behind the service seam", async () => {
  const calls: string[] = [];
  const service = createWorkflowBackendService({
    fetchJob: async <TResult>(jobId: string) => {
      calls.push(`job:${jobId}`);
      return jobEnvelope(jobId) as JobEnvelope<TResult>;
    },
    fetchCatalog: async (query) => {
      calls.push(`catalog:${query?.q ?? ""}`);
      return { workflows: [] } satisfies WorkflowCatalogPayload;
    },
    fetchOperators: async (query) => {
      calls.push(`operators:${query?.q ?? ""}`);
      return { modules: [], operators: [] } satisfies WorkflowOperatorCatalogPayload;
    },
    submitCatalogJob: async () => jobEnvelope("catalog-job"),
    submitGraphJob: async () => jobEnvelope("graph-job"),
  });

  await service.fetchCatalog({ q: "thermal" });
  await service.fetchOperators({ q: "solve" });
  await service.fetchJob("job-42");

  assert.deepEqual(calls, ["catalog:thermal", "operators:solve", "job:job-42"]);
});

test("workflow backend service exposes injectable preflight seam", async () => {
  const service = createWorkflowBackendService({
    fetchJob: async <TResult>(jobId: string) => jobEnvelope(jobId) as JobEnvelope<TResult>,
    fetchCatalog: async () => ({ workflows: [] }) satisfies WorkflowCatalogPayload,
    fetchOperators: async () => ({ modules: [], operators: [] }) satisfies WorkflowOperatorCatalogPayload,
    preflightWorkflow: async ({ workflow }) => ({
      schema_version: "kyuubiki.workbench-workflow-preflight/v1",
      ok: true,
      status: "ready",
      workflow_id: workflow.id,
      issues: [],
    }),
    submitCatalogJob: async () => jobEnvelope("catalog-job"),
    submitGraphJob: async () => jobEnvelope("graph-job"),
  });

  const result = await service.preflightWorkflow({
    workflow: {
      entry_inputs: [],
      id: "workflow.ready",
      name: "Ready",
      output_artifacts: [],
      summary: "ready",
      version: "2.0.0",
    },
  });

  assert.equal(result.ok, true);
  assert.equal(result.workflow_id, "workflow.ready");
});

test("workflow backend service reports unconfigured preflight seam", async () => {
  const service = createWorkflowBackendService({
    fetchJob: async <TResult>(jobId: string) => jobEnvelope(jobId) as JobEnvelope<TResult>,
    fetchCatalog: async () => ({ workflows: [] }) satisfies WorkflowCatalogPayload,
    fetchOperators: async () => ({ modules: [], operators: [] }) satisfies WorkflowOperatorCatalogPayload,
    submitCatalogJob: async () => jobEnvelope("catalog-job"),
    submitGraphJob: async () => jobEnvelope("graph-job"),
  });

  const result = await service.preflightWorkflow({
    workflow: {
      entry_inputs: [],
      id: "workflow.not-configured",
      name: "Not configured",
      output_artifacts: [],
      summary: "blocked",
      version: "2.0.0",
    },
  });

  assert.equal(result.ok, false);
  assert.equal(result.status, "blocked");
  assert.equal(result.issues[0]?.id, "workflow:preflight:not-configured");
});
