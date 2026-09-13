import { projectTruss3dPoint, TRUSS3D_PROJECTION, type CameraState, type ProjectionMode,
  type DisplayTruss3dNode, type buildProjectedBounds } from "./workbench-viewport-core";

export const TRUSS3D_ZOOM = { min: 0.08, max: 1024 };
const clampZoom = (zoom: number) => Math.max(TRUSS3D_ZOOM.min, Math.min(TRUSS3D_ZOOM.max, zoom));

export function zoomTruss3dAt(camera: CameraState, anchor: { x: number; y: number }, deltaY: number, deltaMode = 0) {
  if (![deltaY, anchor.x, anchor.y].every(Number.isFinite) || deltaY === 0) return camera;
  const pixels = deltaY * (deltaMode === 1 ? 16 : deltaMode === 2 ? 460 : 1);
  const zoom = clampZoom(camera.zoom * Math.exp(-Math.max(-500, Math.min(500, pixels)) * 0.002));
  const ratio = zoom / camera.zoom;
  const x = anchor.x - TRUSS3D_PROJECTION.centerX, y = anchor.y - TRUSS3D_PROJECTION.centerY;
  return { ...camera, zoom, panX: x - (x - camera.panX) * ratio, panY: y - (y - camera.panY) * ratio };
}

export function focusTruss3d(camera: CameraState, bounds: ReturnType<typeof buildProjectedBounds>,
  projection: ProjectionMode, nodes: readonly Pick<DisplayTruss3dNode, "x" | "y" | "z">[]) {
  if (!nodes.length) return { ...camera, zoom: 1, panX: 0, panY: 0 };
  const fit = { ...camera, zoom: 1, panX: 0, panY: 0 };
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const node of nodes) {
    const p = projectTruss3dPoint(node, bounds, fit, projection);
    minX = Math.min(minX, p.x); minY = Math.min(minY, p.y);
    maxX = Math.max(maxX, p.x); maxY = Math.max(maxY, p.y);
  }
  const zoom = nodes.length === 1 ? Math.max(8, camera.zoom) : clampZoom(Math.min(600 / Math.max(maxX - minX, 1e-6), 220 / Math.max(maxY - minY, 1e-6)));
  return { ...camera, zoom, panX: (TRUSS3D_PROJECTION.centerX - (minX + maxX) / 2) * zoom,
    panY: (TRUSS3D_PROJECTION.centerY - (minY + maxY) / 2) * zoom };
}
