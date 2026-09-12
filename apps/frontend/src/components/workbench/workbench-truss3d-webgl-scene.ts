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

export type Truss3dSceneBounds = {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  minZ: number;
  maxZ: number;
  spanX: number;
  spanY: number;
  spanZ: number;
  diagonal: number;
  center: { x: number; y: number; z: number };
};

function createColorResolver() {
  const parser = typeof document === "undefined" ? null : document.createElement("canvas").getContext("2d");
  const colors = new Map<string, [number, number, number, number]>();
  return (input: string, fallback: [number, number, number, number]): [number, number, number, number] => {
    if (!parser) return fallback;
    const cached = colors.get(input);
    if (cached) return cached;
    // Invalid CSS must not inherit the previous member's color.
    parser.fillStyle = `rgba(${fallback[0] * 255}, ${fallback[1] * 255}, ${fallback[2] * 255}, ${fallback[3]})`;
    parser.fillStyle = input;
    const normalized = parser.fillStyle;
    let color = fallback;
    if (typeof normalized === "string") {
      const hex = normalized.match(/^#([0-9a-f]{6})([0-9a-f]{2})?$/i);
      const rgb = normalized.match(/^rgba?\(([^)]+)\)$/);
      if (hex) {
        color = [0, 2, 4].map((offset) => Number.parseInt(hex[1].slice(offset, offset + 2), 16) / 255)
          .concat(hex[2] ? Number.parseInt(hex[2], 16) / 255 : 1) as typeof color;
      } else if (rgb) {
        const [r = 0, g = 0, b = 0, a = 1] = rgb[1].split(",").map((part) => Number.parseFloat(part.trim()));
        if ([r, g, b, a].every(Number.isFinite)) color = [r / 255, g / 255, b / 255, a];
      }
    }
    colors.set(input, color);
    return color;
  };
}

function withAlpha(color: [number, number, number, number], alpha: number): [number, number, number, number] {
  return [color[0], color[1], color[2], alpha];
}

export function computeTruss3dSceneBounds(nodes: Array<{ x: number; y: number; z: number }>): Truss3dSceneBounds {
  if (nodes.length === 0) {
    return {
      minX: 0,
      maxX: 0,
      minY: 0,
      maxY: 0,
      minZ: 0,
      maxZ: 0,
      spanX: 0,
      spanY: 0,
      spanZ: 0,
      diagonal: 0,
      center: { x: 0, y: 0, z: 0 },
    };
  }
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity, minZ = Infinity, maxZ = -Infinity;
  for (const node of nodes) {
    minX = Math.min(minX, node.x);
    maxX = Math.max(maxX, node.x);
    minY = Math.min(minY, node.y);
    maxY = Math.max(maxY, node.y);
    minZ = Math.min(minZ, node.z);
    maxZ = Math.max(maxZ, node.z);
  }
  const spanX = maxX - minX;
  const spanY = maxY - minY;
  const spanZ = maxZ - minZ;
  return {
    minX,
    maxX,
    minY,
    maxY,
    minZ,
    maxZ,
    spanX,
    spanY,
    spanZ,
    diagonal: Math.hypot(spanX, spanY, spanZ),
    center: { x: (minX + maxX) / 2, y: (minY + maxY) / 2, z: (minZ + maxZ) / 2 },
  };
}

