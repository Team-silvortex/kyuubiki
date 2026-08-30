import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { after, before, test } from "node:test";

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
  const target = page.locator(selector);
  await target.waitFor({ state: "visible", timeout: 30_000 });
  assert.equal(await target.count(), 1, `${label} should resolve to one visible control`);
  await target.click({ timeout: 15_000 });
}

async function openWorkbench(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt), undefined, { timeout: 30_000 });
}

test("Workbench System exports, imports, inspects, and resets a topology snapshot", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await openWorkbench(page);
    await click(page, '[aria-label="workbench-rail:system"]', "System rail");
    await click(page, '[data-workbench-system-surface-tab="runtime"]', "Runtime surface");
    await click(page, '[data-workbench-runtime-tab="control"]', "Control window");

    const controlWindow = page.locator('[data-workbench-control-window="root"]');
    await controlWindow.waitFor({ state: "visible" });
    assert.equal(await controlWindow.getAttribute("data-workbench-control-source"), "derived_frontend");

    for (const mode of ["direct", "mesh", "orchestrated"]) {
      const modeTab = controlWindow.locator(`[data-workbench-control-mode-tab="${mode}"]`);
      await modeTab.click();
      assert.match(await modeTab.getAttribute("class"), /panel-tab--active/u);
    }

    const downloadPromise = page.waitForEvent("download");
    await click(page, '[data-workbench-control-action="export-snapshot"]', "Export topology snapshot");
    const download = await downloadPromise;
    assert.match(download.suggestedFilename(), /^mesh-topology-.+\.json$/u);
    const downloadPath = await download.path();
    assert.ok(downloadPath);
    const exportedSnapshot = JSON.parse(await readFile(downloadPath, "utf8"));
    assert.deepEqual(exportedSnapshot.schema, {
      name: "kyuubiki.mesh-topology-snapshot",
      version: 1,
    });

    const importedAt = "2031-05-17T09:30:00.000Z";
    const importedSnapshot = {
      ...exportedSnapshot,
      observed_at: importedAt,
      control_mode: "mesh",
      entry_agent_id: "qualification-imported-agent",
      peer_count: 7,
      graph_summary: "qualification imported mesh graph",
    };
    await controlWindow.locator('[data-workbench-control-action="import-snapshot"]').setInputFiles({
      name: "qualification-topology.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify(importedSnapshot)),
    });
    await page.waitForFunction(() =>
      document.querySelector('[data-workbench-control-window="root"]')?.getAttribute("data-workbench-control-source") === "imported_snapshot",
    );
    assert.match(
      await controlWindow.locator('[data-workbench-control-window="snapshot-meta"]').innerText(),
      new RegExp(importedAt.replaceAll(".", "\\."), "u"),
    );
    await controlWindow.locator('[data-workbench-control-mode-tab="mesh"]').click();
    assert.match(
      await controlWindow.locator('[data-workbench-control-window="metrics"]').innerText(),
      /qualification imported mesh graph/u,
    );

    await click(page, '[data-workbench-control-action="reset-snapshot-source"]', "Reset topology source");
    await page.waitForFunction(() =>
      document.querySelector('[data-workbench-control-window="root"]')?.getAttribute("data-workbench-control-source") === "derived_frontend",
    );
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench Store GUI stages, exports, removes, and restores an asset action", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, acceptDownloads: true });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const entry = {
    id: "qualification-gui-operator",
    kind: "operator",
    title: "Qualification GUI operator",
    version: "2.18.3",
    source_id: "qualification",
    source_kind: "builtin",
    tags: ["qualification", "gui"],
    install: { mode: "workspace", requires_download: false, target: "operators/qualification-gui" },
  };

  await page.route("**/api/v1/store**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
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
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.waitUntil(
      (state) => state.selectedProjectId === "qualification-project",
      { timeoutMs: 15_000 },
    ));
    await click(page, '[aria-label="workbench-rail:store"]', "Store rail");
    const storePanel = page.locator('[data-workbench-store-panel="true"]');
    const catalogEntry = page.locator('[data-workbench-store-entry-id="qualification-gui-operator"]');
    await catalogEntry.waitFor({ state: "visible", timeout: 30_000 });

    const stageButton = catalogEntry.locator('[data-workbench-store-entry-action="stage"]');
    await stageButton.click();
    await page.waitForFunction(() =>
      document.querySelector('[data-workbench-store-panel="true"]')?.getAttribute("data-workbench-store-manifest-count") === "1",
    );
    assert.equal(await stageButton.isDisabled(), true);

    await click(page, '[data-workbench-store-view-tab="project"]', "Project Store manifest");
    const manifestEntry = page.locator('[data-workbench-store-manifest-entry-id="qualification-gui-operator"]');
    await manifestEntry.waitFor({ state: "visible" });
    const downloadPromise = page.waitForEvent("download");
    await click(page, '[data-workbench-store-manifest-action="export"]', "Export Store manifest");
    const download = await downloadPromise;
    assert.equal(download.suggestedFilename(), "qualification-project.store-manifest.json");
    const downloadPath = await download.path();
    assert.ok(downloadPath);
    const manifest = JSON.parse(await readFile(downloadPath, "utf8"));
    assert.equal(manifest.project_id, "qualification-project");
    assert.equal(manifest.entries[0]?.id, entry.id);

    await manifestEntry.locator('[data-workbench-store-manifest-action="remove"]').click();
    await manifestEntry.waitFor({ state: "detached" });
    assert.equal(await storePanel.getAttribute("data-workbench-store-manifest-count"), "0");
    await click(page, '[data-workbench-store-view-tab="catalog"]', "Store catalog return");
    assert.equal(await catalogEntry.locator('[data-workbench-store-entry-action="stage"]').isEnabled(), true);
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });
