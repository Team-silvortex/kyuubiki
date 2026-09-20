import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";
import { ROOT } from "./workbench-ui-isolated.shared.mjs";

installProjectWorkbenchTestHooks();

async function openPwdt(page) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.locator('[data-workbench-system-surface-tab="settings"]').click();
  await page.locator('[data-workbench-system-settings-page="scripts"]').click();
  await page.locator('[data-workbench-pwdt="workspace"]').waitFor();
}
const tab = (page, name) => page.locator(`[data-workbench-pwdt-page="${name}"]`);
const panel = (page, name) => page.locator(`[data-workbench-pwdt-content="${name}"]`);
const python = (page) => panel(page, "script").locator("textarea");

test("PWDT starts with one editor and preserves Python, DSL and SDK drafts across subpages", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openPwdt(page);
    assert.equal(await page.locator('[data-workbench-pwdt-content]:visible').count(), 1);
    assert.equal(await panel(page, "headless").locator("input").count(), 0, "SDK builder does not mount on startup");
    assert.equal(await panel(page, "catalog").locator("button").count(), 0, "catalog does not mount on startup");
    await python(page).fill("ky.log('retained Python')");
    await tab(page, "dsl").click();
    await panel(page, "dsl").locator("textarea").fill('{"draft":"retained DSL"}');
    await tab(page, "headless").click();
    const sdkName = panel(page, "headless").locator('input[type="text"]').first();
    await sdkName.fill("macro/retained-sdk-draft");
    await tab(page, "record").click();
    await panel(page, "record").getByRole("button", { name: "Start recording", exact: true }).click();
    await tab(page, "inspect").click();
    await page.locator(".pwdt-workspace__notice").getByRole("button", { name: "Stop recording", exact: true }).waitFor();
    await panel(page, "inspect").getByRole("button", { name: "Timeline", exact: true }).waitFor();
    await tab(page, "script").click();
    assert.equal(await python(page).inputValue(), "ky.log('retained Python')");
    await tab(page, "dsl").click();
    assert.equal(await panel(page, "dsl").locator("textarea").inputValue(), '{"draft":"retained DSL"}');
    await tab(page, "headless").click();
    assert.equal(await sdkName.inputValue(), "macro/retained-sdk-draft");
    await tab(page, "record").click();
    await panel(page, "record").getByRole("button", { name: "Stop recording", exact: true }).click();
    assert.equal(await page.locator('[data-workbench-pwdt-content]:visible').count(), 1);
    assert.equal(await page.locator(".pwdt-workspace__notice button").count(), 0);
  });
});

test("PWDT embedded editor handles indent, outdent, undo, newline and keyboard escape", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openPwdt(page);
    const editor = python(page);
    await editor.fill("x");
    await editor.press("End");
    await editor.press("Tab");
    assert.equal(await editor.inputValue(), "x   ");
    await editor.press("ControlOrMeta+z");
    assert.equal(await editor.inputValue(), "x");
    await editor.press("ControlOrMeta+Shift+z");
    assert.equal(await editor.inputValue(), "x   ");
    await editor.fill("alpha\nbeta\ngamma");
    await editor.evaluate((input) => input.setSelectionRange(0, 11));
    await editor.press("Tab");
    assert.equal(await editor.inputValue(), "    alpha\n    beta\ngamma");
    await editor.press("Shift+Tab");
    assert.equal(await editor.inputValue(), "alpha\nbeta\ngamma");
    await editor.fill("if True:");
    await editor.press("End");
    await editor.press("Enter");
    assert.equal(await editor.inputValue(), "if True:\n    ");
    await editor.press("Escape");
    await editor.press("Tab");
    assert.equal(await editor.evaluate((input) => document.activeElement === input), false);
    await tab(page, "script").focus();
    await tab(page, "script").press("ArrowRight");
    assert.equal(await tab(page, "dsl").getAttribute("aria-selected"), "true");
  });
});

test("PWDT catalog insertion and DSL compilation lead back to the Python editor", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openPwdt(page);
    await python(page).fill("# research draft");
    await tab(page, "catalog").click();
    await panel(page, "catalog").getByRole("button", { name: "Actions", exact: true }).click();
    await panel(page, "catalog").getByRole("button", { name: "Insert", exact: true }).first().click();
    assert.equal(await tab(page, "script").getAttribute("aria-selected"), "true");
    assert.match(await python(page).inputValue(), /# research draft[\s\S]*await ky\.invoke/u);
    assert.equal(await python(page).evaluate((input) => document.activeElement === input), true);
    await tab(page, "dsl").click();
    await panel(page, "dsl").getByRole("button", { name: "Compile to script", exact: true }).click();
    assert.equal(await tab(page, "script").getAttribute("aria-selected"), "true");
    const compiled = await python(page).inputValue();
    assert.equal(await python(page).evaluate((input) => document.activeElement === input), true);
    await tab(page, "dsl").click();
    await panel(page, "dsl").locator("textarea").fill("{broken");
    await panel(page, "dsl").getByRole("button", { name: "Compile to script", exact: true }).click();
    assert.equal(await tab(page, "dsl").getAttribute("aria-selected"), "true");
    assert.equal(await python(page).inputValue(), compiled, "invalid DSL never overwrites Python");
  });
});

