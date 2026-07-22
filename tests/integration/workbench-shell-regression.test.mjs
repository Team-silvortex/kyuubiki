import test from "node:test";
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
    } finally {
      await browser?.close();
      await environment.cleanup();
    }
  },
  { timeout: 180_000 },
);
