import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptStateAction } from "../../src/components/workbench/workbench-script-state-controller.ts";

type StateActionArgs = Parameters<typeof handleWorkbenchScriptStateAction>[0];

test("PWDT fullscreen validates tool tabs before any UI or native fullscreen mutation", async () => {
  const calls: string[] = [];
  for (const toolTab of ["unknown", "", null, 1, ["batch"], {}]) {
    await assert.rejects(handleWorkbenchScriptStateAction(actionArgs({
      action: "viewport/setUiState", payload: { immersiveViewport: true, toolTab, toolDrawerOpen: true },
      immersiveViewport: false,
      toggleImmersiveViewport: async () => { calls.push("fullscreen"); },
      setImmersiveToolTab: () => calls.push("tab"), setImmersiveToolDrawerOpen: () => calls.push("drawer"),
    })), /viewport:invalid_tool_tab/);
  }
  assert.deepEqual(calls, []);
});

test("PWDT fullscreen errors cannot masquerade as successful UI changes", async () => {
  const calls: string[] = [];
  await assert.rejects(handleWorkbenchScriptStateAction(actionArgs({
    action: "viewport/setUiState", payload: { immersiveViewport: true, toolTab: "batch", toolDrawerOpen: true },
    immersiveViewport: false, toggleImmersiveViewport: async () => { throw new Error("fullscreen denied"); },
    setImmersiveToolTab: () => calls.push("tab"), setImmersiveToolDrawerOpen: () => calls.push("drawer"),
  })), /fullscreen denied/);
  assert.deepEqual(calls, []);
});

test("PWDT exposes every fullscreen editing tab without re-entering an active fullscreen", async () => {
  for (const toolTab of ["node", "props", "batch", "study", "save"] as const) {
    const calls: unknown[] = [];
    const result = await handleWorkbenchScriptStateAction(actionArgs({
      action: "viewport/setUiState", payload: { immersiveViewport: true, toolTab, toolDrawerOpen: true },
      immersiveViewport: true, toggleImmersiveViewport: async () => { calls.push("fullscreen"); },
      setImmersiveToolTab: (value) => calls.push(value), setImmersiveToolDrawerOpen: (value) => calls.push(value),
    }));
    assert.deepEqual(result, { ok: true, action: "viewport/setUiState" });
    assert.deepEqual(calls, [toolTab, true]);
  }
});

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
