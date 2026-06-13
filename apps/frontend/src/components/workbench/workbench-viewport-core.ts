"use client";

import type { PointerEvent as ReactPointerEvent } from "react";
import type { ViewportRenderDiagnostics, ViewportRenderStrategy } from "./workbench-render-diagnostics";

type SidebarSection = "study" | "model" | "workflow" | "library" | "system";
export type StudyKind =
  | "axial_bar_1d"
  | "heat_bar_1d"
  | "electrostatic_plane_triangle_2d"
  | "electrostatic_plane_quad_2d"
  | "heat_plane_triangle_2d"
  | "heat_plane_quad_2d"
  | "thermal_bar_1d"
  | "thermal_beam_1d"
  | "thermal_frame_2d"
  | "thermal_truss_2d"
  | "thermal_truss_3d"
  | "thermal_plane_triangle_2d"
  | "thermal_plane_quad_2d"
  | "spring_1d"
  | "spring_2d"
  | "spring_3d"
  | "beam_1d"
  | "torsion_1d"
  | "truss_2d"
  | "truss_3d"
  | "plane_triangle_2d"
  | "plane_quad_2d"
  | "frame_2d";

export type PlaneResultField =
  | "von_mises"
  | "principal_stress_1"
  | "max_in_plane_shear"
  | "average_potential"
  | "potential_gradient_x"
  | "potential_gradient_y"
  | "electric_field_x"
  | "electric_field_y"
  | "electric_field_magnitude"
  | "electric_flux_density_x"
  | "electric_flux_density_y"
  | "electric_flux_density_magnitude"
  | "average_temperature"
  | "average_temperature_delta"
  | "temperature_gradient_x"
  | "temperature_gradient_y"
  | "heat_flux_x"
  | "heat_flux_y"
  | "heat_flux_magnitude"
  | "thermal_strain"
  | "mechanical_strain";

export type LineResultField =
  | "axial_stress"
  | "max_bending_stress"
  | "max_combined_stress"
  | "moment"
  | "shear_force"
  | "average_temperature_delta"
  | "temperature_gradient_y"
  | "thermal_curvature";

export type DisplayTrussNode = {
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

export type DisplayTrussElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
  axial_stress?: number;
  max_bending_stress?: number;
  max_combined_stress?: number;
  shear_force_i?: number;
  moment_i?: number;
  shear_force_j?: number;
  moment_j?: number;
  average_temperature_delta?: number;
  temperature_gradient_y?: number;
  thermal_curvature?: number;
  material_id?: string;
};

export type DisplayTruss3dNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  z: number;
  ux: number;
  uy: number;
  uz: number;
  temperature_delta?: number;
};

export type DisplayTruss3dElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
  average_temperature_delta?: number;
  thermal_strain?: number;
  mechanical_strain?: number;
  total_strain?: number;
  material_id?: string;
};

export type PlaneNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  ux: number;
  uy: number;
  potential?: number;
  charge_density?: number;
  fix_potential?: boolean;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type PlaneElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l?: number;
  von_mises?: number;
  principal_stress_1?: number;
  max_in_plane_shear?: number;
  average_temperature?: number;
  average_temperature_delta?: number;
  temperature_gradient_x?: number;
  temperature_gradient_y?: number;
  heat_flux_x?: number;
  heat_flux_y?: number;
  heat_flux_magnitude?: number;
  average_potential?: number;
  potential_gradient_x?: number;
  potential_gradient_y?: number;
  electric_field_x?: number;
  electric_field_y?: number;
  electric_field_magnitude?: number;
  electric_flux_density_x?: number;
  electric_flux_density_y?: number;
  electric_flux_density_magnitude?: number;
  thermal_strain?: number;
  mechanical_strain_x?: number;
  mechanical_strain_y?: number;
  material_id?: string;
};

export type Bounds = {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  width: number;
  height: number;
};

export type CameraState = {
  yaw: number;
  pitch: number;
  zoom: number;
  panX: number;
  panY: number;
};

export type ViewPreset = "iso" | "front" | "right" | "top";
export type ProjectionMode = "ortho" | "persp";

export type WorkbenchViewportProps = {
  studyKind: StudyKind;
  sidebarSection: SidebarSection;
  title: string;
  axialTitle: string;
  trussTitle: string;
  trussLegend?: string;
  truss3dTitle: string;
  truss3dLegend?: string;
  planeTitle: string;
  planeLegend: string;
  axialNodes: Array<{ x: number; displacement: number }>;
  axialLength: number;
  axialScale: number;
  displayTrussNodes: DisplayTrussNode[];
  displayTrussElements: DisplayTrussElement[];
  trussElementColors: string[];
  hiddenTrussMaterialIds: string[];
  trussBounds: Bounds;
  trussResult: boolean;
  frameResultField: LineResultField;
  frameResultFieldMax: number;
  focusedFrameElement: number | null;
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
  planeResultField: PlaneResultField;
  planeResultFieldMax: number;
  selectedPlaneNodeId: string | null;
  focusedPlaneElement: number | null;
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
  onRenderDiagnosticsChange?: (diagnostics: ViewportRenderDiagnostics) => void;
  renderStrategy?: ViewportRenderStrategy;
};

export const VIEWPORT_CLIP = { x: 48, y: 76, width: 884, height: 340 };

export function toSvgPoint(node: { x: number; y: number }, bounds: Bounds) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;

  return {
    x: paddingX + ((node.x - bounds.minX) / bounds.width) * usableWidth,
    y: 460 - paddingY - ((node.y - bounds.minY) / bounds.height) * usableHeight,
  };
}

