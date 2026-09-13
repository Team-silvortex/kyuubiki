import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, holdRequest, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const field = (page, name) => page.locator(`[data-model-batch="${name}"]`);
const action = (page, name) => page.locator(`[data-workbench-immersive="${name}"]`);
const tab = (page, name) => page.locator(`[data-workbench-immersive-tab="${name}"]`);
const model = () => ({
  nodes: Array.from({ length: 6 }, (_, x) => ({ id: `node-${x}`, x, y: 0, z: 0,
    fix_x: x === 0, fix_y: x === 0, fix_z: x === 0, load_x: 0, load_y: 0, load_z: 0 })),
  elements: Array.from({ length: 5 }, (_, node_i) => ({ id: `member-${node_i}`, node_i, node_j: node_i + 1,
    area: 0.01, youngs_modulus: 70e9 })),
});

async function enterFullscreen(page) {
  await action(page, "toggle").click();
  await page.waitForFunction(() => document.fullscreenElement?.matches('[data-workbench-panel="viewport"]') &&
    window.__kyuubikiPwdt.state().immersiveViewport);
}

async function savedPayload(page, library, name) {
  await page.evaluate((name) => window.__kyuubikiPwdt.saveModel({ name, saveAs: true }), name);
  return library.versions.at(-1).payload;
}

test("WebView without fullscreen APIs uses a reversible window-local modeling workspace", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    const viewport = page.locator('[data-workbench-panel="viewport"]');
    await viewport.evaluate((panel) => {
      Object.defineProperty(panel, "requestFullscreen", { value: undefined, configurable: true });
      Object.defineProperty(panel, "webkitRequestFullscreen", { value: undefined, configurable: true });
    });
    const initialInert = await page.locator("[inert]").count();
    await action(page, "toggle").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().immersiveViewport);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().sidebarSection), "model");
    assert.equal(await page.locator(".immersive-drawer").count(), 0);
    assert.equal(await page.evaluate(() => document.fullscreenElement), null);
    assert.equal(await viewport.getAttribute("data-workbench-window-fullscreen"), "true");
    await action(page, "model").click();
    for (const [width, height] of [[1324, 768], [640, 600]]) {
      await page.setViewportSize({ width, height });
      await page.waitForFunction(() => {
        const rect = document.querySelector('[data-workbench-panel="viewport"]').getBoundingClientRect();
        return rect.width === innerWidth && Math.abs(rect.height - innerHeight) <= 1;
      });
      const box = await viewport.boundingBox();
      assert.equal(box.x, 0);
      assert.equal(box.y, 0);
      const apply = await field(page, "apply").boundingBox();
      assert.ok(apply.y >= 0 && apply.y + apply.height <= height);
    }
    assert.equal(await page.locator('.workbench-shell__sidebar').getAttribute("inert"), "");
    await tab(page, "save").click();
    const name = page.locator('[data-workbench-immersive-save="name"]');
    await name.fill("native keyboard acceptance");
    await name.press(process.platform === "darwin" ? "Meta+a" : "Control+a");
    await name.press("Backspace");
    assert.equal(await name.inputValue(), "", "immersive guardrails cannot block form Select All");
    assert.equal(await name.evaluate((field) => {
      const event = new Event("paste", { bubbles: true, cancelable: true });
      return field.dispatchEvent(event);
    }), true, "form clipboard events must not be cancelled");
    await name.fill("name after empty guard");
    await action(page, "toggle").focus();
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => !window.__kyuubikiPwdt.state().immersiveViewport);
    assert.equal(await page.locator("[data-workbench-fullscreen-ancestor]").count(), 0);
    assert.equal(await page.locator("[inert]").count(), initialInert);
    await action(page, "toggle").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().immersiveViewport);
    await action(page, "library").click();
    await page.waitForFunction(() => !window.__kyuubikiPwdt.state().immersiveViewport &&
      window.__kyuubikiPwdt.state().sidebarSection === "library");
    assert.equal(await page.locator("[inert]").count(), initialInert);
    await action(page, "toggle").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().immersiveViewport);
    await invoke(page, "nav/setStudyKind", { studyKind: "truss_2d" });
    await page.waitForFunction(() => !window.__kyuubikiPwdt.state().immersiveViewport);
    assert.equal(await page.locator("[inert]").count(), initialInert);
    assert.equal(await page.locator("[data-workbench-window-fullscreen]").count(), 0);
  });
});

