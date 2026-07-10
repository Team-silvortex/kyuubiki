import test from "node:test";
import assert from "node:assert/strict";

import { createWorkbenchRuntimeBackedBackendServices } from "@/lib/workbench/backend-service-composer";
import type { RuntimeApiClient } from "@/lib/api/runtime-client";

function createFakeRuntimeClient(calls: string[]): RuntimeApiClient {
  return {
    cancelJob: async (jobId) => {
      calls.push(`cancel:${jobId}`);
      return { job_id: jobId, status: "cancelled" } as never;
    },
    createDirectMeshSolve: async () => {
      calls.push("direct-mesh");
      return { job_id: "mesh-job", direct_mesh: { endpoint: "127.0.0.1:5001" } } as never;
    },
    deleteJobRecord: async (jobId) => {
      calls.push(`delete:${jobId}`);
      return { deleted: true, job: { id: jobId } } as never;
    },
    fetchAssetStore: async () => ({ entries: [] }) as never,
    fetchAssetStoreEntry: async () => ({ entry: null }) as never,
    fetchAssetStoreSources: async () => ({ sources: [] }) as never,
    fetchDirectMeshAgents: async () => ({ agents: [], discovery: "test", endpoint_count: 0 }) as never,
    fetchHealth: async () => {
      calls.push("health");
      return { service: "fake", status: "ok" } as never;
    },
    fetchJobHistory: async () => {
      calls.push("history");
      return { jobs: [] } as never;
    },
    fetchJobStatus: async (jobId) => {
      calls.push(`job:${jobId}`);
      return { job_id: jobId, status: "completed" } as never;
    },
    fetchProtocolAgents: async () => ({ agents: [] }) as never,
    fetchRegisteredAgents: async () => ({ agents: [], summary: {} }) as never,
    fetchWorkflowCatalog: async () => {
      calls.push("catalog");
      return { workflows: [] } as never;
    },
    fetchWorkflowOperators: async () => ({ operators: [] }) as never,
    submitWorkflowCatalogJob: async (workflowId) => {
      calls.push(`workflow:${workflowId}`);
      return { job_id: "workflow-job" } as never;
    },
    submitWorkflowGraphJob: async () => ({ job_id: "graph-job" }) as never,
    updateJobRecord: async (jobId) => {
      calls.push(`update:${jobId}`);
      return { job_id: jobId } as never;
    },
  };
}

test("runtime-backed service composer routes services through one runtime client", async () => {
  const calls: string[] = [];
  const services = createWorkbenchRuntimeBackedBackendServices(createFakeRuntimeClient(calls));

  await services.runtimeStatus.fetchStatus({
    directMeshEndpointsText: "",
    directMeshSelectionMode: "healthiest",
    frontendRuntimeMode: "orchestrated_gui",
  });
  await services.jobHistory.fetchHistory();
  await services.workflow.fetchCatalog();
  await services.workflow.submitWorkflow({ workflowId: "wf-a", inputArtifacts: {} });
  await services.adminData.fetchJob("job-a");
  await services.studyRun.fetchJob("job-b");

  assert.deepEqual(calls, ["health", "history", "catalog", "workflow:wf-a", "job:job-a", "job:job-b"]);
});
