import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, before, test } from "node:test";

import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

const PROJECT_ID = "qualification-project";
let browser;
let runtime;
let initialProject;

before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  initialProject = structuredClone(runtime.state.projects[0]);
  browser = await launchIntegrationBrowser(chromium);
}, { timeout: 180_000 });

after(async () => {
  try { await browser?.close(); } finally { await runtime?.stop(); }
}, { timeout: 90_000 });

async function usingWorkbench(run) {
  runtime.state.projects.splice(0, runtime.state.projects.length, structuredClone(initialProject));
  for (const key of ["adminJobs", "adminResults", "projectMutations", "jobRecordMutations", "resultRecordMutations"]) {
    runtime.state[key].length = 0;
  }
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  try {
    await run(page);
    assert.deepEqual(errors, []);
  } catch (error) {
    throw new Error(`${error.message}\nBrowser errors: ${JSON.stringify(errors)}`, { cause: error });
  } finally {
    await context.close();
  }
}

async function openWorkbench(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
  await page.evaluate((projectId) => window.__kyuubikiPwdt.waitForState(
    { selectedProjectId: projectId }, { timeoutMs: 15_000 },
  ), PROJECT_ID);
}

async function invoke(page, action, payload = {}) {
  return page.evaluate(({ action, payload }) => window.__kyuubikiPwdt.invoke(action, payload), { action, payload });
}

async function exportProject(page) {
  page.once("dialog", (dialog) => dialog.accept());
  const downloaded = page.waitForEvent("download", { timeout: 20_000 });
  const outcome = await invoke(page, "project/exportJson");
  const download = await downloaded;
  const filename = await download.path();
  assert.ok(filename);
  return { outcome, bundle: JSON.parse(await readFile(filename, "utf8")) };
}

function job(jobId, projectId) {
  return {
    job_id: jobId, project_id: projectId, status: "completed", progress: 1,
    worker_id: "qualification-agent", has_result: true,
    created_at: "2026-08-13T00:00:00.000Z", updated_at: "2026-08-13T00:00:00.000Z",
  };
}

test("Workbench Pwdt generates, undoes, redoes, and exports a parametric model", async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({
      bays: 4, span: 12, height: 2, loadY: -800, modelName: "recovery-truss",
    }));
    const first = (await exportProject(page)).bundle.workspace_snapshot;
    assert.equal(first.kind, "truss_2d");
    assert.equal(first.name, "recovery-truss");
    assert.equal(first.nodes.length, 9);
    assert.equal(Math.max(...first.nodes.map((node) => node.x)), 12);
    assert.equal(Math.max(...first.nodes.map((node) => node.y)), 2);

    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({
      bays: 6, span: 18, height: 3, loadY: -1200, modelName: "recovery-truss",
    }));
    const second = (await exportProject(page)).bundle.workspace_snapshot;
    assert.equal(second.nodes.length, 13);
    await invoke(page, "history/undo");
    const undone = (await exportProject(page)).bundle.workspace_snapshot;
    assert.deepEqual(undone.nodes, first.nodes);
    assert.deepEqual(undone.elements, first.elements);
    await invoke(page, "history/redo");
    const redone = (await exportProject(page)).bundle.workspace_snapshot;
    assert.deepEqual(redone.nodes, second.nodes);
    assert.deepEqual(redone.elements, second.elements);
    assert.equal(runtime.state.projectMutations.length, 0);
  });
}, { timeout: 90_000 });

test("Workbench Pwdt retries a failed project creation without losing the active project", async () => {
  await usingWorkbench(async (page) => {
    let unavailable = true;
    await page.route("**/api/v1/projects", async (route) => {
      if (route.request().method() === "POST" && unavailable) {
        await route.fulfill({ status: 503, json: { error: "qualification project service unavailable" } });
      } else await route.continue();
    });
    await openWorkbench(page);
    await assert.rejects(invoke(page, "project/create", { name: "Retry project" }), /unavailable|503/u);
    assert.equal(runtime.state.projectMutations.length, 0);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedProjectId), PROJECT_ID);
    unavailable = false;
    const created = await invoke(page, "project/create", { name: "Retry project" });
    await page.evaluate((projectId) => window.__kyuubikiPwdt.waitForState(
      { selectedProjectId: projectId, projectCount: 2 }, { timeoutMs: 10_000 },
    ), created.projectId);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
    await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "projects" }));
    const selection = page.locator('[data-workbench-library-project-field="selection"]');
    await selection.waitFor({ state: "visible" });
    assert.equal(await selection.inputValue(), created.projectId);
    assert.match(await selection.innerText(), /Retry project/u);
    assert.equal(runtime.state.projectMutations.length, 1);
  });
}, { timeout: 75_000 });

