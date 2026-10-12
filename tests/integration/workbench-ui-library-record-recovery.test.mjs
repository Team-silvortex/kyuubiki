import assert from "node:assert/strict";
import { test } from "node:test";
import { PROJECT_ID, runtime, usingWorkbench, invoke, openWorkbench, openSavedModels, openProjectManager,
  holdRequest, waitForGuiTransition, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const rows = (page) => page.locator("button.history-item strong");
const cases = [
  { kind: "project", action: "project/create", method: "POST", path: () => "/api/v1/projects", gui: "create" },
  { kind: "project", action: "project/updateSelected", method: "PATCH", path: () => `/api/v1/projects/${PROJECT_ID}`, gui: "update" },
  { kind: "project", action: "project/deleteSelected", method: "DELETE", path: () => `/api/v1/projects/${PROJECT_ID}`, gui: "delete" },
  { kind: "model", action: "model/deleteSelected", method: "DELETE", path: (library) => `/api/v1/models/${library.models[0].model_id}`, gui: "Delete model" },
  { kind: "version", action: "model/renameSelectedVersion", method: "PATCH", path: (library) => `/api/v1/model-versions/${library.versions[1].version_id}`, gui: "Rename version" },
  { kind: "version", action: "model/deleteSelectedVersion", method: "DELETE", path: (library) => `/api/v1/model-versions/${library.versions[1].version_id}`, gui: "Delete version" },
];

async function seed(page) {
  page.on("dialog", (dialog) => dialog.accept());
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Earlier" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Latest" }));
}

async function openPanel(page, entry) {
  if (entry.kind === "project") await openProjectManager(page);
  else {
    await openSavedModels(page);
    if (entry.kind === "version") await page.locator('[data-workbench-library-model-page="versions"]').click();
  }
}

async function mutate(page, library, entry, surface) {
  if (surface === "pwdt") return invoke(page, entry.action, { name: "Edited" });
  const pathname = entry.path(library);
  const response = page.waitForResponse((response) => new URL(response.url()).pathname === pathname &&
    response.request().method() === entry.method);
  if (entry.kind === "project") await page.locator(`[data-workbench-library-project-action="${entry.gui}"]`).click();
  else await page.getByRole("button", { name: entry.gui, exact: true }).click();
  await response;
  await waitForGuiTransition(page);
}

function assertNoExecutionWrites() {
  assert.equal(runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

for (const surface of ["pwdt", "gui"]) for (const entry of cases) {
  test(`Workbench ${surface} ${entry.action} rejects an unconfirmed receipt without losing the working copy`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      await openPanel(page, entry);
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const saved = structuredClone({ models: library.models, versions: library.versions, projects: runtime.state.projects });
      const writes = library.writes.length;
      let attempts = 0;
      let reads = 0;
      await page.route("**/api/v1/projects", (route) => {
        if (route.request().method() === "GET") reads += 1;
        return route.fallback();
      });
      await page.route(`**/api/v1/models/${library.models[0].model_id}/versions`, (route) => { reads += 1; return route.fallback(); });
      await page.route(`**${entry.path(library)}`, (route) => {
        if (route.request().method() !== entry.method) return route.fallback();
        attempts += 1;
        return route.fulfill({ json: null });
      });
      if (surface === "pwdt") await assert.rejects(mutate(page, library, entry, surface), /LIBRARY_RECORD_MUTATION_UNCONFIRMED/u);
      else {
        await mutate(page, library, entry, surface);
        await page.waitForFunction(() => window.__kyuubikiPwdt.state().message.includes("LIBRARY_RECORD_MUTATION_UNCONFIRMED"), null, { timeout: 5_000 });
      }
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName"]) assert.equal(after[key], before[key]);
      assert.equal(attempts, 1);
      assert.equal(reads, 0);
      await invoke(page, "runtime/refreshAll");
      await page.waitForLoadState("networkidle");
      assert.equal(attempts, 1, "read recovery must not replay an unconfirmed library write");
      assert.deepEqual({ models: library.models, versions: library.versions, projects: runtime.state.projects }, saved);
      assert.equal(library.writes.length, writes);
      assertNoExecutionWrites();
    });
  });
}

