"use client";

import {
  buildWorkbenchScriptSnapshot,
  buildWorkbenchUiSnapshot,
  restoreWorkbenchUiSnapshot,
} from "@/components/workbench/workbench-script-orchestration";
import {
  buildThermalBarFromHeatResult,
  buildThermalPlaneQuadFromHeatResult,
  buildThermalPlaneTriangleFromHeatResult,
} from "@/components/workbench/workbench-model-transform-helpers";
import { pushHistoryEntry, stepHistory, type WorkbenchSnapshot } from "@/lib/workbench/history";

type SessionControllerDeps = {
  t: any;
  language: "en" | "zh" | "ja" | "es";
  studyKind: string;
  sidebarSection: string;
  studyTab: string;
  modelTab: string;
  libraryTab: string;
  systemPanelTab: string;
  systemDataTab: string;
  theme: string;
  frontendRuntimeMode: string;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  adminFilterProjectId: string;
  adminFilterModelVersionId: string;
  loadedModelName: string;
  activeMaterial: string;
  selectedNode: number | null;
  selectedElement: number | null;
  selectedTruss3dNodes: number[];
  memberDraftNodes: number[];
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  truss3dProjectionMode: string;
  truss3dViewPreset: string;
  truss3dBoxSelectMode: boolean;
  truss3dLinkMode: boolean;
  hasAnyResult: boolean;
  job: any;
  projects: any[];
  jobHistory: any[];
  resultRecords: any[];
  protocolAgents: any[];
  health: any;
  message: string;
  axialForm: any;
  heatBarModel: any;
  heatPlaneModel: any;
  thermalBarModel: any;
  thermalBeamModel: any;
  thermalFrameModel: any;
  thermalTrussModel: any;
  trussModel: any;
  thermalTruss3dModel: any;
  truss3dModel: any;
  planeModel: any;
  frameModel: any;
  beamModel: any;
  torsionModel: any;
  springModel: any;
  spring2dModel: any;
  spring3dModel: any;
  parametric: any;
  panelParametric: any;
  undoStack: any[];
  redoStack: any[];
  heatBarResult: any;
  heatPlaneTriangleResult: any;
  heatPlaneQuadResult: any;
  setUndoStack: (value: any) => void;
  setRedoStack: (value: any) => void;
  setMessage: (value: string) => void;
  setStudyKind: (value: any) => void;
  setAxialForm: (value: any) => void;
  setHeatBarModel: (value: any) => void;
  setHeatPlaneModel: (value: any) => void;
  setThermalBarModel: (value: any) => void;
  setThermalBeamModel: (value: any) => void;
  setThermalFrameModel: (value: any) => void;
  setThermalTrussModel: (value: any) => void;
  setTrussModel: (value: any) => void;
  setThermalTruss3dModel: (value: any) => void;
  setTruss3dModel: (value: any) => void;
  setPlaneModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setBeamModel: (value: any) => void;
  setTorsionModel: (value: any) => void;
  setSpringModel: (value: any) => void;
  setSpring2dModel: (value: any) => void;
  setSpring3dModel: (value: any) => void;
  setParametric: (value: any) => void;
  setPanelParametric: (value: any) => void;
  setActiveMaterial: (value: string) => void;
  setLoadedModelName: (value: string) => void;
  setSidebarSection: (value: any) => void;
  setSelectedNode: (value: number | null) => void;
  setSelectedElement: (value: number | null) => void;
  setMemberDraftNodes: (value: number[]) => void;
  setPlaneResultField: (value: any) => void;
  resetActiveResult: () => void;
  openWorkspaceStudy: (tab?: any) => void;
};

