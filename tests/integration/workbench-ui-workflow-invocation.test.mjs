import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { readFile } from "node:fs/promises";

import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import {
  chromium,
  startIsolatedWorkbenchUiRuntime,
  workbenchUrl,
} from "./workbench-ui-isolated.shared.mjs";

let browser;
let runtime;

before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  browser = await launchIntegrationBrowser(chromium);
}, { timeout: 180_000 });

after(async () => {
  await browser?.close();
  await runtime?.stop();
}, { timeout: 90_000 });

async function click(page, selector, label) {
  const candidates = page.locator(selector);
  const target = candidates.first();
  await waitForVisibleOrPageError(page, target, label);
  assert.equal(await candidates.count(), 1, `${label} should resolve to one visible control`);
  await target.click({ timeout: 15_000 });
  return target;
}

async function waitForVisibleOrPageError(page, locator, label, timeout = 30_000) {
  let rejectPageError;
  const pageError = new Promise((_, reject) => {
    rejectPageError = (error) => reject(new Error(`${label} aborted after client error: ${error.message}`));
    page.once("pageerror", rejectPageError);
  });
  try {
    await Promise.race([
      locator.waitFor({ state: "visible", timeout }),
      pageError,
    ]);
  } finally {
    page.off("pageerror", rejectPageError);
  }
}

async function openQualificationBuilder(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await click(page, '[aria-label="workbench-rail:workflow"]', "Workflow rail");
  await page.locator('[data-workbench-workflow-surface]').waitFor({ state: "visible", timeout: 30_000 });
  await click(page, '[data-workflow-surface-tab="catalog"]', "Workflow catalog tab");
  const catalogSearch = page.locator('[data-workflow-catalog-search="query"]');
  await catalogSearch.waitFor({ state: "visible", timeout: 30_000 });
  await catalogSearch.fill("mechanical");
  const catalogCard = page.locator('[data-workflow-catalog-id="workflow.bar-1d-summary-json"]');
  await catalogCard.waitFor({ state: "visible", timeout: 30_000 });
  await catalogCard.locator('[data-workflow-catalog-action="open-builder"]').click();
  const builder = page.locator('[data-workflow-builder-shell="builder"]');
  await builder.waitFor({ state: "visible", timeout: 30_000 });
  return builder;
}

test("Workbench isolated workflow UI completes catalog, builder, draft, submit, poll, and result rendering", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  try {
    const builder = await openQualificationBuilder(page);
    const nodesBefore = await builder.locator('[data-workflow-node-id]').count();
    await builder.locator('[data-workflow-topology-kind="select"]').selectOption("solve");
    const operatorSearch = builder.locator('[data-workflow-operator-search="query"]');
    await operatorSearch.fill("mechanical");
    const quickInsert = builder.locator('[data-workflow-operator-action="quick-insert"][data-workflow-operator-id="solve.bar_1d"]').first();
    await quickInsert.waitFor({ state: "visible", timeout: 30_000 });
    await quickInsert.click();
    await page.waitForFunction(
      (previousCount) => document.querySelectorAll("[data-workflow-node-id]").length > previousCount,
      nodesBefore,
      { timeout: 30_000 },
    );

    await click(page, '[data-workflow-builder-action="save-draft"]', "Save workflow draft");
    await builder.locator('[data-workflow-import-message="text"]').waitFor({ state: "visible", timeout: 15_000 });
    await click(page, '[data-workflow-builder-action="run-catalog"]', "Run catalog workflow");

    const completedRun = page.locator(
      '[data-workflow-run-id="qualification-workflow-job"][data-workflow-run-status="completed"]',
    );
    await completedRun.waitFor({ state: "visible", timeout: 45_000 });
    assert.match(await completedRun.innerText(), /100%/u);
    assert.match(await completedRun.innerText(), /max_displacement=1\.250e-3/u);
    assert.equal(runtime.state.catalogSubmissions, 1);
    assert.equal(runtime.state.graphSubmissions, 0);
    assert.ok(runtime.state.catalogFetches >= 1);
    assert.ok(runtime.state.operatorFetches >= 1);
    assert.ok(runtime.state.jobPolls >= 1);
    assert.ok(runtime.state.historyFetches >= 2);
    assert.equal(
      typeof runtime.state.submissionBodies[0]?.input_artifacts?.bar_1d_model,
      "object",
    );
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench isolated workflow UI blocks invalid draft input without backend submission", async () => {
  const context = await browser.newContext({ viewport: { width: 1180, height: 920 } });
  const page = await context.newPage();
  try {
    const builder = await openQualificationBuilder(page);
    const runDraft = builder.locator('[data-workflow-builder-action="run-draft"]');
    await runDraft.waitFor({ state: "visible", timeout: 30_000 });
    assert.equal(await runDraft.isEnabled(), true, "valid qualification draft should be runnable");
    const submissionsBefore = runtime.state.catalogSubmissions + runtime.state.graphSubmissions;
    await builder.locator('[data-workflow-input-artifact="bar_1d_model"]').fill('{"nodes":');
    await page.waitForFunction(
      () => document.querySelector('[data-workflow-builder-action="run-draft"]')?.hasAttribute("disabled"),
      undefined,
      { timeout: 15_000 },
    );
    assert.equal(await runDraft.isDisabled(), true);
    assert.match(await builder.innerText(), /input JSON is missing or invalid|输入 JSON 缺失或无效/u);
    await page.waitForTimeout(250);
    assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, submissionsBefore);
  } finally {
    await context.close();
  }
}, { timeout: 75_000 });

