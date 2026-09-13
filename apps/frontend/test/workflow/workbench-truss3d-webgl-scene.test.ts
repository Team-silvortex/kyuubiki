import test from "node:test";
import assert from "node:assert/strict";

import {
  buildTruss3dSceneBuffers,
  computeTruss3dSceneBounds,
  type Truss3dSceneBuildArgs,
} from "../../src/components/workbench/workbench-truss3d-webgl-scene.ts";
import { modelingSceneFixture } from "../support/modeling-fixtures";

const nodes = [
  { index: 0, id: "n0", x: -1, y: -2, z: 0, ux: 0, uy: 0, uz: 0 },
  { index: 1, id: "n1", x: 3, y: 4, z: 2, ux: 0, uy: 0, uz: 0 },
];

function buildArgs(overrides: Partial<Truss3dSceneBuildArgs> = {}): Truss3dSceneBuildArgs {
  return {
    displayTruss3dNodes: nodes,
    gridExtent: 1,
    gridStep: 1,
    hiddenTruss3dMaterialIds: [],
    isModelMode: true,
    memberDraftNodes: [],
    selectedTruss3dElement: null,
    selectedTruss3dNode: null,
    selectedTruss3dNodeIndices: [],
    showGrid: true,
    showNodes: false,
    truss3dElementColors: [],
    truss3dLinkMode: false,
    visibleTruss3dElements: [],
    visibleTruss3dNodes: nodes,
    deformationViewMode: "original",
    ...overrides,
  };
}

test("computes 3D scene bounds with center, spans, and diagonal", () => {
  const bounds = computeTruss3dSceneBounds(nodes);

  assert.deepEqual(bounds.center, { x: 1, y: 1, z: 1 });
  assert.equal(bounds.spanX, 4);
  assert.equal(bounds.spanY, 6);
  assert.equal(bounds.spanZ, 2);
  assert.equal(bounds.diagonal, Math.hypot(4, 6, 2));
});

test("adds a spatial bounding box to visible 3D grid buffers", () => {
  const buffers = buildTruss3dSceneBuffers(buildArgs());
  const gridSegments = 6;
  const boundingBoxSegments = 12;

  assert.equal(buffers.linePositions.length, (gridSegments + boundingBoxSegments) * 2 * 3);
  assert.equal(buffers.lineColors.length, (gridSegments + boundingBoxSegments) * 2 * 4);
});

test("omits spatial bounding box when grid reference is hidden", () => {
  const buffers = buildTruss3dSceneBuffers(buildArgs({ showGrid: false }));

  assert.equal(buffers.linePositions.length, 0);
  assert.equal(buffers.lineColors.length, 0);
});

test("visible node highlighting uses model identity, not a filtered array offset", () => {
  const buffers = buildTruss3dSceneBuffers(buildArgs({
    showGrid: false, showNodes: true, visibleTruss3dNodes: [nodes[1], nodes[0]],
    selectedTruss3dNodeIndices: [1], memberDraftNodes: [0],
  }));
  assert.deepEqual([...buffers.nodeSizes], [12, 9]);
});

test("deformed-only views retain point-size data and cannot hide the modeling geometry", () => {
  const args = modelingSceneFixture(3);
  const buffers = buildTruss3dSceneBuffers({ ...args, isModelMode: false, deformationViewMode: "deformed" });
  assert.equal(buffers.nodePositions.length, 0);
  assert.equal(buffers.deformedNodePositions.length, 9);
  assert.equal(buffers.nodeSizes.length, 3);
  const modeling = buildTruss3dSceneBuffers({ ...args, deformationViewMode: "deformed" });
  assert.equal(modeling.nodePositions.length, 9);
  assert.equal(modeling.deformedNodePositions.length, 0);
  const noResult = buildTruss3dSceneBuffers(buildArgs({ showNodes: true, isModelMode: false, deformationViewMode: "deformed" }));
  assert.equal(noResult.nodePositions.length, nodes.length * 3, "clearing the result cannot leave a hidden original view");
});

test("model coordinate scale cannot create an unbounded visual grid", () => {
  const buffers = buildTruss3dSceneBuffers(buildArgs({ gridExtent: 1e12, gridStep: 1 }));
  assert.ok(buffers.linePositions.length <= (258 + 12) * 6);
  for (const gridStep of [0, -1, NaN, Infinity]) {
    assert.equal(buildTruss3dSceneBuffers(buildArgs({ gridStep })).linePositions.length, 0);
  }
});

test("GPU positions preserve submillimeter detail at a million-unit origin", () => {
  const args = modelingSceneFixture(2);
  args.displayTruss3dNodes[0].x = 1e6;
  args.displayTruss3dNodes[1].x = 1e6 + 0.001;
  const bounds = computeTruss3dSceneBounds(args.displayTruss3dNodes);
  const scene = buildTruss3dSceneBuffers({ ...args, sceneBounds: bounds });
  assert.ok(Math.abs(scene.nodePositions[3] - scene.nodePositions[0] - 0.001) < 1e-9);
  assert.deepEqual(scene.origin, bounds.center);
});

test("unit-gain physical deformation remains visible instead of being mistaken for no result", () => {
  const args = modelingSceneFixture(2);
  args.displayTruss3dNodes[1].ux = 100;
  const scene = buildTruss3dSceneBuffers({ ...args, isModelMode: false, deformationViewMode: "deformed" });
  assert.equal(scene.deformationScale, 1);
  assert.equal(scene.nodePositions.length, 0);
  assert.equal(scene.deformedNodePositions.length, 6);
});

test("scene colors reuse one parser and resolve paged global member IDs to the correct material colors", () => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "document");
  let canvases = 0, materialParses = 0, style = "#000000";
  const context = {
    get fillStyle() { return style; },
    set fillStyle(value: string) {
      if (value === "hsl(120 100% 50%)") { materialParses += 1; style = "#00ff00"; }
      else if (value.startsWith("rgba(")) style = value;
      // Browser canvas ignores invalid CSS assignments.
    },
  };
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: { createElement: () => { canvases += 1; return { getContext: () => context }; } },
  });
  try {
    const args = modelingSceneFixture(2_000);
    const colors = args.visibleTruss3dElements.map(() => "hsl(120 100% 50%)");
    for (const element of args.visibleTruss3dElements) element.index += 5000;
    colors[1] = "invalid-css";
    const buffers = buildTruss3dSceneBuffers({ ...args, truss3dElementColors: colors, elementColorByIndex: (index) => colors[index - 5000] });
    assert.equal(canvases, 1);
    assert.equal(materialParses, 1);
    assert.deepEqual([...buffers.lineColors.slice(0, 4)], [0, 1, 0, 1]);
    assert.ok(Math.abs(buffers.lineColors[8] - 0.48) < 1e-6, "invalid CSS must use the member fallback, not green");
  } finally {
    if (descriptor) Object.defineProperty(globalThis, "document", descriptor);
    else Reflect.deleteProperty(globalThis, "document");
  }
});

test("hidden materials and missing endpoints do not generate geometry", () => {
  const args = modelingSceneFixture(3);
  args.visibleTruss3dElements[0].material_id = "hidden";
  args.visibleTruss3dElements[1].node_j = 99;
  const buffers = buildTruss3dSceneBuffers({ ...args, hiddenTruss3dMaterialIds: ["hidden"] });
  assert.equal(buffers.linePositions.length, 0);
});
