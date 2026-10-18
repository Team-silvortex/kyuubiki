import assert from "node:assert/strict";
import { test } from "node:test";
import { PROJECT_ID, runtime, usingWorkbench, openWorkbench, invoke, openSavedModels, openProjectManager, holdRequest,
  installProjectWorkbenchTestHooks, waitForGuiTransition } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
async function seed(page, library) {
  page.on("dialog", (dialog) => dialog.accept());
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Working research" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  await openSavedModels(page);
  return { model: structuredClone(library.models[0]), version: structuredClone(library.versions[0]) };
}

for (const firstSurface of ["PWDT", "GUI"]) {
  test(`Workbench ${firstSurface} pending model save rejects the other surface's deletion instead of merging success`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { model } = await seed(page, library);
      const path = `/api/v1/models/${model.model_id}/versions`;
      const held = await holdRequest(page, path, "POST");
      const deletes = [];
      page.on("request", (request) => {
        if (request.method() === "DELETE" && new URL(request.url()).pathname === `/api/v1/models/${model.model_id}`) deletes.push(request.url());
      });
      let first;
      try {
        if (firstSurface === "PWDT") { first = invoke(page, "model/save"); void first.catch(() => {}); }
        else await page.getByRole("button", { name: "Save", exact: true }).click();
        await held.received;
        if (firstSurface === "PWDT") {
          await page.getByRole("button", { name: "Delete model", exact: true }).click();
          await page.waitForFunction(() => /WORKBENCH_LIBRARY_WRITE_PENDING|Deleted/.test(window.__kyuubikiPwdt.state().message));
        } else {
          const outcome = await invoke(page, "model/deleteSelected").catch((error) => ({ error: error.message }));
          assert.match(outcome.error ?? "", /WORKBENCH_LIBRARY_WRITE_PENDING/);
        }
        assert.deepEqual(deletes, [], "a conflicting delete cannot dispatch while the model save owns recovery");
        assert.equal(library.models.length, 1);
      } finally { held.release(); await first; await page.waitForLoadState("networkidle"); }
    });
  });
}

const actions = {
  save: { action: "model/save", method: "POST", gui: "Save", path: (library) => `/api/v1/models/${library.models[0].model_id}/versions` },
  saveAs: { action: "model/saveAs", method: "POST", gui: "Save As", path: () => `/api/v1/projects/${PROJECT_ID}/models` },
  modelDelete: { action: "model/deleteSelected", method: "DELETE", gui: "Delete model", path: (library) => `/api/v1/models/${library.models[0].model_id}` },
  rename: { action: "model/renameSelectedVersion", method: "PATCH", gui: "Rename version", version: true, path: (library) => `/api/v1/model-versions/${library.versions[0].version_id}` },
  versionDelete: { action: "model/deleteSelectedVersion", method: "DELETE", gui: "Delete version", version: true, path: (library) => `/api/v1/model-versions/${library.versions[0].version_id}` },
  projectUpdate: { action: "project/updateSelected", method: "PATCH", gui: "update", project: true, path: () => `/api/v1/projects/${PROJECT_ID}` },
  projectDelete: { action: "project/deleteSelected", method: "DELETE", gui: "delete", project: true, path: () => `/api/v1/projects/${PROJECT_ID}` },
  projectCreate: { action: "project/create", method: "POST", gui: "create", project: true, path: () => "/api/v1/projects" },
};
async function panel(page, entry) {
  if (entry.project) await openProjectManager(page);
  else { await openSavedModels(page); if (entry.version) await page.locator('[data-workbench-library-model-page="versions"]').click(); }
}
async function click(page, entry) {
  if (entry.project) await page.locator(`[data-workbench-library-project-action="${entry.gui}"]`).click();
  else await page.getByRole("button", { name: entry.gui, exact: true }).click();
}
async function blocked(page, entry, surface) {
  if (surface === "PWDT") await assert.rejects(invoke(page, entry.action), /WORKBENCH_LIBRARY_WRITE_PENDING/);
  else { await panel(page, entry); await click(page, entry);
    await page.waitForFunction(() => /WORKBENCH_LIBRARY_WRITE_PENDING/.test(window.__kyuubikiPwdt.state().message)); }
}
const pairs = [["save", "rename"], ["versionDelete", "save"], ["rename", "modelDelete"],
  ["modelDelete", "projectDelete"], ["projectUpdate", "saveAs"], ["projectCreate", "projectCreate"]];
for (const surface of ["GUI", "PWDT"]) for (const [firstName, nextName] of pairs) {
  test(`Workbench ${surface} ${firstName} excludes the other surface's ${nextName} until recovery completes`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, library); const firstEntry = actions[firstName], nextEntry = actions[nextName];
      if (surface === "GUI") await panel(page, firstEntry);
      const held = await holdRequest(page, firstEntry.path(library), firstEntry.method);
      const writes = [];
      page.on("request", (request) => {
        const path = new URL(request.url()).pathname;
        if (["POST", "PATCH", "DELETE"].includes(request.method()) && /^\/api\/v1\/(projects|models|model-versions)(\/|$)/.test(path)) writes.push({ method: request.method(), path });
      });
      let first;
      try {
        if (surface === "PWDT") { first = invoke(page, firstEntry.action); void first.catch(() => {}); }
        else await click(page, firstEntry);
        await held.received;
        await blocked(page, nextEntry, surface === "GUI" ? "PWDT" : "GUI");
        assert.equal(writes.length, 1, "conflicting work is rejected, never queued or merged");
        held.release(); await first; await page.waitForLoadState("networkidle");
        assert.equal(writes.length, 1, "release cannot silently execute the blocked action");
      } finally { held.release(); await first; }
    });
  });
}

