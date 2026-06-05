"use client";

import {
  downloadWorkbenchLanguagePackTemplate,
  exportWorkbenchInstalledLanguagePack,
  importWorkbenchLanguagePack,
  removeWorkbenchLanguagePack,
} from "@/components/workbench/workbench-language-pack-controller";
import {
  downloadWorkbenchSecurityEventCsvExport,
  downloadWorkbenchSecurityEventExport,
} from "@/components/workbench/workbench-export-controller";
import {
  buildWorkbenchScriptSnapshot,
  buildWorkbenchUiSnapshot,
  restoreWorkbenchUiSnapshot,
} from "@/components/workbench/workbench-script-orchestration";
import {
  pushHistoryEntry,
  stepHistory,
  type HistoryEntry,
  type WorkbenchSnapshot,
} from "@/lib/workbench/history";
import type { WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";

type TopLevelActionsArgs = {
  language: any;
  t: any;
  activeLanguagePack: any;
  setLanguagePacks: (updater: any) => void;
  setMessage: (value: string) => void;
  securityEventWindowFilter: any;
  securityEventSourceFilter: any;
  securityEventRiskFilter: any;
  securityEventStatusFilter: any;
  securityEventActionFilter: any;
  undoStack: HistoryEntry[];
  redoStack: HistoryEntry[];
  setUndoStack: (updater: any) => void;
  setRedoStack: (updater: any) => void;
  studyKind: any;
  sidebarSection: any;
  studyTab: any;
  modelTab: any;
  libraryTab: any;
  systemPanelTab: any;
  systemDataTab: any;
  theme: any;
  frontendRuntimeMode: any;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  adminFilterProjectId: string;
  adminFilterModelVersionId: string;
  loadedModelName: string;
  activeMaterial: any;
  selectedNode: any;
  selectedElement: any;
  selectedTruss3dNodes: any;
  memberDraftNodes: any;
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  truss3dProjectionMode: any;
  truss3dViewPreset: any;
  truss3dBoxSelectMode: any;
  truss3dLinkMode: any;
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
  setActiveMaterial: (value: any) => void;
  setLoadedModelName: (value: any) => void;
  setSidebarSection: (value: any) => void;
  setSelectedNode: (value: any) => void;
  setSelectedElement: (value: any) => void;
  setMemberDraftNodes: (value: any) => void;
  resetActiveResult: () => void;
};

export function createWorkbenchTopLevelActionsController(args: TopLevelActionsArgs) {
  const buildScriptSnapshot = (): WorkbenchScriptSnapshot =>
    buildWorkbenchScriptSnapshot({
      studyKind: args.studyKind,
      sidebarSection: args.sidebarSection,
      studyTab: args.studyTab,
      modelTab: args.modelTab,
      libraryTab: args.libraryTab,
      systemPanelTab: args.systemPanelTab,
      systemDataTab: args.systemDataTab,
      language: args.language,
      theme: args.theme,
      frontendRuntimeMode: args.frontendRuntimeMode,
      selectedProjectId: args.selectedProjectId,
      selectedModelId: args.selectedModelId,
      selectedVersionId: args.selectedVersionId,
      selectedAdminJobId: args.selectedAdminJobId,
      selectedAdminResultJobId: args.selectedAdminResultJobId,
      adminFilterProjectId: args.adminFilterProjectId,
      adminFilterModelVersionId: args.adminFilterModelVersionId,
      loadedModelName: args.loadedModelName,
      activeMaterial: args.activeMaterial,
      selectedNode: args.selectedNode,
      selectedElement: args.selectedElement,
      selectedTruss3dNodes: args.selectedTruss3dNodes,
      memberDraftNodes: args.memberDraftNodes,
      immersiveViewport: args.immersiveViewport,
      immersiveToolDrawerOpen: args.immersiveToolDrawerOpen,
      immersiveHelpDrawerOpen: args.immersiveHelpDrawerOpen,
      truss3dProjectionMode: args.truss3dProjectionMode,
      truss3dViewPreset: args.truss3dViewPreset,
      truss3dBoxSelectMode: args.truss3dBoxSelectMode,
      truss3dLinkMode: args.truss3dLinkMode,
      hasAnyResult: args.hasAnyResult,
      job: args.job,
      projects: args.projects,
      jobHistory: args.jobHistory,
      resultRecords: args.resultRecords,
      protocolAgents: args.protocolAgents,
      health: args.health,
      message: args.message,
    });

  const buildSnapshot = (): WorkbenchSnapshot =>
    buildWorkbenchUiSnapshot({
      studyKind: args.studyKind,
      axialForm: args.axialForm,
      heatBarModel: args.heatBarModel,
      heatPlaneModel: args.heatPlaneModel,
      thermalBarModel: args.thermalBarModel,
      thermalBeamModel: args.thermalBeamModel,
      thermalFrameModel: args.thermalFrameModel,
      thermalTrussModel: args.thermalTrussModel,
      trussModel: args.trussModel,
      thermalTruss3dModel: args.thermalTruss3dModel,
      truss3dModel: args.truss3dModel,
      planeModel: args.planeModel,
      frameModel: args.frameModel,
      beamModel: args.beamModel,
      torsionModel: args.torsionModel,
      springModel: args.springModel,
      spring2dModel: args.spring2dModel,
      spring3dModel: args.spring3dModel,
      parametric: args.parametric,
      panelParametric: args.panelParametric,
      activeMaterial: args.activeMaterial,
      loadedModelName: args.loadedModelName,
      sidebarSection: args.sidebarSection,
      selectedNode: args.selectedNode,
      selectedElement: args.selectedElement,
      memberDraftNodes: args.memberDraftNodes,
    });

  const restoreSnapshot = (snapshot: WorkbenchSnapshot) =>
    restoreWorkbenchUiSnapshot(snapshot, {
      setStudyKind: args.setStudyKind,
      setAxialForm: args.setAxialForm,
      setHeatBarModel: args.setHeatBarModel,
      setHeatPlaneModel: args.setHeatPlaneModel,
      setThermalBarModel: args.setThermalBarModel,
      setThermalBeamModel: args.setThermalBeamModel,
      setThermalFrameModel: args.setThermalFrameModel,
      setThermalTrussModel: args.setThermalTrussModel,
      setTrussModel: args.setTrussModel,
      setThermalTruss3dModel: args.setThermalTruss3dModel,
      setTruss3dModel: args.setTruss3dModel,
      setPlaneModel: args.setPlaneModel,
      setFrameModel: args.setFrameModel,
      setBeamModel: args.setBeamModel,
      setTorsionModel: args.setTorsionModel,
      setSpringModel: args.setSpringModel,
      setSpring2dModel: args.setSpring2dModel,
      setSpring3dModel: args.setSpring3dModel,
      setParametric: args.setParametric,
      setPanelParametric: args.setPanelParametric,
      setActiveMaterial: args.setActiveMaterial,
      setLoadedModelName: args.setLoadedModelName,
      setSidebarSection: args.setSidebarSection,
      setSelectedNode: args.setSelectedNode,
      setSelectedElement: args.setSelectedElement,
      setMemberDraftNodes: args.setMemberDraftNodes,
      resetActiveResult: args.resetActiveResult,
    });

  const recordHistory = (label: string) => {
    const snapshot = buildSnapshot();
    args.setUndoStack((current: HistoryEntry[]) => pushHistoryEntry(current, label, snapshot));
    args.setRedoStack([]);
  };

  const handleUndo = () => {
    const currentSnapshot = buildSnapshot();
    const { entry, nextSource, nextTarget } = stepHistory(args.undoStack, args.redoStack, currentSnapshot);
    if (!entry) return;
    args.setUndoStack(nextSource);
    args.setRedoStack(nextTarget);
    restoreSnapshot(entry.snapshot);
    args.setMessage(args.t.undoApplied);
  };

  const handleRedo = () => {
    const currentSnapshot = buildSnapshot();
    const { entry, nextSource, nextTarget } = stepHistory(args.redoStack, args.undoStack, currentSnapshot);
    if (!entry) return;
    args.setRedoStack(nextSource);
    args.setUndoStack(nextTarget);
    restoreSnapshot(entry.snapshot);
    args.setMessage(args.t.redoApplied);
  };

  const handleDownloadLanguagePackTemplate = () => {
    downloadWorkbenchLanguagePackTemplate({ language: args.language, copy: args.t, setMessage: args.setMessage });
  };

  const handleExportInstalledLanguagePack = () => {
    exportWorkbenchInstalledLanguagePack({
      language: args.language,
      activeLanguagePack: args.activeLanguagePack,
      setMessage: args.setMessage,
    });
  };

  const handleImportLanguagePack = async (file: File) => {
    await importWorkbenchLanguagePack({
      file,
      language: args.language,
      setLanguagePacks: args.setLanguagePacks,
      setMessage: args.setMessage,
    });
  };

  const handleRemoveLanguagePack = (packId: string) => {
    removeWorkbenchLanguagePack({
      packId,
      setLanguagePacks: args.setLanguagePacks,
      language: args.language,
      setMessage: args.setMessage,
    });
  };

  const downloadSecurityEventExport = async () => {
    await downloadWorkbenchSecurityEventExport({
      language: args.language,
      securityEventWindowFilter: args.securityEventWindowFilter,
      securityEventSourceFilter: args.securityEventSourceFilter,
      securityEventRiskFilter: args.securityEventRiskFilter,
      securityEventStatusFilter: args.securityEventStatusFilter,
      securityEventActionFilter: args.securityEventActionFilter,
      setMessage: args.setMessage,
      labels: { initialFailed: args.t.initialFailed },
    });
  };

  const downloadSecurityEventCsvExport = async () => {
    await downloadWorkbenchSecurityEventCsvExport({
      language: args.language,
      securityEventWindowFilter: args.securityEventWindowFilter,
      securityEventSourceFilter: args.securityEventSourceFilter,
      securityEventRiskFilter: args.securityEventRiskFilter,
      securityEventStatusFilter: args.securityEventStatusFilter,
      securityEventActionFilter: args.securityEventActionFilter,
      setMessage: args.setMessage,
      labels: { initialFailed: args.t.initialFailed },
    });
  };

  return {
    buildScriptSnapshot,
    buildSnapshot,
    restoreSnapshot,
    scriptSnapshot: buildScriptSnapshot(),
    recordHistory,
    handleUndo,
    handleRedo,
    handleDownloadLanguagePackTemplate,
    handleExportInstalledLanguagePack,
    handleImportLanguagePack,
    handleRemoveLanguagePack,
    downloadSecurityEventExport,
    downloadSecurityEventCsvExport,
  };
}
