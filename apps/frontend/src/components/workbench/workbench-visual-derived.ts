"use client";

import { useMemo } from "react";
import { lineResultFieldValue, planeStressFill } from "@/components/workbench/workbench-result-helpers";
import { parseDirectMeshEndpoints } from "@/lib/workbench/helpers";
import { clampChunkOffset } from "@/lib/workbench/result-window";

type WorkbenchVisualDerivedArgs = {
  studyKind: string;
  isAxial: boolean;
  isBeam: boolean;
  isFrameLike: boolean;
  isHeatBar: boolean;
  isHeatPlane: boolean;
  isHeatPlaneTriangle: boolean;
  isSpring: boolean;
  isSpring3d: boolean;
  isThermalBar: boolean;
  isThermalTruss2d: boolean;
  isThermalTruss3d: boolean;
  isTorsion: boolean;
  isTruss: boolean;
  isTruss3d: boolean;
  activeResultWindow: { totalNodes: number; limit: number } | null;
  activeResultWindowLimitBase: number;
  resultWindowMaxTotal: number;
  resultWindowOffset: number;
  directMeshEndpointsText: string;
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  showShortcutHints: boolean;
  activeLineResultField: any;
  frameResultFieldMax: number;
  currentMaterials: any[];
  materialColorMap: Map<string, string>;
  displayTrussElements: Array<Record<string, any>>;
  displayTruss3dElements: Array<Record<string, any>>;
  activePlaneInputModel: { elements: Array<Record<string, any>>; nodes: Array<Record<string, any>> };
  planeModel: { elements: Array<Record<string, any>>; nodes: Array<Record<string, any>>; materials?: any[] };
  beamModel: { materials?: any[] };
  frameModel: { materials?: any[] };
  thermalBeamModel: { materials?: any[] };
  thermalFrameModel: { materials?: any[] };
  thermalTrussModel: { materials?: any[] };
  thermalTruss3dModel: { materials?: any[] };
  trussModel: { materials?: any[]; nodes: Array<Record<string, any>> };
  truss3dModel: { materials?: any[]; nodes: Array<Record<string, any>> };
  spring3dModel: { nodes: Array<Record<string, any>> };
  activeSpringModel: { nodes: Array<Record<string, any>> };
  activeBeamLikeModel: { nodes: Array<Record<string, any>> };
  torsionModel: { nodes: Array<Record<string, any>> };
  activeFrameLikeModel: { nodes: Array<Record<string, any>> };
  activeFrameLikeResult: Record<string, any> | null;
  activeBeamLikeResult: Record<string, any> | null;
  torsionResult: { max_stress?: number; max_rotation?: number; nodes?: any[] } | null;
  axialResult: { tip_displacement?: number; max_stress?: number; reaction_force?: number } | null;
  trussResult: { nodes: any[]; max_stress?: number; max_displacement?: number } | null;
  thermalTrussResult: { nodes: any[]; max_stress?: number; max_displacement?: number } | null;
  thermalTruss3dResult: { nodes: any[]; max_stress?: number; max_displacement?: number } | null;
  truss3dResult: { nodes: any[]; max_stress?: number; max_displacement?: number } | null;
  springResult: { nodes: any[]; max_force?: number; max_displacement?: number } | null;
  spring2dResult: { nodes: any[]; max_force?: number; max_displacement?: number } | null;
  spring3dResult: { nodes: any[]; max_force?: number; max_displacement?: number } | null;
  activeSpringResult: { nodes: any[]; max_force?: number; max_displacement?: number } | null;
  beamResult: { nodes: any[]; max_stress?: number; max_rotation?: number; max_displacement?: number } | null;
  frameResult: { nodes: any[]; max_stress?: number; max_rotation?: number; max_displacement?: number } | null;
  planeResult: { nodes: any[]; max_stress?: number; max_displacement?: number } | null;
  heatBarResult: { max_temperature?: number } | null;
  thermalBarResult: { max_displacement?: number; max_stress?: number } | null;
  thermalBeamResult: { max_displacement?: number; max_stress?: number } | null;
  thermalFrameResult: { max_displacement?: number; max_stress?: number } | null;
  axialNodes: any[];
  t: {
    jumpStart: string;
    jumpQuarter: string;
    jumpMid: string;
    jumpThreeQuarter: string;
    jumpEnd: string;
  };
};