test("PWDT keyboard run shares the runtime lock with DSL and keeps output in the editor page", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    // Exercise UI dispatch and concurrency, not Python or WASM computation.
    await page.route("https://cdn.jsdelivr.net/pyodide/**/pyodide.js", (route) => route.fulfill({
      contentType: "application/javascript",
      body: `window.loadPyodide = async () => ({ runPythonAsync: async source => {
        window.pwdtSources = [...(window.pwdtSources || []), source];
        await new Promise(resolve => { window.releasePwdtRun = resolve; });
        window.__kyuubikiBridge.log("workspace execution complete");
      } });`,
    }));
    await openPwdt(page);
    await python(page).fill("ky.log('keyboard run')");
    await python(page).press("ControlOrMeta+Enter");
    await page.waitForFunction(() => typeof window.releasePwdtRun === "function");
    await python(page).press("ControlOrMeta+Enter");
    await tab(page, "dsl").click();
    assert.equal(await panel(page, "dsl").getByRole("button", { name: "Run DSL", exact: true }).isDisabled(), true);
    await panel(page, "dsl").locator("textarea").press("ControlOrMeta+Enter");
    assert.equal(await page.evaluate(() => window.pwdtSources.length), 1);
    await page.evaluate(() => window.releasePwdtRun());
    await tab(page, "script").click();
    await panel(page, "script").locator(".status-chip--good").waitFor();
    assert.match(await panel(page, "script").innerText(), /workspace execution complete/u);
    assert.match(await page.evaluate(() => window.pwdtSources[0]), /keyboard run/u);
  });
});

test("PWDT native fullscreen restores its panel and window fallback cleans up on unmount", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openPwdt(page);
    await python(page).fill("# fullscreen draft");
    await page.locator('[data-workbench-pwdt-expand]').click();
    await page.waitForFunction(() => document.fullscreenElement?.matches('[data-workbench-pwdt="workspace"]'));
    await page.locator('[data-workbench-pwdt-expand]').click();
    await page.waitForFunction(() => !document.fullscreenElement
      && document.querySelector('[data-workbench-pwdt="workspace"]')?.dataset.expanded === "false");
    assert.equal(await python(page).inputValue(), "# fullscreen draft");
    await page.evaluate(() => {
      Object.defineProperty(HTMLElement.prototype, "requestFullscreen", { configurable: true, value: undefined });
      Object.defineProperty(HTMLElement.prototype, "webkitRequestFullscreen", { configurable: true, value: undefined });
    });
    await page.locator('[data-workbench-pwdt-expand]').click();
    await page.locator('[data-workbench-window-fullscreen]').waitFor();
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    await page.locator('[data-workbench-pwdt="workspace"]').waitFor({ state: "detached" });
    assert.equal(await page.locator("[inert], [data-workbench-fullscreen-ancestor], [data-workbench-window-fullscreen]").count(), 0);
  });
});

for (const viewport of [{ width: 1440, height: 1000 }, { width: 1024, height: 700 }, { width: 390, height: 844 }]) {
test(`PWDT panel and expanded editor stay bounded at ${viewport.width}px`, { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    await page.setViewportSize(viewport);
    await openPwdt(page);
    // Tauri's window-immersive fallback must work without a native fullscreen API.
    await page.evaluate(() => {
      Object.defineProperty(HTMLElement.prototype, "requestFullscreen", { configurable: true, value: undefined });
      Object.defineProperty(HTMLElement.prototype, "webkitRequestFullscreen", { configurable: true, value: undefined });
    });
    const root = page.locator('[data-workbench-pwdt="workspace"]');
    await python(page).fill("ky.log('expand without losing this draft')");
    for (const expanded of [false, true]) {
      if (expanded) await page.locator('[data-workbench-pwdt-expand]').click();
      await page.waitForFunction((value) => document.querySelector('[data-workbench-pwdt="workspace"]')?.dataset.expanded === String(value), expanded);
      const dimensions = await root.evaluate((element) => {
        const tabs = element.querySelector('[role="tablist"]');
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        const ancestors = [];
        for (let parent = element.parentElement; parent; parent = parent.parentElement) {
          const computed = getComputedStyle(parent);
          ancestors.push({ name: parent.className, contain: computed.contain, transform: computed.transform,
            contentVisibility: computed.contentVisibility, filter: computed.filter, backdrop: computed.backdropFilter,
            willChange: computed.willChange, containerType: computed.containerType });
        }
        return { width: rect.width, height: rect.height, scroll: element.scrollWidth, client: element.clientWidth,
          tabScroll: tabs.scrollWidth, tabClient: tabs.clientWidth, right: rect.right, bottom: rect.bottom,
          position: style.position, inset: style.inset, expanded: element.getAttribute("data-workbench-window-fullscreen"), ancestors };
      });
      assert.ok(dimensions.height > 100, JSON.stringify(dimensions));
      assert.ok(dimensions.scroll <= dimensions.client + 2, JSON.stringify(dimensions));
      assert.ok(dimensions.tabScroll <= dimensions.tabClient + 2, JSON.stringify(dimensions));
      assert.ok(dimensions.right <= viewport.width + 2, JSON.stringify(dimensions));
      if (expanded) {
        assert.ok(dimensions.width >= viewport.width - 30, JSON.stringify(dimensions));
        assert.ok(dimensions.bottom <= viewport.height + 2, JSON.stringify(dimensions));
      }
      await page.screenshot({ path: `${ROOT}/tmp/pwdt-workspace-${viewport.width}-${expanded ? "expanded" : "panel"}.png` });
    }
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => document.querySelector('[data-workbench-pwdt="workspace"]')?.dataset.expanded === "false");
    assert.equal(await python(page).inputValue(), "ky.log('expand without losing this draft')");
    assert.equal(await page.locator("[data-workbench-fullscreen-ancestor]").count(), 0);
    assert.equal(await page.locator("[inert]").count(), 0);
  });
});
}
