import test from "node:test";
import assert from "node:assert/strict";

import {
  buildTruss3dSceneBuffers,
  computeTruss3dSceneBounds,
  type Truss3dSceneBuildArgs,
} from "../../src/components/workbench/workbench-truss3d-webgl-scene.ts";

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
