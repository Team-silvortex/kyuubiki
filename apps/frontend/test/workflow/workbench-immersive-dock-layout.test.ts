import assert from "node:assert/strict";
import { test } from "node:test";
import { resolveImmersiveDockLayout, resizeImmersiveDock } from "../../src/components/workbench/workbench-immersive-dock-layout";

test("immersive dock budgets remain finite and leave the majority of space for the canvas", () => {
  for (const width of [0, -1, NaN, Infinity, 320, 640, 1440, 7680]) {
    for (const height of [0, -1, NaN, Infinity, 180, 600, 1000]) {
      for (const stacked of [false, true]) {
        for (const preference of [-1, NaN, Infinity, 1, 1e9]) {
          const layout = resolveImmersiveDockLayout({ width: preference, height: preference }, { width, height, stacked });
          assert.ok([layout.min, layout.max, layout.size].every(Number.isFinite));
          assert.ok(layout.size >= layout.min && layout.size <= layout.max);
          const available = stacked ? height : width;
          if (Number.isFinite(available) && available >= 0) assert.ok(layout.max <= available * (stacked ? 0.45 : 0.4));
          if (!stacked) assert.ok(layout.max <= 460);
        }
      }
    }
  }
});

test("immersive dock clamps drag edges and restores separate width and height preferences", () => {
  const desktop = { width: 1440, height: 900, stacked: false };
  const portrait = { width: 640, height: 800, stacked: true };
  const preference = Object.freeze({ width: 400, height: 220 });
  const narrow = resolveImmersiveDockLayout(preference, { ...desktop, width: 800 });
  assert.ok(narrow.size < preference.width);
  assert.equal(resolveImmersiveDockLayout(preference, desktop).size, 400);
  assert.equal(resolveImmersiveDockLayout(preference, portrait).size, 220);
  const smaller = resizeImmersiveDock(preference, desktop, -1e6);
  assert.deepEqual(smaller, { width: resolveImmersiveDockLayout(preference, desktop).min, height: 220 });
  const larger = resizeImmersiveDock(preference, portrait, 1e6);
  assert.equal(larger.height, resolveImmersiveDockLayout(preference, portrait).max);
  assert.equal(larger.width, 400);
  assert.deepEqual(resizeImmersiveDock(preference, desktop, NaN), preference);
  assert.deepEqual(preference, { width: 400, height: 220 });
});

test("stacked dock reserves room for viewport chrome rather than counting it as canvas", () => {
  const geometry = { width: 640, height: 460, stacked: true, minMainSize: 280, gutter: 28 };
  assert.equal(resolveImmersiveDockLayout({ height: 1000 }, geometry).size, 152);
  assert.equal(resolveImmersiveDockLayout({}, { ...geometry, minMainSize: 1000 }).size, 0);
  assert.ok(Number.isFinite(resolveImmersiveDockLayout({}, { ...geometry, minMainSize: NaN }).size));
});
