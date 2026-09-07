import assert from "node:assert/strict";
import { test } from "node:test";
import {
  PROJECT_ID, runtime, initialProject, usingWorkbench, invoke, openWorkbench,
  exportProject, importProject, holdRequest, openSavedModels, openProjectManager,
  waitForGuiTransition, installProjectWorkbenchTestHooks,
} from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

const mutationCases = [
  { action: "project/create", method: "POST", path: () => "/api/v1/projects", gui: "create" },
  { action: "project/updateSelected", method: "PATCH", path: () => `/api/v1/projects/${PROJECT_ID}`, gui: "update" },
  { action: "project/deleteSelected", method: "DELETE", path: () => `/api/v1/projects/${PROJECT_ID}`, gui: "delete" },
  { action: "model/deleteSelected", method: "DELETE", path: (library) => `/api/v1/models/${library.models[0].model_id}`, gui: "Delete model" },
  { action: "model/renameSelectedVersion", method: "PATCH", path: (library) => `/api/v1/model-versions/${library.versions[0].version_id}`, gui: "Rename version" },
  { action: "model/deleteSelectedVersion", method: "DELETE", path: (library) => `/api/v1/model-versions/${library.versions[0].version_id}`, gui: "Delete version" },
];

for (const surface of ["pwdt", "gui"]) {
for (const removed of ["project", "model", "version"]) {
test(`Workbench ${surface} pending ${removed} deletion reconciles only bindings to removed resources`, { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    page.on("dialog", (dialog) => dialog.accept());
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Earlier version" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Latest version" }));
    const modelId = library.models[0].model_id;
    const earlierId = library.versions[0].version_id;
    const latestId = library.versions[1].version_id;
    const action = removed === "project" ? "project/deleteSelected"
      : removed === "model" ? "model/deleteSelected" : "model/deleteSelectedVersion";
    const pathname = removed === "project" ? `/api/v1/projects/${PROJECT_ID}`
      : removed === "model" ? `/api/v1/models/${modelId}` : `/api/v1/model-versions/${latestId}`;
    if (surface === "gui") {
      if (removed === "project") await openProjectManager(page);
      else {
        await openSavedModels(page);
        if (removed === "version") await page.locator('[data-workbench-library-model-page="versions"]').click();
      }
    }
    const pending = await holdRequest(page, pathname, "DELETE");
    let operation;
    try {
      if (surface === "pwdt") operation = invoke(page, action).catch((error) => ({ error: error.message }));
      else if (removed === "project") await page.locator('[data-workbench-library-project-action="delete"]').click();
      else await page.getByRole("button", { name: removed === "model" ? "Delete model" : "Delete version", exact: true }).click();
      await pending.received;
      if (surface === "gui") await waitForGuiTransition(page, true);
      await openSavedModels(page);
      await page.locator('[data-workbench-library-model-page="versions"]').click();
      await page.locator("button.history-item").filter({ hasText: "Earlier version" }).click();
      await page.evaluate((versionId) => window.__kyuubikiPwdt.waitForState({ selectedVersionId: versionId }), earlierId);
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const finished = page.waitForResponse((response) => new URL(response.url()).pathname === pathname && response.request().method() === "DELETE");
      pending.release();
      await finished;
      const outcome = await operation;
      if (surface === "gui") await waitForGuiTransition(page);
      await page.evaluate((expected) => window.__kyuubikiPwdt.waitForState(expected), {
        selectedProjectId: removed === "project" ? null : PROJECT_ID,
        selectedModelId: removed === "version" ? modelId : null,
        selectedVersionId: removed === "version" ? earlierId : null,
      });
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(after.selectedProjectId, removed === "project" ? null : PROJECT_ID);
      assert.equal(after.selectedModelId, removed === "version" ? modelId : null);
      assert.equal(after.selectedVersionId, removed === "version" ? earlierId : null);
      assert.equal(after.loadedModelName, "Earlier version", "deleting a saved record must retain the working model");
      assert.equal(after.message, before.message, "an older operation must not replace newer feedback");
      if (surface === "pwdt") { assert.equal(outcome.ok, true); assert.equal(outcome.contextChanged, true); }
      await openSavedModels(page);
      await page.locator('[data-workbench-library-model-page="versions"]').click();
      assert.equal(await page.locator("button.history-item").filter({ hasText: "Latest version" }).count(), 0);
      if (removed === "version") assert.equal(library.versions.length, 1);
    } finally { pending.release(); await operation; }
  });
});
}
}