test("real fullscreen shares batch drafts, geometry edits and undo/redo with sidebar and PWDT", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ modelTab: "tools", modelToolsPage: "studio" }));
    await field(page, "toggle").click();
    await field(page, "query").selectOption("indices");
    await field(page, "indices").fill("1-3");
    await field(page, "operation").selectOption("rotate");
    await field(page, "pivot").selectOption("origin");
    await field(page, "angle").fill("90");
    await enterFullscreen(page);
    await action(page, "model").click();
    await field(page, "editor").waitFor({ state: "visible" });
    assert.equal(await field(page, "editor").count(), 1, "no second hidden batch editor may remain mounted");
    assert.equal(await field(page, "indices").inputValue(), "1-3");
    assert.equal(await field(page, "operation").inputValue(), "rotate");
    await field(page, "apply").click();
    await field(page, "status").waitFor({ state: "visible" });
    const rotated = await savedPayload(page, library, "immersive-rotated");
    assert.deepEqual(rotated.nodes.slice(1, 4).map((n) => [n.x, n.y, n.z]), [[0, 1, 0], [0, 2, 0], [0, 3, 0]]);
    await action(page, "undo").click();
    assert.deepEqual((await savedPayload(page, library, "immersive-undone")).nodes, model().nodes);
    await action(page, "redo").click();
    assert.deepEqual((await savedPayload(page, library, "immersive-redone")).nodes, rotated.nodes);
    await invoke(page, "viewport/setUiState", { toolTab: "props", toolDrawerOpen: true });
    assert.equal(await field(page, "editor").count(), 0, "inactive editors unmount");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().immersiveToolTab), "props");
    await tab(page, "batch").click();
    assert.equal(await field(page, "angle").inputValue(), "90");
    await field(page, "operation").selectOption("delete");
    await field(page, "confirm-delete").check();
    await action(page, "toggle-tools").click();
    assert.equal(await field(page, "editor").count(), 0);
    await action(page, "model").click();
    assert.equal(await field(page, "operation").inputValue(), "delete");
    assert.equal(await field(page, "confirm-delete").isChecked(), false, "deletion consent must never survive remounts");
    await field(page, "operation").selectOption("array");
    await field(page, "vector-z").fill("2.5");
    await invoke(page, "viewport/setUiState", { immersiveViewport: false });
    await page.waitForFunction(() => !document.fullscreenElement && !window.__kyuubikiPwdt.state().immersiveViewport);
    await field(page, "toggle").click();
    assert.equal(await field(page, "vector-z").inputValue(), "2.5");
    assert.equal(await field(page, "indices").inputValue(), "1-3");
  });
});

test("fullscreen study and multi-node controls are live; resize keeps canvas and Apply reachable", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    await invoke(page, "selection/set3d", { nodeIndices: [1, 2, 3], anchorNodeIndex: 1 });
    await enterFullscreen(page);
    await tab(page, "props").click();
    const propsPanel = page.locator('[data-workbench-immersive-panel="props"]');
    await propsPanel.getByRole("button", { name: "Fix Z", exact: true }).click();
    const supported = await savedPayload(page, library, "immersive-multi-support");
    assert.ok(supported.nodes.slice(1, 4).every((node) => node.fix_z));
    await action(page, "undo").click();
    assert.ok((await savedPayload(page, library, "immersive-multi-undo")).nodes.slice(1, 4).every((node) => !node.fix_z));
    await propsPanel.getByRole("button", { name: "Y+", exact: true }).click();
    assert.ok((await savedPayload(page, library, "immersive-nudge")).nodes.slice(1, 4).every((node) => node.y > 0));
    await action(page, "undo").click();
    await tab(page, "node").click();
    const nodePanel = page.locator('[data-workbench-immersive-panel="node"]');
    await nodePanel.getByRole("button", { name: "Duplicate", exact: true }).click();
    const copied = await savedPayload(page, library, "immersive-quick-copy");
    assert.equal(copied.nodes.length, 9);
    assert.equal(copied.elements.length, 7);
    assert.deepEqual(copied.nodes.slice(6).map((node) => [node.x, node.y, node.z]), [[1.4, 0.2, 0.4], [2.4, 0.2, 0.4], [3.4, 0.2, 0.4]]);
    await action(page, "undo").click();
    await invoke(page, "selection/set3d", { nodeIndices: [2], anchorNodeIndex: 2 });
    await tab(page, "props").click();
    const coordinate = propsPanel.locator('input[type="number"]').first();
    await coordinate.fill("2.25");
    assert.equal((await savedPayload(page, library, "immersive-single-edit")).nodes[2].x, 2.25);
    await action(page, "undo").click();
    assert.equal((await savedPayload(page, library, "immersive-single-undo")).nodes[2].x, 2);
    await invoke(page, "selection/set3d", { nodeIndices: [1, 2, 3], anchorNodeIndex: 1 });
    await action(page, "study").click();
    assert.equal(await page.locator('[data-workbench-model-study-kind="select"]').count(), 1);
    assert.equal(await page.locator('[data-workbench-model-study-run="true"]').isVisible(), true);
    await action(page, "model").click();
    await field(page, "operation").selectOption("rotate");
    await field(page, "pivot").selectOption("point");
    await field(page, "create-copy").check();
    for (const [width, height] of [[1440, 1000], [1024, 768], [800, 600], [640, 800]]) {
      await page.setViewportSize({ width, height });
      await page.waitForFunction(() => {
        const stage = document.querySelector('[data-workbench-viewport="stage"]')?.getBoundingClientRect();
        const apply = document.querySelector('[data-model-batch="apply"]')?.getBoundingClientRect();
        return stage?.height > 100 && apply?.bottom <= innerHeight;
      });
      const canvas = await page.locator('[data-workbench-viewport="stage"]').boundingBox();
      const apply = await field(page, "apply").boundingBox();
      assert.ok(apply && apply.y >= 0 && apply.y + apply.height <= height, `Apply must stay visible at ${width}x${height}`);
      assert.ok(canvas && canvas.y + canvas.height <= height + 1);
      assert.ok(width > 700 ? canvas.width >= width * 0.55 : canvas.height >= height * 0.4,
        `canvas must retain priority at ${width}x${height}: ${JSON.stringify(canvas)}`);
      assert.equal(await field(page, "fields").evaluate((el) => el.scrollWidth <= el.clientWidth + 1), true);
      const readout = await page.locator('[data-workbench-3d-readout="true"]').evaluate((el) => {
        const box = el.getBoundingClientRect();
        const svg = el.closest("svg"), clip = svg.querySelector("clipPath rect");
        const toLocal = svg.getScreenCTM().inverse();
        return {
          top: new DOMPoint(box.left, box.top).matrixTransform(toLocal).y,
          bottom: new DOMPoint(box.right, box.bottom).matrixTransform(toLocal).y,
          clipTop: Number(clip.getAttribute("y")),
          clipBottom: Number(clip.getAttribute("y")) + Number(clip.getAttribute("height")),
        };
      });
      assert.ok(readout.bottom <= readout.clipBottom + 1 && readout.top >= readout.clipTop - 1,
        `selection readout cannot be clipped by the viewport: ${JSON.stringify(readout)}`);
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    if (process.env.KYUUBIKI_IMMERSIVE_SCREENSHOT) await page.screenshot({ path: process.env.KYUUBIKI_IMMERSIVE_SCREENSHOT });
    await action(page, "study").click();
    await page.locator('[data-workbench-model-study-kind="select"]').selectOption("truss_2d");
    await page.waitForFunction(() => !document.fullscreenElement && !window.__kyuubikiPwdt.state().immersiveViewport);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ modelTab: "tools", modelToolsPage: "studio" }));
    await field(page, "toggle").click();
    assert.equal(await field(page, "operation").inputValue(), "translate", "different study kinds start with compatible drafts");
    assert.equal(await field(page, "vector-z").count(), 0);
  });
});

test("fullscreen refusal is observable in PWDT and UI without false success or mutations", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    await page.locator('[data-workbench-panel="viewport"]').evaluate((panel) => {
      panel.requestFullscreen = async () => { throw new Error("test fullscreen denied"); };
    });
    await assert.rejects(() => invoke(page, "viewport/setUiState", { immersiveViewport: true, toolTab: "batch" }), /test fullscreen denied/);
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().immersiveToolTab), "node");
    await action(page, "toggle").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "test fullscreen denied");
    assert.equal(await page.evaluate(() => Boolean(document.fullscreenElement)), false);
    assert.equal(await field(page, "editor").count(), 0);
  });
});

