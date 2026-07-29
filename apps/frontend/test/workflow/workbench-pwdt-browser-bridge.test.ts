import test from "node:test";
import assert from "node:assert/strict";

import {
  CLOSED_LOOP_TRUSS_RECIPE_ID,
  createWorkbenchPwdtBrowserBridge,
  filterWorkbenchScriptRecipes,
  HEAT_TO_THERMO_QUAD_RECIPE_ID,
  installWorkbenchPwdtBrowserBridge,
  WORKBENCH_SCRIPT_RECIPES,
} from "../../src/lib/scripting/workbench-script-runtime.ts";

function makeBridge(calls: string[] = []) {
  return createWorkbenchPwdtBrowserBridge({
    getSnapshot: () => ({
      sidebarSection: "study",
      studyKind: "truss_2d",
    }),
    invokeAction: async (action, payload, source, note) => {
      calls.push(`${source}:${action}:${note ?? ""}`);
      return { ok: true, action, payload };
    },
  });
}

test("Pwdt browser bridge invokes registered Workbench actions without DOM clicks", async () => {
  const calls: string[] = [];
  const bridge = makeBridge(calls);

  const result = await bridge.invoke("nav/setSidebarSection", { section: "system" });

  assert.deepEqual(result, {
    ok: true,
    action: "nav/setSidebarSection",
    payload: { section: "system" },
  });
  assert.deepEqual(calls, ["script:nav/setSidebarSection:Pwdt browser bridge"]);
  assert.equal(bridge.hasAction("job/run"), true);
  assert.equal(bridge.hasAction("missing/action"), false);
  assert.equal(bridge.hasRecipe(CLOSED_LOOP_TRUSS_RECIPE_ID), true);
  assert.equal(bridge.recipes().length, WORKBENCH_SCRIPT_RECIPES.length);
  assert.deepEqual(bridge.recipesMatching({ category: "study" }).map((recipe) => recipe.id), [
    CLOSED_LOOP_TRUSS_RECIPE_ID,
  ]);
  assert.ok(filterWorkbenchScriptRecipes({ risk: "normal" }).some((recipe) => recipe.id === CLOSED_LOOP_TRUSS_RECIPE_ID));
  assert.ok(filterWorkbenchScriptRecipes({ risk: "normal" }).some((recipe) => recipe.id === HEAT_TO_THERMO_QUAD_RECIPE_ID));
});

test("Pwdt browser bridge runs macros and action steps through the same action bridge", async () => {
  const calls: string[] = [];
  const bridge = makeBridge(calls);

  await bridge.runMacro("macro/openDataResults", { projectId: "project-a" });
  const stepResults = await bridge.runSteps([
    { action: "nav/setSidebarSection", payload: { section: "model" } },
    { action: "model/generateTruss" },
  ]);

  assert.equal(bridge.hasMacro("macro/openDataResults"), true);
  assert.deepEqual(
    calls.map((entry) => entry.split(":")[1]),
    ["macro/run", "nav/setSidebarSection", "model/generateTruss"],
  );
  assert.equal(stepResults.length, 2);
});

test("Pwdt browser bridge resolves UI contract selectors for stable automation", () => {
  const previousDocument = (globalThis as any).document;
  (globalThis as any).document = {
    querySelector: (selector: string) => ({ selector }),
    querySelectorAll: (selector: string) => [{ selector }, { selector }],
  };

  try {
    const bridge = makeBridge();

    assert.equal(bridge.uiSelector("runtimeTab", "control"), '[data-workbench-runtime-tab="control"]');
    assert.equal(bridge.querySelector("workflowBuilder")?.getAttribute?.("missing") ?? null, null);
    assert.equal(bridge.querySelectorAll("workflowBuilderAction", "validate").length, 2);
    assert.equal(bridge.selectorExists("workflowSurface"), true);
  } finally {
    (globalThis as any).document = previousDocument;
  }
});

