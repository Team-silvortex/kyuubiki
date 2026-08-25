import test from "node:test";
import assert from "node:assert/strict";
import {
  assertWorkbenchShellRegression,
  chromium,
  createWorkbenchRegressionEnvironment,
} from "./workbench-shell-regression.shared.mjs";
import { captureDesktopGuiArtifacts } from "./desktop-gui-artifacts.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

test(
  "Workbench desktop shell mounts runtime, logs, and embedded viewer cleanly in headless preview",
  async () => {
    const environment = await createWorkbenchRegressionEnvironment();
    let browser;

    try {
      browser = await launchIntegrationBrowser(chromium);
      for (const viewport of [
        { width: 1440, height: 1100 },
        { width: 1180, height: 920 },
        { width: 1080, height: 760 },
      ]) {
        const page = await browser.newPage();
        try {
          await page.goto(environment.workbenchUrl, { waitUntil: "networkidle", timeout: 60_000 });
          await assertWorkbenchShellRegression(page, viewport);
        } catch (error) {
          await captureDesktopGuiArtifacts(page, {
            suite: "workbench-shell-regression",
            scenario: "runtime-logs-viewer",
            viewport,
            error,
          });
          throw error;
        } finally {
          await page.close();
        }
      }

      const failurePage = await browser.newPage();
      try {
        await failurePage.goto(environment.workbenchUrl, { waitUntil: "networkidle", timeout: 60_000 });
        await failurePage.evaluate(() => {
          document.querySelector('[data-action="reload-frame"]')?.click();
        });
        await failurePage.waitForFunction(
          () => window.__kyuubikiWorkbenchActionStatus === "completed",
        );
        const priorCompletion = await failurePage.evaluate(() => ({
          action: window.__kyuubikiWorkbenchLastCompletedAction,
          at: window.__kyuubikiWorkbenchActionCompletedAt,
        }));
        await failurePage.evaluate(() => {
          const originalInvoke = window.__TAURI__.core.invoke;
          window.__workbenchOriginalInvoke = originalInvoke;
          window.__TAURI__.core.invoke = async (command, payload) => {
            if (command === "service_status") throw new Error("forced workbench status failure");
            return originalInvoke(command, payload);
          };
        });
        await failurePage.evaluate(() => {
          document.querySelector('[data-action="refresh"]')?.click();
        });
        await failurePage.waitForFunction(
          () => window.__kyuubikiWorkbenchActionStatus === "failed",
        );
        const failure = await failurePage.evaluate(() => ({
          action: window.__kyuubikiWorkbenchLastCompletedAction,
          at: window.__kyuubikiWorkbenchActionCompletedAt,
          output: document.querySelector("#status-output")?.textContent || "",
        }));

        assert.deepEqual(
          { action: failure.action, at: failure.at },
          priorCompletion,
          "a failed refresh must preserve the previous successful action receipt",
        );
        assert.match(failure.output, /forced workbench status failure/);
      } finally {
        await failurePage.close();
      }
    } finally {
      await browser?.close();
      await environment.cleanup();
    }
  },
  { timeout: 180_000 },
);
