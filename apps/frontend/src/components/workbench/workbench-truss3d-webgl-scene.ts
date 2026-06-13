"use client";

import type { DisplayTruss3dElement, DisplayTruss3dNode } from "@/components/workbench/workbench-viewport-core";

export type DeformationViewMode = "overlay" | "original" | "deformed";

export type Truss3dSceneBuildArgs = {
  displayTruss3dNodes: DisplayTruss3dNode[];
  gridExtent: number;
  gridStep: number;
  hiddenTruss3dMaterialIds: string[];
  isModelMode: boolean;
  memberDraftNodes: number[];
  selectedTruss3dElement: number | null;
  selectedTruss3dNode: number | null;
  selectedTruss3dNodeIndices: number[];
  showGrid: boolean;
  showNodes: boolean;
  truss3dElementColors: string[];
  truss3dLinkMode: boolean;
  visibleTruss3dElements: DisplayTruss3dElement[];
  visibleTruss3dNodes: DisplayTruss3dNode[];
  deformationViewMode: DeformationViewMode;
};

export type SceneBufferSet = {
  linePositions: Float32Array;
  lineColors: Float32Array;
  nodePositions: Float32Array;
  nodeColors: Float32Array;
  nodeSizes: Float32Array;
  deformedLinePositions: Float32Array;
  deformedLineColors: Float32Array;
  deformedNodePositions: Float32Array;
  deformedNodeColors: Float32Array;
  deformationScale: number;
};

function parseCssColor(input: string, fallback: [number, number, number, number]): [number, number, number, number] {
  if (typeof document === "undefined") return fallback;
  const parser = document.createElement("canvas").getContext("2d");
  if (!parser) return fallback;
  parser.fillStyle = input;
  const normalized = parser.fillStyle;
  if (typeof normalized !== "string") return fallback;
  const match = normalized.match(/^rgba?\(([^)]+)\)$/);
  if (!match) return fallback;
  const parts = match[1].split(",").map((part) => Number.parseFloat(part.trim()));
  const [r = 0, g = 0, b = 0, a = 1] = parts;
  return [r / 255, g / 255, b / 255, a];
}

function withAlpha(color: [number, number, number, number], alpha: number): [number, number, number, number] {
  return [color[0], color[1], color[2], alpha];
}

export function resolveDeformationScale(nodes: DisplayTruss3dNode[], enabled: boolean) {
  if (!enabled || nodes.length === 0) return 1;
  const maxDisplacement = Math.max(...nodes.map((node) => Math.hypot(node.ux, node.uy, node.uz)), 0);
  if (maxDisplacement <= 1.0e-9) return 1;
  const xs = nodes.map((node) => node.x);
  const ys = nodes.map((node) => node.y);
  const zs = nodes.map((node) => node.z);
  const span = Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys), Math.max(...zs) - Math.min(...zs), 1);
  return Math.min(24, Math.max(1, (span * 0.18) / maxDisplacement));
}

function pushSegment(
  positions: number[],
  colors: number[],
  start: { x: number; y: number; z: number },
  end: { x: number; y: number; z: number },
  color: [number, number, number, number],
) {
  positions.push(start.x, start.y, start.z, end.x, end.y, end.z);
  colors.push(...color, ...color);
}

