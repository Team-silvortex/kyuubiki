import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";

import { runWorkbenchAnalysis } from "../../src/components/workbench/workbench-run-controller.ts";

type RunArgs = Parameters<typeof runWorkbenchAnalysis>[0];

function useImmediateBrowserTimers(t: TestContext) {
  const previousWindow = globalThis.window;
  globalThis.window = { setTimeout: (callback: () => void) => setTimeout(callback, 0) } as unknown as Window & typeof globalThis;
  t.after(() => { globalThis.window = previousWindow; });
}

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

for (const backend of ["direct_mesh", "orchestrated"] as const) {
  test(`${backend} run rejects a superseded submission before applying its response`, async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const token = { current: 0 };
    const writes: unknown[] = [];
    const operation = runWorkbenchAnalysis(runArgs({
      jobPollTokenRef: token,
      setJob: (value) => writes.push(value), setResult: (value) => writes.push(value), setMessage: (value) => writes.push(value),
      runBackendService: {
        fetchJob: async () => ({ job: job("completed"), result: {} } as never),
        submitRun: async () => {
          await gate;
          return backend === "orchestrated" ? { backend, envelope: { job: job("queued") } } as never
            : runArgs().runBackendService!.submitRun({} as never);
        },
      },
    }));
    token.current += 1;
    writes.length = 0;
    release();
    await assert.rejects(operation, /superseded/u);
    assert.deepEqual(writes, []);
  });
}

for (const phase of ["before-poll", "terminal-refresh"]) {
  test(`orchestrated run cannot regain ownership after ${phase} is superseded`, async () => {
    const token = { current: 0 };
    const writes: unknown[] = [];
    let refreshes = 0;
    await assert.rejects(runWorkbenchAnalysis(runArgs({
      jobPollTokenRef: token,
      setJob: (value) => writes.push(value), setResult: (value) => writes.push(value), setMessage: (value) => writes.push(value),
      refreshJobHistory: async () => {
        if (++refreshes === (phase === "before-poll" ? 1 : 2)) { token.current += 1; writes.length = 0; }
      },
      runBackendService: {
        fetchJob: async () => ({ job: job("completed"), result: {} } as never),
        submitRun: async () => ({ backend: "orchestrated", envelope: { job: job("queued") } } as never),
      },
    })), /superseded/u);
    assert.deepEqual(writes, []);
  });
}

for (const backend of ["direct_mesh", "orchestrated"] as const) {
  test(`${backend} explicit null result is not a successful completion`, async () => {
    await assert.rejects(runWorkbenchAnalysis(runArgs({
      runBackendService: {
        submitRun: async () => backend === "orchestrated"
          ? { backend, envelope: { job: job("queued") } } as never
          : { backend, envelope: { job: job("completed"), result: null,
            direct_mesh: { endpoint: "agent:5001", strategy: "healthiest" } } } as never,
        fetchJob: async () => ({ job: job("completed"), result: null } as never),
      },
    })), /did not include a result/u);
  });
}

test("polling rejects another job's response before applying its job or result", async (t) => {
  useImmediateBrowserTimers(t);
  const writes: unknown[] = [];
  let polls = 0;
  await assert.rejects(runWorkbenchAnalysis(runArgs({
    setJob: (value) => writes.push(value), setResult: (value) => writes.push(value),
    runBackendService: {
      submitRun: async () => ({ backend: "orchestrated", envelope: { job: job("queued") } }),
      fetchJob: async () => { polls += 1; return { job: { ...job("completed"), job_id: "wrong-job" }, result: {} } as never; },
    },
  })), /different job/u);
  assert.equal(writes.some((value: any) => value?.job_id === "wrong-job"), false);
  assert.equal(writes.some((value: any) => value && !value.job_id), false);
  assert.equal(polls, 3, "identity failures use the bounded observation retry budget");
});

test("polling recovers from a mismatched frame without exposing it or resubmitting", async (t) => {
  useImmediateBrowserTimers(t);
  const writes: any[] = [];
  const expected = { correct: true };
  let polls = 0;
  let submissions = 0;
  const outcome = await runWorkbenchAnalysis(runArgs({
    setJob: (value) => writes.push(value), setResult: (value) => writes.push(value),
    runBackendService: {
      submitRun: async () => { submissions += 1; return { backend: "orchestrated", envelope: { job: job("queued") } }; },
      fetchJob: async () => ++polls === 1
        ? { job: { ...job("completed"), job_id: "wrong-job" }, result: { foreign: true } } as never
        : { job: job("completed"), result: expected } as never,
    },
  }));
  assert.equal(outcome.ok, true);
  assert.equal(submissions, 1);
  assert.equal(polls, 2);
  assert.equal(writes.some((value) => value?.job_id === "wrong-job" || value?.foreign), false);
  assert.equal(writes.at(-1), expected);
});

for (const absent of [null, undefined]) {
  test(`terminal ${absent} result clears an earlier progress result and rejects completion`, async (t) => {
    useImmediateBrowserTimers(t);
    const results: unknown[] = [];
    const preview = { partial: true };
    let polls = 0;
    await assert.rejects(runWorkbenchAnalysis(runArgs({
      setResult: (value) => results.push(value),
      runBackendService: {
        submitRun: async () => ({ backend: "orchestrated", envelope: { job: job("queued") } }),
        fetchJob: async () => ++polls === 1
          ? { job: job("queued"), result: preview } as never
          : { job: job("completed"), result: absent } as never,
      },
    })), /did not include a result/u);
    assert.deepEqual(results, [null, preview, null]);
    assert.equal(polls, 2);
  });
}
