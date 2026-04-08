"use client";

import { memo, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from "react";

type SidebarSection = "study" | "model" | "library" | "system";
type StudyKind = "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";

type DisplayTrussNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  ux: number;
  uy: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

type DisplayTrussElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
  material_id?: string;
};

type DisplayTruss3dNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  z: number;
  ux: number;
  uy: number;
  uz: number;
};

type DisplayTruss3dElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
  material_id?: string;
};

type PlaneNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  ux: number;
  uy: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

type PlaneElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  von_mises?: number;
  material_id?: string;
};

type Bounds = {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  width: number;
  height: number;
};

type WorkbenchViewportProps = {
  studyKind: StudyKind;
  sidebarSection: SidebarSection;
  title: string;
  axialTitle: string;
  trussTitle: string;
  truss3dTitle: string;
  planeTitle: string;
  axialNodes: Array<{ x: number; displacement: number }>;
  axialLength: number;
  axialScale: number;
  displayTrussNodes: DisplayTrussNode[];
  displayTrussElements: DisplayTrussElement[];
  trussElementColors: string[];
  hiddenTrussMaterialIds: string[];
  trussBounds: Bounds;
  trussResult: boolean;
  trussHotspotNodes: number[];
  trussNodeIssues: Record<number, string[]>;
  selectedNode: number | null;
  selectedElement: number | null;
  memberDraftNodes: number[];
  onTrussPointerMove: (event: ReactPointerEvent<SVGSVGElement>) => void;
  onStopDraggingNode: () => void;
  onSelectTrussElement: (index: number) => void;
  onStartTrussNodeDrag: (index: number) => void;
  displayTruss3dNodes: DisplayTruss3dNode[];
  displayTruss3dElements: DisplayTruss3dElement[];
  truss3dElementColors: string[];
  hiddenTruss3dMaterialIds: string[];
  planeNodes: PlaneNode[];
  planeElements: PlaneElement[];
  planeElementColors: string[];
  hiddenPlaneMaterialIds: string[];
  planeBounds: Bounds;
  planeResult: boolean;
  planeMaxVonMises: number;
  selectedPlaneNodeId: string | null;
  onSelectPlaneElement: (index: number) => void;
  onSelectPlaneNode: (index: number) => void;
  selectedTruss3dNode: number | null;
  selectedTruss3dNodeIndices: number[];
  selectedTruss3dElement: number | null;
  onSelectTruss3dNode: (index: number) => void;
  onSelectTruss3dNodes: (indices: number[], append: boolean) => void;
  onSelectTruss3dElement: (index: number) => void;
  onUpdateTruss3dNodePosition: (index: number, position: { x: number; y: number; z: number }) => void;
  onBeginTruss3dNodeDrag: () => void;
  onEndTruss3dNodeDrag: () => void;
  workspaceBadge: string;
  truss3dLinkMode: boolean;
  immersiveViewport: boolean;
  projectionMode: ProjectionMode;
  showGrid: boolean;
  showLabels: boolean;
  showNodes: boolean;
  boxSelectMode: boolean;
  activeViewPreset: ViewPreset;
  focusRequestVersion: number;
  resetRequestVersion: number;
  showShortcutHints: boolean;
  shortcutLegendTitle: string;
  shortcutLegendRows: string[];
  onProjectionModeChange: (mode: ProjectionMode) => void;
  onShowGridChange: (value: boolean) => void;
  onShowLabelsChange: (value: boolean) => void;
  onShowNodesChange: (value: boolean) => void;
  onBoxSelectModeChange: (value: boolean) => void;
  viewportPixelWidth?: number;
};

const VIEWPORT_CLIP = { x: 48, y: 76, width: 884, height: 340 };

function toSvgPoint(node: { x: number; y: number }, bounds: Bounds) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;

  return {
    x: paddingX + ((node.x - bounds.minX) / bounds.width) * usableWidth,
    y: 460 - paddingY - ((node.y - bounds.minY) / bounds.height) * usableHeight,
  };
}

type CameraState = {
  yaw: number;
  pitch: number;
  zoom: number;
  panX: number;
  panY: number;
};

type ViewPreset = "iso" | "front" | "right" | "top";
type ProjectionMode = "ortho" | "persp";

function rotatePoint(node: { x: number; y: number; z: number }, camera: CameraState) {
  const cy = Math.cos(camera.yaw);
  const sy = Math.sin(camera.yaw);
  const cp = Math.cos(camera.pitch);
  const sp = Math.sin(camera.pitch);

  const x1 = node.x * cy - node.y * sy;
  const y1 = node.x * sy + node.y * cy;
  const z1 = node.z;

  return {
    x: x1,
    y: y1 * cp - z1 * sp,
    z: y1 * sp + z1 * cp,
  };
}

function buildProjectedBounds(nodes: DisplayTruss3dNode[], camera: CameraState) {
  const projected = nodes.map((node) => {
    const rotated = rotatePoint(node, camera);
    return { x: rotated.x, y: rotated.z };
  });

  const xs = projected.map((node) => node.x);
  const ys = projected.map((node) => node.y);
  const minX = Math.min(...xs, -1);
  const maxX = Math.max(...xs, 1);
  const minY = Math.min(...ys, -1);
  const maxY = Math.max(...ys, 1);
  const width = Math.max(maxX - minX, 1);
  const height = Math.max(maxY - minY, 1);

  return {
    minX,
    maxX,
    minY,
    maxY,
    width,
    height,
    projected,
  };
}

function cameraForPreset(preset: ViewPreset): CameraState {
  if (preset === "front") {
    return { yaw: 0, pitch: 0, zoom: 1, panX: 0, panY: 0 };
  }

  if (preset === "right") {
    return { yaw: Math.PI / 2, pitch: 0, zoom: 1, panX: 0, panY: 0 };
  }

  if (preset === "top") {
    return { yaw: 0, pitch: -Math.PI / 2, zoom: 1, panX: 0, panY: 0 };
  }

  return { yaw: -0.7, pitch: 0.55, zoom: 1, panX: 0, panY: 0 };
}