export function createWorkbenchSessionController(deps: SessionControllerDeps) {
  const buildScriptSnapshot = () =>
    buildWorkbenchScriptSnapshot({
      studyKind: deps.studyKind,
      sidebarSection: deps.sidebarSection,
      studyTab: deps.studyTab,
      modelTab: deps.modelTab,
      libraryTab: deps.libraryTab,
      systemPanelTab: deps.systemPanelTab,
      systemDataTab: deps.systemDataTab,
      language: deps.language,
      theme: deps.theme,
      frontendRuntimeMode: deps.frontendRuntimeMode,
      selectedProjectId: deps.selectedProjectId,
      selectedModelId: deps.selectedModelId,
      selectedVersionId: deps.selectedVersionId,
      selectedAdminJobId: deps.selectedAdminJobId,
      selectedAdminResultJobId: deps.selectedAdminResultJobId,
      adminFilterProjectId: deps.adminFilterProjectId,
      adminFilterModelVersionId: deps.adminFilterModelVersionId,
      loadedModelName: deps.loadedModelName,
      activeMaterial: deps.activeMaterial,
      selectedNode: deps.selectedNode,
      selectedElement: deps.selectedElement,
      selectedTruss3dNodes: deps.selectedTruss3dNodes,
      memberDraftNodes: deps.memberDraftNodes,
      immersiveViewport: deps.immersiveViewport,
      immersiveToolDrawerOpen: deps.immersiveToolDrawerOpen,
      immersiveHelpDrawerOpen: deps.immersiveHelpDrawerOpen,
      truss3dProjectionMode: deps.truss3dProjectionMode,
      truss3dViewPreset: deps.truss3dViewPreset,
      truss3dBoxSelectMode: deps.truss3dBoxSelectMode,
      truss3dLinkMode: deps.truss3dLinkMode,
      hasAnyResult: deps.hasAnyResult,
      job: deps.job,
      projects: deps.projects,
      jobHistory: deps.jobHistory,
      resultRecords: deps.resultRecords,
      protocolAgents: deps.protocolAgents,
      health: deps.health,
      message: deps.message,
    });

  const buildSnapshot = (): WorkbenchSnapshot =>
    buildWorkbenchUiSnapshot({
      studyKind: deps.studyKind,
      axialForm: deps.axialForm,
      heatBarModel: deps.heatBarModel,
      heatPlaneModel: deps.heatPlaneModel,
      thermalBarModel: deps.thermalBarModel,
      thermalBeamModel: deps.thermalBeamModel,
      thermalFrameModel: deps.thermalFrameModel,
      thermalTrussModel: deps.thermalTrussModel,
      trussModel: deps.trussModel,
      thermalTruss3dModel: deps.thermalTruss3dModel,
      truss3dModel: deps.truss3dModel,
      planeModel: deps.planeModel,
      frameModel: deps.frameModel,
      beamModel: deps.beamModel,
      torsionModel: deps.torsionModel,
      springModel: deps.springModel,
      spring2dModel: deps.spring2dModel,
      spring3dModel: deps.spring3dModel,
      parametric: deps.parametric,
      panelParametric: deps.panelParametric,
      activeMaterial: deps.activeMaterial,
      loadedModelName: deps.loadedModelName,
      sidebarSection: deps.sidebarSection,
      selectedNode: deps.selectedNode,
      selectedElement: deps.selectedElement,
      memberDraftNodes: deps.memberDraftNodes,
    });

  const restoreSnapshot = (snapshot: WorkbenchSnapshot) =>
    restoreWorkbenchUiSnapshot(snapshot, {
      setStudyKind: deps.setStudyKind,
      setAxialForm: deps.setAxialForm,
      setHeatBarModel: deps.setHeatBarModel,
      setHeatPlaneModel: deps.setHeatPlaneModel,
      setThermalBarModel: deps.setThermalBarModel,
      setThermalBeamModel: deps.setThermalBeamModel,
      setThermalFrameModel: deps.setThermalFrameModel,
      setThermalTrussModel: deps.setThermalTrussModel,
      setTrussModel: deps.setTrussModel,
      setThermalTruss3dModel: deps.setThermalTruss3dModel,
      setTruss3dModel: deps.setTruss3dModel,
      setPlaneModel: deps.setPlaneModel,
      setFrameModel: deps.setFrameModel,
      setBeamModel: deps.setBeamModel,
      setTorsionModel: deps.setTorsionModel,
      setSpringModel: deps.setSpringModel,
      setSpring2dModel: deps.setSpring2dModel,
      setSpring3dModel: deps.setSpring3dModel,
      setParametric: deps.setParametric,
      setPanelParametric: deps.setPanelParametric,
      setActiveMaterial: deps.setActiveMaterial,
      setLoadedModelName: deps.setLoadedModelName,
      setSidebarSection: deps.setSidebarSection,
      setSelectedNode: deps.setSelectedNode,
      setSelectedElement: deps.setSelectedElement,
      setMemberDraftNodes: deps.setMemberDraftNodes,
      resetActiveResult: deps.resetActiveResult,
    });

  const recordHistory = (label: string) => {
    const snapshot = buildSnapshot();
    deps.setUndoStack((current: any[]) => pushHistoryEntry(current, label, snapshot));
    deps.setRedoStack([]);
  };

  const handleUndo = () => {
    const currentSnapshot = buildSnapshot();
    const { entry, nextSource, nextTarget } = stepHistory(deps.undoStack, deps.redoStack, currentSnapshot);
    if (!entry) return;
    deps.setUndoStack(nextSource);
    deps.setRedoStack(nextTarget);
    restoreSnapshot(entry.snapshot);
    deps.setMessage(deps.t.undoApplied);
  };

  const handleRedo = () => {
    const currentSnapshot = buildSnapshot();
    const { entry, nextSource, nextTarget } = stepHistory(deps.redoStack, deps.undoStack, currentSnapshot);
    if (!entry) return;
    deps.setRedoStack(nextSource);
    deps.setUndoStack(nextTarget);
    restoreSnapshot(entry.snapshot);
    deps.setMessage(deps.t.redoApplied);
  };

  const canProjectHeatToThermo =
    (deps.studyKind === "heat_bar_1d" && Boolean(deps.heatBarResult)) ||
    (deps.studyKind === "heat_plane_triangle_2d" && Boolean(deps.heatPlaneTriangleResult)) ||
    (deps.studyKind === "heat_plane_quad_2d" && Boolean(deps.heatPlaneQuadResult));

  const projectHeatToThermoStudy = () => {
    if (deps.studyKind === "heat_bar_1d" && deps.heatBarResult) {
      recordHistory(deps.t.projectHeatToThermoAction);
      deps.resetActiveResult();
      deps.setThermalBarModel(buildThermalBarFromHeatResult(deps.heatBarModel, deps.heatBarResult, deps.thermalBarModel));
      deps.setStudyKind("thermal_bar_1d");
      deps.openWorkspaceStudy("controls");
      deps.setMessage(deps.t.projectedHeatToThermo);
      return "thermal_bar_1d" as const;
    }

    if (deps.studyKind === "heat_plane_triangle_2d" && deps.heatPlaneTriangleResult) {
      recordHistory(deps.t.projectHeatToThermoAction);
      deps.resetActiveResult();
      deps.setPlaneModel(
        buildThermalPlaneTriangleFromHeatResult(
          deps.heatPlaneModel,
          deps.heatPlaneTriangleResult,
          deps.planeModel,
          deps.activeMaterial,
        ),
      );
      deps.setPlaneResultField("average_temperature_delta");
      deps.setStudyKind("thermal_plane_triangle_2d");
      deps.openWorkspaceStudy("controls");
      deps.setMessage(deps.t.projectedHeatToThermo);
      return "thermal_plane_triangle_2d" as const;
    }

    if (deps.studyKind === "heat_plane_quad_2d" && deps.heatPlaneQuadResult) {
      recordHistory(deps.t.projectHeatToThermoAction);
      deps.resetActiveResult();
      deps.setPlaneModel(
        buildThermalPlaneQuadFromHeatResult(
          deps.heatPlaneModel,
          deps.heatPlaneQuadResult,
          deps.planeModel,
          deps.activeMaterial,
        ),
      );
      deps.setPlaneResultField("average_temperature_delta");
      deps.setStudyKind("thermal_plane_quad_2d");
      deps.openWorkspaceStudy("controls");
      deps.setMessage(deps.t.projectedHeatToThermo);
      return "thermal_plane_quad_2d" as const;
    }

    return null;
  };

  return {
    buildScriptSnapshot,
    buildSnapshot,
    canProjectHeatToThermo,
    handleRedo,
    handleUndo,
    projectHeatToThermoStudy,
    recordHistory,
    restoreSnapshot,
  };
}
