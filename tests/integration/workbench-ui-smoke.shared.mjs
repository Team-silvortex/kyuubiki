import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

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

async function clickStable(locator, label, attempts = 4) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await locator.waitFor({ state: "visible", timeout: 10_000 });
      await locator.click({ timeout: 10_000 });
      return;
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, attempt * 250));
    }
  }
  throw new Error(`failed to click ${label}: ${lastError?.message ?? lastError}`);
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
  await page
    .locator('[data-workbench-state="loaded-model"]')
    .filter({ hasText: importedModelLabel })
    .first()
    .waitFor({ state: "attached", timeout: 30_000 });

  const resultButton = page.getByRole("button", { name: "Result" }).first();
  assert.equal(await resultButton.isVisible(), true, `${sampleLabel} should expose Result`);
  await clickStable(resultButton, `${sampleLabel} Result tab`);
  await clickStable(page.getByRole("button", { name: "Actions" }).first(), `${sampleLabel} Actions tab`);
  await clickStable(page.getByRole("button", { name: "Export Data" }).first(), `${sampleLabel} Export Data action`);

  const exportJsonButton = page.getByRole("button", { name: "Export Data JSON" }).first();
  const exportCsvButton = page.getByRole("button", { name: "Export Data CSV" }).first();

  assert.equal(await exportJsonButton.isVisible(), true, `${sampleLabel} should expose Export Data JSON`);
  assert.equal(await exportCsvButton.isVisible(), true, `${sampleLabel} should expose Export Data CSV`);
}
