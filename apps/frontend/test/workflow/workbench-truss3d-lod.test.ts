import assert from "node:assert/strict";
import test from "node:test";
import { buildTruss3dLodIndex, queryTruss3dLod, truss3dLodBudget, type Truss3dLodIndex } from "../../src/components/workbench/workbench-truss3d-lod";
import { buildProjectedBounds, cameraForPreset, projectTruss3dPoint, type CameraState } from "../../src/components/workbench/workbench-viewport-core";
import { buildTruss3dSceneBuffers } from "../../src/components/workbench/workbench-truss3d-webgl-scene";
import { focusTruss3d } from "../../src/components/workbench/workbench-truss3d-camera";
import { modelingSceneFixture } from "../support/modeling-fixtures";

function grid(count: number) {
  const args = modelingSceneFixture(count);
  for (const node of args.displayTruss3dNodes) {
    node.x = node.index % 1000; node.y = 0; node.z = Math.floor(node.index / 1000);
  }
  return args;
}
function view(index: Truss3dLodIndex, camera: CameraState = cameraForPreset("front")) {
  return { camera, projected3d: buildProjectedBounds(index.corners, camera), projectionMode: "ortho" as const };
}

test("small models retain all original identities; LOD never mutates nodes or connectivity", () => {
  const args = grid(101), before = structuredClone(args);
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements);
  const lod = queryTruss3dLod(index, view(index), truss3dLodBudget("auto"));
  assert.deepEqual(lod.nodes, args.displayTruss3dNodes);
  assert.deepEqual(lod.elements, args.visibleTruss3dElements);
  assert.equal(lod.limited, false);
  assert.equal(lod.nodes[99], args.displayTruss3dNodes[99]);
  assert.deepEqual(args, before);
});

test("100k model has bounded, distributed geometry and query work under every render strategy", (t) => {
  const args = grid(100_000), start = performance.now();
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements);
  const buildMs = performance.now() - start;
  for (const strategy of ["auto", "full", "progressive", "focus"] as const) {
    const budget = truss3dLodBudget(strategy), queryStart = performance.now();
    const lod = queryTruss3dLod(index, view(index), budget);
    assert.ok(lod.nodes.length > 0 && lod.nodes.length <= budget.nodes);
    assert.ok(lod.elements.length > 0 && lod.elements.length <= budget.elements);
    assert.ok(lod.visits <= 4 * (budget.nodes + budget.elements));
    assert.ok(lod.tested < 10 * (budget.nodes + budget.elements));
    assert.ok(lod.nodes.some((n) => n.index >= 95_000), "overview must not be the file prefix");
    assert.ok(lod.nodes.some((n) => n.index < 5_000));
    assert.equal(lod.limited, true);
    const buffers = buildTruss3dSceneBuffers({ ...args, visibleTruss3dNodes: lod.nodes, visibleTruss3dElements: lod.elements,
      sceneBounds: index.bounds, deformationScale: index.deformationScale });
    assert.ok(buffers.nodePositions.length <= budget.nodes * 3);
    assert.ok(buffers.linePositions.length <= budget.elements * 6);
    t.diagnostic(`${strategy}: query+buffers ${(performance.now() - queryStart).toFixed(2)}ms; nodes=${lod.nodes.length}; elements=${lod.elements.length}; cells=${lod.visits}`);
  }
  t.diagnostic(`100k spatial index build ${buildMs.toFixed(2)}ms (not an FPS or solver benchmark)`);
});

test("zooming to the tail refines local details that are absent from the overview", () => {
  const args = grid(100_000), index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements);
  const initial = view(index), budget = truss3dLodBudget("auto");
  const overview = queryTruss3dLod(index, initial, budget), overviewIds = new Set(overview.nodes.map((n) => n.index));
  const target = args.displayTruss3dNodes.slice(91_910, 91_916);
  const camera = focusTruss3d(initial.camera, initial.projected3d, "ortho", target);
  const zoomed = queryTruss3dLod(index, view(index, camera), budget);
  assert.ok(camera.zoom > 2.8);
  assert.ok(zoomed.nodes.some((node) => !overviewIds.has(node.index)));
  for (const node of target) assert.ok(zoomed.nodes.includes(node), `missing local node ${node.index}`);
  assert.ok(zoomed.nodes.every((n) => n.index > 80_000));
  assert.ok(zoomed.nodes.length <= budget.nodes);
  const outside = queryTruss3dLod(index, view(index, { ...camera, panX: 1e12 }), budget);
  assert.equal(outside.nodes.length, 0);
  assert.equal(outside.elements.length, 0);
});

