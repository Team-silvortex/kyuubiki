import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, initialProject, usingWorkbench, openWorkbench, invoke, holdRequest,
  openSavedModels, openProjectManager, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
async function seed(page, library) {
  runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Saved research" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  await invoke(page, "model/setWorkspaceMeta", { loadedModelName: "Current research" });
  await openSavedModels(page);
  return { model: structuredClone(library.models[0]), version: structuredClone(library.versions[0]) };
}
const snapshot = (page) => page.evaluate(() => window.__kyuubikiPwdt.state());
function unchanged(before, after, feedback = true) {
  for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", ...(feedback ? ["message"] : [])]) {
    assert.equal(after[key], before[key], `obsolete navigation cannot change ${key}`);
  }
}
async function changeBackend(page) {
  await page.evaluate((base) => {
    const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/navigation-authority`);
    history.replaceState(null, "", url);
  }, runtime.backendUrl);
}
async function openRecord(page, kind) {
  if (kind === "version") await page.locator('[data-workbench-library-model-page="versions"]').click();
  await page.locator("button.history-item").filter({ hasText: "Saved research" }).click();
}

test("Workbench saved model rejects a same-ID response from an unrelated project before changing geometry", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { model } = await seed(page, library);
    const before = await snapshot(page);
    await page.route(`**/api/v1/models/${model.model_id}`, (route) => route.fulfill({ json: { model: { ...model, project_id: "unrelated-project" } } }));
    await openRecord(page, "model");
    await page.waitForFunction(() => /WORKBENCH_SAVED_MODEL_INVALID|Loaded a saved model/.test(window.__kyuubikiPwdt.state().message));
    await page.waitForLoadState("networkidle");
    unchanged(before, await snapshot(page), false);
    assert.match((await snapshot(page)).message, /WORKBENCH_SAVED_MODEL_INVALID/u);
    assert.equal(library.writes.length, 1);
  });
});

test("Workbench retained saved model cannot read a new backend before its catalog is refreshed", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { model } = await seed(page, library), reads = [];
    const before = await snapshot(page);
    await page.route("**/navigation-authority/api/**", (route) => {
      reads.push(route.request().url());
      return route.fulfill({ json: { model, versions: [] } });
    });
    await changeBackend(page);
    await openRecord(page, "model");
    await page.waitForFunction(() => /WORKBENCH_RECORD_SOURCE_CHANGED|Loaded a saved model/.test(window.__kyuubikiPwdt.state().message));
    await page.waitForLoadState("networkidle");
    assert.deepEqual(reads, []);
    unchanged(before, await snapshot(page), false);
    assert.match((await snapshot(page)).message, /WORKBENCH_RECORD_SOURCE_CHANGED/u);
    assert.equal(library.writes.length, 1);
  });
});

test("Workbench ordinary project selection supersedes a delayed saved-model load", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { model } = await seed(page, library);
    const held = await holdRequest(page, `/api/v1/models/${model.model_id}`);
    try {
      await openRecord(page, "model");
      await held.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      const before = await snapshot(page);
      const response = page.waitForResponse((response) => new URL(response.url()).pathname === `/api/v1/models/${model.model_id}`);
      held.release();
      await response;
      await page.waitForLoadState("networkidle");
      unchanged(before, await snapshot(page));
      assert.equal((await snapshot(page)).selectedProjectId, "second-project");
      assert.equal((await snapshot(page)).selectedModelId, null);
      assert.equal(library.writes.length, 1);
    } finally { held.release(); }
  });
});

for (const kind of ["model", "version"]) {
  test(`Workbench saved ${kind} opens a valid same-source record and clears an earlier load error`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { model, version } = await seed(page, library);
      const path = kind === "model" ? `/api/v1/models/${model.model_id}` : `/api/v1/model-versions/${version.version_id}`;
      let failed = true;
      await page.route(`**${path}`, (route) => failed ? route.fulfill({ status: 503, json: { error: "temporary model read outage" } }) : route.fallback());
      const before = await snapshot(page);
      await openRecord(page, kind);
      await page.waitForFunction(() => /temporary model read outage/.test(window.__kyuubikiPwdt.state().message));
      unchanged(before, await snapshot(page), false);
      failed = false;
      await openRecord(page, kind);
      await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ loadedModelName: "Saved research" }));
      await page.waitForLoadState("networkidle");
      const after = await snapshot(page);
      assert.equal(after.selectedProjectId, model.project_id);
      assert.equal(after.selectedModelId, model.model_id);
      assert.equal(after.selectedVersionId, version.version_id);
      assert.equal(after.hasResult, false);
      assert.equal(await page.getByText("temporary model read outage", { exact: true }).count(), 0);
      assert.equal(library.writes.length, 1);
    });
  });
}

for (const mismatch of ["id", "project", "model"]) {
  test(`Workbench saved version rejects a wrong ${mismatch} envelope`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { version } = await seed(page, library), before = await snapshot(page);
      const wrong = { ...version, ...(mismatch === "id" ? { version_id: "different" } : {}),
        ...(mismatch === "project" ? { project_id: "different" } : {}), ...(mismatch === "model" ? { model_id: "different" } : {}) };
      await page.route(`**/api/v1/model-versions/${version.version_id}`, (route) => route.fulfill({ json: { version: wrong } }));
      await openRecord(page, "version");
      await page.waitForFunction(() => /WORKBENCH_SAVED_VERSION_INVALID/.test(window.__kyuubikiPwdt.state().message));
      unchanged(before, await snapshot(page), false);
      assert.equal(library.writes.length, 1);
    });
  });
}

test("Workbench retained saved version requires a current version-list source", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { version } = await seed(page, library), reads = [];
    await page.locator('[data-workbench-library-model-page="versions"]').click();
    const before = await snapshot(page);
    await page.route("**/navigation-authority/api/**", (route) => {
      reads.push(route.request().url()); return route.fulfill({ json: { version, versions: [] } });
    });
    await changeBackend(page);
    await openRecord(page, "version");
    await page.waitForFunction(() => /WORKBENCH_RECORD_SOURCE_CHANGED/.test(window.__kyuubikiPwdt.state().message));
    unchanged(before, await snapshot(page), false);
    assert.deepEqual(reads, []);
    assert.equal(library.writes.length, 1);
  });
});

for (const kind of ["model", "version"]) for (const change of ["backend", "project", "feedback"]) for (const failed of [false, true]) {
  test(`Workbench pending saved ${kind} ${failed ? "failure" : "payload"} cannot replace a newer ${change}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { model, version } = await seed(page, library);
      const path = kind === "model" ? `/api/v1/models/${model.model_id}` : `/api/v1/model-versions/${version.version_id}`;
      const held = await holdRequest(page, path, "GET", (route) => failed
        ? route.fulfill({ status: 503, json: { error: "obsolete read outage" } }) : route.fallback());
      const reads = [];
      await page.route("**/navigation-authority/api/**", (route) => {
        reads.push(route.request().url()); return route.fulfill({ status: 503, json: { error: "unexpected new-source read" } });
      });
      try {
        await openRecord(page, kind);
        await held.received;
        if (change === "backend") await changeBackend(page);
        else if (change === "project") await invoke(page, "project/select", { projectId: "second-project" });
        else await invoke(page, "model/setWorkspaceMeta", { loadedModelName: "Newer edit" });
        const before = await snapshot(page);
        const response = page.waitForResponse((response) => new URL(response.url()).pathname === path);
        held.release(); await response;
        await page.waitForLoadState("networkidle");
        unchanged(before, await snapshot(page));
        assert.deepEqual(reads, [], "obsolete payload cannot start a new-source version-list read");
        assert.equal(library.writes.length, 1);
      } finally { held.release(); }
    });
  });
}