test("fullscreen query selection reaches actual nodes and clears without editing the model", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    await enterFullscreen(page);
    await action(page, "model").click();
    const undoEnabled = await action(page, "undo").isEnabled();
    await field(page, "query").selectOption("range");
    await field(page, "query-axis").selectOption("x");
    await field(page, "min").fill("1");
    await field(page, "max").fill("3");
    await field(page, "invert").check();
    await field(page, "select").click();
    assert.deepEqual(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices), [0, 4, 5]);
    assert.equal(await field(page, "query").inputValue(), "current");
    assert.equal(await field(page, "invert").isChecked(), false, "the newly selected set must not be inverted again by Apply");
    assert.equal(await action(page, "undo").isEnabled(), undoEnabled);
    await field(page, "clear-selection").click();
    const cleared = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.deepEqual(cleared.selectedTruss3dNodeIndices, []);
    assert.equal(cleared.selectedNode, null);
    assert.deepEqual(cleared.memberDraftNodeIndices, []);
    assert.equal(await field(page, "apply").isDisabled(), true);
    assert.deepEqual(await invoke(page, "selection/query3d", { query: { kind: "indices", indices: [5, 1, 5] } }),
      { ok: true, action: "selection/query3d", selectedNodes: 2 });
    await assert.rejects(() => invoke(page, "selection/query3d", { query: { kind: "indices", indices: [99] } }), /invalid_indices/);
    assert.deepEqual(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices), [1, 5]);
    assert.deepEqual((await savedPayload(page, library, "Selection is not an edit")).nodes, model().nodes);
    assert.deepEqual(library.versions.at(-1).payload.elements, model().elements);
  });
});

const saveField = (page, name) => page.locator(`[data-workbench-immersive-save="${name}"]`);
async function waitForSave(page, failed = false) {
  await page.waitForFunction((failed) => {
    const panel = document.querySelector('[data-workbench-immersive-save="panel"]');
    const status = document.querySelector('[data-workbench-immersive-save="status"]')?.textContent?.toLowerCase() ?? "";
    return panel?.getAttribute("aria-busy") === "false" && status.includes(failed ? "failed" : "completed");
  }, failed);
}

test("fullscreen save and save-as persist the live model, recover from errors and lock across tab remounts", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    await enterFullscreen(page);
    await action(page, "save").click();
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().immersiveToolTab), "save");
    await saveField(page, "name").fill("  Fullscreen research  ");
    await saveField(page, "save").click();
    await waitForSave(page);
    assert.equal(library.models.length, 1);
    const modelId = library.models[0].model_id;
    assert.equal(library.models[0].name, "Fullscreen research");
    assert.equal(await saveField(page, "name").inputValue(), "Fullscreen research");
    assert.deepEqual(library.versions[0].payload.nodes, model().nodes);
    await action(page, "model").click();
    await field(page, "query").selectOption("all");
    await field(page, "vector-y").fill("2");
    await field(page, "apply").click();
    await action(page, "save").click();
    await saveField(page, "name").fill("Revised research");
    library.failVersions = true;
    await saveField(page, "save").click();
    await waitForSave(page, true);
    assert.equal(library.versions.length, 1, "a failed version write cannot be presented as a saved version");
    assert.equal(library.models[0].name, "Fullscreen research", "failed checkpoints must not rename stored models");
    assert.deepEqual(library.models[0].payload.nodes, model().nodes, "failed checkpoints must not overwrite stored geometry");
    assert.equal(library.writes.filter((write) => write.method === "PATCH").length, 0,
      "saving uses the backend atomic checkpoint, never a preceding metadata write");
    assert.equal(await saveField(page, "name").inputValue(), "Revised research");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().loadedModelName), "Fullscreen research");
    library.failVersions = false;
    const pending = await holdRequest(page, `/api/v1/models/${modelId}/versions`, "POST");
    try {
      await saveField(page, "save").click();
      await pending.received;
      await action(page, "model").click();
      await action(page, "save").click();
      assert.equal(await saveField(page, "panel").getAttribute("aria-busy"), "true");
      assert.equal(await saveField(page, "save").isDisabled(), true);
      assert.equal(await saveField(page, "save-as").isDisabled(), true);
      await saveField(page, "save-as").evaluate((button) => button.click());
    } finally { pending.release(); }
    await waitForSave(page);
    assert.equal(library.models.length, 1);
    assert.equal(library.versions.length, 2);
    assert.equal(library.versions[1].model_id, modelId);
    assert.equal(library.models[0].name, "Revised research");
    assert.ok(library.versions[1].payload.nodes.every((node) => node.y === 2));
    await saveField(page, "name").fill("Research variant");
    await saveField(page, "save-as").click();
    await waitForSave(page);
    assert.equal(library.models.length, 2);
    assert.equal(library.versions.length, 3);
    assert.notEqual(library.models[1].model_id, modelId);
    assert.equal(library.models[1].name, "Research variant");
    assert.deepEqual(library.versions[2].payload.nodes, library.versions[1].payload.nodes);
    assert.equal(await page.evaluate(() => Boolean(document.fullscreenElement)), true);
    await action(page, "library").click();
    await page.waitForFunction(() => !document.fullscreenElement && window.__kyuubikiPwdt.state().sidebarSection === "library");
  });
});

