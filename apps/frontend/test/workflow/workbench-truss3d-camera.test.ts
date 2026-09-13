import assert from "node:assert/strict";
import test from "node:test";
import { focusTruss3d, TRUSS3D_ZOOM, zoomTruss3dAt } from "../../src/components/workbench/workbench-truss3d-camera";
import { buildProjectedBounds, cameraForPreset, projectTruss3dPoint } from "../../src/components/workbench/workbench-viewport-core";
import { modelingSceneFixture } from "../support/modeling-fixtures";
const near = (a: number, b: number, tolerance = 1e-6) => assert.ok(Math.abs(a - b) < tolerance, `${a} != ${b}`);

test("wheel zoom keeps the world point under the cursor in both projections", () => {
  const nodes = modelingSceneFixture(40).displayTruss3dNodes;
  const camera = { ...cameraForPreset("iso"), panX: 12, panY: -8 }, bounds = buildProjectedBounds(nodes, camera);
  for (const mode of ["ortho", "persp"] as const) {
    const before = projectTruss3dPoint(nodes[27], bounds, camera, mode);
    const zoomed = zoomTruss3dAt(camera, before, -200);
    const after = projectTruss3dPoint(nodes[27], bounds, zoomed, mode);
    near(before.x, after.x); near(before.y, after.y);
    const restored = zoomTruss3dAt(zoomed, before, 200);
    near(restored.zoom, camera.zoom); near(restored.panX, camera.panX); near(restored.panY, camera.panY);
  }
});

test("large zoom is bounded, zero/nonfinite input is inert and wheel units are normalized", () => {
  const camera = cameraForPreset("front"), point = { x: 600, y: 250 };
  for (const delta of [0, NaN, Infinity]) assert.equal(zoomTruss3dAt(camera, point, delta), camera);
  assert.deepEqual(zoomTruss3dAt(camera, point, -10, 1), zoomTruss3dAt(camera, point, -160, 0));
  assert.equal(zoomTruss3dAt({ ...camera, zoom: 1023 }, point, -1e6).zoom, TRUSS3D_ZOOM.max);
  assert.equal(zoomTruss3dAt({ ...camera, zoom: 0.09 }, point, 1e6).zoom, TRUSS3D_ZOOM.min);
});

test("perspective is invariant under model translation and unit scale, without the old y=-8 singularity", () => {
  const base = modelingSceneFixture(20).displayTruss3dNodes;
  for (const preset of ["front", "iso", "top", "right"] as const) {
    const camera = cameraForPreset(preset), bounds = buildProjectedBounds(base, camera);
    for (const scale of [1e-4, 1, 1e6]) {
      const nodes = base.map((n) => ({ ...n, x: n.x * scale + 1e6, y: n.y * scale - 8, z: n.z * scale - 1e6 }));
      const translatedBounds = buildProjectedBounds(nodes, camera);
      for (let i = 0; i < base.length; i++) {
        const a = projectTruss3dPoint(base[i], bounds, camera, "persp");
        const b = projectTruss3dPoint(nodes[i], translatedBounds, camera, "persp");
        near(a.x, b.x, 0.001); near(a.y, b.y, 0.001);
      }
    }
  }
});

test("focus fits a local selection, and empty selection returns to full-model fit", () => {
  const nodes = modelingSceneFixture(10_000).displayTruss3dNodes;
  const camera = cameraForPreset("top"), bounds = buildProjectedBounds(nodes, camera);
  const selected = nodes.slice(5000, 5004), focused = focusTruss3d(camera, bounds, "ortho", selected);
  assert.ok(focused.zoom > 2.8);
  for (const n of selected) {
    const p = projectTruss3dPoint(n, bounds, focused, "ortho");
    assert.ok(p.x >= 180 && p.x <= 800 && p.y >= 140 && p.y <= 370);
  }
  assert.deepEqual(focusTruss3d(focused, bounds, "ortho", []), camera);
});