for (const phase of ["catalog", "versions"]) for (const surface of ["GUI", "PWDT"]) {
  test(`Workbench ${surface} saved-model reservation survives its ${phase} read and permits an explicit later rename`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, library);
      const modelId = library.models[0].model_id, path = phase === "catalog" ? "/api/v1/projects" : `/api/v1/models/${modelId}/versions`;
      const held = await holdRequest(page, path);
      let first;
      try {
        if (surface === "PWDT") { first = invoke(page, "model/save"); void first.catch(() => {}); }
        else await click(page, actions.save);
        await held.received;
        if (surface === "GUI") await waitForGuiTransition(page, true);
        assert.equal(library.writes.length, 2, "write confirmation precedes the blocked recovery read");
        await blocked(page, actions.rename, surface === "GUI" ? "PWDT" : "GUI");
        assert.equal(library.writes.length, 2);
        held.release(); await first; await page.waitForLoadState("networkidle");
        if (surface === "GUI") await waitForGuiTransition(page);
        assert.equal(library.writes.length, 2);
        assert.equal((await invoke(page, "model/renameSelectedVersion")).ok, true);
        assert.equal(library.writes.length, 3);
      } finally { held.release(); await first; }
    });
  });
}

for (const surface of ["GUI", "PWDT"]) {
  test(`Workbench ${surface} failed save releases exclusion but never replays the blocked deletion`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, library);
      const held = await holdRequest(page, actions.save.path(library), "POST", (route) => route.fulfill({ status: 503, json: { error: "explicit save failure" } }));
      let first;
      try {
        if (surface === "PWDT") first = invoke(page, "model/save").catch((error) => ({ error: error.message }));
        else await click(page, actions.save);
        await held.received;
        await blocked(page, actions.modelDelete, surface === "GUI" ? "PWDT" : "GUI");
        const response = page.waitForResponse((response) => new URL(response.url()).pathname === actions.save.path(library) && response.request().method() === "POST");
        held.release(); await response; await first; await page.waitForLoadState("networkidle");
        assert.equal(library.models.length, 1); assert.equal(library.writes.length, 1);
        assert.equal((await invoke(page, "model/renameSelectedVersion")).ok, true);
        assert.equal(library.writes.length, 2, "only the explicit rename may run after failure");
      } finally { held.release(); await first; }
    });
  });
}

test("Workbench PWDT busy rejection stops dependent Save As instead of borrowing an earlier save's success", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page, library); const held = await holdRequest(page, actions.save.path(library), "POST");
    const first = invoke(page, "model/save"); void first.catch(() => {});
    try {
      await held.received;
      await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.runSteps([
        { action: "model/deleteSelected" }, { action: "model/saveAs" },
      ])), /WORKBENCH_LIBRARY_WRITE_PENDING/);
      assert.equal(library.models.length, 1); assert.equal(library.writes.length, 1);
      held.release(); await first; await page.waitForLoadState("networkidle");
      assert.equal(library.models.length, 1); assert.equal(library.writes.length, 2);
    } finally { held.release(); await first; }
  });
});

test("Workbench same-ID B project editing is independent of an active A model write", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page, library); const projects = structuredClone(runtime.state.projects), versions = structuredClone(library.versions), bWrites = [];
    await page.route("**/flight-authority/api/**", (route) => {
      const request = route.request(), path = new URL(request.url()).pathname;
      if (request.method() === "GET") {
        if (path.endsWith("/health")) return route.fulfill({ json: { service: "B runtime", status: "ok" } });
        if (path.endsWith("/protocol/agents") || path.endsWith("/agents")) return route.fulfill({ json: { agents: [] } });
        if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [] } });
        if (path.endsWith("/results")) return route.fulfill({ json: { results: [] } });
        if (path.endsWith("/projects")) return route.fulfill({ json: { projects } });
        if (path.endsWith("/versions")) return route.fulfill({ json: { versions } });
      }
      if (request.method() === "PATCH" && path.endsWith(`/projects/${PROJECT_ID}`)) {
        bWrites.push(path); Object.assign(projects[0], request.postDataJSON());
        return route.fulfill({ json: { project: projects[0] } });
      }
      return route.fulfill({ status: 503, json: { error: "unrelated B operation" } });
    });
    const held = await holdRequest(page, actions.save.path(library), "POST");
    const first = invoke(page, "model/save").catch(error => ({ error: error.message }));
    try {
      await held.received;
      await page.evaluate((base) => { const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/flight-authority`); history.replaceState(null, "", url); }, runtime.backendUrl);
      await invoke(page, "runtime/refreshAll");
      assert.equal((await invoke(page, "project/updateSelected", { name: "B project" })).ok, true);
      assert.equal(bWrites.length, 1);
      held.release(); assert.match((await first).error ?? "", /checkpoint:context_changed/);
      await page.waitForLoadState("networkidle");
      assert.equal(projects[0].name, "B project"); assert.equal(library.writes.length, 2);
    } finally { held.release(); await first; }
  });
});
