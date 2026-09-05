import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { after, before, test } from "node:test";

import { clickIntegrationControl as click, launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import {
  ROOT, chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl,
} from "./workbench-ui-isolated.shared.mjs";

let browser;
let runtime;
const screenshots = path.join(ROOT, "tmp/desktop-gui-regression-artifacts/workflow-readiness");

before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  browser = await launchIntegrationBrowser(chromium);
  await mkdir(screenshots, { recursive: true });
}, { timeout: 180_000 });

after(async () => {
  await browser?.close();
  await runtime?.stop();
}, { timeout: 90_000 });

async function openBuilder(page) {
  await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
  await click(page, '[aria-label="workbench-rail:workflow"]', "Workflow rail");
  await click(page, '[data-workflow-surface-tab="catalog"]', "Catalog");
  await page.locator('[data-workflow-catalog-search="query"]').fill("mechanical");
  await click(page,
    '[data-workflow-catalog-id="workflow.bar-1d-summary-json"] [data-workflow-catalog-action="open-builder"]',
    "Open builder",
  );
  await page.locator('[data-workflow-builder-action="run-draft"]:enabled').waitFor({ state: "visible" });
  return page.locator('[data-workflow-builder-shell="builder"]');
}

async function openInputs(builder) {
  await builder.locator('[data-workflow-builder-stage-target="contracts"]').click();
  await builder.locator('[data-workflow-contract-view-target="inputs"]').click();
  return builder.locator('[data-workflow-input-artifact="bar_1d_model"]');
}

async function sourceGraph(page) {
  const response = await page.request.get(`${runtime.backendUrl}/api/v1/workflows/catalog`);
  assert.equal(response.status(), 200);
  return (await response.json()).workflows[0].graph;
}

async function importGraph(builder, graph) {
  await builder.locator('input[type="file"]').first().setInputFiles({
    name: "readiness.workflow-graph.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(graph)),
  });
  await builder.locator('[data-workflow-import-message="text"]').waitFor({ state: "visible" });
}

for (const viewport of [{ width: 1180, height: 920 }, { width: 390, height: 844 }]) {
  test(`Workbench draft blocker closes locate, repair, submit, and result at ${viewport.width}px`, async () => {
    const context = await browser.newContext({ viewport });
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const graphSubmissionsBefore = runtime.state.graphSubmissions;
    const catalogSubmissionsBefore = runtime.state.catalogSubmissions;
    try {
      const builder = await openBuilder(page);
      const input = await openInputs(builder);
      const original = await input.inputValue();
      await input.fill('{"nodes":');
      await builder.locator('[data-workflow-draft-readiness="blocked"]').waitFor({ state: "visible" });
      await builder.locator('[data-workflow-builder-stage-target="topology"]').click();

      const summary = builder.locator('[data-workflow-draft-blocker="summary"]');
      const locate = builder.locator('[data-workflow-builder-action="locate-blocker"]');
      const run = builder.locator('[data-workflow-builder-action="run-draft"]');
      assert.equal(await run.isDisabled(), true);
      assert.match(await summary.innerText(), /bar_1d_model/u);
      assert.ok(await run.getAttribute("aria-describedby"));
      await summary.scrollIntoViewIfNeeded();
      const bounds = await summary.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const text = element.querySelector('[role="status"]').getBoundingClientRect();
        const button = element.querySelector("button");
        const action = button.getBoundingClientRect();
        const hit = document.elementFromPoint(action.x + action.width / 2, action.y + action.height / 2);
        return {
          overflow: element.scrollWidth > element.clientWidth + 1,
          outsideViewport: box.left < 0 || box.right > window.innerWidth + 1,
          overlap: Math.min(text.right, action.right) - Math.max(text.left, action.left) > 1 &&
            Math.min(text.bottom, action.bottom) - Math.max(text.top, action.top) > 1,
          covered: !hit || !button.contains(hit),
        };
      });
      assert.deepEqual(bounds, { overflow: false, outsideViewport: false, overlap: false, covered: false });
      await page.screenshot({ path: path.join(screenshots, `blocked-${viewport.width}.png`) });

      for (let attempt = 0; attempt < 2; attempt += 1) {
        await locate.click();
        await page.waitForFunction(() =>
          document.activeElement?.getAttribute("data-workflow-input-artifact") === "bar_1d_model",
        );
        assert.equal(await builder.getAttribute("data-workflow-builder-stage"), "contracts");
        assert.equal(await input.getAttribute("aria-invalid"), "true");
        if (attempt === 0) await builder.locator('[data-workflow-contract-view-target="dataset"]').click();
      }
      assert.equal(runtime.state.graphSubmissions, graphSubmissionsBefore);
      assert.equal(runtime.state.catalogSubmissions, catalogSubmissionsBefore);
      await input.fill(original);
      await builder.locator('[data-workflow-draft-readiness="ready"]').waitFor({ state: "visible" });
      assert.equal(await summary.count(), 0);
      assert.equal(await input.getAttribute("aria-invalid"), "false");
      assert.equal(await run.isEnabled(), true);
      assert.equal(await run.getAttribute("aria-describedby"), null);

      await run.click();
      const completed = page.locator('[data-workflow-run-id="qualification-workflow-job"][data-workflow-run-status="completed"]');
      await completed.waitFor({ state: "visible", timeout: 45_000 });
      assert.match(await completed.innerText(), /max_displacement=1\.250e-3/u);
      assert.equal(runtime.state.graphSubmissions, graphSubmissionsBefore + 1);
      assert.equal(runtime.state.catalogSubmissions, catalogSubmissionsBefore);
      assert.deepEqual(runtime.state.submissionBodies.at(-1).input_artifacts.bar_1d_model, JSON.parse(original));
      assert.deepEqual(errors, []);
    } finally {
      await context.close();
    }
  }, { timeout: 90_000 });
}