export function resolveDeformationScale(nodes: DisplayTruss3dNode[], enabled: boolean) {
  if (!enabled || nodes.length === 0) return 1;
  let maxDisplacement = 0;
  for (const node of nodes) maxDisplacement = Math.max(maxDisplacement, Math.hypot(node.ux, node.uy, node.uz));
  if (maxDisplacement <= 1.0e-9) return 1;
  const bounds = computeTruss3dSceneBounds(nodes);
  const span = Math.max(bounds.spanX, bounds.spanY, bounds.spanZ, 1);
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

function pushBoundingBox(positions: number[], colors: number[], bounds: Truss3dSceneBounds, color: [number, number, number, number]) {
  if (bounds.diagonal <= 1.0e-9) return;
  const { minX, maxX, minY, maxY, minZ, maxZ } = bounds;
  const corners = [
    { x: minX, y: minY, z: minZ },
    { x: maxX, y: minY, z: minZ },
    { x: maxX, y: maxY, z: minZ },
    { x: minX, y: maxY, z: minZ },
    { x: minX, y: minY, z: maxZ },
    { x: maxX, y: minY, z: maxZ },
    { x: maxX, y: maxY, z: maxZ },
    { x: minX, y: maxY, z: maxZ },
  ];
  for (const [startIndex, endIndex] of [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4],
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
  ] as const) {
    pushSegment(positions, colors, corners[startIndex], corners[endIndex], color);
  }
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
  const parseCssColor = createColorResolver();
  const selectedNodes = new Set(args.selectedTruss3dNodeIndices);
  const draftNodes = new Set(args.memberDraftNodes);
  const hiddenMaterials = new Set(args.hiddenTruss3dMaterialIds);
  const gridColor = parseCssColor("rgba(132, 146, 166, 0.22)", [0.52, 0.57, 0.65, 0.22]);
  const boundsColor = parseCssColor("rgba(93, 217, 255, 0.18)", [0.36, 0.85, 1, 0.18]);
  const selectedColor = parseCssColor("rgba(255, 184, 77, 1)", [1, 0.72, 0.3, 1]);
  const draftColor = parseCssColor("rgba(93, 217, 255, 1)", [0.36, 0.85, 1, 1]);
  const nodeColor = parseCssColor("rgba(222, 229, 239, 0.95)", [0.87, 0.9, 0.94, 0.95]);
  const hiddenNodeColor = parseCssColor("rgba(222, 229, 239, 0.45)", [0.87, 0.9, 0.94, 0.45]);
  const showDeformation = !args.isModelMode;
  const deformationScale = resolveDeformationScale(args.visibleTruss3dNodes, showDeformation);
  const canShowDeformation = showDeformation && deformationScale > 1;
  const showOriginal = !canShowDeformation || args.deformationViewMode !== "deformed";
  const showDeformed = canShowDeformation && args.deformationViewMode !== "original";

  if (args.showGrid && showOriginal && Number.isFinite(args.gridExtent) && args.gridExtent > 0 && Number.isFinite(args.gridStep) && args.gridStep > 0) {
    const step = Math.max(args.gridStep, args.gridExtent / 64);
    const steps = Math.min(128, Math.floor((args.gridExtent * 2) / step));
    for (let index = 0; index <= steps; index += 1) {
      const value = -args.gridExtent + index * step;
      pushSegment(linePositions, lineColors, { x: -args.gridExtent, y: value, z: 0 }, { x: args.gridExtent, y: value, z: 0 }, gridColor);
      pushSegment(linePositions, lineColors, { x: value, y: -args.gridExtent, z: 0 }, { x: value, y: args.gridExtent, z: 0 }, gridColor);
    }
    pushBoundingBox(linePositions, lineColors, computeTruss3dSceneBounds(args.visibleTruss3dNodes), boundsColor);
  }

  args.visibleTruss3dElements.forEach((element) => {
    if (element.material_id && hiddenMaterials.has(element.material_id)) return;
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
    args.visibleTruss3dNodes.forEach((node) => {
      const isSelected = selectedNodes.has(node.index) || args.selectedTruss3dNode === node.index;
      const isDraft = draftNodes.has(node.index);
      if (showOriginal || showDeformed) nodeSizes.push(isSelected ? 12 : 9);
      if (showOriginal) {
        nodePositions.push(node.x, node.y, node.z);
        nodeColors.push(...(isSelected ? selectedColor : isDraft ? draftColor : args.truss3dLinkMode ? hiddenNodeColor : showDeformed ? withAlpha(nodeColor, 0.35) : nodeColor));
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
