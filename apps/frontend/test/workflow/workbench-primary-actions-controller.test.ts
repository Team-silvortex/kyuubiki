import test from "node:test";
import assert from "node:assert/strict";

import { createWorkbenchPrimaryActionsController } from "../../src/components/workbench/workbench-primary-actions-controller.ts";

function controllerDeps(overrides: Record<string, unknown> = {}) {
  return {
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
