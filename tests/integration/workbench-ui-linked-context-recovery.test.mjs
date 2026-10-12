import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, usingWorkbench, openWorkbench, invoke, holdRequest, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "linked-record";
async function seed(page, library) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Research working" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  const version = structuredClone(library.versions[0]);
  const job = { job_id: ID, status: "completed", progress: 1, has_result: true,
    project_id: version.project_id, model_version_id: version.version_id, simulation_case_id: "linked-case" };
  runtime.state.adminJobs.push(job);
  runtime.state.adminResults.push({ job_id: ID, result: { metric: 42 } });
  await invoke(page, "runtime/refreshAll");
  return { version, job };
}

const button = (page, target, mode) => page.locator(`[data-workbench-data-action="${mode === "apply" ? `apply-${target}-context` : `open-${target}-${mode}`}"]`);
async function openData(page, target) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate((target) => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: `${target}s` }), target);
  await page.locator('[data-workbench-data-page="browse"]').click();
  await page.locator(`[data-workbench-data-record-kind="${target}"][data-workbench-data-record-id="${ID}"]`).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}
async function changeBackend(page) {
  await page.evaluate((base) => {
    const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/linked-authority`);
    history.replaceState(null, "", url);
  }, runtime.backendUrl);
}

for (const surface of ["GUI", "PWDT"]) for (const target of ["job", "result"]) for (const mode of ["project", "version", "apply"]) {
  test(`Workbench ${surface} linked ${target}/${mode} blocks stale source before every read or selection change`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { version } = await seed(page, library);
      if (surface === "GUI") await openData(page, target);
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const reads = [];
      await page.route("**/linked-authority/api/**", (route) => {
        reads.push(route.request().url());
        return route.fulfill({ json: { version, versions: [] } });
      });
      await changeBackend(page);
      if (surface === "GUI") {
        await button(page, target, mode).evaluate((element) => { element.disabled = false; element.click(); });
        await waitForGuiTransition(page);
      } else {
        const outcome = await invoke(page, "data/openLinkedContext", { target, jobId: ID, mode }).catch((error) => ({ error: error.message }));
        assert.match(outcome.error ?? "", /WORKBENCH_RECORD_SOURCE_CHANGED/u);
      }
      await page.waitForLoadState("networkidle");
      assert.deepEqual(reads, []);
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "adminFilterProjectId", "adminFilterModelVersionId"]) assert.equal(after[key], before[key]);
      if (surface === "GUI") {
        assert.match(after.message, /WORKBENCH_RECORD_SOURCE_CHANGED/u);
        assert.equal(await button(page, target, mode).isDisabled(), true);
      }
      assert.equal(library.writes.length, 1, "linked navigation must not submit a new persistence write");
    });
  });
}

for (const target of ["job", "result"]) for (const mode of ["project", "version", "apply"]) {
  test(`Workbench successful linked ${target}/${mode} preserves truthful geometry binding`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { version } = await seed(page, library);
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      await openData(page, target);
      if (target === "job") await button(page, target, mode).click();
      else assert.equal((await invoke(page, "data/openLinkedContext", { target, jobId: ID, mode })).ok, true);
      await waitForGuiTransition(page);
      await page.waitForLoadState("networkidle");
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.selectedProjectId, version.project_id);
      if (mode === "project") {
        assert.equal(state.selectedModelId, null);
        assert.equal(state.selectedVersionId, null);
        assert.equal(state.loadedModelName, "Current research");
        assert.equal(state.sidebarSection, "library");
      } else {
        assert.equal(state.selectedModelId, version.model_id);
        assert.equal(state.selectedVersionId, version.version_id);
        assert.equal(state.loadedModelName, "Research working");
        if (mode === "apply") assert.equal(state.adminFilterProjectId, version.project_id);
      }
      assert.equal(library.writes.length, 1);
    });
  });
}

for (const mode of ["project", "version", "apply"]) {
  test(`Workbench PWDT result ${mode} requires an actual result, not just a same-ID task`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page, library);
      runtime.state.adminResults.length = 0;
      await invoke(page, "runtime/refreshAll");
      const outcome = await invoke(page, "data/openLinkedContext", { target: "result", resultJobId: ID, mode }).catch((error) => ({ error: error.message }));
      assert.match(outcome.error ?? "", /result|Result/u);
      assert.equal(library.writes.length, 1);
    });
  });
}

for (const phase of ["payload", "version-list"]) for (const change of ["backend", "edit"]) {
  test(`Workbench linked load superseded during ${phase} by ${change} cannot continue dependent writes`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { version } = await seed(page, library);
      const held = await holdRequest(page, phase === "payload" ? `/api/v1/model-versions/${version.version_id}` : `/api/v1/models/${version.model_id}/versions`);
      const reads = [];
      await page.route("**/linked-authority/api/**", (route) => {
        reads.push(route.request().url());
        return route.fulfill({ status: 503, json: { error: "unexpected new backend request" } });
      });
      const pending = page.evaluate((jobId) => window.__kyuubikiPwdt.runSteps([
        { action: "data/openLinkedContext", payload: { jobId, mode: "version" } }, { action: "model/saveAs" },
      ]), ID).catch((error) => ({ error: error.message }));
      try {
        await held.received;
        if (change === "backend") await changeBackend(page);
        else await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        held.release();
        const outcome = await pending;
        assert.match(outcome.error ?? "", /WORKBENCH_CONTEXT_CHANGED|WORKBENCH_RECORD_SOURCE_CHANGED/u);
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "message"]) assert.equal(after[key], before[key]);
        assert.deepEqual(reads, []);
        assert.equal(library.writes.length, 1, "dependent SaveAs cannot follow a superseded context load");
      } finally { held.release(); await pending; }
    });
  });
}

test("Workbench linked project requires a verified project catalog after task-only source recovery", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { job } = await seed(page, library);
    let unavailable = true;
    const writes = [];
    await page.route("**/linked-authority/api/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      if (route.request().method() !== "GET") {
        writes.push(path);
        return route.fulfill({ status: 503, json: { error: "unexpected write in read-recovery fixture" } });
      }
      if (path.endsWith("/health")) return route.fulfill({ json: { service: "B qualification runtime", status: "ok" } });
      if (path.endsWith("/protocol/agents")) return route.fulfill({ json: { agents: [] } });
      if (path.endsWith("/agents")) return route.fulfill({ json: { agents: [], summary: { active_execution_lease_count: 0, stale_execution_lease_count: 0 } } });
      if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [job] } });
      if (path.endsWith("/results")) return route.fulfill({ json: { results: runtime.state.adminResults } });
      if (path.endsWith("/projects")) return unavailable ? route.fulfill({ status: 503, json: { error: "B project catalog unavailable" } })
        : route.fulfill({ json: { projects: runtime.state.projects } });
      return route.fulfill({ status: 503, json: { error: "unrelated B read unavailable" } });
    });
    await changeBackend(page);
    await invoke(page, "runtime/refreshAll");
    const outcome = await invoke(page, "data/openLinkedContext", { jobId: ID, mode: "project" }).catch((error) => ({ error: error.message }));
    assert.match(outcome.error ?? "", /WORKBENCH_RECORD_SOURCE_CHANGED/u);
    unavailable = false;
    await invoke(page, "runtime/refreshAll");
    assert.equal((await invoke(page, "data/openLinkedContext", { jobId: ID, mode: "project" })).ok, true);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), null);
    assert.equal(library.writes.length, 1);
    await page.waitForLoadState("networkidle");
    assert.deepEqual(writes, [], "catalog recovery and linked project navigation cannot perform a backend write");
  });
});

test("Workbench linked version cannot read a new backend using an old task record", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { version } = await seed(page, library);
    const reads = [];
    await page.route("**/linked-authority/api/**", (route) => {
      reads.push(route.request().url());
      const path = new URL(route.request().url()).pathname;
      if (path.includes("/model-versions/")) return route.fulfill({ json: { version: { ...version, name: "backend B" } } });
      return route.fulfill({ json: { versions: [] } });
    });
    await page.evaluate((base) => {
      const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/linked-authority`);
      history.replaceState(null, "", url);
    }, runtime.backendUrl);
    const outcome = await invoke(page, "data/openLinkedContext", { jobId: ID, mode: "version" }).catch((error) => ({ error: error.message }));
    assert.deepEqual(reads, [], "retained task links cannot authorize a new backend read");
    assert.match(outcome.error ?? "", /WORKBENCH_RECORD_SOURCE_CHANGED/u);
  });
});

test("Workbench linked version rejects a same-ID version from an unrelated project before loading", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const { version } = await seed(page, library);
    const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
    await page.route(`**/api/v1/model-versions/${version.version_id}`, (route) =>
      route.fulfill({ json: { version: { ...version, project_id: "unrelated-project", name: "wrong version" } } }));
    const outcome = await invoke(page, "data/openLinkedContext", { jobId: ID, mode: "version" }).catch((error) => ({ error: error.message }));
    assert.match(outcome.error ?? "", /WORKBENCH_LINKED_VERSION_INVALID/u);
    const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
    for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "studyKind"]) assert.equal(after[key], before[key]);
  });
});
