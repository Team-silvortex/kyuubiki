"use client";

import { memo, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import {
  buildProjectedBounds,
  cameraForPreset,
  initialRenderBudget,
  pointerToViewport,
  projectTruss3dPoint,
  renderBatchSize,
  rotatedDeltaToWorld,
  stepForDensity,
  type CameraState,
  type ProjectionMode,
  type ViewPreset,
  type WorkbenchViewportProps,
} from "@/components/workbench/workbench-viewport-core";
import {
  renderAxialViewport,
  renderLineViewport,
  renderPlaneViewport,
  renderTruss3dViewport,
} from "@/components/workbench/workbench-viewport-renderers";

function WorkbenchViewportInner(props: WorkbenchViewportProps) {
  const {
    activeViewPreset,
    axialLength,
    axialNodes,
    axialScale,
    axialTitle,
    boxSelectMode,
    displayTruss3dElements,
    displayTruss3dNodes,
    displayTrussElements,
    displayTrussNodes,
    focusRequestVersion,
    focusedFrameElement,
    focusedPlaneElement,
    frameResultField,
    frameResultFieldMax,
    hiddenPlaneMaterialIds,
    hiddenTruss3dMaterialIds,
    hiddenTrussMaterialIds,
    immersiveViewport,
    memberDraftNodes,
    onBeginTruss3dNodeDrag,
    onBoxSelectModeChange,
    onEndTruss3dNodeDrag,
    onProjectionModeChange,
    onSelectPlaneElement,
    onSelectPlaneNode,
    onSelectTruss3dElement,
    onSelectTruss3dNode,
    onSelectTruss3dNodes,
    onSelectTrussElement,
    onShowGridChange,
    onShowLabelsChange,
    onShowNodesChange,
    onStartTrussNodeDrag,
    onStopDraggingNode,
    onTrussPointerMove,
    onUpdateTruss3dNodePosition,
    planeBounds,
    planeElementColors,
    planeElements,
    planeLegend,
    planeNodes,
    planeResult,
    planeResultField,
    planeResultFieldMax,
    planeTitle,
    projectionMode,
    resetRequestVersion,
    selectedElement,
    selectedNode,
    selectedPlaneNodeId,
    selectedTruss3dElement,
    selectedTruss3dNode,
    selectedTruss3dNodeIndices,
    shortcutLegendRows,
    shortcutLegendTitle,
    showGrid,
    showLabels,
    showNodes,
    showShortcutHints,
    sidebarSection,
    studyKind,
    title,
    truss3dElementColors,
    truss3dLinkMode,
    truss3dTitle,
    trussBounds,
    trussElementColors,
    trussHotspotNodes,
    trussLegend,
    trussNodeIssues,
    trussResult,
    trussTitle,
    viewportPixelWidth,
    workspaceBadge,
  } = props;

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
  const draftStartNode = truss3dLinkMode && draftStartNodeIndex !== null ? displayTruss3dNodes[draftStartNodeIndex] ?? null : null;
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
      const factor = projectionMode === "persp" ? Math.max(0.64, Math.min(1.42, 8 / (8 + projectTruss3dPoint(target, projected3d, camera, projectionMode).y))) : 1;
      const deltaRotatedX = (dx / Math.max(camera.zoom * factor, 0.01)) * (projected3d.width / usableWidth);
      const deltaRotatedZ = (-dy / Math.max(camera.zoom * factor, 0.01)) * (projected3d.height / usableHeight);
      const deltaWorld = rotatedDeltaToWorld(deltaRotatedX, deltaRotatedZ, camera);
      onUpdateTruss3dNodePosition(targetIndex, { x: target.x + deltaWorld.x, y: target.y + deltaWorld.y, z: target.z + deltaWorld.z });
      return;
    }

    if (dragAxisRef.current !== null && selectedTruss3dNode !== null) {
      const target = displayTruss3dNodes[selectedTruss3dNode];
      if (!target) return;
      onBeginTruss3dNodeDrag();
      const axis = dragAxisRef.current;
      const origin = projectTruss3dPoint(target, projected3d, camera, projectionMode);
      const axisTarget = projectTruss3dPoint({ ...target, [axis]: target[axis] + 1 }, projected3d, camera, projectionMode);
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
        : { ...current, yaw: current.yaw + dx * 0.008, pitch: Math.max(-1.35, Math.min(1.35, current.pitch - dy * 0.008)) },
    );
  };

  const stop3dPointer = (event?: ReactPointerEvent<SVGSVGElement>) => {
    dragModeRef.current = null;
    if (dragNode3dRef.current !== null || dragAxisRef.current !== null) {
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
      setCamera((current) => ({ ...current, panX: current.panX - event.deltaY * 0.45 }));
      return;
    }
    const direction = event.deltaY > 0 ? 0.92 : 1.08;
    setCamera((current) => ({ ...current, zoom: Math.max(0.55, Math.min(2.8, current.zoom * direction)) }));
  };

  const focusSelected3dNode = () => {
    const selectedNodes =
      selectedTruss3dNodeIndices.length > 0
        ? selectedTruss3dNodeIndices.map((index) => displayTruss3dNodes[index]).filter(Boolean)
        : selected3dNodeData
          ? [selected3dNodeData]
          : displayTruss3dNodes;
    if (selectedNodes.length === 0) return;

    const center = selectedNodes.reduce((acc, node) => ({ x: acc.x + node.x, y: acc.y + node.y, z: acc.z + node.z }), { x: 0, y: 0, z: 0 });
    const target = { x: center.x / selectedNodes.length, y: center.y / selectedNodes.length, z: center.z / selectedNodes.length };

    setCamera((current) => {
      const bounds = buildProjectedBounds(displayTruss3dNodes, current);
      const point = projectTruss3dPoint(target, bounds, { ...current, panX: 0, panY: 0 }, projectionMode);
      const nextPanX = current.panX + (490 - point.x);
      const nextPanY = current.panY + (230 - point.y);
      return { ...current, panX: Number.isFinite(nextPanX) ? nextPanX : current.panX, panY: Number.isFinite(nextPanY) ? nextPanY : current.panY };
    });
  };

  const resetCamera = () => {
    setCamera((current) => ({ ...cameraForPreset("iso"), zoom: current.zoom }));
  };

  useEffect(() => {
    if (focusRequestVersion > 0) focusSelected3dNode();
  }, [focusRequestVersion]);

  useEffect(() => {
    if (resetRequestVersion > 0) resetCamera();
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

  const gridExtent = Math.max(...displayTruss3dNodes.map((node) => Math.max(Math.abs(node.x), Math.abs(node.y))), 2);
  const gridStep = gridExtent > 6 ? 2 : 1;

  if (studyKind === "axial_bar_1d") {
    return renderAxialViewport({ axialLength, axialNodes, axialScale, axialTitle });
  }

  if (studyKind === "thermal_bar_1d" || studyKind === "thermal_beam_1d" || studyKind === "thermal_frame_2d" || studyKind === "thermal_truss_2d" || studyKind === "truss_2d" || studyKind === "frame_2d" || studyKind === "beam_1d" || studyKind === "torsion_1d" || studyKind === "spring_1d" || studyKind === "spring_2d") {
    return renderLineViewport({
      displayTrussElements,
      displayTrussNodes,
      focusedFrameElement,
      frameResultField,
      frameResultFieldMax,
      hiddenTrussMaterialIds,
      isModelMode,
      memberDraftNodes,
      onSelectTrussElement,
      onStartTrussNodeDrag,
      onStopDraggingNode,
      onTrussPointerMove,
      selectedElement,
      selectedNode,
      studyKind,
      trussBounds,
      trussDeformedStep,
      trussElementColors,
      trussHotspotNodes,
      trussLabelStep,
      trussLegend,
      trussMarkerStep,
      trussNodeIssues,
      trussResult,
      trussTitle,
      title,
      visibleTrussElements,
      visibleTrussNodes,
      svgStyle,
    });
  }

  if (studyKind === "truss_3d" || studyKind === "thermal_truss_3d" || studyKind === "spring_3d") {
    return renderTruss3dViewport({
      activeViewPreset,
      boxSelectMode,
      camera,
      displayTruss3dElements,
      displayTruss3dNodes,
      draftStartNode,
      draftStartNodeIndex,
      gridExtent,
      gridStep,
      handle3dKeyDown,
      handle3dPointerDown,
      handle3dPointerMove,
      handle3dWheel,
      hiddenTruss3dMaterialIds,
      hoveredTruss3dNode,
      immersiveViewport,
      isModelMode,
      memberDraftNodes,
      onSelectTruss3dElement,
      onSelectTruss3dNode,
      onUpdateTruss3dNodePosition,
      projected3d,
      projectionMode,
      selected3dNodeData,
      selectedTruss3dElement,
      selectedTruss3dNode,
      selectedTruss3dNodeIndices,
      selectionRect,
      setHoveredTruss3dNode,
      showGrid,
      showLabels,
      showNodes,
      stop3dPointer,
      svgStyle,
      truss3dElementColors,
      truss3dLabelStep,
      truss3dLinkMode,
      truss3dTitle,
      visibleTruss3dElements,
      visibleTruss3dNodes,
      workspaceBadge,
      startAxisDrag: (axis, event) => {
        dragAxisRef.current = axis;
        pointerRef.current = { x: event.clientX, y: event.clientY };
      },
      startNodeDrag: (index, event) => {
        dragNode3dRef.current = index;
        pointerRef.current = { x: event.clientX, y: event.clientY };
      },
    });
  }

  return renderPlaneViewport({
    focusedPlaneElement,
    hiddenPlaneMaterialIds,
    isModelMode,
    onSelectPlaneElement,
    onSelectPlaneNode,
    planeBounds,
    planeDeformedStep,
    planeElementColors,
    planeElements,
    planeLegend,
    planeNodeLabelStep,
    planeNodeMarkerStep,
    planeNodes,
    planeResult,
    planeResultField,
    planeResultFieldMax,
    planeTitle,
    selectedElement,
    selectedPlaneNodeId,
    svgStyle,
    visiblePlaneElements,
    visiblePlaneNodes,
  });
}

export const WorkbenchViewport = memo(WorkbenchViewportInner);
