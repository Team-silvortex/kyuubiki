"use client";

import { memo, type PointerEvent as ReactPointerEvent } from "react";

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
  truss3dProjectedBounds: Bounds;
  planeNodes: PlaneNode[];
  planeElements: PlaneElement[];
  planeBounds: Bounds;
  planeResult: boolean;
  planeMaxVonMises: number;
  selectedPlaneNodeId: string | null;
  onSelectPlaneElement: (index: number) => void;
  onSelectPlaneNode: (index: number) => void;
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

function projectTruss3dPoint(node: { x: number; y: number; z: number }, bounds: Bounds) {
  const isoX = node.x - node.y * 0.55;
  const isoY = node.z + node.y * 0.35;
  return toSvgPoint({ x: isoX, y: isoY }, bounds);
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
  truss3dProjectedBounds,
  planeNodes,
  planeElements,
  planeBounds,
  planeResult,
  planeMaxVonMises,
  selectedPlaneNodeId,
  onSelectPlaneElement,
  onSelectPlaneNode,
}: WorkbenchViewportProps) {
  const isModelMode = sidebarSection === "model";
  const trussLabelStep = stepForDensity(displayTrussNodes.length, isModelMode ? 28 : 18);
  const trussMarkerStep = stepForDensity(displayTrussNodes.length, isModelMode ? 240 : 140);
  const trussDeformedStep = stepForDensity(displayTrussElements.length, 320);
  const truss3dLabelStep = stepForDensity(displayTruss3dNodes.length, 20);
  const planeNodeLabelStep = stepForDensity(planeNodes.length, isModelMode ? 24 : 16);
  const planeNodeMarkerStep = stepForDensity(planeNodes.length, isModelMode ? 200 : 120);
  const planeDeformedStep = stepForDensity(planeElements.length, 240);

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
        className="viewport-svg"
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
          {displayTrussElements.map((element) => {
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
            ? displayTrussElements.flatMap((element, index) => {
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

          {displayTrussNodes.flatMap((node, index) => {
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
      <svg viewBox="0 0 980 460" className="viewport-svg" aria-label="3d truss response">
        <defs>
          <clipPath id="viewportClipTruss3d">
            <rect x={VIEWPORT_CLIP.x} y={VIEWPORT_CLIP.y} width={VIEWPORT_CLIP.width} height={VIEWPORT_CLIP.height} rx="22" />
          </clipPath>
        </defs>
        <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
        <text x="48" y="58" className="svg-title">
          {truss3dTitle}
        </text>
        <g clipPath="url(#viewportClipTruss3d)">
          {displayTruss3dElements.map((element) => {
            const start = projectTruss3dPoint(displayTruss3dNodes[element.node_i], truss3dProjectedBounds);
            const end = projectTruss3dPoint(displayTruss3dNodes[element.node_j], truss3dProjectedBounds);
            if (!lineInsideViewport(start, end)) return null;
            return (
              <line
                key={`space-${element.id}`}
                x1={start.x}
                y1={start.y}
                x2={end.x}
                y2={end.y}
                className="bar bar--base"
              />
            );
          })}
          {displayTruss3dNodes.map((node, index) => {
            const point = projectTruss3dPoint(node, truss3dProjectedBounds);
            if (!pointInsideViewport(point, 24)) return null;
            const showLabel = index % truss3dLabelStep === 0;
            return (
              <g key={node.id}>
                <circle cx={point.x} cy={point.y} r={7} className="node-base" />
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

  return (
    <svg viewBox="0 0 980 460" className="viewport-svg" aria-label="2d plane triangle response">
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
        {planeElements.map((element) => {
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
              style={{ fill: planeStressFill(element.von_mises ?? 0, planeMaxVonMises) }}
              onPointerDown={() => {
                if (isModelMode) onSelectPlaneElement(element.index);
              }}
            />
          );
        })}
        {planeResult
          ? planeElements.flatMap((element, index) => {
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
        {planeNodes.flatMap((node, index) => {
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
