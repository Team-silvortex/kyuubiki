"use client";

import { WorkbenchViewport } from "./workbench-viewport";
import type { WorkbenchCopy } from "./workbench-copy";
import type { DisplayTruss3dElement, DisplayTruss3dNode, DisplayTrussElement, DisplayTrussNode } from "./workbench-defaults";
import type { ViewportRenderDiagnostics, ViewportRenderStrategy } from "./workbench-render-diagnostics";
import type { BeamResultField } from "./workbench-types";
import type { PlaneElement, PlaneNode, PlaneResultField } from "./workbench-viewport-core";
import type { LineResultField } from "./workbench-viewport-core";

type WorkbenchViewportMountProps = {
  t: WorkbenchCopy;
  studyKind: string;
  sidebarSection: "study" | "model" | "workflow" | "library" | "system";
  isSpring2d: boolean;
  isSpring1d: boolean;
  isHeatBar: boolean;
  isThermalBar: boolean;
  isThermalBeam: boolean;
  isThermalFrame: boolean;
  isThermalTruss2d: boolean;
  isTorsion: boolean;
  isFrame: boolean;
  isFrameLike: boolean;
  isBeam: boolean;
  isSpring: boolean;
  isSpring3d: boolean;
  isThermalTruss3d: boolean;
  isHeatPlane: boolean;
  isHeatPlaneTriangle: boolean;
  isPlane: boolean;
  isTruss: boolean;
  isTruss3d: boolean;
  isThermal: boolean;
  hiddenMaterialIds: string[];
  frameLegendText?: string;
  truss3dLegendText?: string;
  planeLegendText: string;
  axialNodes: Array<{ x: number; displacement: number }>;
  axialLength: number;
  axialScale: number;
  displayTrussNodes: DisplayTrussNode[];
  displayTrussElements: DisplayTrussElement[];
  displayTruss3dNodes: DisplayTruss3dNode[];
  displayTruss3dElements: DisplayTruss3dElement[];
  planeNodes: PlaneNode[];
  planeElements: PlaneElement[];
  trussElementColors: string[];
  truss3dElementColors: string[];
  planeElementColors: string[];
  trussBounds: { minX: number; minY: number; maxX: number; maxY: number; width: number; height: number };
  planeBounds: { minX: number; minY: number; maxX: number; maxY: number; width: number; height: number };
  trussResult: unknown;
  truss3dResult: unknown;
  thermalTrussResult: unknown;
  activeFrameLikeResult: unknown;
  activeBeamLikeResult: unknown;
  torsionResult: unknown;
  heatBarResult: unknown;
  thermalBarResult: unknown;
  thermalTruss3dResult: unknown;
  springResult: unknown;
  spring2dResult: unknown;
  spring3dResult: unknown;
  heatPlaneQuadResult: unknown;
  heatPlaneTriangleResult: unknown;
  planeResult: unknown;
  activeLineResultField: LineResultField;
  frameResultFieldMax: number;
  focusedFrameElement: number | null;
  trussStability: { hotspotNodes: number[] } | null;
  trussDiagnostics: { nodeIssues: Record<number, string[]> } | null;
  selectedNode: number | null;
  selectedElement: number | null;
  memberDraftNodes: number[];
  stopDraggingNode: () => void;
  handleTrussPointerMove: React.ComponentProps<typeof WorkbenchViewport>["onTrussPointerMove"];
  setSelectedElement: (value: number | null) => void;
  setSelectedNode: (value: number | null) => void;
  setMemberDraftNodes: (value: number[]) => void;
  setFocusedFrameElement: (value: number | null) => void;
  startTrussNodeDrag: (index: number) => void;
  hiddenPlaneMaterialIds: string[];
  planeResultField: PlaneResultField;
  planeResultFieldMax: number;
  selectedPlaneNodeData: { id?: string } | null;
  focusedPlaneElement: number | null;
  handleTruss3dNodePick: (index: number) => void;
  setSelectedTruss3dNodes: (value: number[]) => void;
  updateTruss3dNodePosition: React.ComponentProps<typeof WorkbenchViewport>["onUpdateTruss3dNodePosition"];
  recordHistory: (label: string) => void;
  drag3dHistoryCapturedRef: React.RefObject<boolean>;
  truss3dLinkMode: boolean;
  immersiveViewport: boolean;
  truss3dProjectionMode: React.ComponentProps<typeof WorkbenchViewport>["projectionMode"];
  truss3dShowGrid: boolean;
  truss3dShowLabels: boolean;
  truss3dShowNodes: boolean;
  truss3dBoxSelectMode: boolean;
  truss3dViewPreset: React.ComponentProps<typeof WorkbenchViewport>["activeViewPreset"];
  truss3dFocusRequestVersion: number;
  truss3dResetRequestVersion: number;
  selectedTruss3dNodes: number[];
  showShortcutHints: boolean;
  viewportPixelWidth?: number;
  handleTruss3dNodesBoxSelect: (indices: number[], append: boolean) => void;
  handleTruss3dProjectionModeChange: React.ComponentProps<typeof WorkbenchViewport>["onProjectionModeChange"];
  handleTruss3dShowGridChange: React.ComponentProps<typeof WorkbenchViewport>["onShowGridChange"];
  handleTruss3dShowLabelsChange: React.ComponentProps<typeof WorkbenchViewport>["onShowLabelsChange"];
  handleTruss3dShowNodesChange: React.ComponentProps<typeof WorkbenchViewport>["onShowNodesChange"];
  handleTruss3dBoxSelectModeChange: React.ComponentProps<typeof WorkbenchViewport>["onBoxSelectModeChange"];
  onRenderDiagnosticsChange?: (diagnostics: ViewportRenderDiagnostics) => void;
  renderStrategy?: ViewportRenderStrategy;
};

