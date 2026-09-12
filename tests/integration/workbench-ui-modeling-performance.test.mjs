import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function paint(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function readGpu(page) {
  return page.evaluate(() => window.__modelingGpu.snapshot());
}

test("Workbench modeling reuses GPU resources for cameras and recovers after context loss", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page) => {
    await page.addInitScript(() => {
      const counts = {}, programs = new Set(), buffers = new Set(), errors = [];
      for (const method of ["createProgram", "deleteProgram", "createBuffer", "deleteBuffer", "compileShader", "bufferData", "drawArrays"]) {
        const original = WebGLRenderingContext.prototype[method];
        WebGLRenderingContext.prototype[method] = function (...args) {
          const result = original.apply(this, args);
          counts[method] = (counts[method] ?? 0) + 1;
          if (method === "createProgram" && result) programs.add(result);
          if (method === "deleteProgram") programs.delete(args[0]);
          if (method === "createBuffer" && result) buffers.add(result);
          if (method === "deleteBuffer") buffers.delete(args[0]);
          if (method === "drawArrays") {
            const error = this.getError();
            if (error !== this.NO_ERROR) errors.push(error);
          }
          return result;
        };
      }
      window.__modelingGpu = { snapshot: () => ({ ...counts, programs: programs.size, buffers: buffers.size, errors: [...errors] }) };
    });
    await openWorkbench(page);
    const count = 1_000;
    const model = {
      nodes: Array.from({ length: count }, (_, index) => ({
        id: `n${index}`, x: (index % 10) / 2, y: (Math.floor(index / 10) % 10) / 2, z: Math.floor(index / 100) / 2,
        fix_x: index === 0, fix_y: index === 0, fix_z: index === 0, load_x: 0, load_y: 0, load_z: 0,
      })),
      elements: Array.from({ length: count - 1 }, (_, index) => ({
        id: `e${index}`, node_i: index, node_j: index + 1, area: 0.01, youngs_modulus: 70e9,
      })),
    };
    await invoke(page, "state/replaceTruss3dModel", model);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    const svg = page.getByLabel("3d truss response", { exact: true });
    await svg.waitFor({ state: "visible", timeout: 30_000 });
    await page.waitForFunction(() => window.__modelingGpu.snapshot().drawArrays > 0);
    await paint(page);
    assert.equal(await svg.locator(".viewport-frame").evaluate((frame) => getComputedStyle(frame).fill), "none",
      "the SVG overlay must not cover the WebGL geometry");
    const before = await readGpu(page);
    assert.equal(before.programs, 1);
    assert.equal(before.buffers, 9);
    assert.deepEqual(before.errors, []);

    await svg.focus();
    for (const key of ["ArrowRight", "ArrowDown", "1", "2", "3", "4"]) {
      await svg.press(key);
      await paint(page);
    }
    const camera = await readGpu(page);
    assert.ok(camera.drawArrays > before.drawArrays);
    assert.equal(camera.compileShader, before.compileShader, "camera changes must not compile shaders");
    assert.equal(camera.createBuffer, before.createBuffer, "camera changes must not allocate buffers");
    assert.equal(camera.bufferData, before.bufferData, "camera changes must not upload unchanged geometry");

    await page.setViewportSize({ width: 1180, height: 820 });
    await paint(page);
    const resized = await readGpu(page);
    assert.equal(resized.programs, 1);
    assert.equal(resized.buffers, 9);
    assert.equal(resized.compileShader, before.compileShader);

    await invoke(page, "selection/set3d", { nodeIndices: [2, 4, 8], anchorNodeIndex: 2 });
    await paint(page);
    assert.ok((await readGpu(page)).bufferData > camera.bufferData, "selection must update rendering");
    const edited = structuredClone(model);
    edited.nodes[2].x += 0.125;
    const beforeEdit = await readGpu(page);
    await invoke(page, "state/replaceTruss3dModel", edited);
    await paint(page);
    assert.ok((await readGpu(page)).bufferData > beforeEdit.bufferData, "geometry edits must update rendering");
    await invoke(page, "history/undo");
    await paint(page);

    await page.evaluate(() => {
      const canvas = document.querySelector(".viewport-3d-shell canvas");
      const gl = canvas.getContext("webgl");
      const extension = gl.getExtension("WEBGL_lose_context");
      if (!extension) throw new Error("WEBGL_lose_context is unavailable");
      window.__restoreModelingContext = () => extension.restoreContext();
      extension.loseContext();
    });
    await page.waitForFunction(() => window.__modelingGpu.snapshot().programs === 0);
    const lost = await readGpu(page);
    assert.equal(lost.buffers, 0);
    await page.evaluate(() => window.__restoreModelingContext());
    await page.waitForFunction((draws) => window.__modelingGpu.snapshot().drawArrays > draws, lost.drawArrays);
    const restored = await readGpu(page);
    assert.equal(restored.programs, 1);
    assert.equal(restored.buffers, 9);
    assert.deepEqual(restored.errors, []);
    if (process.env.KYUUBIKI_MODELING_SCREENSHOT) {
      await page.screenshot({ path: process.env.KYUUBIKI_MODELING_SCREENSHOT });
    }

    await invoke(page, "nav/setStudyKind", { studyKind: "truss_2d" });
    await paint(page);
    const unmounted = await readGpu(page);
    assert.equal(unmounted.programs, 0, "leaving the 3D viewport must release its program");
    assert.equal(unmounted.buffers, 0, "leaving the 3D viewport must release its buffers");
    console.log(JSON.stringify({ scope: "isolated Workbench + real browser WebGL; mock backend", nodes: count, before, camera, resized, restored, unmounted }));
  });
});