test("Pwdt browser bridge installs a window-level control surface and Pyodide bridge", async () => {
  const previousWindow = (globalThis as any).window;
  const fakeWindow = { setTimeout };
  (globalThis as any).window = fakeWindow;
  const calls: string[] = [];

  const cleanup = installWorkbenchPwdtBrowserBridge({
    getSnapshot: () => ({ sidebarSection: "system", studyKind: "truss_2d" }),
    invokeAction: async (action, payload) => {
      calls.push(action);
      return { ok: true, action, payload };
    },
  });

  try {
    assert.equal((fakeWindow as any).__kyuubikiPwdt.version, "kyuubiki.pwdt.browser-bridge/v1");
    assert.equal((fakeWindow as any).__kyuubikiPwdt.parityReport().current_sidebar, "system");
    assert.equal((fakeWindow as any).__kyuubikiPwdt.parityReport().recipe_count, WORKBENCH_SCRIPT_RECIPES.length);
    assert.equal(
      (fakeWindow as any).__kyuubikiPwdt.parityReport().normal_recipe_count,
      filterWorkbenchScriptRecipes({ risk: "normal" }).length,
    );
    assert.equal(
      JSON.parse((fakeWindow as any).__kyuubikiBridge.recipes_json())[0].id,
      CLOSED_LOOP_TRUSS_RECIPE_ID,
    );

    const resultJson = await (fakeWindow as any).__kyuubikiBridge.invoke(
      "nav/setSidebarSection",
      JSON.stringify({ section: "library" }),
    );

    assert.deepEqual(JSON.parse(resultJson), {
      ok: true,
      action: "nav/setSidebarSection",
      payload: { section: "library" },
    });
    assert.deepEqual(calls, ["nav/setSidebarSection"]);
  } finally {
    cleanup();
    assert.equal((fakeWindow as any).__kyuubikiPwdt, undefined);
    assert.equal((fakeWindow as any).__kyuubikiBridge, undefined);
    (globalThis as any).window = previousWindow;
  }
});

test("Pwdt browser bridge runs a closed-loop truss study recipe through action calls", async () => {
  const calls: string[] = [];
  const snapshot: Record<string, unknown> = {
    jobStatus: null,
    resultCount: 0,
    selectedProjectId: null,
    sidebarSection: "study",
    studyKind: "axial_bar_1d",
    systemDataTab: "jobs",
  };
  const bridge = createWorkbenchPwdtBrowserBridge({
    getSnapshot: () => snapshot,
    invokeAction: async (action, payload) => {
      calls.push(action);
      if (action === "project/create") {
        snapshot.selectedProjectId = "project-created";
        return { ok: true, action, projectId: "project-created" };
      }
      if (action === "nav/setSidebarSection") snapshot.sidebarSection = payload?.section;
      if (action === "nav/setStudyKind") snapshot.studyKind = payload?.studyKind;
      if (action === "job/run") {
        snapshot.jobStatus = "completed";
        snapshot.resultCount = 1;
      }
      if (action === "data/setFilters") snapshot.systemDataTab = payload?.activeTab;
      return { ok: true, action, payload };
    },
  });

  const result = await bridge.runRecipe(CLOSED_LOOP_TRUSS_RECIPE_ID, {
    activeMaterial: "210",
    bays: 4,
    height: 2.5,
    loadY: -900,
    modelName: "pwdt-test-truss",
    projectName: "Pwdt Test Project",
    span: 12,
    timeoutMs: 50,
  });

  assert.equal(result.ok, true);
  assert.equal(result.projectId, "project-created");
  assert.deepEqual(
    calls,
    [
      "project/create",
      "nav/setStudyKind",
      "nav/setSidebarSection",
      "nav/setTabs",
      "model/setWorkspaceMeta",
      "state/setParametric",
      "model/generateTruss",
      "model/setWorkspaceMeta",
      "model/saveAs",
      "job/run",
      "data/setFilters",
    ],
  );
  assert.equal((await bridge.waitForState({ systemDataTab: "results" }, { timeoutMs: 50 })).systemDataTab, "results");
  await assert.rejects(() => bridge.runRecipe("recipe/missing"), /Unknown Pwdt recipe/);
});

