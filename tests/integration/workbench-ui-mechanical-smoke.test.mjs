import test from "node:test";
import {
  assertWorkbenchSampleUi,
  chromium,
  FRONTEND_URL,
  startWorkbenchIntegrationRuntime,
  stopWorkbenchIntegrationRuntime,
  waitForFrontend,
} from "./workbench-ui-smoke.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

test(
  "Workbench can open representative mechanical samples and expose report/export actions",
  async () => {
    const browser = await launchIntegrationBrowser(chromium);

    try {
      startWorkbenchIntegrationRuntime();
      await waitForFrontend();

      const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
      await page.goto(FRONTEND_URL, { waitUntil: "networkidle", timeout: 60_000 });

      await assertWorkbenchSampleUi(page, "mechanical", "spring-grid-2d", "Spring Grid 2D", "spring-grid-2d");
      await assertWorkbenchSampleUi(page, "mechanical", "spring-cage-3d", "Spring Cage 3D", "spring-cage-3d");
      await assertWorkbenchSampleUi(page, "mechanical", "portal-frame-2d", "Portal Frame 2D", "portal-frame-2d");
      await assertWorkbenchSampleUi(page, "mechanical", "quad-plate-patch-2d", "Quad Plate Patch 2D", "quad-plate-patch-2d");
    } finally {
      await browser.close();
      try {
        stopWorkbenchIntegrationRuntime();
      } catch {
        // keep cleanup best-effort for local integration runs
      }
    }
  },
  { timeout: 180_000 },
);
