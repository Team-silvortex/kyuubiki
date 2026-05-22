import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
export const ENTRYPOINT = `${ROOT}/scripts/kyuubiki`;
export const FRONTEND_URL = "http://127.0.0.1:3000";

const requireFromFrontend = createRequire(`${ROOT}/apps/frontend/package.json`);
export const { chromium } = requireFromFrontend("playwright");

export function runKyuubiki(args) {
  return execFileSync("zsh", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
    env: {
      ...process.env,
    },
  });
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
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

export async function assertWorkbenchSampleUi(page, domainLabel, sampleLabel, importedModelLabel, studyLabel) {
  await page.getByRole("button", { name: "H History" }).click();
  await page.getByRole("button", { name: /^S\s+Samples$/ }).click();
  if (domainLabel) {
    await page
      .locator("button")
      .filter({ hasText: new RegExp(`^${escapeRegExp(domainLabel)}$`) })
      .first()
      .click();
  }
  await page
    .locator("button.history-item")
    .filter({ hasText: sampleLabel })
    .first()
    .click();
  await page.waitForFunction(
    ({ importedModelLabel: importedModel, studyLabel: study }) => {
      const text = document.body.innerText || "";
      return text.includes(`Imported model: ${importedModel}`) && text.includes(study);
    },
    { importedModelLabel, studyLabel },
    { timeout: 30_000 },
  );

  const resultButton = page.getByRole("button", { name: "Result" }).first();
  assert.equal(await resultButton.isVisible(), true, `${sampleLabel} should expose Result`);
  await resultButton.click();
  await page.getByRole("button", { name: "Actions" }).first().click();
  await page.getByRole("button", { name: "Export Data" }).first().click();

  const exportJsonButton = page.getByRole("button", { name: "Export Data JSON" }).first();
  const exportCsvButton = page.getByRole("button", { name: "Export Data CSV" }).first();

  assert.equal(await exportJsonButton.isVisible(), true, `${sampleLabel} should expose Export Data JSON`);
  assert.equal(await exportCsvButton.isVisible(), true, `${sampleLabel} should expose Export Data CSV`);
}
