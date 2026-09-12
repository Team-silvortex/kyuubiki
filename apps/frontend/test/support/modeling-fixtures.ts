import type { Truss3dJobInput } from "../../src/lib/api";
import type { Truss3dSceneBuildArgs } from "../../src/components/workbench/workbench-truss3d-webgl-scene";

export function modelingFixture(count: number): Truss3dJobInput {
  return {
    nodes: Array.from({ length: count }, (_, index) => ({
      id: `n${index}`, x: index % 100, y: Math.floor(index / 100), z: index % 7,
      fix_x: index === 0, fix_y: index === 0, fix_z: index === 0,
      load_x: 0, load_y: 0, load_z: 0,
    })),
    elements: Array.from({ length: Math.max(0, count - 1) }, (_, index) => ({
      id: `e${index}`, node_i: index, node_j: index + 1, area: 0.01, youngs_modulus: 70e9,
    })),
  };
}

export function modelingSceneFixture(count: number): Truss3dSceneBuildArgs {
  const model = modelingFixture(count);
  const nodes = model.nodes.map((node, index) => ({ ...node, index, ux: 0.001, uy: 0, uz: 0 }));
  return {
    displayTruss3dNodes: nodes,
    visibleTruss3dNodes: nodes,
    visibleTruss3dElements: model.elements.map((element, index) => ({
      ...element, index, length: 1, strain: 0, stress: 0, axial_force: 0,
    })),
    gridExtent: 2, gridStep: 1, hiddenTruss3dMaterialIds: [], isModelMode: true,
    memberDraftNodes: [], selectedTruss3dElement: null, selectedTruss3dNode: null,
    selectedTruss3dNodeIndices: nodes.filter((_, index) => index % 2 === 0).map((node) => node.index),
    showGrid: false, showNodes: true, truss3dElementColors: [], truss3dLinkMode: false,
    deformationViewMode: "original",
  };
}