for (const surface of ["pwdt", "gui"]) for (const entry of cases.filter((entry) => entry.method === "DELETE")) {
  for (const oldRead of [false, "success", "failure"]) test(`Workbench ${surface} confirmed ${entry.kind} deletion removes local records despite failed reads${oldRead ? ` and an older ${oldRead}` : ""}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      await openPanel(page, entry);
      const modelId = library.models[0].model_id;
      const earlierId = library.versions[0].version_id;
      const path = entry.kind === "version" ? `/api/v1/models/${modelId}/versions` : "/api/v1/projects";
      const previous = entry.kind === "version" ? { versions: structuredClone(library.versions).toReversed() }
        : { projects: structuredClone(runtime.state.projects) };
      const beforeWrites = library.writes.length + runtime.state.projectMutations.length;
      let held;
      let refresh;
      if (oldRead) {
        held = await holdRequest(page, path, "GET", (route) => route.fulfill(oldRead === "failure"
          ? { status: 503, json: { error: "obsolete library read" } } : { json: previous }));
        refresh = invoke(page, "runtime/refreshAll");
        void refresh.catch(() => {});
        await held.received;
        await page.unroute(`**${path}`);
      }
      let unavailable = true;
      await page.route("**/api/v1/projects", (route) => route.request().method() === "GET" && unavailable
        ? route.fulfill({ status: 503, json: { error: "post-delete catalog unavailable" } }) : route.fallback());
      await page.route(`**/api/v1/models/${modelId}/versions`, (route) => unavailable
        ? route.fulfill({ status: 503, json: { error: "post-delete versions unavailable" } }) : route.fallback());
      try {
        await mutate(page, library, entry, surface);
        held?.release();
        await refresh;
        assert.equal(await page.getByText(/obsolete library read/u).count(), 0);
        assert.ok(await page.getByText(/post-delete catalog unavailable/u).count() > 0);
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.loadedModelName, "Latest");
        assert.equal(state.selectedProjectId, entry.kind === "project" ? null : PROJECT_ID);
        assert.equal(state.selectedModelId, entry.kind === "version" ? modelId : null);
        assert.equal(state.selectedVersionId, null);
        await openSavedModels(page);
        assert.equal(await rows(page).count(), entry.kind === "version" ? 1 : 0);
        await page.locator('[data-workbench-library-model-page="versions"]').click();
        assert.deepEqual(await rows(page).allTextContents(), entry.kind === "version" ? ["Earlier"] : []);
        assert.equal(state.projectCount, entry.kind === "project" ? 0 : 1);
        const writes = library.writes.length + runtime.state.projectMutations.length;
        assert.equal(writes, beforeWrites + 1);
        unavailable = false;
        await invoke(page, "runtime/refreshAll");
        await page.waitForLoadState("networkidle");
        assert.equal(library.writes.length + runtime.state.projectMutations.length, writes);
        if (entry.kind === "version") assert.equal(library.versions[0].version_id, earlierId);
        assertNoExecutionWrites();
      } finally { held?.release(); await refresh; }
    });
  });
}

for (const surface of ["pwdt", "gui"]) {
  test(`Workbench ${surface} version deletion recovers history independently of a blocked catalog read`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      const entry = cases.at(-1);
      await openPanel(page, entry);
      const modelId = library.models[0].model_id;
      const writes = library.writes.length;
      const held = await holdRequest(page, "/api/v1/projects", "GET",
        (route) => route.fulfill({ status: 503, json: { error: "post-delete catalog unavailable" } }));
      await page.route(`**/api/v1/models/${modelId}/versions`, (route) =>
        route.fulfill({ status: 503, json: { error: "independent version read failed" } }));
      let operation;
      try {
        if (surface === "pwdt") operation = mutate(page, library, entry, surface);
        else await page.getByRole("button", { name: "Delete version", exact: true }).click();
        await held.received;
        await page.getByText(/independent version read failed/u).first().waitFor();
        await page.waitForFunction(() => {
          const names = [...document.querySelectorAll("button.history-item strong")].map((row) => row.textContent);
          return names.length === 1 && names[0] === "Earlier";
        }, null, { timeout: 5_000 });
        assert.deepEqual(await rows(page).allTextContents(), ["Earlier"]);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedVersionId), null);
        held.release();
        await operation;
        if (surface === "gui") await waitForGuiTransition(page);
        assert.equal(library.writes.length, writes + 1);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), "Version deleted.");
        assertNoExecutionWrites();
      } finally { held.release(); await operation; }
    });
  });
}