test("Pwdt browser bridge runs a heat-to-thermo quad composite recipe through action calls", async () => {
  const calls: string[] = [];
  const snapshot: Record<string, unknown> = {
    jobStatus: null,
    resultCount: 0,
    selectedProjectId: null,
    sidebarSection: "study",
    studyKind: "axial_bar_1d",
    systemDataTab: "jobs",
  };
  const bridge = createWorkbenchPwdtBrowserBridge({
    getSnapshot: () => snapshot,
    invokeAction: async (action, payload) => {
      calls.push(action);
      if (action === "project/create") {
        snapshot.selectedProjectId = "project-created";
        return { ok: true, action, projectId: "project-created" };
      }
      if (action === "nav/setSidebarSection") snapshot.sidebarSection = payload?.section;
      if (action === "nav/setStudyKind") snapshot.studyKind = payload?.studyKind;
      if (action === "job/run") {
        snapshot.jobStatus = "completed";
        snapshot.resultCount = snapshot.studyKind === "thermal_plane_quad_2d" ? 2 : 1;
      }
      if (action === "state/projectHeatToThermo") {
        snapshot.studyKind = "thermal_plane_quad_2d";
        snapshot.jobStatus = null;
        return { ok: true, action, studyKind: "thermal_plane_quad_2d" };
      }
      if (action === "data/setFilters") snapshot.systemDataTab = payload?.activeTab;
      return { ok: true, action, payload };
    },
  });

  const result = await bridge.runRecipe(HEAT_TO_THERMO_QUAD_RECIPE_ID, {
    activeMaterial: "210",
    heatModelName: "pwdt-test-heat",
    projectName: "Pwdt Test Project",
    thermoModelName: "pwdt-test-thermo",
    timeoutMs: 50,
  });

  assert.equal(result.ok, true);
  assert.equal(result.projectId, "project-created");
  assert.equal(result.heatJobStatus, "completed");
  assert.equal(result.thermoJobStatus, "completed");
  assert.deepEqual(result.thermoProjection, {
    ok: true,
    action: "state/projectHeatToThermo",
    studyKind: "thermal_plane_quad_2d",
  });
  assert.deepEqual(
    calls,
    [
      "project/create",
      "nav/setStudyKind",
      "nav/setSidebarSection",
      "nav/setTabs",
      "model/setWorkspaceMeta",
      "model/setWorkspaceMeta",
      "model/saveAs",
      "job/run",
      "state/projectHeatToThermo",
      "model/setWorkspaceMeta",
      "model/saveAs",
      "job/run",
      "data/setFilters",
    ],
  );
  assert.equal(snapshot.studyKind, "thermal_plane_quad_2d");
  assert.equal(snapshot.systemDataTab, "results");
});

test("Pwdt browser bridge wait helpers observe state and messages", async () => {
  const snapshot: Record<string, unknown> = {
    jobStatus: "running",
    message: "dispatching",
    sidebarSection: "system",
    studyKind: "truss_2d",
  };
  const bridge = createWorkbenchPwdtBrowserBridge({
    getSnapshot: () => snapshot,
    invokeAction: async () => ({ ok: true }),
  });

  void globalThis.setTimeout(() => {
    snapshot.message = "solve completed";
    snapshot.jobStatus = "completed";
  }, 5);

  assert.equal((await bridge.waitForMessage("completed", { timeoutMs: 100, intervalMs: 5 })).message, "solve completed");
  assert.equal((await bridge.waitForJobDone({ timeoutMs: 100, intervalMs: 5 })).jobStatus, "completed");
});
