import assert from "node:assert/strict";
import test from "node:test";
import {
  assertNoPageErrors,
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

test("Installer mock-IPC mutations reject overlap and recover after failure without automatic replay", {
  timeout: 120_000,
}, async () => {
  const environment = await createDesktopShellRegressionEnvironment();
  let browser;
  try {
    browser = await launchIntegrationBrowser(chromium);
    const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
    await page.goto(environment.installerUrl, { waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => /Unified regression gate is warn/.test(
      document.querySelector("#completion-message")?.textContent || "",
    ));
    await page.evaluate(() => {
      window.__installerRecoveryEvents = [];
      window.__installerRecoveryRequests = [];
      document.addEventListener("kyuubiki:installer-action", (event) => {
        window.__installerRecoveryEvents.push(event.detail);
      });
      const original = window.__TAURI__.core.invoke;
      window.__TAURI__.core.invoke = async (command, payload) => {
        if (command === "guarded_mutation_action") {
          window.__installerRecoveryRequests.push(payload.payload.action);
          // The preview owns all IPC. Delay or fail it without touching a real installation.
          await new Promise((resolve, reject) => {
            window.__finishInstallerMutation = (fail) => fail
              ? reject(new Error("injected installer transport failure")) : resolve();
          });
        }
        return original(command, payload);
      };
    });

    await page.locator('[data-action="doctor"]:visible').first().click();
    await page.waitForFunction(() => window.__kyuubikiInstallerLastCompletedAction === "doctor");
    const completedAt = await page.evaluate(() => window.__kyuubikiInstallerActionCompletedAt);
    const bootstrap = page.locator('[data-action="bootstrap"]:visible').first();
    await bootstrap.click();
    await page.waitForFunction(() => window.__installerRecoveryRequests.length === 1);
    assert.equal(await bootstrap.getAttribute("aria-busy"), "true");
    await bootstrap.click();
    await page.waitForFunction(() => window.__kyuubikiInstallerActionStatus === "blocked");
    await page.locator('.sidebar-tab[data-tab="services"]').click();
    assert.equal(await page.locator('[data-panel="services"]').isVisible(), true);
    await page.locator('[data-action="service-start-local"]:visible').click();
    await page.waitForFunction(() => window.__kyuubikiInstallerLastAction === "service-start-local");
    assert.deepEqual(await page.evaluate(() => ({
      requests: window.__installerRecoveryRequests,
      status: window.__kyuubikiInstallerActionStatus,
      active: window.__kyuubikiInstallerActiveAction,
      completed: window.__kyuubikiInstallerLastCompletedAction,
      completedAt: window.__kyuubikiInstallerActionCompletedAt,
    })), { requests: ["bootstrap"], status: "blocked", active: "bootstrap", completed: "doctor", completedAt });

    await page.evaluate(() => window.__finishInstallerMutation(true));
    await page.waitForFunction(() => window.__kyuubikiInstallerActionStatus === "failed");
    assert.equal(await bootstrap.getAttribute("aria-busy"), null);
    assert.equal(await page.evaluate(() => window.__kyuubikiInstallerActiveAction), null);
    assert.equal(await page.evaluate(() => window.__kyuubikiInstallerActionCompletedAt), completedAt);
    assert.match(await page.locator("#output").textContent(), /injected installer transport failure/);

    await page.locator('.sidebar-tab[data-tab="setup"]').click();
    await bootstrap.click();
    await page.waitForFunction(() => window.__installerRecoveryRequests.length === 2);
    await page.evaluate(() => window.__finishInstallerMutation(false));
    await page.waitForFunction(() => window.__kyuubikiInstallerLastCompletedAction === "bootstrap");
    assert.match(await page.locator("#completion-message").textContent(), /Bootstrap complete/);
    assert.equal(await bootstrap.getAttribute("aria-busy"), null);

    await page.locator('.sidebar-tab[data-tab="services"]').click();
    await page.locator('[data-action="service-start-local"]:visible').click();
    await page.waitForFunction(() => window.__installerRecoveryRequests.length === 3);
    await page.evaluate(() => window.__finishInstallerMutation(false));
    await page.waitForFunction(() => window.__kyuubikiInstallerLastCompletedAction === "service-start-local");
    assert.deepEqual(await page.evaluate(() => window.__installerRecoveryRequests), [
      "bootstrap", "bootstrap", "service_start",
    ]);
    assert.deepEqual(await page.evaluate(() => window.__installerRecoveryEvents
      .filter(({ action }) => action !== "doctor").map(({ action, status }) => [action, status])), [
      ["bootstrap", "running"], ["bootstrap", "blocked"], ["service-start-local", "blocked"],
      ["bootstrap", "failed"], ["bootstrap", "running"], ["bootstrap", "completed"],
      ["service-start-local", "running"], ["service-start-local", "completed"],
    ]);
    await assertNoPageErrors(page);
  } finally {
    await browser?.close();
    await environment.cleanup();
  }
});
