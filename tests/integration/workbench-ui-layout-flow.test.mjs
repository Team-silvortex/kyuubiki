import assert from "node:assert/strict";
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

async function openRail(page, section) {
  await page.locator(`[aria-label="workbench-rail:${section}"]`).click();
  await page.locator(`[data-workbench-sidebar-section="${section}"]`).waitFor({ state: "visible" });
}

test("Workbench keeps overview routes shallow and isolates inspector actions", async () => {
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  try {
    await page.route("**/api/v1/store**", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        entries: Array.from({ length: 20 }, (_, index) => ({
          id: `layout-operator-${index}`,
          kind: "operator",
          title: `Layout operator ${index}`,
          summary: "Bounded catalog entry used to verify the compact Store window.",
          version: "2.18.3",
          source_id: "layout-test",
          source_kind: "builtin",
          tags: ["layout"],
          install: { mode: "workspace", requires_download: false },
        })),
        sources: [{
          id: "layout-test",
          type: "builtin",
          label: "Layout test",
          enabled: true,
          editable: false,
          status: "ready",
          supports: ["operator"],
        }],
        summary: { entry_count: 20, kinds: { operator: 20 }, sources: { "layout-test": 20 } },
      }),
    }));
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    assert.equal(await page.locator('[data-workbench-inspector-actions-export="true"]').count(), 0);
    await page.locator('[data-workbench-inspector-tab-target="actions"]').click();
    await page.locator('[data-workbench-inspector-actions-export="true"]').waitFor({ state: "visible" });
    await page.locator('[data-workbench-inspector-tab-target="status"]').click();
    assert.equal(await page.locator('[data-workbench-inspector-actions-export="true"]').count(), 0);

    for (const [section, stepCount] of [["model", 4], ["workflow", 3], ["system", 2]]) {
      await openRail(page, section);
      await page.locator(".workbench-route-step").first().waitFor({ state: "visible" });
      assert.equal(await page.locator(".workbench-route-step").count(), stepCount);
    }

    await openRail(page, "store");
    const store = page.locator('[data-workbench-store-panel="true"]');
    await store.locator('[data-workbench-store-entry-id]').first().waitFor({ state: "visible" });
    assert.ok(Number(await store.getAttribute("data-workbench-store-visible-count")) <= 6);
    assert.ok(await store.locator('[data-workbench-store-entry-id]').count() <= 6);
    const depth = await store.evaluate((element) => element.scrollHeight / element.clientHeight);
    assert.ok(depth <= 3, `Store task depth should stay bounded, received ${depth.toFixed(2)}`);
    const firstPageIds = await store.locator('[data-workbench-store-entry-id]').evaluateAll(
      (entries) => entries.map((entry) => entry.dataset.workbenchStoreEntryId),
    );
    await store.locator('[data-workbench-store-page-action="next"]').click();
    const secondPageIds = await store.locator('[data-workbench-store-entry-id]').evaluateAll(
      (entries) => entries.map((entry) => entry.dataset.workbenchStoreEntryId),
    );
    assert.notDeepEqual(secondPageIds, firstPageIds);
    await store.locator('[data-workbench-store-view-tab="project"]').click();
    assert.equal(await store.getAttribute("data-workbench-store-view"), "project");
    await openRail(page, "workflow");
    assert.equal(await page.locator('[data-workbench-shell="root"]').getAttribute("data-workbench-section"), "workflow");
    const workflowLayout = await page.locator('[data-workbench-shell="root"]').evaluate((element) => ({
      gridColumns: getComputedStyle(element).gridTemplateColumns,
      innerWidth: window.innerWidth,
      sidebarWidth: element.querySelector(".workspace-sidebar")?.getBoundingClientRect().width ?? 0,
    }));
    assert.ok(
      workflowLayout.sidebarWidth >= 280,
      `Workflow editor sidebar should stay usable, received ${JSON.stringify(workflowLayout)}`,
    );
    await page.locator('[data-workflow-surface-tab="builder"]').click();
    assert.equal(await page.locator('[data-workbench-workflow-surface]').getAttribute("data-workbench-workflow-surface"), "builder");
    const builder = page.locator('[data-workflow-builder-shell="builder"]');
    await builder.waitFor({ state: "visible" });
    assert.equal(await builder.locator('[data-workflow-builder-stage-target]').count(), 4);
    await builder.locator('[data-workflow-builder-stage-target="validation"]').click();
    await builder.locator('[data-workflow-validation-card="card"]').waitFor({ state: "visible" });
    assert.ok(await builder.locator('[data-workflow-validation-issue-id]').count() <= 5);
    await builder.locator('[data-workflow-builder-stage-target="contracts"]').click();
    await builder.locator('[data-workflow-dataset-view-target]').first().waitFor({ state: "visible" });
    assert.equal(await builder.locator('[data-workflow-dataset-view-target]').count(), 4);
    assert.equal(await builder.locator('[data-workflow-dataset-editor]').count(), 1);
    await openRail(page, "system");
    await page.locator('[data-workbench-system-surface-tab="runtime"]').click();
    await page.locator('[data-workbench-runtime-tab="agents"]').click();
    assert.match(await page.locator('[data-workbench-runtime-tab="agents"]').getAttribute("class"), /panel-tab--active/u);
    assert.deepEqual(pageErrors, []);
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });
