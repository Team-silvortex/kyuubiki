import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  buildWorkbenchPythonPrelude, createWorkbenchPwdtBrowserBridge,
  WORKBENCH_SCRIPT_ACTIONS, WORKBENCH_SCRIPT_RECIPES,
} from "../../src/lib/scripting/workbench-script-runtime.ts";

function runPython(recipe?: string, stopAt?: number, options: { stopRunAt?: number; jobStatus?: string } = {}) {
  const process = spawnSync(globalThis.process.platform === "win32" ? "python" : "python3", [
    "-I", "-B", fileURLToPath(new URL("../support/pwdt-python-harness.py", import.meta.url)),
  ], {
    input: JSON.stringify({ recipe, stopAt, ...options, prelude: buildWorkbenchPythonPrelude(),
      actions: WORKBENCH_SCRIPT_ACTIONS, recipes: WORKBENCH_SCRIPT_RECIPES }),
    encoding: "utf8", timeout: 15_000,
  });
  assert.ifError(process.error);
  assert.equal(process.status, 0, process.stderr);
  return JSON.parse(process.stdout);
}

test("PWDT shipped Python prelude initializes and calls its JavaScript bridge", () => {
  const outcome = runPython();
  assert.equal(outcome.error, undefined);
  assert.equal(outcome.result.initialized, true);
  assert.equal(outcome.result.actions, WORKBENCH_SCRIPT_ACTIONS.length);
  assert.deepEqual(outcome.logs, ["bridge ready"]);
});

for (const recipe of WORKBENCH_SCRIPT_RECIPES) {
  const stages = recipe.id.includes("electrostatic") ? 3 : recipe.id.includes("heat-thermo") ? 2 : 1;
  test(`PWDT Python recipe ${recipe.id} completes all stages without a workspace switch`, () => {
    const outcome = runPython(recipe.id);
    assert.equal(outcome.error, undefined);
    assert.equal(outcome.result.ok, true);
    assert.equal(outcome.calls.filter((action: string) => action === "job/run").length, stages);
  });
  for (let stopAt = 0; stopAt <= stages; stopAt += 1) {
    for (const surface of ["browser", "python"]) {
      test(`PWDT ${surface} recipe ${recipe.id} stops after changed context at stage ${stopAt}`, async () => {
        if (surface === "python") {
          const outcome = runPython(recipe.id, stopAt);
          assert.match(outcome.error ?? "", /WORKBENCH_CONTEXT_CHANGED/u);
          assert.equal(outcome.calls.at(-1), stopAt === 0 ? "project/create" : "model/saveAs");
          assert.equal(outcome.calls.filter((action: string) => action === "job/run").length, Math.max(0, stopAt - 1));
          return;
        }
        const calls: string[] = [];
        let saves = 0;
        const snapshot: Record<string, unknown> = { selectedProjectId: null, jobStatus: "completed" };
        const bridge = createWorkbenchPwdtBrowserBridge({
          getSnapshot: () => snapshot,
          invokeAction: async (action, payload) => {
            calls.push(action);
            if (action === "nav/setStudyKind") snapshot.studyKind = payload?.studyKind;
            if (action === "project/create") {
              snapshot.selectedProjectId = stopAt === 0 ? "other-project" : "created-project";
              return { ok: true, projectId: "created-project", contextChanged: stopAt === 0 };
            }
            if (action === "model/saveAs") {
              saves += 1;
              return { ok: true, modelId: `saved-${saves}`, contextChanged: saves === stopAt };
            }
            return { ok: true, action };
          },
        });
        await assert.rejects(bridge.runRecipe(recipe.id, { timeoutMs: 50 }), /WORKBENCH_CONTEXT_CHANGED/u);
        assert.equal(calls.at(-1), stopAt === 0 ? "project/create" : "model/saveAs");
        assert.equal(calls.filter((action) => action === "job/run").length, Math.max(0, stopAt - 1));
      });
    }
  }
  for (let stopRunAt = 1; stopRunAt <= stages; stopRunAt += 1) {
    for (const jobStatus of ["failed", "cancelled"]) {
      for (const surface of ["browser", "python"]) {
        test(`PWDT ${surface} recipe ${recipe.id} stops on ${jobStatus} computation at stage ${stopRunAt}`, async () => {
          const expectedError = new RegExp(`WORKBENCH_JOB_${jobStatus.toUpperCase()}`, "u");
          let calls: string[];
          if (surface === "python") {
            const outcome = runPython(recipe.id, undefined, { stopRunAt, jobStatus });
            assert.match(outcome.error ?? "", expectedError);
            calls = outcome.calls;
          } else {
            calls = [];
            let runs = 0;
            const snapshot: Record<string, unknown> = { selectedProjectId: "existing-project", jobStatus: null };
            const bridge = createWorkbenchPwdtBrowserBridge({
              getSnapshot: () => snapshot,
              invokeAction: async (action, payload) => {
                calls.push(action);
                if (action === "nav/setStudyKind") snapshot.studyKind = payload?.studyKind;
                if (action === "job/run") snapshot.jobStatus = ++runs === stopRunAt ? jobStatus : "completed";
                return { ok: true, action };
              },
            });
            await assert.rejects(bridge.runRecipe(recipe.id, { timeoutMs: 50 }), expectedError);
            assert.equal((await bridge.waitForJobDone()).jobStatus, jobStatus, "terminal inspection remains available");
          }
          assert.equal(calls.at(-1), "job/run", "failed computation must not project, save, or open results");
          assert.equal(calls.filter((action) => action === "job/run").length, stopRunAt);
          assert.equal(calls.filter((action) => action === "model/saveAs").length, stopRunAt);
        });
      }
    }
  }
}
