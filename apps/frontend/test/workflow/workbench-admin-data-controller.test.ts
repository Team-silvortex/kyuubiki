import test from "node:test";
import assert from "node:assert/strict";

import {
  applyJobContextToWorkbench,
  openProjectContextById,
} from "../../src/components/workbench/workbench-admin-data-controller.ts";
import type { JobState } from "../../src/lib/api/fem-shared.ts";

type AdminDataDeps = Parameters<typeof openProjectContextById>[1];

function baseDeps(overrides: Partial<AdminDataDeps> = {}): AdminDataDeps {
  return {
    jobHistory: [],
    labels: {
      linkedProjectMissing: "linked project missing",
      linkedProjectOpened: "linked project opened",
      missingResultJob: "result missing",
      noJobProject: "job project missing",
      noJobVersion: "job version missing",
      noRecordContext: "record context missing",
      noResultProject: "result project missing",
      noResultVersion: "result version missing",
      recordContextApplied: "context applied",
      selectJobFirst: "select job",
    },
    openModelVersionById: async () => ({ ok: true }),
    projects: [{
      description: "context qualification",
      inserted_at: "2026-08-28T00:00:00.000Z",
      models: [{
        inserted_at: "2026-08-28T00:00:00.000Z",
        kind: "truss_2d",
        latest_version_id: "version-a",
        model_id: "model-a",
        model_schema_version: "kyuubiki.model/v1",
        name: "Model A",
        payload: {},
        project_id: "project-a",
        updated_at: "2026-08-28T00:00:00.000Z",
      }],
      name: "Project A",
      project_id: "project-a",
      updated_at: "2026-08-28T00:00:00.000Z",
    }],
    refreshVersions: async () => {},
    selectedAdminJob: null,
    selectedAdminJobId: null,
    selectedAdminResultJobId: null,
    setAdminFilterModelVersionId: () => {},
    setAdminFilterProjectId: () => {},
    setAdminJobCaseId: () => {},
    setLibraryTab: () => {},
    setMessage: () => {},
    setModelVersions: () => {},
    setSelectedModelId: () => {},
    setSelectedProjectId: () => {},
    setSelectedVersionId: () => {},
    setSidebarSection: () => {},
    ...overrides,
  };
}

test("project context waits for model-version refresh before reporting success", async () => {
  let release!: () => void;
  let settled = false;
  const messages: string[] = [];
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const operation = openProjectContextById("project-a", baseDeps({
    refreshVersions: async () => gate,
    setMessage: (message) => messages.push(message),
  }));
  void operation.then(() => { settled = true; });

  await Promise.resolve();
  assert.equal(settled, false);
  release();
  assert.deepEqual(await operation, { ok: true });
  assert.deepEqual(messages, ["linked project opened"]);
});

test("project context returns refresh failures instead of optimistic success", async () => {
  const failure = new Error("version refresh failed");
  const messages: string[] = [];
  const result = await openProjectContextById("project-a", baseDeps({
    refreshVersions: async () => { throw failure; },
    setMessage: (message) => messages.push(message),
  }));

  assert.equal(result.ok, false);
  if (result.ok) assert.fail("refresh failure must not report success");
  assert.equal(result.error, failure);
  assert.deepEqual(messages, [failure.message]);
});

test("job context propagates linked-version loading failures", async () => {
  const failure = new Error("linked version failed");
  const job: JobState = {
    has_result: true,
    job_id: "job-a",
    model_version_id: "version-a",
    progress: 1,
    project_id: "project-a",
    status: "completed",
    worker_id: "worker-a",
  };
  const result = await applyJobContextToWorkbench(job, baseDeps({
    openModelVersionById: async () => ({ ok: false, error: failure }),
  }));

  assert.equal(result.ok, false);
  if (result.ok) assert.fail("version failure must not report success");
  assert.equal(result.error, failure);
});
