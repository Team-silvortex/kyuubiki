import assert from "node:assert/strict";
import test from "node:test";
import { buildProjectedBounds, cameraForPreset, projectTruss3dPoint, truss3dDragDelta,
  TRUSS3D_PROJECTION, VIEWPORT_CLIP, type DisplayTruss3dNode } from "../../src/components/workbench/workbench-viewport-core";

const nodes: DisplayTruss3dNode[] = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].map(([x, y, z], index) =>
  ({ id: `node-${index}`, index, x, y, z, ux: 0, uy: 0, uz: 0 }));
const near = (a: number, b: number) => assert.ok(Math.abs(a - b) < 1e-7, `${a} != ${b}`);

test("default 3D presets fit every tetrahedron node inside the canvas, below its title", () => {
  for (const preset of ["iso", "front", "right", "top"] as const) {
    for (const offset of [0, 1e6]) {
      const translated = nodes.map((node) => ({ ...node, x: node.x + offset, y: node.y - offset }));
      const camera = cameraForPreset(preset), bounds = buildProjectedBounds(translated, camera);
      for (const node of translated) {
        const point = projectTruss3dPoint(node, bounds, camera, "ortho");
        assert.ok(point.x >= VIEWPORT_CLIP.x + 10 && point.x <= VIEWPORT_CLIP.x + VIEWPORT_CLIP.width - 10);
        assert.ok(point.y >= 112 && point.y <= VIEWPORT_CLIP.y + VIEWPORT_CLIP.height - 10);
      }
    }
  }
});

test("orthographic fitting preserves equal physical lengths and zooms around the viewport center", () => {
  const camera = cameraForPreset("front"), bounds = buildProjectedBounds(nodes, camera);
  const points = nodes.map((node) => projectTruss3dPoint(node, bounds, camera, "ortho"));
  near(points[1].x - points[0].x, points[0].y - points[3].y);
  const center = { x: 0.5, y: 0, z: 0.5 };
  for (const zoom of [0.5, 1, 2]) {
    const point = projectTruss3dPoint(center, bounds, { ...camera, zoom }, "ortho");
    near(point.x, TRUSS3D_PROJECTION.centerX);
    near(point.y, TRUSS3D_PROJECTION.centerY);
  }
});

test("screen-plane dragging inverts yaw, pitch, zoom and perspective depth", () => {
  for (const preset of ["iso", "front", "right", "top"] as const) {
    for (const projection of ["ortho", "persp"] as const) {
      const camera = { ...cameraForPreset(preset), zoom: 1.3, panX: 12, panY: -18 };
      const bounds = buildProjectedBounds(nodes, camera), node = nodes[3];
      const delta = truss3dDragDelta(node, bounds, camera, projection, 13, -9);
      const before = projectTruss3dPoint(node, bounds, camera, projection);
      const after = projectTruss3dPoint({ x: node.x + delta.x, y: node.y + delta.y, z: node.z + delta.z }, bounds, camera, projection);
      near(after.x - before.x, 13);
      near(after.y - before.y, -9);
    }
  }
});