function pointerToViewport(event: ReactPointerEvent<SVGSVGElement>) {
  const rect = event.currentTarget.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left) / rect.width) * 980,
    y: ((event.clientY - rect.top) / rect.height) * 460,
  };
}

function projectTruss3dPoint(
  node: { x: number; y: number; z: number },
  bounds: Bounds,
  camera: CameraState,
  projection: ProjectionMode,
) {
  const rotated = rotatePoint(node, camera);
  const point = toSvgPoint({ x: rotated.x, y: rotated.z }, bounds);
  const perspectiveScale = projection === "persp" ? Math.max(0.64, Math.min(1.42, 8 / (8 + rotated.y))) : 1;
  return {
    x: point.x * camera.zoom * perspectiveScale + camera.panX,
    y: point.y * camera.zoom * perspectiveScale + camera.panY,
    depth: rotated.y,
    scale: perspectiveScale,
  };
}

function rotatedDeltaToWorld(deltaX: number, deltaZ: number, camera: CameraState) {
  const cy = Math.cos(camera.yaw);
  const sy = Math.sin(camera.yaw);
  const cp = Math.cos(camera.pitch);
  const sp = Math.sin(camera.pitch);

  const x1 = deltaX;
  const yTemp = deltaZ * sp;
  const z = deltaZ * cp;

  return {
    x: x1 * cy + yTemp * sy,
    y: -x1 * sy + yTemp * cy,
    z,
  };
}

function planeStressFill(value: number, maxValue: number): string {
  const normalized = maxValue > 0 ? Math.max(0, Math.min(1, value / maxValue)) : 0;
  const hue = 205 - normalized * 180;
  const lightness = 72 - normalized * 22;
  return `hsla(${hue}, 72%, ${lightness}%, 0.72)`;
}

function renderSupportGlyph(
  point: { x: number; y: number },
  constraints: { fix_x: boolean; fix_y: boolean },
  key: string,
) {
  if (!constraints.fix_x && !constraints.fix_y) return null;

  return (
    <g key={key} className="support-glyph">
      {constraints.fix_y ? <line x1={point.x - 12} y1={point.y + 14} x2={point.x + 12} y2={point.y + 14} /> : null}
      {constraints.fix_x ? <line x1={point.x - 14} y1={point.y - 12} x2={point.x - 14} y2={point.y + 12} /> : null}
    </g>
  );
}

function renderLoadGlyph(
  point: { x: number; y: number },
  load: { load_x: number; load_y: number },
  key: string,
) {
  if (Math.abs(load.load_x) < 1.0e-9 && Math.abs(load.load_y) < 1.0e-9) return null;

  const scale = 0.01;
  const x2 = point.x + load.load_x * scale;
  const y2 = point.y - load.load_y * scale;

  return (
    <g key={key} className="load-glyph">
      <line x1={point.x} y1={point.y} x2={x2} y2={y2} />
      <circle cx={x2} cy={y2} r={3.5} />
    </g>
  );
}

function pointInsideViewport(point: { x: number; y: number }, margin = 18) {
  return (
    point.x >= VIEWPORT_CLIP.x - margin &&
    point.x <= VIEWPORT_CLIP.x + VIEWPORT_CLIP.width + margin &&
    point.y >= VIEWPORT_CLIP.y - margin &&
    point.y <= VIEWPORT_CLIP.y + VIEWPORT_CLIP.height + margin
  );
}

function lineInsideViewport(start: { x: number; y: number }, end: { x: number; y: number }, margin = 24) {
  const minX = Math.min(start.x, end.x);
  const maxX = Math.max(start.x, end.x);
  const minY = Math.min(start.y, end.y);
  const maxY = Math.max(start.y, end.y);

  return !(
    maxX < VIEWPORT_CLIP.x - margin ||
    minX > VIEWPORT_CLIP.x + VIEWPORT_CLIP.width + margin ||
    maxY < VIEWPORT_CLIP.y - margin ||
    minY > VIEWPORT_CLIP.y + VIEWPORT_CLIP.height + margin
  );
}

function polygonInsideViewport(points: Array<{ x: number; y: number }>, margin = 24) {
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  return !(
    Math.max(...xs) < VIEWPORT_CLIP.x - margin ||
    Math.min(...xs) > VIEWPORT_CLIP.x + VIEWPORT_CLIP.width + margin ||
    Math.max(...ys) < VIEWPORT_CLIP.y - margin ||
    Math.min(...ys) > VIEWPORT_CLIP.y + VIEWPORT_CLIP.height + margin
  );
}

function stepForDensity(length: number, softLimit: number) {
  return length > softLimit ? Math.ceil(length / softLimit) : 1;
}

function initialRenderBudget(length: number) {
  if (length >= 20_000) return 320;
  if (length >= 10_000) return 280;
  if (length >= 2_000) return 240;
  if (length >= 800) return 200;
  return length;
}

function renderBatchSize(length: number) {
  if (length >= 20_000) return 640;
  if (length >= 10_000) return 560;
  if (length >= 2_000) return 420;
  if (length >= 800) return 320;
  return length;
}

