import assert from "node:assert/strict";
import { test } from "node:test";
import { getTrussBounds, findNearestConnectableNode } from "../../src/lib/workbench/model-geometry";
import {
  updateTruss3dSelectedNodes, nudgeTruss3dSelectedNodes, applyTruss3dSelectedLoads,
  updateTruss3dNodePositionCommand,
} from "../../src/lib/workbench/truss3d-commands";
import { buildProjectedBounds, buildTruss3dGrid, cameraForPreset } from "../../src/components/workbench/workbench-viewport-core";
import { computeTruss3dSceneBounds, resolveDeformationScale } from "../../src/components/workbench/workbench-truss3d-webgl-scene";
import { buildSelectionSummary } from "../../src/components/workbench/workbench-truss3d-readout";
import { modelingFixture, modelingSceneFixture } from "../support/modeling-fixtures";

const round = (value: number) => Math.round(value * 1000) / 1000;

test("batch modeling preserves untouched geometry and undo snapshots", () => {
  const model = modelingFixture(8);
  const before = structuredClone(model);
  const selected = [6, 2, 2, -1, 99, 0.5, NaN];
  const updated = updateTruss3dSelectedNodes(model, selected, 0, "fix_x", true);
  const nudged = nudgeTruss3dSelectedNodes(model, selected, 0, "x", 0.1254, round);
  const loaded = applyTruss3dSelectedLoads(model, selected, 0, "apply", { x: 1, y: -2, z: 3 });
  for (const next of [updated, nudged, loaded]) {
    assert.notEqual(next.nodes, model.nodes);
    assert.equal(next.elements, model.elements);
    for (const index of [0, 1, 3, 4, 5, 7]) assert.equal(next.nodes[index], model.nodes[index]);
  }
  assert.equal(updated.nodes[2].fix_x, true);
  assert.equal(nudged.nodes[2].x, 2.125, "duplicate selection cannot apply a nudge twice");
  assert.equal(loaded.nodes[6].load_y, -2);
  assert.deepEqual(model, before);
});

test("unchanged, empty, and stale selections do not allocate a replacement model", () => {
  const model = modelingFixture(4);
  assert.equal(updateTruss3dSelectedNodes(model, [], null, "fix_x", true), model);
  assert.equal(updateTruss3dSelectedNodes(model, [0], null, "fix_x", true), model);
  assert.equal(updateTruss3dSelectedNodes(model, [99, -1, NaN], 0, "fix_x", false), model);
  assert.equal(nudgeTruss3dSelectedNodes(model, [1, 2], null, "x", 0, round), model);
  assert.equal(applyTruss3dSelectedLoads(model, [1, 2], null, "clear", { x: 4, y: 5, z: 6 }), model);
  assert.equal(updateTruss3dNodePositionCommand(model, 1, model.nodes[1], round), model);
  assert.equal(updateTruss3dNodePositionCommand(model, -1, model.nodes[1], round), model);
  assert.equal(updateTruss3dSelectedNodes(model, [], 1, "fix_z", true).nodes[1].fix_z, true);
  const moved = updateTruss3dNodePositionCommand(model, 1, { x: 1.0001, y: 2.2345, z: 4 }, round);
  assert.equal(moved.nodes[1].y, 2.235);
  assert.equal(moved.nodes[0], model.nodes[0]);
});

test("batch commands read the selection once rather than once per model node", () => {
  const model = modelingFixture(2_000);
  let reads = 0;
  const selected = new Proxy(Array.from({ length: 1_000 }, (_, index) => index * 2), {
    get(target, key, receiver) {
      if (typeof key === "string" && /^\d+$/.test(key)) reads += 1;
      return Reflect.get(target, key, receiver);
    },
  });
  for (const run of [
    () => updateTruss3dSelectedNodes(model, selected, null, "fix_y", true),
    () => nudgeTruss3dSelectedNodes(model, selected, null, "z", 1, round),
    () => applyTruss3dSelectedLoads(model, selected, null, "apply", { x: 0, y: 1, z: 0 }),
  ]) {
    reads = 0;
    run();
    assert.ok(reads <= selected.length * 2, `selection lookup regressed: ${reads} reads`);
  }
});