test("Workbench startup preserves hydrated settings and language packs", async () => {
  const context = await browser.newContext({ viewport: { width: 1180, height: 920 } });
  const page = await context.newPage();
  const settingsKey = "kyuubiki-workbench-settings";
  const languagePacksKey = "kyuubiki-workbench-language-packs";
  const seededPack = {
    schema_version: "kyuubiki.language-pack/v1",
    id: "qualification-fr-pack",
    language: "fr",
    targetSurface: "workbench",
    name: "Qualification French pack",
    version: "2.17.0",
    source: "imported",
    updatedAt: "2026-08-28T00:00:00.000Z",
    overrides: { workflowCatalogTitle: "Catalogue de qualification" },
  };

  await context.addInitScript(({ settingsKey, languagePacksKey, seededPack }) => {
    window.localStorage.setItem(settingsKey, JSON.stringify({
      theme: "marine",
      language: "fr",
      showShortcutHints: false,
      immersiveGuardrails: true,
      frontendRuntimeMode: "orchestrated_gui",
      directMeshEndpointsText: "solver-a:5001",
      directMeshSelectionMode: "healthiest",
      assistantMode: "local",
      assistantApiBaseUrl: "https://assistant.example.test",
      assistantModel: "qualification-model",
    }));
    window.localStorage.setItem(languagePacksKey, JSON.stringify([seededPack]));
  }, { settingsKey, languagePacksKey, seededPack });

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.locator('[data-workbench-shell="root"]').waitFor({ state: "visible", timeout: 30_000 });
    await page.waitForFunction(() => document.documentElement.dataset.theme === "marine");
    await page.waitForTimeout(500);

    const hydrated = await page.evaluate(({ settingsKey, languagePacksKey }) => ({
      settings: JSON.parse(window.localStorage.getItem(settingsKey) || "{}"),
      packs: JSON.parse(window.localStorage.getItem(languagePacksKey) || "[]"),
    }), { settingsKey, languagePacksKey });
    assert.equal(hydrated.settings.theme, "marine");
    assert.equal(hydrated.settings.language, "fr");
    assert.equal(hydrated.settings.showShortcutHints, false);
    assert.equal(hydrated.packs.length, 1);
    assert.equal(hydrated.packs[0]?.id, seededPack.id);

    await page.reload({ waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => document.documentElement.dataset.theme === "marine");
    const reloaded = await page.evaluate(({ settingsKey, languagePacksKey }) => ({
      settings: JSON.parse(window.localStorage.getItem(settingsKey) || "{}"),
      packs: JSON.parse(window.localStorage.getItem(languagePacksKey) || "[]"),
    }), { settingsKey, languagePacksKey });
    assert.equal(reloaded.settings.language, "fr");
    assert.equal(reloaded.packs[0]?.id, seededPack.id);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench startup does not overwrite a corrupt language pack store", async () => {
  const context = await browser.newContext({ viewport: { width: 1180, height: 920 } });
  const page = await context.newPage();
  const languagePacksKey = "kyuubiki-workbench-language-packs";
  await context.addInitScript((key) => {
    window.localStorage.setItem(key, "");
  }, languagePacksKey);

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.locator('[data-workbench-shell="root"]').waitFor({ state: "visible", timeout: 30_000 });
    await page.waitForTimeout(500);
    assert.equal(await page.evaluate((key) => window.localStorage.getItem(key), languagePacksKey), "");
  } finally {
    await context.close();
  }
}, { timeout: 75_000 });

