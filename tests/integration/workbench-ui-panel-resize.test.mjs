import assert from "node:assert/strict";
import { test } from "node:test";
import { installProjectWorkbenchTestHooks, usingWorkbench, openWorkbench, invoke } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const key = "kyuubiki.workbench.panelLayout.v1";
const handle = (page, panel) => page.locator(`[data-workbench-resize="${panel}"]`);
const reset = page => page.locator('[data-workbench-layout-reset="true"]');
const report = page => page.locator('[data-workbench-report-toggle="true"]');
const settle = page => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));

test("navigation updates default panel widths even while animation frames are delayed", { timeout: 90_000 }, async () => {
  await usingWorkbench(async page => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await openWorkbench(page);
    await sizes(page);
    await page.evaluate(() => {
      const request = window.requestAnimationFrame.bind(window);
      const cancel = window.cancelAnimationFrame.bind(window);
      const pending = new Map();
      let id = 0;
      window.requestAnimationFrame = callback => { pending.set(--id, callback); return id; };
      window.cancelAnimationFrame = handle => { if (handle < 0) pending.delete(handle); else cancel(handle); };
      window.__restoreLayoutFrames = () => {
        window.requestAnimationFrame = request;
        window.cancelAnimationFrame = cancel;
        for (const callback of pending.values()) request(callback);
        pending.clear();
      };
    });
    try {
      for (const section of ["workflow", "model", "workflow"]) {
        await page.evaluate(value => window.__kyuubikiPwdt.openSidebar(value), section);
        await page.locator(`[data-workbench-shell="root"][data-workbench-section="${section}"]`).waitFor();
        const width = await page.locator(".workspace-sidebar").evaluate(element => element.getBoundingClientRect().width);
        assert.ok(section === "workflow" ? width >= 280 : width < 280, `${section} retained a stale width: ${width}`);
      }
      assert.equal(await page.evaluate(storageKey => localStorage.getItem(storageKey), key), null, "navigation must not save a user preference");
    } finally { await page.evaluate(() => window.__restoreLayoutFrames()); }
  });
});

async function sizes(page) {
  await page.waitForFunction(() => [
    '[data-workbench-panel="sidebar"]', '[data-workbench-panel="inspector"]', ".console-panel",
    '[data-workbench-viewport="stage"] .viewport-svg',
  ].every(selector => document.querySelector(selector)), undefined, { timeout: 15_000 });
  await settle(page);
  return page.evaluate(() => {
    const rect = selector => {
      const { x, y, width, height } = document.querySelector(selector).getBoundingClientRect();
      return { x, y, width, height };
    };
    return {
      sidebar: rect('[data-workbench-panel="sidebar"]'), inspector: rect('[data-workbench-panel="inspector"]'),
      report: rect(".console-panel"), stage: rect('[data-workbench-viewport="stage"]'),
      svg: rect('[data-workbench-viewport="stage"] .viewport-svg'), main: rect(".workspace-main"),
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  });
}

async function assertFits(page) {
  const current = await sizes(page);
  assert.ok(current.svg.width > 100 && current.svg.height > 50, JSON.stringify(current));
  for (const axis of ["x", "y"]) {
    const size = axis === "x" ? "width" : "height";
    assert.ok(current.svg[axis] >= current.stage[axis] - 1, JSON.stringify(current));
    assert.ok(current.svg[axis] + current.svg[size] <= current.stage[axis] + current.stage[size] + 1, JSON.stringify(current));
  }
  assert.ok(current.overflow <= 1, JSON.stringify(current));
  const tabBars = await page.locator(".inspector-stack .panel-tabs").evaluateAll(bars => bars.map(bar => bar.getBoundingClientRect().height));
  assert.ok(tabBars.every(height => height <= 140), `Inspector tab containers stretched: ${tabBars}`);
  return current;
}

async function beginDrag(page, panel, dx, dy = 0) {
  const box = await handle(page, panel).boundingBox();
  assert.ok(box?.height && box?.width, `${panel} splitter is invisible`);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy, { steps: 12 });
  await settle(page);
}

test("all three panel splitters resize real surfaces, persist on release and restore on reload", { timeout: 90_000 }, async () => {
  await usingWorkbench(async page => {
    await page.setViewportSize({ width: 1470, height: 900 });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 4, span: 12, height: 2 }));
    await page.evaluate(storageKey => {
      window.__layoutWrites = 0;
      const original = Storage.prototype.setItem;
      Storage.prototype.setItem = function (name, value) {
        if (name === storageKey) window.__layoutWrites++;
        return original.call(this, name, value);
      };
    }, key);
    const initial = await sizes(page);
    await beginDrag(page, "sidebar", 90);
    assert.equal(await page.evaluate(() => window.__layoutWrites), 0, "pointer moves must not write storage");
    await page.mouse.up();
    assert.equal(await page.evaluate(() => window.__layoutWrites), 1);
    assert.ok((await assertFits(page)).sidebar.width > initial.sidebar.width + 75);
    await beginDrag(page, "inspector", -60);
    await page.mouse.up();
    assert.ok((await assertFits(page)).inspector.width > initial.inspector.width + 45);
    await report(page).click();
    const opened = await sizes(page);
    await beginDrag(page, "report", 0, -80);
    await page.mouse.up();
    const resized = await assertFits(page);
    assert.ok(resized.report.height > opened.report.height + 65);
    assert.ok(resized.main.height - resized.report.height > resized.main.height * 0.5);
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await report(page).click();
    const restored = await assertFits(page);
    for (const panel of ["sidebar", "inspector", "report"]) {
      const dimension = panel === "report" ? "height" : "width";
      assert.ok(Math.abs(restored[panel][dimension] - resized[panel][dimension]) < 2, panel);
    }
    await reset(page).click();
    assert.equal(await page.evaluate(storageKey => localStorage.getItem(storageKey), key), null);
    assert.ok(Math.abs((await sizes(page)).sidebar.width - initial.sidebar.width) < 2);
  });
});

