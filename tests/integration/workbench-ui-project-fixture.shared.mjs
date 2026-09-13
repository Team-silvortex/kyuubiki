import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, before } from "node:test";

import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

export const PROJECT_ID = "qualification-project";
export let runtime;
let browser;
export let initialProject;

export function installProjectWorkbenchTestHooks() {
  before(async () => {
    runtime = await startIsolatedWorkbenchUiRuntime();
    initialProject = structuredClone(runtime.state.projects[0]);
    browser = await launchIntegrationBrowser(chromium);
  }, { timeout: 180_000 });

  after(async () => {
    try { await browser?.close(); } finally { await runtime?.stop(); }
  }, { timeout: 90_000 });
}

export async function usingWorkbench(run) {
  runtime.state.projects.splice(0, runtime.state.projects.length, structuredClone(initialProject));
  runtime.state.projectMutations.length = 0;
  runtime.state.adminJobs.length = 0;
  runtime.state.adminResults.length = 0;
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.stack ?? error.message));
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

export function invoke(page, action, payload = {}) {
  return page.evaluate(({ action, payload }) => window.__kyuubikiPwdt.invoke(action, payload), { action, payload });
}

export async function openWorkbench(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
  await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: "qualification-project" }));
}

export async function exportProject(page) {
  page.once("dialog", (dialog) => dialog.accept());
  const [download, outcome] = await Promise.all([
    page.waitForEvent("download", { timeout: 20_000 }),
    invoke(page, "project/exportJson"),
  ]);
  assert.equal(outcome.partial, false);
  return JSON.parse(await readFile(await download.path(), "utf8"));
}

