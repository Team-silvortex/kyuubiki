import { chromium } from "playwright";
import { isRestrictedPlaywrightLaunchError, reportRestrictedPlaywrightSkip } from "./playwright-runtime-guard.mjs";

const baseUrl = process.env.WORKFLOW_BENCHMARK_URL || "http://127.0.0.1:3000/workflow-benchmark";

async function waitForDoublePaint(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function openBuilder(page) {
  await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 30_000 });
  await page.waitForFunction(() => Boolean(window.__kyuubikiWorkflowDebug), { timeout: 30_000 });
  await page.evaluate(async () => {
    const waitForPaint = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    window.__kyuubikiWorkflowDebug?.setSurfaceTab("builder");
    await waitForPaint();
  });
  await waitForDoublePaint(page);
}

async function setInputValue(page, selector, value) {
  const locator = page.locator(selector);
  await locator.waitFor({ state: "visible", timeout: 10_000 });
  await locator.fill(value);
  await locator.blur();
  await waitForDoublePaint(page);
}

async function getInputValue(page, selector) {
  return page.locator(selector).inputValue();
}

async function getSelectValue(page, selector) {
  return page.locator(selector).inputValue();
}

async function run() {
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
  } catch (error) {
    if (isRestrictedPlaywrightLaunchError(error)) {
      reportRestrictedPlaywrightSkip("Workflow topology regression check", error);
      return;
    }
    throw error;
  }
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  try {
    await openBuilder(page);

    const outputPortIdField = '[data-workflow-port-stable="extract.summary:outputs:0:id"]';
    const outputPortTypeField = '[data-workflow-port-stable="extract.summary:outputs:0:artifact_type"]';
    const edgeSourcePortSelect = '[data-workflow-edge-select="edge.extract.transform:from.port"]';
    const edgeArtifactTypeField = '[data-workflow-edge-field="edge.extract.transform:artifact_type"]';

    const renamedPortId = "summary_regression_port";
    const renamedArtifactType = "workflow.summary.regression";

    await setInputValue(page, outputPortIdField, renamedPortId);
    const syncedSourcePort = await getSelectValue(page, edgeSourcePortSelect);
    if (syncedSourcePort !== renamedPortId) {
      throw new Error(`Expected edge source port to sync to "${renamedPortId}", received "${syncedSourcePort}"`);
    }

    await setInputValue(page, outputPortTypeField, renamedArtifactType);
    const syncedArtifactType = await getInputValue(page, edgeArtifactTypeField);
    if (syncedArtifactType !== renamedArtifactType) {
      throw new Error(`Expected edge artifact type to sync to "${renamedArtifactType}", received "${syncedArtifactType}"`);
    }

    console.log("Workflow topology regression check passed");
    console.log(JSON.stringify({
      url: baseUrl,
      syncedSourcePort,
      syncedArtifactType,
    }, null, 2));
  } finally {
    await page.close();
    await browser.close();
  }
}

run().catch((error) => {
  if (isRestrictedPlaywrightLaunchError(error)) {
    reportRestrictedPlaywrightSkip("Workflow topology regression check", error);
    process.exit(0);
  }
  console.error(`Workflow topology regression check failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
