import assert from "node:assert/strict";
import test from "node:test";
import { fitWorkbenchViewport } from "../../src/components/workbench/workbench-viewport-fit.ts";
import { resolveWorkbenchResolutionAdaptation } from "../../src/components/workbench/workbench-resolution-adaptation.ts";

for (const [width, height] of [[1000, 700], [540, 280], [350, 600], [1800, 240], [0, 240]]) {
  test(`viewport surface fits ${width}x${height} without clipping or aspect distortion`, () => {
    const fit = fitWorkbenchViewport(width, height);
    assert.ok(fit.width <= width && fit.height <= height);
    assert.ok(fit.width >= 0 && fit.height >= 0);
    if (fit.width > 0) assert.ok(Math.abs(fit.width / fit.height - 980 / 460) < 1e-10);
  });
}

test("unmeasured or invalid viewport geometry remains finite", () => {
  for (const [width, height] of [[NaN, 10], [Infinity, 400], [200, -1]]) {
    assert.deepEqual(fitWorkbenchViewport(width, height), { width: 0, height: 0 });
  }
});

test("stacked tablets scroll, while short desktop windows retain a bounded main viewport", () => {
  const tablet = resolveWorkbenchResolutionAdaptation({ width: 900, height: 900 });
  assert.equal(tablet.shouldStackPanels, true);
  assert.equal(tablet.shouldUseScrollableShell, true);
  const desktop = resolveWorkbenchResolutionAdaptation({ width: 1280, height: 620 });
  assert.equal(desktop.shouldStackPanels, false);
  assert.equal(desktop.shouldUseScrollableShell, false);
  assert.equal(resolveWorkbenchResolutionAdaptation({ width: 1280, height: 360 }).shouldUseScrollableShell, true);
});
