import assert from "node:assert/strict";
import { test } from "node:test";
import { PROJECT_ID, runtime, usingWorkbench, openWorkbench, invoke, openSavedModels, openProjectManager,
  holdRequest, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const cases = [
  { action: "project/updateSelected", kind: "project", method: "PATCH", gui: "update", path: () => `/api/v1/projects/${PROJECT_ID}` },
  { action: "project/deleteSelected", kind: "project", method: "DELETE", gui: "delete", path: () => `/api/v1/projects/${PROJECT_ID}` },
  { action: "model/save", kind: "model", method: "POST", gui: "Save", path: (library) => `/api/v1/models/${library.models[0].model_id}/versions` },
  { action: "model/saveAs", kind: "model", method: "POST", gui: "Save As", path: () => `/api/v1/projects/${PROJECT_ID}/models` },
  { action: "model/deleteSelected", kind: "model", method: "DELETE", gui: "Delete model", path: (library) => `/api/v1/models/${library.models[0].model_id}` },
  { action: "model/renameSelectedVersion", kind: "version", method: "PATCH", gui: "Rename version", path: (library) => `/api/v1/model-versions/${library.versions[0].version_id}` },
  { action: "model/deleteSelectedVersion", kind: "version", method: "DELETE", gui: "Delete version", path: (library) => `/api/v1/model-versions/${library.versions[0].version_id}` },
];
async function seed(page, entry) {
  page.on("dialog", (dialog) => dialog.accept());
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Working research" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  if (entry.kind === "project") await openProjectManager(page);
  else {
    await openSavedModels(page);
    if (entry.kind === "version") await page.locator('[data-workbench-library-model-page="versions"]').click();
  }
}
const snapshot = (page) => page.evaluate(() => window.__kyuubikiPwdt.state());
const isAudit = (request) => request.method === "POST"
  && (request.path ?? new URL(request.url).pathname).endsWith("/api/v1/security-events");
function noLibraryRequests(requests) {
  assert.deepEqual(requests.filter((request) => !isAudit(request)), [], "library writes and recovery reads remain blocked; independent security audit posts are not library mutations");
}
async function changeBackend(page) {
  await page.evaluate((base) => {
    const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/write-authority`);
    history.replaceState(null, "", url);
  }, runtime.backendUrl);
}
async function click(page, entry) {
  if (entry.kind === "project") await page.locator(`[data-workbench-library-project-action="${entry.gui}"]`).click();
  else await page.getByRole("button", { name: entry.gui, exact: true }).click();
}
function unchanged(before, after, feedback = true) {
  for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "projectCount", ...(feedback ? ["message"] : [])]) {
    assert.equal(after[key], before[key], `old-source write cannot change ${key}`);
  }
}

test("Workbench old-source GUI model deletion cannot write to a new backend", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const entry = cases.find((entry) => entry.action === "model/deleteSelected");
    await seed(page, entry);
    const before = await snapshot(page), requests = [];
    await page.route("**/write-authority/api/**", (route) => {
      requests.push({ method: route.request().method(), url: route.request().url() });
      return route.fulfill({ json: { model: library.models[0], projects: runtime.state.projects } });
    });
    await changeBackend(page); await click(page, entry);
    await page.waitForFunction(() => /WORKBENCH_RECORD_SOURCE_CHANGED|Deleted/.test(window.__kyuubikiPwdt.state().message));
    await page.waitForLoadState("networkidle");
    noLibraryRequests(requests);
    unchanged(before, await snapshot(page), false);
    assert.equal(library.writes.length, 1);
  });
});

test("Workbench old-source PWDT project update cannot write to a new backend", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const entry = cases[0]; await seed(page, entry);
    const before = await snapshot(page), requests = [];
    await page.route("**/write-authority/api/**", (route) => {
      requests.push({ method: route.request().method(), url: route.request().url() });
      return route.fulfill({ json: { project: runtime.state.projects[0], projects: runtime.state.projects } });
    });
    await changeBackend(page);
    const result = await invoke(page, entry.action, { name: "New name" }).catch((error) => ({ error: error.message }));
    assert.match(result.error ?? "", /WORKBENCH_RECORD_SOURCE_CHANGED/);
    noLibraryRequests(requests);
    unchanged(before, await snapshot(page), false);
    assert.equal(library.writes.length, 1);
  });
});

for (const surface of ["GUI", "PWDT"]) for (const entry of cases) {
  if ((surface === "GUI" && entry.action === "model/deleteSelected") || (surface === "PWDT" && entry.action === "project/updateSelected")) continue;
  test(`Workbench ${surface} ${entry.action} requires a current catalog before any write`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, entry); const before = await snapshot(page), requests = [];
      await page.route("**/write-authority/api/**", (route) => {
        requests.push({ method: route.request().method(), url: route.request().url() });
        return route.fulfill({ status: 503, json: { error: "unexpected stale-source request" } });
      });
      await changeBackend(page);
      if (surface === "GUI") {
        await click(page, entry);
        await page.waitForFunction(() => /WORKBENCH_RECORD_SOURCE_CHANGED/.test(window.__kyuubikiPwdt.state().message));
      } else {
        const result = await invoke(page, entry.action, { name: "New name" }).catch((error) => ({ error: error.message }));
        assert.match(result.error ?? "", /WORKBENCH_RECORD_SOURCE_CHANGED/);
      }
      await page.waitForLoadState("networkidle");
      noLibraryRequests(requests); unchanged(before, await snapshot(page), false);
      assert.equal(library.writes.length, 1);
    });
  });
}

async function installBackendB(page, library) {
  const projects = structuredClone(runtime.state.projects), versions = structuredClone(library.versions);
  const requests = [];
  await page.route("**/write-authority/api/**", (route) => {
    const request = route.request(), path = new URL(request.url()).pathname;
    requests.push({ method: request.method(), path });
    if (request.method() === "POST" && path.endsWith("/api/v1/security-events")) {
      return route.fulfill({ json: { event: { ...request.postDataJSON(), event_id: "fixture-audit" } } });
    }
    if (request.method() === "GET") {
      if (path.endsWith("/health")) return route.fulfill({ json: { service: "B runtime", status: "ok" } });
      if (path.endsWith("/protocol/agents") || path.endsWith("/agents")) return route.fulfill({ json: { agents: [] } });
      if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [] } });
      if (path.endsWith("/results")) return route.fulfill({ json: { results: [] } });
      if (path.endsWith("/projects")) return route.fulfill({ json: { projects } });
      if (path.endsWith("/versions")) return route.fulfill({ json: { versions } });
    }
    if (request.method() === "PATCH" && path.endsWith(`/projects/${PROJECT_ID}`)) {
      Object.assign(projects[0], request.postDataJSON()); return route.fulfill({ json: { project: projects[0] } });
    }
    if (request.method() === "PATCH" && path.endsWith(`/model-versions/${versions[0].version_id}`)) {
      Object.assign(versions[0], request.postDataJSON()); return route.fulfill({ json: { version: versions[0] } });
    }
    return route.fulfill({ status: 503, json: { error: "unrelated B operation unavailable" } });
  });
  return { projects, versions, requests };
}

for (const surface of ["GUI", "PWDT"]) for (const entry of cases) {
  test(`Workbench ${surface} late ${entry.action} confirmation cannot reconcile or refresh a same-ID B catalog`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, entry);
      const b = await installBackendB(page, library), originalB = structuredClone({ projects: b.projects, versions: b.versions });
      const path = entry.path(library), held = await holdRequest(page, path, entry.method);
      let pending;
      try {
        if (surface === "PWDT") { pending = invoke(page, entry.action, { name: "Old-source change" }); void pending.catch(() => {}); }
        else await click(page, entry);
        await held.received;
        await changeBackend(page); await invoke(page, "runtime/refreshAll");
        await invoke(page, "model/setWorkspaceMeta", { loadedModelName: "B working copy" });
        const before = await snapshot(page); b.requests.length = 0;
        const response = page.waitForResponse((response) => new URL(response.url()).pathname === path && response.request().method() === entry.method);
        held.release(); await response;
        const outcome = await pending; await page.waitForLoadState("networkidle");
        unchanged(before, await snapshot(page));
        noLibraryRequests(b.requests);
        assert.deepEqual({ projects: b.projects, versions: b.versions }, originalB);
        assert.equal(library.writes.length + runtime.state.projectMutations.length, 2, "the original A write is retained exactly once in the fixture");
        if (surface === "PWDT") { assert.equal(outcome.ok, true); assert.equal(outcome.contextChanged, true); }
      } finally { held.release(); await pending; }
    });
  });
}

for (const surface of ["GUI", "PWDT"]) for (const entry of cases.filter((entry) =>
  ["project/updateSelected", "model/save", "model/deleteSelectedVersion"].includes(entry.action))) {
  test(`Workbench ${surface} failed old-source ${entry.action} cannot warn over the B workspace or retry`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, entry); const b = await installBackendB(page, library), path = entry.path(library);
      const held = await holdRequest(page, path, entry.method, (route) => route.fulfill({ status: 503, json: { error: "old-source write failure" } }));
      let pending;
      try {
        if (surface === "PWDT") pending = invoke(page, entry.action).catch((error) => ({ error: error.message }));
        else await click(page, entry);
        await held.received; await changeBackend(page); await invoke(page, "runtime/refreshAll");
        const before = await snapshot(page); b.requests.length = 0;
        const response = page.waitForResponse((response) => new URL(response.url()).pathname === path && response.request().method() === entry.method);
        held.release(); await response; const outcome = await pending;
        await page.waitForLoadState("networkidle"); unchanged(before, await snapshot(page));
        noLibraryRequests(b.requests); assert.equal(library.writes.length, 1);
        if (surface === "PWDT") assert.match(outcome.error ?? "", /old-source write failure/);
      } finally { held.release(); await pending; }
    });
  });
}

for (const surface of ["GUI", "PWDT"]) for (const failed of [false, true]) {
  test(`Workbench ${surface} new-project ${failed ? "failure" : "confirmation"} stays with its dispatch backend`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const entry = { kind: "project", action: "project/create", gui: "create" };
      await seed(page, entry); const b = await installBackendB(page, library);
      const held = await holdRequest(page, "/api/v1/projects", "POST", (route) => failed
        ? route.fulfill({ status: 503, json: { error: "old project creation failure" } }) : route.fallback());
      let pending;
      try {
        if (surface === "PWDT") pending = invoke(page, entry.action, { name: "Created on A" }).catch((error) => ({ error: error.message }));
        else await click(page, entry);
        await held.received; await changeBackend(page); await invoke(page, "runtime/refreshAll");
        const before = await snapshot(page); b.requests.length = 0;
        const response = page.waitForResponse((response) => new URL(response.url()).pathname === "/api/v1/projects" && response.request().method() === "POST");
        held.release(); await response; const outcome = await pending;
        await page.waitForLoadState("networkidle"); unchanged(before, await snapshot(page));
        noLibraryRequests(b.requests);
        assert.equal(runtime.state.projectMutations.length, failed ? 0 : 1);
        if (surface === "PWDT") {
          if (failed) assert.match(outcome.error ?? "", /old project creation failure/);
          else { assert.equal(outcome.ok, true); assert.equal(outcome.contextChanged, true); }
        }
      } finally { held.release(); await pending; }
    });
  });
}

for (const action of ["project/updateSelected", "model/renameSelectedVersion"]) {
  test(`Workbench explicit read recovery permits ${action} only against the verified B target`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const entry = cases.find((entry) => entry.action === action); await seed(page, entry);
      const b = await installBackendB(page, library); await changeBackend(page);
      await assert.rejects(invoke(page, action), /WORKBENCH_RECORD_SOURCE_CHANGED/);
      noLibraryRequests(b.requests);
      await invoke(page, "runtime/refreshAll"); b.requests.length = 0;
      assert.equal((await invoke(page, action, { name: "B project edit" })).ok, true);
      assert.equal(b.requests.filter((request) => request.method === "PATCH").length, 1);
      assert.equal(library.writes.length, 1, "verified B editing cannot replay writes on A");
    });
  });
}