for (const surface of ["GUI", "PWDT"]) {
  test(`Workbench ${surface} ordinary project selection rejects a retained old-source project`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, library);
      if (surface === "GUI") await openProjectManager(page);
      const before = await snapshot(page);
      await changeBackend(page);
      if (surface === "GUI") {
        await page.locator('[data-workbench-library-project-field="selection"]').selectOption("second-project");
        await page.waitForFunction(() => /WORKBENCH_RECORD_SOURCE_CHANGED/.test(window.__kyuubikiPwdt.state().message));
      } else {
        const outcome = await invoke(page, "project/select", { projectId: "second-project" }).catch((error) => ({ error: error.message }));
        assert.match(outcome.error ?? "", /WORKBENCH_RECORD_SOURCE_CHANGED/);
      }
      unchanged(before, await snapshot(page), false);
      assert.equal(library.writes.length, 1);
    });
  });

  test(`Workbench ${surface} project deselection preserves geometry and detaches saved bindings`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, library);
      if (surface === "GUI") {
        await openProjectManager(page);
        await page.locator('[data-workbench-library-project-field="selection"]').selectOption("");
      } else await invoke(page, "project/select", { projectId: null });
      await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: null, selectedModelId: null, selectedVersionId: null }));
      assert.equal((await snapshot(page)).loadedModelName, "Current research");
      assert.equal(library.writes.length, 1);
    });
  });
}

