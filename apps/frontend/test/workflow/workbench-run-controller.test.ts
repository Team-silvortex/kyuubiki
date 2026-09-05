import test from "node:test";
import assert from "node:assert/strict";

import { runWorkbenchAnalysis } from "../../src/components/workbench/workbench-run-controller.ts";

type RunArgs = Parameters<typeof runWorkbenchAnalysis>[0];

function job(status: "queued" | "completed" | "failed", message?: string) {
  return {
    job_id: "job-a",
    status,
    worker_id: "worker-a",
    progress: status === "queued" ? 0 : 1,
    has_result: status === "completed",
    message,
  };
}

function runArgs(overrides: Partial<RunArgs> = {}): RunArgs {
  return {
    copy: {},
    directMeshEndpointsText: "agent:5001",
    directMeshSelectionMode: "healthiest",
    frontendRuntimeMode: "direct_mesh_gui",
    jobPollTokenRef: { current: 0 },
    labels: {
      precheckPrefix: "precheck failed",
      dispatching: "dispatching",
      directMeshEndpointsHelp: "endpoints required",
      directMeshCompleted: "direct complete",
      initialFailed: "run failed",
      pollingDetached: "polling detached",
      requestTimedOut: "request timed out",
    },
    refreshJobHistory: async () => {},
    runBackendService: {
      fetchJob: async () => ({ job: job("completed") }),
      submitRun: async () => ({
        backend: "direct_mesh",
        envelope: {
          direct_mesh: { endpoint: "agent:5001", progress_frames: [], strategy: "healthiest" },
          job: job("completed"),
          result: {},
        },
      }),
    },
    setDirectMeshExecution: () => {},
    setJob: () => {},
    setMessage: () => {},
    setResult: () => {},
    setSystemAlerts: () => {},
    studyKind: "truss_2d",
    trussDiagnostics: null,
    ...overrides,
  } as RunArgs;
}

test("run precheck returns an explicit failure before submission", async () => {
  let submitted = false;
  const result = await runWorkbenchAnalysis(runArgs({
    runBackendService: {
      fetchJob: async () => ({ job: job("completed") }),
      submitRun: async () => {
        submitted = true;
        throw new Error("must not submit");
      },
    },
    trussDiagnostics: { blockingMessages: ["unstable model"] } as RunArgs["trussDiagnostics"],
  }));

  assert.equal(result.ok, false);
  assert.equal(submitted, false);
  if (result.ok) assert.fail("precheck failure must not report success");
  assert.match(result.error.message, /unstable model/);
});

test("direct-mesh run reports its completed backend outcome", async () => {
  const result = await runWorkbenchAnalysis(runArgs());
  assert.deepEqual(result, {
    ok: true,
    backend: "direct_mesh",
    completion: "terminal",
    jobId: "job-a",
    status: "completed",
  });
});

test("direct-mesh run rejects non-terminal and result-free completion envelopes", async () => {
  await assert.rejects(
    runWorkbenchAnalysis(runArgs({
      runBackendService: {
        fetchJob: async () => ({ job: job("completed") }),
        submitRun: async () => ({
          backend: "direct_mesh",
          envelope: {
            direct_mesh: { endpoint: "agent:5001", progress_frames: [], strategy: "healthiest" },
            job: job("queued"),
          },
        }),
      },
    })),
    /did not return a terminal status/,
  );

  await assert.rejects(
    runWorkbenchAnalysis(runArgs({
      runBackendService: {
        fetchJob: async () => ({ job: job("completed") }),
        submitRun: async () => ({
          backend: "direct_mesh",
          envelope: {
            direct_mesh: { endpoint: "agent:5001", progress_frames: [], strategy: "healthiest" },
            job: job("completed"),
          },
        }),
      },
    })),
    /did not include a result/,
  );
});

test("orchestrated terminal failures reject instead of reporting run success", async () => {
  await assert.rejects(
    runWorkbenchAnalysis(runArgs({
      frontendRuntimeMode: "orchestrated_gui",
      runBackendService: {
        fetchJob: async () => ({ job: job("failed", "solver failed") }),
        submitRun: async () => ({ backend: "orchestrated", envelope: { job: job("queued") } }),
      },
    })),
    /solver failed/,
  );
});

test("orchestrated completion without a result rejects instead of reporting success", async () => {
  await assert.rejects(
    runWorkbenchAnalysis(runArgs({
      frontendRuntimeMode: "orchestrated_gui",
      runBackendService: {
        fetchJob: async () => ({ job: job("completed") }),
        submitRun: async () => ({ backend: "orchestrated", envelope: { job: job("queued") } }),
      },
    })),
    /did not include a result/,
  );
});
