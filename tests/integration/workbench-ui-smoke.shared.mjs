import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  acquireLocalRuntimeLock,
  assertNoUnmanagedLocalRuntime,
  releaseLocalRuntimeLock,
} from "./support/local-runtime-lock.mjs";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
export const ENTRYPOINT = `${ROOT}/scripts/kyuubiki-runtime.mjs`;
export const FRONTEND_URL = "http://127.0.0.1:3000";

const requireFromFrontend = createRequire(`${ROOT}/apps/frontend/package.json`);
export const { chromium } = requireFromFrontend("playwright");

export function runKyuubiki(args) {
  return execFileSync("node", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
    env: {
      ...process.env,
    },
  });
}

export function startWorkbenchIntegrationRuntime() {
  acquireLocalRuntimeLock();
  try {
    assertNoUnmanagedLocalRuntime(runKyuubiki(["status"]));
    runKyuubiki(["restart-local"]);
  } catch (error) {
    releaseLocalRuntimeLock();
    throw error;
  }
}

export function stopWorkbenchIntegrationRuntime() {
  try {
    runKyuubiki(["stop"]);
  } finally {
    releaseLocalRuntimeLock();
  }
}

async function clickStable(locator, label, attempts = 4) {
  await locator.waitFor({ state: "visible", timeout: 15_000 });
  assert.equal(await locator.count(), 1, `${label} should resolve to exactly one control`);
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await locator.click({ timeout: 10_000 });
      return;
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, attempt * 250));
    }
  }
  throw new Error(`failed to click ${label}: ${lastError?.message ?? lastError}`);
}

async function waitForAttribute(page, selector, attribute, expected) {
  await page.waitForFunction(
    ({ targetSelector, targetAttribute, expectedValue }) =>
      document.querySelector(targetSelector)?.getAttribute(targetAttribute) === expectedValue,
    { targetSelector: selector, targetAttribute: attribute, expectedValue: expected },
    { timeout: 15_000 },
  );
}

export async function waitForFrontend(timeoutMs = 60_000, intervalMs = 500) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(FRONTEND_URL);
      if (response.status === 200) {
        return;
      }
    } catch {
      // keep polling while Next boots
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${FRONTEND_URL}`);
}

export async function assertWorkbenchSampleUi(page, domainKey, sampleId, sampleLabel, importedModelLabel) {
  await clickStable(page.getByLabel("workbench-rail:library"), "History rail button");
  await clickStable(page.getByLabel("workbench-library-tab:samples"), "Samples tab");
  if (domainKey) {
    await clickStable(page.getByLabel(`workbench-sample-domain:${domainKey}`), `${domainKey} sample domain`);
  }
  await clickStable(page.getByLabel(`workbench-sample:${sampleId}`), `${sampleLabel} sample`);
  const loadedModelState = page.locator('[data-workbench-state="loaded-model"]');
  assert.equal(await loadedModelState.count(), 1, "loaded model state should remain unique");
  await loadedModelState
    .filter({ hasText: importedModelLabel })
    .waitFor({ state: "attached", timeout: 30_000 });

  const inspector = page.locator('[data-workbench-panel="inspector"]');
  assert.equal(await inspector.count(), 1, "Workbench should expose one inspector");
  const resultButton = inspector.getByRole("button", { name: "Result", exact: true });
  assert.equal(await resultButton.isVisible(), true, `${sampleLabel} should expose Result`);
  await clickStable(resultButton, `${sampleLabel} Result tab`);
  await waitForAttribute(page, '[data-workbench-panel="inspector"]', "data-workbench-inspector-tab", "result");

  await clickStable(inspector.getByRole("button", { name: "Actions", exact: true }), `${sampleLabel} Actions tab`);
  await waitForAttribute(page, '[data-workbench-panel="inspector"]', "data-workbench-inspector-tab", "actions");
  await clickStable(inspector.getByRole("button", { name: "Operation History", exact: true }), `${sampleLabel} History action`);
  await waitForAttribute(page, '[data-workbench-panel="inspector"]', "data-workbench-inspector-actions-page", "history");
  await clickStable(inspector.getByRole("button", { name: "Export Data", exact: true }), `${sampleLabel} Export Data action`);
  await waitForAttribute(page, '[data-workbench-panel="inspector"]', "data-workbench-inspector-actions-page", "exports");

  const exportJsonButton = inspector.getByRole("button", { name: "Export Data JSON", exact: true });
  const exportCsvButton = inspector.getByRole("button", { name: "Export Data CSV", exact: true });

  assert.equal(await exportJsonButton.count(), 1, `${sampleLabel} should expose one JSON export`);
  assert.equal(await exportCsvButton.count(), 1, `${sampleLabel} should expose one CSV export`);
  assert.equal(await exportJsonButton.isVisible(), true, `${sampleLabel} should expose Export Data JSON`);
  assert.equal(await exportCsvButton.isVisible(), true, `${sampleLabel} should expose Export Data CSV`);
}