test("Workbench rail mounts every declared sidebar chunk without client errors", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const sections = ["model", "workflow", "store", "library", "system"];

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    for (const section of sections) {
      await click(page, `[aria-label="workbench-rail:${section}"]`, `${section} rail`);
      await waitForVisibleOrPageError(
        page,
        page.locator(`[data-workbench-sidebar-section="${section}"]`),
        `${section} sidebar section`,
      );
      const chunk = page.locator(`[data-workbench-ui-chunk="section.${section}"]`);
      await waitForVisibleOrPageError(page, chunk, `${section} sidebar chunk`);
      assert.equal(await chunk.getAttribute("data-workbench-ui-chunk-phase"), "load");
    }
  } finally {
    await context.close();
  }
}, { timeout: 120_000 });

test("Workbench Model navigation and Pwdt study alias preserve the complete workspace path", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await click(page, '[aria-label="workbench-rail:model"]', "Model rail");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-model="panel"]'),
      "Model panel",
    );

    for (const pageName of ["overview", "study", "studio", "generate"]) {
      const selector = `[data-workbench-model-tools-page="${pageName}"]`;
      await click(
        page,
        selector,
        `Model ${pageName} page`,
      );
      await page.locator(`${selector}.panel-tab--active`).waitFor({ state: "visible", timeout: 15_000 });
    }

    await click(page, '[data-workbench-model-tools-page="study"]', "Model study return");
    const studyKind = page.locator('[data-workbench-model-study-kind="select"]');
    await studyKind.selectOption("truss_2d");
    await page.waitForFunction(
      () => document.querySelector('[data-workbench-model-study-kind="select"]')?.value === "truss_2d",
      undefined,
      { timeout: 15_000 },
    );
    await click(page, '[data-workbench-model-tools-page="materials"]', "Model materials page");
    await page.locator('[data-workbench-model-tools-page="materials"].panel-tab--active').waitFor({
      state: "visible",
      timeout: 15_000,
    });

    await click(page, '[data-workbench-model-tab="tree"]', "Model tree tab");
    await page.locator('[data-workbench-model-tab="tree"].panel-tab--active').waitFor({
      state: "visible",
      timeout: 15_000,
    });
    await click(page, '[data-workbench-model-tab="tools"]', "Model tools tab");
    await click(page, '[data-workbench-model-tools-page="studio"]', "Model studio detour");

    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    const aliasResult = await page.evaluate(() =>
      window.__kyuubikiPwdt.invoke("nav/setSidebarSection", { section: "study" }),
    );
    assert.equal(aliasResult.ok, true);
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-model-study="panel"]'),
      "Workspace study panel",
    );
    await page.locator('[data-workbench-model-tools-page="study"].panel-tab--active').waitFor({
      state: "visible",
      timeout: 15_000,
    });
    assert.equal(await page.locator('[data-workbench-sidebar-section="model"]').count(), 1);
    assert.equal(await page.locator('[data-workbench-model-study-kind="select"]').count(), 1);
    assert.equal(await page.locator('[data-workbench-model-study-run="true"]').count(), 1);
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench System navigation preserves controlled state across deep page round trips", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await click(page, '[aria-label="workbench-rail:system"]', "System rail");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-system-sidebar="root"]'),
      "System sidebar",
    );

    const overview = page.locator('[data-workbench-system-settings-page="overview"]');
    assert.match(await overview.getAttribute("class"), /panel-tab--active/u);

    await click(page, '[data-workbench-system-surface-tab="runtime"]', "Runtime surface");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-runtime="panel"]'),
      "Runtime panel",
    );
    for (const runtimeTab of ["overview", "control", "stack", "security", "agents", "audit", "watchdog"]) {
      const target = await click(
        page,
        `[data-workbench-runtime-tab="${runtimeTab}"]`,
        `Runtime ${runtimeTab} tab`,
      );
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }

    await click(page, '[data-workbench-system-surface-tab="settings"]', "Settings surface");
    assert.match(await overview.getAttribute("class"), /panel-tab--active/u);
    await click(page, '[aria-label="workbench-rail:model"]', "Model rail");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-sidebar-section="model"]'),
      "Model sidebar",
    );
    await click(page, '[aria-label="workbench-rail:system"]', "System rail return");
    await waitForVisibleOrPageError(page, overview, "Preserved System overview");
    assert.match(await overview.getAttribute("class"), /panel-tab--active/u);

    await click(page, '[data-workbench-system-settings-page="config"]', "System config page");
    assert.match(
      await page.locator('[data-workbench-system-settings-page="config"]').getAttribute("class"),
      /panel-tab--active/u,
    );
    await click(page, '[data-workbench-system-settings-page="scripts"]', "System scripts page");
    assert.match(
      await page.locator('[data-workbench-system-settings-page="scripts"]').getAttribute("class"),
      /panel-tab--active/u,
    );

    await click(page, '[data-workbench-system-surface-tab="data"]', "Data surface");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-data-admin="panel"]'),
      "Data admin panel",
    );
    for (const dataTab of ["jobs", "results"]) {
      const target = await click(page, `[data-workbench-data-tab="${dataTab}"]`, `Data ${dataTab} tab`);
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }
    for (const dataPage of ["overview", "browse", "edit"]) {
      const target = await click(page, `[data-workbench-data-page="${dataPage}"]`, `Data ${dataPage} page`);
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }

    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 120_000 });