for (const surface of ["pwdt", "gui"]) {
test(`Workbench ${surface} deleting a saved model leaves its working copy detached from remaining models`, { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    page.on("dialog", (dialog) => dialog.accept());
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Keep this model" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Delete this model", saveAs: true }));
    const kept = structuredClone(library.models[0]);
    if (surface === "pwdt") await invoke(page, "model/deleteSelected");
    else {
      await openSavedModels(page);
      await page.getByRole("button", { name: "Delete model", exact: true }).click();
    }
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedModelId: null, selectedVersionId: null }));
    const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(snapshot.selectedProjectId, PROJECT_ID);
    assert.equal(snapshot.loadedModelName, "Delete this model");
    assert.deepEqual(library.models, [kept]);
    await page.evaluate(() => window.__kyuubikiPwdt.refreshAll());
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), null,
      "refreshing the catalog must not attach the detached working copy to a different saved model");
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Recovered working copy" }));
    assert.equal(library.models.length, 2);
    assert.deepEqual(library.models[0], kept, "saving the detached copy cannot overwrite the remaining model");
    assert.equal(library.models[1].name, "Recovered working copy");
  });
});
}

for (const surface of ["pwdt", "gui"]) {
for (const scenario of mutationCases) {
for (const unavailable of [false, true]) {
test(`Workbench ${surface} pending ${scenario.action} ${unavailable ? "failure" : "success"} preserves a newer saved workspace`, { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    page.on("dialog", (dialog) => dialog.accept());
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Original research" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    const originalModel = structuredClone(library.models[0]);
    const originalVersion = structuredClone(library.versions[0]);
    const pathname = scenario.path(library);
    if (scenario.action === "model/renameSelectedVersion") {
      await invoke(page, "model/setWorkspaceMeta", { loadedModelName: "Pending edit" });
    }
    if (surface === "gui") {
      if (scenario.action.startsWith("project/")) {
        await openProjectManager(page);
        if (scenario.method !== "DELETE") await page.locator('[data-workbench-library-project-field="name"]').fill("Pending edit");
      } else {
        await openSavedModels(page);
        if (scenario.action.includes("Version")) await page.locator('[data-workbench-library-model-page="versions"]').click();
      }
    }
    const pending = await holdRequest(page, pathname, scenario.method, (route) => unavailable
      ? route.fulfill({ status: 503, json: { error: "previous workspace mutation unavailable" } }) : route.fallback());
    let operation;
    try {
      if (surface === "pwdt") {
        operation = invoke(page, scenario.action, { name: "Pending edit" }).catch((error) => ({ error: error.message }));
      } else if (scenario.action.startsWith("project/")) {
        await page.locator(`[data-workbench-library-project-action="${scenario.gui}"]`).click();
      } else await page.getByRole("button", { name: scenario.gui, exact: true }).click();
      await pending.received;
      if (surface === "gui") await waitForGuiTransition(page, true);
      await invoke(page, "project/select", { projectId: "second-project" });
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
      const currentModel = structuredClone(library.models[1]);
      const currentVersion = structuredClone(library.versions[1]);
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const finished = page.waitForResponse((response) => new URL(response.url()).pathname === pathname && response.request().method() === scenario.method);
      pending.release();
      await finished;
      const outcome = await operation;
      if (surface === "gui") await waitForGuiTransition(page);
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "message"]) {
        assert.equal(after[key], before[key], `${scenario.action} must preserve the newer workspace's ${key}`);
      }
      assert.deepEqual(library.models.find((entry) => entry.model_id === currentModel.model_id), currentModel);
      assert.deepEqual(library.versions.find((entry) => entry.version_id === currentVersion.version_id), currentVersion);
      if (surface === "pwdt") {
        if (unavailable) assert.match(outcome.error, /unavailable|503/u);
        else { assert.equal(outcome.ok, true); assert.equal(outcome.contextChanged, true); }
      }
      if (unavailable) {
        assert.deepEqual(library.models.find((entry) => entry.model_id === originalModel.model_id), originalModel);
        assert.deepEqual(library.versions.find((entry) => entry.version_id === originalVersion.version_id), originalVersion);
        assert.equal(runtime.state.projectMutations.length, 0);
      } else if (scenario.action === "project/create") {
        assert.equal(runtime.state.projects.at(-1).name, "Pending edit");
        assert.equal(runtime.state.projects.length, 3);
      } else if (scenario.action === "project/updateSelected") {
        assert.equal(runtime.state.projects.find((entry) => entry.project_id === PROJECT_ID).name, "Pending edit");
      } else if (scenario.action === "project/deleteSelected") {
        assert.equal(runtime.state.projects.some((entry) => entry.project_id === PROJECT_ID), false);
      } else if (scenario.action === "model/deleteSelected") {
        assert.equal(library.models.some((entry) => entry.model_id === originalModel.model_id), false);
      } else if (scenario.action === "model/renameSelectedVersion") {
        assert.equal(library.versions.find((entry) => entry.version_id === originalVersion.version_id).name, "Pending edit");
      } else assert.equal(library.versions.some((entry) => entry.version_id === originalVersion.version_id), false);
      await openProjectManager(page);
      assert.equal(await page.locator('[data-workbench-library-project-field="name"]').inputValue(), "Second project");
    } finally { pending.release(); await operation; }
  });
});
}
}
}

for (const surface of ["pwdt", "gui"]) {
test(`Workbench ${surface} slow save stays in its original project after a workspace switch`, async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Original research" }));
    const pending = await holdRequest(page, `/api/v1/projects/${PROJECT_ID}/models`, "POST");
    let saving;
    try {
      if (surface === "pwdt") {
        saving = page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true })).catch((error) => ({ error: error.message }));
      } else {
        await openSavedModels(page);
        await page.getByRole("button", { name: "Save As", exact: true }).click();
      }
      await pending.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "New research" }));
      const refreshed = page.waitForResponse((response) => new URL(response.url()).pathname === "/api/v1/projects");
      pending.release();
      if (saving) assert.equal((await saving).ok, true, "the original write should still succeed");
      await refreshed;
      if (surface === "gui") await waitForGuiTransition(page);
      const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(snapshot.selectedProjectId, "second-project", "a completed save must not pull the user back");
      assert.equal(snapshot.selectedModelId, null, "the saved model belongs to the previous project");
      assert.equal(snapshot.selectedVersionId, null);
      assert.equal(snapshot.loadedModelName, "New research");
      assert.equal(library.models.length, 1);
      assert.equal(library.models[0].project_id, PROJECT_ID);
      assert.equal(library.models[0].name, "Original research");
    } finally { pending.release(); await saving; }
  });
}, { timeout: 90_000 });
}

test("Workbench an older model load cannot overwrite a more recently opened model", async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Earlier model" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Later model" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await openSavedModels(page);
    const pending = await holdRequest(page, `/api/v1/models/${library.models[0].model_id}`);
    try {
      await page.locator("button.history-item").filter({ hasText: "Earlier model" }).click();
      await pending.received;
      const laterLoaded = page.waitForRequest((request) => new URL(request.url()).pathname === `/api/v1/models/${library.models[1].model_id}/versions`);
      await page.locator("button.history-item").filter({ hasText: "Later model" }).click();
      await laterLoaded;
      const finished = page.waitForResponse((response) => new URL(response.url()).pathname === `/api/v1/models/${library.models[0].model_id}`);
      pending.release();
      await finished;
      await page.waitForLoadState("networkidle");
      const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(snapshot.selectedModelId, library.models[1].model_id);
      assert.equal(snapshot.loadedModelName, "Later model");
      assert.deepEqual((await exportProject(page)).workspace_snapshot.nodes, library.models[1].payload.nodes);
    } finally { pending.release(); }
  });
}, { timeout: 90_000 });

for (const source of ["model", "version", "failed-model"]) {
test(`Workbench pending ${source} load cannot replace a switched project or its message`, async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Saved research" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await openSavedModels(page);
    const pathname = source === "version"
      ? `/api/v1/model-versions/${library.versions[0].version_id}`
      : `/api/v1/models/${library.models[0].model_id}`;
    const pending = await holdRequest(page, pathname, "GET", (route) => source === "failed-model"
      ? route.fulfill({ status: 503, json: { error: "obsolete model load failed" } }) : route.fallback());
    try {
      if (source === "version") await page.locator('[data-workbench-library-model-page="versions"]').click();
      await page.locator("button.history-item").filter({ hasText: "Saved research" }).click();
      await pending.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const finished = page.waitForResponse((response) => new URL(response.url()).pathname === pathname);
      pending.release();
      await finished;
      await page.waitForLoadState("networkidle");
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(after.selectedProjectId, "second-project");
      assert.equal(after.selectedModelId, null);
      assert.equal(after.selectedVersionId, null);
      assert.equal(after.loadedModelName, "Current research");
      assert.equal(after.message, before.message, "stale success and failure messages must stay out of the current workspace");
    } finally { pending.release(); }
  });
}, { timeout: 90_000 });
}

test("Workbench a pending version save cannot reattach a model after navigating away and back", async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    const pending = await holdRequest(page, `/api/v1/models/${library.models[0].model_id}/versions`, "POST");
    const saving = page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Saved before navigation" })).catch((error) => ({ error: error.message }));
    try {
      await pending.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      await invoke(page, "project/select", { projectId: PROJECT_ID });
      pending.release();
      const outcome = await saving;
      assert.equal(outcome.ok, true);
      assert.equal(outcome.contextChanged, true);
      const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(snapshot.selectedProjectId, PROJECT_ID);
      assert.equal(snapshot.selectedModelId, null);
      assert.equal(snapshot.selectedVersionId, null);
      assert.equal(library.versions.length, 2, "the original save must remain persisted");
    } finally { pending.release(); await saving; }
  });
}, { timeout: 90_000 });

test("Workbench a delayed catalog refresh preserves newer selection and a failed refresh preserves the catalog", async () => {
  await usingWorkbench(async (page) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    const pending = await holdRequest(page, "/api/v1/projects");
    const refreshing = invoke(page, "runtime/refreshAll").catch((error) => ({ error: error.message }));
    try {
      await Promise.race([pending.received, refreshing.then((outcome) => { throw new Error(`Refresh returned before requesting projects: ${JSON.stringify(outcome)}`); })]);
      await invoke(page, "project/select", { projectId: "second-project" });
      pending.release();
      assert.equal((await refreshing).ok, true);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), "second-project");
      await page.route("**/api/v1/projects", (route) => route.fulfill({ status: 503, json: { error: "catalog temporarily unavailable" } }));
      await invoke(page, "runtime/refreshAll");
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().projectCount), 2);
      await invoke(page, "project/select", { projectId: PROJECT_ID });
    } finally { pending.release(); await refreshing; }
  });
}, { timeout: 90_000 });

test("Workbench a delayed version list cannot leak into another project's model library", async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Previous project version" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await openSavedModels(page);
    const pathname = `/api/v1/models/${library.models[0].model_id}/versions`;
    const pending = await holdRequest(page, pathname);
    try {
      await page.locator("button.history-item").filter({ hasText: "Previous project version" }).click();
      await pending.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      const finished = page.waitForResponse((response) => new URL(response.url()).pathname === pathname);
      pending.release();
      await finished;
      await page.waitForLoadState("networkidle");
      await page.locator('[data-workbench-library-model-page="versions"]').click();
      assert.equal(await page.locator("button.history-item").count(), 0);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), "second-project");
    } finally { pending.release(); }
  });
}, { timeout: 90_000 });