function rotatePoint(node: { x: number; y: number; z: number }, camera: CameraState) {
  const cy = Math.cos(camera.yaw);
  const sy = Math.sin(camera.yaw);
  const cp = Math.cos(camera.pitch);
  const sp = Math.sin(camera.pitch);
  const yawX = node.x * cy - node.y * sy;
  const yawY = node.x * sy + node.y * cy;
  const pitchY = yawY * cp - node.z * sp;
  const pitchZ = yawY * sp + node.z * cp;
  return { x: yawX, y: pitchY, z: pitchZ };
}

export function buildProjectedBounds(nodes: DisplayTruss3dNode[], camera: CameraState) {
  if (!nodes.length) return { minX: -1, maxX: 1, minZ: -1, maxZ: 1, width: 2, height: 2 };
  const rotated = nodes.map((node) => rotatePoint(node, camera));
  const xs = rotated.map((node) => node.x);
  const zs = rotated.map((node) => node.z);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minZ = Math.min(...zs);
  const maxZ = Math.max(...zs);
  return {
    minX,
    maxX,
    minZ,
    maxZ,
    width: Math.max(maxX - minX, 1.0e-6),
    height: Math.max(maxZ - minZ, 1.0e-6),
  };
}

export function cameraForPreset(preset: ViewPreset): CameraState {
  if (preset === "front") return { yaw: 0, pitch: 0, zoom: 1.2, panX: 0, panY: 0 };
  if (preset === "right") return { yaw: Math.PI / 2, pitch: 0, zoom: 1.2, panX: 0, panY: 0 };
  if (preset === "top") return { yaw: 0, pitch: -Math.PI / 2 + 0.0001, zoom: 1.15, panX: 0, panY: 0 };
  return { yaw: -0.78, pitch: 0.66, zoom: 1.18, panX: 0, panY: 0 };
}

export function pointerToViewport(event: ReactPointerEvent<SVGSVGElement>) {
  const rect = event.currentTarget.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left) / rect.width) * 980,
    y: ((event.clientY - rect.top) / rect.height) * 460,
  };
}

export function projectTruss3dPoint(
  node: { x: number; y: number; z: number },
  bounds: { minX: number; maxX: number; minZ: number; maxZ: number; width: number; height: number },
  camera: CameraState,
  projection: ProjectionMode,
) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;
  const rotated = rotatePoint(node, camera);
  const depth = projection === "persp" ? 8 / (8 + rotated.y) : 1;
  const baseX = ((rotated.x - bounds.minX) / bounds.width) * usableWidth;
  const baseY = (1 - (rotated.z - bounds.minZ) / bounds.height) * usableHeight;
  return {
    x: paddingX + baseX * camera.zoom * depth + camera.panX,
    y: paddingY + baseY * camera.zoom * depth + camera.panY,
  };
}

export function rotatedDeltaToWorld(deltaX: number, deltaZ: number, camera: CameraState) {
  const cy = Math.cos(camera.yaw);
  const sy = Math.sin(camera.yaw);
  return { x: deltaX * cy, y: -deltaX * sy, z: deltaZ };
}

export function planeStressFill(value: number, maxValue: number): string {
  if (!Number.isFinite(value) || !Number.isFinite(maxValue) || maxValue <= 1.0e-9) return "hsl(206 20% 84%)";
  const ratio = Math.min(1, Math.max(0, Math.abs(value) / maxValue));
  const hue = 205 - ratio * 175;
  const lightness = 84 - ratio * 28;
  return `hsl(${hue.toFixed(0)} 76% ${lightness.toFixed(0)}%)`;
}

export function pointInsideViewport(point: { x: number; y: number }, margin = 18) {
  return (
    point.x >= VIEWPORT_CLIP.x - margin &&
    point.x <= VIEWPORT_CLIP.x + VIEWPORT_CLIP.width + margin &&
    point.y >= VIEWPORT_CLIP.y - margin &&
    point.y <= VIEWPORT_CLIP.y + VIEWPORT_CLIP.height + margin
  );
}

export function lineInsideViewport(start: { x: number; y: number }, end: { x: number; y: number }, margin = 24) {
  const minX = VIEWPORT_CLIP.x - margin;
  const maxX = VIEWPORT_CLIP.x + VIEWPORT_CLIP.width + margin;
  const minY = VIEWPORT_CLIP.y - margin;
  const maxY = VIEWPORT_CLIP.y + VIEWPORT_CLIP.height + margin;
  return !(
    (start.x < minX && end.x < minX) ||
    (start.x > maxX && end.x > maxX) ||
    (start.y < minY && end.y < minY) ||
    (start.y > maxY && end.y > maxY)
  );
}

export function polygonInsideViewport(points: Array<{ x: number; y: number }>, margin = 24) {
  const minX = VIEWPORT_CLIP.x - margin;
  const maxX = VIEWPORT_CLIP.x + VIEWPORT_CLIP.width + margin;
  const minY = VIEWPORT_CLIP.y - margin;
  const maxY = VIEWPORT_CLIP.y + VIEWPORT_CLIP.height + margin;
  return points.some((point) => point.x >= minX && point.x <= maxX && point.y >= minY && point.y <= maxY);
}

export function stepForDensity(length: number, softLimit: number) {
  return Math.max(1, Math.ceil(length / Math.max(softLimit, 1)));
}

export function initialRenderBudget(length: number) {
  if (length > 40_000) return 1_600;
  if (length > 20_000) return 2_400;
  if (length > 10_000) return 3_600;
  return length;
}

export function renderBatchSize(length: number) {
  if (length > 40_000) return 1_400;
  if (length > 20_000) return 1_800;
  if (length > 10_000) return 2_400;
  return length;
}
