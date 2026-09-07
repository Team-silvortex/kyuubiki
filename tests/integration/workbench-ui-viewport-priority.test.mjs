import assert from "node:assert/strict";
import { test } from "node:test";
import { installProjectWorkbenchTestHooks, usingWorkbench, openWorkbench, invoke } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function checkViewport(page, label) {
  await page.waitForFunction(() => {
    const stage = document.querySelector('[data-workbench-viewport="stage"]');
    return parseFloat(stage?.style.getPropertyValue("--workbench-fit-width") || "0") > 0;
  });
  // Wait for the resize observer's next frame after toggling a dock.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const sizes = await page.evaluate(() => {
    const rect = (element) => {
      const { x, y, width, height } = element.getBoundingClientRect();
      return { x, y, width, height };
    };
    const stage = document.querySelector('[data-workbench-viewport="stage"]');
    const svg = stage.querySelector(".viewport-svg");
    return {
      stage: rect(stage), svg: rect(svg), panel: rect(document.querySelector('[data-workbench-panel="viewport"]')),
      frame: rect(svg.querySelector(".viewport-frame")), main: rect(document.querySelector(".workspace-main")),
      report: rect(document.querySelector(".console-panel")),
      horizontalOverflow: stage.scrollWidth - stage.clientWidth,
      verticalOverflow: stage.scrollHeight - stage.clientHeight,
    };
  });
  for (const [inner, outer] of [[sizes.svg, sizes.stage], [sizes.frame, sizes.stage], [sizes.stage, sizes.panel]]) {
    assert.ok(inner.x >= outer.x - 1 && inner.y >= outer.y - 1 &&
      inner.x + inner.width <= outer.x + outer.width + 1 &&
      inner.y + inner.height <= outer.y + outer.height + 1, `${label}: clipped geometry ${JSON.stringify(sizes)}`);
  }
  assert.ok(sizes.svg.width > 100 && sizes.svg.height > 50, `${label}: unusable surface`);
  assert.ok(Math.abs(sizes.svg.width / sizes.svg.height - 980 / 460) < 0.02, `${label}: distorted picking surface`);
  assert.ok(sizes.horizontalOverflow <= 1 && sizes.verticalOverflow <= 1, `${label}: viewport needs scrolling`);
  return sizes;
}

for (const [width, height] of [[1470, 820], [1280, 720], [1100, 620], [900, 900], [390, 844]]) {
  test(`Workbench prioritizes the full model at ${width}x${height}, including auxiliary expansion`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      await page.setViewportSize({ width, height });
      await openWorkbench(page);
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 4, span: 12, height: 2, loadY: -800 }));
      for (const section of ["model", "store", "system"]) {
        await invoke(page, "nav/setSidebarSection", { section });
        const sizes = await checkViewport(page, `${width} ${section}`);
        assert.ok(sizes.report.height <= 56, `Report stole the viewport: ${JSON.stringify(sizes)}`);
        assert.ok(sizes.stage.height >= sizes.main.height * 0.65, `Primary view is not dominant: ${JSON.stringify(sizes)}`);
      }
      const report = page.locator('[data-workbench-report-toggle="true"]');
      await report.click();
      assert.equal(await report.getAttribute("aria-expanded"), "true");
      await checkViewport(page, `${width} report open`);
      await report.click();
      const diagnostics = page.locator('[data-workbench-render-details="true"]');
      assert.equal(await diagnostics.getAttribute("open"), null);
      await diagnostics.locator("summary").click();
      await diagnostics.getByRole("button", { name: "Full", exact: true }).click();
      await checkViewport(page, `${width} diagnostics open`);
      await diagnostics.locator("summary").click();
      const heights = await page.locator('[data-workbench-inspector-tab-target]').evaluateAll(buttons => buttons.map(button => button.getBoundingClientRect().height));
      assert.ok(heights.every(value => value <= 60), `Inspector tabs stretched: ${heights}`);
    });
  });
}

test("Workbench fits axial, plane and 3D surfaces after live window resizing", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    for (const kind of ["axial_bar_1d", "heat_plane_quad_2d", "truss_3d"]) {
      await invoke(page, "nav/setStudyKind", { studyKind: kind });
      for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 620 }]) {
        await page.setViewportSize(viewport);
        await checkViewport(page, `${kind} ${viewport.width}`);
      }
    }
  });
});