test("Workbench a delayed bundle import persists its new project without replacing a newer workspace", async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Archived research" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    const bundle = await exportProject(page);
    const pending = await holdRequest(page, "/api/v1/projects", "POST");
    const importing = importProject(page, bundle).catch((error) => ({ error: error.message }));
    try {
      await pending.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Newer research" }));
      const refreshed = page.waitForResponse((response) => new URL(response.url()).pathname === "/api/v1/projects" && response.request().method() === "GET");
      pending.release();
      await refreshed;
      await page.waitForLoadState("networkidle");
      const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(snapshot.selectedProjectId, "second-project");
      assert.equal(snapshot.selectedModelId, null);
      assert.equal(snapshot.loadedModelName, "Newer research");
      assert.equal(runtime.state.projects.length, 3);
      assert.equal(library.models.length, 2);
      assert.equal(library.models[1].project_id, "qualification-created-project-1");
    } finally { pending.release(); await importing; }
  });
}, { timeout: 90_000 });

test("Workbench Pwdt switching project cannot save over the previous project's model", async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "First model" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    const original = structuredClone(library.models[0]);
    await invoke(page, "project/select", { projectId: "second-project" });
    const selection = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(selection.selectedProjectId, "second-project");
    assert.equal(selection.selectedModelId, null, "project switching must clear the old model association");
    assert.equal(selection.selectedVersionId, null);
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Second model" }));
    assert.equal(library.models.length, 2);
    assert.deepEqual(library.models[0], original);
    assert.equal(library.models[1].project_id, "second-project");
  });
}, { timeout: 90_000 });

test("Workbench Pwdt rejects an unknown project without changing the active context", async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await assert.rejects(invoke(page, "project/select", { projectId: "missing-project" }));
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), PROJECT_ID);
    assert.equal(runtime.state.projectMutations.length, 0);
  });
}, { timeout: 60_000 });

test("Workbench project deletion preserves context on cancellation and service failure", async () => {
  await usingWorkbench(async (page) => {
    let unavailable = true;
    let deleteRequests = 0;
    await page.route(`**/api/v1/projects/${PROJECT_ID}`, async (route) => {
      if (route.request().method() !== "DELETE") return route.fallback();
      deleteRequests += 1;
      if (unavailable) await route.fulfill({ status: 503, json: { error: "qualification delete unavailable" } });
      else await route.fallback();
    });
    await openWorkbench(page);
    page.once("dialog", (dialog) => dialog.dismiss());
    await assert.rejects(invoke(page, "project/deleteSelected"), /cancelled/iu);
    assert.equal(deleteRequests, 0);
    page.once("dialog", (dialog) => dialog.accept());
    await assert.rejects(invoke(page, "project/deleteSelected"), /unavailable|503/u);
    assert.equal(runtime.state.projects.length, 1);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), PROJECT_ID);
    unavailable = false;
    page.once("dialog", (dialog) => dialog.accept());
    await invoke(page, "project/deleteSelected");
    assert.equal(runtime.state.projects.length, 0);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), null);
    assert.equal(runtime.state.projectMutations.length, 1);
  });
}, { timeout: 75_000 });

for (const withSnapshot of [true, false]) {
test("Workbench project import preserves model, active version, and workspace name before saving" + (withSnapshot ? "" : " without a workspace snapshot"), async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Roundtrip workspace" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Roundtrip workspace" }));
    const bundle = await exportProject(page);
    const originalSnapshot = bundle.workspace_snapshot;
    bundle.active_version_id = library.versions[0].version_id;
    if (!withSnapshot) delete bundle.workspace_snapshot;
    await importProject(page, bundle);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: "qualification-created-project-1" }));
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Project bundle imported.");
    const imported = await exportProject(page);
    assert.equal(imported.models.length, 1);
    assert.equal(imported.model_versions.length, 2);
    assert.equal(imported.active_model_id, library.models[1].model_id);
    assert.equal(imported.active_version_id, library.versions[2].version_id, "the older active version must be remapped, not replaced by latest");
    assert.equal(imported.workspace_snapshot.name, originalSnapshot.name);
    assert.deepEqual(imported.workspace_snapshot.nodes, originalSnapshot.nodes);
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Continued research" }));
    assert.equal(library.models.length, 2, "saving an imported model must append a version rather than create another model");
    assert.equal(library.versions.length, 5);
    assert.equal(library.models[0].name, "Roundtrip workspace");
  });
}, { timeout: 90_000 });
}