test("fullscreen dock resizes with pointer and keyboard, cancels interrupted drags and preserves editing drafts", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", model());
    await enterFullscreen(page);
    await action(page, "model").click();
    await field(page, "query").selectOption("all");
    await field(page, "operation").selectOption("rotate");
    await field(page, "pivot").selectOption("point");
    await field(page, "angle").fill("37");
    const handle = page.locator('[data-workbench-immersive-resize="true"]');
    const size = async () => Number(await handle.getAttribute("aria-valuenow"));
    const initial = await size();
    let box = await handle.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 55, box.y + box.height / 2, { steps: 6 });
    await page.mouse.up();
    assert.ok(Math.abs(await size() - initial - 55) <= 1);
    await handle.press("Shift+ArrowLeft");
    const preferred = await size();
    assert.ok(Math.abs(preferred - initial - 15) <= 1);
    await action(page, "toggle-tools").click();
    await action(page, "model").click();
    assert.equal(await size(), preferred);
    assert.equal(await field(page, "angle").inputValue(), "37");
    box = await handle.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x - 75, box.y + box.height / 2, { steps: 4 });
    await page.evaluate(() => window.dispatchEvent(new Event("blur")));
    await page.mouse.up();
    assert.equal(await size(), preferred, "blur cancels, rather than persisting a half-completed drag");
    for (const [width, height] of [[800, 600], [640, 800], [640, 600]]) {
      await page.setViewportSize({ width, height });
      await page.waitForFunction((stacked) => document.querySelector('[data-workbench-immersive-resize="true"]')
        ?.getAttribute("aria-orientation") === (stacked ? "horizontal" : "vertical"), width <= 700);
      await handle.press("Home");
      assert.ok(Math.abs(await size() - Number(await handle.getAttribute("aria-valuemin"))) <= 1);
      assert.equal(await field(page, "angle").inputValue(), "37");
      for (const edge of ["Home", "End"]) {
        await handle.press(edge);
        await field(page, "apply").click({ trial: true });
        const stage = await page.locator('[data-workbench-viewport="stage"]').boundingBox();
        const geometry = await page.locator("[data-immersive-resizable]").evaluate((layout) => ({
          clientHeight: layout.clientHeight, box: layout.getBoundingClientRect().toJSON(),
          rows: getComputedStyle(layout).gridTemplateRows, padding: getComputedStyle(layout).padding,
          gap: getComputedStyle(layout).rowGap, size: layout.style.getPropertyValue("--workbench-immersive-dock-size"),
          main: layout.querySelector(".canvas-layout__main").getBoundingClientRect().toJSON(),
          chrome: layout.querySelector(".canvas-layout__chrome").getBoundingClientRect().toJSON(),
        }));
        assert.ok(stage.y + stage.height <= height + 1);
        assert.ok(width > 700 ? stage.width >= width * 0.55 : stage.height >= height * 0.4,
          `canvas must remain primary at ${edge} ${width}x${height}: ${JSON.stringify({ stage, geometry })}`);
      }
      if (process.env.KYUUBIKI_IMMERSIVE_SCREENSHOT && width === 640 && height === 600) {
        await page.screenshot({ path: process.env.KYUUBIKI_IMMERSIVE_SCREENSHOT.replace(/\.png$/, "-compact.png") });
      }
      await field(page, "angle").click();
      await field(page, "angle").fill("37");
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await handle.dblclick();
    assert.equal(await size(), initial, "double click restores the session default");
    assert.equal(await field(page, "angle").inputValue(), "37");
    await invoke(page, "viewport/setUiState", { immersiveViewport: false });
    assert.equal(await handle.count(), 0);
  });
});