function WorkbenchViewportInner({
  studyKind,
  sidebarSection,
  title,
  axialTitle,
  trussTitle,
  truss3dTitle,
  planeTitle,
  axialNodes,
  axialLength,
  axialScale,
  displayTrussNodes,
  displayTrussElements,
  trussElementColors,
  hiddenTrussMaterialIds,
  trussBounds,
  trussResult,
  trussHotspotNodes,
  trussNodeIssues,
  selectedNode,
  selectedElement,
  memberDraftNodes,
  onTrussPointerMove,
  onStopDraggingNode,
  onSelectTrussElement,
  onStartTrussNodeDrag,
  displayTruss3dNodes,
  displayTruss3dElements,
  truss3dElementColors,
  hiddenTruss3dMaterialIds,
  planeNodes,
  planeElements,
  planeElementColors,
  hiddenPlaneMaterialIds,
  planeBounds,
  planeResult,
  planeMaxVonMises,
  selectedPlaneNodeId,
  onSelectPlaneElement,
  onSelectPlaneNode,
  selectedTruss3dNode,
  selectedTruss3dNodeIndices,
  selectedTruss3dElement,
  onSelectTruss3dNode,
  onSelectTruss3dNodes,
  onSelectTruss3dElement,
  onUpdateTruss3dNodePosition,
  onBeginTruss3dNodeDrag,
  onEndTruss3dNodeDrag,
  workspaceBadge,
  truss3dLinkMode,
  immersiveViewport,
  projectionMode,
  showGrid,
  showLabels,
  showNodes,
  boxSelectMode,
  activeViewPreset,
  focusRequestVersion,
  resetRequestVersion,
  showShortcutHints,
  shortcutLegendTitle,
  shortcutLegendRows,
  onProjectionModeChange,
  onShowGridChange,
  onShowLabelsChange,
  onShowNodesChange,
  onBoxSelectModeChange,
  viewportPixelWidth,
}: WorkbenchViewportProps) {
  const svgStyle = viewportPixelWidth ? { width: `${viewportPixelWidth}px`, minWidth: `${viewportPixelWidth}px` } : undefined;
  const isModelMode = sidebarSection === "model";
  const [trussElementRenderLimit, setTrussElementRenderLimit] = useState(displayTrussElements.length);
  const [trussNodeRenderLimit, setTrussNodeRenderLimit] = useState(displayTrussNodes.length);
  const [truss3dElementRenderLimit, setTruss3dElementRenderLimit] = useState(displayTruss3dElements.length);
  const [truss3dNodeRenderLimit, setTruss3dNodeRenderLimit] = useState(displayTruss3dNodes.length);
  const [planeElementRenderLimit, setPlaneElementRenderLimit] = useState(planeElements.length);
  const [planeNodeRenderLimit, setPlaneNodeRenderLimit] = useState(planeNodes.length);
  const progressiveRenderFrameRef = useRef<number | null>(null);
  const [camera, setCamera] = useState<CameraState>(cameraForPreset("iso"));
  const dragModeRef = useRef<"orbit" | "pan" | null>(null);
  const dragNode3dRef = useRef<number | null>(null);
  const dragAxisRef = useRef<"x" | "y" | "z" | null>(null);
  const pointerRef = useRef<{ x: number; y: number } | null>(null);
  const selectionAppendRef = useRef(false);
  const [hoveredTruss3dNode, setHoveredTruss3dNode] = useState<number | null>(null);
  const [selectionRect, setSelectionRect] = useState<{ x: number; y: number; width: number; height: number } | null>(null);
  const selectionStartRef = useRef<{ x: number; y: number } | null>(null);
  const projected3d = buildProjectedBounds(displayTruss3dNodes, camera);
  const selected3dNodeData = selectedTruss3dNode !== null ? displayTruss3dNodes[selectedTruss3dNode] : null;
  const draftStartNodeIndex = memberDraftNodes[0] ?? null;
  const draftStartNode =
    truss3dLinkMode && draftStartNodeIndex !== null ? displayTruss3dNodes[draftStartNodeIndex] ?? null : null;
  const visibleTrussElements = displayTrussElements.slice(0, trussElementRenderLimit);
  const visibleTrussNodes = displayTrussNodes.slice(0, trussNodeRenderLimit);
  const visibleTruss3dElements = displayTruss3dElements.slice(0, truss3dElementRenderLimit);
  const visibleTruss3dNodes = displayTruss3dNodes.slice(0, truss3dNodeRenderLimit);
  const visiblePlaneElements = planeElements.slice(0, planeElementRenderLimit);
  const visiblePlaneNodes = planeNodes.slice(0, planeNodeRenderLimit);
  const trussLabelStep = stepForDensity(visibleTrussNodes.length, isModelMode ? 22 : 12);
  const trussMarkerStep = stepForDensity(visibleTrussNodes.length, isModelMode ? 160 : 84);
  const trussDeformedStep = stepForDensity(visibleTrussElements.length, 180);
  const truss3dLabelStep = stepForDensity(visibleTruss3dNodes.length, 12);
  const planeNodeLabelStep = stepForDensity(visiblePlaneNodes.length, isModelMode ? 18 : 10);
  const planeNodeMarkerStep = stepForDensity(visiblePlaneNodes.length, isModelMode ? 128 : 72);
  const planeDeformedStep = stepForDensity(visiblePlaneElements.length, 120);

  useEffect(() => {
    const targets = {
      trussElements: displayTrussElements.length,
      trussNodes: displayTrussNodes.length,
      truss3dElements: displayTruss3dElements.length,
      truss3dNodes: displayTruss3dNodes.length,
      planeElements: planeElements.length,
      planeNodes: planeNodes.length,
    };

    setTrussElementRenderLimit(initialRenderBudget(targets.trussElements));
    setTrussNodeRenderLimit(initialRenderBudget(targets.trussNodes));
    setTruss3dElementRenderLimit(initialRenderBudget(targets.truss3dElements));
    setTruss3dNodeRenderLimit(initialRenderBudget(targets.truss3dNodes));
    setPlaneElementRenderLimit(initialRenderBudget(targets.planeElements));
    setPlaneNodeRenderLimit(initialRenderBudget(targets.planeNodes));

    if (progressiveRenderFrameRef.current !== null) {
      window.cancelAnimationFrame(progressiveRenderFrameRef.current);
    }

    const advance = () => {
      let complete = true;

      setTrussElementRenderLimit((current) => {
        const next = Math.min(targets.trussElements, current + renderBatchSize(targets.trussElements));
        complete &&= next >= targets.trussElements;
        return next;
      });
      setTrussNodeRenderLimit((current) => {
        const next = Math.min(targets.trussNodes, current + renderBatchSize(targets.trussNodes));
        complete &&= next >= targets.trussNodes;
        return next;
      });
      setTruss3dElementRenderLimit((current) => {
        const next = Math.min(targets.truss3dElements, current + renderBatchSize(targets.truss3dElements));
        complete &&= next >= targets.truss3dElements;
        return next;
      });
      setTruss3dNodeRenderLimit((current) => {
        const next = Math.min(targets.truss3dNodes, current + renderBatchSize(targets.truss3dNodes));
        complete &&= next >= targets.truss3dNodes;
        return next;
      });
      setPlaneElementRenderLimit((current) => {
        const next = Math.min(targets.planeElements, current + renderBatchSize(targets.planeElements));
        complete &&= next >= targets.planeElements;
        return next;
      });
      setPlaneNodeRenderLimit((current) => {
        const next = Math.min(targets.planeNodes, current + renderBatchSize(targets.planeNodes));
        complete &&= next >= targets.planeNodes;
        return next;
      });

      if (!complete) {
        progressiveRenderFrameRef.current = window.requestAnimationFrame(advance);
      } else {
        progressiveRenderFrameRef.current = null;
      }
    };

    const needsProgressive =
      targets.trussElements > initialRenderBudget(targets.trussElements) ||
      targets.trussNodes > initialRenderBudget(targets.trussNodes) ||
      targets.truss3dElements > initialRenderBudget(targets.truss3dElements) ||
      targets.truss3dNodes > initialRenderBudget(targets.truss3dNodes) ||
      targets.planeElements > initialRenderBudget(targets.planeElements) ||
      targets.planeNodes > initialRenderBudget(targets.planeNodes);

    if (needsProgressive) {
      progressiveRenderFrameRef.current = window.requestAnimationFrame(advance);
    }

    return () => {
      if (progressiveRenderFrameRef.current !== null) {
        window.cancelAnimationFrame(progressiveRenderFrameRef.current);
        progressiveRenderFrameRef.current = null;
      }
    };
  }, [
    displayTrussElements.length,
    displayTrussNodes.length,
    displayTruss3dElements.length,
    displayTruss3dNodes.length,
    planeElements.length,
    planeNodes.length,
  ]);

  useEffect(() => {
    setCamera((current) => ({ ...cameraForPreset(activeViewPreset), zoom: current.zoom }));
  }, [activeViewPreset]);

  const handle3dPointerDown = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (boxSelectMode) {
      event.currentTarget.setPointerCapture(event.pointerId);
      const point = pointerToViewport(event);
      selectionAppendRef.current = event.shiftKey || event.metaKey || event.ctrlKey;
      selectionStartRef.current = point;
      setSelectionRect({ x: point.x, y: point.y, width: 0, height: 0 });
      return;
    }
    dragModeRef.current = event.altKey ? "orbit" : "pan";
    pointerRef.current = { x: event.clientX, y: event.clientY };
  };

  const handle3dPointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (selectionStartRef.current) {
      const start = selectionStartRef.current;
      const current = pointerToViewport(event);
      setSelectionRect({
        x: Math.min(start.x, current.x),
        y: Math.min(start.y, current.y),
        width: Math.abs(current.x - start.x),
        height: Math.abs(current.y - start.y),
      });
      return;
    }

    if (!pointerRef.current) return;
    const dx = event.clientX - pointerRef.current.x;
    const dy = event.clientY - pointerRef.current.y;
    pointerRef.current = { x: event.clientX, y: event.clientY };

    if (dragNode3dRef.current !== null) {
      const targetIndex = dragNode3dRef.current;
      const target = displayTruss3dNodes[targetIndex];
      if (!target) return;
      onBeginTruss3dNodeDrag();
      const usableWidth = 980 - 120 * 2;
      const usableHeight = 460 - 80 * 2;
      const factor = projectionMode === "persp" ? Math.max(0.64, Math.min(1.42, 8 / (8 + rotatePoint(target, camera).y))) : 1;
      const deltaRotatedX = (dx / Math.max(camera.zoom * factor, 0.01)) * (projected3d.width / usableWidth);
      const deltaRotatedZ = (-dy / Math.max(camera.zoom * factor, 0.01)) * (projected3d.height / usableHeight);
      const deltaWorld = rotatedDeltaToWorld(deltaRotatedX, deltaRotatedZ, camera);
      onUpdateTruss3dNodePosition(targetIndex, {
        x: target.x + deltaWorld.x,
        y: target.y + deltaWorld.y,
        z: target.z + deltaWorld.z,
      });
      return;
    }

    if (dragAxisRef.current !== null && selectedTruss3dNode !== null) {
      const target = displayTruss3dNodes[selectedTruss3dNode];
      if (!target) return;
      onBeginTruss3dNodeDrag();
      const axis = dragAxisRef.current;
      const origin = projectTruss3dPoint(target, projected3d, camera, projectionMode);
      const axisTarget = projectTruss3dPoint(
        {
          ...target,
          [axis]: target[axis] + 1,
        },
        projected3d,
        camera,
        projectionMode,
      );
      const vectorX = axisTarget.x - origin.x;
      const vectorY = axisTarget.y - origin.y;
      const lengthSquared = vectorX * vectorX + vectorY * vectorY;
      if (lengthSquared <= 1.0e-9) return;
      const delta = (dx * vectorX + dy * vectorY) / lengthSquared;
      onUpdateTruss3dNodePosition(selectedTruss3dNode, {
        x: target.x + (axis === "x" ? delta : 0),
        y: target.y + (axis === "y" ? delta : 0),
        z: target.z + (axis === "z" ? delta : 0),
      });
      return;
    }
    if (!dragModeRef.current) return;

    setCamera((current) =>
      dragModeRef.current === "pan"
        ? { ...current, panX: current.panX + dx, panY: current.panY + dy }
        : {
            ...current,
            yaw: current.yaw + dx * 0.008,
            pitch: Math.max(-1.35, Math.min(1.35, current.pitch - dy * 0.008)),
          },
    );
  };

  const stop3dPointer = (event?: ReactPointerEvent<SVGSVGElement>) => {
    dragModeRef.current = null;
    if (dragNode3dRef.current !== null) {
      onEndTruss3dNodeDrag();
    }
    if (dragAxisRef.current !== null) {
      onEndTruss3dNodeDrag();
    }
    if (selectionStartRef.current && selectionRect) {
      const minX = selectionRect.x;
      const maxX = selectionRect.x + selectionRect.width;
      const minY = selectionRect.y;
      const maxY = selectionRect.y + selectionRect.height;
      const selectedIndices = displayTruss3dNodes.flatMap((node, index) => {
        const point = projectTruss3dPoint(node, projected3d, camera, projectionMode);
        return point.x >= minX && point.x <= maxX && point.y >= minY && point.y <= maxY ? [index] : [];
      });
      onSelectTruss3dNodes(selectedIndices, selectionAppendRef.current);
    }
    if (event && event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    dragNode3dRef.current = null;
    dragAxisRef.current = null;
    pointerRef.current = null;
    selectionStartRef.current = null;
    selectionAppendRef.current = false;
    setSelectionRect(null);
  };

  const handle3dWheel = (event: ReactWheelEvent<SVGSVGElement>) => {
    event.preventDefault();
    if (event.shiftKey) {
      setCamera((current) => ({
        ...current,
        panX: current.panX - event.deltaY * 0.45,
      }));
      return;
    }
    const direction = event.deltaY > 0 ? 0.92 : 1.08;
    setCamera((current) => ({
      ...current,
      zoom: Math.max(0.55, Math.min(2.8, current.zoom * direction)),
    }));
  };

  const focusSelected3dNode = () => {
    const selectedNodes =
      selectedTruss3dNodeIndices.length > 0
        ? selectedTruss3dNodeIndices.map((index) => displayTruss3dNodes[index]).filter(Boolean)
        : selected3dNodeData
          ? [selected3dNodeData]
          : displayTruss3dNodes;

    if (selectedNodes.length === 0) return;

    const center = selectedNodes.reduce(
      (acc, node) => ({
        x: acc.x + node.x,
        y: acc.y + node.y,
        z: acc.z + node.z,
      }),
      { x: 0, y: 0, z: 0 },
    );

    const target = {
      x: center.x / selectedNodes.length,
      y: center.y / selectedNodes.length,
      z: center.z / selectedNodes.length,
    };

    setCamera((current) => {
      const bounds = buildProjectedBounds(displayTruss3dNodes, current);
      const point = projectTruss3dPoint(target, bounds, { ...current, panX: 0, panY: 0 }, projectionMode);
      const nextPanX = current.panX + (490 - point.x);
      const nextPanY = current.panY + (230 - point.y);

      return {
        ...current,
        panX: Number.isFinite(nextPanX) ? nextPanX : current.panX,
        panY: Number.isFinite(nextPanY) ? nextPanY : current.panY,
      };
    });
  };

  const resetCamera = () => {
    setCamera((current) => ({ ...cameraForPreset("iso"), zoom: current.zoom }));
  };

  useEffect(() => {
    if (focusRequestVersion > 0) {
      focusSelected3dNode();
    }
  }, [focusRequestVersion]);

  useEffect(() => {
    if (resetRequestVersion > 0) {
      resetCamera();
    }
  }, [resetRequestVersion]);

  const handle3dKeyDown = (event: ReactKeyboardEvent<SVGSVGElement>) => {
    const step = event.shiftKey ? 32 : 18;
    if (event.key === "1") setCamera((current) => ({ ...cameraForPreset("iso"), zoom: current.zoom }));
    if (event.key === "2") setCamera((current) => ({ ...cameraForPreset("front"), zoom: current.zoom }));
    if (event.key === "3") setCamera((current) => ({ ...cameraForPreset("right"), zoom: current.zoom }));
    if (event.key === "4") setCamera((current) => ({ ...cameraForPreset("top"), zoom: current.zoom }));
    if (event.key.toLowerCase() === "g") onShowGridChange(!showGrid);
    if (event.key.toLowerCase() === "l") onShowLabelsChange(!showLabels);
    if (event.key.toLowerCase() === "n") onShowNodesChange(!showNodes);
    if (event.key.toLowerCase() === "p") onProjectionModeChange(projectionMode === "ortho" ? "persp" : "ortho");
    if (event.key.toLowerCase() === "f") focusSelected3dNode();
    if (event.key.toLowerCase() === "r") resetCamera();
    if (event.key === "ArrowLeft" || event.key.toLowerCase() === "a") setCamera((current) => ({ ...current, panX: current.panX + step }));
    if (event.key === "ArrowRight" || event.key.toLowerCase() === "d") setCamera((current) => ({ ...current, panX: current.panX - step }));
    if (event.key === "ArrowUp" || event.key.toLowerCase() === "w") setCamera((current) => ({ ...current, panY: current.panY + step }));
    if (event.key === "ArrowDown" || event.key.toLowerCase() === "s") setCamera((current) => ({ ...current, panY: current.panY - step }));
  };

  const gridExtent = Math.max(
    ...displayTruss3dNodes.map((node) => Math.max(Math.abs(node.x), Math.abs(node.y))),
    2,
  );
  const gridStep = gridExtent > 6 ? 2 : 1;

  if (studyKind === "axial_bar_1d") {
    return (
      <svg viewBox="0 0 980 460" className="viewport-svg" aria-label="Axial bar response">
        <defs>
          <linearGradient id="beamGradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="var(--accent-cool)" />
            <stop offset="100%" stopColor="var(--accent)" />
          </linearGradient>
          <clipPath id="viewportClipAxial">
            <rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" />
          </clipPath>
        </defs>
        <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
        <text x="48" y="58" className="svg-title">
          {axialTitle}
        </text>
        <g clipPath="url(#viewportClipAxial)">
          <line x1="80" y1="160" x2="880" y2="160" className="guide" />
          <line x1="80" y1="360" x2="880" y2="360" className="guide guide--soft" />
          {axialNodes.length > 0 ? (
            <>
              <polyline
                points={axialNodes.map((node) => `${80 + (node.x / axialLength) * 800},160`).join(" ")}
                className="bar bar--base"
              />
              <polyline
                points={axialNodes.map((node) => `${80 + (node.x / axialLength) * 800 + node.displacement * axialScale},360`).join(" ")}
                className="bar bar--deformed"
              />
            </>
          ) : null}
        </g>
      </svg>
    );
  }

  if (studyKind === "truss_2d") {
    return (
      <svg
        viewBox="0 0 980 460"
        className={`viewport-svg${boxSelectMode ? " viewport-svg--box-select" : ""}`}
        style={svgStyle}
        aria-label="2d truss response"
        onPointerLeave={onStopDraggingNode}
        onPointerMove={onTrussPointerMove}
        onPointerUp={onStopDraggingNode}
      >
        <defs>
          <linearGradient id="beamGradientTruss" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="var(--accent-cool)" />
            <stop offset="100%" stopColor="var(--accent)" />
          </linearGradient>
          <clipPath id="viewportClipTruss">
            <rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" />
          </clipPath>
        </defs>
        <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
        <text x="48" y="58" className="svg-title">
          {isModelMode ? title : trussTitle}
        </text>
        <g clipPath="url(#viewportClipTruss)">
          {visibleTrussElements.map((element) => {
            if (element.material_id && hiddenTrussMaterialIds.includes(element.material_id)) return null;
            const start = toSvgPoint(displayTrussNodes[element.node_i], trussBounds);
            const end = toSvgPoint(displayTrussNodes[element.node_j], trussBounds);
            if (!lineInsideViewport(start, end)) return null;

            return (
              <line
                key={`base-${element.id}`}
                x1={start.x}
                y1={start.y}
                x2={end.x}
                y2={end.y}
                className={`bar bar--base${selectedElement === element.index ? " bar--selected" : ""}`}
                style={{ stroke: trussElementColors[element.index] }}
                onPointerDown={(event) => {
                  if (isModelMode) {
                    event.stopPropagation();
                    onSelectTrussElement(element.index);
                  }
                }}
              />
            );
          })}

          {trussResult
            ? visibleTrussElements.flatMap((element, index) => {
                if (element.material_id && hiddenTrussMaterialIds.includes(element.material_id)) return [];
                if (index % trussDeformedStep !== 0) return [];
                const start = toSvgPoint(
                  {
                    x: displayTrussNodes[element.node_i].x + displayTrussNodes[element.node_i].ux * 10000,
                    y: displayTrussNodes[element.node_i].y + displayTrussNodes[element.node_i].uy * 10000,
                  },
                  trussBounds,
                );
                const end = toSvgPoint(
                  {
                    x: displayTrussNodes[element.node_j].x + displayTrussNodes[element.node_j].ux * 10000,
                    y: displayTrussNodes[element.node_j].y + displayTrussNodes[element.node_j].uy * 10000,
                  },
                  trussBounds,
                );
                if (!lineInsideViewport(start, end, 80)) return [];

                return (
                  <line
                    key={`def-${element.id}`}
                    x1={start.x}
                    y1={start.y}
                    x2={end.x}
                    y2={end.y}
                    className="bar bar--deformed-truss"
                  />
                );
              })
            : null}

          {visibleTrussNodes.flatMap((node, index) => {
            const point = toSvgPoint(node, trussBounds);
            if (!pointInsideViewport(point, 26)) return [];
            const showLabel = index % trussLabelStep === 0 || selectedNode === index || memberDraftNodes.includes(index);
            const showMarker = index % trussMarkerStep === 0 || showLabel || (trussNodeIssues[index] ?? []).length > 0;
            if (!showMarker) return [];

            const deformed = toSvgPoint({ x: node.x + node.ux * 10000, y: node.y + node.uy * 10000 }, trussBounds);
            return (
              <g key={node.id}>
                {renderSupportGlyph(point, node, `support-${node.id}`)}
                {renderLoadGlyph(point, node, `load-${node.id}`)}
                {trussHotspotNodes.includes(index) ? (
                  <circle cx={point.x} cy={point.y} r={isModelMode ? 16 : 12} className="node-hotspot-ring" />
                ) : null}
                <circle
                  cx={point.x}
                  cy={point.y}
                  r={isModelMode ? 10 : 7}
                  className={`node-base${selectedNode === index ? " node-base--active" : ""}${memberDraftNodes.includes(index) ? " node-base--draft" : ""}${(trussNodeIssues[index] ?? []).length > 0 ? " node-base--warning" : ""}`}
                  onPointerDown={() => {
                    if (isModelMode) onStartTrussNodeDrag(index);
                  }}
                />
                {showLabel ? (
                  <text x={point.x + 12} y={point.y - 10} className="node-label">
                    {node.id}
                  </text>
                ) : null}
                {trussResult && pointInsideViewport(deformed, 80) ? (
                  <circle cx={deformed.x} cy={deformed.y} r={5} className="node-deformed" />
                ) : null}
              </g>
            );
          })}
        </g>
      </svg>
    );
  }

  if (studyKind === "truss_3d") {
    return (
      <svg
        viewBox="0 0 980 460"
        className="viewport-svg"
        style={svgStyle}
        aria-label="3d truss response"
        onPointerDown={handle3dPointerDown}
        onPointerMove={handle3dPointerMove}
        onPointerUp={stop3dPointer}
        onPointerLeave={stop3dPointer}
        onWheel={handle3dWheel}
        onKeyDown={handle3dKeyDown}
        tabIndex={0}
      >
        <defs>
          <clipPath id="viewportClipTruss3d">
            <rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" />
          </clipPath>
        </defs>
        <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
        <text x="48" y="58" className="svg-title">
          {truss3dTitle}
        </text>
        <text x={immersiveViewport ? 660 : 790} y="58" className="legend-label">
          {workspaceBadge}
        </text>
        <text x={immersiveViewport ? 560 : 640} y="58" className="legend-label">
          {projectionMode === "ortho" ? "ORTHO" : "PERSP"}
        </text>
        <g clipPath="url(#viewportClipTruss3d)">
          {showGrid
            ? Array.from({ length: Math.floor((gridExtent * 2) / gridStep) + 1 }, (_, index) => -gridExtent + index * gridStep).flatMap((value) => {
                const xLineStart = projectTruss3dPoint({ x: -gridExtent, y: value, z: 0 }, projected3d, camera, projectionMode);
                const xLineEnd = projectTruss3dPoint({ x: gridExtent, y: value, z: 0 }, projected3d, camera, projectionMode);
                const yLineStart = projectTruss3dPoint({ x: value, y: -gridExtent, z: 0 }, projected3d, camera, projectionMode);
                const yLineEnd = projectTruss3dPoint({ x: value, y: gridExtent, z: 0 }, projected3d, camera, projectionMode);

                return [
                  <line key={`grid-x-${value}`} x1={xLineStart.x} y1={xLineStart.y} x2={xLineEnd.x} y2={xLineEnd.y} className="guide guide--soft" />,
                  <line key={`grid-y-${value}`} x1={yLineStart.x} y1={yLineStart.y} x2={yLineEnd.x} y2={yLineEnd.y} className="guide guide--soft" />,
                ];
              })
            : null}
          <line x1="74" y1="386" x2="130" y2="386" className="guide" />
          <line x1="74" y1="386" x2="74" y2="330" className="guide" />
          <line x1="74" y1="386" x2="104" y2="356" className="guide guide--soft" />
          <text x="136" y="390" className="node-label">X</text>
          <text x="68" y="324" className="node-label">Z</text>
          <text x="108" y="350" className="node-label">Y</text>
          {visibleTruss3dElements.map((element) => {
            if (element.material_id && hiddenTruss3dMaterialIds.includes(element.material_id)) return null;
            const start = projectTruss3dPoint(displayTruss3dNodes[element.node_i], projected3d, camera, projectionMode);
            const end = projectTruss3dPoint(displayTruss3dNodes[element.node_j], projected3d, camera, projectionMode);
            if (!lineInsideViewport(start, end)) return null;
            return (
              <line
                key={`space-${element.id}`}
                x1={start.x}
                y1={start.y}
                x2={end.x}
                y2={end.y}
                className={`bar bar--base${selectedTruss3dElement === element.index ? " bar--selected" : ""}`}
                style={{ stroke: truss3dElementColors[element.index] }}
                onPointerDown={(event) => {
                  event.stopPropagation();
                  onSelectTruss3dElement(element.index);
                }}
              />
            );
          })}
          {draftStartNode && hoveredTruss3dNode !== null && hoveredTruss3dNode !== draftStartNodeIndex ? (() => {
            const start = projectTruss3dPoint(draftStartNode, projected3d, camera, projectionMode);
            const endNode = displayTruss3dNodes[hoveredTruss3dNode];
            if (!endNode) return null;
            const end = projectTruss3dPoint(endNode, projected3d, camera, projectionMode);
            if (!lineInsideViewport(start, end, 42)) return null;
            return <line x1={start.x} y1={start.y} x2={end.x} y2={end.y} className="bar bar--preview" />;
          })() : null}
          {visibleTruss3dNodes.map((node, index) => {
            const point = projectTruss3dPoint(node, projected3d, camera, projectionMode);
            if (!pointInsideViewport(point, 24)) return null;
            const showLabel = index % truss3dLabelStep === 0;
            const isSelected = selectedTruss3dNodeIndices.includes(index) || selectedTruss3dNode === index;
            return (
              <g key={node.id}>
                {showNodes ? (
                  <circle
                    cx={point.x}
                    cy={point.y}
                    r={isSelected ? 9 : 7}
                    className={`node-base${isSelected ? " node-base--active" : ""}${memberDraftNodes.includes(index) ? " node-base--draft" : ""}${truss3dLinkMode && hoveredTruss3dNode === index ? " node-base--warning" : ""}`}
                    onPointerDown={(event) => {
                      if (boxSelectMode) {
                        return;
                      }
                      event.stopPropagation();
                      onSelectTruss3dNode(index);
                      if (truss3dLinkMode) {
                        return;
                      }
                      if (isModelMode && event.button === 0) {
                        dragNode3dRef.current = index;
                        pointerRef.current = { x: event.clientX, y: event.clientY };
                      }
                    }}
                    onPointerEnter={() => {
                      if (truss3dLinkMode) setHoveredTruss3dNode(index);
                    }}
                    onPointerLeave={() => {
                      if (truss3dLinkMode) setHoveredTruss3dNode((current) => (current === index ? null : current));
                    }}
                  />
                ) : null}
                {showLabels && (showLabel || selectedTruss3dNode === index) ? (
                  <text x={point.x + 10} y={point.y - 10} className="node-label">
                    {node.id}
                  </text>
                ) : null}
              </g>
            );
          })}
          {isModelMode && selected3dNodeData && !truss3dLinkMode
            ? (["x", "y", "z"] as const).map((axis) => {
                const origin = projectTruss3dPoint(selected3dNodeData, projected3d, camera, projectionMode);
                const target = projectTruss3dPoint(
                  {
                    ...selected3dNodeData,
                    [axis]: selected3dNodeData[axis] + 0.8,
                  },
                  projected3d,
                  camera,
                  projectionMode,
                );
                const classes =
                  axis === "x"
                    ? "gizmo-line gizmo-line--x"
                    : axis === "y"
                      ? "gizmo-line gizmo-line--y"
                      : "gizmo-line gizmo-line--z";

                return (
                  <g key={axis}>
                    <line
                      x1={origin.x}
                      y1={origin.y}
                      x2={target.x}
                      y2={target.y}
                      className={classes}
                      onPointerDown={(event) => {
                        event.stopPropagation();
                        dragAxisRef.current = axis;
                        pointerRef.current = { x: event.clientX, y: event.clientY };
                      }}
                    />
                    <circle
                      cx={target.x}
                      cy={target.y}
                      r={6}
                      className={`gizmo-handle ${classes}`}
                      onPointerDown={(event) => {
                        event.stopPropagation();
                        dragAxisRef.current = axis;
                        pointerRef.current = { x: event.clientX, y: event.clientY };
                      }}
                    />
                  </g>
                );
              })
            : null}
          {selectionRect ? (
            <rect
              x={selectionRect.x}
              y={selectionRect.y}
              width={selectionRect.width}
              height={selectionRect.height}
              className="selection-box"
            />
          ) : null}
        </g>
      </svg>
    );
  }

  return (
    <svg viewBox="0 0 980 460" className="viewport-svg" style={svgStyle} aria-label="2d plane triangle response">
      <defs>
        <clipPath id="viewportClipPlane">
          <rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" />
        </clipPath>
      </defs>
      <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
      <text x="48" y="58" className="svg-title">
        {planeTitle}
      </text>
      <g clipPath="url(#viewportClipPlane)">
        {visiblePlaneElements.map((element) => {
          if (element.material_id && hiddenPlaneMaterialIds.includes(element.material_id)) return null;
          const points = [
            toSvgPoint(planeNodes[element.node_i], planeBounds),
            toSvgPoint(planeNodes[element.node_j], planeBounds),
            toSvgPoint(planeNodes[element.node_k], planeBounds),
          ];
          if (!polygonInsideViewport(points)) return null;
          return (
            <polygon
              key={`plane-${element.id}`}
              points={points.map((point) => `${point.x},${point.y}`).join(" ")}
              className={`plane-triangle${selectedElement === element.index ? " plane-triangle--active" : ""}`}
              style={{ fill: planeResult ? planeStressFill(element.von_mises ?? 0, planeMaxVonMises) : planeElementColors[element.index] }}
              onPointerDown={() => {
                if (isModelMode) onSelectPlaneElement(element.index);
              }}
            />
          );
        })}
        {planeResult
          ? visiblePlaneElements.flatMap((element, index) => {
              if (element.material_id && hiddenPlaneMaterialIds.includes(element.material_id)) return [];
              if (index % planeDeformedStep !== 0) return [];
              const points = [
                toSvgPoint(
                  { x: planeNodes[element.node_i].x + planeNodes[element.node_i].ux * 5000, y: planeNodes[element.node_i].y + planeNodes[element.node_i].uy * 5000 },
                  planeBounds,
                ),
                toSvgPoint(
                  { x: planeNodes[element.node_j].x + planeNodes[element.node_j].ux * 5000, y: planeNodes[element.node_j].y + planeNodes[element.node_j].uy * 5000 },
                  planeBounds,
                ),
                toSvgPoint(
                  { x: planeNodes[element.node_k].x + planeNodes[element.node_k].ux * 5000, y: planeNodes[element.node_k].y + planeNodes[element.node_k].uy * 5000 },
                  planeBounds,
                ),
              ];
              if (!polygonInsideViewport(points, 70)) return [];
              return (
                <polygon
                  key={`plane-def-${element.id}`}
                  points={points.map((point) => `${point.x},${point.y}`).join(" ")}
                  className="plane-triangle plane-triangle--deformed"
                />
              );
            })
          : null}
        {visiblePlaneNodes.flatMap((node, index) => {
          const point = toSvgPoint(node, planeBounds);
          if (!pointInsideViewport(point, 18)) return [];
          const showLabel = index % planeNodeLabelStep === 0 || selectedPlaneNodeId === node.id;
          const showMarker = index % planeNodeMarkerStep === 0 || showLabel;
          if (!showMarker) return [];
          return (
            <g key={node.id}>
              {renderSupportGlyph(point, node, `plane-support-${node.id}`)}
              {renderLoadGlyph(point, node, `plane-load-${node.id}`)}
              <circle
                cx={point.x}
                cy={point.y}
                r={selectedPlaneNodeId === node.id ? 8 : 6}
                className={`node-base${selectedPlaneNodeId === node.id ? " node-base--active" : ""}`}
                onPointerDown={() => {
                  if (isModelMode) onSelectPlaneNode(node.index);
                }}
              />
              {showLabel ? (
                <text x={point.x + 10} y={point.y - 10} className="node-label">
                  {node.id}
                </text>
              ) : null}
            </g>
          );
        })}
      </g>
    </svg>
  );
}

export const WorkbenchViewport = memo(WorkbenchViewportInner);