test("Workbench project export excludes other projects and unassigned jobs", async () => {
  await usingWorkbench(async (page) => {
    const owned = job("owned-result", PROJECT_ID);
    const foreign = job("foreign-result", "another-project");
    const unassigned = job("unassigned-result", null);
    runtime.state.adminJobs.push(owned, foreign, unassigned);
    const fetched = [];
    await page.route("**/api/v1/jobs/*", async (route) => {
      const jobId = new URL(route.request().url()).pathname.split("/").at(-1);
      fetched.push(jobId);
      const record = runtime.state.adminJobs.find((entry) => entry.job_id === jobId);
      await route.fulfill({ json: { job: record, result: { owner: record.project_id } } });
    });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobHistoryCount: 3 }));
    const { bundle, outcome } = await exportProject(page);
    assert.equal(outcome.partial, false);
    assert.deepEqual(bundle.jobs.map((entry) => entry.job_id), [owned.job_id]);
    assert.deepEqual(bundle.results.map((entry) => entry.job_id), [owned.job_id]);
    assert.deepEqual(fetched, [owned.job_id], "foreign results must not even be requested");
  });
}, { timeout: 75_000 });

test("Workbench project export reports missing results and recovers on retry", async () => {
  await usingWorkbench(async (page) => {
    const record = job("retry-result", PROJECT_ID);
    runtime.state.adminJobs.push(record);
    let unavailable = true;
    await page.route(`**/api/v1/jobs/${record.job_id}`, async (route) => {
      await route.fulfill(unavailable
        ? { status: 503, json: { error: "qualification result unavailable" } }
        : { json: { job: record, result: { displacement: 0.00125 } } });
    });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobHistoryCount: 1 }));
    const first = await exportProject(page);
    assert.equal(first.outcome.partial, true, "a missing result must not be reported as a complete export");
    assert.equal(first.bundle.jobs.length, 1);
    assert.equal(first.bundle.results.length, 0);
    const partialMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
    unavailable = false;
    const retried = await exportProject(page);
    assert.equal(retried.outcome.partial, false);
    assert.equal(retried.bundle.results.length, 1);
    assert.equal(retried.bundle.results[0].result.displacement, 0.00125);
    assert.notEqual(await page.evaluate(() => window.__kyuubikiPwdt.state().message), partialMessage);
  });
}, { timeout: 75_000 });

test("Workbench Pwdt Store retry preserves project isolation and survives reload", async () => {
  await usingWorkbench(async (page) => {
    runtime.state.projects.push({ ...initialProject, project_id: "second-project", name: "Second project" });
    const entry = {
      id: "recovery-operator", kind: "operator", title: "Recovery operator", version: "1.0.0",
      source_id: "qualification", source_kind: "builtin", tags: ["qualification"],
      install: { mode: "workspace", requires_download: false, target: "operators/recovery" },
    };
    let unavailable = true;
    await page.route("**/api/v1/store**", async (route) => {
      const detail = new URL(route.request().url()).pathname.endsWith(`/${entry.id}`);
      await route.fulfill(detail && unavailable
        ? { status: 503, json: { error: "qualification store unavailable" } }
        : { json: detail ? { entry } : {
          entries: [entry], sources: [],
          summary: { entry_count: 1, kinds: { operator: 1 }, sources: { qualification: 1 } },
        } });
    });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
    await page.locator('[data-workbench-store-entry-id="recovery-operator"]').waitFor({ state: "visible" });
    await assert.rejects(invoke(page, "store/stageEntry", { kind: "operator", entryId: entry.id }), /unavailable|503/u);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().storeManifestEntryCount), 0);
    unavailable = false;
    await invoke(page, "store/stageEntry", { kind: "operator", entryId: entry.id });
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 1 }));
    await invoke(page, "project/select", { projectId: "second-project" });
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({
      selectedProjectId: "second-project", storeManifestEntryCount: 0,
    }));
    assert.equal(await page.locator('[data-workbench-store-entry-action="stage"]').isEnabled(), true);
    await invoke(page, "project/select", { projectId: PROJECT_ID });
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 1 }));
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({
      selectedProjectId: "qualification-project", storeManifestEntryCount: 1, storeManifestReadable: true,
    }));
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
    const stage = page.locator('[data-workbench-store-entry-action="stage"]');
    await stage.waitFor({ state: "visible" });
    assert.equal(await stage.isDisabled(), true);
  });
}, { timeout: 90_000 });

