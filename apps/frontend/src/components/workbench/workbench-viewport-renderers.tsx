"use client";

import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent, KeyboardEvent as ReactKeyboardEvent } from "react";
import {
  lineInsideViewport,
  planeStressFill,
  pointInsideViewport,
  projectTruss3dPoint,
  toSvgPoint,
  type CameraState,
  type DisplayTruss3dElement,
  type DisplayTruss3dNode,
  type DisplayTrussElement,
  type DisplayTrussNode,
  type LineResultField,
  type ProjectionMode,
  VIEWPORT_CLIP,
  type Bounds,
} from "@/components/workbench/workbench-viewport-core";

function renderSupportGlyph(
  point: { x: number; y: number },
  node: { fix_x?: boolean; fix_y?: boolean },
  key: string,
) {
  if (!node.fix_x && !node.fix_y) return null;
  return <rect key={key} x={point.x - 14} y={point.y + 14} width="28" height="9" className="support-glyph" />;
}

function renderLoadGlyph(
  point: { x: number; y: number },
  node: { load_x?: number; load_y?: number },
  key: string,
) {
  if (!node.load_x && !node.load_y) return null;
  const dx = node.load_x ? Math.sign(node.load_x) * 24 : 0;
  const dy = node.load_y ? -Math.sign(node.load_y) * 24 : 0;
  return (
    <line
      key={key}
      x1={point.x}
      y1={point.y}
      x2={point.x + dx}
      y2={point.y + dy}
      className="load-glyph"
      markerEnd="url(#arrow-head)"
    />
  );
}

export function renderAxialViewport({
  axialLength,
  axialNodes,
  axialScale,
  axialTitle,
}: {
  axialLength: number;
  axialNodes: Array<{ x: number; displacement: number }>;
  axialScale: number;
  axialTitle: string;
}) {
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
      <text x="48" y="58" className="svg-title">{axialTitle}</text>
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

export function renderLineViewport({
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
  trussElementColors,
  trussHotspotNodes,
  trussLabelStep,
  trussLegend,
  trussMarkerStep,
  trussNodeIssues,
  trussResult,
  trussTitle,
  visibleTrussElements,
  visibleTrussNodes,
  trussDeformedStep,
  title,
  svgStyle,
}: {
  displayTrussElements: DisplayTrussElement[];
  displayTrussNodes: DisplayTrussNode[];
  focusedFrameElement: number | null;
  frameResultField: LineResultField;
  frameResultFieldMax: number;
  hiddenTrussMaterialIds: string[];
  isModelMode: boolean;
  memberDraftNodes: number[];
  onSelectTrussElement: (index: number) => void;
  onStartTrussNodeDrag: (index: number) => void;
  onStopDraggingNode: () => void;
  onTrussPointerMove: (event: ReactPointerEvent<SVGSVGElement>) => void;
  selectedElement: number | null;
  selectedNode: number | null;
  studyKind: string;
  trussBounds: Bounds;
  trussElementColors: string[];
  trussHotspotNodes: number[];
  trussLabelStep: number;
  trussLegend?: string;
  trussMarkerStep: number;
  trussNodeIssues: Record<number, string[]>;
  trussResult: boolean;
  trussTitle: string;
  visibleTrussElements: DisplayTrussElement[];
  visibleTrussNodes: DisplayTrussNode[];
  trussDeformedStep: number;
  title: string;
  svgStyle?: { width: string; minWidth: string };
}) {
  return (
    <svg
      viewBox="0 0 980 460"
      className="viewport-svg"
      style={svgStyle}
      aria-label="2d line response"
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
      <text x="48" y="58" className="svg-title">{isModelMode ? title : trussTitle}</text>
      {trussResult && trussLegend ? <text x="760" y="58" className="svg-copy svg-copy--muted">{trussLegend}</text> : null}
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
              className={`bar bar--base${selectedElement === element.index ? " bar--selected" : ""}${focusedFrameElement === element.index ? " bar--focused" : ""}`}
              style={{
                stroke:
                  (studyKind === "thermal_frame_2d" || studyKind === "frame_2d" || studyKind === "beam_1d" || studyKind === "thermal_beam_1d" || studyKind === "torsion_1d" || studyKind === "spring_1d" || studyKind === "spring_2d") && trussResult
                    ? planeStressFill(
                        frameResultField === "axial_stress"
                          ? Math.abs(element.axial_stress ?? 0)
                          : frameResultField === "shear_force"
                            ? Math.max(Math.abs(element.shear_force_i ?? 0), Math.abs(element.shear_force_j ?? 0))
                            : frameResultField === "max_bending_stress"
                              ? Math.abs(element.max_bending_stress ?? 0)
                              : frameResultField === "moment"
                                ? Math.max(Math.abs(element.moment_i ?? 0), Math.abs(element.moment_j ?? 0))
                                : frameResultField === "average_temperature_delta"
                                  ? Math.abs(element.average_temperature_delta ?? 0)
                                  : frameResultField === "temperature_gradient_y"
                                    ? Math.abs(element.temperature_gradient_y ?? 0)
                                    : frameResultField === "thermal_curvature"
                                      ? Math.abs(element.thermal_curvature ?? 0)
                                        : Math.abs(element.max_combined_stress ?? 0),
                        frameResultFieldMax,
                      )
                    : trussElementColors[element.index],
              }}
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
              const start = toSvgPoint({ x: displayTrussNodes[element.node_i].x + displayTrussNodes[element.node_i].ux * 10000, y: displayTrussNodes[element.node_i].y + displayTrussNodes[element.node_i].uy * 10000 }, trussBounds);
              const end = toSvgPoint({ x: displayTrussNodes[element.node_j].x + displayTrussNodes[element.node_j].ux * 10000, y: displayTrussNodes[element.node_j].y + displayTrussNodes[element.node_j].uy * 10000 }, trussBounds);
              if (!lineInsideViewport(start, end, 80)) return [];
              return <line key={`def-${element.id}`} x1={start.x} y1={start.y} x2={end.x} y2={end.y} className="bar bar--deformed-truss" />;
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
              {trussHotspotNodes.includes(index) ? <circle cx={point.x} cy={point.y} r={isModelMode ? 16 : 12} className="node-hotspot-ring" /> : null}
              <circle
                cx={point.x}
                cy={point.y}
                r={isModelMode ? 10 : 7}
                className={`node-base${selectedNode === index ? " node-base--active" : ""}${memberDraftNodes.includes(index) ? " node-base--draft" : ""}${(trussNodeIssues[index] ?? []).length > 0 ? " node-base--warning" : ""}`}
                onPointerDown={() => {
                  if (isModelMode) onStartTrussNodeDrag(index);
                }}
              />
              {showLabel ? <text x={point.x + 12} y={point.y - 10} className="node-label">{node.id}</text> : null}
              {trussResult && pointInsideViewport(deformed, 80) ? <circle cx={deformed.x} cy={deformed.y} r={5} className="node-deformed" /> : null}
            </g>
          );
        })}
      </g>
    </svg>
  );
}

