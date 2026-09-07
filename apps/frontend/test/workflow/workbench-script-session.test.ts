import assert from "node:assert/strict";
import test from "node:test";
import { createWorkbenchScriptSession } from "../../src/lib/scripting/workbench-script-session.ts";
import { buildLiveWorkbenchPythonBridge } from "../../src/lib/scripting/workbench-script-live-bridge.ts";
import { createWorkbenchPwdtBrowserBridge } from "../../src/lib/scripting/workbench-script-browser-bridge.ts";

test("PWDT execution status and bounded output survive panel unsubscribe and resubscribe", () => {
  const session = createWorkbenchScriptSession();
  let updates = 0;
  const unsubscribe = session.subscribe(() => { updates += 1; });
  session.setRuntimeStatus("running");
  assert.equal(session.isBusy(), true);
  unsubscribe();
  session.setOutput(Array.from({ length: 205 }, (_, i) => String(i)));
  session.setRuntimeError("retained error");
  session.setRuntimeStatus("error");
  assert.equal(updates, 1);
  assert.equal(session.getSnapshot().output.length, 200);
  assert.equal(session.getSnapshot().output[0], "5");
  assert.equal(session.getSnapshot().runtimeError, "retained error");
  assert.equal(session.isBusy(), false);
  const remount = session.subscribe(() => { updates += 1; });
  session.setRuntimeError(null);
  session.setRuntimeStatus("ready");
  assert.equal(updates, 3);
  assert.equal(session.getSnapshot().runtimeStatus, "ready");
  assert.equal(session.getSnapshot(), session.getSnapshot());
  assert.equal(session.getServerSnapshot().runtimeStatus, "idle");
  remount();
});

test("Python bridge reads live root state after its author panel has gone away", async () => {
  const previousWindow = globalThis.window;
  let snapshot = { jobStatus: null as string | null, sidebarSection: "system" };
  const calls: string[] = [];
  const controller = createWorkbenchPwdtBrowserBridge({
    getSnapshot: () => snapshot,
    invokeAction: async (action) => { calls.push(action); return { ok: true }; },
  });
  const fakeWindow = { __kyuubikiPwdt: controller } as Window & typeof globalThis;
  globalThis.window = fakeWindow;
  try {
    const bridge = buildLiveWorkbenchPythonBridge(() => {});
    assert.equal(JSON.parse(bridge.state_json()).jobStatus, null);
    snapshot = { jobStatus: "completed", sidebarSection: "model" };
    assert.equal(JSON.parse(bridge.state_json()).jobStatus, "completed");
    await bridge.invoke("runtime/refreshAll");
    assert.deepEqual(calls, ["runtime/refreshAll"]);
    delete fakeWindow.__kyuubikiPwdt;
    assert.throws(() => bridge.state_json(), /WORKBENCH_CONTEXT_CHANGED/);
    await assert.rejects(bridge.invoke("job/run"), /WORKBENCH_CONTEXT_CHANGED/);
    assert.equal(calls.length, 1);
    assert.throws(() => buildLiveWorkbenchPythonBridge(() => {}), /WORKBENCH_CONTEXT_CHANGED/);
  } finally {
    if (previousWindow) globalThis.window = previousWindow;
    else Reflect.deleteProperty(globalThis, "window");
  }
});