export function WorkbenchViewportMount({
  t,
  studyKind,
  sidebarSection,
  isSpring2d,
  isSpring1d,
  isHeatBar,
  isThermalBar,
  isThermalBeam,
  isThermalFrame,
  isThermalTruss2d,
  isTorsion,
  isFrame,
  isFrameLike,
  isBeam,
  isSpring,
  isSpring3d,
  isThermalTruss3d,
  isHeatPlane,
  isHeatPlaneTriangle,
  isPlane,
  isTruss,
  isTruss3d,
  isThermal,
  hiddenMaterialIds,
  frameLegendText,
  truss3dLegendText,
  planeLegendText,
  axialNodes,
  axialLength,
  axialScale,
  displayTrussNodes,
  displayTrussElements,
  displayTruss3dNodes,
  displayTruss3dElements,
  planeNodes,
  planeElements,
  trussElementColors,
  truss3dElementColors,
  planeElementColors,
  trussBounds,
  planeBounds,
  trussResult,
  truss3dResult,
  thermalTrussResult,
  activeFrameLikeResult,
  activeBeamLikeResult,
  torsionResult,
  heatBarResult,
  thermalBarResult,
  thermalTruss3dResult,
  springResult,
  spring2dResult,
  spring3dResult,
  heatPlaneQuadResult,
  heatPlaneTriangleResult,
  planeResult,
  activeLineResultField,
  frameResultFieldMax,
  focusedFrameElement,
  trussStability,
  trussDiagnostics,
  selectedNode,
  selectedElement,
  memberDraftNodes,
  stopDraggingNode,
  handleTrussPointerMove,
  setSelectedElement,
  setSelectedNode,
  setMemberDraftNodes,
  setFocusedFrameElement,
  startTrussNodeDrag,
  hiddenPlaneMaterialIds,
  planeResultField,
  planeResultFieldMax,
  selectedPlaneNodeData,
  focusedPlaneElement,
  handleTruss3dNodePick,
  setSelectedTruss3dNodes,
  updateTruss3dNodePosition,
  recordHistory,
  drag3dHistoryCapturedRef,
  truss3dLinkMode,
  immersiveViewport,
  truss3dProjectionMode,
  truss3dShowGrid,
  truss3dShowLabels,
  truss3dShowNodes,
  truss3dBoxSelectMode,
  truss3dViewPreset,
  truss3dFocusRequestVersion,
  truss3dResetRequestVersion,
  selectedTruss3dNodes,
  showShortcutHints,
  viewportPixelWidth,
  handleTruss3dNodesBoxSelect,
  handleTruss3dProjectionModeChange,
  handleTruss3dShowGridChange,
  handleTruss3dShowLabelsChange,
  handleTruss3dShowNodesChange,
  handleTruss3dBoxSelectModeChange,
  onRenderDiagnosticsChange,
  renderStrategy,
}: WorkbenchViewportMountProps) {
  return (
    <WorkbenchViewport
      studyKind={studyKind as React.ComponentProps<typeof WorkbenchViewport>["studyKind"]}
      sidebarSection={sidebarSection}
      title={t.sections.model}
      axialTitle={t.kinds.axial_bar_1d}
      trussTitle={
        t.kinds[
          isSpring2d
            ? "spring_2d"
            : isSpring1d
              ? "spring_1d"
              : isHeatBar
                ? "heat_bar_1d"
                : isThermalBar
                  ? "thermal_bar_1d"
                  : isThermalBeam
                    ? "thermal_beam_1d"
                    : isThermalFrame
                      ? "thermal_frame_2d"
                      : isThermalTruss2d
                        ? "thermal_truss_2d"
                        : studyKind === "beam_1d"
                          ? "beam_1d"
                          : isTorsion
                            ? "torsion_1d"
                            : isFrame
                              ? "frame_2d"
                              : "truss_2d"
        ]
      }
      trussLegend={
        (isFrameLike && activeFrameLikeResult) ||
        (isBeam && activeBeamLikeResult) ||
        (isTorsion && torsionResult) ||
        (isHeatBar && heatBarResult) ||
        (isThermalBar && thermalBarResult) ||
        (isThermalTruss2d && thermalTrussResult) ||
        (isThermalTruss3d && thermalTruss3dResult) ||
        (isSpring1d && springResult) ||
        (isSpring2d && spring2dResult) ||
        (isSpring3d && spring3dResult)
          ? frameLegendText
          : undefined
      }
      truss3dLegend={
        Boolean(truss3dResult) || Boolean(thermalTruss3dResult) || Boolean(spring3dResult)
          ? truss3dLegendText
          : undefined
      }
      truss3dTitle={t.kinds[isSpring3d ? "spring_3d" : isThermalTruss3d ? "thermal_truss_3d" : "truss_3d"]}
      planeTitle={
        t.kinds[
          studyKind === "heat_plane_quad_2d"
            ? "heat_plane_quad_2d"
            : studyKind === "heat_plane_triangle_2d"
              ? "heat_plane_triangle_2d"
              : studyKind === "thermal_plane_quad_2d"
                ? "thermal_plane_quad_2d"
                : studyKind === "plane_quad_2d"
                  ? "plane_quad_2d"
                  : studyKind === "thermal_plane_triangle_2d"
                    ? "thermal_plane_triangle_2d"
                    : "plane_triangle_2d"
        ]
      }
      planeLegend={planeLegendText}
      axialNodes={axialNodes}
      axialLength={axialLength}
      axialScale={axialScale}
      displayTrussNodes={displayTrussNodes}
      displayTrussElements={displayTrussElements}
      trussElementColors={trussElementColors}
      hiddenTrussMaterialIds={isTruss || isFrameLike || isBeam || isSpring || isThermal ? hiddenMaterialIds : []}
      trussBounds={trussBounds}
      trussResult={Boolean(trussResult || thermalTrussResult || activeFrameLikeResult || activeBeamLikeResult || torsionResult || heatBarResult || thermalBarResult || thermalTruss3dResult || springResult || spring2dResult || spring3dResult)}
      frameResultField={activeLineResultField}
      frameResultFieldMax={frameResultFieldMax}
      focusedFrameElement={focusedFrameElement}
      trussHotspotNodes={trussStability?.hotspotNodes ?? []}
      trussNodeIssues={trussDiagnostics?.nodeIssues ?? {}}
      selectedNode={selectedNode}
      selectedElement={selectedElement}
      memberDraftNodes={memberDraftNodes}
      onTrussPointerMove={handleTrussPointerMove}
      onStopDraggingNode={stopDraggingNode}
      onSelectTrussElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        setMemberDraftNodes([]);
        if (isBeam || isTorsion || isHeatBar || isThermal) setFocusedFrameElement(index);
      }}
      onStartTrussNodeDrag={(index) => {
        startTrussNodeDrag(index);
      }}
      displayTruss3dNodes={displayTruss3dNodes}
      displayTruss3dElements={displayTruss3dElements}
      truss3dElementColors={truss3dElementColors}
      hiddenTruss3dMaterialIds={isTruss3d ? hiddenMaterialIds : []}
      planeNodes={planeNodes}
      planeElements={planeElements}
      planeElementColors={planeElementColors}
      hiddenPlaneMaterialIds={isPlane ? hiddenPlaneMaterialIds : []}
      planeBounds={planeBounds}
      planeResult={Boolean(isHeatPlane ? (isHeatPlaneTriangle ? heatPlaneTriangleResult : heatPlaneQuadResult) : planeResult)}
      planeResultField={planeResultField}
      planeResultFieldMax={planeResultFieldMax}
      selectedPlaneNodeId={selectedPlaneNodeData?.id ?? null}
      focusedPlaneElement={focusedPlaneElement}
      onSelectPlaneElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
      }}
      onSelectPlaneNode={(index) => {
        setSelectedNode(index);
        setSelectedElement(null);
      }}
      selectedTruss3dNode={selectedNode}
      selectedTruss3dElement={selectedElement}
      onSelectTruss3dNode={(index) => {
        handleTruss3dNodePick(index);
      }}
      onSelectTruss3dElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        setSelectedTruss3dNodes([]);
        setMemberDraftNodes([]);
      }}
      onUpdateTruss3dNodePosition={updateTruss3dNodePosition}
      onBeginTruss3dNodeDrag={() => {
        if (!drag3dHistoryCapturedRef.current) {
          recordHistory(t.dragNodeAction);
          drag3dHistoryCapturedRef.current = true;
        }
      }}
      onEndTruss3dNodeDrag={() => {
        drag3dHistoryCapturedRef.current = false;
      }}
      workspaceBadge={isTruss3d ? t.spaceStudio : t.sections.model}
      truss3dLinkMode={truss3dLinkMode}
      immersiveViewport={immersiveViewport}
      projectionMode={truss3dProjectionMode}
      showGrid={truss3dShowGrid}
      showLabels={truss3dShowLabels}
      showNodes={truss3dShowNodes}
      boxSelectMode={truss3dBoxSelectMode}
      activeViewPreset={truss3dViewPreset}
      focusRequestVersion={truss3dFocusRequestVersion}
      resetRequestVersion={truss3dResetRequestVersion}
      selectedTruss3dNodeIndices={selectedTruss3dNodes}
      onSelectTruss3dNodes={handleTruss3dNodesBoxSelect}
      showShortcutHints={showShortcutHints}
      shortcutLegendTitle={t.shortcutLegendTitle}
      shortcutLegendRows={[...t.shortcutLegendRows]}
      onProjectionModeChange={handleTruss3dProjectionModeChange}
      onShowGridChange={handleTruss3dShowGridChange}
      onShowLabelsChange={handleTruss3dShowLabelsChange}
      onShowNodesChange={handleTruss3dShowNodesChange}
      onBoxSelectModeChange={handleTruss3dBoxSelectModeChange}
      viewportPixelWidth={viewportPixelWidth}
      onRenderDiagnosticsChange={onRenderDiagnosticsChange}
      renderStrategy={renderStrategy}
    />
  );
}