test("selected nodes, selected member endpoints and link drafts survive bounded LOD without reindexing", () => {
  const args = grid(10_000), index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements);
  const budget = { nodes: 20, elements: 30 };
  const lod = queryTruss3dLod(index, view(index), budget, { node: 9999, element: 9501, draft: [8000], nodes: Array.from({ length: 9000 }, (_, i) => i) });
  for (const i of [9999, 9501, 9502, 8000]) assert.ok(lod.nodes.includes(args.displayTruss3dNodes[i]));
  assert.ok(lod.elements.includes(args.visibleTruss3dElements[9501]));
  assert.ok(lod.nodes.length <= budget.nodes, "large multi-selection must not lift the display cap");
  assert.ok(lod.elements.length <= budget.elements);
});

test("members crossing the viewport are retained even if both endpoints are off screen", () => {
  const args = grid(2);
  args.displayTruss3dNodes[0].x = -1000; args.displayTruss3dNodes[1].x = 1000;
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements);
  const lod = queryTruss3dLod(index, view(index, { ...cameraForPreset("front"), zoom: 100 }), { nodes: 40, elements: 80 });
  assert.equal(lod.nodes.length, 0);
  assert.equal(lod.elements[0], args.visibleTruss3dElements[0]);
});

test("material visibility and invalid endpoints cannot be overridden by pinning", () => {
  const args = grid(4);
  args.visibleTruss3dElements[0].material_id = "hidden";
  args.visibleTruss3dElements[1].node_j = 5000;
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements, ["hidden"]);
  for (const element of [0, 1, 5000]) {
    const lod = queryTruss3dLod(index, view(index), truss3dLodBudget("auto"), { element, node: 5000 });
    assert.deepEqual(lod.elements, [args.visibleTruss3dElements[2]]);
  }
});

test("paged results resolve global node and member IDs instead of treating them as local offsets", () => {
  const args = grid(3);
  for (const n of args.displayTruss3dNodes) n.index += 9000;
  for (const e of args.visibleTruss3dElements) { e.index += 5000; e.node_i += 9000; e.node_j += 9000; }
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements, [], true);
  const lod = queryTruss3dLod(index, view(index), truss3dLodBudget("auto"), { node: 9002, element: 5001 });
  assert.deepEqual(lod.nodes.map((n) => n.index), [9000, 9001, 9002]);
  assert.deepEqual(lod.elements.map((e) => e.index), [5000, 5001]);
  assert.equal(index.nodeAt(9002), args.displayTruss3dNodes[2]);
  const buffers = buildTruss3dSceneBuffers({ ...args, nodeByIndex: index.nodeAt,
    visibleTruss3dNodes: lod.nodes, visibleTruss3dElements: lod.elements });
  assert.equal(buffers.linePositions.length, 12);
  assert.equal(index.nodeAt(2), undefined);
});

test("deformation scale and global bounds stay constant when only a subset is visible", () => {
  const args = grid(3000);
  args.displayTruss3dNodes[2999].ux = 10;
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements, [], true);
  for (const subset of [args.displayTruss3dNodes.slice(0, 10), args.displayTruss3dNodes.slice(-10)]) {
    const buffers = buildTruss3dSceneBuffers({ ...args, isModelMode: false, showGrid: true, deformationViewMode: "overlay",
      visibleTruss3dNodes: subset, visibleTruss3dElements: [], sceneBounds: index.bounds, deformationScale: index.deformationScale });
    assert.equal(buffers.deformationScale, index.deformationScale);
    assert.ok([...buffers.linePositions].includes(index.bounds.maxX - index.bounds.center.x));
  }
});

test("deformed geometry outside the original cells is not culled", () => {
  const args = grid(2);
  args.displayTruss3dNodes[1].ux = 1;
  const index = buildTruss3dLodIndex(args.displayTruss3dNodes, args.visibleTruss3dElements, [], true);
  const initial = view(index), target = { x: 1 + index.deformationScale, y: 0, z: 0 };
  const camera = focusTruss3d({ ...initial.camera, zoom: 100 }, initial.projected3d, "ortho", [target]);
  const current = view(index, camera);
  assert.ok(projectTruss3dPoint(args.displayTruss3dNodes[1], current.projected3d, camera, "ortho").x < 0);
  const lod = queryTruss3dLod(index, current, truss3dLodBudget("auto"));
  assert.ok(lod.nodes.includes(args.displayTruss3dNodes[1]));
  assert.ok(lod.elements.includes(args.visibleTruss3dElements[0]));
});
