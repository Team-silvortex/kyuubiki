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

      await assertWorkbenchSampleUi(page, "thermal", "heat-bar-1d", "Heat Bar 1D", "Heat Bar 1D");
      await assertWorkbenchSampleUi(page, "thermal", "heat-plane-quad-2d", "Heat Plane Quad 2D", "Heat Plane Quad 2D");
      await assertWorkbenchSampleUi(page, "thermoMechanical", "thermal-bar-1d", "Thermal Bar 1D", "Thermal Bar 1D");
      await assertWorkbenchSampleUi(
        page,
        "thermoMechanical",
        "thermal-plane-quad-2d",
        "Thermal Plane Quad 2D",
        "Thermal Plane Quad 2D",
      );
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
