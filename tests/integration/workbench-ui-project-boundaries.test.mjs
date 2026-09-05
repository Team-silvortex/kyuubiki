import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, before, test } from "node:test";

import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

const PROJECT_ID = "qualification-project";
let runtime;
let browser;
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
  runtime.state.projectMutations.length = 0;
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  try {
    const library = await mockModelLibrary(page);
    await run(page, library);
    assert.deepEqual(errors, []);
  } catch (error) {
    const snapshot = await page.evaluate(() => window.__kyuubikiPwdt?.state()).catch(() => null);
    throw new Error(`${error.message}\nstate=${JSON.stringify(snapshot)}\nerrors=${JSON.stringify(errors)}`, { cause: error });
  } finally {
    await context.close();
  }
}

function invoke(page, action, payload = {}) {
  return page.evaluate(({ action, payload }) => window.__kyuubikiPwdt.invoke(action, payload), { action, payload });
}

async function openWorkbench(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
  await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: "qualification-project" }));
}

async function exportProject(page) {
  page.once("dialog", (dialog) => dialog.accept());
  const [download, outcome] = await Promise.all([
    page.waitForEvent("download", { timeout: 20_000 }),
    invoke(page, "project/exportJson"),
  ]);
  assert.equal(outcome.partial, false);
  return JSON.parse(await readFile(await download.path(), "utf8"));
}

async function importProject(page, bundle) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "projects" }));
  await page.locator('[data-workbench-library-project-page="exchange"]').click();
  const previousMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
  await page.locator('[data-workbench-library-project-action="import"]').setInputFiles({
    name: "roundtrip.kyuubiki.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(bundle)),
  });
  assert.equal(await page.locator('[data-workbench-library-project-action="import"]').inputValue(), "", "the same file must remain selectable for retry");
  await page.waitForFunction((previous) => window.__kyuubikiPwdt.state().message !== previous, previousMessage, { timeout: 15_000 });
}

async function mockModelLibrary(page) {
  const library = { models: [], versions: [], writes: [], failVersions: false };
  function addVersion(model, input) {
    const version = {
      ...input, project_id: model.project_id, model_id: model.model_id,
      version_id: `boundary-version-${library.versions.length + 1}`,
      version_number: library.versions.filter((entry) => entry.model_id === model.model_id).length + 1,
      inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
    };
    library.versions.push(version);
    model.latest_version_id = version.version_id;
    model.latest_version_number = version.version_number;
    return version;
  }
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;
    const method = request.method();
    const create = pathname.match(/^\/api\/v1\/projects\/([^/]+)\/models$/u);
    const modelPath = pathname.match(/^\/api\/v1\/models\/([^/]+)(\/versions)?$/u);
    if (create && method === "POST") {
      const project = runtime.state.projects.find((entry) => entry.project_id === create[1]);
      assert.ok(project);
      const model = {
        ...request.postDataJSON(), model_id: `boundary-model-${library.models.length + 1}`,
        project_id: project.project_id, inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
      };
      library.writes.push({ method, pathname });
      addVersion(model, request.postDataJSON());
      library.models.push(model);
      project.models.push(model);
      await route.fulfill({ status: 201, json: { model } });
    } else if (modelPath) {
      const model = library.models.find((entry) => entry.model_id === modelPath[1]);
      assert.ok(model);
      if (modelPath[2]) {
        if (method === "POST") {
          if (library.failVersions) {
            await route.fulfill({ status: 503, json: { error: "qualification version service unavailable" } });
          } else {
            library.writes.push({ method, pathname });
            await route.fulfill({ status: 201, json: { version: addVersion(model, request.postDataJSON()) } });
          }
        } else await route.fulfill({ json: {
          versions: library.versions.filter((entry) => entry.model_id === model.model_id).toReversed(),
        } });
      } else {
        if (method === "PATCH") {
          library.writes.push({ method, pathname });
          Object.assign(model, request.postDataJSON());
        }
        await route.fulfill({ json: { model } });
      }
    } else await route.fallback();
  });
  return library;
}

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