export async function importProject(page, bundle) {
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
  const library = { models: [], versions: [], writes: [], requests: [], failVersions: false, failVersionReads: false,
    loseCreateResponseOnce: false, loseVersionResponseOnce: false };
  const checkpointReceipts = new Map();
  let modelSequence = 0;
  let versionSequence = 0;
  function addVersion(model, input) {
    input = { ...input };
    delete input.request_id;
    const version = {
      ...input, project_id: model.project_id, model_id: model.model_id,
      version_id: `boundary-version-${++versionSequence}`,
      version_number: library.versions.filter((entry) => entry.model_id === model.model_id).length + 1,
      inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
    };
    library.versions.push(version);
    // The real checkpoint endpoint commits model metadata/payload and version together.
    Object.assign(model, input);
    model.latest_version_id = version.version_id;
    model.latest_version_number = version.version_number;
    return version;
  }
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;
    const method = request.method();
    const lookup = pathname.match(/^\/api\/v1\/checkpoints\/(model|version)\/([^/]+)\/([^/]+)$/u);
    if (lookup && method === "GET") {
      const [, operation, parent, id] = lookup.map(decodeURIComponent);
      const path = operation === "model" ? `/api/v1/projects/${parent}/models` : `/api/v1/models/${parent}/versions`;
      const stored = checkpointReceipts.get(`${path}:${id}`);
      const saved = stored?.body.model ?? stored?.body.version;
      const versionId = saved?.latest_version_id ?? saved?.version_id;
      const exists = library.models.some((entry) => entry.model_id === saved?.model_id)
        && library.versions.some((entry) => entry.version_id === versionId);
      return route.fulfill({ json: { checkpoint: saved ? {
        status: exists ? "committed" : "deleted", project_id: saved.project_id,
        model_id: saved.model_id, version_id: versionId,
      } : { status: "unknown" } } });
    }
    const create = pathname.match(/^\/api\/v1\/projects\/([^/]+)\/models$/u);
    const modelPath = pathname.match(/^\/api\/v1\/models\/([^/]+)(\/versions)?$/u);
    const versionPath = pathname.match(/^\/api\/v1\/model-versions\/([^/]+)$/u);
    const checkpoint = method === "POST" && (create || modelPath?.[2]);
    const input = checkpoint ? request.postDataJSON() : null;
    const requestKey = input?.request_id ? `${pathname}:${input.request_id}` : null;
    if (checkpoint) library.requests.push({ pathname, requestId: input.request_id });
    const receipt = requestKey ? checkpointReceipts.get(requestKey) : null;
    if (receipt) {
      const existing = receipt.body.model ?? receipt.body.version;
      const versionId = existing.latest_version_id ?? existing.version_id;
      if (receipt.input !== JSON.stringify(input)) {
        return route.fulfill({ status: 409, json: { error: "checkpoint_request_conflict" } });
      }
      if (!library.models.some((model) => model.model_id === existing.model_id) ||
          !library.versions.some((version) => version.version_id === versionId)) {
        return route.fulfill({ status: 409, json: { error: "checkpoint_result_deleted" } });
      }
      return route.fulfill({ status: 201, json: receipt.body });
    }
    async function publishCheckpoint(body, lossFlag) {
      if (requestKey) checkpointReceipts.set(requestKey, { input: JSON.stringify(input), body: structuredClone(body) });
      if (library[lossFlag]) {
        library[lossFlag] = false;
        return route.abort("failed");
      }
      return route.fulfill({ status: 201, json: body });
    }
    if (versionPath) {
      const version = library.versions.find((entry) => entry.version_id === versionPath[1]);
      assert.ok(version);
      if (method === "PATCH") {
        library.writes.push({ method, pathname });
        Object.assign(version, request.postDataJSON());
      } else if (method === "DELETE") {
        library.writes.push({ method, pathname });
        library.versions.splice(library.versions.indexOf(version), 1);
        const model = library.models.find((entry) => entry.model_id === version.model_id);
        const latest = library.versions.filter((entry) => entry.model_id === version.model_id).at(-1);
        model.latest_version_id = latest?.version_id ?? null;
        model.latest_version_number = latest?.version_number ?? null;
      }
      await route.fulfill({ json: { version } });
    } else if (create && method === "POST") {
      const project = runtime.state.projects.find((entry) => entry.project_id === create[1]);
      assert.ok(project);
      const model = {
        ...request.postDataJSON(), model_id: `boundary-model-${++modelSequence}`,
        project_id: project.project_id, inserted_at: initialProject.inserted_at, updated_at: initialProject.updated_at,
      };
      delete model.request_id;
      library.writes.push({ method, pathname });
      addVersion(model, request.postDataJSON());
      library.models.push(model);
      project.models.push(model);
      await publishCheckpoint({ model }, "loseCreateResponseOnce");
    } else if (modelPath) {
      const model = library.models.find((entry) => entry.model_id === modelPath[1]);
      assert.ok(model);
      if (modelPath[2]) {
        if (method === "POST") {
          if (library.failVersions) {
            await route.fulfill({ status: 503, json: { error: "qualification version service unavailable" } });
          } else {
            library.writes.push({ method, pathname });
            await publishCheckpoint({ version: addVersion(model, request.postDataJSON()) }, "loseVersionResponseOnce");
          }
        } else if (library.failVersionReads) {
          await route.fulfill({ status: 503, json: { error: "qualification version read unavailable" } });
        } else await route.fulfill({ json: {
          versions: library.versions.filter((entry) => entry.model_id === model.model_id).toReversed(),
        } });
      } else {
        if (method === "PATCH") {
          library.writes.push({ method, pathname });
          Object.assign(model, request.postDataJSON());
        } else if (method === "DELETE") {
          library.writes.push({ method, pathname });
          library.models.splice(library.models.indexOf(model), 1);
          const project = runtime.state.projects.find((entry) => entry.project_id === model.project_id);
          project.models.splice(project.models.indexOf(model), 1);
          library.versions = library.versions.filter((entry) => entry.model_id !== model.model_id);
        }
        await route.fulfill({ json: { model } });
      }
    } else await route.fallback();
  });
  return library;
}

export async function holdRequest(page, pathname, method = "GET", respond = (route) => route.fallback()) {
  let release;
  let reached;
  const gate = new Promise((resolve) => { release = resolve; });
  let timer;
  const received = new Promise((resolve, reject) => {
    reached = () => { clearTimeout(timer); resolve(); };
    timer = setTimeout(() => reject(new Error(`Expected request did not arrive: ${method} ${pathname}`)), 10_000);
    timer.unref();
  });
  void received.catch(() => {});
  await page.route(`**${pathname}`, async (route) => {
    if (route.request().method() !== method) return route.fallback();
    reached();
    await gate;
    await respond(route);
  });
  return { received, release: () => { clearTimeout(timer); release(); } };
}

export async function openSavedModels(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "models" }));
  await page.locator('[data-workbench-library-model-page="saved"]').click();
}

export async function openProjectManager(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "projects" }));
  await page.locator('[data-workbench-library-project-page="manage"]').click();
}

export async function waitForGuiTransition(page, pending = false) {
  await page.waitForFunction((expected) =>
    document.querySelector('[data-workbench-panel="inspector"] > .panel-head > span')?.textContent === expected,
  pending ? "busy" : "ready", { timeout: 15_000 });
}