test("Workbench Library navigation mounts every primary and secondary page without client errors", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await click(page, '[aria-label="workbench-rail:library"]', "Library rail");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-library="panel"]'),
      "Library panel",
    );

    for (const tab of ["jobs", "results", "models", "projects", "samples"]) {
      const target = await click(page, `[data-workbench-library-tab="${tab}"]`, `Library ${tab} tab`);
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }
    for (const pageName of ["catalog", "import"]) {
      const target = await click(
        page,
        `[data-workbench-library-sample-page="${pageName}"]`,
        `Library sample ${pageName} page`,
      );
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }

    await click(page, '[data-workbench-library-tab="projects"]', "Library projects return");
    for (const pageName of ["manage", "exchange"]) {
      const target = await click(
        page,
        `[data-workbench-library-project-page="${pageName}"]`,
        `Library project ${pageName} page`,
      );
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }

    await click(page, '[data-workbench-library-tab="models"]', "Library models return");
    for (const pageName of ["saved", "versions"]) {
      const target = await click(
        page,
        `[data-workbench-library-model-page="${pageName}"]`,
        `Library model ${pageName} page`,
      );
      assert.match(await target.getAttribute("class"), /panel-tab--active/u);
    }
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench Store ignores stale responses after the active kind changes", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.route("**/api/v1/store**", async (route) => {
    const url = new URL(route.request().url());
    const requestedKind = url.searchParams.get("kind") || "all";
    const entryKind = requestedKind === "all" ? "operator" : requestedKind;
    const delay = requestedKind === "operator" ? 700 : requestedKind === "workflow_template" ? 20 : 5;
    await new Promise((resolve) => setTimeout(resolve, delay));
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        entries: [{
          id: `qualification-${requestedKind}`,
          kind: entryKind,
          title: `Qualification ${requestedKind}`,
          version: "1",
          source_id: "qualification",
          source_kind: "builtin",
          tags: [],
          install: { mode: "workspace", requires_download: false },
        }],
        sources: [{
          id: "qualification",
          type: "builtin",
          label: "Qualification",
          enabled: true,
          editable: false,
          status: "ready",
          supports: ["operator", "workflow_template", "frontend_dsl_template"],
        }],
        summary: { entry_count: 1, kinds: { [entryKind]: 1 }, sources: { qualification: 1 } },
      }),
    });
  });

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await click(page, '[aria-label="workbench-rail:store"]', "Store rail");
    const storePanel = page.locator('[data-workbench-store-panel="true"]');
    await waitForVisibleOrPageError(page, storePanel, "Store panel");
    await page.locator('[data-workbench-store-entry-id="qualification-all"]').waitFor({
      state: "visible",
      timeout: 30_000,
    });

    const operatorRequest = page.waitForRequest((request) =>
      request.url().includes("/api/v1/store?kind=operator"),
    );
    await click(page, '[data-workbench-store-kind="operator"]', "Store operator kind");
    await operatorRequest;
    await click(page, '[data-workbench-store-kind="workflow_template"]', "Store workflow kind");
    const workflowEntry = page.locator(
      '[data-workbench-store-entry-id="qualification-workflow_template"]',
    );
    await workflowEntry.waitFor({ state: "visible", timeout: 30_000 });
    await page.waitForTimeout(900);

    assert.match(
      await page.locator('[data-workbench-store-kind="workflow_template"]').getAttribute("class"),
      /active/u,
    );
    assert.equal(await workflowEntry.count(), 1);
    assert.equal(await page.locator('[data-workbench-store-entry-id="qualification-operator"]').count(), 0);
    assert.equal(await storePanel.getAttribute("data-workbench-store-status"), "ready");
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench Pwdt stages, exports, and removes Store assets through shared commands", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  const entry = {
    id: "qualification-operator",
    kind: "operator",
    title: "Qualification operator",
    version: "2.17.0",
    source_id: "qualification",
    source_kind: "builtin",
    tags: ["qualification"],
    install: { mode: "workspace", requires_download: false, target: "operators/qualification" },
  };
  await page.route("**/api/v1/store**", async (route) => {
    const url = new URL(route.request().url());
    const detailPath = "/api/v1/store/operator/qualification-operator";
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(url.pathname === detailPath
        ? { entry }
        : {
            entries: [entry],
            sources: [{
              id: "qualification",
              type: "builtin",
              label: "Qualification",
              enabled: true,
              editable: false,
              status: "ready",
              supports: ["operator"],
            }],
            summary: { entry_count: 1, kinds: { operator: 1 }, sources: { qualification: 1 } },
          }),
    });
  });

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt), undefined, { timeout: 30_000 });
    const projectId = await page.evaluate(async () => {
      const snapshot = await window.__kyuubikiPwdt.waitUntil(
        (state) => Boolean(state.selectedProjectId),
        { timeoutMs: 15_000 },
      );
      return snapshot.selectedProjectId;
    });
    assert.equal(projectId, "qualification-project");

    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-store-entry-id="qualification-operator"]'),
      "Store qualification entry",
    );

    const staged = await page.evaluate(() =>
      window.__kyuubikiPwdt.stageStoreEntry("operator", "qualification-operator"));
    assert.equal(staged.manifestEntryCount, 1);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState(
      { storeManifestEntryCount: 1, storeManifestReadable: true },
      { timeoutMs: 15_000 },
    ));
    const manifestEntry = page.locator('[data-workbench-store-manifest-entry-id="qualification-operator"]');
    await manifestEntry.waitFor({ state: "visible", timeout: 15_000 });
    assert.equal(await page.locator('[data-workbench-store-panel="true"]').getAttribute("data-workbench-store-manifest-count"), "1");
    assert.equal(await page.locator('[data-workbench-store-entry-action="stage"]').isDisabled(), true);

    const downloadPromise = page.waitForEvent("download");
    const exportedPromise = page.evaluate(() => window.__kyuubikiPwdt.exportStoreManifest());
    const [download, exported] = await Promise.all([downloadPromise, exportedPromise]);
    assert.equal(download.suggestedFilename(), "qualification-project.store-manifest.json");
    assert.equal(exported.manifestEntryCount, 1);
    const downloadPath = await download.path();
    assert.ok(downloadPath);
    const exportedManifest = JSON.parse(await readFile(downloadPath, "utf8"));
    assert.equal(exportedManifest.entries[0].id, "qualification-operator");

    page.once("dialog", (dialog) => dialog.accept());
    const projectDownloadPromise = page.waitForEvent("download");
    const projectExportPromise = page.evaluate(() =>
      window.__kyuubikiPwdt.invoke("project/exportJson"));
    const [projectDownload, projectExported] = await Promise.all([
      projectDownloadPromise,
      projectExportPromise,
    ]);
    assert.equal(projectDownload.suggestedFilename(), "Qualification project.kyuubiki.json");
    assert.equal(projectExported.ok, true);
    const projectDownloadPath = await projectDownload.path();
    assert.ok(projectDownloadPath);
    const projectBundle = JSON.parse(await readFile(projectDownloadPath, "utf8"));
    assert.equal(projectBundle.store_manifest.project_id, "qualification-project");
    assert.equal(projectBundle.store_manifest.entries[0].id, "qualification-operator");

    page.once("dialog", (dialog) => dialog.accept());
    const archiveDownloadPromise = page.waitForEvent("download");
    const archiveExportPromise = page.evaluate(() =>
      window.__kyuubikiPwdt.invoke("project/exportZip"));
    const [archiveDownload, archiveExported] = await Promise.all([
      archiveDownloadPromise,
      archiveExportPromise,
    ]);
    assert.equal(archiveDownload.suggestedFilename(), "Qualification project.kyuubiki");
    assert.equal(archiveExported.ok, true);
    const archiveDownloadPath = await archiveDownload.path();
    assert.ok(archiveDownloadPath);
    const archiveBytes = await readFile(archiveDownloadPath);
    assert.equal(archiveBytes.subarray(0, 2).toString("ascii"), "PK");

    page.once("dialog", (dialog) => dialog.accept());
    const databaseExportFailure = await page.evaluate(async () => {
      try {
        await window.__kyuubikiPwdt.invoke("data/exportDatabase");
        return null;
      } catch (error) {
        return error instanceof Error ? error.message : String(error);
      }
    });
    assert.match(databaseExportFailure ?? "", /404|unhandled qualification route/u);

    page.once("dialog", (dialog) => dialog.accept());
    const removed = await page.evaluate(() =>
      window.__kyuubikiPwdt.removeStoreEntry("operator", "qualification-operator"));
    assert.equal(removed.manifestEntryCount, 0);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState(
      { storeManifestEntryCount: 0 },
      { timeoutMs: 15_000 },
    ));
    await manifestEntry.waitFor({ state: "detached", timeout: 15_000 });
    assert.equal(await page.locator('[data-workbench-store-entry-action="stage"]').isEnabled(), true);
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench Pwdt opens Store and round-trips every Workflow surface without DOM clicks", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt), undefined, { timeout: 30_000 });

    const storeResult = await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
    assert.equal(storeResult.section, "store");
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState(
      { sidebarSection: "store" },
      { timeoutMs: 15_000 },
    ));
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-store-panel="true"]'),
      "Pwdt Store panel",
    );

    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("workflow"));
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState(
      { sidebarSection: "workflow" },
      { timeoutMs: 15_000 },
    ));
    for (const workflowPanelTab of ["overview", "catalog", "builder", "runs"]) {
      await page.evaluate((tab) => window.__kyuubikiPwdt.openTabs({ workflowPanelTab: tab }), workflowPanelTab);
      await page.evaluate((tab) => window.__kyuubikiPwdt.waitForState(
        { workflowPanelTab: tab },
        { timeoutMs: 15_000 },
      ), workflowPanelTab);
      const activeTab = page.locator(`[data-workflow-surface-tab="${workflowPanelTab}"]`);
      await waitForVisibleOrPageError(page, activeTab, `Pwdt Workflow ${workflowPanelTab} tab`);
      assert.match(await activeTab.getAttribute("class"), /panel-tab--active/u);
    }

    const snapshot = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(snapshot.sidebarSection, "workflow");
    assert.equal(snapshot.workflowPanelTab, "runs");
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench Pwdt preserves hydrated script and DSL sessions", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await context.newPage();
  const scriptKey = "kyuubiki-workbench-python-panel";
  const dslKey = "kyuubiki-workbench-dsl-panel";
  const scriptCode = "print('hydrated-pwdt-script')";
  const dslCode = JSON.stringify({
    schema_version: "kyuubiki.frontend-dsl/v1",
    name: "hydrated-pwdt-dsl",
    steps: [],
  }, null, 2);
  await context.addInitScript(({ scriptKey, dslKey, scriptCode, dslCode }) => {
    window.sessionStorage.setItem(scriptKey, JSON.stringify({ code: scriptCode }));
    window.sessionStorage.setItem(dslKey, JSON.stringify({ code: dslCode }));
  }, { scriptKey, dslKey, scriptCode, dslCode });

  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await click(page, '[aria-label="workbench-rail:system"]', "System rail");
    await waitForVisibleOrPageError(
      page,
      page.locator('[data-workbench-sidebar-section="system"]'),
      "System sidebar section",
    );
    await click(
      page,
      '[data-workbench-system-surface-tab="settings"]',
      "System settings surface",
    );
    await click(
      page,
      '[data-workbench-system-settings-page="scripts"]',
      "System scripts page",
    );
    await page.waitForFunction(
      ({ scriptCode, dslCode }) => {
        const values = [...document.querySelectorAll("textarea.script-panel__editor")]
          .map((element) => element.value);
        return values.includes(scriptCode) && values.includes(dslCode);
      },
      { scriptCode, dslCode },
      { timeout: 30_000 },
    );
    await page.waitForTimeout(500);

    const persisted = await page.evaluate(({ scriptKey, dslKey }) => ({
      script: JSON.parse(window.sessionStorage.getItem(scriptKey) || "{}")?.code,
      dsl: JSON.parse(window.sessionStorage.getItem(dslKey) || "{}")?.code,
    }), { scriptKey, dslKey });
    assert.equal(persisted.script, scriptCode);
    assert.equal(persisted.dsl, dslCode);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });
