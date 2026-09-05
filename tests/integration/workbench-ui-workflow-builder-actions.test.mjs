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

async function click(page, selector, label) {
  const target = page.locator(selector);
  await target.waitFor({ state: "visible", timeout: 30_000 });
  assert.equal(await target.count(), 1, `${label} should resolve to one visible control`);
  await target.click({ timeout: 15_000 });
}

async function openQualificationBuilder(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await click(page, '[aria-label="workbench-rail:workflow"]', "Workflow rail");
  await click(page, '[data-workflow-surface-tab="catalog"]', "Workflow catalog tab");
  const catalogSearch = page.locator('[data-workflow-catalog-search="query"]');
  await catalogSearch.waitFor({ state: "visible" });
  await catalogSearch.fill("mechanical");
  const catalogCard = page.locator('[data-workflow-catalog-id="workflow.bar-1d-summary-json"]');
  await catalogCard.waitFor({ state: "visible" });
  await catalogCard.locator('[data-workflow-catalog-action="open-builder"]').click();
  const builder = page.locator('[data-workflow-builder-shell="builder"]');
  await builder.waitFor({ state: "visible" });
  return builder;
}

async function openBuilderSecondaryTools(builder) {
  const tools = builder.locator('[data-workflow-builder-tools="secondary"]');
  if (await tools.getAttribute("open") === null) await tools.locator("summary").click();
  await tools.locator('[data-workflow-builder-action="promote-draft"]').waitFor({ state: "visible" });
}

function resetWorkflowRuntimeState() {
  runtime.state.catalogFetches = 0;
  runtime.state.operatorFetches = 0;
  runtime.state.catalogSubmissions = 0;
  runtime.state.graphSubmissions = 0;
  runtime.state.jobPolls = 0;
  runtime.state.historyFetches = 0;
  runtime.state.submissionBodies.length = 0;
}

test("Workbench workflow catalog runs a visible entry directly to completion", async () => {
  resetWorkflowRuntimeState();
  const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
  const page = await context.newPage();
  try {
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await click(page, '[aria-label="workbench-rail:workflow"]', "Workflow rail");
    await click(page, '[data-workflow-surface-tab="catalog"]', "Workflow catalog tab");
    await page.locator('[data-workflow-catalog-search="query"]').fill("mechanical");
    const catalogCard = page.locator('[data-workflow-catalog-id="workflow.bar-1d-summary-json"]');
    await catalogCard.waitFor({ state: "visible" });
    const submission = page.waitForResponse((response) =>
      response.request().method() === "POST" &&
      new URL(response.url()).pathname.endsWith("/workflow.bar-1d-summary-json/jobs"),
    );
    await catalogCard.locator('[data-workflow-catalog-action="run"]').click();
    assert.equal((await submission).status(), 202);
    await page.locator(
      '[data-workflow-run-id="qualification-workflow-job"][data-workflow-run-status="completed"]',
    ).waitFor({ state: "visible", timeout: 45_000 });
    assert.equal(
      await page.locator('[data-workbench-workflow-surface]').getAttribute("data-workbench-workflow-surface"),
      "runs",
    );
    assert.equal(runtime.state.catalogSubmissions, 1);
    assert.equal(runtime.state.graphSubmissions, 0);
  } finally {
    await context.close();
  }
}, { timeout: 75_000 });

test("Workbench isolated workflow UI promotes and runs a valid graph draft", async () => {
  resetWorkflowRuntimeState();
  const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
  const page = await context.newPage();
  try {
    const builder = await openQualificationBuilder(page);
    await openBuilderSecondaryTools(builder);
    await click(page, '[data-workflow-builder-action="promote-draft"]', "Promote workflow draft");
    await page.waitForFunction(() => {
      const records = JSON.parse(
        window.localStorage.getItem("kyuubiki.workbench.workflowLibrary.v1") || "[]",
      );
      return records.length === 1;
    });
    const localWorkflows = await page.evaluate(() => JSON.parse(
      window.localStorage.getItem("kyuubiki.workbench.workflowLibrary.v1") || "[]",
    ));
    assert.equal(localWorkflows[0]?.sourceWorkflowId, "workflow.bar-1d-summary-json");
    assert.match(localWorkflows[0]?.id ?? "", /^workflow\.local\./u);

    const graphSubmissionsBefore = runtime.state.graphSubmissions;
    const graphSubmission = page.waitForResponse((response) =>
      response.request().method() === "POST" &&
      new URL(response.url()).pathname === "/api/v1/workflows/graph/jobs",
    );
    await click(page, '[data-workflow-builder-action="run-draft"]', "Run workflow draft");
    assert.equal((await graphSubmission).status(), 202);
    const completedRun = page.locator(
      '[data-workflow-run-id="qualification-workflow-job"][data-workflow-run-status="completed"]',
    );
    await completedRun.waitFor({ state: "visible", timeout: 45_000 });
    assert.equal(
      await page.locator('[data-workbench-workflow-surface]').getAttribute("data-workbench-workflow-surface"),
      "runs",
    );
    assert.equal(runtime.state.graphSubmissions, graphSubmissionsBefore + 1);
    assert.equal(runtime.state.submissionBodies.at(-1)?.graph?.schema_version, "kyuubiki.workflow-graph/v1");
    assert.equal(typeof runtime.state.submissionBodies.at(-1)?.input_artifacts?.bar_1d_model, "object");
  } finally {
    await context.close();
  }
}, { timeout: 90_000 });

test("Workbench workflow builder mutates topology and inserts a control-flow plane", async () => {
  resetWorkflowRuntimeState();
  const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
  const page = await context.newPage();
  try {
    const builder = await openQualificationBuilder(page);
    const nodesView = builder.locator('[data-workflow-topology-view-target="nodes"]');
    const edgesView = builder.locator('[data-workflow-topology-view-target="edges"]');
    const nodesBefore = Number(await nodesView.getAttribute("data-workflow-topology-view-count"));
    const edgesBefore = Number(await edgesView.getAttribute("data-workflow-topology-view-count"));

    await builder.locator('[data-workflow-topology-view-target="add"]').click();
    await builder.locator('[data-workflow-topology-kind="select"]').selectOption("transform");
    await builder.locator('[data-workflow-topology-action="add-node"]').click();
    await page.waitForFunction(
      (previousCount) => Number(document.querySelector('[data-workflow-topology-view-target="nodes"]')?.getAttribute("data-workflow-topology-view-count")) === previousCount + 1,
      nodesBefore,
    );

    await edgesView.click();
    await builder.locator('[data-workflow-topology-action="add-edge"]').click();
    await page.waitForFunction(
      (previousCount) => Number(document.querySelector('[data-workflow-topology-view-target="edges"]')?.getAttribute("data-workflow-topology-view-count")) === previousCount + 1,
      edgesBefore,
    );

    await builder.locator('[data-workflow-builder-stage-target="control"]').click();
    const insertControlFlow = builder.locator('[data-workflow-control-empty-action="insert"]');
    await insertControlFlow.waitFor({ state: "visible" });
    await insertControlFlow.click();
    await page.waitForFunction(() => document.querySelectorAll("[data-workflow-control-node-id]").length === 2);
    assert.equal(await insertControlFlow.count(), 0);

    await builder.locator('[data-workflow-builder-stage-target="topology"]').click();
    assert.equal(Number(await nodesView.getAttribute("data-workflow-topology-view-count")), nodesBefore + 3);
    assert.ok(Number(await edgesView.getAttribute("data-workflow-topology-view-count")) >= edgesBefore + 4);
  } finally {
    await context.close();
  }
}, { timeout: 75_000 });