export function resolveWorkbenchCurrentMaterials({
  studyKind,
  planeModel,
  beamModel,
  frameModel,
  thermalBeamModel,
  thermalFrameModel,
  thermalTrussModel,
  thermalTruss3dModel,
  trussModel,
  truss3dModel,
}: {
  studyKind: string;
  planeModel: { materials?: any[] };
  beamModel: { materials?: any[] };
  frameModel: { materials?: any[] };
  thermalBeamModel: { materials?: any[] };
  thermalFrameModel: { materials?: any[] };
  thermalTrussModel: { materials?: any[] };
  thermalTruss3dModel: { materials?: any[] };
  trussModel: { materials?: any[] };
  truss3dModel: { materials?: any[] };
}) {
  if (studyKind === "thermal_truss_2d") return thermalTrussModel.materials ?? [];
  if (studyKind === "truss_2d") return trussModel.materials ?? [];
  if (studyKind === "thermal_truss_3d") return thermalTruss3dModel.materials ?? [];
  if (studyKind === "truss_3d") return truss3dModel.materials ?? [];
  if (studyKind === "heat_bar_1d" || studyKind === "thermal_bar_1d") return [];
  if (studyKind === "thermal_beam_1d") return thermalBeamModel.materials ?? [];
  if (studyKind === "thermal_frame_2d") return thermalFrameModel.materials ?? [];
  if (studyKind === "spring_1d" || studyKind === "spring_2d") return [];
  if (studyKind === "beam_1d") return beamModel.materials ?? [];
  if (studyKind === "torsion_1d") return [];
  if (studyKind === "frame_2d") return frameModel.materials ?? [];
  if (studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d") return planeModel.materials ?? [];
  return [];
}

export function useWorkbenchVisualDerived({
  studyKind,
  isAxial,
  isBeam,
  isFrameLike,
  isHeatBar,
  isHeatPlane,
  isHeatPlaneTriangle,
  isSpring,
  isSpring3d,
  isThermalBar,
  isThermalTruss2d,
  isThermalTruss3d,
  isTorsion,
  isTruss,
  isTruss3d,
  activeResultWindow,
  activeResultWindowLimitBase,
  resultWindowMaxTotal,
  resultWindowOffset,
  directMeshEndpointsText,
  immersiveViewport,
  immersiveToolDrawerOpen,
  immersiveHelpDrawerOpen,
  showShortcutHints,
  activeLineResultField,
  frameResultFieldMax,
  currentMaterials,
  materialColorMap,
  displayTrussElements,
  displayTruss3dElements,
  activePlaneInputModel,
  planeModel,
  beamModel,
  frameModel,
  thermalBeamModel,
  thermalFrameModel,
  thermalTrussModel,
  thermalTruss3dModel,
  trussModel,
  truss3dModel,
  spring3dModel,
  activeSpringModel,
  activeBeamLikeModel,
  torsionModel,
  activeFrameLikeModel,
  activeFrameLikeResult,
  activeBeamLikeResult,
  torsionResult,
  axialResult,
  trussResult,
  thermalTrussResult,
  thermalTruss3dResult,
  truss3dResult,
  springResult,
  spring2dResult,
  spring3dResult,
  activeSpringResult,
  beamResult,
  frameResult,
  planeResult,
  heatBarResult,
  thermalBarResult,
  thermalBeamResult,
  thermalFrameResult,
  axialNodes,
  t,
}: WorkbenchVisualDerivedArgs) {
  const trussElementColors = useMemo(
    () =>
      displayTrussElements.map((element) =>
        (isFrameLike && activeFrameLikeResult) || (isBeam && activeBeamLikeResult) || (isTorsion && torsionResult)
          ? planeStressFill(lineResultFieldValue(element, activeLineResultField), frameResultFieldMax)
          : materialColorMap.get(element.material_id ?? "") ?? "#1677a3",
      ),
    [
      activeBeamLikeResult,
      activeFrameLikeResult,
      activeLineResultField,
      displayTrussElements,
      frameResultFieldMax,
      isBeam,
      isFrameLike,
      isTorsion,
      materialColorMap,
      torsionResult,
    ],
  );

  const truss3dElementColors = useMemo(
    () => displayTruss3dElements.map((element) => materialColorMap.get(element.material_id ?? "") ?? "#1677a3"),
    [displayTruss3dElements, materialColorMap],
  );

  const planeElementColors = useMemo(
    () =>
      (isHeatPlane ? activePlaneInputModel.elements : planeModel.elements).map((element) =>
        materialColorMap.get(("material_id" in element ? element.material_id : "") ?? "") ?? planeStressFill(0, 1),
      ),
    [activePlaneInputModel.elements, isHeatPlane, materialColorMap, planeModel.elements],
  );

  const nodeCount =
    isAxial
      ? axialNodes.length
      : activeResultWindow?.totalNodes ??
        (isTruss
          ? trussResult?.nodes.length
          : isSpring3d
            ? spring3dResult?.nodes.length
            : isTruss3d
              ? truss3dResult?.nodes.length
              : isSpring
                ? activeSpringResult?.nodes.length
                : isBeam
                  ? activeBeamLikeResult?.nodes.length
                  : isTorsion
                    ? torsionResult?.nodes?.length
                    : isFrameLike
                      ? activeFrameLikeResult?.nodes?.length
                      : planeResult?.nodes.length) ??
        (isTruss
          ? trussModel.nodes.length
          : isSpring3d
            ? spring3dModel.nodes.length
            : isTruss3d
              ? truss3dModel.nodes.length
              : isSpring
                ? activeSpringModel.nodes.length
                : isBeam
                  ? activeBeamLikeModel.nodes.length
                  : isTorsion
                    ? torsionModel.nodes.length
                    : isFrameLike
                      ? activeFrameLikeModel.nodes.length
                      : activePlaneInputModel.nodes.length);

  const activeResultWindowLimit = activeResultWindow?.limit ?? activeResultWindowLimitBase;
  const resultWindowStart = activeResultWindow ? Math.min(resultWindowOffset, Math.max(0, resultWindowMaxTotal - 1)) + 1 : 0;
  const resultWindowEnd = activeResultWindow ? Math.min(resultWindowOffset + activeResultWindowLimit, resultWindowMaxTotal) : 0;
  const resultWindowJumps = activeResultWindow
    ? [
        { label: t.jumpStart, offset: 0 },
        { label: t.jumpQuarter, offset: clampChunkOffset(resultWindowMaxTotal * 0.25, resultWindowMaxTotal, activeResultWindowLimit) },
        { label: t.jumpMid, offset: clampChunkOffset(resultWindowMaxTotal * 0.5, resultWindowMaxTotal, activeResultWindowLimit) },
        { label: t.jumpThreeQuarter, offset: clampChunkOffset(resultWindowMaxTotal * 0.75, resultWindowMaxTotal, activeResultWindowLimit) },
        { label: t.jumpEnd, offset: clampChunkOffset(Math.max(0, resultWindowMaxTotal - activeResultWindowLimit), resultWindowMaxTotal, activeResultWindowLimit) },
      ].filter((jump, index, jumps) => jumps.findIndex((candidate) => candidate.offset === jump.offset) === index)
    : [];

  const hasViewportDock = isTruss3d && ((immersiveViewport && immersiveToolDrawerOpen) || (showShortcutHints && immersiveHelpDrawerOpen));
  const showViewportToolStrip = isTruss3d && immersiveToolDrawerOpen;
  const shouldStretchSpaceViewport = isTruss3d && !hasViewportDock && !activeResultWindow;
  const viewportPixelWidth = activeResultWindow
    ? Math.min(3200, 980 + Math.ceil(resultWindowMaxTotal / activeResultWindowLimit) * 180)
    : isTruss3d
      ? hasViewportDock
        ? 1120
        : undefined
      : 980;

  const directMeshEndpoints = useMemo(
    () => parseDirectMeshEndpoints(directMeshEndpointsText),
    [directMeshEndpointsText],
  );

  const hasAnyResult = Boolean(
    axialResult ||
      heatBarResult ||
      thermalBarResult ||
      thermalBeamResult ||
      thermalFrameResult ||
      thermalTrussResult ||
      thermalTruss3dResult ||
      trussResult ||
      truss3dResult ||
      springResult ||
      spring2dResult ||
      spring3dResult ||
      beamResult ||
      torsionResult ||
      frameResult ||
      planeResult,
  );

  return {
    trussElementColors,
    truss3dElementColors,
    planeElementColors,
    nodeCount,
    activeResultWindowLimit,
    resultWindowStart,
    resultWindowEnd,
    resultWindowJumps,
    hasViewportDock,
    showViewportToolStrip,
    shouldStretchSpaceViewport,
    viewportPixelWidth,
    directMeshEndpoints,
    hasAnyResult,
  };
}
