"use client";

import { scientific, formatTime } from "@/lib/workbench/helpers";
import { WorkbenchInspector } from "./workbench-inspector";
import type { WorkbenchInspectorProps } from "./workbench-inspector-types";
import type { ModelPanelTab } from "./workbench-types";

type ElementWithMaterial = { area?: number; youngs_modulus?: number; material_id?: string };
type Element3dWithMaterial = ElementWithMaterial & { youngs_modulus?: number };
type PlaneElementInput = { thickness?: number; youngs_modulus?: number; poisson_ratio?: number; material_id?: string };
type LineElementInput = { material_id?: string };
type TrussDiagnosticsLike = { suggestions: Array<{ id: string }> } | null;

type InspectorMountProps = {
  t: WorkbenchInspectorProps["t"];
  reportScopeLabel?: string;
  reportScopeHint?: string;
  sidebarSection: WorkbenchInspectorProps["sidebarSection"];
  studyKind: WorkbenchInspectorProps["studyKind"];
  language: string;
  isPending: boolean;
  isFrameLike: boolean;
  isThermalFrame: boolean;
  isBeam: boolean;
  isTorsion: boolean;
  isHeatBar: boolean;
  isThermalBar: boolean;
  isThermalTruss2d: boolean;
  isThermalTruss3d: boolean;
  isSpring: boolean;
  isSpring3d: boolean;
  isHeatPlane: boolean;
  isHeatPlaneTriangle: boolean;
  isThermalPlaneTriangle: boolean;
  isThermalPlaneQuad: boolean;
  canProjectHeatToThermo: boolean;
  currentStudyFamilyLabel?: string;
  currentStudyFamilyHint?: string;
  selectedNode: number | null;
  selectedNodeData: WorkbenchInspectorProps["selectedNodeData"];
  selectedElementData: WorkbenchInspectorProps["selectedElementData"];
  selectedTruss3dNodeData: WorkbenchInspectorProps["selectedTruss3dNodeData"];
  selectedTruss3dElementData: WorkbenchInspectorProps["selectedTruss3dElementData"];
  selectedPlaneNodeData: WorkbenchInspectorProps["selectedPlaneNodeData"];
  selectedPlaneElementData: WorkbenchInspectorProps["selectedPlaneElementData"];
  selectedFrameNodeData: WorkbenchInspectorProps["selectedFrameNodeData"];
  selectedFrameElementData: WorkbenchInspectorProps["selectedFrameElementData"];
  selectedBeamNodeData: WorkbenchInspectorProps["selectedFrameNodeData"];
  selectedBeamElementData: WorkbenchInspectorProps["selectedFrameElementData"];
  selectedTorsionNodeData: WorkbenchInspectorProps["selectedFrameNodeData"];
  selectedTorsionElementData: WorkbenchInspectorProps["selectedFrameElementData"];
  selectedThermalNodeData: WorkbenchInspectorProps["selectedFrameNodeData"];
  selectedThermalElementData: WorkbenchInspectorProps["selectedFrameElementData"];
  selectedSpringElementData: WorkbenchInspectorProps["selectedFrameElementData"];
  selectedNodeIssues: unknown[];
  materialOptions: WorkbenchInspectorProps["materialOptions"];
  materialLabel: string;
  trussDiagnostics: WorkbenchInspectorProps["trussDiagnostics"] & TrussDiagnosticsLike;
  trussStability: WorkbenchInspectorProps["trussStability"];
  undoStack: WorkbenchInspectorProps["undoStack"];
  redoStack: WorkbenchInspectorProps["redoStack"];
  job: WorkbenchInspectorProps["job"];
  nodeCount: number;
  tipDisplacement: string;
  maxStressValue: string;
  frameMaxAxialForce?: number;
  frameMaxShearForce?: number;
  reactionValue: string;
  frameMaxRotationValue?: string;
  thermalFrameMaxTemperatureDelta?: number;
  thermalFrameMaxTemperatureGradient?: number;
  thermalBeamMaxTemperatureGradient?: number;
  thermalPlaneMaxTemperatureDelta?: number;
  planeHotspotFieldLabel?: string;
  planeHotspotElements: WorkbenchInspectorProps["planeHotspotElements"];
  planeThermalRows: WorkbenchInspectorProps["planeThermalRows"];
  frameHotspotFieldLabel?: string;
  frameHotspotElements: WorkbenchInspectorProps["frameHotspotElements"];
  frameForceRows: WorkbenchInspectorProps["frameForceRows"];
  planeHotspotLimit: number;
  heartbeatStatusValue: string;
  heartbeatTone: WorkbenchInspectorProps["heartbeatTone"];
  translatedFailureReason?: string | null;
  jobIsActive: boolean;
  activeFrameLikeResult?: { max_rotation?: number } | null;
  activeBeamLikeResult?: { max_rotation?: number } | null;
  torsionResult?: { max_rotation?: number } | null;
  activePlaneInputModel: { elements: PlaneElementInput[] };
  planeModel: { elements: PlaneElementInput[] };
  activeFrameLikeModel: { elements: LineElementInput[] };
  activeBeamLikeModel: { elements: LineElementInput[] };
  trussModel: { nodes: Array<{ id?: string }>; elements: ElementWithMaterial[] };
  thermalTrussModel: { elements: ElementWithMaterial[] };
  truss3dModel: { nodes: Array<Record<string, unknown>>; elements: Element3dWithMaterial[] };
  thermalTruss3dModel: { nodes: Array<Record<string, unknown>>; elements: Element3dWithMaterial[] };
  spring3dModel: { nodes: Array<Record<string, unknown>> };
  onUpdateSelectedNode: WorkbenchInspectorProps["onUpdateSelectedNode"];
  onUpdateSelectedElement: WorkbenchInspectorProps["onUpdateSelectedElement"];
  onAssignSelectedElementMaterial: WorkbenchInspectorProps["onAssignSelectedElementMaterial"];
  onUpdateSelectedTruss3dNode: WorkbenchInspectorProps["onUpdateSelectedTruss3dNode"];
  onUpdateSelectedTruss3dElement: WorkbenchInspectorProps["onUpdateSelectedTruss3dElement"];
  onAssignSelectedTruss3dElementMaterial: WorkbenchInspectorProps["onAssignSelectedTruss3dElementMaterial"];
  onUpdateSelectedPlaneNode: WorkbenchInspectorProps["onUpdateSelectedPlaneNode"];
  onUpdateSelectedPlaneElement: WorkbenchInspectorProps["onUpdateSelectedPlaneElement"];
  onAssignSelectedPlaneElementMaterial: WorkbenchInspectorProps["onAssignSelectedPlaneElementMaterial"];
  onUpdateSelectedFrameNode: WorkbenchInspectorProps["onUpdateSelectedFrameNode"];
  onUpdateSelectedFrameElement: WorkbenchInspectorProps["onUpdateSelectedFrameElement"];
  onAssignSelectedFrameElementMaterial: WorkbenchInspectorProps["onAssignSelectedFrameElementMaterial"];
  applyTrussSuggestion: (suggestionId: string) => void;
  handleUndo: () => void;
  handleRedo: () => void;
  cancelCurrentJob: () => void;
  downloadResultJson: () => void;
  downloadResultCsv: () => void;
  projectHeatToThermoLabel: string;
  projectHeatToThermoStudy: () => void;
  downloadPlaneHotspotSummary: () => void;
  downloadFrameHotspotSummary: () => void;
  downloadFrameForceSummary: () => void;
  setSidebarSection: (section: "model") => void;
  setModelTab: (tab: ModelPanelTab) => void;
  setSelectedElement: (value: number | null) => void;
  setSelectedNode: (value: number | null) => void;
  setFocusedPlaneElement: (value: number | null) => void;
  setFocusedFrameElement: (value: number | null) => void;
  setPlaneHotspotLimit: (limit: number) => void;
};

