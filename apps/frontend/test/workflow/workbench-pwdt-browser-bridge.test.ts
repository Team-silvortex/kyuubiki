import test from "node:test";
import assert from "node:assert/strict";

import {
  CLOSED_LOOP_TRUSS_RECIPE_ID,
  createWorkbenchPwdtBrowserBridge,
  ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID,
  filterWorkbenchScriptRecipes,
  HEAT_TO_THERMO_QUAD_RECIPE_ID,
  HEAT_TO_THERMO_TRIANGLE_RECIPE_ID,
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
  assert.ok(filterWorkbenchScriptRecipes({ risk: "normal" }).some((recipe) => recipe.id === HEAT_TO_THERMO_TRIANGLE_RECIPE_ID));
  assert.ok(filterWorkbenchScriptRecipes({ risk: "normal" }).some((recipe) => recipe.id === ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID));
  assert.ok(filterWorkbenchScriptRecipes({ risk: "normal" }).some((recipe) => recipe.id === ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID));
});

test("Pwdt browser bridge rejects unknown actions and recipes without side effects", async () => {
  const calls: string[] = [];
  const bridge = makeBridge(calls);

  await assert.rejects(
    () => bridge.invoke("missing/action"),
    /Unknown Workbench frontend action/,
  );
  await assert.rejects(
    () => bridge.runRecipe("recipe/missing"),
    /Unknown Pwdt recipe/,
  );
  assert.deepEqual(calls, []);
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

test("Pwdt browser bridge exposes Store manifest commands without DOM clicks", async () => {
  const calls: string[] = [];
  const bridge = makeBridge(calls);

  await bridge.stageStoreEntry("operator", "solve.bar_1d");
  await bridge.exportStoreManifest();
  await bridge.removeStoreEntry("operator", "solve.bar_1d");

  assert.deepEqual(
    calls.map((entry) => entry.split(":")[1]),
    ["store/stageEntry", "store/exportManifest", "store/removeEntry"],
  );
});

test("Pwdt browser bridge resolves UI contract selectors for stable automation", () => {
  const previousDocument = (globalThis as any).document;
  (globalThis as any).document = {
    querySelector: (selector: string) => ({ selector }),
    querySelectorAll: (selector: string) => [{ selector }, { selector }],
  };

  try {
    const bridge = makeBridge();

    assert.equal(bridge.uiContract().contractVersion, 2);
    assert.equal(bridge.uiSelector("railButton", "library"), '[aria-label="workbench-rail:library"]');
    assert.equal(bridge.uiSelector("modelTab", "tree"), '[data-workbench-model-tab="tree"]');
    assert.equal(bridge.uiSelector("modelToolsPage", "study"), '[data-workbench-model-tools-page="study"]');
    assert.equal(bridge.uiSelector("modelStudyDomain", "thermal"), '[data-workbench-model-study-domain="thermal"]');
    assert.equal(bridge.uiSelector("modelStudyKind"), '[data-workbench-model-study-kind="select"]');
    assert.equal(bridge.uiSelector("runtimeTab", "control"), '[data-workbench-runtime-tab="control"]');
    assert.equal(bridge.uiSelector("libraryTab", "models"), '[data-workbench-library-tab="models"]');
    assert.equal(bridge.uiSelector("libraryModelPage", "versions"), '[data-workbench-library-model-page="versions"]');
    assert.equal(bridge.uiSelector("storeKind", "operator"), '[data-workbench-store-kind="operator"]');
    assert.equal(bridge.uiSelector("storeEntryAction", "stage"), '[data-workbench-store-entry-action="stage"]');
    assert.equal(bridge.uiSelector("storeManifestEntry", "solve.bar_1d"), '[data-workbench-store-manifest-entry-id="solve.bar_1d"]');
    assert.equal(bridge.uiSelector("storeManifestAction", "remove"), '[data-workbench-store-manifest-action="remove"]');
    assert.equal(bridge.uiSelector("systemSettingsPage", "overview"), '[data-workbench-system-settings-page="overview"]');
    assert.equal(bridge.uiSelector("libraryProjectsPanel"), '[data-workbench-library-projects="panel"]');
    assert.equal(bridge.uiSelector("libraryProjectAction", "create"), '[data-workbench-library-project-action="create"]');
    assert.equal(bridge.uiSelector("libraryProjectField", "name"), '[data-workbench-library-project-field="name"]');
    assert.equal(bridge.uiSelector("dataAction", "save-result"), '[data-workbench-data-action="save-result"]');
    assert.equal(bridge.uiSelector("dataField", "result-payload"), '[data-workbench-data-field="result-payload"]');
    assert.equal(bridge.uiSelector("dataRecord", "job-42"), '[data-workbench-data-record-id="job-42"]');
    assert.equal(bridge.uiSelector("dataRecordKind", "result"), '[data-workbench-data-record-kind="result"]');
    assert.equal(bridge.uiSelector("workflowCatalogEntry", "solve.bar_1d"), '[data-workflow-catalog-id="solve.bar_1d"]');
    assert.equal(bridge.uiSelector("workflowBuilderSecondaryTools"), '[data-workflow-builder-tools="secondary"]');
    assert.equal(bridge.uiSelector("workflowTopologyKind"), '[data-workflow-topology-kind="select"]');
    assert.equal(bridge.uiSelector("workflowTopologyAction", "add-edge"), '[data-workflow-topology-action="add-edge"]');
    assert.equal(bridge.uiSelector("workflowControlNode", "condition_2"), '[data-workflow-control-node-id="condition_2"]');
    assert.equal(bridge.uiSelector("workflowControlEmptyAction"), '[data-workflow-control-empty-action="insert"]');
    assert.equal(bridge.uiSelector("workflowInputArtifact", "load-input"), '[data-workflow-input-artifact="load-input"]');
    assert.equal(bridge.uiSelector("workflowRun", "job-42"), '[data-workflow-run-id="job-42"]');
    assert.equal(bridge.uiSelector("workflowRunStatus", "completed"), '[data-workflow-run-status="completed"]');
    assert.equal(bridge.uiSelector("workflowRunWorkflow", "solve.bar_1d"), '[data-workflow-run-workflow-id="solve.bar_1d"]');
    assert.equal(bridge.querySelector("workflowBuilder")?.getAttribute?.("missing") ?? null, null);
    assert.equal(
      (bridge.querySelector("workflowCatalogEntry", "solve.bar_1d") as any)?.selector,
      '[data-workflow-catalog-id="solve.bar_1d"]',
    );
    assert.equal(bridge.querySelectorAll("workflowBuilderAction", "validate").length, 2);
    assert.equal(bridge.selectorExists("workflowSurface"), true);
  } finally {
    (globalThis as any).document = previousDocument;
  }
});

test("Pwdt browser bridge rejects unknown selectors without DOM fallback", () => {
  const previousDocument = (globalThis as any).document;
  let queryCount = 0;
  (globalThis as any).document = {
    querySelector: () => {
      queryCount += 1;
      return null;
    },
    querySelectorAll: () => {
      queryCount += 1;
      return [];
    },
  };

  try {
    const bridge = makeBridge();
    assert.throws(
      () => bridge.querySelector("missingSelector"),
      /Unknown Workbench UI selector key/,
    );
    assert.equal(queryCount, 0);
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

test("Pwdt browser bridge runs a heat-to-thermo triangle composite recipe through action calls", async () => {
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
        snapshot.resultCount = snapshot.studyKind === "thermal_plane_triangle_2d" ? 2 : 1;
      }
      if (action === "state/projectHeatToThermo") {
        snapshot.studyKind = "thermal_plane_triangle_2d";
        snapshot.jobStatus = null;
        return { ok: true, action, studyKind: "thermal_plane_triangle_2d" };
      }
      if (action === "data/setFilters") snapshot.systemDataTab = payload?.activeTab;
      return { ok: true, action, payload };
    },
  });

  const result = await bridge.runRecipe(HEAT_TO_THERMO_TRIANGLE_RECIPE_ID, {
    activeMaterial: "210",
    heatModelName: "pwdt-test-heat-triangle",
    projectName: "Pwdt Test Project",
    thermoModelName: "pwdt-test-thermo-triangle",
    timeoutMs: 50,
  });

  assert.equal(result.ok, true);
  assert.equal(result.heatJobStatus, "completed");
  assert.equal(result.thermoJobStatus, "completed");
  assert.deepEqual(result.thermoProjection, {
    ok: true,
    action: "state/projectHeatToThermo",
    studyKind: "thermal_plane_triangle_2d",
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
  assert.equal(snapshot.studyKind, "thermal_plane_triangle_2d");
  assert.equal(snapshot.systemDataTab, "results");
});

test("Pwdt browser bridge runs an electrostatic-to-heat-to-thermo quad recipe through action calls", async () => {
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
        snapshot.resultCount =
          snapshot.studyKind === "thermal_plane_quad_2d" ? 3 : snapshot.studyKind === "heat_plane_quad_2d" ? 2 : 1;
      }
      if (action === "state/projectElectrostaticToHeat") {
        snapshot.studyKind = "heat_plane_quad_2d";
        snapshot.jobStatus = null;
        return { ok: true, action, studyKind: "heat_plane_quad_2d" };
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

  const result = await bridge.runRecipe(ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID, {
    activeMaterial: "210",
    electrostaticModelName: "pwdt-test-electrostatic",
    heatModelName: "pwdt-test-heat",
    projectName: "Pwdt Test Project",
    thermoModelName: "pwdt-test-thermo",
    timeoutMs: 50,
  });

  assert.equal(result.ok, true);
  assert.equal(result.projectId, "project-created");
  assert.equal(result.electrostaticJobStatus, "completed");
  assert.equal(result.heatJobStatus, "completed");
  assert.equal(result.thermoJobStatus, "completed");
  assert.deepEqual(result.heatProjection, {
    ok: true,
    action: "state/projectElectrostaticToHeat",
    studyKind: "heat_plane_quad_2d",
  });
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
      "state/projectElectrostaticToHeat",
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

test("Pwdt browser bridge runs an electrostatic-to-heat-to-thermo triangle recipe through action calls", async () => {
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
        snapshot.resultCount =
          snapshot.studyKind === "thermal_plane_triangle_2d" ? 3 : snapshot.studyKind === "heat_plane_triangle_2d" ? 2 : 1;
      }
      if (action === "state/projectElectrostaticToHeat") {
        snapshot.studyKind = "heat_plane_triangle_2d";
        snapshot.jobStatus = null;
        return { ok: true, action, studyKind: "heat_plane_triangle_2d" };
      }
      if (action === "state/projectHeatToThermo") {
        snapshot.studyKind = "thermal_plane_triangle_2d";
        snapshot.jobStatus = null;
        return { ok: true, action, studyKind: "thermal_plane_triangle_2d" };
      }
      if (action === "data/setFilters") snapshot.systemDataTab = payload?.activeTab;
      return { ok: true, action, payload };
    },
  });

  const result = await bridge.runRecipe(ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID, {
    activeMaterial: "210",
    electrostaticModelName: "pwdt-test-electrostatic-triangle",
    heatModelName: "pwdt-test-heat-triangle",
    projectName: "Pwdt Test Project",
    thermoModelName: "pwdt-test-thermo-triangle",
    timeoutMs: 50,
  });

  assert.equal(result.ok, true);
  assert.equal(result.electrostaticJobStatus, "completed");
  assert.equal(result.heatJobStatus, "completed");
  assert.equal(result.thermoJobStatus, "completed");
  assert.deepEqual(result.heatProjection, {
    ok: true,
    action: "state/projectElectrostaticToHeat",
    studyKind: "heat_plane_triangle_2d",
  });
  assert.deepEqual(result.thermoProjection, {
    ok: true,
    action: "state/projectHeatToThermo",
    studyKind: "thermal_plane_triangle_2d",
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
      "state/projectElectrostaticToHeat",
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
  assert.equal(snapshot.studyKind, "thermal_plane_triangle_2d");
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

test("Pwdt browser bridge wait helpers reject bounded timeouts", async () => {
  const bridge = createWorkbenchPwdtBrowserBridge({
    getSnapshot: () => ({ jobStatus: "running", message: "dispatching" }),
    invokeAction: async () => ({ ok: true }),
  });

  await assert.rejects(
    () => bridge.waitForState({ jobStatus: "completed" }, { timeoutMs: 5, intervalMs: 1 }),
    /Pwdt wait timed out after 5ms/,
  );
});
