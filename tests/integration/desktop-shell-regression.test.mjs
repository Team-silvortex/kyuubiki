import test from "node:test";
import {
  assertHubRegression,
  assertInstallerRegression,
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import { captureDesktopGuiArtifacts } from "./desktop-gui-artifacts.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

test(
  "Hub and Installer desktop shells render regression-critical panels in a headless preview",
  async (t) => {
    const environment = await createDesktopShellRegressionEnvironment();
    let browser;

    try {
      browser = await launchIntegrationBrowser(chromium);
      await t.test("Hub guides regression gate stays visible and stable", async () => {
        const page = await browser.newPage();
        try {
          for (const viewport of [
            { width: 1440, height: 1100 },
            { width: 1180, height: 920 },
          ]) {
            await page.goto(environment.hubUrl, { waitUntil: "networkidle", timeout: 60_000 });
            try {
              await assertHubRegression(page, viewport);
            } catch (error) {
              await captureDesktopGuiArtifacts(page, {
                suite: "hub-shell-regression",
                scenario: "guides-gate",
                viewport,
                error,
              });
              throw error;
            }
          }
        } finally {
          await page.close();
        }
      });

      await t.test("Installer startup, integrity, updates, and logs mount cleanly", async () => {
        const page = await browser.newPage();
        try {
          for (const viewport of [
            { width: 1440, height: 1100 },
            { width: 1180, height: 920 },
          ]) {
            await page.goto(environment.installerUrl, { waitUntil: "networkidle", timeout: 60_000 });
            try {
              await assertInstallerRegression(page, viewport);
            } catch (error) {
              await captureDesktopGuiArtifacts(page, {
                suite: "installer-shell-regression",
                scenario: "startup-integrity-updates",
                viewport,
                error,
              });
              throw error;
            }
          }
        } finally {
          await page.close();
        }
      });
    } finally {
      await browser?.close();
      await environment.cleanup();
    }
  },
  { timeout: 180_000 },
);