export function WorkbenchInspectorMount(props: InspectorMountProps) {
  const {
    t,
    reportScopeLabel,
    reportScopeHint,
    sidebarSection,
    studyKind,
    language,
    isPending,
    isFrameLike,
    isThermalFrame,
    isBeam,
    isTorsion,
    isHeatBar,
    isThermalBar,
    isThermalTruss2d,
    isThermalTruss3d,
    isSpring,
    isSpring3d,
    isHeatPlane,
    isHeatPlaneTriangle,
    isThermalPlaneTriangle,
    isThermalPlaneQuad,
    canProjectHeatToThermo,
    selectedNode,
    selectedNodeData,
    selectedElementData,
    selectedTruss3dNodeData,
    selectedTruss3dElementData,
    selectedPlaneNodeData,
    selectedPlaneElementData,
    selectedFrameNodeData,
    selectedFrameElementData,
    selectedBeamNodeData,
    selectedBeamElementData,
    selectedTorsionNodeData,
    selectedTorsionElementData,
    selectedThermalNodeData,
    selectedThermalElementData,
    selectedSpringElementData,
    selectedNodeIssues,
    materialOptions,
    materialLabel,
    trussDiagnostics,
    trussStability,
    undoStack,
    redoStack,
    job,
    nodeCount,
    tipDisplacement,
    maxStressValue,
    frameMaxAxialForce,
    frameMaxShearForce,
    reactionValue,
    frameMaxRotationValue,
    thermalFrameMaxTemperatureDelta,
    thermalFrameMaxTemperatureGradient,
    thermalBeamMaxTemperatureGradient,
    thermalPlaneMaxTemperatureDelta,
    planeHotspotFieldLabel,
    planeHotspotElements,
    planeThermalRows,
    frameHotspotFieldLabel,
    frameHotspotElements,
    frameForceRows,
    planeHotspotLimit,
    heartbeatStatusValue,
    heartbeatTone,
    translatedFailureReason,
    jobIsActive,
    activeFrameLikeResult,
    activeBeamLikeResult,
    torsionResult,
    activePlaneInputModel,
    planeModel,
    activeFrameLikeModel,
    activeBeamLikeModel,
    trussModel,
    thermalTrussModel,
    truss3dModel,
    thermalTruss3dModel,
    spring3dModel,
    onUpdateSelectedNode,
    onUpdateSelectedElement,
    onAssignSelectedElementMaterial,
    onUpdateSelectedTruss3dNode,
    onUpdateSelectedTruss3dElement,
    onAssignSelectedTruss3dElementMaterial,
    onUpdateSelectedPlaneNode,
    onUpdateSelectedPlaneElement,
    onAssignSelectedPlaneElementMaterial,
    onUpdateSelectedFrameNode,
    onUpdateSelectedFrameElement,
    onAssignSelectedFrameElementMaterial,
    applyTrussSuggestion,
    handleUndo,
    handleRedo,
    cancelCurrentJob,
    downloadResultJson,
    downloadResultCsv,
    projectHeatToThermoLabel,
    projectHeatToThermoStudy,
    downloadPlaneHotspotSummary,
    downloadFrameHotspotSummary,
    downloadFrameForceSummary,
    setSidebarSection,
    setModelTab,
    setSelectedElement,
    setSelectedNode,
    setFocusedPlaneElement,
    setFocusedFrameElement,
    setPlaneHotspotLimit,
  } = props;

  return (
    <WorkbenchInspector
      t={t}
      reportScopeLabel={reportScopeLabel}
      reportScopeHint={reportScopeHint}
      sidebarSection={sidebarSection}
      studyKind={studyKind}
      isPending={isPending}
      selectedNodeData={selectedNodeData ? { ...selectedNodeData } : null}
      selectedElementData={selectedElementData ? { ...selectedElementData } : null}
      selectedTruss3dNodeData={selectedTruss3dNodeData && selectedNode !== null ? { ...selectedTruss3dNodeData, ...(isSpring3d ? spring3dModel.nodes[selectedNode] : isThermalTruss3d ? thermalTruss3dModel.nodes[selectedNode] : truss3dModel.nodes[selectedNode]) } : null}
      selectedTruss3dElementData={selectedTruss3dElementData ? { ...selectedTruss3dElementData } : null}
      selectedPlaneNodeData={selectedPlaneNodeData ? { ...selectedPlaneNodeData } : null}
      selectedPlaneElementData={selectedPlaneElementData ? { ...selectedPlaneElementData } : null}
      selectedFrameNodeData={(isFrameLike ? selectedFrameNodeData : isBeam ? selectedBeamNodeData : isTorsion ? selectedTorsionNodeData : isHeatBar || isThermalBar ? selectedThermalNodeData : null) ? { ...(isFrameLike ? selectedFrameNodeData : isBeam ? selectedBeamNodeData : isTorsion ? selectedTorsionNodeData : selectedThermalNodeData)! } : null}
      selectedFrameElementData={(isFrameLike ? selectedFrameElementData : isBeam ? selectedBeamElementData : isTorsion ? selectedTorsionElementData : isHeatBar || isThermalBar ? selectedThermalElementData : isSpring ? selectedSpringElementData : null) ? { ...(isFrameLike ? selectedFrameElementData : isBeam ? selectedBeamElementData : isTorsion ? selectedTorsionElementData : isHeatBar || isThermalBar ? selectedThermalElementData : selectedSpringElementData)! } : null}
      trussElementArea={selectedElementData ? (isThermalTruss2d ? thermalTrussModel.elements[selectedElementData.index]?.area : trussModel.elements[selectedElementData.index]?.area) ?? 0 : 0}
      trussElementModulusGpa={selectedElementData ? Math.round((((isThermalTruss2d ? thermalTrussModel.elements[selectedElementData.index]?.youngs_modulus : trussModel.elements[selectedElementData.index]?.youngs_modulus) ?? 0) / 1.0e9)) : 0}
      trussElementMaterialId={selectedElementData ? (isThermalTruss2d ? thermalTrussModel.elements[selectedElementData.index]?.material_id : trussModel.elements[selectedElementData.index]?.material_id) ?? materialOptions[0]?.id ?? "" : ""}
      truss3dElementArea={selectedTruss3dElementData ? (studyKind === "thermal_truss_3d" ? thermalTruss3dModel.elements[selectedTruss3dElementData.index]?.area : studyKind === "truss_3d" ? truss3dModel.elements[selectedTruss3dElementData.index]?.area : undefined) ?? 0 : 0}
      truss3dElementModulusGpa={selectedTruss3dElementData ? Math.round(((((studyKind === "thermal_truss_3d" ? thermalTruss3dModel.elements[selectedTruss3dElementData.index]?.youngs_modulus : studyKind === "truss_3d" ? truss3dModel.elements[selectedTruss3dElementData.index]?.youngs_modulus : undefined) ?? 0)) / 1.0e9)) : 0}
      truss3dElementMaterialId={selectedTruss3dElementData ? ((studyKind === "thermal_truss_3d" ? thermalTruss3dModel.elements[selectedTruss3dElementData.index]?.material_id : studyKind === "truss_3d" ? truss3dModel.elements[selectedTruss3dElementData.index]?.material_id : undefined) ?? materialOptions[0]?.id ?? "") : ""}
      planeElementThickness={selectedPlaneElementData ? activePlaneInputModel.elements[selectedPlaneElementData.index]?.thickness ?? 0 : 0}
      planeElementModulusGpa={selectedPlaneElementData && !isHeatPlane ? Math.round((((planeModel.elements[selectedPlaneElementData.index]?.youngs_modulus ?? 0) / 1.0e9))) : 0}
      planeElementPoissonRatio={selectedPlaneElementData && !isHeatPlane ? (planeModel.elements[selectedPlaneElementData.index]?.poisson_ratio ?? 0.33) : 0.33}
      planeElementMaterialId={selectedPlaneElementData && !isHeatPlane ? (planeModel.elements[selectedPlaneElementData.index]?.material_id ?? materialOptions[0]?.id ?? "") : ""}
      frameElementMaterialId={isFrameLike ? selectedFrameElementData ? activeFrameLikeModel.elements[selectedFrameElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : "" : isBeam ? selectedBeamElementData ? activeBeamLikeModel.elements[selectedBeamElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : "" : ""}
      materialOptions={materialOptions}
      materialLabel={materialLabel}
      onUpdateSelectedNode={onUpdateSelectedNode}
      onUpdateSelectedElement={onUpdateSelectedElement}
      onAssignSelectedElementMaterial={onAssignSelectedElementMaterial}
      onUpdateSelectedTruss3dNode={onUpdateSelectedTruss3dNode}
      onUpdateSelectedTruss3dElement={onUpdateSelectedTruss3dElement}
      onAssignSelectedTruss3dElementMaterial={onAssignSelectedTruss3dElementMaterial}
      onUpdateSelectedPlaneNode={onUpdateSelectedPlaneNode}
      onUpdateSelectedPlaneElement={onUpdateSelectedPlaneElement}
      onAssignSelectedPlaneElementMaterial={onAssignSelectedPlaneElementMaterial}
      onUpdateSelectedFrameNode={onUpdateSelectedFrameNode}
      onUpdateSelectedFrameElement={onUpdateSelectedFrameElement}
      onAssignSelectedFrameElementMaterial={onAssignSelectedFrameElementMaterial}
      trussDiagnostics={trussDiagnostics}
      trussStability={trussStability}
      hotspotNodeLabels={(trussStability?.hotspotNodes ?? []).map((nodeIndex) => trussModel.nodes[nodeIndex]?.id ?? nodeIndex).join(", ")}
      onApplyTrussSuggestion={(id) => {
        const suggestion = trussDiagnostics?.suggestions.find((entry) => entry.id === id);
        if (suggestion) applyTrussSuggestion(suggestion.id);
      }}
      undoStack={undoStack}
      redoStack={redoStack}
      onUndo={handleUndo}
      onRedo={handleRedo}
      job={job}
      nodeCount={nodeCount}
      tipDisplacement={tipDisplacement}
      maxStressValue={maxStressValue}
      frameMaxAxialForceValue={isFrameLike || isSpring || isHeatBar || isThermalBar || isThermalTruss2d || isThermalTruss3d ? scientific(frameMaxAxialForce) : undefined}
      frameMaxShearForceValue={isFrameLike || isBeam ? scientific(frameMaxShearForce) : undefined}
      reactionValue={reactionValue}
      frameMaxRotationValue={frameMaxRotationValue ?? (isFrameLike ? scientific(activeFrameLikeResult?.max_rotation) : isBeam ? scientific(activeBeamLikeResult?.max_rotation) : isTorsion ? scientific(torsionResult?.max_rotation) : undefined)}
      thermalPlaneMaxTemperatureDeltaValue={isThermalPlaneTriangle || isThermalPlaneQuad ? scientific(thermalPlaneMaxTemperatureDelta) : undefined}
      thermalFrameMaxTemperatureDeltaValue={isThermalFrame ? scientific(thermalFrameMaxTemperatureDelta) : undefined}
      thermalFrameMaxTemperatureGradientValue={isThermalFrame ? scientific(thermalFrameMaxTemperatureGradient) : undefined}
      thermalBeamMaxTemperatureGradientValue={isBeam ? scientific(thermalBeamMaxTemperatureGradient) : undefined}
      planeHotspotFieldLabel={isHeatPlane || isThermalPlaneTriangle || isThermalPlaneQuad || studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d" ? planeHotspotFieldLabel : undefined}
      planeHotspotElements={isHeatPlane || isThermalPlaneTriangle || isThermalPlaneQuad || studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d" ? planeHotspotElements : []}
      planeThermalRows={isHeatPlane ? planeThermalRows : []}
      frameHotspotFieldLabel={isFrameLike || isBeam || isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d || isHeatBar || isTorsion ? frameHotspotFieldLabel : undefined}
      frameHotspotElements={isFrameLike || isBeam || isTorsion || isSpring || isHeatBar || isThermalBar || isThermalTruss2d || isThermalTruss3d ? frameHotspotElements : []}
      frameForceRows={isFrameLike || isBeam || isTorsion || isSpring || isHeatBar || isThermalBar || isThermalTruss2d || isThermalTruss3d ? frameForceRows : []}
      planeHotspotLimit={planeHotspotLimit}
      createdAtValue={formatTime(job?.created_at, language)}
      updatedAtValue={formatTime(job?.updated_at, language)}
      heartbeatStatusValue={heartbeatStatusValue}
      heartbeatTone={heartbeatTone}
      failureReasonValue={translatedFailureReason ?? job?.message ?? "--"}
      canCancelJob={jobIsActive}
      onCancelJob={cancelCurrentJob}
      onDownloadJson={downloadResultJson}
      onDownloadCsv={downloadResultCsv}
      canProjectHeatToThermo={canProjectHeatToThermo}
      projectHeatToThermoLabel={projectHeatToThermoLabel}
      onProjectHeatToThermo={projectHeatToThermoStudy}
      onDownloadPlaneHotspots={downloadPlaneHotspotSummary}
      onDownloadFrameHotspots={downloadFrameHotspotSummary}
      onDownloadFrameForces={downloadFrameForceSummary}
      onSelectPlaneHotspot={(index) => {
        setSidebarSection("model");
        setModelTab("tree");
        setSelectedElement(index);
        setSelectedNode(null);
        setFocusedPlaneElement(index);
      }}
      onSelectFrameHotspot={(index) => {
        setSidebarSection("model");
        setModelTab("tree");
        setSelectedElement(index);
        setSelectedNode(null);
        setFocusedFrameElement(index);
      }}
      onPlaneHotspotLimitChange={setPlaneHotspotLimit}
    />
  );
}
