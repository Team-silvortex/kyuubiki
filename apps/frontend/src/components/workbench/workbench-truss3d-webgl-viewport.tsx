"use client";

import { useMemo, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import { lineInsideViewport, pointInsideViewport, projectTruss3dPoint, type CameraState, type DisplayTruss3dElement, type DisplayTruss3dNode, type ProjectionMode, type ViewPreset, VIEWPORT_CLIP } from "@/components/workbench/workbench-viewport-core";
import { buildTruss3dSceneBuffers, type Truss3dSceneBounds, type DeformationViewMode } from "@/components/workbench/workbench-truss3d-webgl-scene";
import type { queryTruss3dLod, Truss3dLodView } from "./workbench-truss3d-lod";
import { buildElementReadout, buildNodeReadout, buildSelectionSummary, type Truss3dReadout, type Truss3dReadoutStudyKind } from "@/components/workbench/workbench-truss3d-readout";
import { useTruss3dCanvas } from "@/components/workbench/use-truss3d-canvas";

type Props = {
  activeViewPreset: ViewPreset;
  boxSelectMode: boolean;
  camera: CameraState;
  displayTruss3dNodes: DisplayTruss3dNode[];
  displayTruss3dElements: DisplayTruss3dElement[];
  sceneBounds: Truss3dSceneBounds;
  deformationScale: number;
  lod: ReturnType<typeof queryTruss3dLod>;
  nodeByIndex: (index: number) => DisplayTruss3dNode | undefined;
  elementByIndex: (index: number) => DisplayTruss3dElement | undefined;
  elementOffsetByIndex: (index: number) => number;
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
  projected3d: Truss3dLodView["projected3d"];
  projectionMode: ProjectionMode;
  selected3dNodeData: DisplayTruss3dNode | null;
  selectedTruss3dElement: number | null;
  selectedTruss3dNode: number | null;
  selectedTruss3dNodeIndices: number[];
  studyKind: Truss3dReadoutStudyKind;
  selectionRect: { x: number; y: number; width: number; height: number } | null;
  setHoveredTruss3dNode: (value: ((current: number | null) => number | null) | number | null) => void;
  showGrid: boolean;
  showLabels: boolean;
  showNodes: boolean;
  stop3dPointer: (event?: ReactPointerEvent<SVGSVGElement>) => void;
  svgStyle?: { width: string; minWidth: string };
  truss3dElementColors: string[];
  truss3dLegend?: string;
  truss3dLabelStep: number;
  truss3dLinkMode: boolean;
  truss3dTitle: string;
  visibleTruss3dElements: DisplayTruss3dElement[];
  visibleTruss3dNodes: DisplayTruss3dNode[];
  workspaceBadge: string;
  startAxisDrag: (axis: "x" | "y" | "z", event: { clientX: number; clientY: number }) => void;
  startNodeDrag: (index: number, event: { clientX: number; clientY: number }) => void;
};

const VIEWBOX_WIDTH = 980;
const VIEWBOX_HEIGHT = 460;

export function WorkbenchTruss3dWebglViewport(props: Props) {
  const deformationScale = props.deformationScale;
  const hasDeformation = useMemo(() => !props.isModelMode && props.displayTruss3dNodes.some((n) => Math.hypot(n.ux, n.uy, n.uz) > 1e-9),
    [props.isModelMode, props.displayTruss3dNodes]);
  const [hoverReadout, setHoverReadout] = useState<Truss3dReadout>(null);
  const [deformationViewMode, setDeformationViewMode] = useState<DeformationViewMode>("overlay");
  const selectedNodes = useMemo(() => new Set(props.selectedTruss3dNodeIndices), [props.selectedTruss3dNodeIndices]);
  const scene = useMemo(() => buildTruss3dSceneBuffers({ ...props, deformationViewMode, selectedNodeSet: selectedNodes, hasDeformation,
    elementColorByIndex: (index) => props.truss3dElementColors[props.elementOffsetByIndex(index)],
  }), [
    props.displayTruss3dNodes, props.visibleTruss3dNodes, props.visibleTruss3dElements,
    props.gridExtent, props.gridStep, props.hiddenTruss3dMaterialIds, props.isModelMode,
    props.memberDraftNodes, props.selectedTruss3dElement, props.selectedTruss3dNode,
    props.selectedTruss3dNodeIndices, props.showGrid, props.showNodes,
    props.truss3dElementColors, props.truss3dLinkMode, deformationViewMode,
    props.sceneBounds, props.deformationScale,
    selectedNodes, hasDeformation,
    props.nodeByIndex, props.elementOffsetByIndex,
  ]);
  const canvasRef = useTruss3dCanvas(scene, props);
  const draftNodes = useMemo(() => new Set(props.memberDraftNodes), [props.memberDraftNodes]);
  const selectedElementData = props.selectedTruss3dElement === null ? null : props.elementByIndex(props.selectedTruss3dElement);
  const selectedNodeGroup = useMemo(
    () => props.selectedTruss3dNodeIndices.map(props.nodeByIndex).filter((node): node is DisplayTruss3dNode => Boolean(node)),
    [props.nodeByIndex, props.selectedTruss3dNodeIndices],
  );
  const sceneBounds = props.sceneBounds;
  const spatialSummary = `span ${sceneBounds.spanX.toFixed(2)} x ${sceneBounds.spanY.toFixed(2)} x ${sceneBounds.spanZ.toFixed(2)} · diag ${sceneBounds.diagonal.toFixed(2)}`;
  const persistentReadout = useMemo(() =>
    selectedElementData ? buildElementReadout(props.studyKind, selectedElementData) : selectedNodeGroup.length > 1 ? buildSelectionSummary(props.studyKind, selectedNodeGroup) : props.selected3dNodeData ? buildNodeReadout(props.studyKind, props.selected3dNodeData) : null,
    [selectedElementData, selectedNodeGroup, props.selected3dNodeData, props.studyKind]);
  const activeReadout = persistentReadout ?? hoverReadout;


  const projectedNodes = useMemo(() => props.visibleTruss3dNodes.map((node, index) => ({ index, point: projectTruss3dPoint(node, props.projected3d, props.camera, props.projectionMode) })), [props.visibleTruss3dNodes, props.projected3d, props.camera, props.projectionMode]);

  return (
    <div className="viewport-3d-shell" data-lod-active={props.lod.limited} data-lod-nodes={props.visibleTruss3dNodes.length}
      data-lod-elements={props.visibleTruss3dElements.length} data-lod-visits={props.lod.visits} data-camera-zoom={props.camera.zoom}
      style={{ position: "relative", width: props.svgStyle?.width, minWidth: props.svgStyle?.minWidth, aspectRatio: `${VIEWBOX_WIDTH} / ${VIEWBOX_HEIGHT}` }}>
      <canvas ref={canvasRef} aria-hidden="true" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", display: "block" }} />
      <svg
        viewBox={`0 0 ${VIEWBOX_WIDTH} ${VIEWBOX_HEIGHT}`}
        className="viewport-svg"
        style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
        aria-label="3d truss response"
        onPointerDown={props.handle3dPointerDown}
        onPointerMove={props.handle3dPointerMove}
        onPointerUp={props.stop3dPointer}
        onPointerLeave={props.stop3dPointer}
        onWheel={props.handle3dWheel}
        onKeyDown={props.handle3dKeyDown}
        tabIndex={0}
      >
        <defs>
          <clipPath id="viewportClipTruss3dWebgl">
            <rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" />
          </clipPath>
        </defs>
        <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" style={{ fill: "none" }} />
        <text x="48" y="58" className="svg-title">{props.truss3dTitle}</text>
        {props.truss3dLegend ? <text x="48" y="78" className="svg-copy svg-copy--muted">{props.truss3dLegend}</text> : null}
        <text x="48" y={props.truss3dLegend ? 98 : 78} className="svg-copy svg-copy--muted">{spatialSummary}</text>
        <text x={props.immersiveViewport ? 660 : 790} y="58" className="legend-label">{props.workspaceBadge}</text>
        <text x={props.immersiveViewport ? 560 : 640} y="58" className="legend-label">{props.projectionMode === "ortho" ? "ORTHO" : "PERSP"}</text>
        <text x="920" y="100" textAnchor="end" className="svg-copy svg-copy--muted" data-workbench-lod="true">
          {props.lod.limited ? "LOD · " : ""}{props.camera.zoom.toFixed(2)}x · {props.visibleTruss3dNodes.length}/{props.displayTruss3dNodes.length}
        </text>
        <g clipPath="url(#viewportClipTruss3dWebgl)">
          <line x1="74" y1="386" x2="130" y2="386" className="guide" />
          <line x1="74" y1="386" x2="74" y2="330" className="guide" />
          <line x1="74" y1="386" x2="104" y2="356" className="guide guide--soft" />
          <text x="136" y="390" className="node-label">X</text>
          <text x="68" y="324" className="node-label">Z</text>
          <text x="108" y="350" className="node-label">Y</text>

          {props.visibleTruss3dElements.map((element) => {
            if (element.material_id && props.hiddenTruss3dMaterialIds.includes(element.material_id)) return null;
            const startNode = props.nodeByIndex(element.node_i), endNode = props.nodeByIndex(element.node_j);
            if (!startNode || !endNode) return null;
            const start = projectTruss3dPoint(startNode, props.projected3d, props.camera, props.projectionMode);
            const end = projectTruss3dPoint(endNode, props.projected3d, props.camera, props.projectionMode);
            if (!lineInsideViewport(start, end, 36)) return null;
            return (
              <line
                key={`hit-space-${element.id}`}
                data-truss3d-element={element.index}
                x1={start.x}
                y1={start.y}
                x2={end.x}
                y2={end.y}
                stroke="transparent"
                strokeWidth={18}
                onPointerEnter={() => {
                  setHoverReadout(buildElementReadout(props.studyKind, element));
                }}
                onPointerLeave={() => setHoverReadout((current) => (current?.kind === "element" ? null : current))}
                onPointerDown={(event) => {
                  if (props.boxSelectMode || event.altKey || event.button !== 0) return;
                  event.stopPropagation();
                  props.onSelectTruss3dElement(element.index);
                }}
              />
            );
          })}

          {props.draftStartNode && props.hoveredTruss3dNode !== null && props.hoveredTruss3dNode !== props.draftStartNodeIndex ? (() => {
            const start = projectTruss3dPoint(props.draftStartNode, props.projected3d, props.camera, props.projectionMode);
            const endNode = props.nodeByIndex(props.hoveredTruss3dNode);
            if (!endNode) return null;
            const end = projectTruss3dPoint(endNode, props.projected3d, props.camera, props.projectionMode);
            if (!lineInsideViewport(start, end, 42)) return null;
            return <line x1={start.x} y1={start.y} x2={end.x} y2={end.y} className="bar bar--preview" />;
          })() : null}

          {projectedNodes.map(({ point }, index) => {
            if (!pointInsideViewport(point, 24)) return null;
            const node = props.visibleTruss3dNodes[index];
            const absoluteIndex = node.index;
            const showLabel = index % props.truss3dLabelStep === 0;
            const isSelected =
              selectedNodes.has(absoluteIndex) || props.selectedTruss3dNode === absoluteIndex;
            return (
              <g key={node.id}>
                <circle
                  cx={point.x}
                  data-truss3d-node={absoluteIndex}
                  cy={point.y}
                  r={props.showNodes ? (isSelected ? 9 : 7) : 12}
                  fill={props.showNodes ? "rgba(0,0,0,0)" : "transparent"}
                  className={`${isSelected ? "node-base node-base--active" : ""}${draftNodes.has(absoluteIndex) ? " node-base--draft" : ""}${props.truss3dLinkMode && props.hoveredTruss3dNode === absoluteIndex ? " node-base--warning" : ""}`}
                  style={{ pointerEvents: "auto" }}
                  onPointerDown={(event) => {
                    if (props.boxSelectMode || event.altKey || event.button !== 0) return;
                    event.stopPropagation();
                    props.onSelectTruss3dNode(absoluteIndex);
                    if (props.truss3dLinkMode) return;
                    if (props.isModelMode && event.button === 0) props.startNodeDrag(absoluteIndex, event);
                  }}
                  onPointerEnter={() => {
                    setHoverReadout(buildNodeReadout(props.studyKind, node));
                    if (props.truss3dLinkMode) props.setHoveredTruss3dNode(absoluteIndex);
                  }}
                  onPointerLeave={() => {
                    setHoverReadout((current) => (current?.kind === "node" ? null : current));
                    if (props.truss3dLinkMode) {
                      props.setHoveredTruss3dNode((current) => (current === absoluteIndex ? null : current));
                    }
                  }}
                />
                {props.showLabels && (showLabel || props.selectedTruss3dNode === absoluteIndex) ? (
                  <text x={point.x + 10} y={point.y - 10} className="node-label">{node.id}</text>
                ) : null}
              </g>
            );
          })}

          {props.isModelMode && !props.truss3dLinkMode ? (() => {
            const selectedNode = props.selected3dNodeData;
            if (!selectedNode) return null;
            return (["x", "y", "z"] as const).map((axis) => {
              const origin = projectTruss3dPoint(selectedNode, props.projected3d, props.camera, props.projectionMode);
              const targetNode = {
                x: selectedNode.x + (axis === "x" ? 0.8 : 0),
                y: selectedNode.y + (axis === "y" ? 0.8 : 0),
                z: selectedNode.z + (axis === "z" ? 0.8 : 0),
              };
              const target = projectTruss3dPoint(targetNode, props.projected3d, props.camera, props.projectionMode);
              const classes = axis === "x" ? "gizmo-line gizmo-line--x" : axis === "y" ? "gizmo-line gizmo-line--y" : "gizmo-line gizmo-line--z";
              return (
                <g key={axis}>
                  <line x1={origin.x} y1={origin.y} x2={target.x} y2={target.y} className={classes} onPointerDown={(event) => { event.stopPropagation(); props.startAxisDrag(axis, event); }} />
                  <circle cx={target.x} cy={target.y} r={6} className={`gizmo-handle ${classes}`} onPointerDown={(event) => { event.stopPropagation(); props.startAxisDrag(axis, event); }} />
                </g>
              );
            });
          })() : null}

          {props.selectionRect ? (
            <rect x={props.selectionRect.x} y={props.selectionRect.y} width={props.selectionRect.width} height={props.selectionRect.height} className="selection-box" />
          ) : null}
        </g>
        <g>
          <text x="48" y="428" className="svg-copy svg-copy--muted">
            WebGL viewport · {props.projectionMode} · {props.immersiveViewport ? "immersive" : "dock"}
          </text>
          <text x="322" y="428" className="svg-copy svg-copy--muted">
            preset {props.activeViewPreset}
          </text>
          {hasDeformation ? (
            <text x="510" y="428" className="svg-copy svg-copy--muted">
              deformed x{deformationScale.toFixed(1)}
            </text>
          ) : null}
          {hasDeformation ? ([
            { mode: "original", x: 638, label: "original" },
            { mode: "overlay", x: 724, label: "overlay" },
            { mode: "deformed", x: 804, label: "deformed" },
          ] as const).map((entry) => (
            <g key={entry.mode} transform={`translate(${entry.x} 406)`} onPointerDown={(event) => { event.stopPropagation(); setDeformationViewMode(entry.mode); }} style={{ cursor: "pointer" }}>
              <rect width={entry.mode === "deformed" ? 82 : 74} height="18" rx="9" fill={deformationViewMode === entry.mode ? "rgba(93, 217, 255, 0.2)" : "rgba(15, 23, 42, 0.18)"} stroke={deformationViewMode === entry.mode ? "rgba(93, 217, 255, 0.65)" : "rgba(148, 163, 184, 0.24)"} />
              <text x="10" y="12.5" className="svg-copy svg-copy--muted">{entry.label}</text>
            </g>
          )) : null}
          {activeReadout ? (
            <g data-workbench-3d-readout="true" transform={`translate(690 ${VIEWPORT_CLIP.y + VIEWPORT_CLIP.height - 36 - activeReadout.lines.length * 14})`}>
              <rect width="222" height={28 + activeReadout.lines.length * 14} rx="12" fill={persistentReadout ? "rgba(8, 20, 30, 0.94)" : "rgba(11, 16, 24, 0.88)"} stroke={persistentReadout ? "rgba(93, 217, 255, 0.34)" : "rgba(148, 163, 184, 0.24)"} />
              <text x="12" y="18" className="svg-copy">{activeReadout.title}</text>
              {activeReadout.lines.map((line, index) => (
                <text key={`${activeReadout.title}-${index}`} x="12" y={34 + index * 14} className="svg-copy svg-copy--muted">{line}</text>
              ))}
            </g>
          ) : null}
        </g>
      </svg>
    </div>
  );
}
