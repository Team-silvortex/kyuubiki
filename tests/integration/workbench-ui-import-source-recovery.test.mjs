import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, usingWorkbench, openWorkbench, invoke, exportProject, holdRequest,
  installProjectWorkbenchTestHooks, waitForGuiTransition, openSavedModels } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
async function seed(page) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Import source research" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Second source version" }));
  return exportProject(page);
}
async function startImport(page, bundle) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "projects" }));
  await page.locator('[data-workbench-library-project-page="exchange"]').click();
  await page.locator('[data-workbench-library-project-action="import"]').setInputFiles({
    name: "source-fence.kyuubiki.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(bundle)),
  });
}
async function backendB(page, library) {
  const projects = structuredClone(runtime.state.projects), versions = structuredClone(library.versions), requests = [];
  await page.route("**/import-authority/api/**", (route) => {
    const request = route.request(), path = new URL(request.url()).pathname;
    requests.push({ method: request.method(), path });
    if (request.method() === "GET") {
      if (path.endsWith("/health")) return route.fulfill({ json: { service: "B runtime", status: "ok" } });
      if (path.endsWith("/protocol/agents") || path.endsWith("/agents")) return route.fulfill({ json: { agents: [] } });
      if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [] } });
      if (path.endsWith("/results")) return route.fulfill({ json: { results: [] } });
      if (path.endsWith("/projects")) return route.fulfill({ json: { projects } });
      if (path.endsWith("/versions")) return route.fulfill({ json: { versions } });
    }
    return route.fulfill({ status: 503, json: { error: "unexpected import continuation on B" } });
  });
  await page.evaluate((base) => {
    const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/import-authority`);
    history.replaceState(null, "", url);
  }, runtime.backendUrl);
  await invoke(page, "runtime/refreshAll");
  requests.length = 0;
  return requests;
}

const phases = [
  { name: "project", path: "/api/v1/projects", method: "POST" },
  { name: "model", path: "/api/v1/projects/qualification-created-project-1/models", method: "POST" },
  { name: "version", path: "/api/v1/models/boundary-model-2/versions", method: "POST" },
  { name: "catalog", path: "/api/v1/projects", method: "GET" },
  { name: "history", path: "/api/v1/models/boundary-model-2/versions", method: "GET" },
];
for (const phase of phases) for (const failed of [false, true]) {
  test(`Workbench import stops after an A ${phase.name} ${failed ? "failure" : "receipt"} when backend B becomes current`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const bundle = await seed(page);
      assert.equal(bundle.models.length, 1); assert.equal(bundle.model_versions.length, 2);
      const held = await holdRequest(page, phase.path, phase.method, failed
        ? (route) => route.fulfill({ status: 503, json: { error: "old-source import failure" } })
        : (route) => route.fallback());
      try {
        await startImport(page, bundle); await held.received;
        await waitForGuiTransition(page, true);
        const requests = await backendB(page, library);
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        const finished = page.waitForResponse((response) => new URL(response.url()).pathname === phase.path && response.request().method() === phase.method);
        held.release(); await finished; await page.waitForLoadState("networkidle");
        await waitForGuiTransition(page);
        assert.deepEqual(requests, [], "old-source import cannot write, recover lists or publish through B");
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "message"]) assert.equal(after[key], before[key]);
        assert.equal(runtime.state.projectMutations.length, phase.name === "project" && failed ? 0 : 1, "accepted A project creation is not rolled back or replayed");
        const modelAdded = phase.name === "project" || (phase.name === "model" && failed) ? 0 : 1;
        const extraVersionAdded = ["catalog", "history"].includes(phase.name) || (phase.name === "version" && !failed) ? 1 : 0;
        assert.equal(library.models.length, 1 + modelAdded);
        assert.equal(library.versions.length, 2 + modelAdded + extraVersionAdded);
      } finally { held.release(); }
    });
  });
}

test("Workbench an explicitly restarted import completes on A after an interrupted source switch", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const bundle = await seed(page), held = await holdRequest(page, "/api/v1/projects", "POST");
    try {
      await startImport(page, bundle); await held.received; await waitForGuiTransition(page, true);
      const requests = await backendB(page, library);
      held.release(); await waitForGuiTransition(page);
      assert.deepEqual(requests, []);
      assert.equal(runtime.state.projectMutations.length, 1); assert.equal(library.models.length, 1);
      await page.evaluate((base) => {
        const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", base); history.replaceState(null, "", url);
      }, runtime.backendUrl);
      await invoke(page, "runtime/refreshAll");
      const retry = await holdRequest(page, "/api/v1/projects", "POST");
      try {
        await startImport(page, bundle); await retry.received; await waitForGuiTransition(page, true);
        retry.release(); await waitForGuiTransition(page);
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.selectedProjectId, "qualification-created-project-2");
        assert.equal(state.message, "Project bundle imported.");
        assert.equal(runtime.state.projectMutations.length, 2, "only the explicit second import creates another project");
        assert.equal(library.models.length, 2); assert.equal(library.versions.length, 4);
      } finally { retry.release(); }
    } finally { held.release(); }
  });
});

test("Workbench imported records survive a history outage and read recovery never repeats creation", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const bundle = await seed(page), held = await holdRequest(page, "/api/v1/models/boundary-model-2/versions", "GET");
    library.failVersionReads = true;
    try {
      await startImport(page, bundle); await held.received; await waitForGuiTransition(page, true);
      assert.notEqual((await page.evaluate(() => window.__kyuubikiPwdt.state())).message, "Project bundle imported.");
      held.release(); await waitForGuiTransition(page);
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.selectedProjectId, "qualification-created-project-1");
      assert.equal(state.selectedModelId, "boundary-model-2");
      assert.equal(state.selectedVersionId, library.versions[3].version_id);
      assert.equal(state.message, "Project bundle imported.", "a history read failure is not a failed import commit");
      await page.getByText(/Versions:.*qualification version read unavailable/).first().waitFor();
      await openSavedModels(page);
      await page.locator('[data-workbench-library-model-page="versions"]').click();
      const rows = page.locator("button.history-item strong");
      assert.equal(await rows.count(), 0, "failed imported history cannot borrow the old model's versions");
      const writes = structuredClone(library.writes), creations = structuredClone(runtime.state.projectMutations);
      library.failVersionReads = false;
      await invoke(page, "runtime/refreshAll");
      const names = library.versions.filter((entry) => entry.model_id === "boundary-model-2").toReversed().map((entry) => entry.name);
      assert.equal(names.length, 2);
      await rows.filter({ hasText: names[0] }).waitFor({ state: "visible" });
      assert.deepEqual(await rows.allTextContents(), names);
      assert.equal(await page.getByText(/Versions:.*qualification version read unavailable/).count(), 0);
      assert.deepEqual(library.writes, writes, "read recovery cannot replay import writes");
      assert.deepEqual(runtime.state.projectMutations, creations);
      assert.equal(library.models.length, 2); assert.equal(library.versions.length, 4);
    } finally { held.release(); }
  });
});
