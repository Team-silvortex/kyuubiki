"use client";

import { useMemo, useState } from "react";
import {
  planeStressFill,
  pointInsideViewport,
  polygonInsideViewport,
  toSvgPoint,
  type Bounds,
  type PlaneElement,
  type PlaneNode,
  type PlaneResultField,
  VIEWPORT_CLIP,
} from "@/components/workbench/workbench-viewport-core";
import { buildPlaneElementReadout, buildPlaneNodeReadout, type PlaneReadout } from "@/components/workbench/workbench-plane-readout";

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
  return <line key={key} x1={point.x} y1={point.y} x2={point.x + dx} y2={point.y + dy} className="load-glyph" markerEnd="url(#arrow-head)" />;
}

function planeFieldValue(element: PlaneElement, field: PlaneResultField) {
  switch (field) {
    case "average_temperature":
      return Math.abs(element.average_temperature ?? 0);
    case "average_temperature_delta":
      return Math.abs(element.average_temperature_delta ?? 0);
    case "temperature_gradient_x":
      return Math.abs(element.temperature_gradient_x ?? 0);
    case "temperature_gradient_y":
      return Math.abs(element.temperature_gradient_y ?? 0);
    case "heat_flux_x":
      return Math.abs(element.heat_flux_x ?? 0);
    case "heat_flux_y":
      return Math.abs(element.heat_flux_y ?? 0);
    case "heat_flux_magnitude":
      return Math.abs(element.heat_flux_magnitude ?? 0);
    case "thermal_strain":
      return Math.abs(element.thermal_strain ?? 0);
    case "mechanical_strain":
      return Math.max(Math.abs(element.mechanical_strain_x ?? 0), Math.abs(element.mechanical_strain_y ?? 0));
    case "principal_stress_1":
      return Math.abs(element.principal_stress_1 ?? 0);
    case "max_in_plane_shear":
      return Math.abs(element.max_in_plane_shear ?? 0);
    default:
      return Math.abs(element.von_mises ?? 0);
  }
}

type WorkbenchPlaneViewportProps = {
  focusedPlaneElement: number | null;
  hiddenPlaneMaterialIds: string[];
  isModelMode: boolean;
  onSelectPlaneElement: (index: number) => void;
  onSelectPlaneNode: (index: number) => void;
  planeBounds: Bounds;
  planeDeformedStep: number;
  planeElementColors: string[];
  planeElements: PlaneElement[];
  planeLegend: string;
  planeNodeLabelStep: number;
  planeNodeMarkerStep: number;
  planeNodes: PlaneNode[];
  planeResult: boolean;
  planeResultField: PlaneResultField;
  planeResultFieldMax: number;
  planeTitle: string;
  selectedElement: number | null;
  selectedPlaneNodeId: string | null;
  studyKind: string;
  svgStyle?: { width: string; minWidth: string };
  visiblePlaneElements: PlaneElement[];
  visiblePlaneNodes: PlaneNode[];
};

export function WorkbenchPlaneViewport(props: WorkbenchPlaneViewportProps) {
  const [hoverReadout, setHoverReadout] = useState<PlaneReadout | null>(null);

  const selectedNodeData = useMemo(
    () => (props.selectedPlaneNodeId ? props.planeNodes.find((node) => node.id === props.selectedPlaneNodeId) ?? null : null),
    [props.planeNodes, props.selectedPlaneNodeId],
  );
  const selectedElementData = props.selectedElement !== null ? props.planeElements[props.selectedElement] ?? null : null;
  const persistentReadout =
    selectedElementData ? buildPlaneElementReadout(props.studyKind, selectedElementData) : selectedNodeData ? buildPlaneNodeReadout(props.studyKind, selectedNodeData) : null;
  const activeReadout = persistentReadout ?? hoverReadout;

  return (
    <svg
      viewBox="0 0 980 460"
      className="viewport-svg"
      style={props.svgStyle}
      aria-label="2d plane triangle response"
      onPointerLeave={() => setHoverReadout(null)}
    >
      <defs><clipPath id="viewportClipPlane"><rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" /></clipPath></defs>
      <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
      <text x="48" y="58" className="svg-title">{props.planeTitle}</text>
      {props.planeResult ? <text x="760" y="58" className="svg-copy svg-copy--muted">{props.planeLegend}</text> : null}
      <g clipPath="url(#viewportClipPlane)">
        {props.visiblePlaneElements.map((element) => {
          if (element.material_id && props.hiddenPlaneMaterialIds.includes(element.material_id)) return null;
          const nodeIndices = typeof element.node_l === "number" ? [element.node_i, element.node_j, element.node_k, element.node_l] : [element.node_i, element.node_j, element.node_k];
          const points = nodeIndices.map((nodeIndex) => toSvgPoint(props.planeNodes[nodeIndex], props.planeBounds));
          if (!polygonInsideViewport(points)) return null;
          return (
            <polygon
              key={`plane-${element.id}`}
              points={points.map((point) => `${point.x},${point.y}`).join(" ")}
              className={`plane-triangle${props.selectedElement === element.index ? " plane-triangle--active" : ""}${props.focusedPlaneElement === element.index ? " plane-triangle--focused" : ""}`}
              style={{ fill: props.planeResult ? planeStressFill(planeFieldValue(element, props.planeResultField), props.planeResultFieldMax) : props.planeElementColors[element.index] }}
              onPointerDown={() => {
                if (props.isModelMode) props.onSelectPlaneElement(element.index);
              }}
              onPointerEnter={() => setHoverReadout(buildPlaneElementReadout(props.studyKind, element))}
              onPointerLeave={() => setHoverReadout((current) => (current?.kind === "element" ? null : current))}
            />
          );
        })}
        {props.planeResult
          ? props.visiblePlaneElements.flatMap((element, index) => {
              if (element.material_id && props.hiddenPlaneMaterialIds.includes(element.material_id)) return [];
              if (index % props.planeDeformedStep !== 0) return [];
              const nodeIndices = typeof element.node_l === "number" ? [element.node_i, element.node_j, element.node_k, element.node_l] : [element.node_i, element.node_j, element.node_k];
              const points = nodeIndices.map((nodeIndex) =>
                toSvgPoint({ x: props.planeNodes[nodeIndex].x + props.planeNodes[nodeIndex].ux * 5000, y: props.planeNodes[nodeIndex].y + props.planeNodes[nodeIndex].uy * 5000 }, props.planeBounds),
              );
              if (!polygonInsideViewport(points, 70)) return [];
              return <polygon key={`plane-def-${element.id}`} points={points.map((point) => `${point.x},${point.y}`).join(" ")} className="plane-triangle plane-triangle--deformed" />;
            })
          : null}
        {props.visiblePlaneNodes.flatMap((node, index) => {
          const point = toSvgPoint(node, props.planeBounds);
          if (!pointInsideViewport(point, 18)) return [];
          const showLabel = index % props.planeNodeLabelStep === 0 || props.selectedPlaneNodeId === node.id;
          const showMarker = index % props.planeNodeMarkerStep === 0 || showLabel;
          if (!showMarker) return [];
          return (
            <g key={node.id}>
              {renderSupportGlyph(point, node, `plane-support-${node.id}`)}
              {renderLoadGlyph(point, node, `plane-load-${node.id}`)}
              <circle
                cx={point.x}
                cy={point.y}
                r={props.selectedPlaneNodeId === node.id ? 8 : 6}
                className={`node-base${props.selectedPlaneNodeId === node.id ? " node-base--active" : ""}`}
                onPointerDown={() => {
                  if (props.isModelMode) props.onSelectPlaneNode(node.index);
                }}
                onPointerEnter={() => setHoverReadout(buildPlaneNodeReadout(props.studyKind, node))}
                onPointerLeave={() => setHoverReadout((current) => (current?.kind === "node" ? null : current))}
              />
              {showLabel ? <text x={point.x + 10} y={point.y - 10} className="node-label">{node.id}</text> : null}
            </g>
          );
        })}
        {activeReadout ? (
          <g transform="translate(734 308)">
            <rect width="222" height={20 + activeReadout.lines.length * 14} rx="12" fill={persistentReadout ? "rgba(8, 20, 30, 0.94)" : "rgba(11, 16, 24, 0.88)"} stroke={persistentReadout ? "rgba(93, 217, 255, 0.34)" : "rgba(148, 163, 184, 0.24)"} />
            <text x="12" y="18" className="svg-copy">{activeReadout.title}</text>
            {activeReadout.lines.map((line, index) => (
              <text key={`${activeReadout.title}-${index}`} x="12" y={34 + index * 14} className="svg-copy svg-copy--muted">{line}</text>
            ))}
          </g>
        ) : null}
      </g>
    </svg>
  );
}
