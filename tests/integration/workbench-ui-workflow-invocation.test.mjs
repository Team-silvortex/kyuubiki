import assert from "node:assert/strict";
import test from "node:test";

import {
  chromium,
  FRONTEND_URL,
  runKyuubiki,
  waitForFrontend,
} from "./workbench-ui-smoke.shared.mjs";

async function click(page, selector, label) {
  const target = page.locator(selector).first();
  await target.waitFor({ state: "visible", timeout: 30_000 });
  await target.click({ timeout: 15_000 });
  return target;
}

test(
  "Workbench workflow UI supports catalog, builder, operator insertion, draft saving, and execution",
  async () => {
    const browser = await chromium.launch({ headless: true });

    try {
      runKyuubiki(["restart-local"]);
      await waitForFrontend();

      const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
      await page.goto(FRONTEND_URL, { waitUntil: "networkidle", timeout: 60_000 });

      await click(page, '[aria-label="workbench-rail:workflow"]', "Workflow rail");
      await page.locator('[data-workbench-workflow-surface]').waitFor({ state: "visible", timeout: 30_000 });

      await click(page, '[data-workflow-surface-tab="catalog"]', "Workflow catalog tab");
      const catalogSearch = page.locator('[data-workflow-catalog-search="query"]');
      await catalogSearch.waitFor({ state: "visible", timeout: 30_000 });
      await catalogSearch.fill("mechanical");
      await page.locator('[data-workflow-catalog-action="open-builder"]').first().waitFor({ state: "visible", timeout: 30_000 });

      await click(page, '[data-workflow-catalog-action="open-builder"]', "Open workflow builder");
      const builder = page.locator('[data-workflow-builder-shell="builder"]');
      await builder.waitFor({ state: "visible", timeout: 30_000 });

      const nodesBefore = await builder.locator('[data-workflow-node-id]').count();
      const operatorSearch = builder.locator('[data-workflow-operator-search="query"]');
      await operatorSearch.fill("mechanical");
      const quickInsert = builder.locator('[data-workflow-operator-action="quick-insert"]').first();
      await quickInsert.waitFor({ state: "visible", timeout: 30_000 });
      await quickInsert.click({ timeout: 15_000 });
      await page.waitForFunction(
        (previousCount) => document.querySelectorAll("[data-workflow-node-id]").length > previousCount,
        nodesBefore,
        { timeout: 30_000 },
      );

      await click(page, '[data-workflow-builder-action="save-draft"]', "Save workflow draft");
      await builder.locator('[data-workflow-import-message="text"]').waitFor({ state: "visible", timeout: 15_000 });

      await click(page, '[data-workflow-builder-action="run-catalog"]', "Run catalog workflow");
      await page.locator('[data-workbench-workflow-surface="runs"]').waitFor({ state: "visible", timeout: 30_000 });
      await page.waitForFunction(
        () => !((document.body.innerText || "").includes("No workflow runs yet.")),
        undefined,
        { timeout: 90_000 },
      );

      assert.equal(await page.locator('[data-workbench-workflow-surface="runs"]').isVisible(), true);
      await page.close();
    } finally {
      await browser.close();
      try {
        runKyuubiki(["stop"]);
      } catch {
        // Keep cleanup best-effort for local integration runs.
      }
    }
  },
  { timeout: 210_000 },
);
