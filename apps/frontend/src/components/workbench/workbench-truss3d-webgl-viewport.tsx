"use client";

import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import { lineInsideViewport, pointInsideViewport, projectTruss3dPoint, type CameraState, type DisplayTruss3dElement, type DisplayTruss3dNode, type ProjectionMode, type ViewPreset, VIEWPORT_CLIP } from "@/components/workbench/workbench-viewport-core";
import { buildTruss3dSceneBuffers, resolveDeformationScale as resolveTruss3dDeformationScale, type DeformationViewMode } from "@/components/workbench/workbench-truss3d-webgl-scene";
import { buildElementReadout, buildNodeReadout, buildSelectionSummary, type Truss3dReadout, type Truss3dReadoutStudyKind } from "@/components/workbench/workbench-truss3d-readout";

type Props = {
  activeViewPreset: ViewPreset;
  boxSelectMode: boolean;
  camera: CameraState;
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
  projected3d: { minX: number; maxX: number; minZ: number; maxZ: number; width: number; height: number };
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
const PADDING_X = 120;
const PADDING_Y = 80;
const USABLE_WIDTH = VIEWBOX_WIDTH - PADDING_X * 2;
const USABLE_HEIGHT = VIEWBOX_HEIGHT - PADDING_Y * 2;

const VERTEX_SHADER = `
attribute vec3 aPosition;
attribute vec4 aColor;
attribute float aPointSize;
uniform vec4 uBounds;
uniform vec3 uCamera;
uniform vec2 uPan;
uniform float uPerspective;
varying vec4 vColor;

vec3 rotatePoint(vec3 point, vec3 camera) {
  float cy = cos(camera.x);
  float sy = sin(camera.x);
  float cp = cos(camera.y);
  float sp = sin(camera.y);
  float yawX = point.x * cy - point.y * sy;
  float yawY = point.x * sy + point.y * cy;
  float pitchY = yawY * cp - point.z * sp;
  float pitchZ = yawY * sp + point.z * cp;
  return vec3(yawX, pitchY, pitchZ);
}

void main() {
  vec3 rotated = rotatePoint(aPosition, uCamera);
  float depth = uPerspective > 0.5 ? 8.0 / (8.0 + rotated.y) : 1.0;
  float baseX = ((rotated.x - uBounds.x) / uBounds.z) * ${USABLE_WIDTH.toFixed(1)};
  float baseY = (1.0 - ((rotated.z - uBounds.y) / uBounds.w)) * ${USABLE_HEIGHT.toFixed(1)};
  float screenX = ${PADDING_X.toFixed(1)} + baseX * uCamera.z * depth + uPan.x;
  float shiftedY = ${PADDING_Y.toFixed(1)} + baseY * uCamera.z * depth + uPan.y;
  float clipX = (screenX / ${VIEWBOX_WIDTH.toFixed(1)}) * 2.0 - 1.0;
  float clipY = 1.0 - (shiftedY / ${VIEWBOX_HEIGHT.toFixed(1)}) * 2.0;
  gl_Position = vec4(clipX, clipY, 0.0, 1.0);
  gl_PointSize = aPointSize;
  vColor = aColor;
}`;

const FRAGMENT_SHADER = `precision mediump float; varying vec4 vColor; void main() { gl_FragColor = vColor; }`;

function createShader(gl: WebGLRenderingContext, type: number, source: string) {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) { gl.deleteShader(shader); return null; }
  return shader;
}

function createProgram(gl: WebGLRenderingContext) {
  const vertexShader = createShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
  const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
  if (!vertexShader || !fragmentShader) return null;
  const program = gl.createProgram();
  if (!program) return null;
  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) { gl.deleteProgram(program); return null; }
  return program;
}

function drawScene(canvas: HTMLCanvasElement, props: Props, deformationViewMode: DeformationViewMode) {
  const gl = canvas.getContext("webgl", { antialias: true, alpha: true });
  if (!gl) return;
  const program = createProgram(gl);
  if (!program) return;

  const pixelRatio = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  const width = Math.max(1, Math.round(rect.width * pixelRatio));
  const height = Math.max(1, Math.round(rect.height * pixelRatio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }

  const buffers = buildTruss3dSceneBuffers({ ...props, deformationViewMode });
  const aPosition = gl.getAttribLocation(program, "aPosition");
  const aColor = gl.getAttribLocation(program, "aColor");
  const aPointSize = gl.getAttribLocation(program, "aPointSize");
  const uBounds = gl.getUniformLocation(program, "uBounds");
  const uCamera = gl.getUniformLocation(program, "uCamera");
  const uPan = gl.getUniformLocation(program, "uPan");
  const uPerspective = gl.getUniformLocation(program, "uPerspective");

  const linePositionBuffer = gl.createBuffer();
  const lineColorBuffer = gl.createBuffer();
  const nodePositionBuffer = gl.createBuffer();
  const nodeColorBuffer = gl.createBuffer();
  const nodeSizeBuffer = gl.createBuffer();
  if (!linePositionBuffer || !lineColorBuffer || !nodePositionBuffer || !nodeColorBuffer || !nodeSizeBuffer) {
    gl.deleteProgram(program);
    return;
  }

  gl.viewport(0, 0, width, height);
  gl.clearColor(0.05, 0.06, 0.08, 0);
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.useProgram(program);
  gl.uniform4f(uBounds, props.projected3d.minX, props.projected3d.minZ, props.projected3d.width, props.projected3d.height);
  gl.uniform3f(uCamera, props.camera.yaw, props.camera.pitch, props.camera.zoom);
  gl.uniform2f(uPan, props.camera.panX, props.camera.panY);
  gl.uniform1f(uPerspective, props.projectionMode === "persp" ? 1 : 0);

  gl.bindBuffer(gl.ARRAY_BUFFER, linePositionBuffer);
  gl.bufferData(gl.ARRAY_BUFFER, buffers.linePositions, gl.STATIC_DRAW);
  gl.enableVertexAttribArray(aPosition);
  gl.vertexAttribPointer(aPosition, 3, gl.FLOAT, false, 0, 0);

  gl.bindBuffer(gl.ARRAY_BUFFER, lineColorBuffer);
  gl.bufferData(gl.ARRAY_BUFFER, buffers.lineColors, gl.STATIC_DRAW);
  gl.enableVertexAttribArray(aColor);
  gl.vertexAttribPointer(aColor, 4, gl.FLOAT, false, 0, 0);

  gl.disableVertexAttribArray(aPointSize);
  gl.vertexAttrib1f(aPointSize, 1);
  gl.drawArrays(gl.LINES, 0, buffers.linePositions.length / 3);

  if (buffers.deformedLinePositions.length > 0) {
    gl.bindBuffer(gl.ARRAY_BUFFER, linePositionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.deformedLinePositions, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aPosition);
    gl.vertexAttribPointer(aPosition, 3, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, lineColorBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.deformedLineColors, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aColor);
    gl.vertexAttribPointer(aColor, 4, gl.FLOAT, false, 0, 0);
    gl.drawArrays(gl.LINES, 0, buffers.deformedLinePositions.length / 3);
  }

  if (buffers.nodePositions.length > 0) {
    gl.bindBuffer(gl.ARRAY_BUFFER, nodePositionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.nodePositions, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aPosition);
    gl.vertexAttribPointer(aPosition, 3, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, nodeColorBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.nodeColors, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aColor);
    gl.vertexAttribPointer(aColor, 4, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, nodeSizeBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.nodeSizes, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aPointSize);
    gl.vertexAttribPointer(aPointSize, 1, gl.FLOAT, false, 0, 0);
    gl.drawArrays(gl.POINTS, 0, buffers.nodePositions.length / 3);
  }

  if (buffers.deformedNodePositions.length > 0) {
    gl.bindBuffer(gl.ARRAY_BUFFER, nodePositionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.deformedNodePositions, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aPosition);
    gl.vertexAttribPointer(aPosition, 3, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, nodeColorBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.deformedNodeColors, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aColor);
    gl.vertexAttribPointer(aColor, 4, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, nodeSizeBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, buffers.nodeSizes, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(aPointSize);
    gl.vertexAttribPointer(aPointSize, 1, gl.FLOAT, false, 0, 0);
    gl.drawArrays(gl.POINTS, 0, buffers.deformedNodePositions.length / 3);
  }

  gl.deleteBuffer(linePositionBuffer);
  gl.deleteBuffer(lineColorBuffer);
  gl.deleteBuffer(nodePositionBuffer);
  gl.deleteBuffer(nodeColorBuffer);
  gl.deleteBuffer(nodeSizeBuffer);
  gl.deleteProgram(program);
}

export function WorkbenchTruss3dWebglViewport(props: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null), deformationScale = useMemo(() => resolveTruss3dDeformationScale(props.visibleTruss3dNodes, !props.isModelMode), [props.isModelMode, props.visibleTruss3dNodes]);
  const hasDeformation = !props.isModelMode && deformationScale > 1;
  const [hoverReadout, setHoverReadout] = useState<Truss3dReadout>(null);
  const [deformationViewMode, setDeformationViewMode] = useState<DeformationViewMode>("overlay");
  const selectedElementData = useMemo(() => props.visibleTruss3dElements.find((element) => element.index === props.selectedTruss3dElement) ?? null, [props.selectedTruss3dElement, props.visibleTruss3dElements]);
  const selectedNodeGroup = useMemo(
    () => props.selectedTruss3dNodeIndices.map((index) => props.displayTruss3dNodes[index]).filter((node): node is DisplayTruss3dNode => Boolean(node)),
    [props.displayTruss3dNodes, props.selectedTruss3dNodeIndices],
  );
  const persistentReadout =
    selectedElementData ? buildElementReadout(props.studyKind, selectedElementData) : selectedNodeGroup.length > 1 ? buildSelectionSummary(props.studyKind, selectedNodeGroup) : props.selected3dNodeData ? buildNodeReadout(props.studyKind, props.selected3dNodeData) : null;
  const activeReadout = persistentReadout ?? hoverReadout;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    drawScene(canvas, props, deformationViewMode);
  }, [
    props.camera.panX,
    props.camera.panY,
    props.camera.pitch,
    props.camera.yaw,
    props.camera.zoom,
    props.displayTruss3dNodes,
    props.gridExtent,
    props.gridStep,
    props.hiddenTruss3dMaterialIds,
    props.projected3d.height,
    props.projected3d.minX,
    props.projected3d.minZ,
    props.projected3d.width,
    props.projectionMode,
    props.selectedTruss3dElement,
    props.selectedTruss3dNode,
    props.selectedTruss3dNodeIndices,
    props.showGrid,
    props.showNodes,
    props.truss3dElementColors,
    props.truss3dLinkMode,
    props.visibleTruss3dElements,
    props.visibleTruss3dNodes,
    props.memberDraftNodes,
    deformationViewMode,
  ]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => drawScene(canvas, props, deformationViewMode));
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [props, deformationViewMode]);

  const projectedNodes = useMemo(() => props.visibleTruss3dNodes.map((node, index) => ({ index, point: projectTruss3dPoint(node, props.projected3d, props.camera, props.projectionMode) })), [props.visibleTruss3dNodes, props.projected3d, props.camera, props.projectionMode]);

  return (
    <div className="viewport-3d-shell" style={{ position: "relative", width: props.svgStyle?.width, minWidth: props.svgStyle?.minWidth, aspectRatio: `${VIEWBOX_WIDTH} / ${VIEWBOX_HEIGHT}` }}>
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
        <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
        <text x="48" y="58" className="svg-title">{props.truss3dTitle}</text>
        {props.truss3dLegend ? <text x="48" y="78" className="svg-copy svg-copy--muted">{props.truss3dLegend}</text> : null}
        <text x={props.immersiveViewport ? 660 : 790} y="58" className="legend-label">{props.workspaceBadge}</text>
        <text x={props.immersiveViewport ? 560 : 640} y="58" className="legend-label">{props.projectionMode === "ortho" ? "ORTHO" : "PERSP"}</text>
        <g clipPath="url(#viewportClipTruss3dWebgl)">
          <line x1="74" y1="386" x2="130" y2="386" className="guide" />
          <line x1="74" y1="386" x2="74" y2="330" className="guide" />
          <line x1="74" y1="386" x2="104" y2="356" className="guide guide--soft" />
          <text x="136" y="390" className="node-label">X</text>
          <text x="68" y="324" className="node-label">Z</text>
          <text x="108" y="350" className="node-label">Y</text>

          {props.visibleTruss3dElements.map((element) => {
            if (element.material_id && props.hiddenTruss3dMaterialIds.includes(element.material_id)) return null;
            const start = projectTruss3dPoint(props.displayTruss3dNodes[element.node_i], props.projected3d, props.camera, props.projectionMode);
            const end = projectTruss3dPoint(props.displayTruss3dNodes[element.node_j], props.projected3d, props.camera, props.projectionMode);
            if (!lineInsideViewport(start, end, 36)) return null;
            return (
              <line
                key={`hit-space-${element.id}`}
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
                  event.stopPropagation();
                  props.onSelectTruss3dElement(element.index);
                }}
              />
            );
          })}

          {props.draftStartNode && props.hoveredTruss3dNode !== null && props.hoveredTruss3dNode !== props.draftStartNodeIndex ? (() => {
            const start = projectTruss3dPoint(props.draftStartNode, props.projected3d, props.camera, props.projectionMode);
            const endNode = props.displayTruss3dNodes[props.hoveredTruss3dNode];
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
              props.selectedTruss3dNodeIndices.includes(absoluteIndex) || props.selectedTruss3dNode === absoluteIndex;
            return (
              <g key={node.id}>
                <circle
                  cx={point.x}
                  cy={point.y}
                  r={props.showNodes ? (isSelected ? 9 : 7) : 12}
                  fill={props.showNodes ? "rgba(0,0,0,0)" : "transparent"}
                  className={`${isSelected ? "node-base node-base--active" : ""}${props.memberDraftNodes.includes(absoluteIndex) ? " node-base--draft" : ""}${props.truss3dLinkMode && props.hoveredTruss3dNode === absoluteIndex ? " node-base--warning" : ""}`}
                  style={{ pointerEvents: "auto" }}
                  onPointerDown={(event) => {
                    if (props.boxSelectMode) return;
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
            <g transform="translate(690 364)">
              <rect width="222" height={20 + activeReadout.lines.length * 14} rx="12" fill={persistentReadout ? "rgba(8, 20, 30, 0.94)" : "rgba(11, 16, 24, 0.88)"} stroke={persistentReadout ? "rgba(93, 217, 255, 0.34)" : "rgba(148, 163, 184, 0.24)"} />
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
