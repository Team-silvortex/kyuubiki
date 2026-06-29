import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkbenchResolutionStyleVars,
  resolveWorkbenchResolutionAdaptation,
  resolveWorkbenchViewportProfile,
  resolveWorkbenchWindowMode,
} from "../../src/components/workbench/workbench-resolution-adaptation.ts";

test("workbench resolution modes separate desktop, tablet, and phone breakpoints", () => {
  assert.equal(resolveWorkbenchWindowMode(1600), "standard");
  assert.equal(resolveWorkbenchWindowMode(1200), "compact");
  assert.equal(resolveWorkbenchWindowMode(900), "narrow");
  assert.equal(resolveWorkbenchWindowMode(520), "ultranarrow");

  assert.equal(resolveWorkbenchViewportProfile(1600), "desktop");
  assert.equal(resolveWorkbenchViewportProfile(1200), "compact");
  assert.equal(resolveWorkbenchViewportProfile(900), "tablet");
  assert.equal(resolveWorkbenchViewportProfile(520), "phone");
});

test("phone workbench layout stacks panels and reserves bottom UI space", () => {
  const adaptation = resolveWorkbenchResolutionAdaptation({ width: 520, height: 820 });

  assert.equal(adaptation.windowMode, "ultranarrow");
  assert.equal(adaptation.profile, "phone");
  assert.equal(adaptation.shouldStackPanels, true);
  assert.equal(adaptation.shouldUseScrollableShell, true);
  assert.equal(adaptation.minTouchTargetPx, 44);
  assert.equal(adaptation.bottomSafeAreaPx, 84);
});

test("short desktop windows keep compact chrome and safe bottom clearance", () => {
  const adaptation = resolveWorkbenchResolutionAdaptation({ width: 1280, height: 620 });
  const styleVars = buildWorkbenchResolutionStyleVars(adaptation);

  assert.equal(adaptation.windowMode, "compact");
  assert.equal(adaptation.profile, "compact");
  assert.equal(adaptation.shouldCompactChrome, true);
  assert.equal(adaptation.shouldStackPanels, false);
  assert.equal(adaptation.shouldUseScrollableShell, true);
  assert.equal(styleVars["--workbench-bottom-safe-area"], "72px");
});
