import test from "node:test";
import {
  assertWorkbenchSampleUi,
  chromium,
  FRONTEND_URL,
  runKyuubiki,
  waitForFrontend,
} from "./workbench-ui-smoke.shared.mjs";

test(
  "Workbench can open representative thermal and thermo-mechanical samples and expose report/export actions",
  async () => {
    const browser = await chromium.launch({ headless: true });

    try {
      runKyuubiki(["restart-local"]);
      await waitForFrontend();

      const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
      await page.goto(FRONTEND_URL, { waitUntil: "networkidle", timeout: 60_000 });

      await assertWorkbenchSampleUi(page, "Thermal", "Heat Bar 1D", "Heat Bar 1D", "1D heat bar");
      await assertWorkbenchSampleUi(page, "Thermal", "Heat Plane Quad 2D", "Heat Plane Quad 2D", "2D heat plane quad");
      await assertWorkbenchSampleUi(page, "Thermo-mechanical", "Thermal Bar 1D", "Thermal Bar 1D", "1D thermal bar");
      await assertWorkbenchSampleUi(page, "Thermo-mechanical", "Thermal Plane Quad 2D", "Thermal Plane Quad 2D", "2D thermal plane quad");
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
