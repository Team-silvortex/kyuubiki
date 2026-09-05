import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptStateAction } from "../../src/components/workbench/workbench-script-state-controller.ts";

type StateActionArgs = Parameters<typeof handleWorkbenchScriptStateAction>[0];

function actionArgs(overrides: Partial<StateActionArgs>): StateActionArgs {
  return {
    action: "job/run",
    payload: {},
    ...overrides,
  } as StateActionArgs;
}

test("PWDT job/run waits for the observable run outcome", async () => {
  let release!: () => void;
  let settled = false;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const operation = handleWorkbenchScriptStateAction(actionArgs({
    runAnalysis: async () => {
      await gate;
      return {
        ok: true,
        backend: "orchestrated",
        completion: "terminal",
        jobId: "job-a",
        status: "completed",
      };
    },
  }));
  void operation.then(() => { settled = true; });

  await Promise.resolve();
  assert.equal(settled, false);
  release();
  assert.deepEqual(await operation, {
    ok: true,
    action: "job/run",
    backend: "orchestrated",
    completion: "terminal",
    jobId: "job-a",
    status: "completed",
  });
});

test("PWDT job/run propagates submission and execution failures", async () => {
  const failure = new Error("solver submission failed");
  await assert.rejects(
    handleWorkbenchScriptStateAction(actionArgs({
      runAnalysis: async () => ({ ok: false, error: failure }),
    })),
    failure,
  );
});

test("PWDT job/cancel waits for cancellation and propagates failures", async () => {
  const cancelled = await handleWorkbenchScriptStateAction(actionArgs({
    action: "job/cancel",
    cancelCurrentJob: async () => ({ ok: true, jobId: "job-a" }),
  }));
  assert.deepEqual(cancelled, { ok: true, action: "job/cancel", jobId: "job-a" });

  const failure = new Error("cancel failed");
  await assert.rejects(
    handleWorkbenchScriptStateAction(actionArgs({
      action: "job/cancel",
      cancelCurrentJob: async () => ({ ok: false, error: failure }),
    })),
    failure,
  );
});
