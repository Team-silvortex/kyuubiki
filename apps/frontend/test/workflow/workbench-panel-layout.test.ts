import assert from "node:assert/strict";
import test from "node:test";
import {
  parseWorkbenchPanelPreferences, resolveWorkbenchPanelLayout, resizeWorkbenchPanel,
} from "../../src/components/workbench/workbench-panel-layout.ts";
import { getWorkbenchPanelLayoutCopy } from "../../src/components/workbench/workbench-panel-layout-copy.ts";
import { buildWorkbenchLanguageOptions } from "../../src/components/workbench/workbench-language-options.ts";

test("panel preferences reject malformed, future and non-finite input without throwing", () => {
  for (const raw of [null, "{", "null", "[]", '{"version":2,"sizes":{"sidebar":300}}', "x".repeat(1025)]) {
    assert.deepEqual(parseWorkbenchPanelPreferences(raw), {});
  }
  assert.deepEqual(parseWorkbenchPanelPreferences('{"version":1,"sizes":{"sidebar":300,"inspector":-1,"report":"220","junk":400}}'), { sidebar: 300 });
  assert.deepEqual(parseWorkbenchPanelPreferences('{"version":1,"sizes":{"sidebar":1e999,"inspector":5001,"report":0}}'), {});
});

test("panel limits preserve primary canvas space across constrained and wide resolutions", () => {
  for (const width of [0, 390, 981, 1100, 1280, 1440, 1920, 3840]) {
    for (const height of [0, 340, 620, 820, 1440]) {
      const geometry = { width, height, gap: 8 };
      const result = resolveWorkbenchPanelLayout({ sidebar: 5000, inspector: 5000, report: 5000 }, geometry);
      const budget = Math.max(0, width - 94);
      assert.ok(result.sizes.sidebar + result.sizes.inspector + result.minimumMain <= budget + 0.001);
      assert.ok(result.sizes.report <= height * 0.45 + 0.001);
      for (const key of ["sidebar", "inspector", "report"] as const) {
        assert.ok(result.sizes[key] >= result.limits[key].min - 0.001);
        assert.ok(result.sizes[key] <= result.limits[key].max + 0.001);
        assert.ok(Number.isFinite(result.sizes[key]));
      }
    }
  }
});

test("window contraction never mutates saved preferred sizes", () => {
  const preferences = Object.freeze({ sidebar: 480, inspector: 390, report: 360 });
  const geometry = { width: 1920, height: 1000, gap: 8 };
  const wide = resolveWorkbenchPanelLayout(preferences, geometry);
  const narrow = resolveWorkbenchPanelLayout(preferences, { ...geometry, width: 1000, height: 620 });
  assert.ok(narrow.sizes.sidebar < wide.sizes.sidebar);
  assert.deepEqual(resolveWorkbenchPanelLayout(preferences, geometry), wide);
});

test("drag bounds resize just the requested panel and do not push its opposite panel", () => {
  const geometry = { width: 1470, height: 820, gap: 8 };
  const preferences = { sidebar: 260, inspector: 240, report: 180 };
  for (const panel of ["sidebar", "inspector", "report"] as const) {
    const maximum = resizeWorkbenchPanel(preferences, geometry, panel, 10000);
    assert.equal(maximum[panel], resolveWorkbenchPanelLayout(preferences, geometry).limits[panel].max);
    for (const other of ["sidebar", "inspector", "report"] as const) if (panel !== other) assert.equal(maximum[other], preferences[other]);
    const invalid = resizeWorkbenchPanel(preferences, geometry, panel, NaN);
    assert.equal(invalid[panel], preferences[panel]);
  }
});

test("invalid geometry is finite and workflow defaults leave room for its tools", () => {
  const geometry = { width: 1440, height: 900, gap: 8 };
  assert.ok(resolveWorkbenchPanelLayout({}, { ...geometry, workflow: true }).sizes.sidebar > resolveWorkbenchPanelLayout({}, geometry).sizes.sidebar);
  const result = resolveWorkbenchPanelLayout({}, { width: NaN, height: Infinity, gap: NaN });
  assert.ok(Object.values(result.sizes).every(Number.isFinite));
});

test("resize controls have translated copy for every offered built-in language", () => {
  const options = buildWorkbenchLanguageOptions({ copy: {}, languagePacks: [], currentLanguage: "en" });
  for (const option of options) {
    const copy = getWorkbenchPanelLayoutCopy(option.value);
    assert.ok(copy.resize && copy.reset);
    if (option.value !== "en") assert.notEqual(copy.reset, getWorkbenchPanelLayoutCopy("en").reset, option.value);
  }
});