test("Workbench rejects malformed project sections before creating any server records", async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Unsaved research" }));
    const bundle = await exportProject(page);
    await importProject(page, { ...bundle, models: {} });
    assert.equal(runtime.state.projectMutations.length, 0, "invalid files must be rejected before project creation");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), PROJECT_ID);
    const unchanged = await exportProject(page);
    assert.deepEqual(unchanged.workspace_snapshot, bundle.workspace_snapshot);
  });
}, { timeout: 75_000 });

test("Workbench model save retries a failed version write without duplicating the model", async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
    const created = await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    library.failVersions = true;
    await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Retry version" })), /unavailable|503/u);
    assert.equal(library.versions.length, 1);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), created.modelId);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedVersionId), library.versions[0].version_id);
    library.failVersions = false;
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Retry version" }));
    assert.equal(library.models.length, 1);
    assert.equal(library.versions.length, 2);
    assert.equal((await exportProject(page)).active_version_id, library.versions[1].version_id);
  });
}, { timeout: 75_000 });

test("Workbench save-as keeps the new model active after refreshing an existing project", async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Original", saveAs: true }));
    const copy = await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Copy", saveAs: true }));
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), copy.modelId);
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Edited copy" }));
    assert.equal(library.models.length, 2);
    assert.equal(library.models[0].name, "Original");
    assert.equal(library.models[1].name, "Edited copy");
    assert.equal((await exportProject(page)).active_version_id, library.versions[2].version_id);
  });
}, { timeout: 75_000 });

test("Workbench creating another project detaches the old saved model", async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Original", saveAs: true }));
    const created = await invoke(page, "project/create", { name: "Separate research" });
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), null);
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "New project model" }));
    assert.equal(library.models.length, 2);
    assert.equal(library.models[0].name, "Original");
    assert.equal(library.models[1].project_id, created.projectId);
  });
}, { timeout: 75_000 });

test("Workbench Pwdt can retry cancellation after a transient backend failure", async () => {
  await usingWorkbench(async (page) => {
    const job = {
      job_id: "boundary-running-job", status: "solving", worker_id: "qualification-agent", progress: 0.25,
      has_result: false, created_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
    };
    let unavailable = true;
    let cancels = 0;
    await page.route("**/api/v1/fem/truss-2d/jobs", (route) => route.fulfill({ status: 202, json: { job } }));
    await page.route(`**/api/v1/jobs/${job.job_id}`, (route) => route.fulfill({ json: { job } }));
    await page.route(`**/api/v1/jobs/${job.job_id}/cancel`, async (route) => {
      cancels += 1;
      if (unavailable) await route.fulfill({ status: 503, json: { error: "qualification cancellation unavailable" } });
      else {
        job.status = "cancelled";
        await route.fulfill({ json: { job } });
      }
    });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
    const running = invoke(page, "job/run");
    // The run promise follows the entire job; cancellation is issued while it is polling.
    const completion = running.catch((error) => ({ error: error.message }));
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "solving" }, { timeoutMs: 15_000 }));
    page.once("dialog", (dialog) => dialog.accept());
    await assert.rejects(invoke(page, "job/cancel"), /unavailable|503/u);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().jobStatus), "solving");
    unavailable = false;
    page.once("dialog", (dialog) => dialog.accept());
    await invoke(page, "job/cancel");
    await completion;
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().jobStatus), "cancelled");
    assert.equal(cancels, 2);
  });
}, { timeout: 90_000 });
