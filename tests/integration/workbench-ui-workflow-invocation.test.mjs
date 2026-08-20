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
}, { timeout: 30_000 });

async function click(page, selector, label) {
  const candidates = page.locator(selector);
  const target = candidates.first();
  await target.waitFor({ state: "visible", timeout: 30_000 });
  assert.equal(await candidates.count(), 1, `${label} should resolve to one visible control`);
  await target.click({ timeout: 15_000 });
  return target;
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
