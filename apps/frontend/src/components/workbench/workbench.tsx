"use client";

import {
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
  useTransition,
  type Dispatch,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type SetStateAction,
} from "react";
import brand from "../../../../../assets/brand/brand.json";
import {
  applyJobContextToWorkbench as applyJobContextToWorkbenchWithDeps,
  applySelectedAdminJobContext as applySelectedAdminJobContextWithDeps,
  applySelectedAdminResultContext as applySelectedAdminResultContextWithDeps,
  openProjectContextById as openProjectContextByIdWithDeps,
  openSelectedAdminJobProject as openSelectedAdminJobProjectWithDeps,
  openSelectedAdminJobVersion as openSelectedAdminJobVersionWithDeps,
  openSelectedAdminResultProject as openSelectedAdminResultProjectWithDeps,
  openSelectedAdminResultVersion as openSelectedAdminResultVersionWithDeps,
  resolveScriptLinkedJob as resolveScriptLinkedJobWithDeps,
} from "@/components/workbench/workbench-admin-data-controller";
import {
  deleteWorkbenchAdminResultRecord,
  exportWorkbenchAdminResultRecord,
  refreshWorkbenchResults,
  saveWorkbenchAdminResultRecord,
} from "@/components/workbench/workbench-admin-result-controller";
import { useWorkbenchAssistantController } from "@/components/workbench/workbench-assistant-controller";
import { useWorkbenchAssistantAuditController } from "@/components/workbench/workbench-assistant-audit-controller";
import { WorkbenchAssistantPanel } from "@/components/workbench/workbench-assistant-panel";
import { WorkbenchAssistantFloat } from "@/components/workbench/workbench-assistant-float";
import { WorkbenchConsoleMount } from "@/components/workbench/workbench-console-mount";
import { WorkbenchImmersiveLibraryDrawer } from "@/components/workbench/workbench-immersive-library-drawer";
import { WorkbenchLibrarySectionMount } from "@/components/workbench/workbench-library-section-mount";
import { WorkbenchMainShellMount } from "@/components/workbench/workbench-main-shell-mount";
import { WorkbenchMainViewportPanelMount } from "@/components/workbench/workbench-main-viewport-panel-mount";
import { WorkbenchModelSectionMount } from "@/components/workbench/workbench-model-section-mount";
import {
  downloadWorkbenchLanguagePackTemplate,
  exportWorkbenchInstalledLanguagePack,
  importWorkbenchLanguagePack,
  removeWorkbenchLanguagePack,
} from "@/components/workbench/workbench-language-pack-controller";
import { createWorkbenchMaterialEditController } from "@/components/workbench/workbench-material-edit-controller";
import {
  copyByLanguage,
  humanizeSolverFailure,
  type WorkbenchCopy,
  type WorkbenchLanguage,
} from "@/components/workbench/workbench-copy";
import { buildRuntimeAuditModelVersionFacets, buildRuntimeAuditProjectFacets, buildRuntimeAuditSourceStatusFacets, buildRuntimeAuditStudyFacets, buildRuntimeAuditSummaryRows, buildRuntimeAuditTrendBars } from "@/components/workbench/workbench-runtime-audit-helpers";
import {
  defaultAxial,
  defaultBeam1d,
  defaultFrame2d,
  defaultHeatBar1d,
  defaultHeatPlaneQuad,
  defaultHeatPlaneTriangle,
  defaultPanelParametric,
  defaultParametric,
  defaultPlaneQuad,
  defaultPlaneTriangle,
  defaultSpring1d,
  defaultSpring2d,
  defaultSpring3d,
  defaultThermalBar1d,
  defaultThermalBeam1d,
  defaultThermalFrame2d,
  defaultThermalPlaneQuad,
  defaultThermalPlaneTriangle,
  defaultThermalTruss2d,
  defaultThermalTruss3d,
  defaultTorsion1d,
  defaultTruss,
  defaultTruss3d,
  type AxialFormState,
  type DirectMeshExecutionState,
  type DisplayTruss3dElement,
  type DisplayTruss3dNode,
  type DisplayTrussElement,
  type DisplayTrussNode,
  type PlaneResultField,
  type SelectionKind,
  type StabilitySummary,
  type TrussDiagnostics,
  type TrussSuggestion,
} from "@/components/workbench/workbench-defaults";
import { applyHistoryJobPayload } from "@/components/workbench/workbench-history-result";
import { useWorkbenchDataRefreshController } from "@/components/workbench/workbench-data-refresh-controller";
import {
  downloadBlobFile,
  downloadTextFile,
  resetActiveResult,
} from "@/components/workbench/workbench-file-helpers";
import {
  buildDisplayBeamElements,
  buildDisplayBeamNodes,
  buildDisplaySpring2dElements,
  buildDisplaySpring2dNodes,
  buildDisplaySpringElements,
  buildDisplaySpringNodes,
  buildDisplayThermalBeamElements,
  buildDisplayThermalBeamNodes,
  buildDisplayTorsionElements,
  buildDisplayTorsionNodes,
} from "@/components/workbench/workbench-display-line-helpers";
import { applyImportedWorkbenchModel } from "@/components/workbench/workbench-model-load";
import {
  buildThermalBarFromHeatResult,
  buildThermalPlaneQuadFromHeatResult,
  buildThermalPlaneTriangleFromHeatResult,
  ensureBeamModelMaterials,
  ensureFrameModelMaterials,
} from "@/components/workbench/workbench-model-transform-helpers";
import {
  buildDisplayFrameElements,
  buildDisplayFrameNodes,
  buildDisplayHeatBarElements,
  buildDisplayHeatBarNodes,
  buildDisplayThermalBarElements,
  buildDisplayThermalBarNodes,
  buildDisplayThermalFrameElements,
  buildDisplayThermalFrameNodes,
  buildDisplayThermalTrussElements,
  buildDisplayThermalTrussNodes,
  buildDisplayTrussElements,
  buildDisplayTrussNodes,
} from "@/components/workbench/workbench-display-planar-helpers";
import {
  downloadWorkbenchDatabaseSnapshot,
  downloadWorkbenchProjectBundleJson,
  downloadWorkbenchProjectBundleZip,
  downloadWorkbenchSecurityEventCsvExport,
  downloadWorkbenchSecurityEventExport,
} from "@/components/workbench/workbench-export-controller";
import {
  downloadWorkbenchFrameForceSummary,
  downloadWorkbenchFrameHotspotSummary,
  downloadWorkbenchPlaneHotspotSummary,
  downloadWorkbenchResultCsv,
  downloadWorkbenchResultJson,
} from "@/components/workbench/workbench-result-export-controller";
import {
  useWorkbenchResultWindowController,
  type ResultWindowState,
} from "@/components/workbench/workbench-result-window-controller";
import { WorkbenchResultWindowBar } from "@/components/workbench/workbench-result-window-bar";
import {
  buildDisplaySpring3dElements,
  buildDisplaySpring3dNodes,
  buildDisplayThermalTruss3dElements,
  buildDisplayThermalTruss3dNodes,
  buildDisplayTruss3dElements,
  buildDisplayTruss3dNodes,
  projectTruss3dPoint,
} from "@/components/workbench/workbench-display-spatial-helpers";
import {
  clusterHealthTone,
  formatJobMessage,
  formatPeerStatus,
  formatProtocolMethodLabel,
  heartbeatStatus,
  heartbeatTone,
  isAxialResult,
  isBeam1dResult,
  isFrame2dResult,
  isHeatBar1dResult,
  isHeatPlaneQuad2dResult,
  isHeatPlaneTriangle2dResult,
  isPlaneResult,
  isSpring1dResult,
  isSpring2dResult,
  isSpring3dResult,
  isThermalBar1dResult,
  isThermalBeam1dResult,
  isThermalFrame2dResult,
  isThermalTruss2dResult,
  isThermalTruss3dResult,
  isTorsion1dResult,
  isTruss3dResult,
  isTrussResult,
  lineResultFieldValue,
  localMaterialLabel,
  materialColorByIndex,
  planeResultFieldValue,
  planeStressFill,
} from "@/components/workbench/workbench-result-helpers";
import {
  importWorkbenchProjectBundle,
  openPersistedWorkbenchModel,
  openPersistedWorkbenchVersion,
  openPersistedWorkbenchVersionById,
} from "@/components/workbench/workbench-persisted-model-controller";
import {
  buildWorkbenchHotspotData,
  buildWorkbenchSecurityUi,
  buildWorkbenchSelectionData,
} from "@/components/workbench/workbench-inspector-derived";
import { WorkbenchInspectorMount } from "@/components/workbench/workbench-inspector-mount";
import { useWorkbenchJobHistoryController } from "@/components/workbench/workbench-job-history-controller";
import { WorkbenchObjectTree } from "@/components/workbench/workbench-object-tree";
import { handleWorkbenchScriptMacroDataAction } from "@/components/workbench/workbench-script-macro-data-controller";
import { handleWorkbenchScriptNavAction } from "@/components/workbench/workbench-script-nav-controller";
import { handleWorkbenchScriptProjectModelAction } from "@/components/workbench/workbench-script-project-model-controller";
import { handleWorkbenchScriptStateAction } from "@/components/workbench/workbench-script-state-controller";
import { WorkbenchViewportHeadActions } from "@/components/workbench/workbench-viewport-head-actions";
import { WorkbenchViewportDock } from "@/components/workbench/workbench-viewport-dock";
import { WorkbenchViewportMount } from "@/components/workbench/workbench-viewport-mount";
import { WorkbenchSidebarMount } from "@/components/workbench/workbench-sidebar-mount";
import {
  importWorkbenchModelFile,
  openWorkbenchSample,
} from "@/components/workbench/workbench-sample-import-controller";
import { createWorkbenchStructureEditController } from "@/components/workbench/workbench-structure-edit-controller";
import {
  applyStudyKindSelection,
  createStudyKindResetHandlers,
  isWorkbenchStudyKind,
} from "@/components/workbench/workbench-study-kind-controller";
import {
  analyzeTrussModel,
  findNearestConnectableNode,
  getTrussBounds,
  renderLoadGlyph,
  renderSupportGlyph,
  round,
  summarizeTrussStability,
  toSvgPoint,
} from "@/components/workbench/workbench-truss-helpers";
import { createWorkbenchTrussGestureController } from "@/components/workbench/workbench-truss-gesture-controller";
import { runWorkbenchAnalysis } from "@/components/workbench/workbench-run-controller";
import { buildWorkbenchStudySidebarData } from "@/components/workbench/workbench-study-sidebar-data";
import { WorkbenchViewportPanel } from "@/components/workbench/workbench-viewport-panel";
import { WorkbenchViewport } from "@/components/workbench/workbench-viewport";
import {
  type AssistantMode,
  type BeamResultField,
  type BeamStudyJobInput,
  type FrameResultField,
  type FrameStudyJobInput,
  type HeatBarStudyJobInput,
  type HeatPlaneStudyJobInput,
  type ImmersiveToolTab,
  type Language,
  type LibraryPanelTab,
  type LineResultField,
  type ModelPanelTab,
  type PlaneStudyJobInput,
  SECURITY_EVENT_WINDOW_MS,
  type SecurityEventWindow,
  type SidebarSection,
  type Spring2dStudyJobInput,
  type Spring3dStudyJobInput,
  type SpringStudyJobInput,
  type StudyKind,
  type StudyPanelTab,
  type SystemDataTab,
  type SystemPanelTab,
  type ThermalBarStudyJobInput,
  type ThermalBeamStudyJobInput,
  type ThermalFrameStudyJobInput,
  type ThermalTruss2dStudyJobInput,
  type ThermalTruss3dStudyJobInput,
  type Theme,
  type WorkflowPanelTab,
} from "@/components/workbench/workbench-types";
import { WorkbenchLibrarySidebar } from "@/components/workbench/library/workbench-library-sidebar";
import { WorkbenchMaterialLibraryCard } from "@/components/workbench/model/workbench-material-library-card";
import type { ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";
import { WorkbenchModelToolsCard } from "@/components/workbench/model/workbench-model-tools-card";
import { WorkbenchParametricCard } from "@/components/workbench/model/workbench-parametric-card";
import { WorkbenchTruss3dTreeCard } from "@/components/workbench/model/workbench-truss3d-tree-card";
import { WorkbenchSystemSidebarMount } from "@/components/workbench/workbench-system-sidebar-mount";
import { WorkbenchWorkflowSectionMount } from "@/components/workbench/workbench-workflow-section-mount";
import { WorkbenchStudySectionMount } from "@/components/workbench/workbench-study-section-mount";
import { WorkbenchStudySidebar } from "@/components/workbench/study/workbench-study-sidebar";
import {
  isWorkflowGraphResult,
  summarizeWorkflowArtifacts,
  upsertWorkflowRunRecord,
  useWorkbenchWorkflowController,
} from "@/components/workbench/workflow/workbench-workflow-controller";
import { WorkbenchWorkflowSidebar } from "@/components/workbench/workflow/workbench-workflow-sidebar";
import { MATERIAL_PRESETS } from "@/lib/materials";
import {
  persistWorkbenchSettings,
  fixed,
  formatMilliseconds,
  formatTime,
  parseDirectMeshEndpoints,
  readWorkbenchLanguagePacks,
  persistWorkbenchLanguagePacks,
  safeStorageGet,
  scientific,
  serializeCurrentModel,
  toAxialInput,
  mergeLanguagePack,
  type WorkbenchLanguagePack,
} from "@/lib/workbench/helpers";
import {
  buildWorkbenchSnapshot,
  pushHistoryEntry,
  restoreWorkbenchSnapshot,
  stepHistory,
  type HistoryEntry,
  type WorkbenchSnapshot,
} from "@/lib/workbench/history";
import {
  readSecurityAuditLog,
  type WorkbenchSecurityAuditRisk,
  type WorkbenchSecurityAuditSource,
} from "@/lib/workbench/security-audit";
import {
  buildAdminJobRows,
  buildAdminResultRows,
  buildLibraryJobRows,
  buildLibraryModelRows,
  buildLibrarySampleRows,
  buildLibraryVersionRows,
  buildProtocolAgentCards,
  buildStudyDomainOptions,
  classifyStudyKindDomain,
  classifyStudyKindFamily,
  buildStudyKindOptionGroups,
} from "@/lib/workbench/view-models";
import {
  ensurePlaneModelMaterials,
  ensureTruss3dModelMaterials,
  ensureTrussModelMaterials,
} from "@/lib/workbench/material-commands";
import { clampChunkOffset, RESULT_WINDOW_BASE_SIZE } from "@/lib/workbench/result-window";
import { updateFrame2dNode } from "@/lib/workbench/frame2d-commands";
import { toggleDraftSelection } from "@/lib/workbench/truss2d-commands";
import {
  addTruss3dNodeCommand,
} from "@/lib/workbench/truss3d-commands";
import {
  exportProjectBundle,
  exportStudyModel,
  generatePrattTruss,
  generateRectangularQuadPanelMesh,
  generateRectangularPanelMesh,
  type ParametricPanelConfig,
  type ParametricTrussConfig,
} from "@/lib/models";
import { SAMPLE_LIBRARY } from "@/lib/models";
import {
  getWorkbenchScriptActionDefinition,
  listWorkbenchMacroPresets,
  type WorkbenchScriptSnapshot,
} from "@/lib/scripting/workbench-script-runtime";
import {
  createAxialBarJob,
  createBeam1dJob,
  createHeatBar1dJob,
  createHeatPlaneQuad2dJob,
  createHeatPlaneTriangle2dJob,
  createThermalBeam1dJob,
  createThermalBar1dJob,
  createThermalFrame2dJob,
  createThermalPlaneQuad2dJob,
  createThermalPlaneTriangle2dJob,
  createThermalTruss2dJob,
  createThermalTruss3dJob,
  createSpring1dJob,
  createSpring2dJob,
  createSpring3dJob,
  createTorsion1dJob,
  createDirectMeshSolve,
  createFrame2dJob,
  createPlaneQuad2dJob,
  createPlaneTriangle2dJob,
  createModel,
  createModelVersion,
  createProject,
  createTruss2dJob,
  createTruss3dJob,
  cancelJob,
  deleteJobRecord,
  deleteModel,
  deleteModelVersion,
  deleteProject,
  deleteResultRecord,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchJobStatus,
  fetchResults,
  type AxialBarJobInput,
  type AxialBarResult,
  type Beam1dElementInput,
  type Beam1dJobInput,
  type Beam1dNodeInput,
  type Beam1dResult,
  type DirectMeshSelectionMode,
  type Frame2dElementInput,
  type Frame2dJobInput,
  type Frame2dNodeInput,
  type Frame2dResult,
  type FrontendRuntimeMode,
  type HeatBar1dElementInput,
  type HeatBar1dJobInput,
  type HeatBar1dNodeInput,
  type HeatBar1dResult,
  type HeatPlaneNodeInput,
  type HeatPlaneQuad2dJobInput,
  type HeatPlaneQuad2dResult,
  type HeatPlaneTriangle2dJobInput,
  type HeatPlaneTriangle2dResult,
  type HealthPayload,
  type JobEnvelope,
  type JobResultRecord,
  type JobState,
  type ModelRecord,
  type ModelVersionRecord,
  type PlaneQuad2dJobInput,
  type PlaneQuad2dResult,
  type PlaneTriangle2dJobInput,
  type PlaneTriangle2dResult,
  type ThermalPlaneQuad2dJobInput,
  type ThermalPlaneQuad2dResult,
  type ThermalPlaneTriangle2dJobInput,
  type ThermalPlaneTriangle2dResult,
  type ProtocolAgentDescriptor,
  type ProjectRecord,
  type ResultRecord,
  type SecurityEventRecord,
  type ThermalBeam1dElementInput,
  type ThermalBeam1dJobInput,
  type ThermalBeam1dNodeInput,
  type ThermalBeam1dResult,
  type ThermalBar1dElementInput,
  type ThermalBar1dJobInput,
  type ThermalBar1dNodeInput,
  type ThermalBar1dResult,
  type ThermalFrame2dElementInput,
  type ThermalFrame2dJobInput,
  type ThermalFrame2dNodeInput,
  type ThermalFrame2dResult,
  type ThermalTruss2dElementInput,
  type ThermalTruss2dJobInput,
  type ThermalTruss2dNodeInput,
  type ThermalTruss2dResult,
  type ThermalTruss3dElementInput,
  type ThermalTruss3dJobInput,
  type ThermalTruss3dNodeInput,
  type ThermalTruss3dResult,
  type WorkflowGraphJobResult,
  type Spring1dJobInput,
  type Spring1dResult,
  type Spring2dJobInput,
  type Spring2dResult,
  type Spring3dJobInput,
  type Spring3dResult,
  type Torsion1dJobInput,
  type Torsion1dResult,
  resolveHeatBar1dJobInput,
  resolveHeatPlaneQuad2dJobInput,
  resolveHeatPlaneTriangle2dJobInput,
  resolveBeam1dJobInput,
  resolveThermalBeam1dJobInput,
  resolveThermalBar1dJobInput,
  resolveThermalFrame2dJobInput,
  resolveThermalTruss2dJobInput,
  resolveThermalTruss3dJobInput,
  resolveSpring1dJobInput,
  resolveSpring2dJobInput,
  resolveSpring3dJobInput,
  resolveTorsion1dJobInput,
  resolveThermalPlaneQuad2dJobInput,
  resolveThermalPlaneTriangle2dJobInput,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveFrame2dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  type Truss2dJobInput,
  type Truss2dResult,
  type Truss3dJobInput,
  type Truss3dResult,
  updateJobRecord,
  updateModel,
  updateModelVersion,
  updateProject,
  updateResultRecord,
} from "@/lib/api";

export function Workbench() {
  const [studyKind, setStudyKind] = useState<StudyKind>("axial_bar_1d");
  const [axialForm, setAxialForm] = useState<AxialFormState>(defaultAxial);
  const [heatBarModel, setHeatBarModel] = useState<HeatBarStudyJobInput>(defaultHeatBar1d);
  const [heatPlaneModel, setHeatPlaneModel] = useState<HeatPlaneStudyJobInput>(defaultHeatPlaneTriangle);
  const [thermalBarModel, setThermalBarModel] = useState<ThermalBarStudyJobInput>(defaultThermalBar1d);
  const [thermalBeamModel, setThermalBeamModel] = useState<ThermalBeamStudyJobInput>(defaultThermalBeam1d);
  const [thermalFrameModel, setThermalFrameModel] = useState<ThermalFrameStudyJobInput>(defaultThermalFrame2d);
  const [thermalTrussModel, setThermalTrussModel] = useState<ThermalTruss2dStudyJobInput>(defaultThermalTruss2d);
  const [trussModel, setTrussModel] = useState<Truss2dJobInput>(defaultTruss);
  const [thermalTruss3dModel, setThermalTruss3dModel] = useState<ThermalTruss3dStudyJobInput>(defaultThermalTruss3d);
  const [truss3dModel, setTruss3dModel] = useState<Truss3dJobInput>(defaultTruss3d);
  const [planeModel, setPlaneModel] = useState<PlaneStudyJobInput>(defaultPlaneTriangle);
  const [frameModel, setFrameModel] = useState<FrameStudyJobInput>(defaultFrame2d);
  const [beamModel, setBeamModel] = useState<BeamStudyJobInput>(defaultBeam1d);
  const [torsionModel, setTorsionModel] = useState<Torsion1dJobInput>(defaultTorsion1d);
  const [springModel, setSpringModel] = useState<SpringStudyJobInput>(defaultSpring1d);
  const [spring2dModel, setSpring2dModel] = useState<Spring2dStudyJobInput>(defaultSpring2d);
  const [spring3dModel, setSpring3dModel] = useState<Spring3dStudyJobInput>(defaultSpring3d);
  const [parametric, setParametric] = useState<ParametricTrussConfig>(defaultParametric);
  const [panelParametric, setPanelParametric] = useState<ParametricPanelConfig>(defaultPanelParametric);
  const [activeMaterial, setActiveMaterial] = useState("210");
  const [planeResultField, setPlaneResultField] = useState<PlaneResultField>("von_mises");
  const [frameResultField, setFrameResultField] = useState<FrameResultField>("max_combined_stress");
  const [beamResultField, setBeamResultField] = useState<BeamResultField>("max_bending_stress");
  const [planeHotspotLimit, setPlaneHotspotLimit] = useState(5);
  const [focusedPlaneElement, setFocusedPlaneElement] = useState<number | null>(null);
  const [focusedFrameElement, setFocusedFrameElement] = useState<number | null>(null);
  const [result, setResult] = useState<
    AxialBarResult | HeatBar1dResult | HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult | ThermalBar1dResult | ThermalBeam1dResult | ThermalTruss2dResult | ThermalTruss3dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult | Spring1dResult | Spring2dResult | Spring3dResult | Beam1dResult | Torsion1dResult | Truss2dResult | Truss3dResult | Frame2dResult | ThermalFrame2dResult | PlaneTriangle2dResult | PlaneQuad2dResult | null
  >(null);
  const [resultWindow, setResultWindow] = useState<ResultWindowState | null>(null);
  const [resultWindowOffset, setResultWindowOffset] = useState(0);
  const [resultWindowLimit, setResultWindowLimit] = useState(RESULT_WINDOW_BASE_SIZE);
  const [canvasViewportWidth, setCanvasViewportWidth] = useState(980);
  const [job, setJob] = useState<JobEnvelope["job"] | null>(null);
  const [resultRecords, setResultRecords] = useState<ResultRecord[]>([]);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [selectedVersionId, setSelectedVersionId] = useState<string | null>(null);
  const [modelVersions, setModelVersions] = useState<ModelVersionRecord[]>([]);
  const [projectNameDraft, setProjectNameDraft] = useState<string>(copyByLanguage.en.defaultProject);
  const [projectDescriptionDraft, setProjectDescriptionDraft] = useState<string>("");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [protocolAgents, setProtocolAgents] = useState<ProtocolAgentDescriptor[]>([]);
  const [loadedModelName, setLoadedModelName] = useState<string>(copyByLanguage.en.defaultModel);
  const [message, setMessage] = useState<string>(copyByLanguage.en.initialLoaded);
  const [language, setLanguage] = useState<Language>("en");
  const [languagePacks, setLanguagePacks] = useState<WorkbenchLanguagePack[]>([]);
  const [theme, setTheme] = useState<Theme>("linen");
  const [frontendRuntimeMode, setFrontendRuntimeMode] = useState<FrontendRuntimeMode>("orchestrated_gui");
  const [directMeshEndpointsText, setDirectMeshEndpointsText] = useState("127.0.0.1:5001,127.0.0.1:5002");
  const [directMeshSelectionMode, setDirectMeshSelectionMode] = useState<DirectMeshSelectionMode>("healthiest");
  const [controlPlaneApiToken, setControlPlaneApiToken] = useState("");
  const [clusterApiToken, setClusterApiToken] = useState("");
  const [directMeshApiToken, setDirectMeshApiToken] = useState("");
  const [assistantMode, setAssistantMode] = useState<AssistantMode>("local");
  const [assistantApiBaseUrl, setAssistantApiBaseUrl] = useState("https://api.openai.com/v1");
  const [assistantApiKey, setAssistantApiKey] = useState("");
  const [assistantModel, setAssistantModel] = useState("gpt-4.1-mini");
  const applyLanguagePreference = (nextLanguage: Language) => {
    setLanguage(nextLanguage);
    setLoadedModelName((current) =>
      current === copyByLanguage.en.defaultModel ||
      current === copyByLanguage.zh.defaultModel ||
      current === copyByLanguage.ja.defaultModel
        ? copyByLanguage[nextLanguage].defaultModel
        : current,
    );
  };
  const [assistantWindowOpen, setAssistantWindowOpen] = useState(false);
  const [directMeshExecution, setDirectMeshExecution] = useState<DirectMeshExecutionState | null>(null);
  const [showShortcutHints, setShowShortcutHints] = useState(true);
  const [immersiveGuardrails, setImmersiveGuardrails] = useState(true);
  const [immersiveViewport, setImmersiveViewport] = useState(false);
  const [immersiveToolDrawerOpen, setImmersiveToolDrawerOpen] = useState(false);
  const [immersiveHelpDrawerOpen, setImmersiveHelpDrawerOpen] = useState(false);
  const [immersiveToolTab, setImmersiveToolTab] = useState<ImmersiveToolTab>("node");
  const [truss3dProjectionMode, setTruss3dProjectionMode] = useState<"ortho" | "persp">("ortho");
  const [truss3dShowGrid, setTruss3dShowGrid] = useState(true);
  const [truss3dShowLabels, setTruss3dShowLabels] = useState(true);
  const [truss3dShowNodes, setTruss3dShowNodes] = useState(true);
  const [truss3dBoxSelectMode, setTruss3dBoxSelectMode] = useState(false);
  const [truss3dViewPreset, setTruss3dViewPreset] = useState<"iso" | "front" | "right" | "top">("iso");
  const [truss3dFocusRequestVersion, setTruss3dFocusRequestVersion] = useState(0);
  const [truss3dResetRequestVersion, setTruss3dResetRequestVersion] = useState(0);
  const [truss3dNudgeStep, setTruss3dNudgeStep] = useState(0.25);
  const [truss3dBatchLoadX, setTruss3dBatchLoadX] = useState(0);
  const [truss3dBatchLoadY, setTruss3dBatchLoadY] = useState(0);
  const [truss3dBatchLoadZ, setTruss3dBatchLoadZ] = useState(0);
  const [hiddenMaterials, setHiddenMaterials] = useState<Record<StudyKind, string[]>>({
    axial_bar_1d: [],
    heat_bar_1d: [],
    heat_plane_triangle_2d: [],
    heat_plane_quad_2d: [],
    thermal_bar_1d: [],
    thermal_beam_1d: [],
    thermal_frame_2d: [],
    thermal_truss_2d: [],
    thermal_truss_3d: [],
    thermal_plane_triangle_2d: [],
    thermal_plane_quad_2d: [],
    spring_1d: [],
    spring_2d: [],
    spring_3d: [],
    beam_1d: [],
    torsion_1d: [],
    truss_2d: [],
    truss_3d: [],
    plane_triangle_2d: [],
    plane_quad_2d: [],
    frame_2d: [],
  });
  const [sidebarSection, setSidebarSection] = useState<SidebarSection>("model");
  const [studyTab, setStudyTab] = useState<StudyPanelTab>("summary");
  const [modelTab, setModelTab] = useState<ModelPanelTab>("tools");
  const [modelToolsPage, setModelToolsPage] = useState<ModelToolsPage>("overview");
  const [libraryTab, setLibraryTab] = useState<LibraryPanelTab>("jobs");
  const [systemDataTab, setSystemDataTab] = useState<SystemDataTab>("jobs");
  const [systemPanelTab, setSystemPanelTab] = useState<SystemPanelTab>("config");
  const [draggingNode, setDraggingNode] = useState<number | null>(null);
  const [truss3dLinkMode, setTruss3dLinkMode] = useState(false);
  const [selectedTruss3dNodes, setSelectedTruss3dNodes] = useState<number[]>([]);
  const [selectedNode, setSelectedNode] = useState<number | null>(null);
  const [selectedElement, setSelectedElement] = useState<number | null>(null);
  const [memberDraftNodes, setMemberDraftNodes] = useState<number[]>([]);
  const [undoStack, setUndoStack] = useState<HistoryEntry[]>([]);
  const [redoStack, setRedoStack] = useState<HistoryEntry[]>([]);
  const [selectedAdminResultJobId, setSelectedAdminResultJobId] = useState<string | null>(null);
  const [adminFilterProjectId, setAdminFilterProjectId] = useState("");
  const [adminFilterModelVersionId, setAdminFilterModelVersionId] = useState("");
  const [securityEventRecords, setSecurityEventRecords] = useState<SecurityEventRecord[]>([]);
  const [scriptRecordingMode, setScriptRecordingMode] = useState(false);
  const [securityEventWindowFilter, setSecurityEventWindowFilter] = useState<SecurityEventWindow>("24h");
  const [securityEventSourceFilter, setSecurityEventSourceFilter] = useState<
    WorkbenchSecurityAuditSource | "hub-assistant" | ""
  >("");
  const [securityEventRiskFilter, setSecurityEventRiskFilter] = useState<
    WorkbenchSecurityAuditRisk | ""
  >("");
  const [securityEventStatusFilter, setSecurityEventStatusFilter] = useState<
    "" | "allowed" | "blocked"
  >("");
  const [securityEventActionFilter, setSecurityEventActionFilter] = useState("");
  const [adminJobMessage, setAdminJobMessage] = useState("");
  const [adminJobProjectId, setAdminJobProjectId] = useState("");

  useEffect(() => {
    if (systemPanelTab === "assistant") {
      setAssistantWindowOpen(true);
      setSystemPanelTab("config");
    }
  }, [systemPanelTab]);
  const [adminJobModelVersionId, setAdminJobModelVersionId] = useState("");
  const [adminJobCaseId, setAdminJobCaseId] = useState("");
  const [adminResultDraft, setAdminResultDraft] = useState("{}");
  const [isPending, startTransition] = useTransition();
  const staleHeartbeatAlertedRef = useRef<string | null>(null);
  const dragHistoryCapturedRef = useRef(false);
  const drag3dHistoryCapturedRef = useRef(false);
  const dragFrameRef = useRef<number | null>(null);
  const pendingDragPointRef = useRef<{ x: number; y: number } | null>(null);
  const viewportPanelRef = useRef<HTMLElement | null>(null);
  const canvasStageRef = useRef<HTMLDivElement | null>(null);
  const resultRefreshSeqRef = useRef(0);
  const jobPollTokenRef = useRef(0);
  const activeLanguagePack = useMemo(
    () => languagePacks.find((pack) => pack.language === language) ?? null,
    [language, languagePacks],
  );
  const t = useMemo(
    () => mergeLanguagePack<WorkbenchCopy>(copyByLanguage[language], activeLanguagePack?.overrides ?? null),
    [activeLanguagePack?.overrides, language],
  );
  const jobIsActive =
    job?.status === "queued" ||
    job?.status === "preprocessing" ||
    job?.status === "partitioning" ||
    job?.status === "solving" ||
    job?.status === "postprocessing";
  const languagePackCatalogRows = useMemo(
    () => [
      { id: "fr-preview", language: "fr", name: "French preview", status: language === "zh" ? "预留远程下载入口" : language === "ja" ? "将来のリモート配布枠" : "Reserved for future remote delivery" },
      { id: "de-preview", language: "de", name: "German preview", status: language === "zh" ? "预留远程下载入口" : language === "ja" ? "将来のリモート配布枠" : "Reserved for future remote delivery" },
    ],
    [language],
  );
  const {
    jobHistory,
    setJobHistory,
    selectedAdminJobId,
    setSelectedAdminJobId,
    refreshJobHistory,
    cancelCurrentJob,
  } = useWorkbenchJobHistoryController({
    labels: {
      jobCancelled: t.jobCancelled,
      initialFailed: t.initialFailed,
      requestTimedOut: t.requestTimedOut,
    },
    job,
    jobIsActive,
    jobPollTokenRef,
    setJob,
    setMessage,
    startTransition,
  });

  const resultWindowMaxTotal = resultWindow ? Math.max(resultWindow.totalNodes, resultWindow.totalElements) : 0;
  const { handleCanvasStageScroll } = useWorkbenchResultWindowController({
    canvasStageRef,
    canvasViewportWidth,
    frontendRuntimeMode,
    guards: {
      isAxialResult,
      isTrussResult,
      isHeatBar1dResult,
      isHeatPlaneQuad2dResult,
      isHeatPlaneTriangle2dResult,
      isThermalBar1dResult,
      isThermalBeam1dResult,
      isThermalTruss2dResult,
      isThermalTruss3dResult,
      isTruss3dResult,
      isSpring1dResult,
      isSpring2dResult,
      isSpring3dResult,
      isBeam1dResult,
      isTorsion1dResult,
      isFrame2dResult,
    },
    jobId: job?.job_id ?? null,
    requestTimedOutLabel: t.requestTimedOut,
    result,
    resultWindow,
    resultWindowLimit,
    resultWindowMaxTotal,
    resultWindowOffset,
    setCanvasViewportWidth,
    setMessage,
    setResultWindow,
    setResultWindowLimit,
    setResultWindowOffset,
    studyKind,
  });

  useEffect(() => {
    if (!job?.job_id) {
      staleHeartbeatAlertedRef.current = null;
      return;
    }

    const tone = heartbeatTone(job);

    if (tone === "stale") {
      if (staleHeartbeatAlertedRef.current !== job.job_id) {
        staleHeartbeatAlertedRef.current = job.job_id;
        setMessage(
          language === "zh"
            ? `任务 ${job.job_id.slice(0, 8)} 心跳已过期，请检查 agent 或考虑取消任务。`
            : `Job ${job.job_id.slice(0, 8)} heartbeat is stale. Check the agent or consider cancelling the run.`,
        );
      }
      return;
    }

    if (staleHeartbeatAlertedRef.current === job.job_id) {
      staleHeartbeatAlertedRef.current = null;
    }
  }, [job, language]);

  useEffect(() => {
    const stored = safeStorageGet();
    const desktopLanguage =
      typeof window !== "undefined"
        ? new URLSearchParams(window.location.search).get("desktopLanguage")
        : null;
    if (stored.theme === "linen" || stored.theme === "marine" || stored.theme === "graphite") {
      setTheme(stored.theme);
    }
    if (typeof stored.showShortcutHints === "boolean") setShowShortcutHints(stored.showShortcutHints);
    if (typeof stored.immersiveGuardrails === "boolean") setImmersiveGuardrails(stored.immersiveGuardrails);
    if (stored.frontendRuntimeMode) setFrontendRuntimeMode(stored.frontendRuntimeMode);
    if (stored.directMeshEndpointsText) setDirectMeshEndpointsText(stored.directMeshEndpointsText);
    if (stored.directMeshSelectionMode === "healthiest" || stored.directMeshSelectionMode === "first_reachable") {
      setDirectMeshSelectionMode(stored.directMeshSelectionMode);
    }
    if (stored.controlPlaneApiToken) setControlPlaneApiToken(stored.controlPlaneApiToken);
    if (stored.clusterApiToken) setClusterApiToken(stored.clusterApiToken);
    if (stored.directMeshApiToken) setDirectMeshApiToken(stored.directMeshApiToken);
    if (stored.assistantMode === "local" || stored.assistantMode === "llm") {
      setAssistantMode(stored.assistantMode);
    }
    if (typeof stored.assistantApiBaseUrl === "string" && stored.assistantApiBaseUrl.trim()) {
      setAssistantApiBaseUrl(stored.assistantApiBaseUrl);
    }
    if (typeof stored.assistantApiKey === "string") setAssistantApiKey(stored.assistantApiKey);
    if (typeof stored.assistantModel === "string" && stored.assistantModel.trim()) {
      setAssistantModel(stored.assistantModel);
    }
    const bootLanguage =
      desktopLanguage === "en" || desktopLanguage === "zh" || desktopLanguage === "ja" || desktopLanguage === "es"
        ? desktopLanguage
        : stored.language === "en" || stored.language === "zh" || stored.language === "ja" || stored.language === "es"
          ? stored.language
          : null;

    if (bootLanguage) {
      applyLanguagePreference(bootLanguage);
      setMessage(copyByLanguage[bootLanguage].initialLoaded);
    }
    setLanguagePacks(readWorkbenchLanguagePacks());
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;

    const handleDesktopLanguage = (event: MessageEvent) => {
      if (event.data?.type !== "kyuubiki:set-language") return;
      const nextLanguage = event.data.language;
      if (nextLanguage === "en" || nextLanguage === "zh" || nextLanguage === "ja" || nextLanguage === "es") {
        applyLanguagePreference(nextLanguage);
      }
    };

    window.addEventListener("message", handleDesktopLanguage);
    return () => {
      window.removeEventListener("message", handleDesktopLanguage);
    };
  }, []);

  useEffect(() => {
    if (studyKind === "torsion_1d" && frameResultField === "max_combined_stress") {
      setFrameResultField("max_bending_stress");
    }
    if (
      studyKind !== "thermal_frame_2d" &&
      (frameResultField === "average_temperature_delta" ||
        frameResultField === "temperature_gradient_y" ||
        frameResultField === "thermal_curvature")
    ) {
      setFrameResultField("max_combined_stress");
    }
  }, [frameResultField, studyKind]);

  useEffect(() => {
    if (
      studyKind !== "thermal_beam_1d" &&
      (beamResultField === "temperature_gradient_y" || beamResultField === "thermal_curvature")
    ) {
      setBeamResultField("max_bending_stress");
    }
  }, [beamResultField, studyKind]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    if (typeof window !== "undefined") {
      window.parent?.postMessage({ type: "kyuubiki:language-changed", language }, "*");
      persistWorkbenchSettings({
        theme,
        language,
        showShortcutHints,
        immersiveGuardrails,
        frontendRuntimeMode,
        directMeshEndpointsText,
        directMeshSelectionMode,
        controlPlaneApiToken,
        clusterApiToken,
        directMeshApiToken,
        assistantMode,
        assistantApiBaseUrl,
        assistantApiKey,
        assistantModel,
      });
    }
  }, [theme, language, showShortcutHints, immersiveGuardrails, frontendRuntimeMode, directMeshEndpointsText, directMeshSelectionMode, controlPlaneApiToken, clusterApiToken, directMeshApiToken, assistantMode, assistantApiBaseUrl, assistantApiKey, assistantModel]);

  useEffect(() => {
    persistWorkbenchLanguagePacks(languagePacks);
  }, [languagePacks]);

  useEffect(() => {
    return () => {
      if (dragFrameRef.current !== null) {
        window.cancelAnimationFrame(dragFrameRef.current);
      }
    };
  }, []);

  useEffect(() => {
    return () => {
      jobPollTokenRef.current += 1;
    };
  }, []);

  useEffect(() => {
    const handleFullscreenChange = () => {
      setImmersiveViewport(document.fullscreenElement === viewportPanelRef.current);
    };

    document.addEventListener("fullscreenchange", handleFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", handleFullscreenChange);
  }, []);

  useEffect(() => {
    if (studyKind === "truss_3d") return;

    setImmersiveToolDrawerOpen(false);
    setImmersiveHelpDrawerOpen(false);

    if (immersiveViewport) {
      setImmersiveViewport(false);
    }

    if (document.fullscreenElement === viewportPanelRef.current) {
      void document.exitFullscreen().catch(() => undefined);
    }
  }, [studyKind, immersiveViewport]);

  useEffect(() => {
    if (!immersiveViewport || !immersiveGuardrails) {
      document.documentElement.classList.remove("immersive-guardrails");
      return;
    }

    const stopEvent = (event: Event) => {
      event.preventDefault();
      event.stopPropagation();
    };

    const handleKeydown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && ["a", "c", "p", "s", "u", "v", "x"].includes(event.key.toLowerCase())) {
        stopEvent(event);
      }
    };

    document.documentElement.classList.add("immersive-guardrails");
    document.addEventListener("contextmenu", stopEvent, true);
    document.addEventListener("copy", stopEvent, true);
    document.addEventListener("cut", stopEvent, true);
    document.addEventListener("paste", stopEvent, true);
    document.addEventListener("selectstart", stopEvent, true);
    document.addEventListener("dragstart", stopEvent, true);
    document.addEventListener("keydown", handleKeydown, true);

    return () => {
      document.documentElement.classList.remove("immersive-guardrails");
      document.removeEventListener("contextmenu", stopEvent, true);
      document.removeEventListener("copy", stopEvent, true);
      document.removeEventListener("cut", stopEvent, true);
      document.removeEventListener("paste", stopEvent, true);
      document.removeEventListener("selectstart", stopEvent, true);
      document.removeEventListener("dragstart", stopEvent, true);
      document.removeEventListener("keydown", handleKeydown, true);
    };
  }, [immersiveViewport, immersiveGuardrails]);

  useEffect(() => {
    const current = jobHistory.find((entry) => entry.job_id === selectedAdminJobId) ?? null;
    setAdminJobMessage(current?.message ?? "");
    setAdminJobProjectId(current?.project_id ?? "");
    setAdminJobModelVersionId(current?.model_version_id ?? "");
    setAdminJobCaseId(current?.simulation_case_id ?? "");
  }, [jobHistory, selectedAdminJobId]);

  useEffect(() => {
    const current = resultRecords.find((entry) => entry.job_id === selectedAdminResultJobId) ?? null;
    setAdminResultDraft(JSON.stringify(current?.result ?? {}, null, 2));
  }, [resultRecords, selectedAdminResultJobId]);

  useEffect(() => {
    const selectedProject = projects.find((project) => project.project_id === selectedProjectId) ?? null;

    if (selectedProject) {
      setProjectNameDraft(selectedProject.name);
      setProjectDescriptionDraft(selectedProject.description ?? "");
    } else if (projects.length === 0) {
      setProjectNameDraft(t.defaultProject);
      setProjectDescriptionDraft("");
    }
  }, [projects, selectedProjectId, t.defaultProject]);

  const {
    workflowCatalog,
    workflowCatalogBusy,
    workflowPanelTab,
    setWorkflowPanelTab,
    selectedWorkflowId,
    setSelectedWorkflowId,
    workflowRuns,
    setWorkflowRuns,
    selectedWorkflow,
    latestWorkflowSummary,
    refreshWorkflowCatalog,
    runWorkflowCatalogEntry,
  } = useWorkbenchWorkflowController({
    labels: {
      workflowCatalogLoaded: t.workflowCatalogLoaded,
      workflowCatalogUnsupported: t.workflowCatalogUnsupported,
      workflowCatalogQueued: t.workflowCatalogQueued,
      workflowCatalogCompleted: t.workflowCatalogCompleted,
      workflowCatalogFailed: t.workflowCatalogFailed,
      initialFailed: t.initialFailed,
      pollingDetached: t.pollingDetached,
    },
    jobPollTokenRef,
    refreshJobHistory,
    setJob,
    setMessage,
    openWorkflowRunsSurface: (workflowId) => {
      setSelectedWorkflowId(workflowId);
      setSidebarSection("workflow");
      setWorkflowPanelTab("runs");
    },
  });

  async function refreshResults() {
    await refreshWorkbenchResults({
      resultRefreshSeqRef,
      fetchResults,
      setResultRecords,
      setSelectedAdminResultJobId,
    });
  }
  const {
    refreshHealth,
    refreshProjects,
    refreshSecurityEvents,
    refreshVersions,
  } = useWorkbenchDataRefreshController({
    directMeshEndpointsText,
    directMeshSelectionMode,
    frontendRuntimeMode,
    securityEventActionFilter,
    securityEventRiskFilter,
    securityEventSourceFilter,
    securityEventStatusFilter,
    securityEventWindowFilter,
    selectedModelId,
    selectedProjectId,
    setHealth,
    setModelVersions,
    setProjects,
    setProtocolAgents,
    setSecurityEventRecords,
    setSelectedModelId,
    setSelectedProjectId,
    setSelectedVersionId,
    refreshJobHistory,
    refreshResults,
    securityEventWindowMs: SECURITY_EVENT_WINDOW_MS,
  });

  const runAnalysis = () => {
    startTransition(async () => {
      try {
        await runWorkbenchAnalysis({
          axialForm,
          beamModel,
          copy: t,
          directMeshEndpointsText,
          directMeshSelectionMode,
          frontendRuntimeMode,
          frameModel,
          heatBarModel,
          heatPlaneModel,
          jobPollTokenRef,
          labels: {
            precheckPrefix: t.precheckPrefix,
            dispatching: t.dispatching,
            directMeshEndpointsHelp: t.directMeshEndpointsHelp,
            directMeshCompleted: t.directMeshCompleted,
            requestTimedOut: t.requestTimedOut,
            initialFailed: t.initialFailed,
            pollingDetached: t.pollingDetached,
          },
          planeModel,
          refreshJobHistory,
          selectedProjectId,
          selectedVersionId,
          setDirectMeshExecution,
          setJob,
          setMessage,
          setResult,
          spring2dModel,
          spring3dModel,
          springModel,
          studyKind,
          thermalBarModel,
          thermalBeamModel,
          thermalFrameModel,
          thermalTruss3dModel,
          thermalTrussModel,
          torsionModel,
          truss3dModel,
          trussDiagnostics,
          trussModel,
        });
      } catch (error) {
        setMessage(
          error instanceof Error
            ? error.message.startsWith("request timed out:")
              ? t.requestTimedOut
              : error.message
            : t.initialFailed,
        );
      }
    });
  };

  const openHistoryJob = (jobId: string) => {
    jobPollTokenRef.current += 1;

    startTransition(async () => {
      try {
        const payload = await fetchJobStatus<
          AxialBarResult | HeatBar1dResult | HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult | ThermalBar1dResult | ThermalBeam1dResult | ThermalTruss2dResult | ThermalTruss3dResult | Spring1dResult | Spring2dResult | Spring3dResult | Beam1dResult | Torsion1dResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | PlaneQuad2dResult | Frame2dResult | ThermalFrame2dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult | WorkflowGraphJobResult
        >(jobId);
        applyHistoryJobPayload(payload, {
          activeMaterial,
          copy: {
            historyAction: t.historyAction,
            historyLoaded: t.historyLoaded,
            workflowCatalogCompleted: t.workflowCatalogCompleted,
          },
          setJob,
          setResult,
          setSidebarSection,
          setWorkflowPanelTab,
          setSelectedWorkflowId,
          setWorkflowRuns,
          setMessage,
          recordHistory,
          openWorkspaceStudy,
          setStudyKind,
          setAxialForm,
          setThermalBarModel,
          setHeatBarModel,
          setHeatPlaneModel,
          setPlaneResultField,
          setThermalBeamModel,
          setThermalTrussModel,
          setThermalTruss3dModel,
          setSpringModel,
          setSpring2dModel,
          setSpring3dModel,
          setBeamModel,
          setTorsionModel,
          setTrussModel,
          setTruss3dModel,
          setFrameModel,
          setThermalFrameModel,
          setPlaneModel,
        });
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const importModel = async (file: File | undefined) => {
    await importWorkbenchModelFile({
      file,
      labels: {
        importAction: t.importAction,
        importedModel: t.importedModel,
        importFailed: t.importFailed,
      },
      recordHistory,
      applyImportedModel: {
        setLoadedModelName,
        setSelectedModelId,
        setSelectedVersionId,
        setModelVersions,
        setStudyKind,
        setAxialForm,
        setHeatBarModel,
        setHeatPlaneModel,
        setThermalBarModel,
        setThermalBeamModel,
        setThermalFrameModel,
        setThermalTrussModel,
        setThermalTruss3dModel,
        setSpringModel,
        setSpring2dModel,
        setSpring3dModel,
        setTrussModel,
        setTruss3dModel,
        setPlaneModel,
        setFrameModel,
        setBeamModel,
        setTorsionModel,
        setPlaneResultField,
        setParametric,
        setActiveMaterial,
      },
      setMessage,
    });
  };

  const openSample = (href: string) => {
    startTransition(async () => {
      await openWorkbenchSample({
        href,
        labels: {
          sampleAction: t.sampleAction,
          importedModel: t.importedModel,
          importFailed: t.importFailed,
          requestTimedOut: t.requestTimedOut,
        },
        recordHistory,
        applyImportedModel: {
          setLoadedModelName,
          setSelectedModelId,
          setSelectedVersionId,
          setModelVersions,
          setStudyKind,
          setAxialForm,
          setHeatBarModel,
          setHeatPlaneModel,
          setThermalBarModel,
          setThermalBeamModel,
          setThermalFrameModel,
          setThermalTrussModel,
          setThermalTruss3dModel,
          setSpringModel,
          setSpring2dModel,
          setSpring3dModel,
          setTrussModel,
          setTruss3dModel,
          setPlaneModel,
          setFrameModel,
          setBeamModel,
          setTorsionModel,
          setPlaneResultField,
          setParametric,
          setActiveMaterial,
        },
        setMessage,
      });
    });
  };

  const handleAxialFieldChange = (key: keyof AxialFormState, value: number | string) => {
    recordHistory(t.editAxialField);
    setAxialForm((current) => ({ ...current, [key]: value }));
  };

  const handleMaterialChange = (value: string) => {
    recordHistory(t.editMaterial);
    const preset = MATERIAL_PRESETS.find((item) => item.value === value);
    setActiveMaterial(value);
    setAxialForm((current) => ({
      ...current,
      material: value,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
    setParametric((current) => ({
      ...current,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
  };

  const handleLanguageChange = (nextLanguage: Language) => {
    applyLanguagePreference(nextLanguage);
  };

  const handleDownloadLanguagePackTemplate = () => {
    downloadWorkbenchLanguagePackTemplate({ language, copy: t, setMessage });
  };

  const handleExportInstalledLanguagePack = () => {
    exportWorkbenchInstalledLanguagePack({ language, activeLanguagePack, setMessage });
  };

  const handleImportLanguagePack = async (file: File) => {
    await importWorkbenchLanguagePack({ file, language, setLanguagePacks, setMessage });
  };

  const handleRemoveLanguagePack = (packId: string) => {
    removeWorkbenchLanguagePack({ packId, setLanguagePacks, language, setMessage });
  };

  const handleParametricChange = (key: keyof ParametricTrussConfig, value: number) => {
    recordHistory(t.editParametric);
    setParametric((current) => ({ ...current, [key]: value }));
  };

  const handlePanelParametricChange = (key: keyof ParametricPanelConfig, value: number) => {
    recordHistory(t.editParametric);
    setPanelParametric((current) => ({ ...current, [key]: value }));
  };

  const generateModel = () => {
    recordHistory(t.generateAction);
    const nextModel = ensureTrussModelMaterials(generatePrattTruss(parametric), activeMaterial);
    setStudyKind("truss_2d");
    setTrussModel(nextModel);
    setSelectedNode(null);
    setSelectedElement(null);
    setSelectedModelId(null);
    setSelectedVersionId(null);
    setModelVersions([]);
    setMemberDraftNodes([]);
    setLoadedModelName("parametric-pratt-truss");
    setMessage(t.generatedModel);
    setSidebarSection("model");
  };

  const generatePanelModel = () => {
    recordHistory(t.generateAction);
    const nextModel = ensurePlaneModelMaterials(
      studyKind === "plane_quad_2d"
        ? generateRectangularQuadPanelMesh(panelParametric)
        : generateRectangularPanelMesh(panelParametric),
      activeMaterial,
    );
    setStudyKind(studyKind === "plane_quad_2d" ? "plane_quad_2d" : "plane_triangle_2d");
    setPlaneModel(nextModel);
    setSelectedNode(null);
    setSelectedElement(null);
    setSelectedModelId(null);
    setSelectedVersionId(null);
    setModelVersions([]);
    setMemberDraftNodes([]);
    setLoadedModelName("parametric-panel-mesh");
    setMessage(t.panelGenerated);
    setSidebarSection("model");
    resetActiveResult(setResult, setJob);
  };

  const downloadModel = () => {
    const contents = JSON.stringify(
      serializeCurrentModel(
        studyKind,
        loadedModelName,
        activeMaterial,
        axialForm,
        heatBarModel,
        heatPlaneModel,
        thermalBarModel,
        thermalBeamModel,
        thermalFrameModel,
        thermalTrussModel,
        trussModel,
        thermalTruss3dModel,
        truss3dModel,
        planeModel,
        frameModel,
        beamModel,
        torsionModel,
        springModel,
        spring2dModel,
        spring3dModel,
        parametric,
        round,
      ),
      null,
      2,
    );

    downloadTextFile(`${loadedModelName || "kyuubiki-model"}.json`, contents);
    setMessage(t.modelDownloaded);
  };

  const downloadResultJson = () => {
    downloadWorkbenchResultJson(resultExportEffects);
  };

  const downloadResultCsv = () => {
    downloadWorkbenchResultCsv(resultExportEffects);
  };

  const downloadPlaneHotspotSummary = () => {
    downloadWorkbenchPlaneHotspotSummary(resultExportEffects);
  };

  const downloadFrameHotspotSummary = () => {
    downloadWorkbenchFrameHotspotSummary(resultExportEffects);
  };

  const downloadFrameForceSummary = () => {
    downloadWorkbenchFrameForceSummary(resultExportEffects);
  };

  const buildProjectBundleJson = async () => {
    if (!selectedProject) {
      throw new Error(t.projectRequired);
    }

    const modelDetailsSettled = await Promise.allSettled(
      selectedProjectModels.map(async (model) => {
        const modelEnvelope = await fetchModel(model.model_id);
        const versionsEnvelope = await fetchModelVersions(model.model_id);
        return {
          model: modelEnvelope.model,
          versions: versionsEnvelope.versions,
        };
      }),
    );

    const modelDetails = modelDetailsSettled.flatMap((entry) => (entry.status === "fulfilled" ? [entry.value] : []));

    const resultCandidatesSettled = await Promise.allSettled(
      jobHistory
        .filter((historyJob) => historyJob.has_result)
        .map(async (historyJob) => {
          try {
            const payload = await fetchJobStatus(historyJob.job_id);

            if (!payload.result) {
              return null;
            }

            return {
              job_id: historyJob.job_id,
              status: payload.job.status,
              worker_id: payload.job.worker_id,
              result: payload.result,
            };
          } catch {
            return null;
          }
        }),
    );

    const resultCandidates = resultCandidatesSettled.flatMap((entry): Array<JobResultRecord | null> =>
      entry.status === "fulfilled" ? [entry.value as JobResultRecord | null] : [],
    );
    const results = resultCandidates.filter((entry): entry is JobResultRecord => entry !== null);
    const partial =
      modelDetails.length !== selectedProjectModels.length ||
      resultCandidatesSettled.some((entry) => entry.status === "rejected");

    return {
      bundle: exportProjectBundle({
        project: selectedProject,
        models: modelDetails.map((entry) => entry.model),
        modelVersions: modelDetails.flatMap((entry) => entry.versions),
        activeModelId: selectedModelId,
        activeVersionId: selectedVersionId,
        workspaceSnapshot: serializeCurrentModel(
          studyKind,
          loadedModelName,
          activeMaterial,
          axialForm,
          heatBarModel,
          heatPlaneModel,
          thermalBarModel,
          thermalBeamModel,
          thermalFrameModel,
          thermalTrussModel,
          trussModel,
          thermalTruss3dModel,
          truss3dModel,
          planeModel,
          frameModel,
          beamModel,
          torsionModel,
          springModel,
          spring2dModel,
          spring3dModel,
          parametric,
          round,
        ),
        automationPresets: listWorkbenchMacroPresets(selectedProject.project_id),
        jobs: jobHistory,
        results,
      }),
      partial,
    };
  };

  const downloadProjectBundleJson = async () => {
    await downloadWorkbenchProjectBundleJson({
      selectedProject,
      buildBundle: buildProjectBundleJson,
      setMessage,
      labels: {
        projectExported: t.projectExported,
        projectExportedPartial: t.projectExportedPartial,
        initialFailed: t.initialFailed,
      },
    });
  };

  const downloadProjectBundleZip = async () => {
    await downloadWorkbenchProjectBundleZip({
      selectedProject,
      buildBundle: buildProjectBundleJson,
      setMessage,
      labels: {
        projectExported: t.projectExported,
        projectExportedPartial: t.projectExportedPartial,
        initialFailed: t.initialFailed,
      },
    });
  };

  const downloadDatabaseSnapshot = async () => {
    await downloadWorkbenchDatabaseSnapshot({
      setMessage,
      labels: {
        databaseExported: t.databaseExported,
        initialFailed: t.initialFailed,
      },
    });
  };

  const downloadSecurityEventExport = async () => {
    await downloadWorkbenchSecurityEventExport({
      language,
      securityEventWindowFilter,
      securityEventSourceFilter,
      securityEventRiskFilter,
      securityEventStatusFilter,
      securityEventActionFilter,
      setMessage,
      labels: { initialFailed: t.initialFailed },
    });
  };

  const downloadSecurityEventCsvExport = async () => {
    await downloadWorkbenchSecurityEventCsvExport({
      language,
      securityEventWindowFilter,
      securityEventSourceFilter,
      securityEventRiskFilter,
      securityEventStatusFilter,
      securityEventActionFilter,
      setMessage,
      labels: { initialFailed: t.initialFailed },
    });
  };

  const saveAdminJobRecord = () => {
    if (!selectedAdminJobId) return;

    startTransition(async () => {
      try {
        await updateJobRecord(selectedAdminJobId, {
          message: adminJobMessage,
          project_id: adminJobProjectId || undefined,
          model_version_id: adminJobModelVersionId || undefined,
          simulation_case_id: adminJobCaseId || undefined,
        });
        await refreshJobHistory();
        setMessage(t.jobSaved);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteAdminJobRecord = () => {
    if (!selectedAdminJobId) return;

    startTransition(async () => {
      try {
        await deleteJobRecord(selectedAdminJobId);
        await refreshJobHistory();
        await refreshResults();
        setMessage(t.jobDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const saveAdminResultRecord = () => {
    if (!selectedAdminResultJobId) return;

    startTransition(async () => {
      await saveWorkbenchAdminResultRecord({
        resultRefreshSeqRef,
        fetchResults,
        setResultRecords,
        setSelectedAdminResultJobId,
        selectedAdminResultJobId,
        adminResultDraft,
        updateResultRecord,
        deleteResultRecord,
        downloadTextFile,
        setMessage,
        labels: {
          resultSaved: t.resultSaved,
          resultDeleted: t.resultDeleted,
          resultJsonDownloaded: t.resultJsonDownloaded,
          invalidJson: t.invalidJson,
          initialFailed: t.initialFailed,
        },
      });
    });
  };

  const deleteAdminResultRecord = () => {
    if (!selectedAdminResultJobId) return;

    startTransition(async () => {
      await deleteWorkbenchAdminResultRecord({
        resultRefreshSeqRef,
        fetchResults,
        setResultRecords,
        setSelectedAdminResultJobId,
        selectedAdminResultJobId,
        adminResultDraft,
        updateResultRecord,
        deleteResultRecord,
        downloadTextFile,
        setMessage,
        labels: {
          resultSaved: t.resultSaved,
          resultDeleted: t.resultDeleted,
          resultJsonDownloaded: t.resultJsonDownloaded,
          invalidJson: t.invalidJson,
          initialFailed: t.initialFailed,
        },
      });
    });
  };

  const exportAdminResultRecord = () => {
    exportWorkbenchAdminResultRecord({
      resultRefreshSeqRef,
      fetchResults,
      setResultRecords,
      setSelectedAdminResultJobId,
      selectedAdminResultJobId,
      adminResultDraft,
      updateResultRecord,
      deleteResultRecord,
      downloadTextFile,
      setMessage,
      labels: {
        resultSaved: t.resultSaved,
        resultDeleted: t.resultDeleted,
        resultJsonDownloaded: t.resultJsonDownloaded,
        invalidJson: t.invalidJson,
        initialFailed: t.initialFailed,
      },
    });
  };

  const importProjectBundle = async (file: File | undefined) => {
    await importWorkbenchProjectBundle(file, persistedModelEffects);
  };

  const createProjectRecord = () => {
    startTransition(async () => {
      try {
        const payload = await createProject({
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        setSelectedProjectId(payload.project.project_id);
        await refreshProjects();
        setMessage(t.projectCreated);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const updateProjectRecord = () => {
    if (!selectedProjectId) return;

    startTransition(async () => {
      try {
        await updateProject(selectedProjectId, {
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        await refreshProjects();
        setMessage(t.projectUpdated);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteProjectRecord = () => {
    if (!selectedProjectId) return;
    if (typeof window !== "undefined" && !window.confirm(projectNameDraft)) return;

    startTransition(async () => {
      try {
        await deleteProject(selectedProjectId);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        await refreshProjects();
        setMessage(t.projectDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const saveModelVersion = (saveAs: boolean) => {
    if (!selectedProjectId) {
      setMessage(t.projectRequired);
      return;
    }

    const payload = serializeCurrentModel(
      studyKind,
      loadedModelName,
      activeMaterial,
      axialForm,
      heatBarModel,
      heatPlaneModel,
      thermalBarModel,
      thermalBeamModel,
      thermalFrameModel,
      thermalTrussModel,
      trussModel,
      thermalTruss3dModel,
      truss3dModel,
      planeModel,
      frameModel,
      beamModel,
      torsionModel,
      springModel,
      spring2dModel,
      spring3dModel,
      parametric,
      round,
    );

    startTransition(async () => {
      try {
        if (!selectedModelId || saveAs) {
          const created = await createModel(selectedProjectId, {
            name: loadedModelName,
            kind: studyKind,
            material: activeMaterial,
            model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
            payload,
          });
          setSelectedModelId(created.model.model_id);
          setSelectedVersionId(created.model.latest_version_id ?? null);
          await refreshProjects();
          await refreshVersions(created.model.model_id);
          setMessage(t.modelCreated);
          return;
        }

        await updateModel(selectedModelId, {
          name: loadedModelName,
          kind: studyKind,
          material: activeMaterial,
          model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
          payload,
        });

        const version = await createModelVersion(selectedModelId, {
          name: loadedModelName,
          kind: studyKind,
          material: activeMaterial,
          model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
          payload,
        });

        setSelectedVersionId(version.version.version_id);
        await refreshProjects();
        await refreshVersions(selectedModelId);
        setMessage(t.modelSaved);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const openSavedModel = (model: ModelRecord) => {
    openPersistedWorkbenchModel(model, persistedModelEffects);
  };

  const openSavedVersion = (version: ModelVersionRecord) => {
    openPersistedWorkbenchVersion(version, persistedModelEffects);
  };

  const openModelVersionById = (versionId: string) => {
    openPersistedWorkbenchVersionById(versionId, persistedModelEffects);
  };

  const renameSelectedVersion = () => {
    if (!selectedVersionId) return;

    startTransition(async () => {
      try {
        await updateModelVersion(selectedVersionId, { name: loadedModelName });
        await refreshVersions(selectedModelId ?? "");
        setMessage(t.versionRenamed);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSelectedVersion = () => {
    if (!selectedVersionId) return;

    startTransition(async () => {
      try {
        await deleteModelVersion(selectedVersionId);
        setSelectedVersionId(null);
        if (selectedModelId) {
          await refreshVersions(selectedModelId);
        }
        await refreshProjects();
        setMessage(t.versionDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const openSelectedAdminJobVersion = () => {
    openSelectedAdminJobVersionWithDeps(adminDataEffects);
  };

  const openSelectedAdminResultVersion = () => {
    openSelectedAdminResultVersionWithDeps(adminDataEffects);
  };

  const openSelectedAdminJobProject = () => {
    openSelectedAdminJobProjectWithDeps(adminDataEffects);
  };

  const openSelectedAdminResultProject = () => {
    openSelectedAdminResultProjectWithDeps(adminDataEffects);
  };

  const applySelectedAdminJobContext = () => {
    applySelectedAdminJobContextWithDeps(adminDataEffects);
  };

  const applySelectedAdminResultContext = () => {
    applySelectedAdminResultContextWithDeps(adminDataEffects);
  };

  const handleSidebarSectionChange = (section: SidebarSection) => {
    const nextSection = section === "study" ? "model" : section;
    setSidebarSection(nextSection);
    if (nextSection === "model") {
      setModelTab("tools");
    }
    if (nextSection === "workflow" && workflowCatalog.length === 0 && !workflowCatalogBusy) {
      void refreshWorkflowCatalog();
    }
    recordManualDslAction("nav/setSidebarSection", { section: nextSection });
  };

  const handleStudyTabChange = (tab: StudyPanelTab) => {
    setStudyTab(tab);
    recordManualDslAction("nav/setTabs", { studyTab: tab });
  };

  const handleModelTabChange = (tab: ModelPanelTab) => {
    setModelTab(tab);
    recordManualDslAction("nav/setTabs", { modelTab: tab });
  };

  const handleModelToolsPageChange = (page: ModelToolsPage) => {
    setModelToolsPage(page);
    recordManualDslAction("nav/setTabs", { modelTab, modelToolsPage: page });
  };

  const handleLibraryTabChange = (tab: LibraryPanelTab) => {
    setLibraryTab(tab);
    if (tab === "samples" && workflowCatalog.length === 0 && !workflowCatalogBusy) {
      void refreshWorkflowCatalog();
    }
    recordManualDslAction("nav/setTabs", { libraryTab: tab });
  };

  const handleWorkflowPanelTabChange = (tab: WorkflowPanelTab) => {
    setWorkflowPanelTab(tab);
    if ((tab === "catalog" || tab === "builder") && workflowCatalog.length === 0 && !workflowCatalogBusy) {
      void refreshWorkflowCatalog();
    }
    recordManualDslAction("nav/setTabs", { workflowPanelTab: tab });
  };

  const handleSystemPanelTabChange = (tab: SystemPanelTab) => {
    setSystemPanelTab(tab);
    recordManualDslAction("nav/setTabs", { systemPanelTab: tab });
  };

  const handleSystemDataTabChange = (tab: SystemDataTab) => {
    setSystemDataTab(tab);
    recordManualDslAction("nav/setTabs", { systemPanelTab: "data", systemDataTab: tab });
  };

  const handleAdminFilterProjectChange = (value: string) => {
    setAdminFilterProjectId(value);
    recordManualDslAction("data/setFilters", { activeTab: systemDataTab, projectId: value, modelVersionId: adminFilterModelVersionId });
  };

  const handleAdminFilterModelVersionChange = (value: string) => {
    setAdminFilterModelVersionId(value);
    recordManualDslAction("data/setFilters", { activeTab: systemDataTab, projectId: adminFilterProjectId, modelVersionId: value });
  };

  const handleSelectAdminJob = (jobId: string) => {
    setSelectedAdminJobId(jobId);
    recordManualDslAction("data/selectRecord", { activeTab: "jobs", jobId });
  };

  const handleSelectAdminResult = (jobId: string) => {
    setSelectedAdminResultJobId(jobId);
    recordManualDslAction("data/selectRecord", { activeTab: "results", resultJobId: jobId });
  };

  const handleTruss3dViewPresetChange = (preset: "iso" | "front" | "right" | "top") => {
    setTruss3dViewPreset(preset);
    recordManualDslAction("viewport/set3dView", { preset });
  };

  const handleTruss3dProjectionModeChange = (mode: "ortho" | "persp") => {
    setTruss3dProjectionMode(mode);
    recordManualDslAction("viewport/set3dView", { projection: mode });
  };

  const handleTruss3dBoxSelectModeChange = (next: boolean) => {
    setTruss3dBoxSelectMode(next);
    recordManualDslAction("viewport/setUiState", { boxSelectMode: next });
  };

  const handleTruss3dShowGridChange = (next: boolean) => {
    setTruss3dShowGrid(next);
    recordManualDslAction("viewport/toggleFlags", { grid: next });
  };

  const handleTruss3dShowLabelsChange = (next: boolean) => {
    setTruss3dShowLabels(next);
    recordManualDslAction("viewport/toggleFlags", { labels: next });
  };

  const handleTruss3dShowNodesChange = (next: boolean) => {
    setTruss3dShowNodes(next);
    recordManualDslAction("viewport/toggleFlags", { nodes: next });
  };

  const handleToggleTruss3dLinkMode = () => {
    toggleTruss3dLinkMode();
    recordManualDslAction("viewport/setUiState", { linkMode: !truss3dLinkMode });
  };

  const handleToggleImmersiveViewport = async () => {
    await toggleImmersiveViewport();
    recordManualDslAction("viewport/setUiState", { immersiveViewport: !immersiveViewport });
  };

  const handleToggleImmersiveToolDrawer = () => {
    setImmersiveToolDrawerOpen((current) => {
      const next = !current;
      recordManualDslAction("viewport/setUiState", { toolDrawerOpen: next });
      return next;
    });
  };

  const handleToggleImmersiveHelpDrawer = () => {
    setImmersiveHelpDrawerOpen((current) => {
      const next = !current;
      recordManualDslAction("viewport/setUiState", { helpDrawerOpen: next });
      return next;
    });
  };

  const handleTruss3dFocusViewport = () => {
    setTruss3dFocusRequestVersion((current) => current + 1);
    recordManualDslAction("viewport/focus3d", {});
  };

  const handleTruss3dResetViewport = () => {
    setTruss3dResetRequestVersion((current) => current + 1);
    recordManualDslAction("viewport/reset3d", {});
  };

  const useCurrentProjectAsAdminFilter = () => {
    setAdminFilterProjectId(selectedProjectId ?? "");
    recordManualDslAction("data/setFilters", {
      activeTab: systemDataTab,
      projectId: selectedProjectId ?? "",
      modelVersionId: adminFilterModelVersionId,
    });
  };

  const useCurrentVersionAsAdminFilter = () => {
    setAdminFilterModelVersionId(selectedVersionId ?? "");
    recordManualDslAction("data/setFilters", {
      activeTab: systemDataTab,
      projectId: adminFilterProjectId,
      modelVersionId: selectedVersionId ?? "",
    });
  };

  const clearAdminFilters = () => {
    setAdminFilterProjectId("");
    setAdminFilterModelVersionId("");
    recordManualDslAction("data/setFilters", { activeTab: systemDataTab, projectId: "", modelVersionId: "" });
  };

  const deleteSavedModelRecord = () => {
    if (!selectedModelId) return;

    startTransition(async () => {
      try {
        await deleteModel(selectedModelId);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
        await refreshProjects();
        setMessage(t.modelDeletedStored);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const railItems: Array<{ key: SidebarSection; label: string; symbol: string }> = [
    { key: "model", label: t.rail.model, symbol: "M" },
    { key: "workflow", label: t.rail.workflow, symbol: "W" },
    { key: "library", label: t.rail.library, symbol: "H" },
    { key: "system", label: t.rail.system, symbol: "Y" },
  ];

  const selectedProject = projects.find((project) => project.project_id === selectedProjectId) ?? null;
  const selectedProjectModels = selectedProject?.models ?? [];
  const deferredProjectModels = useDeferredValue(selectedProjectModels);
  const deferredModelVersions = useDeferredValue(modelVersions);
  const deferredJobHistory = useDeferredValue(jobHistory);
  const deferredResultRecords = useDeferredValue(resultRecords);
  const selectedAdminJob = jobHistory.find((entry) => entry.job_id === selectedAdminJobId) ?? null;
  const selectedAdminResult = resultRecords.find((entry) => entry.job_id === selectedAdminResultJobId) ?? null;
  const adminDataEffects = {
    selectedAdminJob,
    selectedAdminJobId,
    selectedAdminResultJobId,
    jobHistory,
    projects,
    refreshVersions,
    openModelVersionById,
    setAdminFilterProjectId,
    setAdminFilterModelVersionId,
    setAdminJobCaseId,
    setLibraryTab,
    setSelectedProjectId,
    setSelectedModelId,
    setSelectedVersionId,
    setModelVersions,
    setSidebarSection,
    setMessage,
    labels: {
      noJobVersion:
        language === "zh"
          ? "这个任务还没有关联模型版本。"
          : language === "ja"
            ? "このジョブには関連するモデルバージョンがまだありません。"
            : "This job does not have a linked model version.",
      noResultVersion:
        language === "zh"
          ? "这个结果还没有关联模型版本。"
          : language === "ja"
            ? "この結果には関連するモデルバージョンがまだありません。"
            : "This result does not have a linked model version.",
      noRecordContext:
        language === "zh"
          ? "这条记录还没有可应用的项目或版本上下文。"
          : language === "ja"
            ? "このレコードには適用できる project / version の文脈がまだありません。"
            : "This record does not have a linked project or version context yet.",
      linkedProjectMissing:
        language === "zh"
          ? "找不到关联项目。"
          : language === "ja"
            ? "関連プロジェクトが見つかりませんでした。"
            : "Could not find the linked project.",
      linkedProjectOpened: t.linkedProjectOpened,
      noJobProject:
        language === "zh"
          ? "这个任务还没有关联项目。"
          : language === "ja"
            ? "このジョブには関連プロジェクトがまだありません。"
            : "This job does not have a linked project.",
      noResultProject:
        language === "zh"
          ? "这个结果还没有关联项目。"
          : language === "ja"
            ? "この結果には関連プロジェクトがまだありません。"
            : "This result does not have a linked project.",
      selectJobFirst:
        language === "zh"
          ? "请先选择一条任务记录。"
          : language === "ja"
            ? "先にジョブレコードを選択してください。"
            : "Select a job record first.",
      missingResultJob:
        language === "zh"
          ? "找不到这条结果对应的任务记录。"
          : language === "ja"
            ? "この結果に対応するジョブレコードが見つかりませんでした。"
            : "Could not find the job record linked to this result.",
      recordContextApplied: t.recordContextApplied,
    },
  };

  const isAxial = studyKind === "axial_bar_1d";
  const isHeatBar = studyKind === "heat_bar_1d";
  const isHeatPlaneTriangle = studyKind === "heat_plane_triangle_2d";
  const isHeatPlaneQuad = studyKind === "heat_plane_quad_2d";
  const isHeatPlane = isHeatPlaneTriangle || isHeatPlaneQuad;
  const isThermalBar = studyKind === "thermal_bar_1d";
  const isThermalBeam = studyKind === "thermal_beam_1d";
  const isThermalFrame = studyKind === "thermal_frame_2d";
  const isThermalTruss2d = studyKind === "thermal_truss_2d";
  const isThermalTruss3d = studyKind === "thermal_truss_3d";
  const isThermalPlaneTriangle = studyKind === "thermal_plane_triangle_2d";
  const isThermalPlaneQuad = studyKind === "thermal_plane_quad_2d";
  const isThermal = isThermalBar || isThermalTruss2d || isThermalTruss3d;
  const isSpring1d = studyKind === "spring_1d";
  const isSpring2d = studyKind === "spring_2d";
  const isSpring3d = studyKind === "spring_3d";
  const isSpring = isSpring1d || isSpring2d || isSpring3d;
  const isBeam = studyKind === "beam_1d" || isThermalBeam;
  const isTorsion = studyKind === "torsion_1d";
  const isTruss = studyKind === "truss_2d" || isThermalTruss2d;
  const isTruss3d = studyKind === "truss_3d" || isThermalTruss3d || isSpring3d;
  const isFrame = studyKind === "frame_2d";
  const isFrameLike = isFrame || isThermalFrame;
  const isPlane =
    isHeatPlane ||
    studyKind === "plane_triangle_2d" ||
    studyKind === "plane_quad_2d" ||
    isThermalPlaneTriangle ||
    isThermalPlaneQuad;
  const axialResult = isAxial && isAxialResult(result) ? result : null;
  const heatBarResult = isHeatBar && isHeatBar1dResult(result) ? result : null;
  const heatPlaneTriangleResult = isHeatPlaneTriangle && isHeatPlaneTriangle2dResult(result) ? result : null;
  const heatPlaneQuadResult = isHeatPlaneQuad && isHeatPlaneQuad2dResult(result) ? result : null;
  const thermalBarResult = isThermalBar && isThermalBar1dResult(result) ? result : null;
  const thermalBeamResult = isThermalBeam && isThermalBeam1dResult(result) ? result : null;
  const thermalFrameResult = isThermalFrame && isThermalFrame2dResult(result) ? result : null;
  const thermalTrussResult = isThermalTruss2d && isThermalTruss2dResult(result) ? result : null;
  const thermalTruss3dResult = isThermalTruss3d && isThermalTruss3dResult(result) ? result : null;
  const springResult = isSpring1d && isSpring1dResult(result) ? result : null;
  const spring2dResult = isSpring2d && isSpring2dResult(result) ? result : null;
  const spring3dResult = isSpring3d && isSpring3dResult(result) ? result : null;
  const activeSpringResult = isSpring1d ? springResult : isSpring2d ? spring2dResult : spring3dResult;
  const activeSpringModel = isSpring1d ? springModel : isSpring2d ? spring2dModel : spring3dModel;
  const trussResult = studyKind === "truss_2d" && isTrussResult(result) ? result : null;
  const truss3dResult = studyKind === "truss_3d" && isTruss3dResult(result) ? result : null;
  const beamResult = studyKind === "beam_1d" && isBeam1dResult(result) ? result : null;
  const activeBeamLikeResult = isThermalBeam ? thermalBeamResult : beamResult;
  const activeBeamLikeModel = isThermalBeam ? thermalBeamModel : beamModel;
  const torsionResult = isTorsion && isTorsion1dResult(result) ? result : null;
  const frameResult = isFrame && isFrame2dResult(result) ? result : null;
  const activeFrameLikeResult = isThermalFrame ? thermalFrameResult : frameResult;
  const activeFrameLikeModel = isThermalFrame ? thermalFrameModel : frameModel;
  const planeResult = !isHeatPlane && isPlane && isPlaneResult(result) ? result : null;
  const activePlaneInputModel = isHeatPlane ? heatPlaneModel : planeModel;
  const activeResultWindow =
    resultWindow && job?.job_id === resultWindow.jobId && studyKind === resultWindow.studyKind ? resultWindow : null;
  const trussDiagnostics = isTruss ? analyzeTrussModel(trussModel, t, selectedNode) : null;
  const trussStability = isTruss && trussDiagnostics ? summarizeTrussStability(trussModel, trussDiagnostics) : null;
  const axialNodes = axialResult?.nodes ?? [];
  const axialElements = axialResult?.elements ?? [];
  const axialLength = axialResult?.input.length ?? axialForm.length;
  const axialScale = axialResult?.max_displacement ? 140 / axialResult.max_displacement : 1;
  const planeWindowNodes =
    activeResultWindow?.studyKind === "heat_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "heat_plane_quad_2d" ||
    activeResultWindow?.studyKind === "plane_triangle_2d" ||
    activeResultWindow?.studyKind === "plane_quad_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_quad_2d"
      ? (activeResultWindow.nodes as PlaneTriangle2dResult["nodes"] | PlaneQuad2dResult["nodes"] | ThermalPlaneTriangle2dResult["nodes"] | ThermalPlaneQuad2dResult["nodes"] | HeatPlaneTriangle2dResult["nodes"] | HeatPlaneQuad2dResult["nodes"])
      : undefined;
  const planeWindowElements =
    activeResultWindow?.studyKind === "heat_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "heat_plane_quad_2d" ||
    activeResultWindow?.studyKind === "plane_triangle_2d" ||
    activeResultWindow?.studyKind === "plane_quad_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_quad_2d"
      ? (activeResultWindow.elements as PlaneTriangle2dResult["elements"] | PlaneQuad2dResult["elements"] | ThermalPlaneTriangle2dResult["elements"] | ThermalPlaneQuad2dResult["elements"] | HeatPlaneTriangle2dResult["elements"] | HeatPlaneQuad2dResult["elements"])
      : undefined;
  const trussWindowNodes =
    activeResultWindow?.studyKind === "truss_2d" ? (activeResultWindow.nodes as Truss2dResult["nodes"]) : undefined;
  const trussWindowElements =
    activeResultWindow?.studyKind === "truss_2d" ? (activeResultWindow.elements as Truss2dResult["elements"]) : undefined;
  const thermalTrussWindowNodes =
    activeResultWindow?.studyKind === "thermal_truss_2d"
      ? (activeResultWindow.nodes as ThermalTruss2dResult["nodes"])
      : undefined;
  const thermalTrussWindowElements =
    activeResultWindow?.studyKind === "thermal_truss_2d"
      ? (activeResultWindow.elements as ThermalTruss2dResult["elements"])
      : undefined;
  const truss3dWindowNodes =
    activeResultWindow?.studyKind === "truss_3d" || activeResultWindow?.studyKind === "thermal_truss_3d" || activeResultWindow?.studyKind === "spring_3d"
      ? (activeResultWindow.nodes as Truss3dResult["nodes"] | ThermalTruss3dResult["nodes"] | Spring3dResult["nodes"])
      : undefined;
  const truss3dWindowElements =
    activeResultWindow?.studyKind === "truss_3d" || activeResultWindow?.studyKind === "thermal_truss_3d" || activeResultWindow?.studyKind === "spring_3d"
      ? (activeResultWindow.elements as Truss3dResult["elements"] | ThermalTruss3dResult["elements"] | Spring3dResult["elements"])
      : undefined;
  const beamWindowNodes =
    activeResultWindow?.studyKind === "beam_1d" ? (activeResultWindow.nodes as Beam1dResult["nodes"]) : undefined;
  const beamWindowElements =
    activeResultWindow?.studyKind === "beam_1d" ? (activeResultWindow.elements as Beam1dResult["elements"]) : undefined;
  const thermalBeamWindowNodes =
    activeResultWindow?.studyKind === "thermal_beam_1d" ? (activeResultWindow.nodes as ThermalBeam1dResult["nodes"]) : undefined;
  const thermalBeamWindowElements =
    activeResultWindow?.studyKind === "thermal_beam_1d" ? (activeResultWindow.elements as ThermalBeam1dResult["elements"]) : undefined;
  const torsionWindowNodes =
    activeResultWindow?.studyKind === "torsion_1d" ? (activeResultWindow.nodes as Torsion1dResult["nodes"]) : undefined;
  const torsionWindowElements =
    activeResultWindow?.studyKind === "torsion_1d" ? (activeResultWindow.elements as Torsion1dResult["elements"]) : undefined;
  const heatBarWindowNodes =
    activeResultWindow?.studyKind === "heat_bar_1d" ? (activeResultWindow.nodes as HeatBar1dResult["nodes"]) : undefined;
  const heatBarWindowElements =
    activeResultWindow?.studyKind === "heat_bar_1d" ? (activeResultWindow.elements as HeatBar1dResult["elements"]) : undefined;
  const thermalWindowNodes =
    activeResultWindow?.studyKind === "thermal_bar_1d" ? (activeResultWindow.nodes as ThermalBar1dResult["nodes"]) : undefined;
  const thermalWindowElements =
    activeResultWindow?.studyKind === "thermal_bar_1d" ? (activeResultWindow.elements as ThermalBar1dResult["elements"]) : undefined;
  const springWindowNodes =
    activeResultWindow?.studyKind === "spring_1d" ? (activeResultWindow.nodes as Spring1dResult["nodes"]) : undefined;
  const springWindowElements =
    activeResultWindow?.studyKind === "spring_1d" ? (activeResultWindow.elements as Spring1dResult["elements"]) : undefined;
  const spring2dWindowNodes =
    activeResultWindow?.studyKind === "spring_2d" ? (activeResultWindow.nodes as Spring2dResult["nodes"]) : undefined;
  const spring2dWindowElements =
    activeResultWindow?.studyKind === "spring_2d" ? (activeResultWindow.elements as Spring2dResult["elements"]) : undefined;
  const spring3dWindowNodes =
    activeResultWindow?.studyKind === "spring_3d" ? (activeResultWindow.nodes as Spring3dResult["nodes"]) : undefined;
  const spring3dWindowElements =
    activeResultWindow?.studyKind === "spring_3d" ? (activeResultWindow.elements as Spring3dResult["elements"]) : undefined;
  const frameWindowNodes =
    activeResultWindow?.studyKind === "frame_2d" ? (activeResultWindow.nodes as Frame2dResult["nodes"]) : undefined;
  const frameWindowElements =
    activeResultWindow?.studyKind === "frame_2d" ? (activeResultWindow.elements as Frame2dResult["elements"]) : undefined;
  const thermalFrameWindowNodes =
    activeResultWindow?.studyKind === "thermal_frame_2d" ? (activeResultWindow.nodes as ThermalFrame2dResult["nodes"]) : undefined;
  const thermalFrameWindowElements =
    activeResultWindow?.studyKind === "thermal_frame_2d" ? (activeResultWindow.elements as ThermalFrame2dResult["elements"]) : undefined;
  const { displayTrussNodes, displayTrussElements, trussBounds } = useMemo(() => {
    const nextDisplayTrussNodes = isThermalFrame
      ? buildDisplayThermalFrameNodes(thermalFrameModel, thermalFrameResult, thermalFrameWindowNodes)
      : isFrame
      ? buildDisplayFrameNodes(frameModel, frameResult, frameWindowNodes)
      : isThermalBeam
        ? buildDisplayThermalBeamNodes(thermalBeamModel, thermalBeamResult, thermalBeamWindowNodes)
      : isHeatBar
        ? buildDisplayHeatBarNodes(heatBarModel, heatBarResult, heatBarWindowNodes)
      : isThermalBar
        ? buildDisplayThermalBarNodes(thermalBarModel, thermalBarResult, thermalWindowNodes)
      : isThermalTruss2d
        ? buildDisplayThermalTrussNodes(thermalTrussModel, thermalTrussResult, thermalTrussWindowNodes)
      : isSpring1d
        ? buildDisplaySpringNodes(springModel, springResult, springWindowNodes)
      : isSpring2d
        ? buildDisplaySpring2dNodes(spring2dModel, spring2dResult, spring2dWindowNodes)
      : isBeam
        ? buildDisplayBeamNodes(beamModel, beamResult, beamWindowNodes)
      : isTorsion
        ? buildDisplayTorsionNodes(torsionModel, torsionResult, torsionWindowNodes)
        : buildDisplayTrussNodes(trussModel, trussResult, trussWindowNodes);
    const nextDisplayTrussElements = isThermalFrame
      ? buildDisplayThermalFrameElements(thermalFrameModel, thermalFrameResult, thermalFrameWindowElements)
      : isFrame
      ? buildDisplayFrameElements(frameModel, frameResult, frameWindowElements)
      : isThermalBeam
        ? buildDisplayThermalBeamElements(thermalBeamModel, thermalBeamResult, thermalBeamWindowElements)
      : isHeatBar
        ? buildDisplayHeatBarElements(heatBarModel, heatBarResult, heatBarWindowElements)
      : isThermalBar
        ? buildDisplayThermalBarElements(thermalBarModel, thermalBarResult, thermalWindowElements)
      : isThermalTruss2d
        ? buildDisplayThermalTrussElements(thermalTrussModel, thermalTrussResult, thermalTrussWindowElements)
      : isSpring1d
        ? buildDisplaySpringElements(springModel, springResult, springWindowElements)
      : isSpring2d
        ? buildDisplaySpring2dElements(spring2dModel, spring2dResult, spring2dWindowElements)
      : isBeam
        ? buildDisplayBeamElements(beamModel, beamResult, beamWindowElements)
      : isTorsion
        ? buildDisplayTorsionElements(torsionModel, torsionResult, torsionWindowElements)
        : buildDisplayTrussElements(trussModel, trussResult, trussWindowElements);

    return {
      displayTrussNodes: nextDisplayTrussNodes,
      displayTrussElements: nextDisplayTrussElements,
      trussBounds: getTrussBounds(nextDisplayTrussNodes),
    };
  }, [beamModel, beamResult, beamWindowElements, beamWindowNodes, frameModel, frameResult, frameWindowElements, frameWindowNodes, heatBarModel, heatBarResult, heatBarWindowElements, heatBarWindowNodes, isBeam, isFrame, isHeatBar, isSpring1d, isSpring2d, isThermalBar, isThermalBeam, isThermalFrame, isThermalTruss2d, isTorsion, spring2dModel, spring2dResult, spring2dWindowElements, spring2dWindowNodes, springModel, springResult, springWindowElements, springWindowNodes, thermalBarModel, thermalBarResult, thermalBeamModel, thermalBeamResult, thermalBeamWindowElements, thermalBeamWindowNodes, thermalFrameModel, thermalFrameResult, thermalFrameWindowElements, thermalFrameWindowNodes, thermalTrussModel, thermalTrussResult, thermalTrussWindowElements, thermalTrussWindowNodes, thermalWindowElements, thermalWindowNodes, torsionModel, torsionResult, torsionWindowElements, torsionWindowNodes, trussModel, trussResult, trussWindowNodes, trussWindowElements]);
  const { displayTruss3dNodes, displayTruss3dElements } = useMemo(
    () => ({
      displayTruss3dNodes: isSpring3d
        ? buildDisplaySpring3dNodes(spring3dModel, spring3dResult, spring3dWindowNodes)
        : isThermalTruss3d
          ? buildDisplayThermalTruss3dNodes(thermalTruss3dModel, thermalTruss3dResult, truss3dWindowNodes as ThermalTruss3dResult["nodes"] | undefined)
        : buildDisplayTruss3dNodes(truss3dModel, truss3dResult, truss3dWindowNodes as Truss3dResult["nodes"] | undefined),
      displayTruss3dElements: isSpring3d
        ? buildDisplaySpring3dElements(spring3dModel, spring3dResult, spring3dWindowElements)
        : isThermalTruss3d
          ? buildDisplayThermalTruss3dElements(thermalTruss3dModel, thermalTruss3dResult, truss3dWindowElements as ThermalTruss3dResult["elements"] | undefined)
        : buildDisplayTruss3dElements(truss3dModel, truss3dResult, truss3dWindowElements as Truss3dResult["elements"] | undefined),
    }),
    [isSpring3d, isThermalTruss3d, spring3dModel, spring3dResult, spring3dWindowElements, spring3dWindowNodes, thermalTruss3dModel, thermalTruss3dResult, truss3dModel, truss3dResult, truss3dWindowElements, truss3dWindowNodes],
  );
  const { planeNodes, planeElements, planeBounds } = useMemo(() => {
    const activePlaneInput = isHeatPlane ? heatPlaneModel : planeModel;
    const activePlaneResult = isHeatPlane
      ? (isHeatPlaneTriangle ? heatPlaneTriangleResult : heatPlaneQuadResult)
      : planeResult;
    const nextPlaneNodes =
      (planeWindowNodes ?? activePlaneResult?.nodes)?.map((node, index) => ({
        ...activePlaneInput.nodes[node.index ?? index],
        ...node,
        index,
        ux: "ux" in node && typeof node.ux === "number" ? node.ux : 0,
        uy: "uy" in node && typeof node.uy === "number" ? node.uy : 0,
        fix_x: !isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as PlaneTriangle2dJobInput["nodes"][number]).fix_x ?? false : false,
        fix_y: !isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as PlaneTriangle2dJobInput["nodes"][number]).fix_y ?? false : false,
        load_x: !isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as PlaneTriangle2dJobInput["nodes"][number]).load_x ?? 0 : 0,
        load_y: !isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as PlaneTriangle2dJobInput["nodes"][number]).load_y ?? 0 : 0,
        fix_temperature: isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as HeatPlaneNodeInput).fix_temperature ?? false : undefined,
        temperature: isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as HeatPlaneNodeInput).temperature ?? 0 : undefined,
        heat_load: isHeatPlane ? (activePlaneInput.nodes[node.index ?? index] as HeatPlaneNodeInput).heat_load ?? 0 : undefined,
      })) ??
      activePlaneInput.nodes.map((node, index) => ({
        ...node,
        index,
        ux: 0,
        uy: 0,
        displacement_magnitude: 0,
        fix_x: !isHeatPlane && "fix_x" in node ? node.fix_x : false,
        fix_y: !isHeatPlane && "fix_y" in node ? node.fix_y : false,
        load_x: !isHeatPlane && "load_x" in node ? node.load_x : 0,
        load_y: !isHeatPlane && "load_y" in node ? node.load_y : 0,
      }));
    const nextPlaneElements =
      (planeWindowElements ?? activePlaneResult?.elements)?.map((element) => ({
        ...activePlaneInput.elements[element.index],
        ...element,
        material_id: "material_id" in activePlaneInput.elements[element.index] ? activePlaneInput.elements[element.index]?.material_id : undefined,
      })) ??
      activePlaneInput.elements.map((element, index) => ({
        ...element,
        index,
        area: 0,
        average_temperature: 0,
        temperature_gradient_x: 0,
        temperature_gradient_y: 0,
        heat_flux_x: 0,
        heat_flux_y: 0,
        heat_flux_magnitude: 0,
        strain_x: 0,
        strain_y: 0,
        average_temperature_delta: 0,
        thermal_strain: 0,
        mechanical_strain_x: 0,
        mechanical_strain_y: 0,
        total_strain_x: 0,
        total_strain_y: 0,
        gamma_xy: 0,
        stress_x: 0,
        stress_y: 0,
        tau_xy: 0,
        principal_stress_1: 0,
        principal_stress_2: 0,
        max_in_plane_shear: 0,
        von_mises: 0,
      }));

    return {
      planeNodes: nextPlaneNodes,
      planeElements: nextPlaneElements,
      planeBounds: getTrussBounds(nextPlaneNodes),
    };
  }, [planeWindowNodes, planeWindowElements, planeResult, planeModel, heatPlaneModel, isHeatPlane, isHeatPlaneTriangle, heatPlaneTriangleResult, heatPlaneQuadResult]);
  const planeResultFieldMax = useMemo(
    () => Math.max(...planeElements.map((element) => planeResultFieldValue(element, planeResultField)), 0),
    [planeElements, planeResultField],
  );
  const activeLineResultField: LineResultField =
    isHeatBar
      ? "average_temperature_delta"
      : isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
      ? "axial_stress"
      : isBeam
        ? beamResultField
        : frameResultField;
  const frameResultFieldMax = useMemo(
    () => Math.max(...displayTrussElements.map((element) => lineResultFieldValue(element, activeLineResultField)), 0),
    [activeLineResultField, displayTrussElements],
  );
  const planeResultFieldLabel =
    planeResultField === "average_temperature"
      ? t.maxTemperature
      : planeResultField === "average_temperature_delta"
      ? t.temperatureDelta
      : planeResultField === "temperature_gradient_x"
        ? t.temperatureGradientX
      : planeResultField === "temperature_gradient_y"
        ? t.temperatureGradientY
        : planeResultField === "heat_flux_x"
          ? t.heatFluxX
          : planeResultField === "heat_flux_y"
            ? t.heatFluxY
        : planeResultField === "heat_flux_magnitude"
          ? t.maxHeatFlux
      : planeResultField === "thermal_strain"
        ? t.thermalStrain
        : planeResultField === "mechanical_strain"
          ? t.mechanicalStrain
      : planeResultField === "principal_stress_1"
      ? t.planeViewPrincipal1
      : planeResultField === "max_in_plane_shear"
        ? t.planeViewMaxShear
        : t.planeViewVonMises;
  const planeLegendText =
    planeResultField === "average_temperature"
      ? `${t.maxTemperature} · ${t.planeResultLegend}`
      : planeResultField === "average_temperature_delta"
      ? `${t.temperatureDelta} · ${t.planeResultLegend}`
      : planeResultField === "temperature_gradient_x"
        ? `${t.temperatureGradientX} · ${t.planeResultLegend}`
      : planeResultField === "temperature_gradient_y"
        ? `${t.temperatureGradientY} · ${t.planeResultLegend}`
        : planeResultField === "heat_flux_x"
          ? `${t.heatFluxX} · ${t.planeResultLegend}`
          : planeResultField === "heat_flux_y"
            ? `${t.heatFluxY} · ${t.planeResultLegend}`
        : planeResultField === "heat_flux_magnitude"
          ? `${t.maxHeatFlux} · ${t.planeResultLegend}`
      : planeResultField === "thermal_strain"
        ? `${t.thermalStrain} · ${t.planeResultLegend}`
        : planeResultField === "mechanical_strain"
          ? `${t.mechanicalStrain} · ${t.planeResultLegend}`
      : planeResultField === "principal_stress_1"
      ? `${t.planeViewPrincipal1} · ${t.planeResultLegend}`
      : planeResultField === "max_in_plane_shear"
        ? `${t.planeViewMaxShear} · ${t.planeResultLegend}`
        : `${t.planeViewVonMises} · ${t.planeResultLegend}`;
  const frameResultFieldLabel =
    activeLineResultField === "average_temperature_delta"
      ? t.temperatureDelta
      : activeLineResultField === "temperature_gradient_y"
        ? t.temperatureGradientY
        : activeLineResultField === "thermal_curvature"
          ? t.thermalCurvature
      : activeLineResultField === "axial_stress"
      ? isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
        ? t.axialForce
        : t.stress
      : activeLineResultField === "shear_force"
        ? t.shearForce
      : activeLineResultField === "max_bending_stress"
        ? isTorsion
          ? t.torsionStress
          : t.bendingStress
        : activeLineResultField === "moment"
          ? isTorsion
            ? t.maxTorque
            : t.maxMoment
          : t.combinedStress;
  const frameLegendText = `${frameResultFieldLabel} · ${t.planeResultLegend}`;
  const frameTreeValueLabel =
    activeLineResultField === "average_temperature_delta"
      ? t.temperatureDelta
      : activeLineResultField === "temperature_gradient_y"
        ? t.temperatureGradientY
        : activeLineResultField === "thermal_curvature"
          ? t.thermalCurvature
      : activeLineResultField === "axial_stress"
      ? isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
        ? t.axialForce
        : t.stress
      : activeLineResultField === "shear_force"
        ? t.shearForce
      : activeLineResultField === "max_bending_stress"
        ? isTorsion
          ? t.torsionStress
          : t.bendingStress
      : activeLineResultField === "moment"
          ? isTorsion
            ? t.maxTorque
            : t.maxMoment
          : t.combinedStress;
  const canProjectHeatToThermo =
    (studyKind === "heat_bar_1d" && Boolean(heatBarResult)) ||
    (studyKind === "heat_plane_triangle_2d" && Boolean(heatPlaneTriangleResult)) ||
    (studyKind === "heat_plane_quad_2d" && Boolean(heatPlaneQuadResult));
  const projectHeatToThermoStudy = () => {
    if (studyKind === "heat_bar_1d" && heatBarResult) {
      recordHistory(t.projectHeatToThermoAction);
      resetActiveResult(setResult, setJob);
      setThermalBarModel(buildThermalBarFromHeatResult(heatBarModel, heatBarResult, thermalBarModel));
      setStudyKind("thermal_bar_1d");
      openWorkspaceStudy("controls");
      setMessage(t.projectedHeatToThermo);
      return "thermal_bar_1d" as const;
    }

    if (studyKind === "heat_plane_triangle_2d" && heatPlaneTriangleResult) {
      recordHistory(t.projectHeatToThermoAction);
      resetActiveResult(setResult, setJob);
      setPlaneModel(
        buildThermalPlaneTriangleFromHeatResult(
          heatPlaneModel as HeatPlaneTriangle2dJobInput,
          heatPlaneTriangleResult,
          planeModel as ThermalPlaneTriangle2dJobInput,
          activeMaterial,
        ),
      );
      setPlaneResultField("average_temperature_delta");
      setStudyKind("thermal_plane_triangle_2d");
      openWorkspaceStudy("controls");
      setMessage(t.projectedHeatToThermo);
      return "thermal_plane_triangle_2d" as const;
    }

    if (studyKind === "heat_plane_quad_2d" && heatPlaneQuadResult) {
      recordHistory(t.projectHeatToThermoAction);
      resetActiveResult(setResult, setJob);
      setPlaneModel(
        buildThermalPlaneQuadFromHeatResult(
          heatPlaneModel as HeatPlaneQuad2dJobInput,
          heatPlaneQuadResult,
          planeModel as ThermalPlaneQuad2dJobInput,
          activeMaterial,
        ),
      );
      setPlaneResultField("average_temperature_delta");
      setStudyKind("thermal_plane_quad_2d");
      openWorkspaceStudy("controls");
      setMessage(t.projectedHeatToThermo);
      return "thermal_plane_quad_2d" as const;
    }

    return null;
  };
  const { planeHotspotElements, planeThermalRows, frameHotspotElements, frameForceRows, frameMaxAxialForce, frameMaxShearForce } = useMemo(
    () =>
      buildWorkbenchHotspotData({
        isHeatPlane,
        isThermalFrame,
        isSpring,
        isThermalBar,
        isThermalTruss2d,
        isThermalTruss3d,
        isTorsion,
        isBeam,
        planeElements,
        planeResultField,
        displayTrussElements,
        activeLineResultField,
        selectedElement,
        planeHotspotLimit,
      }),
    [activeLineResultField, displayTrussElements, isBeam, isHeatPlane, isSpring, isThermalBar, isThermalFrame, isThermalTruss2d, isThermalTruss3d, isTorsion, planeElements, planeHotspotLimit, planeResultField, selectedElement],
  );
  const resultExportEffects = {
    result,
    studyKind,
    loadedModelName,
    job,
    isPlane,
    isFrameLike,
    isBeam,
    isSpring,
    isThermal,
    isThermalBar,
    isThermalTruss2d,
    isThermalTruss3d,
    planeResultField,
    activeLineResultField,
    planeHotspotElements,
    frameHotspotElements,
    frameForceRows,
    downloadTextFile,
    setMessage,
    labels: {
      noResultToExport: t.noResultToExport,
      resultJsonDownloaded: t.resultJsonDownloaded,
      resultCsvDownloaded: t.resultCsvDownloaded,
    },
  };
  const thermalFrameMaxTemperatureDelta = isThermalFrame ? thermalFrameResult?.max_temperature_delta ?? 0 : 0;
  const thermalFrameMaxTemperatureGradient = isThermalFrame ? thermalFrameResult?.max_temperature_gradient ?? 0 : 0;
  const thermalBeamMaxTemperatureGradient = isThermalBeam ? thermalBeamResult?.max_temperature_gradient ?? 0 : 0;

  useEffect(() => {
    if (focusedPlaneElement === null) return;

    const timeout = window.setTimeout(() => {
      setFocusedPlaneElement(null);
    }, 1400);

    return () => window.clearTimeout(timeout);
  }, [focusedPlaneElement]);

  useEffect(() => {
    if (focusedFrameElement === null) return;

    const timeout = window.setTimeout(() => {
      setFocusedFrameElement(null);
    }, 1400);

    return () => window.clearTimeout(timeout);
  }, [focusedFrameElement]);

  useEffect(() => {
    if (focusedFrameElement === null) return;

    const timeout = window.setTimeout(() => {
      setFocusedFrameElement(null);
    }, 1400);

    return () => window.clearTimeout(timeout);
  }, [focusedFrameElement]);
  const heartbeatStatusValue = heartbeatStatus(job, t);
  const heartbeatToneValue = heartbeatTone(job);
  const {
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
  } = useMemo(
    () =>
      buildWorkbenchSelectionData({
        selectedNode,
        selectedElement,
        displayTrussNodes,
        displayTrussElements,
        displayTruss3dNodes,
        displayTruss3dElements,
        planeNodes,
        planeElements,
        isThermalFrame,
        thermalFrameModel,
        thermalFrameResult,
        frameModel,
        frameResult,
        isThermalBeam,
        thermalBeamModel,
        thermalBeamResult,
        beamModel,
        beamResult,
        torsionModel,
        torsionResult,
        isHeatBar,
        heatBarModel,
        heatBarResult,
        thermalBarModel,
        thermalBarResult,
        activeSpringModel,
      }),
    [
      activeSpringModel,
      beamModel,
      beamResult,
      displayTruss3dElements,
      displayTruss3dNodes,
      displayTrussElements,
      displayTrussNodes,
      frameModel,
      frameResult,
      heatBarModel,
      heatBarResult,
      isHeatBar,
      isThermalBeam,
      isThermalFrame,
      planeElements,
      planeNodes,
      selectedElement,
      selectedNode,
      thermalBarModel,
      thermalBarResult,
      thermalBeamModel,
      thermalBeamResult,
      thermalFrameModel,
      thermalFrameResult,
      torsionModel,
      torsionResult,
    ],
  );
  const selectedNodeIssues =
    selectedNode !== null && trussDiagnostics ? trussDiagnostics.nodeIssues[selectedNode] ?? [] : [];
  const translatedFailureReason = humanizeSolverFailure(job?.message, t);
  const securityUi = useMemo(() => buildWorkbenchSecurityUi(language), [language]);
  const currentMaterials = useMemo(
    () =>
      studyKind === "thermal_truss_2d"
        ? thermalTrussModel.materials ?? []
        : studyKind === "truss_2d"
        ? trussModel.materials ?? []
        : studyKind === "thermal_truss_3d"
          ? thermalTruss3dModel.materials ?? []
          : studyKind === "truss_3d"
          ? truss3dModel.materials ?? []
          : studyKind === "heat_bar_1d" || studyKind === "thermal_bar_1d"
            ? []
          : studyKind === "thermal_beam_1d"
            ? thermalBeamModel.materials ?? []
          : studyKind === "thermal_frame_2d"
            ? thermalFrameModel.materials ?? []
          : studyKind === "spring_1d" || studyKind === "spring_2d"
            ? []
          : studyKind === "beam_1d"
            ? beamModel.materials ?? []
          : studyKind === "torsion_1d"
            ? []
          : studyKind === "frame_2d"
            ? frameModel.materials ?? []
            : studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d"
              ? planeModel.materials ?? []
              : isHeatPlane
                ? []
              : [],
    [beamModel.materials, frameModel.materials, isHeatPlane, planeModel.materials, studyKind, thermalBeamModel.materials, thermalFrameModel.materials, thermalTruss3dModel.materials, thermalTrussModel.materials, truss3dModel.materials, trussModel.materials],
  );
  const hiddenMaterialIds = hiddenMaterials[studyKind] ?? [];
  const materialColorMap = useMemo(
    () => new Map(currentMaterials.map((material, index) => [material.id, materialColorByIndex(index)])),
    [currentMaterials],
  );
  const materialOptions = useMemo(
    () =>
      currentMaterials.map((material) => ({
        id: material.id,
        label: `${material.name} (${round(material.youngs_modulus / 1.0e9)} GPa)`,
      })),
    [currentMaterials],
  );
  const adminJobRows = useMemo(
    () =>
      buildAdminJobRows({
        jobs: deferredJobHistory.filter((job) => {
          const matchesProject =
            !adminFilterProjectId ||
            (job.project_id ?? "").toLowerCase().includes(adminFilterProjectId.trim().toLowerCase());
          const matchesVersion =
            !adminFilterModelVersionId ||
            (job.model_version_id ?? "").toLowerCase().includes(adminFilterModelVersionId.trim().toLowerCase());
          return matchesProject && matchesVersion;
        }),
        heartbeatTone: (job) => heartbeatTone(job),
        heartbeatLabel: (job) => heartbeatStatus(job, t),
        detailLabel: (job) => humanizeSolverFailure(job.message, t) ?? job.message ?? job.worker_id ?? "--",
      }),
    [adminFilterModelVersionId, adminFilterProjectId, deferredJobHistory, t],
  );
  const adminResultRows = useMemo(
    () =>
      buildAdminResultRows({
        records: deferredResultRecords.filter((record) => {
          const linkedJob = jobHistory.find((job) => job.job_id === record.job_id);
          const matchesProject =
            !adminFilterProjectId ||
            (linkedJob?.project_id ?? "").toLowerCase().includes(adminFilterProjectId.trim().toLowerCase());
          const matchesVersion =
            !adminFilterModelVersionId ||
            (linkedJob?.model_version_id ?? "").toLowerCase().includes(adminFilterModelVersionId.trim().toLowerCase());
          return matchesProject && matchesVersion;
        }),
        jobs: jobHistory,
        updatedAtLabel: (record) => (record.updated_at ? formatTime(record.updated_at, language) : t.hasResult),
        summaryLabel: (record) => Object.keys(record.result).join(", ").slice(0, 64) || t.resultPayload,
      }),
    [adminFilterModelVersionId, adminFilterProjectId, deferredResultRecords, formatTime, jobHistory, language, t.hasResult, t.resultPayload],
  );
  const librarySampleRows = useMemo(
    () =>
      buildLibrarySampleRows({
        samples: SAMPLE_LIBRARY,
        kindLabel: (kind) => (kind in t.kinds ? t.kinds[kind as StudyKind] : kind),
        domainLabel: (domain) => t.studyDomains[domain],
        familyLabel: (family) => t.studyFamilies[family],
      }),
    [t],
  );
  const libraryModelRows = useMemo(
    () =>
      buildLibraryModelRows({
        models: deferredProjectModels,
        kindLabel: (kind) => (kind in t.kinds ? t.kinds[kind as StudyKind] : kind),
        updatedAtLabel: (value) => formatTime(value, language),
      }),
    [deferredProjectModels, formatTime, language, t],
  );
  const libraryVersionRows = useMemo(
    () =>
      buildLibraryVersionRows({
        versions: deferredModelVersions,
        updatedAtLabel: (value) => formatTime(value, language),
      }),
    [deferredModelVersions, formatTime, language],
  );
  const libraryJobRows = useMemo(
    () =>
      buildLibraryJobRows({
        jobs: deferredJobHistory,
        updatedAtLabel: (value) => formatTime(value, language),
        hasResultLabel: (hasResult) => (hasResult ? t.yes : t.no),
      }),
    [deferredJobHistory, formatTime, language, t.no, t.yes],
  );
  const protocolAgentCards = useMemo(
    () =>
      buildProtocolAgentCards({
        agents: protocolAgents,
        labels: {
          runtimeMode: t.runtimeMode,
          cluster: t.cluster,
          clusterSize: t.clusterSize,
          clusterHealth: t.clusterHealth,
          peers: t.peers,
          headless: t.headless,
          yes: t.yes,
          no: t.no,
          capabilities: t.capabilities,
          methods: t.methods,
          peerState: t.peerState,
        },
        clusterHealthTone,
        peerStatusLabel: (status) => formatPeerStatus(status, t),
      }),
    [protocolAgents, t],
  );
  const runtimeBackendRows = useMemo(
    () => [
      { label: t.ui, value: "3000" },
      { label: t.orchestrator, value: health ? "4000" : t.offline },
      { label: t.solverAgent, value: health?.transport?.solver_agent_tcp ?? 5001 },
    ],
    [health, t],
  );
  const runtimeProtocolRows = useMemo(
    () => [
      { label: t.controlPlaneProtocol, value: health?.protocol?.protocol?.name ?? "--" },
      { label: t.solverRpcProtocol, value: health?.protocol?.compatible_solver_rpc?.name ?? "--" },
      { label: t.deploymentMode, value: health?.deployment?.mode ?? "--" },
      { label: t.discoveryMode, value: health?.deployment?.discovery ?? "--" },
      { label: t.registeredAgents, value: health?.remote_solver_registry?.active_agents ?? 0 },
      { label: t.reachableAgents, value: protocolAgents.length },
      ...(frontendRuntimeMode === "direct_mesh_gui"
        ? [
            { label: t.directMeshStrategy, value: t.directMeshStrategies[directMeshSelectionMode] },
            { label: t.directMeshLastAgent, value: directMeshExecution?.endpoint ?? "--" },
            {
              label: t.directMeshLastRoute,
              value: directMeshExecution
                ? `${t.directMeshStrategies[directMeshExecution.strategy]} · ${formatTime(directMeshExecution.at, language)}`
                : "--",
            },
          ]
        : []),
    ],
    [directMeshExecution, directMeshSelectionMode, formatTime, frontendRuntimeMode, health, language, protocolAgents.length, t],
  );
  const runtimeProtocolMethods = useMemo(
    () => health?.protocol?.compatible_solver_rpc?.methods?.map((method) => formatProtocolMethodLabel(method)),
    [health?.protocol?.compatible_solver_rpc?.methods],
  );
  const runtimeSecurityRows = useMemo(
    () => [
      {
        label: securityUi.controlPlaneToken,
        value: health?.security?.api_token_configured ? securityUi.configured : securityUi.notConfigured,
      },
      {
        label: securityUi.clusterToken,
        value: health?.security?.cluster_token_configured ? securityUi.configured : securityUi.notConfigured,
      },
      {
        label: securityUi.clusterWindow,
        value: `${health?.security?.cluster_timestamp_window_ms ?? 30000} ms`,
      },
      {
        label: language === "zh" ? "Agent 白名单" : language === "ja" ? "Agent 許可リスト" : "Agent allowlist",
        value: health?.security?.cluster_agent_allowlist_enabled
          ? `${securityUi.enabled} · ${health?.security?.cluster_agent_allowlist_count ?? 0}`
          : securityUi.disabled,
      },
      {
        label: language === "zh" ? "Cluster 白名单" : language === "ja" ? "Cluster 許可リスト" : "Cluster allowlist",
        value: health?.security?.cluster_cluster_allowlist_enabled
          ? `${securityUi.enabled} · ${health?.security?.cluster_cluster_allowlist_count ?? 0}`
          : securityUi.disabled,
      },
      {
        label: language === "zh" ? "Fingerprint 绑定" : language === "ja" ? "Fingerprint バインディング" : "Fingerprint binding",
        value: health?.security?.cluster_fingerprint_required ? securityUi.enabled : securityUi.disabled,
      },
      {
        label: securityUi.protectReads,
        value: health?.security?.protect_reads ? securityUi.enabled : securityUi.disabled,
      },
      {
        label: securityUi.mutatingRoutes,
        value: health?.security?.mutating_routes_protected ? securityUi.enabled : securityUi.disabled,
      },
      {
        label: securityUi.clusterRoutes,
        value: health?.security?.cluster_routes_protected ? securityUi.enabled : securityUi.disabled,
      },
      {
        label: securityUi.directMeshRoutes,
        value: directMeshApiToken ? securityUi.configured : securityUi.enabled,
      },
    ],
    [directMeshApiToken, health, language, securityUi],
  );
  const runtimeAuditEntries = useMemo(
    () =>
      securityEventRecords.map((entry) => ({
        id: entry.event_id,
        at: formatTime(entry.occurred_at, language),
        action: entry.action,
        source:
          entry.source === "assistant"
            ? language === "zh"
              ? "助手"
              : language === "ja"
                ? "アシスタント"
                : "Assistant"
            : language === "zh"
              ? "脚本"
              : language === "ja"
                ? "スクリプト"
                : "Script",
        risk:
          entry.risk === "destructive"
            ? language === "zh"
              ? "高风险"
              : language === "ja"
                ? "破壊的"
                : "Destructive"
            : language === "zh"
              ? "敏感"
              : language === "ja"
                ? "機微"
                : "Sensitive",
        status:
          entry.status === "prompted"
            ? language === "zh"
              ? "待确认"
              : language === "ja"
                ? "確認待ち"
                : "Prompted"
            : entry.status === "cancelled"
              ? language === "zh"
                ? "已取消"
                : language === "ja"
                  ? "取消済み"
                  : "Cancelled"
              : entry.status === "completed"
                ? language === "zh"
                  ? "已执行"
                  : language === "ja"
                    ? "完了"
                    : "Completed"
                : language === "zh"
                  ? "失败"
                  : language === "ja"
                    ? "失敗"
                    : "Failed",
        note: entry.note ?? "--",
      })),
    [formatTime, language, securityEventRecords],
  );
  const runtimeAuditSummaryRows = useMemo(
    () => buildRuntimeAuditSummaryRows(language, securityEventRecords),
    [language, securityEventRecords],
  );
  const runtimeAuditTrendBars = useMemo(
    () => buildRuntimeAuditTrendBars(language, securityEventRecords, securityEventWindowFilter),
    [language, securityEventRecords, securityEventWindowFilter],
  );
  const runtimeAuditSourceStatusFacets = useMemo(
    () => buildRuntimeAuditSourceStatusFacets(language, securityEventRecords),
    [language, securityEventRecords],
  );
  const runtimeAuditStudyFacets = useMemo(
    () => buildRuntimeAuditStudyFacets(securityEventRecords),
    [securityEventRecords],
  );
  const runtimeAuditProjectFacets = useMemo(
    () => buildRuntimeAuditProjectFacets(securityEventRecords),
    [securityEventRecords],
  );
  const runtimeAuditModelVersionFacets = useMemo(
    () => buildRuntimeAuditModelVersionFacets(securityEventRecords),
    [securityEventRecords],
  );
  const runtimeWatchdogRows = useMemo(
    () => [
      { label: t.activeJobs, value: health?.watchdog?.active_jobs ?? 0 },
      { label: t.stalledJobs, value: health?.watchdog?.stalled_jobs ?? 0 },
      { label: t.timedOutJobs, value: health?.watchdog?.timed_out_jobs ?? 0 },
      { label: t.scanEvery, value: formatMilliseconds(health?.watchdog?.scan_interval_ms) },
      { label: t.staleAfter, value: formatMilliseconds(health?.watchdog?.stale_job_ms) },
      { label: t.timeoutAfter, value: formatMilliseconds(health?.watchdog?.job_timeout_ms) },
    ],
    [health, t],
  );
  const trussElementColors = useMemo(
    () =>
      displayTrussElements.map((element) =>
        (isFrameLike && activeFrameLikeResult) || (isBeam && activeBeamLikeResult) || (isTorsion && torsionResult)
          ? planeStressFill(lineResultFieldValue(element, activeLineResultField), frameResultFieldMax)
          : materialColorMap.get(element.material_id ?? "") ?? "#1677a3",
      ),
    [activeBeamLikeResult, activeFrameLikeResult, activeLineResultField, displayTrussElements, frameResultFieldMax, isBeam, isFrameLike, isTorsion, materialColorMap, torsionResult],
  );
  const truss3dElementColors = useMemo(
    () => displayTruss3dElements.map((element) => materialColorMap.get(element.material_id ?? "") ?? "#1677a3"),
    [displayTruss3dElements, materialColorMap],
  );
  const planeElementColors = useMemo(
    () => (isHeatPlane ? activePlaneInputModel.elements : planeModel.elements).map((element) => materialColorMap.get(("material_id" in element ? element.material_id : "") ?? "") ?? planeStressFill(0, 1)),
    [activePlaneInputModel.elements, isHeatPlane, planeModel.elements, materialColorMap],
  );
  const nodeCount =
    isAxial
      ? axialNodes.length
      : activeResultWindow?.totalNodes ??
        (isTruss ? trussResult?.nodes.length : isSpring3d ? spring3dResult?.nodes.length : isTruss3d ? truss3dResult?.nodes.length : isSpring ? activeSpringResult?.nodes.length : isBeam ? activeBeamLikeResult?.nodes.length : isTorsion ? torsionResult?.nodes.length : isFrameLike ? activeFrameLikeResult?.nodes.length : planeResult?.nodes.length) ??
        (isTruss ? trussModel.nodes.length : isSpring3d ? spring3dModel.nodes.length : isTruss3d ? truss3dModel.nodes.length : isSpring ? activeSpringModel.nodes.length : isBeam ? activeBeamLikeModel.nodes.length : isTorsion ? torsionModel.nodes.length : isFrameLike ? activeFrameLikeModel.nodes.length : activePlaneInputModel.nodes.length);
  const activeResultWindowLimit = activeResultWindow?.limit ?? resultWindowLimit;
  const resultWindowStart = activeResultWindow ? Math.min(resultWindowOffset, Math.max(0, resultWindowMaxTotal - 1)) + 1 : 0;
  const resultWindowEnd = activeResultWindow ? Math.min(resultWindowOffset + activeResultWindowLimit, resultWindowMaxTotal) : 0;
  const resultWindowJumps = activeResultWindow
    ? [
        { label: t.jumpStart, offset: 0 },
        { label: t.jumpQuarter, offset: clampChunkOffset(resultWindowMaxTotal * 0.25, resultWindowMaxTotal, activeResultWindowLimit) },
        { label: t.jumpMid, offset: clampChunkOffset(resultWindowMaxTotal * 0.5, resultWindowMaxTotal, activeResultWindowLimit) },
        {
          label: t.jumpThreeQuarter,
          offset: clampChunkOffset(resultWindowMaxTotal * 0.75, resultWindowMaxTotal, activeResultWindowLimit),
        },
        {
          label: t.jumpEnd,
          offset: clampChunkOffset(Math.max(0, resultWindowMaxTotal - activeResultWindowLimit), resultWindowMaxTotal, activeResultWindowLimit),
        },
      ].filter((jump, index, jumps) => jumps.findIndex((candidate) => candidate.offset === jump.offset) === index)
    : [];
  const hasViewportDock = isTruss3d && ((immersiveViewport && immersiveToolDrawerOpen) || (showShortcutHints && immersiveHelpDrawerOpen));
  const showViewportToolStrip = isTruss3d && immersiveToolDrawerOpen;
  const shouldStretchSpaceViewport = isTruss3d && !hasViewportDock && !activeResultWindow;
  const viewportPixelWidth =
    activeResultWindow
      ? Math.min(3200, 980 + Math.ceil(resultWindowMaxTotal / activeResultWindowLimit) * 180)
      : isTruss3d
        ? hasViewportDock
          ? 1120
          : undefined
        : 980;
  const directMeshEndpoints = parseDirectMeshEndpoints(directMeshEndpointsText);
  const hasAnyResult = Boolean(axialResult || heatBarResult || thermalBarResult || thermalBeamResult || thermalFrameResult || thermalTrussResult || thermalTruss3dResult || trussResult || truss3dResult || springResult || spring2dResult || spring3dResult || beamResult || torsionResult || frameResult || planeResult);

  const buildScriptSnapshot = (): WorkbenchScriptSnapshot => ({
    studyKind,
    sidebarSection,
    studyTab,
    modelTab,
    libraryTab,
    systemPanelTab,
    systemDataTab,
    language,
    theme,
    frontendRuntimeMode,
    selectedProjectId,
    selectedModelId,
    selectedVersionId,
    selectedAdminJobId,
    selectedAdminResultJobId,
    adminFilterProjectId,
    adminFilterModelVersionId,
    loadedModelName,
    activeMaterial,
    selectedNode,
    selectedElement,
    selectedTruss3dNodeIndices: selectedTruss3dNodes,
    memberDraftNodeIndices: memberDraftNodes,
    immersiveViewport,
    immersiveToolDrawerOpen,
    immersiveHelpDrawerOpen,
    truss3dProjectionMode,
    truss3dViewPreset,
    truss3dBoxSelectMode,
    truss3dLinkMode,
    hasResult: hasAnyResult,
    jobStatus: job?.status ?? null,
    projectCount: projects.length,
    jobHistoryCount: jobHistory.length,
    resultCount: resultRecords.length,
    protocolAgentCount: protocolAgents.length,
    healthStatus: health?.status ?? null,
    message,
  });

  const invokeScriptAction = async (
    action: string,
    payload: Record<string, unknown> = {},
    source: WorkbenchSecurityAuditSource = "script",
    note?: string,
  ) => {
    const actionDefinition = getWorkbenchScriptActionDefinition(action);
    if (actionDefinition?.requiresConfirmation) {
      const auditRisk = actionDefinition.risk as WorkbenchSecurityAuditRisk;
      recordSecurityAuditEvent({
        action,
        source,
        risk: auditRisk,
        status: "prompted",
        note: note ?? (language === "zh" ? "等待操作员确认。" : language === "ja" ? "オペレーター確認待ちです。" : "Waiting for operator confirmation."),
      });
      const confirmationMessage =
        language === "zh"
          ? `动作 ${action} 属于高风险操作，可能修改、删除或导出敏感数据。\n\n请确认是否继续执行。`
          : language === "ja"
            ? `操作 ${action} は高リスクで、機微データの変更・削除・出力を行う可能性があります。\n\n実行を続けますか。`
          : `The action ${action} is high risk and may modify, delete, or export sensitive data.\n\nConfirm execution?`;
      if (typeof window !== "undefined" && !window.confirm(confirmationMessage)) {
        const summary = language === "zh" ? "已被操作员取消确认。" : language === "ja" ? "オペレーター確認で取り消されました。" : "Cancelled by operator confirmation.";
        recordSecurityAuditEvent({
          action,
          source,
          risk: auditRisk,
          status: "cancelled",
          note: summary,
        });
        appendScriptActionLog({ action, source, status: "failed", summary, payload, note: summary });
        throw new Error(summary);
      }
    }

    appendScriptActionLog({ action, source, status: "started", summary: JSON.stringify(payload), payload, note });

    try {
      let resultPayload: Record<string, unknown>;
      const navResult = await handleWorkbenchScriptNavAction({
        action,
        payload,
        studyKind,
        studyKindResetHandlers,
        setStudyKind,
        handleSidebarSectionChange,
        recordHistory,
        changeStudyTypeLabel: t.changeStudyType,
        setStudyTab,
        setModelTab,
        setModelToolsPage,
        setLibraryTab,
        setSystemPanelTab,
        setAssistantWindowOpen,
        setSystemDataTab,
        handleLanguageChange,
        setTheme,
        setFrontendRuntimeMode,
        setDirectMeshEndpointsText,
        setDirectMeshSelectionMode,
        refreshHealth,
        refreshJobHistory,
        refreshResults,
        refreshProjects,
        refreshSecurityEvents,
      });
      if (navResult) {
        resultPayload = navResult;
      } else {
      const projectModelResult = await handleWorkbenchScriptProjectModelAction({
        action,
        payload,
        selectedProjectId,
        selectedModelId,
        selectedVersionId,
        projectNameDraft,
        projectDescriptionDraft,
        loadedModelName,
        activeMaterial,
        studyKind,
        setSelectedProjectId,
        setProjectNameDraft,
        setProjectDescriptionDraft,
        setSelectedModelId,
        setSelectedVersionId,
        setModelVersions,
        setLoadedModelName,
        setActiveMaterial,
        refreshProjects,
        refreshVersions,
        downloadProjectBundleJson,
        downloadProjectBundleZip,
        generateModel,
        generatePanelModel,
        serializeCurrentModel: () =>
          serializeCurrentModel(
            studyKind,
            loadedModelName,
            activeMaterial,
            axialForm,
            heatBarModel,
            heatPlaneModel,
            thermalBarModel,
            thermalBeamModel,
            thermalFrameModel,
            thermalTrussModel,
            trussModel,
            thermalTruss3dModel,
            truss3dModel,
            planeModel,
            frameModel,
            beamModel,
            torsionModel,
            springModel,
            spring2dModel,
            spring3dModel,
            parametric,
            round,
          ),
        createProject,
        updateProject,
        deleteProject,
        createModel,
        updateModel,
        deleteModel,
        createModelVersion,
        updateModelVersion,
        deleteModelVersion,
        projectRequiredLabel: t.projectRequired,
        defaultProjectLabel: t.defaultProject,
        projectCreatedLabel: t.projectCreated,
        projectUpdatedLabel: t.projectUpdated,
        projectDeletedLabel: t.projectDeleted,
        noSavedModelsLabel: t.noSavedModels,
        noVersionsLabel: t.noVersions,
        modelCreatedLabel: t.modelCreated,
        modelSavedLabel: t.modelSaved,
        modelDeletedLabel: t.modelDeletedStored,
        versionRenamedLabel: t.versionRenamed,
        versionDeletedLabel: t.versionDeleted,
        setMessage,
      });
      if (projectModelResult) {
        resultPayload = projectModelResult;
      } else {
      const stateResult = await handleWorkbenchScriptStateAction({
        action,
        payload,
        language,
        setStudyKind,
        setParametric,
        setPanelParametric,
        setTrussModel,
        setTruss3dModel,
        setPlaneModel,
        setFrameModel,
        setBeamModel,
        setSelectedNode,
        setSelectedElement,
        setSelectedTruss3dNodes,
        setMemberDraftNodes,
        setTruss3dLinkMode,
        setTruss3dFocusRequestVersion,
        setTruss3dResetRequestVersion,
        setTruss3dShowGrid,
        setTruss3dShowLabels,
        setTruss3dShowNodes,
        setImmersiveToolDrawerOpen,
        setImmersiveHelpDrawerOpen,
        setTruss3dBoxSelectMode,
        immersiveViewport,
        recordHistory,
        importActionLabel: t.importAction,
        editParametricLabel: t.editParametric,
        resolveTruss2dJobInput,
        resolveTruss3dJobInput,
        resolvePlaneQuad2dJobInput,
        resolvePlaneTriangle2dJobInput,
        ensureFrameModelMaterials,
        ensureBeamModelMaterials,
        activeMaterial,
        resetActiveResult: () => resetActiveResult(setResult, setJob),
        projectHeatToThermoStudy,
        toggleImmersiveViewport,
        handleUndo,
        handleRedo,
        runAnalysis,
        cancelCurrentJob,
        setTruss3dViewPreset,
        setTruss3dProjectionMode,
      });
      if (stateResult) {
        resultPayload = stateResult;
      } else {
      const macroDataResult = await handleWorkbenchScriptMacroDataAction({
        action,
        payload,
        source,
        note,
        language,
        getScriptSnapshot,
        invokeScriptAction,
        setSystemDataTab,
        setAdminFilterProjectId,
        setAdminFilterModelVersionId,
        setSelectedAdminJobId,
        setSelectedAdminResultJobId,
        setSidebarSection,
        setSystemPanelTab,
        resolveScriptLinkedJob: (nextPayload) => resolveScriptLinkedJobWithDeps(nextPayload, adminDataEffects),
        openModelVersionById,
        openProjectContextById: (projectId) => openProjectContextByIdWithDeps(projectId, adminDataEffects),
        applyJobContextToWorkbench: (linkedJob) => applyJobContextToWorkbenchWithDeps(linkedJob, adminDataEffects),
        downloadDatabaseSnapshot,
      });
      if (macroDataResult) {
        resultPayload = macroDataResult;
      } else {
        throw new Error(`Unknown script action: ${action}`);
      }
      }
      }
      }

      appendScriptActionLog({ action, source, status: "completed", summary: JSON.stringify(resultPayload), payload, result: resultPayload, note });
      if (actionDefinition?.requiresConfirmation) {
        recordSecurityAuditEvent({
          action,
          source,
          risk: actionDefinition.risk as WorkbenchSecurityAuditRisk,
          status: "completed",
          note: note ?? (language === "zh" ? "高风险动作已执行完成。" : language === "ja" ? "高リスク操作の実行が完了しました。" : "High-risk action completed."),
        });
      }
      return resultPayload;
    } catch (error) {
      const summary = error instanceof Error ? error.message : String(error);
      if (actionDefinition?.requiresConfirmation) {
        recordSecurityAuditEvent({
          action,
          source,
          risk: actionDefinition.risk as WorkbenchSecurityAuditRisk,
          status: "failed",
          note: summary,
        });
      }
      appendScriptActionLog({ action, source, status: "failed", summary, payload, note: summary });
      throw error;
    }
  };

  const buildSnapshot = (): WorkbenchSnapshot => ({
    ...buildWorkbenchSnapshot({
      studyKind,
      axialForm,
      heatBarModel,
      heatPlaneModel,
      thermalBarModel,
      thermalBeamModel,
      thermalFrameModel,
      thermalTrussModel,
      trussModel,
      thermalTruss3dModel,
      truss3dModel,
      planeModel,
      frameModel,
      beamModel,
      torsionModel,
      springModel,
      spring2dModel,
      spring3dModel,
      parametric,
      panelParametric,
      activeMaterial,
      loadedModelName,
      sidebarSection,
      selectedNode,
      selectedElement,
      memberDraftNodes,
    }),
  });

  const restoreSnapshot = (snapshot: WorkbenchSnapshot) => {
    restoreWorkbenchSnapshot(
      snapshot,
      {
        setStudyKind,
        setAxialForm,
        setHeatBarModel,
        setHeatPlaneModel,
        setThermalBarModel,
        setThermalBeamModel,
        setThermalFrameModel,
        setThermalTrussModel,
        setTrussModel,
        setThermalTruss3dModel,
        setTruss3dModel,
        setPlaneModel,
        setFrameModel,
        setBeamModel,
        setTorsionModel,
        setSpringModel,
        setSpring2dModel,
        setSpring3dModel,
        setParametric,
        setPanelParametric,
        setActiveMaterial,
        setLoadedModelName,
        setSidebarSection,
        setSelectedNode,
        setSelectedElement,
        setMemberDraftNodes,
      },
      () => resetActiveResult(setResult, setJob),
    );
  };

  const {
    assistantTransactions,
    appendScriptActionLog,
    executeAssistantPlan,
    getScriptSnapshot,
    recordManualDslAction,
    recordSecurityAuditEvent,
    rollbackAssistantTransaction,
    scriptActionLog,
    securityAuditLog,
  } = useWorkbenchAssistantAuditController({
    language,
    scriptRecordingMode,
    frontendRuntimeMode,
    studyKind,
    selectedProjectId,
    selectedModelId,
    selectedVersionId,
    immersiveViewport,
    setMessage,
    buildScriptSnapshot,
    buildWorkbenchSnapshot: () => buildSnapshot(),
    restoreWorkbenchSnapshot: restoreSnapshot,
  });

  const scriptSnapshot = buildScriptSnapshot();

  const recordHistory = (label: string) => {
    const snapshot = buildSnapshot();
    setUndoStack((current) => pushHistoryEntry(current, label, snapshot));
    setRedoStack([]);
  };

  const handleUndo = () => {
    const currentSnapshot = buildSnapshot();
    const { entry, nextSource, nextTarget } = stepHistory(undoStack, redoStack, currentSnapshot);
    if (!entry) return;
    setUndoStack(nextSource);
    setRedoStack(nextTarget);
    restoreSnapshot(entry.snapshot);
    setMessage(t.undoApplied);
  };

  const handleRedo = () => {
    const currentSnapshot = buildSnapshot();
    const { entry, nextSource, nextTarget } = stepHistory(redoStack, undoStack, currentSnapshot);
    if (!entry) return;
    setRedoStack(nextSource);
    setUndoStack(nextTarget);
    restoreSnapshot(entry.snapshot);
    setMessage(t.redoApplied);
  };

  const persistedModelEffects = {
    startTransition,
    activeMaterial,
    createProject,
    createModel,
    createModelVersion,
    updateModelVersion,
    fetchModel,
    fetchModelVersion,
    refreshProjects,
    refreshVersions,
    recordHistory,
    resetActiveResult: () => resetActiveResult(setResult, setJob),
    importActionLabel: t.importAction,
    historyActionLabel: t.historyAction,
    importedModelLabel: t.persistedModelLoaded,
    importedProjectLabel: t.projectImported,
    importedVersionLabel: t.versionLoaded,
    importFailedLabel: t.importFailed,
    setMessage,
    setSelectedProjectId,
    setSidebarSection,
    setLoadedModelName,
    setSelectedModelId,
    setSelectedVersionId,
    setModelVersions,
    setStudyKind,
    setAxialForm,
    setHeatBarModel,
    setHeatPlaneModel,
    setThermalBarModel,
    setThermalBeamModel,
    setThermalFrameModel,
    setThermalTrussModel,
    setThermalTruss3dModel,
    setSpringModel,
    setSpring2dModel,
    setSpring3dModel,
    setTrussModel,
    setTruss3dModel,
    setPlaneModel,
    setFrameModel,
    setBeamModel,
    setTorsionModel,
    setPlaneResultField,
    setParametric,
    setActiveMaterial,
  };

  const applyTrussSuggestion = (suggestion: TrussSuggestion) => {
    recordHistory(t.applySuggestionAction);
    resetActiveResult(setResult, setJob);
    setStudyKind("truss_2d");
    setSidebarSection("model");
    setSelectedElement(null);
    setSelectedNode(suggestion.nodeIndex);
    setMemberDraftNodes([]);

    if (suggestion.kind === "fix_support") {
      setTrussModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === suggestion.nodeIndex
            ? { ...node, [suggestion.axis === "x" ? "fix_x" : "fix_y"]: true }
            : node,
        ),
      }));
      setMessage(suggestion.axis === "x" ? t.suggestionAppliedSupportX : t.suggestionAppliedSupportY);
      return;
    }

    let connected = false;
    setTrussModel((current) => {
      const nearestIndex = findNearestConnectableNode(current, suggestion.nodeIndex);
      if (nearestIndex === null) return current;
      connected = true;
      const material = current.materials?.[0];
      return {
        ...current,
        elements: [
          ...current.elements,
          {
            id: `e${current.elements.length}`,
            node_i: suggestion.nodeIndex,
            node_j: nearestIndex,
            area: parametric.area,
            youngs_modulus: material?.youngs_modulus ?? parametric.youngsModulusGpa * 1.0e9,
            material_id: material?.id,
          },
        ],
      };
    });
    setMessage(connected ? t.suggestionAppliedLink : t.suggestionNoLinkTarget);
  };

  const {
    addCustomMaterialToCurrentModel,
    addMaterialToCurrentModel,
    applyMaterialToCurrentModel,
    deleteCurrentMaterial,
    importMaterials,
    toggleMaterialVisibility,
    updateCurrentMaterial,
  } = createWorkbenchMaterialEditController({
    activeMaterial,
    labels: {
      editMemberAction: t.editMemberAction,
      editMaterial: t.editMaterial,
      importedMaterialLibrary:
        language === "zh"
          ? "外部材料库已导入。"
          : language === "ja"
            ? "外部材料ライブラリを取り込みました。"
            : "Imported external material library.",
      initialFailed: t.initialFailed,
    },
    recordHistory,
    resetResults: () => resetActiveResult(setResult, setJob),
    selectedElement,
    setFrameModel,
    setHiddenMaterials,
    setMessage,
    setPlaneModel,
    setThermalFrameModel,
    setThermalTruss3dModel,
    setThermalTrussModel,
    setTruss3dModel,
    setTrussModel,
    studyKind,
  });

  const {
    addNode,
    applySelectedTruss3dLoads,
    assignSelectedElementMaterial,
    assignSelectedFrameElementMaterial: assignSelectedFrameElementMaterialBase,
    assignSelectedPlaneElementMaterial,
    assignSelectedTruss3dElementMaterial,
    cloneSelectedTruss3dNodes,
    deleteSelectedElement,
    deleteSelectedNode,
    deleteSelectedTruss3dElement,
    deleteSelectedTruss3dNode,
    nudgeSelectedTruss3dNodes,
    toggleMemberFromDraft,
    updateSelectedElement,
    updateSelectedFrameElement: updateSelectedFrameElementBase,
    updateSelectedFrameNode: updateSelectedFrameNodeBase,
    updateSelectedNode,
    updateSelectedPlaneElement,
    updateSelectedPlaneNode,
    updateSelectedTruss3dElement,
    updateSelectedTruss3dNode,
    updateSelectedTruss3dNodes,
  } = createWorkbenchStructureEditController({
    activeFrameLikeModel,
    isFrameLike,
    isHeatPlane,
    isThermalFrame,
    isThermalTruss2d,
    isThermalTruss3d,
    labels: {
      addNodeAction: t.addNodeAction,
      branchCreated: t.branchCreated,
      deleteMemberAction: t.deleteMemberAction,
      deleteNodeAction: t.deleteNodeAction,
      editMemberAction: t.editMemberAction,
      editNodeAction: t.editNodeAction,
      memberCreated: t.memberCreated,
      memberDeleted: t.memberDeleted,
      memberRemoved: t.memberRemoved,
      nodeCreated: t.nodeCreated,
      nodeDeleted: t.nodeDeleted,
      selectTwoNodes: t.selectTwoNodes,
      spaceMemberDeleted: t.spaceMemberDeleted,
      spaceNodeDeleted: t.spaceNodeDeleted,
      toggleMemberAction: t.toggleMemberAction,
    },
    memberDraftNodes,
    parametric,
    recordHistory,
    resetResults: () => resetActiveResult(setResult, setJob),
    roundValue: round,
    selectedElement,
    selectedNode,
    selectedTruss3dNodes,
    setFrameModel,
    setHeatPlaneModel,
    setMemberDraftNodes,
    setMessage,
    setPlaneModel,
    setSelectedElement,
    setSelectedNode,
    setSelectedTruss3dNodes,
    setSidebarSection,
    setStudyKind,
    setThermalFrameModel,
    setThermalTruss3dModel,
    setThermalTrussModel,
    setTruss3dModel,
    setTrussModel,
    studyKind,
    truss3dBatchLoadX,
    truss3dBatchLoadY,
    truss3dBatchLoadZ,
    truss3dModel,
    trussModel,
  });
  const {
    addTruss3dNode,
    handleTruss3dNodePick,
    handleTruss3dNodesBoxSelect,
    handleTrussPointerMove,
    startTrussNodeDrag,
    stopDraggingNode,
    toggleDraftNode,
    toggleTruss3dLinkMode,
    toggleTruss3dMemberFromDraft,
    updateTruss3dNodePosition,
  } = createWorkbenchTrussGestureController({
    studyKind,
    isFrameLike,
    isBeam,
    isTorsion,
    isHeatBar,
    isThermal,
    isThermalBar,
    truss3dLinkMode,
    truss3dModel,
    trussBounds,
    roundValue: round,
    selectedNode,
    selectedTruss3dNodes,
    memberDraftNodes,
    draggingNode,
    dragHistoryCapturedRef,
    dragFrameRef,
    pendingDragPointRef,
    setStudyKind,
    setSidebarSection,
    setSelectedNode,
    setSelectedTruss3dNodes,
    setSelectedElement,
    setMemberDraftNodes,
    setMessage,
    setTruss3dLinkMode,
    setDraggingNode,
    setTruss3dModel,
    setTrussModel,
    setFrameModel,
    setThermalFrameModel,
    resetResults: () => resetActiveResult(setResult, setJob),
    recordHistory,
    labels: {
      addNodeAction: t.addNodeAction,
      branchCreated: t.spaceBranchCreated,
      spaceNodeCreated: t.spaceNodeCreated,
      linkModeEnabled: t.linkModeEnabled,
      linkModeDisabled: t.linkModeDisabled,
      toggleMemberAction: t.toggleMemberAction,
      memberRemoved: t.memberRemoved,
      linkModeCompleted: t.linkModeCompleted,
      selectTwoNodes: t.selectTwoNodes,
      dragNodeAction: t.dragNodeAction,
    },
  });

  const updateSelectedFrameNode = (
    key: keyof Frame2dJobInput["nodes"][number] | keyof ThermalFrame2dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);

    if (isTorsion) {
      setTorsionModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode
            ? {
                ...node,
                ...(key === "x" ? { x: Number(value) } : {}),
                ...(key === "moment_z" ? { torque_z: Number(value) } : {}),
                ...(key === "fix_rz" ? { fix_rz: Boolean(value) } : {}),
              }
            : node,
        ),
      }));
      return;
    }

    if (isHeatBar) {
      setHeatBarModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode
            ? {
                ...node,
                ...(key === "x" ? { x: Number(value) } : {}),
                ...(key === "load_x" ? { heat_load: Number(value) } : {}),
                ...(key === "fix_x" ? { fix_temperature: Boolean(value) } : {}),
                ...(key === "temperature_delta" ? { temperature: Number(value) } : {}),
              }
            : node,
        ),
      }));
      return;
    }

    updateSelectedFrameNodeBase(key, value);
  };

  const updateSelectedFrameElement = (
    key:
      | keyof Frame2dJobInput["elements"][number]
      | keyof ThermalFrame2dJobInput["elements"][number]
      | "distributed_load_y",
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);

    if (isTorsion) {
      setTorsionModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement
            ? {
                ...element,
                ...(key === "youngs_modulus" ? { shear_modulus: value } : {}),
                ...(key === "moment_of_inertia" ? { polar_moment: value } : {}),
                ...(key === "section_modulus" ? { section_modulus: value } : {}),
              }
            : element,
        ),
      }));
      return;
    }

    if (isHeatBar) {
      setHeatBarModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement
            ? {
                ...element,
                ...(key === "area" ? { area: value } : {}),
                ...(key === "youngs_modulus" ? { conductivity: value } : {}),
              }
            : element,
        ),
      }));
      return;
    }

    if (isBeam) {
      setBeamModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }

    updateSelectedFrameElementBase(key, value);
  };

  const assignSelectedFrameElementMaterial = (materialId: string) => {
    if (selectedElement === null || isTorsion) return;
    assignSelectedFrameElementMaterialBase(materialId);
  };

  const toggleImmersiveViewport = async () => {
    const target = viewportPanelRef.current;
    if (!target) return;

    try {
      if (document.fullscreenElement === target) {
        await document.exitFullscreen();
        setMessage(t.immersiveModeDisabled);
      } else {
        await target.requestFullscreen();
        setMessage(t.immersiveModeEnabled);
      }
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const studyKindOptionGroups = buildStudyKindOptionGroups({
    kinds: t.kinds,
    domains: t.studyDomains,
    families: t.studyFamilies,
  });
  const studyDomainOptions = buildStudyDomainOptions(t.studyDomains);
  const currentStudyFamily = classifyStudyKindFamily(studyKind);
  const currentStudyFamilyLabel = t.studyFamilies[currentStudyFamily];
  const currentStudyFamilyHint = t.familyHints[currentStudyFamily];
  const { thermalIntentValue, thermalBoundaryValue, studySummaryRows, studyControlsRows, truss3dTreeRows } =
    buildWorkbenchStudySidebarData({
      t,
      language,
      studyKind,
      loadedModelName,
      activeMaterial,
      localMaterialLabel,
      fixed,
      isAxial,
      isSpring,
      isSpring1d,
      isSpring2d,
      isSpring3d,
      isBeam,
      isTorsion,
      isTruss,
      isTruss3d,
      isFrameLike,
      isFrame,
      isPlane,
      isHeatBar,
      isHeatPlane,
      isHeatPlaneTriangle,
      isHeatPlaneQuad,
      isThermal,
      isThermalBar,
      isThermalBeam,
      isThermalFrame,
      isThermalTruss2d,
      isThermalPlaneTriangle,
      isThermalPlaneQuad,
      axialForm,
      heatBarModel,
      heatPlaneModel,
      thermalBarModel,
      thermalBeamModel,
      thermalFrameModel,
      thermalTrussModel,
      thermalTruss3dModel,
      springModel,
      spring2dModel,
      spring3dModel,
      beamModel,
      torsionModel,
      trussModel,
      truss3dModel,
      frameModel,
      activePlaneInputModel,
      activeFrameLikeModel,
      displayTruss3dElements,
      truss3dTreeNodes: isSpring3d ? spring3dModel.nodes : truss3dModel.nodes,
      selectedNode,
      selectedTruss3dNodes,
      memberDraftNodes,
    });

  const studyKindResetHandlers = useMemo(
    () =>
      createStudyKindResetHandlers({
        activeMaterial,
        setPlaneModel,
        setHeatBarModel,
        setHeatPlaneModel,
        setThermalBarModel,
        setThermalBeamModel,
        setThermalFrameModel,
        setThermalTrussModel,
        setThermalTruss3dModel,
        setSpringModel,
        setSpring2dModel,
        setSpring3dModel,
        setBeamModel,
        setTorsionModel,
        setFrameModel,
        setPlaneResultField,
        ensurePlaneModelMaterials,
        ensureBeamModelMaterials,
        ensureFrameModelMaterials,
        defaultPlaneQuad,
        defaultThermalPlaneQuad,
        defaultPlaneTriangle,
        defaultThermalPlaneTriangle,
        defaultHeatBar1d,
        defaultHeatPlaneQuad,
        defaultHeatPlaneTriangle,
        defaultThermalBar1d,
        defaultThermalBeam1d,
        defaultThermalFrame2d,
        defaultThermalTruss2d,
        defaultThermalTruss3d,
        defaultSpring1d,
        defaultSpring2d,
        defaultSpring3d,
        defaultBeam1d,
        defaultTorsion1d,
        defaultFrame2d,
      }),
    [activeMaterial],
  );

  const selectStudyKind = (nextStudyKind: typeof studyKind) => {
    recordHistory(t.changeStudyType);
    applyStudyKindSelection({
      currentStudyKind: studyKind,
      nextStudyKind,
      setStudyKind,
      resetHandlers: studyKindResetHandlers,
    });
  };

  const openWorkspaceStudy = (tab: StudyPanelTab = "controls") => {
    setSidebarSection("model");
    setModelTab("tools");
    setModelToolsPage("study");
    setStudyTab(tab);
  };

  const { assistantCards, assistantPromptPresets, requestLlmAssistantPlan } = useWorkbenchAssistantController({
    t,
    language,
    studyKind,
    frontendRuntimeMode,
    selectedProjectId,
    directMeshEndpointsCount: directMeshEndpoints.length,
    hasHealth: Boolean(health),
    jobIsActive,
    isTruss,
    isTruss3d,
    immersiveViewport,
    hasAnyResult,
    trussDiagnostics,
    openProjects: () => {
      setSidebarSection("library");
      setLibraryTab("projects");
    },
    openSystemConfig: () => {
      setSidebarSection("system");
      setSystemPanelTab("config");
    },
    refreshHealth: () => {
      void refreshHealth();
    },
    cancelCurrentJob,
    applyTrussSuggestion,
    openSample,
    openWorkspaceStudy,
    runAnalysis,
    downloadResultCsv,
    toggleImmersiveViewport: () => {
      void toggleImmersiveViewport();
    },
    assistantApiBaseUrl,
    assistantApiKey,
    assistantModel,
    getScriptSnapshot,
  });

  const studyControlsContent = isAxial ? (
    <div className="form-grid compact">
      <label>
        <span>{t.length}</span>
        <input
          type="number"
          value={axialForm.length}
          min={0.1}
          step={0.1}
          onChange={(event) => handleAxialFieldChange("length", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.area}</span>
        <input
          type="number"
          value={axialForm.area}
          min={0.0001}
          step={0.0001}
          onChange={(event) => handleAxialFieldChange("area", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.material}</span>
        <select value={axialForm.material} onChange={(event) => handleMaterialChange(event.target.value)}>
          {MATERIAL_PRESETS.map((preset) => (
            <option key={preset.value} value={preset.value}>
              {localMaterialLabel(preset.value, language)}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{t.modulus}</span>
        <input
          type="number"
          value={axialForm.youngsModulusGpa}
          min={0.1}
          step={0.1}
          onChange={(event) => handleAxialFieldChange("youngsModulusGpa", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.elements}</span>
        <input
          type="number"
          value={axialForm.elements}
          min={1}
          max={120}
          step={1}
          onChange={(event) => handleAxialFieldChange("elements", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.tipForce}</span>
        <input
          type="number"
          value={axialForm.tipForce}
          step={100}
          onChange={(event) => handleAxialFieldChange("tipForce", Number(event.target.value))}
        />
      </label>
    </div>
  ) : null;

  const modelStudyContent: ReactNode = (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{t.sections.study}</h2>
        <span>{loadedModelName}</span>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{t.studyDomain}</span>
          <div className="button-row">
            {studyDomainOptions.map((option) => (
              <button
                key={option.key}
                className={`ghost-button ghost-button--compact${classifyStudyKindDomain(studyKind) === option.key ? " ghost-button--active" : ""}`}
                onClick={() => {
                  const fallback = studyKindOptionGroups.find((group) => group.domainKey === option.key)?.options[0]?.value;
                  if (fallback && fallback !== studyKind) {
                    selectStudyKind(fallback);
                  }
                }}
                type="button"
              >
                {option.label}
              </button>
            ))}
          </div>
        </label>
        <label>
          <span>{t.studyTypeLabel}</span>
          <select
            value={studyKind}
            onChange={(event) => {
              selectStudyKind(event.target.value as typeof studyKind);
            }}
          >
            {studyKindOptionGroups
              .filter((group) => group.options.length > 0)
              .map((group) => (
                <optgroup key={group.label} label={group.label}>
                  {group.options.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </optgroup>
              ))}
          </select>
        </label>
      </div>
      {studyControlsContent}
      <div className="sidebar-list">
        {studyControlsRows.slice(0, 4).map((row) => (
          <div key={row.label}>
            <span>{row.label}</span>
            <strong>{row.value}</strong>
          </div>
        ))}
      </div>
      <button className="solve-button" disabled={isPending} onClick={runAnalysis} type="button">
        {isPending ? t.running : t.run}
      </button>
    </section>
  );

  const modelStudioContent: ReactNode = (
    <WorkbenchModelToolsCard
      title={isTruss3d ? t.spaceStudio : t.sections.model}
      status={isTruss3d ? t.orbitHint : isFrameLike ? t.ready : t.dragToEdit}
      hint={isTruss3d ? t.spaceStudioHint : isPlane ? t.planeHint : isFrameLike ? t.frameEditorHint : isTorsion ? t.torsionHint : isThermal ? currentStudyFamilyHint : t.modelStudioHint}
      selectionHint={t.selectionHint}
      addNodeLabel={t.addNode}
      addBranchNodeLabel={t.addBranchNode}
      deleteNodeLabel={t.deleteNode}
      toggleMemberLabel={t.toggleMember}
      deleteMemberLabel={t.deleteMember}
      linkModeLabel={t.linkMode}
      linkModeActiveLabel={t.linkModeActive}
      linkModeHint={t.linkModeIdle}
      undoLabel={t.undo}
      redoLabel={t.redo}
      downloadLabel={t.download}
      saveForSolverLabel={t.saveForSolver}
      canAddBranchNode={selectedNode !== null}
      canDeleteNode={selectedNode !== null}
      canDeleteMember={selectedElement !== null}
      canUndo={undoStack.length > 0}
      canRedo={redoStack.length > 0}
      isTruss={isTruss}
      isFrame={isFrameLike}
      isTruss3d={isTruss3d}
      truss3dLinkMode={truss3dLinkMode}
      onAddNode={() => {
        if (isTruss3d) {
          addTruss3dNode(false);
          return;
        }
        addNode(false);
      }}
      onAddBranchNode={() => {
        if (isTruss3d) {
          addTruss3dNode(true);
          return;
        }
        addNode(true);
      }}
      onDeleteNode={() => {
        if (isTruss3d) {
          deleteSelectedTruss3dNode();
          return;
        }
        deleteSelectedNode();
      }}
      onToggleMember={() => {
        if (isTruss3d) {
          toggleTruss3dMemberFromDraft();
          return;
        }
        toggleMemberFromDraft();
      }}
      onDeleteMember={() => {
        if (isTruss3d) {
          deleteSelectedTruss3dElement();
          return;
        }
        deleteSelectedElement();
      }}
      onToggleLinkMode={toggleTruss3dLinkMode}
      onUndo={handleUndo}
      onRedo={handleRedo}
      onDownload={downloadModel}
      onSaveForSolver={() => {
        setStudyKind(isPlane || isBeam || isTorsion || isThermal ? studyKind : isTruss3d ? "truss_3d" : isFrameLike ? studyKind : "truss_2d");
        setSidebarSection("model");
        setMessage(isPlane ? t.planeHint : isBeam ? t.modelStudioHint : isTorsion ? t.torsionHint : isThermal ? currentStudyFamilyHint : isTruss3d ? t.switchedTo3dStudio : isFrameLike ? t.frameEditorHint : t.switchedTo2dStudio);
      }}
    />
  );

  const modelMaterialsContent: ReactNode = !isAxial && !isBeam && !isTorsion && !isHeatBar && !isThermalBar ? (
    <WorkbenchMaterialLibraryCard
      language={language}
      materialLabel={t.material}
      modulusLabel={t.modulus}
      poissonRatioLabel={t.poissonRatio}
      activeMaterial={activeMaterial}
      currentMaterials={currentMaterials}
      hiddenMaterialIds={hiddenMaterialIds}
      isPlane={isPlane}
      selectedElement={selectedElement}
      localMaterialLabel={localMaterialLabel}
      getMaterialColor={(materialId) => materialColorMap.get(materialId) ?? "#1677a3"}
      onActiveMaterialChange={setActiveMaterial}
      onAddMaterial={addMaterialToCurrentModel}
      onAddCustomMaterial={addCustomMaterialToCurrentModel}
      onImportMaterials={(file) => void importMaterials(file)}
      onUpdateMaterial={updateCurrentMaterial}
      onToggleMaterialVisibility={toggleMaterialVisibility}
      onApplyMaterial={applyMaterialToCurrentModel}
      onDeleteMaterial={deleteCurrentMaterial}
      round={round}
    />
  ) : null;

  const modelGenerateContent: ReactNode = !isTruss3d && !isFrameLike && !isBeam && !isTorsion && !isThermal ? (
    <WorkbenchParametricCard
      isPlane={isPlane}
      title={isPlane ? t.panelGenerator : t.parametric}
      subtitle={t.modelTools}
      lengthLabel={t.length}
      heightLabel={t.height}
      divisionsXLabel={t.divisionsX}
      divisionsYLabel={t.divisionsY}
      thicknessLabel={t.planeThickness}
      modulusLabel={t.modulus}
      poissonRatioLabel={t.poissonRatio}
      loadCaseLabel={t.loadCase}
      baysLabel={t.bays}
      areaLabel={t.area}
      generateLabel={isPlane ? t.generatePanel : t.generate}
      panelParametric={panelParametric}
      parametric={parametric}
      onPanelParametricChange={handlePanelParametricChange}
      onParametricChange={handleParametricChange}
      onGenerate={isPlane ? generatePanelModel : generateModel}
    />
  ) : null;

  const modelTreeContent: ReactNode = isTruss3d ? (
    <WorkbenchTruss3dTreeCard
      title={t.objectTree}
      countLabel={
        selectedTruss3dNodes.length > 1
          ? `${selectedTruss3dNodes.length} ${t.nodes}`
          : truss3dLinkMode
            ? t.linkModeActive
            : `${memberDraftNodes.length}/2`
      }
      hint={truss3dLinkMode ? t.linkModeIdle : t.orbitHint}
      nodeILabel={t.nodeI}
      nodeJLabel={t.nodeJ}
      areaLabel={t.area}
      nodes={truss3dTreeRows.nodes}
      elements={truss3dTreeRows.elements.map((element) => ({
        ...element,
        area: fixed(truss3dModel.elements[element.index]?.area, 4),
        active: selectedElement === element.index,
      }))}
      onSelectNode={handleTruss3dNodePick}
      onSelectElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        setSelectedTruss3dNodes([]);
        setMemberDraftNodes([]);
      }}
    />
  ) : (
    <WorkbenchObjectTree
      title={t.objectTree}
      scopeLabel={currentStudyFamilyLabel}
      countLabel={
        isTruss || isFrameLike
          ? `${memberDraftNodes.length}/2`
          : isBeam
            ? String(beamModel.elements.length)
            : isTorsion
              ? String(torsionModel.elements.length)
            : isHeatBar
              ? String(heatBarModel.elements.length)
            : isThermal
              ? String(thermalBarModel.elements.length)
            : isSpring1d
              ? String(springModel.elements.length)
              : isSpring2d
                ? String(spring2dModel.elements.length)
                : isSpring3d
                  ? String(spring3dModel.elements.length)
              : String(activePlaneInputModel.elements.length)
      }
      hint={isTorsion ? t.torsionHint : isThermal ? currentStudyFamilyHint : currentStudyFamilyHint}
      geometryLabel={t.geometry}
      resultsLabel={t.results}
      sortByLabel={t.sortBy}
      diagnosticsLabel={t.diagnostics}
      loadCaseLabel={t.loadCase}
      nodeJLabel={t.nodeJ}
      nodeKLabel={t.nodeK}
      elementValueLabel={isFrameLike || isBeam || isTorsion || isSpring || isHeatBar || isThermal ? frameTreeValueLabel : undefined}
      nodeRows={(isPlane ? activePlaneInputModel.nodes : isFrameLike ? activeFrameLikeModel.nodes : isBeam ? activeBeamLikeModel.nodes : isTorsion ? torsionModel.nodes : isHeatBar ? heatBarModel.nodes : isThermalBar ? thermalBarModel.nodes : isThermalTruss2d ? thermalTrussModel.nodes : isSpring1d ? springModel.nodes : isSpring2d ? spring2dModel.nodes : isSpring3d ? spring3dModel.nodes : trussModel.nodes).map((node) => ({
        id: node.id,
        x: node.x,
        y: Number("y" in node ? node.y : 0),
        load_y: "load_y" in node ? node.load_y : "torque_z" in node ? node.torque_z : "load_x" in node ? node.load_x : 0,
      }))}
      elementRows={
        isPlane
          ? planeElements.map((element) => ({
              id: element.id,
              node_i: element.node_i,
              node_j: "node_j" in element ? element.node_j : undefined,
              node_k: "node_k" in element ? element.node_k : undefined,
            }))
          : displayTrussElements.map((element) => ({
              id: element.id,
              node_i: element.node_i,
              node_j: element.node_j,
              resultMagnitude: isFrameLike || isBeam || isSpring || isHeatBar || isThermal ? lineResultFieldValue(element, activeLineResultField) : undefined,
              resultValue: isFrameLike || isBeam || isSpring || isHeatBar || isThermal ? scientific(lineResultFieldValue(element, activeLineResultField)) : undefined,
            }))
      }
      isPlane={isPlane}
      isTruss={isTruss}
      enableElementResultsMode={isFrameLike || isBeam || isTorsion || isSpring || isHeatBar || isThermal}
      selectedNode={selectedNode}
      selectedElement={selectedElement}
      nodeIssueCounts={Object.fromEntries(
        Object.entries(trussDiagnostics?.nodeIssues ?? {}).map(([key, issues]) => [Number(key), issues.length]),
      )}
      onSelectNode={(index) => {
        if (isPlane) {
          setSelectedNode(index);
          setSelectedElement(null);
        } else if (isBeam || isTorsion || isSpring || isThermal) {
          setSelectedNode(index);
          setSelectedElement(null);
          setMemberDraftNodes([]);
        } else {
          toggleDraftNode(index);
        }
      }}
      onSelectElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        if (isTruss || isFrameLike || isBeam || isTorsion || isSpring || isThermal) setMemberDraftNodes([]);
        if (isFrameLike || isBeam || isTorsion || isSpring || isThermal) setFocusedFrameElement(index);
      }}
    />
  );

  return (
    <div className="workbench-shell">
      <WorkbenchSidebarMount
        shortTitle={t.shortTitle}
        roleLabel={t.roleLabel}
        title={t.title}
        subtitle={t.subtitle}
        railItems={railItems}
        sidebarSection={sidebarSection}
        onSidebarSectionChange={handleSidebarSectionChange}
        studySection={
          <WorkbenchStudySectionMount
            studyTab={studyTab}
            onStudyTabChange={handleStudyTabChange}
            sectionTitle={t.sections.study}
            summaryTabLabel={t.tabs.summary}
            controlsTabLabel={t.tabs.controls}
            loadedModelName={loadedModelName}
            studyTypeLabel={t.studyTypeLabel}
            studyKind={studyKind}
            studyDomainLabel={t.studyDomain}
            studyDomainOptions={studyDomainOptions}
            noDomainStudiesLabel={t.noDomainStudies}
            studyKindOptionGroups={studyKindOptionGroups}
            onStudyKindChange={selectStudyKind}
            summaryRows={studySummaryRows}
            controlsRows={studyControlsRows}
            controlsContent={studyControlsContent}
            controlsTitle={t.controls}
            controlsSetupPageLabel={t.controlsSetupPage}
            controlsReviewPageLabel={t.controlsReviewPage}
            readyLabel={t.ready}
            busyLabel={t.busy}
            isPending={isPending}
            runLabel={t.run}
            runningLabel={t.running}
            onRun={runAnalysis}
          />
        }
        modelSection={
          <WorkbenchModelSectionMount
            modelTab={modelTab}
            onModelTabChange={handleModelTabChange}
            toolsPage={modelToolsPage}
            onToolsPageChange={handleModelToolsPageChange}
            isTruss3d={isTruss3d}
            toolsTabLabel={t.tabs.tools}
            treeTabLabel={t.tabs.tree}
            toolsPageOverviewLabel={t.modelOverviewPage}
            toolsPageStudyLabel={t.modelStudyPage}
            toolsPageStudioLabel={t.modelStudioPage}
            toolsPageMaterialsLabel={t.modelMaterialsPage}
            toolsPageGenerateLabel={t.modelGeneratePage}
            studyOverviewHint={t.workspaceStudyHint}
            studioOverviewHint={t.workspaceStudioHint}
            materialsOverviewHint={t.workspaceMaterialsHint}
            generateOverviewHint={t.workspaceGenerateHint}
            browseOverviewHint={t.workspaceBrowseHint}
            studyContent={modelStudyContent}
            studioContent={modelStudioContent}
            materialsContent={modelMaterialsContent}
            generateContent={modelGenerateContent}
            treeContent={modelTreeContent}
          />
        }
        workflowSection={
          <WorkbenchWorkflowSectionMount
            surfaceTab={workflowPanelTab}
            onSurfaceTabChange={handleWorkflowPanelTabChange}
            labels={{
              sectionTitle: t.sections.workflow,
              overviewPageLabel: t.workflowOverviewPage,
              catalogPageLabel: t.workflowCatalogPage,
              builderPageLabel: t.workflowBuilderPage,
              runsPageLabel: t.workflowRunsPage,
              overviewHint: t.workflowOverviewHint,
              catalogHint: t.workflowCatalogHint,
              builderHint: t.workflowBuilderHint,
              runsHint: t.workflowRunsHint,
              catalogTitle: t.workflowCatalogTitle,
              refreshLabel: t.workflowCatalogRefresh,
              runLabel: t.workflowCatalogRun,
              emptyCatalogLabel: t.workflowCatalogEmpty,
              noSelectionLabel: t.workflowNoSelection,
              nodesTitle: t.workflowNodesTitle,
              edgesTitle: t.workflowEdgesTitle,
              entryInputsTitle: t.workflowEntryInputsTitle,
              outputArtifactsTitle: t.workflowOutputArtifactsTitle,
              datasetContractTitle: t.workflowDatasetContractTitle,
              datasetValuesTitle: t.workflowDatasetValuesTitle,
              datasetValueLabel: t.workflowDatasetValueLabel,
              datasetSemanticTypeLabel: t.workflowDatasetSemanticTypeLabel,
              datasetEncodingLabel: t.workflowDatasetEncodingLabel,
              datasetShapeLabel: t.workflowDatasetShapeLabel,
              datasetAxesLabel: t.workflowDatasetAxesLabel,
              datasetSchemaLabel: t.workflowDatasetSchemaLabel,
              datasetClassLabel: t.workflowDatasetClassLabel,
              datasetNoneLabel: t.workflowDatasetNoneLabel,
              datasetDraftHint: t.workflowDatasetDraftHint,
              datasetEditorTitle: t.workflowDatasetEditorTitle,
              datasetValueSelectLabel: t.workflowDatasetValueSelectLabel,
              datasetUnitLabel: t.workflowDatasetUnitLabel,
              datasetMetadataLabel: t.workflowDatasetMetadataLabel,
              datasetPortMappingsTitle: t.workflowDatasetPortMappingsTitle,
              datasetEdgeMappingsTitle: t.workflowDatasetEdgeMappingsTitle,
              datasetDraftLocalLabel: t.workflowDatasetDraftLocalLabel,
              datasetUnassignedLabel: t.workflowDatasetUnassignedLabel,
              exportGraphLabel: t.workflowExportGraphLabel,
              exportDatasetContractLabel: t.workflowExportDatasetContractLabel,
              operatorLabel: t.workflowOperatorLabel,
              kindLabel: t.workflowKindLabel,
              progressLabel: t.workflowProgressLabel,
              currentNodeLabel: t.workflowCurrentNodeLabel,
              latestSummaryLabel: t.workflowLatestSummaryLabel,
              openRunLabel: t.workflowOpenRunLabel,
              emptyRunsLabel: t.workflowRunsEmpty,
              selectForBuilderLabel: t.workflowSelectForBuilder,
              statusReadyLabel: t.ready,
              statusBusyLabel: t.busy,
            }}
            workflowCatalogEntries={workflowCatalog}
            workflowCatalogBusy={workflowCatalogBusy}
            selectedWorkflowId={selectedWorkflowId}
            selectedWorkflow={selectedWorkflow}
            latestJob={job}
            latestWorkflowSummary={latestWorkflowSummary}
            workflowRuns={workflowRuns}
            refreshWorkflowCatalog={refreshWorkflowCatalog}
            setSelectedWorkflowId={setSelectedWorkflowId}
            runWorkflowCatalogEntry={runWorkflowCatalogEntry}
            openHistoryJob={openHistoryJob}
          />
        }
        librarySection={
          <WorkbenchLibrarySectionMount
            labels={t}
            libraryTab={libraryTab}
            onLibraryTabChange={handleLibraryTabChange}
            sampleRows={librarySampleRows}
            workflowCatalogEntries={workflowCatalog}
            workflowCatalogBusy={workflowCatalogBusy}
            projects={projects}
            selectedProjectId={selectedProjectId}
            setSelectedProjectId={setSelectedProjectId}
            setSelectedModelId={setSelectedModelId}
            projectNameDraft={projectNameDraft}
            setProjectNameDraft={setProjectNameDraft}
            projectDescriptionDraft={projectDescriptionDraft}
            setProjectDescriptionDraft={setProjectDescriptionDraft}
            createProjectRecord={createProjectRecord}
            updateProjectRecord={updateProjectRecord}
            deleteProjectRecord={deleteProjectRecord}
            downloadProjectBundleJson={downloadProjectBundleJson}
            downloadProjectBundleZip={downloadProjectBundleZip}
            importProjectBundle={importProjectBundle}
            selectedProjectModels={selectedProjectModels}
            modelRows={libraryModelRows}
            selectedModelId={selectedModelId}
            loadedModelName={loadedModelName}
            setLoadedModelName={setLoadedModelName}
            saveModelVersion={saveModelVersion}
            deleteSavedModelRecord={deleteSavedModelRecord}
            openSavedModel={openSavedModel}
            versionRows={libraryVersionRows}
            modelVersions={modelVersions}
            selectedVersionId={selectedVersionId}
            renameSelectedVersion={renameSelectedVersion}
            deleteSelectedVersion={deleteSelectedVersion}
            openSavedVersion={openSavedVersion}
            jobRows={libraryJobRows}
            jobCount={jobHistory.length}
            activeJobId={job?.job_id ?? null}
            openHistoryJob={openHistoryJob}
            openSample={openSample}
            refreshWorkflowCatalog={refreshWorkflowCatalog}
            runWorkflowCatalogEntry={runWorkflowCatalogEntry}
            refreshJobHistory={refreshJobHistory}
            refreshProjects={refreshProjects}
            importModel={importModel}
          />
        }
        systemSection={
          <WorkbenchSystemSidebarMount
            t={t}
            systemPanelTab={systemPanelTab === "assistant" ? "config" : systemPanelTab}
            handleSystemPanelTabChange={handleSystemPanelTabChange}
            healthStatus={health?.status}
            healthProtocolOnline={Boolean(health?.protocol)}
            healthWatchdogOnline={Boolean(health?.watchdog)}
            healthSecurityApiTokenConfigured={Boolean(health?.security?.api_token_configured)}
            runtimeBackendRows={runtimeBackendRows}
            runtimeProtocolRows={runtimeProtocolRows}
            runtimeProtocolMethods={runtimeProtocolMethods}
            securityUi={securityUi}
            runtimeSecurityRows={runtimeSecurityRows}
            runtimeAuditSummaryRows={runtimeAuditSummaryRows}
            runtimeAuditTrendBars={runtimeAuditTrendBars}
            runtimeAuditSourceStatusFacets={runtimeAuditSourceStatusFacets}
            runtimeAuditStudyFacets={runtimeAuditStudyFacets}
            runtimeAuditProjectFacets={runtimeAuditProjectFacets}
            runtimeAuditModelVersionFacets={runtimeAuditModelVersionFacets}
            securityEventRecords={securityEventRecords}
            securityEventWindowFilter={securityEventWindowFilter}
            securityEventSourceFilter={securityEventSourceFilter}
            securityEventRiskFilter={securityEventRiskFilter}
            securityEventStatusFilter={securityEventStatusFilter}
            securityEventActionFilter={securityEventActionFilter}
            setSecurityEventWindowFilter={setSecurityEventWindowFilter}
            setSecurityEventSourceFilter={setSecurityEventSourceFilter}
            setSecurityEventRiskFilter={setSecurityEventRiskFilter}
            setSecurityEventStatusFilter={setSecurityEventStatusFilter}
            setSecurityEventActionFilter={setSecurityEventActionFilter}
            refreshSecurityEvents={refreshSecurityEvents}
            downloadSecurityEventExport={downloadSecurityEventExport}
            downloadSecurityEventCsvExport={downloadSecurityEventCsvExport}
            runtimeAuditEntries={runtimeAuditEntries}
            protocolAgents={protocolAgents}
            protocolAgentCards={protocolAgentCards}
            runtimeWatchdogRows={runtimeWatchdogRows}
            theme={theme}
            language={language}
            frontendRuntimeMode={frontendRuntimeMode}
            directMeshSelectionMode={directMeshSelectionMode}
            directMeshEndpointsText={directMeshEndpointsText}
            controlPlaneApiToken={controlPlaneApiToken}
            clusterApiToken={clusterApiToken}
            directMeshApiToken={directMeshApiToken}
            showShortcutHints={showShortcutHints}
            immersiveGuardrails={immersiveGuardrails}
            languagePacks={languagePacks}
            languagePackCatalogRows={languagePackCatalogRows}
            setTheme={setTheme}
            handleLanguageChange={handleLanguageChange}
            handleDownloadLanguagePackTemplate={handleDownloadLanguagePackTemplate}
            handleExportInstalledLanguagePack={handleExportInstalledLanguagePack}
            handleImportLanguagePack={handleImportLanguagePack}
            handleRemoveLanguagePack={handleRemoveLanguagePack}
            setFrontendRuntimeMode={setFrontendRuntimeMode}
            setDirectMeshSelectionMode={setDirectMeshSelectionMode}
            setDirectMeshEndpointsText={setDirectMeshEndpointsText}
            setControlPlaneApiToken={setControlPlaneApiToken}
            setClusterApiToken={setClusterApiToken}
            setDirectMeshApiToken={setDirectMeshApiToken}
            setShowShortcutHints={setShowShortcutHints}
            setImmersiveGuardrails={setImmersiveGuardrails}
            downloadDatabaseSnapshot={downloadDatabaseSnapshot}
            scriptActionLog={scriptActionLog}
            getScriptSnapshot={getScriptSnapshot}
            scriptRecordingMode={scriptRecordingMode}
            invokeScriptAction={async (action, payload) => {
              await invokeScriptAction(action, payload);
              return {};
            }}
            setScriptRecordingMode={setScriptRecordingMode}
            scriptSnapshot={scriptSnapshot}
            systemDataTab={systemDataTab}
            handleSystemDataTabChange={handleSystemDataTabChange}
            adminJobRows={adminJobRows}
            selectedAdminJobId={selectedAdminJobId}
            handleSelectAdminJob={handleSelectAdminJob}
            selectedAdminJob={selectedAdminJob}
            adminJobMessage={adminJobMessage}
            setAdminJobMessage={setAdminJobMessage}
            adminJobProjectId={adminJobProjectId}
            setAdminJobProjectId={setAdminJobProjectId}
            adminJobModelVersionId={adminJobModelVersionId}
            setAdminJobModelVersionId={setAdminJobModelVersionId}
            adminJobCaseId={adminJobCaseId}
            setAdminJobCaseId={setAdminJobCaseId}
            saveAdminJobRecord={saveAdminJobRecord}
            deleteAdminJobRecord={deleteAdminJobRecord}
            adminResultRows={adminResultRows}
            selectedAdminResultJobId={selectedAdminResultJobId}
            handleSelectAdminResult={handleSelectAdminResult}
            jobHistory={jobHistory}
            adminResultDraft={adminResultDraft}
            setAdminResultDraft={setAdminResultDraft}
            saveAdminResultRecord={saveAdminResultRecord}
            applySelectedAdminResultContext={applySelectedAdminResultContext}
            openSelectedAdminResultProject={openSelectedAdminResultProject}
            openSelectedAdminResultVersion={openSelectedAdminResultVersion}
            exportAdminResultRecord={exportAdminResultRecord}
            deleteAdminResultRecord={deleteAdminResultRecord}
            adminFilterProjectId={adminFilterProjectId}
            handleAdminFilterProjectChange={handleAdminFilterProjectChange}
            adminFilterModelVersionId={adminFilterModelVersionId}
            handleAdminFilterModelVersionChange={handleAdminFilterModelVersionChange}
            selectedProjectId={selectedProjectId}
            selectedVersionId={selectedVersionId}
            useCurrentProjectAsAdminFilter={useCurrentProjectAsAdminFilter}
            useCurrentVersionAsAdminFilter={useCurrentVersionAsAdminFilter}
            clearAdminFilters={clearAdminFilters}
            applySelectedAdminJobContext={applySelectedAdminJobContext}
            openSelectedAdminJobProject={openSelectedAdminJobProject}
            openSelectedAdminJobVersion={openSelectedAdminJobVersion}
            jobId={job?.job_id ?? null}
            cancelCurrentJob={cancelCurrentJob}
            cancelJob={cancelJob}
            setMessage={setMessage}
            refreshJobHistory={refreshJobHistory}
          />
        }
      />

      <WorkbenchMainShellMount
        t={t}
        assistantWindowOpen={assistantWindowOpen}
        setAssistantWindowOpen={setAssistantWindowOpen}
        job={job}
        hasAnyResult={hasAnyResult}
        frontendRuntimeMode={frontendRuntimeMode}
        studyKind={studyKind}
        language={language}
        assistantApiKey={assistantApiKey}
        assistantApiBaseUrl={assistantApiBaseUrl}
        assistantModel={assistantModel}
        assistantCards={assistantCards}
        assistantMode={assistantMode}
        assistantPromptPresets={assistantPromptPresets}
        assistantTransactions={assistantTransactions}
        executeAssistantPlan={executeAssistantPlan}
        invokeScriptAction={invokeScriptAction}
        setAssistantApiKey={setAssistantApiKey}
        setAssistantApiBaseUrl={setAssistantApiBaseUrl}
        setAssistantModel={setAssistantModel}
        setAssistantMode={setAssistantMode}
        requestLlmAssistantPlan={requestLlmAssistantPlan}
        rollbackAssistantTransaction={rollbackAssistantTransaction}
        viewportPanelRef={viewportPanelRef}
        canvasStageRef={canvasStageRef}
        viewportPixelWidth={viewportPixelWidth}
        immersiveViewport={immersiveViewport}
        sidebarSection={sidebarSection}
        modelTab={modelTab}
        modelToolsPage={modelToolsPage}
        isTruss3d={isTruss3d}
        immersiveToolDrawerOpen={immersiveToolDrawerOpen}
        immersiveHelpDrawerOpen={immersiveHelpDrawerOpen}
        handleSidebarSectionChange={handleSidebarSectionChange}
        setModelTab={setModelTab}
        setModelToolsPage={setModelToolsPage}
        handleToggleImmersiveToolDrawer={handleToggleImmersiveToolDrawer}
        handleToggleImmersiveHelpDrawer={handleToggleImmersiveHelpDrawer}
        handleToggleImmersiveViewport={handleToggleImmersiveViewport}
        hasViewportDock={hasViewportDock}
        showShortcutHints={showShortcutHints}
        showViewportToolStrip={showViewportToolStrip}
        immersiveToolTab={immersiveToolTab}
        truss3dViewPreset={truss3dViewPreset}
        selectedNode={selectedNode}
        selectedElement={selectedElement}
        selectedTruss3dNodes={selectedTruss3dNodes}
        selectedTruss3dNodeData={selectedTruss3dNodeData}
        truss3dLinkMode={truss3dLinkMode}
        truss3dProjectionMode={truss3dProjectionMode}
        truss3dShowGrid={truss3dShowGrid}
        truss3dShowLabels={truss3dShowLabels}
        truss3dShowNodes={truss3dShowNodes}
        truss3dBoxSelectMode={truss3dBoxSelectMode}
        truss3dNudgeStep={truss3dNudgeStep}
        truss3dBatchLoadX={truss3dBatchLoadX}
        truss3dBatchLoadY={truss3dBatchLoadY}
        truss3dBatchLoadZ={truss3dBatchLoadZ}
        undoStack={undoStack}
        redoStack={redoStack}
        truss3dModel={truss3dModel}
        setImmersiveToolTab={setImmersiveToolTab}
        handleTruss3dViewPresetChange={handleTruss3dViewPresetChange}
        handleTruss3dFocusViewport={handleTruss3dFocusViewport}
        setTruss3dFocusRequestVersion={setTruss3dFocusRequestVersion}
        handleTruss3dProjectionModeChange={handleTruss3dProjectionModeChange}
        setTruss3dShowGrid={setTruss3dShowGrid}
        setTruss3dShowLabels={setTruss3dShowLabels}
        setTruss3dShowNodes={setTruss3dShowNodes}
        handleTruss3dBoxSelectModeChange={handleTruss3dBoxSelectModeChange}
        handleTruss3dResetViewport={handleTruss3dResetViewport}
        addTruss3dNode={addTruss3dNode}
        deleteSelectedTruss3dNode={deleteSelectedTruss3dNode}
        handleToggleTruss3dLinkMode={handleToggleTruss3dLinkMode}
        toggleTruss3dMemberFromDraft={toggleTruss3dMemberFromDraft}
        deleteSelectedTruss3dElement={deleteSelectedTruss3dElement}
        cloneSelectedTruss3dNodes={cloneSelectedTruss3dNodes}
        handleUndo={handleUndo}
        handleRedo={handleRedo}
        updateSelectedTruss3dNode={updateSelectedTruss3dNode}
        nudgeSelectedTruss3dNodes={nudgeSelectedTruss3dNodes}
        setTruss3dNudgeStep={setTruss3dNudgeStep}
        updateSelectedTruss3dNodes={updateSelectedTruss3dNodes}
        setTruss3dBatchLoadX={setTruss3dBatchLoadX}
        setTruss3dBatchLoadY={setTruss3dBatchLoadY}
        setTruss3dBatchLoadZ={setTruss3dBatchLoadZ}
        applySelectedTruss3dLoads={applySelectedTruss3dLoads}
        activeResultWindow={activeResultWindow}
        resultWindowStart={resultWindowStart}
        resultWindowEnd={resultWindowEnd}
        activeResultWindowLimit={activeResultWindowLimit}
        resultWindowOffset={resultWindowOffset}
        resultWindowMaxTotal={resultWindowMaxTotal}
        resultWindowJumps={resultWindowJumps}
        isPlane={isPlane}
        isHeatPlane={isHeatPlane}
        isHeatPlaneTriangle={isHeatPlaneTriangle}
        isThermalPlaneTriangle={isThermalPlaneTriangle}
        isThermalPlaneQuad={isThermalPlaneQuad}
        isFrameLike={isFrameLike}
        isThermalFrame={isThermalFrame}
        isBeam={isBeam}
        isTorsion={isTorsion}
        planeResult={planeResult}
        activeFrameLikeResult={activeFrameLikeResult}
        activeBeamLikeResult={activeBeamLikeResult}
        torsionResult={torsionResult}
        planeResultField={planeResultField}
        frameResultField={frameResultField}
        beamResultField={beamResultField}
        setPlaneResultField={setPlaneResultField}
        setFrameResultField={setFrameResultField}
        setBeamResultField={setBeamResultField}
        setResultWindowOffset={setResultWindowOffset}
        clampChunkOffset={clampChunkOffset}
        shouldStretchSpaceViewport={shouldStretchSpaceViewport}
        handleCanvasStageScroll={handleCanvasStageScroll}
        isSpring2d={isSpring2d}
        isSpring1d={isSpring1d}
        isHeatBar={isHeatBar}
        isThermalBar={isThermalBar}
        isThermalBeam={isThermalBeam}
        isThermalTruss2d={isThermalTruss2d}
        isFrame={isFrame}
        isSpring={isSpring}
        isSpring3d={isSpring3d}
        isThermalTruss3d={isThermalTruss3d}
        isTruss={isTruss}
        isThermal={isThermal}
        hiddenMaterialIds={hiddenMaterialIds}
        frameLegendText={frameLegendText}
        planeLegendText={planeLegendText}
        axialNodes={axialNodes}
        axialLength={axialLength}
        axialScale={axialScale}
        displayTrussNodes={displayTrussNodes}
        displayTrussElements={displayTrussElements}
        displayTruss3dNodes={displayTruss3dNodes}
        displayTruss3dElements={displayTruss3dElements}
        planeNodes={planeNodes}
        planeElements={planeElements}
        trussElementColors={trussElementColors}
        truss3dElementColors={truss3dElementColors}
        planeElementColors={planeElementColors}
        trussBounds={trussBounds}
        planeBounds={planeBounds}
        trussResult={trussResult}
        thermalTrussResult={thermalTrussResult}
        heatBarResult={heatBarResult}
        thermalBarResult={thermalBarResult}
        thermalTruss3dResult={thermalTruss3dResult}
        springResult={springResult}
        spring2dResult={spring2dResult}
        spring3dResult={spring3dResult}
        heatPlaneQuadResult={heatPlaneQuadResult}
        heatPlaneTriangleResult={heatPlaneTriangleResult}
        activeLineResultField={activeLineResultField}
        frameResultFieldMax={frameResultFieldMax}
        focusedFrameElement={focusedFrameElement}
        trussStability={trussStability}
        trussDiagnostics={trussDiagnostics}
        memberDraftNodes={memberDraftNodes}
        stopDraggingNode={stopDraggingNode}
        handleTrussPointerMove={handleTrussPointerMove}
        setSelectedElement={setSelectedElement}
        setSelectedNode={setSelectedNode}
        setMemberDraftNodes={setMemberDraftNodes}
        setFocusedFrameElement={setFocusedFrameElement}
        startTrussNodeDrag={startTrussNodeDrag}
        planeResultFieldMax={planeResultFieldMax}
        selectedPlaneNodeData={selectedPlaneNodeData}
        focusedPlaneElement={focusedPlaneElement}
        handleTruss3dNodePick={handleTruss3dNodePick}
        setSelectedTruss3dNodes={setSelectedTruss3dNodes}
        updateTruss3dNodePosition={updateTruss3dNodePosition}
        recordHistory={recordHistory}
        drag3dHistoryCapturedRef={drag3dHistoryCapturedRef}
        truss3dFocusRequestVersion={truss3dFocusRequestVersion}
        truss3dResetRequestVersion={truss3dResetRequestVersion}
        handleTruss3dNodesBoxSelect={handleTruss3dNodesBoxSelect}
        handleTruss3dShowGridChange={handleTruss3dShowGridChange}
        handleTruss3dShowLabelsChange={handleTruss3dShowLabelsChange}
        handleTruss3dShowNodesChange={handleTruss3dShowNodesChange}
        librarySampleRows={librarySampleRows}
        libraryModelRows={libraryModelRows}
        libraryJobRows={libraryJobRows}
        selectedProjectModels={selectedProjectModels}
        openSample={openSample}
        openSavedModel={openSavedModel}
        openHistoryJob={openHistoryJob}
        message={message}
        isAxial={isAxial}
        selectedNodeIssues={selectedNodeIssues}
        selectedNodeData={selectedNodeData}
        selectedBeamNodeData={selectedBeamNodeData}
        selectedTorsionNodeData={selectedTorsionNodeData}
        selectedFrameNodeData={selectedFrameNodeData}
        selectedThermalNodeData={selectedThermalNodeData}
        spring3dModel={spring3dModel}
        axialElements={axialElements}
        currentStudyFamilyLabel={currentStudyFamilyLabel}
        currentStudyFamilyHint={currentStudyFamilyHint}
        isPending={isPending}
        canProjectHeatToThermo={canProjectHeatToThermo}
        selectedElementData={selectedElementData}
        selectedTruss3dElementData={selectedTruss3dElementData}
        selectedPlaneElementData={selectedPlaneElementData}
        selectedFrameElementData={selectedFrameElementData}
        selectedBeamElementData={selectedBeamElementData}
        selectedTorsionElementData={selectedTorsionElementData}
        selectedThermalElementData={selectedThermalElementData}
        selectedSpringElementData={selectedSpringElementData}
        materialOptions={materialOptions}
        nodeCount={nodeCount}
        tipDisplacement={isAxial ? scientific(axialResult?.tip_displacement) : isHeatBar ? scientific(heatBarResult?.max_temperature) : isHeatPlane ? scientific(isHeatPlaneTriangle ? heatPlaneTriangleResult?.max_temperature : heatPlaneQuadResult?.max_temperature) : isThermalTruss2d ? scientific(thermalTrussResult?.max_displacement) : studyKind === "thermal_truss_3d" ? scientific(thermalTruss3dResult?.max_displacement) : isTruss ? scientific(trussResult?.max_displacement) : isSpring3d ? scientific(spring3dResult?.max_displacement) : studyKind === "truss_3d" ? scientific(truss3dResult?.max_displacement) : isThermalBar ? scientific(thermalBarResult?.max_displacement) : isSpring ? scientific(activeSpringResult?.max_displacement) : isBeam ? scientific(activeBeamLikeResult?.max_displacement) : isTorsion ? scientific(torsionResult?.max_rotation) : isFrameLike ? scientific(activeFrameLikeResult?.max_displacement) : scientific(planeResult?.max_displacement)}
        maxStressValue={scientific(isAxial ? axialResult?.max_stress : isHeatBar ? heatBarResult?.max_heat_flux : isHeatPlane ? (isHeatPlaneTriangle ? heatPlaneTriangleResult?.max_heat_flux : heatPlaneQuadResult?.max_heat_flux) : isThermalTruss2d ? thermalTrussResult?.max_stress : studyKind === "thermal_truss_3d" ? thermalTruss3dResult?.max_stress : isTruss ? trussResult?.max_stress : isSpring3d ? spring3dResult?.max_force : studyKind === "truss_3d" ? truss3dResult?.max_stress : isThermalBar ? thermalBarResult?.max_stress : isSpring ? activeSpringResult?.max_force : isBeam ? activeBeamLikeResult?.max_stress : isTorsion ? torsionResult?.max_stress : isFrameLike ? activeFrameLikeResult?.max_stress : planeResult?.max_stress)}
        frameMaxAxialForce={frameMaxAxialForce}
        frameMaxShearForce={frameMaxShearForce}
        reactionValue={isAxial ? scientific(axialResult?.reaction_force) : isHeatBar ? scientific(heatBarResult?.max_heat_flux) : isThermalBar ? scientific(thermalBarResult?.max_axial_force) : isThermalTruss2d ? scientific(thermalTrussResult?.max_axial_force) : isThermalTruss3d ? scientific(thermalTruss3dResult?.max_axial_force) : isSpring ? scientific(activeSpringResult?.max_force) : isTorsion ? scientific(torsionResult?.max_torque) : isFrameLike ? scientific(activeFrameLikeResult?.max_moment) : isBeam ? scientific(activeBeamLikeResult?.max_moment) : "--"}
        frameMaxRotationValue={isFrameLike ? scientific(activeFrameLikeResult?.max_rotation) : isBeam ? scientific(activeBeamLikeResult?.max_rotation) : isTorsion ? scientific(torsionResult?.max_rotation) : undefined}
        thermalPlaneMaxTemperatureDelta={planeResult && "max_temperature_delta" in planeResult ? planeResult.max_temperature_delta : undefined}
        thermalFrameMaxTemperatureDelta={thermalFrameMaxTemperatureDelta}
        thermalFrameMaxTemperatureGradient={thermalFrameMaxTemperatureGradient}
        thermalBeamMaxTemperatureGradient={thermalBeamMaxTemperatureGradient}
        planeHotspotFieldLabel={planeResultFieldLabel}
        planeHotspotElements={planeHotspotElements}
        planeThermalRows={planeThermalRows}
        frameHotspotFieldLabel={frameResultFieldLabel}
        frameHotspotElements={frameHotspotElements}
        frameForceRows={frameForceRows}
        planeHotspotLimit={planeHotspotLimit}
        heartbeatStatusValue={heartbeatStatusValue}
        heartbeatToneValue={heartbeatToneValue}
        translatedFailureReason={translatedFailureReason}
        jobIsActive={jobIsActive}
        activePlaneInputModel={activePlaneInputModel}
        planeModel={planeModel}
        activeFrameLikeModel={activeFrameLikeModel}
        activeBeamLikeModel={activeBeamLikeModel}
        trussModel={trussModel}
        thermalTrussModel={thermalTrussModel}
        thermalTruss3dModel={thermalTruss3dModel}
        updateSelectedElement={updateSelectedElement}
        assignSelectedElementMaterial={assignSelectedElementMaterial}
        updateSelectedTruss3dElement={updateSelectedTruss3dElement}
        assignSelectedTruss3dElementMaterial={assignSelectedTruss3dElementMaterial}
        updateSelectedPlaneNode={updateSelectedPlaneNode}
        updateSelectedPlaneElement={updateSelectedPlaneElement}
        assignSelectedPlaneElementMaterial={assignSelectedPlaneElementMaterial}
        updateSelectedFrameNode={updateSelectedFrameNode}
        updateSelectedFrameElement={updateSelectedFrameElement}
        assignSelectedFrameElementMaterial={assignSelectedFrameElementMaterial}
        applyTrussSuggestion={applyTrussSuggestion}
        cancelCurrentJob={cancelCurrentJob}
        downloadResultJson={downloadResultJson}
        downloadResultCsv={downloadResultCsv}
        projectHeatToThermoStudy={projectHeatToThermoStudy}
        downloadPlaneHotspotSummary={downloadPlaneHotspotSummary}
        downloadFrameHotspotSummary={downloadFrameHotspotSummary}
        downloadFrameForceSummary={downloadFrameForceSummary}
        setSidebarSection={setSidebarSection}
        setFocusedPlaneElement={setFocusedPlaneElement}
        setPlaneHotspotLimit={setPlaneHotspotLimit}
      />
    </div>
  );
}
