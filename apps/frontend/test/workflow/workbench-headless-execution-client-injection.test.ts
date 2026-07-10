import test from "node:test";
import assert from "node:assert/strict";

import type { HeadlessResultsApiClient } from "@/lib/api/headless-results-client";
import type { ProjectApiClient } from "@/lib/api/project-client";
import type { RuntimeApiClient } from "@/lib/api/runtime-client";
import { runHeadlessExecutionBatch } from "@/lib/scripting/workbench-headless-execution";
import type { HeadlessWorkflowExecutionBatch } from "@/components/workbench/workbench-headless-workflow-export";

function literal(value: unknown) {
  return { kind: "literal" as const, value };
}

function batch(steps: HeadlessWorkflowExecutionBatch["steps"]): HeadlessWorkflowExecutionBatch {
  return {
    exported_at: "2026-07-10T00:00:00.000Z",
    language: "ts",
    schema_version: "kyuubiki.headless-execution-batch/v1",
    steps,
    warnings: [],
    workflow_id: "test-workflow",
  };
}

test("headless execution batch uses injected project and runtime clients", async () => {
  const calls: string[] = [];
  const projectClient = {
    createProject: async (input: { name: string }) => {
      calls.push(`project:${input.name}`);
      return { project: { project_id: "project-a", name: input.name } };
    },
    createModel: async (projectId: string, input: { name: string; kind: string }) => {
      calls.push(`model:${projectId}:${input.name}:${input.kind}`);
      return { model: { model_id: "model-a", kind: input.kind } };
    },
    createModelVersion: async (modelId: string, input: { payload: Record<string, unknown> }) => {
      calls.push(`version:${modelId}:${Object.keys(input.payload).join(",")}`);
      return { version: { version_id: "version-a", kind: "truss_3d" } };
    },
  } as unknown as ProjectApiClient;
  const runtimeClient = {
    fetchHealth: async () => {
      calls.push("health");
      return { service: "runtime", status: "ok" };
    },
  } as unknown as RuntimeApiClient;

  const result = await runHeadlessExecutionBatch(
    batch([
      { action: "service_health", guidanceNotes: [], index: 0, payload: literal({}), risk: "normal" },
      {
        action: "project_create",
        guidanceNotes: [],
        index: 1,
        payload: literal({ name: "Headless" }),
        risk: "normal",
      },
      {
        action: "model_create",
        guidanceNotes: [],
        index: 2,
        payload: literal({ kind: "truss_2d", name: "Model", project_id: "project-a", payload: {} }),
        risk: "normal",
      },
      {
        action: "model_version_create",
        guidanceNotes: [],
        index: 3,
        payload: literal({ model_id: "model-a", payload: { nodes: [] } }),
        risk: "normal",
      },
    ]),
    undefined,
    { projectClient, runtimeClient },
  );

  assert.deepEqual(calls, [
    "health",
    "project:Headless",
    "model:project-a:Model:truss_2d",
    "version:model-a:nodes",
  ]);
  assert.deepEqual(result.steps.map((step) => step.result), [
    { service: "runtime", status: "ok" },
    { project_id: "project-a", name: "Headless" },
    { model_id: "model-a", kind: "truss_2d" },
    { model_version_id: "version-a", kind: "truss_3d" },
  ]);
});

test("headless execution batch uses injected result client for direct mesh result reads", async () => {
  const calls: string[] = [];
  const headlessResultsClient = {
    fetchDirectMeshResultRecord: async (jobId: string) => {
      calls.push(`direct-result:${jobId}`);
      return { job_id: jobId, result: { energy: 42 } };
    },
  } as unknown as HeadlessResultsApiClient;

  const result = await runHeadlessExecutionBatch(
    batch([
      {
        action: "result_fetch",
        guidanceNotes: [],
        index: 0,
        payload: literal({ direct_mesh: true, job_id: "job-a" }),
        risk: "normal",
      },
    ]),
    undefined,
    { headlessResultsClient },
  );

  assert.deepEqual(calls, ["direct-result:job-a"]);
  assert.deepEqual(result.steps[0]?.result, { job_id: "job-a", result: { energy: 42 } });
});
