import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function paint(page, frames = 2) {
  await page.evaluate((frames) => new Promise((resolve) => {
    const next = () => --frames <= 0 ? resolve() : requestAnimationFrame(next);
    requestAnimationFrame(next);
  }), frames);
}

async function inspect(page) {
  return page.evaluate(() => {
    const shell = document.querySelector(".viewport-3d-shell");
    return {
      nodes: Number(shell.dataset.lodNodes), elements: Number(shell.dataset.lodElements),
      zoom: Number(shell.dataset.cameraZoom), visits: Number(shell.dataset.lodVisits),
      limited: shell.dataset.lodActive === "true",
      nodeIds: [...shell.querySelectorAll("[data-truss3d-node]")].map((el) => Number(el.dataset.truss3dNode)),
      memberIds: [...shell.querySelectorAll("[data-truss3d-element]")].map((el) => Number(el.dataset.truss3dElement)),
      gpu: structuredClone(window.__lodGpu),
    };
  });
}

test("PWDT imports 100k nodes; real WebGL stays bounded and zoom/picking refine the true tail", { timeout: 180_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await page.addInitScript(() => {
      const metrics = window.__lodGpu = { draws: 0, uploads: 0, maxPoints: 0, maxLines: 0, maxBufferBytes: 0, errors: [] };
      const draw = WebGLRenderingContext.prototype.drawArrays;
      WebGLRenderingContext.prototype.drawArrays = function (mode, first, count) {
        const result = draw.call(this, mode, first, count);
        metrics.draws++;
        if (mode === this.POINTS) metrics.maxPoints = Math.max(metrics.maxPoints, count);
        if (mode === this.LINES) metrics.maxLines = Math.max(metrics.maxLines, count / 2);
        const error = this.getError();
        if (error !== this.NO_ERROR) metrics.errors.push(error);
        return result;
      };
      const buffer = WebGLRenderingContext.prototype.bufferData;
      WebGLRenderingContext.prototype.bufferData = function (...args) {
        metrics.uploads++;
        metrics.maxBufferBytes = Math.max(metrics.maxBufferBytes, args[1]?.byteLength ?? 0);
        return buffer.apply(this, args);
      };
    });
    await openWorkbench(page);
    // Generate in the browser, through the same PWDT entry used by user scripts. No giant fixture file.
    await page.evaluate(async () => {
      const count = 100_000;
      const nodes = Array.from({ length: count }, (_, i) => ({ id: `lod-node-${i}`, x: i % 1000, y: 0, z: Math.floor(i / 1000),
        fix_x: i === 0, fix_y: i === 0, fix_z: i === 0, load_x: 0, load_y: 0, load_z: 0 }));
      const elements = Array.from({ length: count - 1 }, (_, i) => ({ id: `lod-member-${i}`, node_i: i, node_j: i + 1,
        area: 0.01, youngs_modulus: 70e9 }));
      await window.__kyuubikiPwdt.invoke("state/replaceTruss3dModel", { nodes, elements });
      await window.__kyuubikiPwdt.openSidebar("model");
      await window.__kyuubikiPwdt.invoke("viewport/set3dView", { preset: "front", projection: "ortho" });
      await window.__kyuubikiPwdt.invoke("viewport/toggleFlags", { grid: false, nodes: true, labels: false });
    });
    const svg = page.getByLabel("3d truss response", { exact: true });
    await svg.waitFor({ state: "visible" });
    await page.waitForFunction(() => document.querySelector('.viewport-3d-shell')?.dataset.lodActive === "true" && window.__lodGpu.draws > 0);
    await paint(page);
    const initial = await inspect(page);
    assert.ok(initial.nodes <= 1200 && initial.elements <= 2400);
    assert.ok(initial.nodeIds.some((i) => i > 95_000), "overview cannot render just the file prefix");
    assert.ok(initial.gpu.maxPoints <= 1200);
    assert.ok(initial.gpu.maxLines <= 2400 + 270, "grid/bounds references also have a fixed budget");
    assert.ok(initial.gpu.maxBufferBytes <= (2400 + 270) * 2 * 4 * 4);
    await paint(page, 40);
    const idle = await inspect(page);
    assert.deepEqual(idle.nodeIds, initial.nodeIds, "idle must not eventually grow to all 100k nodes");
    assert.equal(idle.gpu.uploads, initial.gpu.uploads, "idle must not keep uploading detail");

    const indices = [91_910, 91_911, 91_912, 91_913, 91_914, 91_915];
    await invoke(page, "selection/set3d", { nodeIndices: indices, anchorNodeIndex: indices[0] });
    await invoke(page, "viewport/focus3d");
    await page.waitForFunction(() => Number(document.querySelector('.viewport-3d-shell').dataset.cameraZoom) > 2.8);
    await paint(page);
    const focused = await inspect(page);
    for (const i of indices) assert.ok(focused.nodeIds.includes(i), `missing focused node ${i}`);
    assert.ok(focused.nodeIds.some((i) => !initial.nodeIds.includes(i)), "zoom must actually refine new detail");
    assert.ok(focused.nodes <= 1200 && focused.elements <= 2400);
    const target = page.locator('[data-truss3d-node="91912"]');
    const anchor = await target.evaluate((node) => {
      const p = new DOMPoint(Number(node.getAttribute("cx")), Number(node.getAttribute("cy"))).matrixTransform(node.getScreenCTM());
      return { x: p.x, y: p.y };
    });
    await page.mouse.move(anchor.x, anchor.y);
    await page.mouse.wheel(100, -100);
    await page.waitForFunction((zoom) => Number(document.querySelector('.viewport-3d-shell').dataset.cameraZoom) > zoom, focused.zoom);
    await paint(page);
    const anchored = await target.evaluate((node) => {
      const p = new DOMPoint(Number(node.getAttribute("cx")), Number(node.getAttribute("cy"))).matrixTransform(node.getScreenCTM());
      return { x: p.x, y: p.y };
    });
    assert.ok(Math.abs(anchor.x - anchored.x) < 2 && Math.abs(anchor.y - anchored.y) < 2, "cursor zoom must not throw the inspected node away");
    await target.click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().selectedNode === 91912);
    assert.match(await page.locator('[data-workbench-3d-readout="true"]').textContent(), /lod-node-91912/);
    if (process.env.KYUUBIKI_LOD_SCREENSHOT) await page.screenshot({ path: process.env.KYUUBIKI_LOD_SCREENSHOT });

    await invoke(page, "viewport/reset3d");
    await page.waitForFunction(() => document.querySelector('.viewport-3d-shell').dataset.cameraZoom === "1");
    await invoke(page, "viewport/set3dView", { preset: "top", projection: "persp" });
    await page.setViewportSize({ width: 1024, height: 768 });
    await paint(page);
    const final = await inspect(page);
    assert.deepEqual(final.gpu.errors, []);
    assert.ok(final.nodes <= 1200 && final.elements <= 2400);
    assert.ok(final.visits <= 14_400);
    assert.equal(library.writes.length, 0, "display operations must not write or resave a decimated model");
    // Saving after LOD must preserve the full model and the same endpoint identities.
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "large-model-display-check", saveAs: true }));
    const payload = library.versions.at(-1).payload;
    assert.equal(payload.nodes.length, 100_000);
    assert.equal(payload.elements.length, 99_999);
    assert.equal(payload.nodes[91912].id, "lod-node-91912");
    assert.equal(payload.elements[91912].node_i, 91912);
    console.log(JSON.stringify({ scope: "100k PWDT + real browser WebGL; mock backend, not installed native or solver qualification",
      overview: { nodes: initial.nodes, elements: initial.elements, visits: initial.visits },
      focused: { nodes: focused.nodes, elements: focused.elements, zoom: focused.zoom, visits: focused.visits }, gpu: final.gpu }));
  });
});