test("Workbench draft blocker repeatedly reopens the relevant topology editor", async () => {
  const context = await browser.newContext({ viewport: { width: 1180, height: 920 } });
  const page = await context.newPage();
  page.on("dialog", (dialog) => dialog.accept());
  try {
    const builder = await openBuilder(page);
    const graph = await sourceGraph(page);
    graph.nodes[0].operator_id = "solve.readiness_unknown";
    await importGraph(builder, graph);
    await builder.locator('[data-workflow-draft-readiness="blocked"]').waitFor({ state: "visible" });
    assert.match(await builder.locator('[data-workflow-draft-blocker="summary"]').innerText(), /solve.readiness_unknown/u);
    for (let attempt = 0; attempt < 2; attempt += 1) {
      await builder.locator('[data-workflow-topology-view-target="edges"]').click();
      await builder.locator('[data-workflow-builder-action="locate-blocker"]').click();
      await builder.locator('[data-workflow-node-field="bar_1d_model:operator_id"]').waitFor({ state: "visible", timeout: 10_000 });
      assert.equal(await builder.locator('[data-workflow-topology]').getAttribute("data-workflow-topology-view"), "nodes");
    }
    await builder.locator('[data-workflow-node-field="bar_1d_model:operator_id"]').fill("solve.bar_1d");
    await builder.locator('[data-workflow-draft-readiness="ready"]').waitFor({ state: "visible" });
  } finally {
    await context.close();
  }
}, { timeout: 75_000 });

test("Workbench draft readiness follows newly added and removed input contracts", async () => {
  const context = await browser.newContext({ viewport: { width: 1180, height: 920 } });
  const page = await context.newPage();
  try {
    const builder = await openBuilder(page);
    const graph = await sourceGraph(page);
    graph.nodes.push({ ...structuredClone(graph.nodes[0]), id: "extra_model" });
    await importGraph(builder, graph);
    const original = await (await openInputs(builder)).inputValue();
    await builder.locator('[data-workflow-contract-view-target="entry"]').click();
    const entries = builder.locator('[data-workflow-artifact-card="entry"]');
    await entries.locator(':scope > .button-row button').click();
    const extraEntry = entries.locator('[data-workflow-artifact-key]').last();
    await extraEntry.locator("input").nth(0).fill("extra_model");
    await extraEntry.locator("input").nth(1).fill("study_model/bar_1d");
    await builder.locator('[data-workflow-draft-readiness="blocked"]').waitFor({ state: "visible" });
    assert.equal(await builder.locator('[data-workflow-draft-blocker-count]').getAttribute("data-workflow-draft-blocker-count"), "1");
    assert.match(await builder.locator('[data-workflow-draft-blocker="summary"]').innerText(), /extra_model/u);
    assert.equal(await builder.locator('[data-workflow-input-artifact="extra_model"]').count(), 0);

    await builder.locator('[data-workflow-builder-action="locate-blocker"]').click();
    const input = builder.locator('[data-workflow-input-artifact="extra_model"]');
    await input.waitFor({ state: "visible" });
    assert.equal(await input.inputValue(), "");
    await input.fill(original);
    await builder.locator('[data-workflow-draft-readiness="ready"]').waitFor({ state: "visible" });
    await input.fill("{");
    await builder.locator('[data-workflow-contract-view-target="entry"]').click();
    await extraEntry.locator(':scope > .button-row button').first().click();
    await builder.locator('[data-workflow-draft-readiness="ready"]').waitFor({ state: "visible" });
    assert.equal(await builder.locator('[data-workflow-draft-blocker="summary"]').count(), 0);
  } finally {
    await context.close();
  }
}, { timeout: 75_000 });