for (const kind of ["model", "version"]) {
  test(`Workbench saved ${kind} can recover after a same-ID catalog is actually read from the new backend`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { model, version } = await seed(page, library), writes = [];
      await page.route("**/navigation-authority/api/**", (route) => {
        const path = new URL(route.request().url()).pathname;
        if (route.request().method() !== "GET") {
          writes.push(path); return route.fulfill({ status: 503, json: { error: "unexpected write" } });
        }
        if (path.endsWith("/health")) return route.fulfill({ json: { service: "recovered runtime", status: "ok" } });
        if (path.endsWith("/protocol/agents") || path.endsWith("/agents")) return route.fulfill({ json: { agents: [] } });
        if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [] } });
        if (path.endsWith("/results")) return route.fulfill({ json: { results: [] } });
        if (path.endsWith("/projects")) return route.fulfill({ json: { projects: runtime.state.projects } });
        if (path.endsWith("/versions")) return route.fulfill({ json: { versions: [version] } });
        if (path.endsWith(`/model-versions/${version.version_id}`)) return route.fulfill({ json: { version: { ...version, name: "Recovered source" } } });
        if (path.endsWith(`/models/${model.model_id}`)) return route.fulfill({ json: { model: { ...model, name: "Recovered source" } } });
        return route.fulfill({ status: 503, json: { error: "unrelated read" } });
      });
      await changeBackend(page);
      await invoke(page, "runtime/refreshAll");
      await openRecord(page, kind);
      await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ loadedModelName: "Recovered source" }));
      await page.waitForLoadState("networkidle");
      assert.equal(library.writes.length, 1);
      assert.deepEqual(writes, []);
    });
  });
}

test("Workbench malformed version history retains known rows without offering a foreign version", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { version } = await seed(page, library);
    await page.route(`**/api/v1/models/${version.model_id}/versions`, (route) => route.fulfill({ json: {
      versions: [{ ...version, version_id: "foreign", model_id: "different", name: "Foreign version" }],
    } }));
    await invoke(page, "runtime/refreshAll");
    await page.locator('[data-workbench-library-model-page="versions"]').click();
    assert.equal(await page.locator("button.history-item").filter({ hasText: "Saved research" }).count(), 1);
    assert.equal(await page.getByRole("button", { name: /Foreign version/ }).count(), 0);
    assert.equal(library.writes.length, 1);
  });
});

for (const failed of [false, true]) {
  test(`Workbench GUI model-name editing suppresses a delayed saved-model ${failed ? "failure" : "payload"}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { model } = await seed(page, library), path = `/api/v1/models/${model.model_id}`;
      const held = await holdRequest(page, path, "GET", (route) => failed
        ? route.fulfill({ status: 503, json: { error: "obsolete GUI read" } }) : route.fallback());
      try {
        await openRecord(page, "model"); await held.received;
        await page.getByRole("textbox", { name: "Model", exact: true }).fill("Newer GUI edit");
        await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ loadedModelName: "Newer GUI edit" }));
        const before = await snapshot(page), response = page.waitForResponse((response) => new URL(response.url()).pathname === path);
        held.release(); await response; await page.waitForLoadState("networkidle");
        unchanged(before, await snapshot(page));
        assert.equal(library.writes.length, 1);
      } finally { held.release(); }
    });
  });
}