test("keyboard, double-click and window guards preserve preferred layout without clipping", { timeout: 90_000 }, async () => {
  await usingWorkbench(async page => {
    await page.setViewportSize({ width: 1600, height: 900 });
    await openWorkbench(page);
    await handle(page, "sidebar").focus();
    const before = await sizes(page);
    await page.keyboard.press("Shift+ArrowRight");
    assert.ok(Math.abs((await sizes(page)).sidebar.width - before.sidebar.width - 40) < 2);
    await handle(page, "inspector").press("End");
    await report(page).click();
    await handle(page, "report").press("End");
    const expanded = await assertFits(page);
    const saved = await page.evaluate(storageKey => localStorage.getItem(storageKey), key);
    for (const viewport of [{ width: 1100, height: 620 }, { width: 390, height: 844 }, { width: 1600, height: 900 }]) {
      await page.setViewportSize(viewport);
      await assertFits(page);
      assert.equal(await handle(page, "sidebar").isVisible(), viewport.width > 980);
      assert.equal(await page.evaluate(storageKey => localStorage.getItem(storageKey), key), saved);
    }
    assert.ok(Math.abs((await sizes(page)).inspector.width - expanded.inspector.width) < 2);
    await handle(page, "inspector").dblclick();
    const stored = await page.evaluate(storageKey => JSON.parse(localStorage.getItem(storageKey)), key);
    assert.equal(stored.sizes.inspector, undefined);
    assert.ok(stored.sizes.sidebar > 0 && stored.sizes.report > 0);
    await invoke(page, "nav/setSidebarSection", { section: "workflow" });
    assert.ok(Math.abs((await sizes(page)).sidebar.width - expanded.sidebar.width) < 2);
  });
});

test("Escape, pointer cancellation, window blur and resizing abort uncommitted drags", { timeout: 90_000 }, async () => {
  await usingWorkbench(async page => {
    await openWorkbench(page);
    await handle(page, "sidebar").press("ArrowRight");
    const before = await sizes(page);
    const saved = await page.evaluate(storageKey => localStorage.getItem(storageKey), key);
    for (const abort of ["escape", "cancel", "blur", "resize"]) {
      await beginDrag(page, "sidebar", 55);
      assert.ok((await sizes(page)).sidebar.width > before.sidebar.width + 40);
      if (abort === "escape") await page.keyboard.press("Escape");
      if (abort === "cancel") await handle(page, "sidebar").dispatchEvent("pointercancel", { pointerId: 1 });
      if (abort === "blur") await page.evaluate(() => window.dispatchEvent(new Event("blur")));
      if (abort === "resize") await page.setViewportSize({ width: 1440, height: 950 });
      await page.mouse.up();
      assert.equal(await page.locator('[data-workbench-shell="root"]').getAttribute("data-workbench-resizing"), null);
      const after = await assertFits(page);
      assert.ok(Math.abs(after.sidebar.width - before.sidebar.width) < 2, `${abort}: ${before.sidebar.width} -> ${after.sidebar.width}`);
      assert.equal(await page.evaluate(storageKey => localStorage.getItem(storageKey), key), saved, abort);
    }
  });
});

test("invalid saved layouts and denied storage leave the workbench usable", { timeout: 90_000 }, async () => {
  await usingWorkbench(async page => {
    await page.addInitScript(storageKey => localStorage.setItem(storageKey, '{"version":1,"sizes":{"sidebar":1e999,"report":-200}}'), key);
    await openWorkbench(page);
    const before = await assertFits(page);
    await page.evaluate(storageKey => {
      const original = Storage.prototype.setItem;
      Storage.prototype.setItem = function (name, value) {
        if (name === storageKey) throw new DOMException("Storage denied", "QuotaExceededError");
        return original.call(this, name, value);
      };
    }, key);
    await handle(page, "sidebar").press("Shift+ArrowRight");
    assert.equal(await page.locator('[data-workbench-shell="root"]').getAttribute("data-workbench-layout-storage"), "memory");
    await invoke(page, "nav/setSidebarSection", { section: "store" });
    assert.ok(Math.abs((await assertFits(page)).sidebar.width - before.sidebar.width - 40) < 2);
    await reset(page).click();
    assert.ok(Math.abs((await sizes(page)).sidebar.width - before.sidebar.width) < 2);
  });
});