test("nearest connection matches exhaustive search including ties and reverse edges", () => {
  for (let count = 2; count <= 40; count += 1) {
    const model = modelingFixture(count);
    model.nodes.forEach((node, index) => { node.x = (index * 17) % 11; node.y = (index * 7) % 13; });
    model.elements.forEach((element, index) => {
      if (index % 2 === 0) [element.node_i, element.node_j] = [element.node_j, element.node_i];
    });
    for (let origin = 0; origin < count; origin += 1) {
      let expected: number | null = null, best = Infinity;
      for (let candidate = 0; candidate < count; candidate += 1) {
        if (origin === candidate || model.elements.some((element) =>
          (element.node_i === origin && element.node_j === candidate) ||
          (element.node_j === origin && element.node_i === candidate))) continue;
        const distance = Math.hypot(model.nodes[candidate].x - model.nodes[origin].x, model.nodes[candidate].y - model.nodes[origin].y);
        if (distance < best) { expected = candidate; best = distance; }
      }
      assert.equal(findNearestConnectableNode(model, origin), expected);
    }
  }
  assert.equal(findNearestConnectableNode(modelingFixture(0), 0), null);
  assert.equal(findNearestConnectableNode(modelingFixture(4), 99), null);
});

test("nearest connection reads each member a bounded number of times", () => {
  const model = modelingFixture(500);
  let reads = 0;
  model.elements = model.elements.map((element) => ({
    ...element,
    get node_i() { reads += 1; return element.node_i; },
    get node_j() { reads += 1; return element.node_j; },
  }));
  assert.notEqual(findNearestConnectableNode(model, 0), null);
  assert.ok(reads <= model.elements.length * 4, `member lookup regressed: ${reads} reads`);
});

test("bounds preserve default margins, signed zero, and camera projections", () => {
  assert.deepEqual(getTrussBounds([]), { minX: 0, maxX: 1, minY: 0, maxY: 1, width: 1, height: 1 });
  assert.ok(Object.is(getTrussBounds([{ x: -0, y: 0 }]).minX, -0));
  assert.ok(Number.isNaN(getTrussBounds([{ x: NaN, y: 0 }]).minX));
  const nodes = modelingSceneFixture(30).displayTruss3dNodes;
  nodes.forEach((node, index) => { node.x -= 10; node.y = index % 5 - 2; node.z -= 4; });
  for (const preset of ["iso", "front", "top", "right"] as const) {
    const camera = cameraForPreset(preset);
    const xs = nodes.map((node) => node.x * Math.cos(camera.yaw) - node.y * Math.sin(camera.yaw));
    const zs = nodes.map((node) => (node.x * Math.sin(camera.yaw) + node.y * Math.cos(camera.yaw)) * Math.sin(camera.pitch) + node.z * Math.cos(camera.pitch));
    const depths = nodes.map((node) => (node.x * Math.sin(camera.yaw) + node.y * Math.cos(camera.yaw)) * Math.cos(camera.pitch) - node.z * Math.sin(camera.pitch));
    assert.deepEqual(buildProjectedBounds(nodes, camera), {
      minX: Math.min(...xs), maxX: Math.max(...xs), minZ: Math.min(...zs), maxZ: Math.max(...zs),
      width: Math.max(Math.max(...xs) - Math.min(...xs), 1e-12), height: Math.max(Math.max(...zs) - Math.min(...zs), 1e-12),
      depthCenter: (Math.min(...depths) + Math.max(...depths)) / 2,
      depthDistance: Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...zs) - Math.min(...zs), Math.max(...depths) - Math.min(...depths), 1e-12) * 4,
    });
  }
});

test("200k-node bounds, deformation, selection, and grid avoid argument-stack overflow", () => {
  const nodes = Array.from({ length: 200_000 }, (_, index) => ({
    index, id: `n${index}`, x: index, y: -index, z: 2 * index, ux: 0.001, uy: 0, uz: 0,
  }));
  assert.equal(getTrussBounds(nodes).maxX, 199_999);
  assert.equal(computeTruss3dSceneBounds(nodes).spanZ, 399_998);
  assert.equal(buildProjectedBounds(nodes, cameraForPreset("front")).maxZ, 399_998);
  assert.equal(resolveDeformationScale(nodes, true), 24);
  assert.match(buildSelectionSummary("truss_3d", nodes)!.title, /200000/);
  const grid = buildTruss3dGrid(nodes);
  assert.ok(grid.extent * 2 / grid.step <= 128);
  assert.deepEqual(buildTruss3dGrid([]), { extent: 2, step: 1 });
});