export function renderTruss3dViewport({
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
  startAxisDrag,
  startNodeDrag,
}: {
  activeViewPreset: string;
  boxSelectMode: boolean;
  camera: CameraState;
  displayTruss3dElements: DisplayTruss3dElement[];
  displayTruss3dNodes: DisplayTruss3dNode[];
  draftStartNode: DisplayTruss3dNode | null;
  draftStartNodeIndex: number | null;
  gridExtent: number;
  gridStep: number;
  handle3dKeyDown: (event: ReactKeyboardEvent<SVGSVGElement>) => void;
  handle3dPointerDown: (event: ReactPointerEvent<SVGSVGElement>) => void;
  handle3dPointerMove: (event: ReactPointerEvent<SVGSVGElement>) => void;
  handle3dWheel: (event: ReactWheelEvent<SVGSVGElement>) => void;
  hiddenTruss3dMaterialIds: string[];
  hoveredTruss3dNode: number | null;
  immersiveViewport: boolean;
  isModelMode: boolean;
  memberDraftNodes: number[];
  onSelectTruss3dElement: (index: number) => void;
  onSelectTruss3dNode: (index: number) => void;
  onUpdateTruss3dNodePosition: (index: number, position: { x: number; y: number; z: number }) => void;
  projected3d: { minX: number; maxX: number; minZ: number; maxZ: number; width: number; height: number };
  projectionMode: ProjectionMode;
  selected3dNodeData: DisplayTruss3dNode | null;
  selectedTruss3dElement: number | null;
  selectedTruss3dNode: number | null;
  selectedTruss3dNodeIndices: number[];
  selectionRect: { x: number; y: number; width: number; height: number } | null;
  setHoveredTruss3dNode: (value: ((current: number | null) => number | null) | number | null) => void;
  showGrid: boolean;
  showLabels: boolean;
  showNodes: boolean;
  stop3dPointer: (event?: ReactPointerEvent<SVGSVGElement>) => void;
  svgStyle?: { width: string; minWidth: string };
  truss3dElementColors: string[];
  truss3dLabelStep: number;
  truss3dLinkMode: boolean;
  truss3dTitle: string;
  visibleTruss3dElements: DisplayTruss3dElement[];
  visibleTruss3dNodes: DisplayTruss3dNode[];
  workspaceBadge: string;
  startAxisDrag: (axis: "x" | "y" | "z", event: { clientX: number; clientY: number }) => void;
  startNodeDrag: (index: number, event: { clientX: number; clientY: number }) => void;
}) {
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
      <defs><clipPath id="viewportClipTruss3d"><rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" /></clipPath></defs>
      <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
      <text x="48" y="58" className="svg-title">{truss3dTitle}</text>
      <text x={immersiveViewport ? 660 : 790} y="58" className="legend-label">{workspaceBadge}</text>
      <text x={immersiveViewport ? 560 : 640} y="58" className="legend-label">{projectionMode === "ortho" ? "ORTHO" : "PERSP"}</text>
      <g clipPath="url(#viewportClipTruss3d)">
        {showGrid ? Array.from({ length: Math.floor((gridExtent * 2) / gridStep) + 1 }, (_, index) => -gridExtent + index * gridStep).flatMap((value) => {
          const xLineStart = projectTruss3dPoint({ x: -gridExtent, y: value, z: 0 }, projected3d, camera, projectionMode);
          const xLineEnd = projectTruss3dPoint({ x: gridExtent, y: value, z: 0 }, projected3d, camera, projectionMode);
          const yLineStart = projectTruss3dPoint({ x: value, y: -gridExtent, z: 0 }, projected3d, camera, projectionMode);
          const yLineEnd = projectTruss3dPoint({ x: value, y: gridExtent, z: 0 }, projected3d, camera, projectionMode);
          return [
            <line key={`grid-x-${value}`} x1={xLineStart.x} y1={xLineStart.y} x2={xLineEnd.x} y2={xLineEnd.y} className="guide guide--soft" />,
            <line key={`grid-y-${value}`} x1={yLineStart.x} y1={yLineStart.y} x2={yLineEnd.x} y2={yLineEnd.y} className="guide guide--soft" />,
          ];
        }) : null}
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
          return <line key={`space-${element.id}`} x1={start.x} y1={start.y} x2={end.x} y2={end.y} className={`bar bar--base${selectedTruss3dElement === element.index ? " bar--selected" : ""}`} style={{ stroke: truss3dElementColors[element.index] }} onPointerDown={(event) => { event.stopPropagation(); onSelectTruss3dElement(element.index); }} />;
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
              {showNodes ? <circle cx={point.x} cy={point.y} r={isSelected ? 9 : 7} className={`node-base${isSelected ? " node-base--active" : ""}${memberDraftNodes.includes(index) ? " node-base--draft" : ""}${truss3dLinkMode && hoveredTruss3dNode === index ? " node-base--warning" : ""}`} onPointerDown={(event) => { if (boxSelectMode) return; event.stopPropagation(); onSelectTruss3dNode(index); if (truss3dLinkMode) return; if (isModelMode && event.button === 0) startNodeDrag(index, event); }} onPointerEnter={() => { if (truss3dLinkMode) setHoveredTruss3dNode(index); }} onPointerLeave={() => { if (truss3dLinkMode) setHoveredTruss3dNode((current) => (current === index ? null : current)); }} /> : null}
              {showLabels && (showLabel || selectedTruss3dNode === index) ? <text x={point.x + 10} y={point.y - 10} className="node-label">{node.id}</text> : null}
            </g>
          );
        })}
        {isModelMode && selected3dNodeData && !truss3dLinkMode ? (["x", "y", "z"] as const).map((axis) => {
          const origin = projectTruss3dPoint(selected3dNodeData, projected3d, camera, projectionMode);
          const target = projectTruss3dPoint({ ...selected3dNodeData, [axis]: selected3dNodeData[axis] + 0.8 }, projected3d, camera, projectionMode);
          const classes = axis === "x" ? "gizmo-line gizmo-line--x" : axis === "y" ? "gizmo-line gizmo-line--y" : "gizmo-line gizmo-line--z";
          return (
            <g key={axis}>
              <line x1={origin.x} y1={origin.y} x2={target.x} y2={target.y} className={classes} onPointerDown={(event) => { event.stopPropagation(); startAxisDrag(axis, event); }} />
              <circle cx={target.x} cy={target.y} r={6} className={`gizmo-handle ${classes}`} onPointerDown={(event) => { event.stopPropagation(); startAxisDrag(axis, event); }} />
            </g>
          );
        }) : null}
        {selectionRect ? <rect x={selectionRect.x} y={selectionRect.y} width={selectionRect.width} height={selectionRect.height} className="selection-box" /> : null}
      </g>
    </svg>
  );
}