test("Workbench Pwdt saves a model then appends a version without duplicating the model", async () => {
  await usingWorkbench(async (page) => {
    const versions = [];
    const writes = [];
    let model;
    function addVersion(body) {
      const version = {
        ...body, model_id: model.model_id, project_id: PROJECT_ID,
        version_id: `recovery-version-${versions.length + 1}`, version_number: versions.length + 1,
        inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
      };
      versions.unshift(version);
      model.latest_version_id = version.version_id;
      model.latest_version_number = version.version_number;
      return version;
    }
    await page.route("**/api/v1/**", async (route) => {
      const request = route.request();
      const pathname = new URL(request.url()).pathname;
      const method = request.method();
      if (method === "POST" && pathname === `/api/v1/projects/${PROJECT_ID}/models`) {
        writes.push("create-model");
        model = {
          ...request.postDataJSON(), model_id: "recovery-model", project_id: PROJECT_ID,
          inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
        };
        addVersion(request.postDataJSON());
        runtime.state.projects[0].models.push(model);
        await route.fulfill({ status: 201, json: { model } });
      } else if (pathname === "/api/v1/models/recovery-model") {
        if (method === "PATCH") {
          writes.push("update-model");
          Object.assign(model, request.postDataJSON());
        }
        await route.fulfill({ json: { model } });
      } else if (pathname === "/api/v1/models/recovery-model/versions") {
        if (method === "POST") {
          writes.push("create-version");
          await route.fulfill({ status: 201, json: { version: addVersion(request.postDataJSON()) } });
        } else await route.fulfill({ json: { versions } });
      } else await route.continue();
    });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, span: 9, height: 2 }));
    const created = await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "saved-v1", saveAs: true }));
    assert.equal(created.modelId, "recovery-model");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), created.modelId);
    const saved = await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "saved-v2" }));
    assert.equal(saved.versionId, "recovery-version-2");
    assert.deepEqual(writes, ["create-model", "update-model", "create-version"]);
    const { bundle, outcome } = await exportProject(page);
    assert.equal(outcome.partial, false);
    assert.equal(bundle.models.length, 1);
    assert.equal(bundle.models[0].name, "saved-v2");
    assert.deepEqual(bundle.model_versions.map((entry) => entry.name), ["saved-v2", "saved-v1"]);
    assert.equal(bundle.model_versions[0].payload.nodes.length, 7);
    assert.equal(bundle.active_version_id, saved.versionId);
  });
}, { timeout: 90_000 });

test("Workbench Pwdt macro opens 3D editing and retains view and selection across navigation", async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    const prepared = await page.evaluate(() => window.__kyuubikiPwdt.runMacro("macro/prepare3dEditing"));
    assert.equal(prepared.stepCount, 4);
    await invoke(page, "viewport/set3dView", { preset: "top", projection: "ortho" });
    await invoke(page, "selection/set3d", { nodeIndices: [0, 1], anchorNodeIndex: 0 });
    const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(snapshot.studyKind, "truss_3d");
    assert.equal(snapshot.sidebarSection, "model");
    assert.equal(snapshot.truss3dViewPreset, "top");
    assert.equal(snapshot.truss3dProjectionMode, "ortho");
    assert.deepEqual(snapshot.selectedTruss3dNodeIndices, [0, 1]);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
    await page.evaluate(() => window.__kyuubikiPwdt.runMacro("macro/focusCurrent3dSelection"));
    const returned = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(returned.sidebarSection, "model");
    assert.equal(returned.truss3dViewPreset, "top");
    assert.deepEqual(returned.selectedTruss3dNodeIndices, [0, 1]);
  });
}, { timeout: 90_000 });