export function buildTruss3dSceneBuffers(args: Truss3dSceneBuildArgs): SceneBufferSet {
  const linePositions: number[] = [];
  const lineColors: number[] = [];
  const nodePositions: number[] = [];
  const nodeColors: number[] = [];
  const nodeSizes: number[] = [];
  const deformedLinePositions: number[] = [];
  const deformedLineColors: number[] = [];
  const deformedNodePositions: number[] = [];
  const deformedNodeColors: number[] = [];
  const gridColor = parseCssColor("rgba(132, 146, 166, 0.22)", [0.52, 0.57, 0.65, 0.22]);
  const selectedColor = parseCssColor("rgba(255, 184, 77, 1)", [1, 0.72, 0.3, 1]);
  const draftColor = parseCssColor("rgba(93, 217, 255, 1)", [0.36, 0.85, 1, 1]);
  const nodeColor = parseCssColor("rgba(222, 229, 239, 0.95)", [0.87, 0.9, 0.94, 0.95]);
  const hiddenNodeColor = parseCssColor("rgba(222, 229, 239, 0.45)", [0.87, 0.9, 0.94, 0.45]);
  const showDeformation = !args.isModelMode;
  const deformationScale = resolveDeformationScale(args.visibleTruss3dNodes, showDeformation);
  const showOriginal = args.deformationViewMode !== "deformed";
  const showDeformed = showDeformation && deformationScale > 1 && args.deformationViewMode !== "original";

  if (args.showGrid && showOriginal) {
    for (let value = -args.gridExtent; value <= args.gridExtent + 1.0e-6; value += args.gridStep) {
      pushSegment(linePositions, lineColors, { x: -args.gridExtent, y: value, z: 0 }, { x: args.gridExtent, y: value, z: 0 }, gridColor);
      pushSegment(linePositions, lineColors, { x: value, y: -args.gridExtent, z: 0 }, { x: value, y: args.gridExtent, z: 0 }, gridColor);
    }
  }

  args.visibleTruss3dElements.forEach((element) => {
    if (element.material_id && args.hiddenTruss3dMaterialIds.includes(element.material_id)) return;
    const start = args.displayTruss3dNodes[element.node_i];
    const end = args.displayTruss3dNodes[element.node_j];
    if (!start || !end) return;
    const base = parseCssColor(args.truss3dElementColors[element.index] ?? "rgba(122, 154, 255, 1)", [0.48, 0.6, 1, 1]);
    if (showOriginal) {
      const color = args.selectedTruss3dElement === element.index ? selectedColor : showDeformed ? withAlpha(base, 0.2) : base;
      pushSegment(linePositions, lineColors, start, end, color);
    }
    if (showDeformed) {
      const deformedStart = { x: start.x + start.ux * deformationScale, y: start.y + start.uy * deformationScale, z: start.z + start.uz * deformationScale };
      const deformedEnd = { x: end.x + end.ux * deformationScale, y: end.y + end.uy * deformationScale, z: end.z + end.uz * deformationScale };
      pushSegment(deformedLinePositions, deformedLineColors, deformedStart, deformedEnd, args.selectedTruss3dElement === element.index ? selectedColor : base);
    }
  });

  if (args.showNodes) {
    args.visibleTruss3dNodes.forEach((node, index) => {
      const isSelected = args.selectedTruss3dNodeIndices.includes(index) || args.selectedTruss3dNode === index;
      const isDraft = args.memberDraftNodes.includes(index);
      if (showOriginal) {
        nodePositions.push(node.x, node.y, node.z);
        nodeColors.push(...(isSelected ? selectedColor : isDraft ? draftColor : args.truss3dLinkMode ? hiddenNodeColor : showDeformed ? withAlpha(nodeColor, 0.35) : nodeColor));
        nodeSizes.push(isSelected ? 12 : 9);
      }
      if (showDeformed) {
        deformedNodePositions.push(node.x + node.ux * deformationScale, node.y + node.uy * deformationScale, node.z + node.uz * deformationScale);
        deformedNodeColors.push(...(isSelected ? selectedColor : isDraft ? draftColor : nodeColor));
      }
    });
  }

  return {
    linePositions: new Float32Array(linePositions),
    lineColors: new Float32Array(lineColors),
    nodePositions: new Float32Array(nodePositions),
    nodeColors: new Float32Array(nodeColors),
    nodeSizes: new Float32Array(nodeSizes),
    deformedLinePositions: new Float32Array(deformedLinePositions),
    deformedLineColors: new Float32Array(deformedLineColors),
    deformedNodePositions: new Float32Array(deformedNodePositions),
    deformedNodeColors: new Float32Array(deformedNodeColors),
    deformationScale,
  };
}
