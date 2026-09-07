import test from "node:test";
import assert from "node:assert/strict";

import { createWorkbenchPrimaryActionsController } from "../../src/components/workbench/workbench-primary-actions-controller.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import { historyEffects, historyResult } from "../support/history-result-fixture.ts";

function controllerDeps(overrides: Record<string, unknown> = {}) {
  return {
    projectContext: createWorkbenchProjectContext({ projectId: "project", modelId: "model", versionId: "version" }),
    directMeshEndpointsText: "agent:5001",
    directMeshSelectionMode: "healthiest",
    frontendRuntimeMode: "direct_mesh_gui",
    jobPollTokenRef: { current: 0 },
    refreshJobHistory: async () => {},
    runBackendService: {
      fetchJob: async () => { throw new Error("not used"); },
      submitRun: async () => ({
        backend: "direct_mesh" as const,
        envelope: {
          direct_mesh: { endpoint: "agent:5001", progress_frames: [], strategy: "healthiest" as const },
          job: {
            job_id: "job-primary",
            status: "completed" as const,
            worker_id: "worker-a",
            progress: 1,
          },
          result: {},
        },
      }),
    },
    setDirectMeshExecution: () => {},
    setJob: () => {},
    setMessage: () => {},
    setResult: () => {},
    setSystemAlerts: () => {},
    startTransition: (callback: () => void) => callback(),
    studyKind: "truss_2d",
    t: {
      directMeshCompleted: "direct complete",
      directMeshEndpointsHelp: "endpoints required",
      dispatching: "dispatching",
      initialFailed: "run failed",
      pollingDetached: "polling detached",
      precheckPrefix: "precheck failed",
      requestTimedOut: "request timed out",
    },
    trussDiagnostics: null,
    ...overrides,
  } as unknown as Parameters<typeof createWorkbenchPrimaryActionsController>[0];
}

test("primary run action remains awaitable through a React transition", async () => {
  let release!: () => void;
  let settled = false;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const controller = createWorkbenchPrimaryActionsController(controllerDeps({
    runBackendService: {
      fetchJob: async () => { throw new Error("not used"); },
      submitRun: async () => {
        await gate;
        return controllerDeps().runBackendService!.submitRun({} as never);
      },
    },
  }));
  const operation = controller.runAnalysis();
  void operation.then(() => { settled = true; });

  await Promise.resolve();
  assert.equal(settled, false);
  release();
  const result = await operation;
  assert.equal(result.ok, true);
  if (!("jobId" in result)) assert.fail("run outcome must expose its job id");
  assert.equal(result.jobId, "job-primary");
});

test("primary run action converts backend errors into explicit failures", async () => {
  const messages: string[] = [];
  const controller = createWorkbenchPrimaryActionsController(controllerDeps({
    runBackendService: {
      fetchJob: async () => { throw new Error("not used"); },
      submitRun: async () => { throw new Error("backend unavailable"); },
    },
    setMessage: (message: string) => messages.push(message),
  }));

  const result = await controller.runAnalysis();
  assert.equal(result.ok, false);
  if (result.ok) assert.fail("backend failure must not report success");
  assert.equal(result.error.message, "backend unavailable");
  assert.deepEqual(messages, ["dispatching", "backend unavailable"]);
});

for (const failed of [false, true]) {
  test(`superseded primary run ${failed ? "failure" : "success"} cannot replace restored feedback`, async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const token = { current: 0 };
    const writes: unknown[] = [];
    const controller = createWorkbenchPrimaryActionsController(controllerDeps({
      jobPollTokenRef: token,
      setJob: (value: unknown) => writes.push(value), setResult: (value: unknown) => writes.push(value),
      setMessage: (value: unknown) => writes.push(value), setSystemAlerts: (value: unknown) => writes.push(value),
      runBackendService: {
        fetchJob: async () => { throw new Error("not used"); },
        submitRun: async () => {
          await gate;
          if (failed) throw new Error("previous run unavailable");
          return controllerDeps().runBackendService!.submitRun({} as never);
        },
      },
    }));
    const operation = controller.runAnalysis();
    token.current += 1;
    writes.length = 0;
    release();
    const outcome = await operation;
    assert.equal(outcome.ok, false);
    assert.deepEqual(writes, []);
  });
}

test("primary run cannot report owned completion after result refresh is superseded", async () => {
  const token = { current: 0 };
  const controller = createWorkbenchPrimaryActionsController(controllerDeps({
    jobPollTokenRef: token,
    resultRefreshSeqRef: { current: 0 },
    setResultRecords: () => {}, setSelectedAdminResultJobId: () => {},
    adminDataBackendService: { fetchResults: async () => { token.current += 1; return { results: [] }; } },
    runBackendService: {
      submitRun: async () => ({ backend: "orchestrated", envelope: { job: { job_id: "archive-job", status: "queued" } } }),
      fetchJob: async () => ({ job: { job_id: "archive-job", status: "completed" }, result: {} }),
    },
  }));
  const result = await controller.runAnalysis();
  assert.equal(result.ok, false);
  if (result.ok) assert.fail("a replaced run must not continue a recipe");
  assert.match(result.error.message, /superseded/u);
});

for (const replacement of ["job", "workspace"]) {
  for (const failed of [false, true]) {
    test(`history open ${failed ? "failure" : "success"} cannot overwrite a newer ${replacement}`, async () => {
      let release!: () => void;
      const gate = new Promise<void>((resolve) => { release = resolve; });
      const { effects, writes } = historyEffects();
      const projectContext = createWorkbenchProjectContext({ projectId: "project", modelId: "model", versionId: "version" });
      const controller = createWorkbenchPrimaryActionsController(controllerDeps({ ...effects, projectContext,
        setSelectedModelId: (value: unknown) => writes.push(["model", value]),
        setSelectedVersionId: (value: unknown) => writes.push(["version", value]),
        setModelVersions: (value: unknown) => writes.push(["versions", value]),
        setSelectedNode: () => {}, setSelectedElement: () => {}, setMemberDraftNodes: () => {}, setLoadedModelName: () => {},
        adminDataBackendService: { fetchJob: async (id: string) => {
          if (id === "old") { await gate; if (failed) throw new Error("old history unavailable"); }
          return { job: { job_id: id, status: "completed", progress: 1 }, result: historyResult("heat_bar_1d") };
        } },
      }));
      const older = controller.openHistoryJob("old");
      if (replacement === "job") await controller.openHistoryJob("new");
      else projectContext.begin();
      await new Promise((resolve) => setImmediate(resolve));
      writes.length = 0;
      release();
      await older;
      await new Promise((resolve) => setImmediate(resolve));
      assert.deepEqual(writes, []);
    });
  }
}

test("history open remains awaitable and rejects a mismatched response before replacing content", async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  let settled = false;
  const { effects, writes } = historyEffects();
  const controller = createWorkbenchPrimaryActionsController(controllerDeps({ ...effects,
    adminDataBackendService: { fetchJob: async () => {
      await gate;
      return { job: { job_id: "wrong", status: "completed", progress: 1 }, result: historyResult("heat_bar_1d") };
    } },
  }));
  const opening = controller.openHistoryJob("requested");
  void opening.then(() => { settled = true; });
  await Promise.resolve();
  assert.equal(settled, false);
  release();
  const outcome = await opening;
  assert.equal(outcome.ok, false);
  if (outcome.ok) assert.fail("a mismatched archive is not a successful open");
  assert.match(outcome.error.message, /HISTORY_RESULT_INVALID/u);
  assert.deepEqual(writes.map(([key]) => key), ["setMessage"]);
});
