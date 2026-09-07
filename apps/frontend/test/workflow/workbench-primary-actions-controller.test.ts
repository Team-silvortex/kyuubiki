import test from "node:test";
import assert from "node:assert/strict";

import { createWorkbenchPrimaryActionsController } from "../../src/components/workbench/workbench-primary-actions-controller.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import { historyEffects, historyResult } from "../support/history-result-fixture.ts";
import { cancelWorkbenchJob } from "../../src/components/workbench/workbench-job-history-controller.ts";

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

for (const phase of ["submission", "polling"]) {
  for (const failure of ["network", "identity", "result"]) {
    test(`failed history ${failure} preserves the original ${phase} without resubmitting`, async () => {
      let release!: () => void;
      let reached!: () => void;
      const gate = new Promise<void>((resolve) => { release = resolve; });
      const pending = new Promise<void>((resolve) => { reached = resolve; });
      const token = { current: 0 };
      const writes: unknown[] = [];
      const result = historyResult("heat_bar_1d");
      let submissions = 0;
      const controller = createWorkbenchPrimaryActionsController(controllerDeps({
        jobPollTokenRef: token, resultRefreshSeqRef: { current: 0 },
        setResultRecords: () => {}, setSelectedAdminResultJobId: () => {},
        setJob: (value: unknown) => writes.push(value), setResult: (value: unknown) => writes.push(value),
        adminDataBackendService: {
          fetchResults: async () => ({ results: [] }),
          fetchJob: async () => {
            if (failure === "network") throw new Error("archive unavailable");
            return { job: { job_id: failure === "identity" ? "wrong" : "archive", status: "completed" },
              result: failure === "result" ? { input: null } : result };
          },
        },
        runBackendService: {
          submitRun: async () => {
            submissions += 1;
            if (phase === "submission") { reached(); await gate; }
            return { backend: "orchestrated", envelope: { job: { job_id: "live", status: "queued" } } };
          },
          fetchJob: async () => {
            if (phase === "polling") { reached(); await gate; }
            return { job: { job_id: "live", status: "completed" }, result };
          },
        },
      }));
      const running = controller.runAnalysis();
      await pending;
      const observation = token.current;
      const opened = await controller.openHistoryJob("archive");
      const afterOpen = token.current;
      release();
      const completed = await running;
      assert.equal(opened.ok, false);
      assert.equal(afterOpen, observation, "a failed read must not stop the live observation");
      assert.equal(completed.ok, true);
      assert.equal(submissions, 1);
      assert.equal(writes.at(-1), result);
    });
  }
}

for (const failed of [false, true]) {
  test(`a new run supersedes pending history ${failed ? "failure" : "success"}`, async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const { effects, writes } = historyEffects();
    const controller = createWorkbenchPrimaryActionsController(controllerDeps({ ...effects,
      adminDataBackendService: { fetchJob: async () => {
        await gate;
        if (failed) throw new Error("archive unavailable");
        return { job: { job_id: "archive", status: "completed" }, result: historyResult("heat_bar_1d") };
      } },
    }));
    const opening = controller.openHistoryJob("archive");
    assert.equal((await controller.runAnalysis()).ok, true);
    writes.length = 0;
    release();
    assert.equal((await opening).ok, false);
    assert.deepEqual(writes, []);
  });
}

for (const first of ["history", "cancellation"]) {
  for (const failed of [false, true]) {
    test(`concurrent history ${failed ? "failure" : "success"} and ${first}-first cancellation keep observation ownership`, async () => {
      let releaseHistory!: () => void;
      let releaseCancel!: () => void;
      const historyGate = new Promise<void>((resolve) => { releaseHistory = resolve; });
      const cancelGate = new Promise<void>((resolve) => { releaseCancel = resolve; });
      const token = { current: 0 };
      const { effects, writes } = historyEffects();
      const controller = createWorkbenchPrimaryActionsController(controllerDeps({ ...effects,
        jobPollTokenRef: token,
        setSelectedModelId: () => {}, setSelectedVersionId: () => {}, setModelVersions: () => {},
        setSelectedNode: () => {}, setSelectedElement: () => {}, setMemberDraftNodes: () => {}, setLoadedModelName: () => {},
        adminDataBackendService: { fetchJob: async () => {
          await historyGate;
          if (failed) throw new Error("archive unavailable");
          return { job: { job_id: "archive", status: "completed" }, result: historyResult("heat_bar_1d") };
        } },
      }));
      const cancelling = cancelWorkbenchJob({
        jobId: "live", jobPollTokenRef: token, setJob: effects.setJob, setResult: effects.setResult, setMessage: effects.setMessage,
        refreshJobHistory: async () => {}, labels: { jobCancelled: "cancelled", initialFailed: "failed", requestTimedOut: "timeout" },
        jobHistoryBackendService: {
          fetchHistory: async () => ({ jobs: [] }),
          cancelJob: async () => { await cancelGate; return { job: { job_id: "live", status: "cancelled", progress: 0, worker_id: "test" } }; },
        },
      });
      const opening = controller.openHistoryJob("archive");
      if (first === "history") {
        releaseHistory();
        const opened = await opening;
        writes.length = 0;
        releaseCancel();
        const cancelled = await cancelling;
        assert.equal(opened.ok, !failed);
        assert.equal(cancelled.ok, true);
        if (cancelled.ok) assert.equal(cancelled.contextChanged === true, !failed);
        assert.deepEqual(writes.filter(([key]) => key === "setJob").map(([, value]) => value.job_id), failed ? ["live"] : []);
      } else {
        releaseCancel();
        assert.equal((await cancelling).ok, true);
        writes.length = 0;
        releaseHistory();
        assert.equal((await opening).ok, false, "a pending read must not resurrect an observation changed by accepted cancellation");
        assert.deepEqual(writes, []);
      }
    });
  }
}
