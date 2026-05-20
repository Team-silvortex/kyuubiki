import test from "node:test";
import {
  assertWorkbenchSampleUi,
  chromium,
  FRONTEND_URL,
  runKyuubiki,
  waitForFrontend,
} from "./workbench-ui-smoke.shared.mjs";

test(
  "Workbench can open representative mechanical samples and expose report/export actions",
  async () => {
    const browser = await chromium.launch({ headless: true });

    try {
      runKyuubiki(["restart-local"]);
      await waitForFrontend();

      const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
      await page.goto(FRONTEND_URL, { waitUntil: "networkidle", timeout: 60_000 });

      await assertWorkbenchSampleUi(page, "Mechanical", "Spring Grid 2D", "spring-grid-2d", "2D spring");
      await assertWorkbenchSampleUi(page, "Mechanical", "Spring Cage 3D", "spring-cage-3d", "3D spring");
      await assertWorkbenchSampleUi(page, "Mechanical", "Portal Frame 2D", "portal-frame-2d", "2D frame");
      await assertWorkbenchSampleUi(page, "Mechanical", "Quad Plate Patch 2D", "quad-plate-patch-2d", "2D plane quad");
    } finally {
      await browser.close();
      try {
        runKyuubiki(["stop"]);
      } catch {
        // keep cleanup best-effort for local integration runs
      }
    }
  },
  { timeout: 180_000 },
);
