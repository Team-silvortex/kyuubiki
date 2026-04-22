"use client";

import {
  useDeferredValue,
  useEffect,
  useRef,
  useState,
  useTransition,
  type Dispatch,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type UIEvent as ReactUIEvent,
  type SetStateAction,
} from "react";
import brand from "../../../../../assets/brand/brand.json";
import { VirtualList } from "@/components/ui/virtual-list";
import { WorkbenchAssistantPanel } from "@/components/workbench/workbench-assistant-panel";
import { WorkbenchConsole } from "@/components/workbench/workbench-console";
import { WorkbenchInspector } from "@/components/workbench/workbench-inspector";
import { WorkbenchLibrarySidebar } from "@/components/workbench/workbench-library-sidebar";
import { WorkbenchMaterialLibraryCard } from "@/components/workbench/workbench-material-library-card";
import { WorkbenchModelSidebar } from "@/components/workbench/workbench-model-sidebar";
import { WorkbenchObjectTree } from "@/components/workbench/workbench-object-tree";
import { WorkbenchParametricCard } from "@/components/workbench/workbench-parametric-card";
import { WorkbenchScriptPanel } from "@/components/workbench/workbench-script-panel";
import { WorkbenchStudySidebar } from "@/components/workbench/workbench-study-sidebar";
import { WorkbenchViewport } from "@/components/workbench/workbench-viewport";
import { requestWorkbenchAssistantPlan, type AssistantPlan } from "@/lib/assistant/openai-compatible";
import { parseMaterialLibrary } from "@/lib/materials";
import { createMaterialDefinition, MATERIAL_PRESETS } from "@/lib/materials";
import { parsePlaygroundModel } from "@/lib/models";
import { exportProjectBundleZip, parseProjectBundleFile } from "@/lib/projects";
import {
  fixed,
  formatMilliseconds,
  formatTime,
  parseDirectMeshEndpoints,
  safeStorageGet,
  sanitizeWorkbenchSettings,
  scientific,
  serializeCurrentModel,
  serializeResultCsv,
  toAxialInput,
  WORKBENCH_SETTINGS_KEY,
} from "@/lib/workbench/helpers";
import {
  buildWorkbenchSnapshot,
  createAssistantTransactionEntry,
  pushHistoryEntry,
  restoreWorkbenchSnapshot,
  stepHistory,
  type AssistantTransactionEntry,
  type HistoryEntry,
  type WorkbenchSnapshot,
} from "@/lib/workbench/history";
import {
  addCustomMaterialToPlaneModel,
  addCustomMaterialToTruss3dModel,
  addCustomMaterialToTrussModel,
  addPresetMaterialToPlaneModel,
  addPresetMaterialToTruss3dModel,
  addPresetMaterialToTrussModel,
  applyMaterialToPlaneModel,
  applyMaterialToTruss3dModel,
  applyMaterialToTrussModel,
  deleteMaterialFromPlaneModel,
  deleteMaterialFromTruss3dModel,
  deleteMaterialFromTrussModel,
  ensurePlaneModelMaterials,
  ensureTruss3dModelMaterials,
  ensureTrussModelMaterials,
  mergeImportedMaterials,
  updateMaterialInPlaneModel,
  updateMaterialInTruss3dModel,
  updateMaterialInTrussModel,
} from "@/lib/workbench/material-commands";
import {
  clampChunkOffset,
  chunkCacheKey,
  computeResultWindowSize,
  computeVisibleResultWindowOffset,
  readChunkCache,
  RESULT_WINDOW_BASE_SIZE,
  RESULT_WINDOW_THRESHOLD,
  writeChunkCache,
} from "@/lib/workbench/result-window";
import {
  addTruss2dNode,
  assignTruss2dElementMaterial,
  deleteTruss2dElement,
  deleteTruss2dNode,
  toggleDraftSelection,
  toggleTruss2dMember,
  updateTruss2dElement,
  updateTruss2dNode,
} from "@/lib/workbench/truss2d-commands";
import {
  addTruss3dNodeCommand,
  applyTruss3dSelectedLoads,
  assignTruss3dElementMaterial,
  cloneTruss3dSelectedNodes,
  completeTruss3dLinkCommand,
  deleteTruss3dElementCommand,
  deleteTruss3dNodeCommand,
  merge3dBoxSelection,
  nudgeTruss3dSelectedNodes,
  updateTruss3dElement,
  updateTruss3dNodePositionCommand,
  updateTruss3dSelectedNodes,
} from "@/lib/workbench/truss3d-commands";
import {
  assignPlaneElementMaterial,
  updatePlaneElement,
  updatePlaneNode,
} from "@/lib/workbench/plane-commands";
import {
  exportProjectBundle,
  exportStudyModel,
  generatePrattTruss,
  generateRectangularPanelMesh,
  type ParametricPanelConfig,
  type ParametricTrussConfig,
} from "@/lib/models";
import { SAMPLE_LIBRARY } from "@/lib/models";
import { type WorkbenchScriptActionLogEntry, type WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";
import {
  createAxialBarJob,
  createDirectMeshSolve,
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
  fetchDatabaseExport,
  fetchDirectMeshAgents,
  fetchDirectMeshResultChunk,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchHealth,
  fetchJobHistory,
  fetchJobStatus,
  fetchProtocolAgents,
  fetchResultChunk,
  fetchProjects,
  fetchResults,
  type AxialBarJobInput,
  type AxialBarResult,
  type DirectMeshSelectionMode,
  type FrontendRuntimeMode,
  type HealthPayload,
  type JobEnvelope,
  type JobResultRecord,
  type JobState,
  type ModelRecord,
  type ModelVersionRecord,
  type PlaneTriangle2dJobInput,
  type PlaneTriangle2dResult,
  type ProtocolAgentDescriptor,
  type ProjectRecord,
  type ResultRecord,
  type ResultChunkPayload,
  resolvePlaneTriangle2dJobInput,
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

type Language = "en" | "zh";
type Theme = "linen" | "marine" | "graphite";
type SidebarSection = "study" | "model" | "library" | "system";
type StudyKind = "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";
type StudyPanelTab = "summary" | "controls";
type ModelPanelTab = "tools" | "tree";
type LibraryPanelTab = "samples" | "projects" | "models" | "jobs";
type ImmersiveToolTab = "node" | "props";
type SystemDataTab = "jobs" | "results";
type SystemPanelTab = "config" | "assistant" | "scripts" | "runtime" | "data";
type AssistantMode = "local" | "llm";

type AxialFormState = {
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

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
  material_id?: string;
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
  material_id?: string;
};

type SelectionKind = "node" | "element";

type TrussSuggestion =
  | { id: string; kind: "fix_support"; axis: "x" | "y"; nodeIndex: number; label: string }
  | { id: string; kind: "connect_nearest"; nodeIndex: number; label: string };

type TrussDiagnostics = {
  blockingMessages: string[];
  nodeIssues: Record<number, string[]>;
  suggestions: TrussSuggestion[];
};

type StabilitySummary = {
  score: number;
  tone: "good" | "watch" | "risk";
  hotspotNodes: number[];
};

type ResultWindowState = {
  jobId: string;
  studyKind: Exclude<StudyKind, "axial_bar_1d">;
  nodes: Record<string, unknown>[];
  elements: Record<string, unknown>[];
  totalNodes: number;
  totalElements: number;
  limit: number;
};

type DirectMeshExecutionState = {
  endpoint: string;
  strategy: DirectMeshSelectionMode;
  at: string;
};

const defaultAxial: AxialFormState = {
  length: 1.2,
  area: 0.01,
  elements: 6,
  tipForce: 1800,
  material: "210",
  youngsModulusGpa: 210,
};

const defaultTruss: Truss2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 0.5, y: 0.8, fix_x: false, fix_y: false, load_x: 0, load_y: -1000 },
  ],
  elements: [
    { id: "e0", node_i: 0, node_j: 2, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e2", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
  ],
};

const defaultParametric: ParametricTrussConfig = {
  bays: 4,
  span: 12,
  height: 3,
  area: 0.01,
  youngsModulusGpa: 70,
  loadY: -1200,
};

const defaultPanelParametric: ParametricPanelConfig = {
  width: 3.2,
  height: 1.8,
  divisionsX: 4,
  divisionsY: 3,
  thickness: 0.02,
  youngsModulusGpa: 70,
  poissonRatio: 0.33,
  loadY: -1200,
};

const defaultPlane: PlaneTriangle2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 1, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -800 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: false, load_x: 0, load_y: -800 },
  ],
  elements: [
    { id: "p0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, material_id: "mat-1" },
    { id: "p1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, material_id: "mat-1" },
  ],
};

const defaultTruss3d: Truss3dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "b0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "b1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "b2", x: 0.1, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "top", x: 0.35, y: 0.3, z: 1.0, fix_x: false, fix_y: false, fix_z: false, load_x: 0, load_y: 0, load_z: -1500 },
  ],
  elements: [
    { id: "e0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e2", node_i: 2, node_j: 0, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e3", node_i: 0, node_j: 3, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e4", node_i: 1, node_j: 3, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
    { id: "e5", node_i: 2, node_j: 3, area: 0.01, youngs_modulus: 70e9, material_id: "mat-1" },
  ],
};

const copy = {
  en: {
    brand: brand.productName,
    title: brand.applicationName.replace(/^Kyuubiki\s+/u, ""),
    subtitle: brand.workbenchDescription,
    rail: { study: "Study", model: "Model", library: "History", system: "System" },
    sections: { study: "Study Setup", model: "Model Studio", library: "Job History", system: "System" },
    kinds: { axial_bar_1d: "1D axial bar", truss_2d: "2D truss", truss_3d: "3D space truss", plane_triangle_2d: "2D plane triangle" },
    importModel: "Import model",
    importHint: "Load a JSON model for 1D or 2D studies.",
    axialSample: "Open 1D sample",
    trussSample: "Open 2D sample",
    modelName: "Model",
    material: "Material",
    mesh: "Mesh",
    load: "Load",
    support: "Support",
    viewport: "Viewport",
    report: "Report",
    metrics: "Solver Metrics",
    messages: "Messages",
    failureReason: "Failure reason",
    lastHeartbeat: "Last heartbeat",
    heartbeatStatus: "Heartbeat status",
    cancelJob: "Cancel job",
    historyPanel: "Operation History",
    undo: "Undo",
    redo: "Redo",
    noOperations: "No reversible operations yet.",
    undoApplied: "Rolled back the last change.",
    redoApplied: "Re-applied the last rolled-back change.",
    changeStudyType: "Changed study type",
    editAxialField: "Edited axial study input",
    editMaterial: "Changed material preset",
    editParametric: "Edited parametric generator",
    importAction: "Imported model file",
    sampleAction: "Loaded sample model",
    historyAction: "Opened historical job",
    generateAction: "Generated parametric truss",
    applySuggestionAction: "Applied diagnostic fix",
    addNodeAction: "Added node",
    deleteNodeAction: "Deleted node",
    toggleMemberAction: "Changed member connectivity",
    deleteMemberAction: "Deleted member",
    dragNodeAction: "Dragged node",
    editNodeAction: "Edited node properties",
    editMemberAction: "Edited member properties",
    overview: "Overview",
    controls: "Controls",
    settings: "Settings",
    scripts: "Scripts",
    assistant: "Assistant",
    assistantSummary: "Context",
    assistantStatusReady: "Ready",
    assistantCurrentStudy: "Study",
    assistantCurrentRuntime: "Runtime",
    assistantCurrentJob: "Job",
    assistantCurrentResult: "Result",
    assistantEmpty: "The workbench looks healthy. No assisted action is needed right now.",
    assistantNeedsProject: "Attach this workspace to a project",
    assistantNeedsProjectHint: "Projects make saved models, jobs, exports, and history easier to manage.",
    assistantOpenProjects: "Open projects",
    assistantRefreshRuntime: "Refresh runtime status",
    assistantRefreshRuntimeHint: "The runtime snapshot is missing or stale. Pull the latest health and agent state.",
    assistantConfigureDirectMesh: "Configure direct mesh endpoints",
    assistantConfigureDirectMeshHint: "Direct mesh mode needs at least one reachable host:port endpoint before it can route solves.",
    assistantRunStudy: "Run the current study",
    assistantRunStudyHint: "The model is ready enough to submit. Launch a fresh solve with the current inputs.",
    assistantCancelRun: "Cancel the active run",
    assistantCancelRunHint: "A solve is still in flight. Stop it if you want to edit the model or retry with different settings.",
    assistantApplyFix: "Apply the first suggested fix",
    assistantApplyFixHint: "The 2D truss precheck found a blocking issue and already prepared a safe first repair.",
    assistantEnterImmersive: "Open immersive 3D viewport",
    assistantEnterImmersiveHint: "Switch the 3D study into fullscreen editing when you want more room for picking and navigation.",
    backend: "Backend",
    protocols: "Protocols",
    controlPlaneProtocol: "Control plane",
    solverRpcProtocol: "Solver RPC",
    deploymentMode: "Deployment mode",
    discoveryMode: "Discovery",
    registeredAgents: "Registered agents",
    reachableAgents: "Reachable agents",
    runtimeMode: "Runtime mode",
    cluster: "Cluster",
    clusterSize: "Cluster size",
    clusterHealth: "Cluster health",
    peers: "Peers",
    peerState: "Peer state",
    headless: "Headless",
    capabilities: "Capabilities",
    methods: "Methods",
    protocolAgents: "Protocol agents",
    noProtocolAgents: "No reachable solver agents were described yet.",
    watchdog: "Watchdog",
    activeJobs: "Active jobs",
    stalledJobs: "Stalled jobs",
    timedOutJobs: "Timed out jobs",
    scanEvery: "Scan every",
    staleAfter: "Stall limit",
    timeoutAfter: "Timeout limit",
    dataAdmin: "Data Admin",
    orchestrator: "Elixir orchestrator",
    solverAgent: "Rust solver agent",
    ui: "Next.js UI",
    theme: "Theme",
    language: "Language",
    frontendMode: "Frontend mode",
    directMeshEndpoints: "Direct mesh endpoints",
    directMeshEndpointsHelp: "Comma or newline separated host:port agents for the LAN mesh GUI path.",
    directMeshCompleted: "Direct mesh solve completed",
    directMeshStrategy: "Mesh strategy",
    directMeshLastRoute: "Last route",
    directMeshLastAgent: "Last agent",
    shortcutHints: "Shortcut hints",
    shortcutHintsHelp: "Show the 3D keyboard legend in the viewport.",
    immersiveGuard: "Immersive guard",
    immersiveGuardHelp: "In immersive view, block text selection, context menu, and common copy shortcuts.",
    enterImmersive: "Immersive",
    exitImmersive: "Exit immersive",
    immersiveModeEnabled: "Immersive viewport enabled.",
    immersiveModeDisabled: "Immersive viewport closed.",
    browserLimitsNote: "Browser extensions cannot be fully disabled from the page, but the viewport can reduce selection and copy interactions.",
    adminJobs: "Jobs",
    adminResults: "Results",
    selectRecord: "Select a record to inspect or edit.",
    adminMessage: "Message",
    adminProjectId: "Project ID",
    adminModelVersionId: "Model version",
    adminCaseId: "Simulation case",
    saveRecord: "Save record",
    deleteRecord: "Delete record",
    exportRecord: "Export record",
    resultPayload: "Result JSON",
    resultSaved: "Result record updated.",
    resultDeleted: "Result record deleted.",
    jobSaved: "Job record updated.",
    jobDeleted: "Job record deleted.",
    invalidJson: "Invalid JSON payload.",
    databaseRecordCount: "Records",
    immersiveStudy: "Study",
    immersiveModel: "Model",
    immersiveLibrary: "Library",
    immersiveTools: "Tools",
    immersiveHelp: "3D Help",
    immersiveViewTools: "View tools",
    immersiveNodeOps: "Node ops",
    immersiveMemberOps: "Member ops",
    immersiveQuickProps: "Quick properties",
    immersiveNodeSelection: "Selected nodes",
    immersiveNoNodeSelection: "Select a 3D node to edit its coordinates, supports, and loads.",
    immersiveTransform: "Transform",
    immersiveLoads: "Batch loads",
    immersiveUtilities: "Utilities",
    frameSelection: "Frame selection",
    duplicateNodes: "Duplicate",
    mirrorX: "Mirror X",
    mirrorY: "Mirror Y",
    mirrorZ: "Mirror Z",
    applyLoads: "Apply loads",
    clearLoads: "Clear loads",
    nudgeStep: "Nudge step",
    immersiveDrawer: "Immersive drawer",
    close: "Close",
    immersiveSamples: "Samples",
    immersiveModels: "Saved",
    immersiveJobs: "Recent jobs",
    immersiveEmptyModels: "No saved models in this project yet.",
    immersiveEmptyJobs: "No jobs yet.",
    themes: { linen: "Linen Draft", marine: "Marine Grid", graphite: "Graphite Lab" },
    languages: { en: "English", zh: "中文" },
    frontendModes: {
      orchestrated_gui: "Orchestrated GUI",
      direct_mesh_gui: "Direct mesh GUI",
    },
    directMeshStrategies: {
      healthiest: "Healthiest agent",
      first_reachable: "First reachable",
    },
    shortcutLegendTitle: "3D controls",
    shortcutLegendRows: [
      "Drag pan · Alt+Drag orbit · Wheel zoom",
      "Shift+Wheel pans sideways",
      "1/2/3/4 views · P projection",
      "G grid · L labels · N nodes",
      "F focus · R reset · WASD/Arrows pan",
      "Link mode: click two nodes to toggle a member",
      "BOX selects visible nodes · Immersive button opens fullscreen",
    ],
    length: "Span (m)",
    area: "Area (m²)",
    modulus: "Young's modulus (GPa)",
    elements: "Elements",
    tipForce: "Tip force (N)",
    run: "Run Study",
    running: "Running...",
    ready: "ready",
    busy: "busy",
    tipDisp: "Tip displacement",
    maxStress: "Max stress",
    reaction: "Reaction",
    progress: "Progress",
    iteration: "Iteration",
    residual: "Residual",
    worker: "Worker",
    status: "Status",
    nodes: "Nodes",
    axialElements: "Element results",
    trussElements: "Truss members",
    spatialTrussElements: "Space-truss members",
    span: "Span",
    stress: "Stress (Pa)",
    axialForce: "Axial force (N)",
    importedModel: "Imported model",
    importFailed: "Import failed",
    initialLoaded: `${brand.productName} connected to the orchestrator.`,
    initialFailed: "Unable to reach the orchestrator.",
    dispatching: "Submitting a study to the orchestrator.",
    defaultModel: "manual-study",
    online: "online",
    offline: "offline",
    historyHint: "Durable jobs survive orchestrator restarts.",
    historyEmpty: "No jobs yet.",
    openJob: "Open",
    refresh: "Refresh",
    resultWindow: "Result Window",
    previousPage: "Previous",
    nextPage: "Next",
    jumpStart: "Start",
    jumpQuarter: "25%",
    jumpMid: "50%",
    jumpThreeQuarter: "75%",
    jumpEnd: "End",
    pageRange: "Rows",
    chunkSize: "Chunk",
    totalElements: "Elements",
    projectLibrary: "Project Library",
    projectNameField: "Project name",
    projectDescriptionField: "Description",
    createProject: "Create project",
    updateProject: "Rename project",
    deleteProject: "Delete project",
    exportProject: "Export project",
    exportProjectJson: "Project JSON",
    exportProjectZip: "Project ZIP",
    importProject: "Import project",
    importProjectHint: "Load a .kyuubiki.json or .kyuubiki archive into the project library.",
    projectEmpty: "No projects yet.",
    savedModels: "Saved Models",
    versions: "Versions",
    save: "Save",
    saveAs: "Save As",
    deleteSavedModel: "Delete model",
    renameVersion: "Rename version",
    deleteVersion: "Delete version",
    exportData: "Export Data",
    exportJson: "JSON",
    exportCsv: "CSV",
    noSavedModels: "No saved models in this project yet.",
    noVersions: "No saved versions yet.",
    defaultProject: "Workspace",
    projectCreated: "Project created.",
    projectUpdated: "Project updated.",
    projectDeleted: "Project deleted.",
    projectRequired: "Create or select a project before saving models.",
    projectExported: "Project bundle exported.",
    databaseExported: "Database snapshot exported.",
    projectImported: "Project bundle imported.",
    modelSaved: "Saved a new model version.",
    modelCreated: "Saved a new model.",
    modelDeletedStored: "Deleted the saved model.",
    versionLoaded: "Loaded a saved version into the workbench.",
    persistedModelLoaded: "Loaded a saved model from the library.",
    versionRenamed: "Version renamed.",
    versionDeleted: "Version deleted.",
    resultJsonDownloaded: "Analysis result JSON downloaded.",
    resultCsvDownloaded: "Analysis result CSV downloaded.",
    jobCancelled: "Job cancelled.",
    noResultToExport: "Run a study first before exporting analysis data.",
    modelTools: "Editing Tools",
    dragHint: "Drag truss nodes directly in the viewport to reshape geometry.",
    planeHint: "Select plane nodes and triangles to edit supports, loads, and material thickness.",
    parametric: "Parametric Generator",
    panelGenerator: "Panel Generator",
    spaceStudio: "3D Space Studio",
    tabs: { summary: "Summary", controls: "Controls", tools: "Tools", tree: "Tree", samples: "Samples", projects: "Projects", models: "Models", jobs: "Jobs" },
    generate: "Generate",
    generatePanel: "Generate Panel",
    download: "Download JSON",
    saveForSolver: "Use current model",
    bays: "Bays",
    height: "Height (m)",
    divisionsX: "Divisions X",
    divisionsY: "Divisions Y",
    nodeTable: "Nodes",
    dragNode: "Selected node",
    noNodeSelected: "No node selected",
    loadCase: "Load case",
    modelStudioHint: "2D truss and 2D plane studies can be edited directly here. 3D studies can already be imported, solved, and reviewed.",
    spaceStudioHint: "Use orbit, pan, and zoom to navigate the space frame. 3D nodes and members can be selected and edited numerically.",
    sourceModel: "Source model",
    createdAt: "Created",
    updatedAt: "Updated",
    hasResult: "Result",
    yes: "Yes",
    no: "No",
    dragToEdit: "Drag to edit",
    historyLoaded: "Loaded a persisted study from history.",
    modelDownloaded: "Model JSON downloaded.",
    generatedModel: "Generated a parametric truss model.",
    planeElements: "Plane elements",
    thickness: "Thickness",
    poisson: "Poisson ratio",
    poissonRatio: "Poisson ratio",
    sampleLibrary: "Sample Library",
    objectTree: "Object Tree",
    properties: "Properties",
    diagnostics: "Diagnostics",
    diagnosticsClear: "No blocking issues detected.",
    suggestedFixes: "Suggested fixes",
    stabilityScore: "Stability score",
    hotspotNodes: "Hotspot nodes",
    stabilityGood: "Stable",
    stabilityWatch: "Watch",
    stabilityRisk: "At risk",
    translatedSmallDeformation:
      "The structure is behaving too softly for a small-deformation solve. Add supports or reinforce the weak span.",
    translatedConnectivity:
      "The truss likely has a weak or disconnected region. Check supports, member links, and isolated nodes.",
    translatedSingular:
      "The stiffness matrix is singular. The model is still acting like a mechanism, so it needs more restraints or diagonal bracing.",
    translatedWatchdogStalled:
      "The job stopped reporting progress for too long. The watchdog marked it as stalled.",
    translatedWatchdogTimedOut:
      "The job ran too long and was stopped by the watchdog timeout.",
    translatedExecutionTimedOut:
      "The solver took too long to respond. The run was timed out and stopped safely.",
    translatedAgentTimeout:
      "The solver agent did not respond in time. Check the agent process or try the run again.",
    heartbeatHealthy: "healthy",
    heartbeatQuiet: "quiet",
    heartbeatStale: "stale",
    mechanismRisk: "This topology still looks mechanism-prone. Add more triangulation or extra supports before solving.",
    addNode: "Add Node",
    addBranchNode: "Branch Node",
    linkMode: "Link Mode",
    linkModeActive: "Linking",
    linkModeIdle: "Pick two nodes in the 3D viewport to link or unlink them.",
    linkModeEnabled: "3D link mode enabled.",
    linkModeDisabled: "3D link mode disabled.",
    linkModeCompleted: "3D link edit applied.",
    deleteNode: "Delete Node",
    addMember: "Add Member",
    toggleMember: "Toggle Link",
    deleteMember: "Delete Member",
    selectTwoNodes: "Select two nodes to create a member.",
    memberCreated: "Member created.",
    memberRemoved: "Member removed.",
    nodeCreated: "Node created.",
    branchCreated: "Node created and linked.",
    nodeDeleted: "Node deleted.",
    memberDeleted: "Member deleted.",
    selectionHint: "Use the object tree or viewport to pick nodes and members.",
    memberSelection: "Member selection",
    noElementSelected: "No member selected",
    nodeX: "Node X",
    nodeY: "Node Y",
    nodeZ: "Node Z",
    fixX: "Fix X",
    fixY: "Fix Y",
    fixZ: "Fix Z",
    loadX: "Load X",
    loadY: "Load Y",
    loadZ: "Load Z",
    nodeI: "Node I",
    nodeJ: "Node J",
    nodeK: "Node K",
    none: "None",
    precheckPrefix: "Precheck failed",
    unstableSupport: "Add at least two restrained degrees of freedom to stabilize the truss.",
    isolatedNode: "Every node should connect to at least one member before solving.",
    underconnected: "This truss is under-connected for a stable 2D solve.",
    freeRigidBody: "Prevent rigid-body motion by constraining both translation directions across the model.",
    supportXMissing: "Add at least one X restraint to prevent sideways drift.",
    supportYMissing: "Add at least one Y restraint to prevent vertical drift.",
    fixCurrentNodeXAction: "Fix current node in X",
    fixCurrentNodeYAction: "Fix current node in Y",
    fixNodeXAction: "Fix flagged node in X",
    fixNodeYAction: "Fix flagged node in Y",
    releaseX: "Release X",
    releaseY: "Release Y",
    releaseZ: "Release Z",
    connectCurrentNodeAction: "Connect current node to nearest node",
    connectNodeAction: "Connect flagged node to nearest node",
    suggestionAppliedSupportX: "Added an X restraint to the suggested node.",
    suggestionAppliedSupportY: "Added a Y restraint to the suggested node.",
    suggestionAppliedLink: "Connected the node to its nearest available neighbor.",
    suggestionNoLinkTarget: "No valid nearby node was available for an automatic link.",
    panelGenerated: "Generated a rectangular plane mesh.",
    planeThickness: "Thickness",
    orbitHint: "Drag to pan, Alt+drag to orbit, scroll to zoom.",
    spaceNodeCreated: "3D node created.",
    spaceBranchCreated: "3D node created and linked.",
    spaceNodeDeleted: "3D node deleted.",
    spaceMemberDeleted: "3D member deleted.",
    switchedTo2dStudio: "Switched to the 2D workbench.",
    switchedTo3dStudio: "Switched to the 3D space studio.",
  },
  zh: {
    brand: brand.productName,
    title: "结构分析工作台",
    subtitle: "更正式的前端工作台，统一建模、编排与求解回看。",
    rail: { study: "研究", model: "建模", library: "历史", system: "系统" },
    sections: { study: "研究设置", model: "建模工作室", library: "任务历史", system: "系统" },
    kinds: { axial_bar_1d: "一维轴向杆", truss_2d: "二维桁架", truss_3d: "三维空间桁架", plane_triangle_2d: "二维三角形单元" },
    importModel: "导入模型",
    importHint: "导入 1D 或 2D 研究 JSON 模型。",
    axialSample: "打开 1D 样例",
    trussSample: "打开 2D 样例",
    modelName: "模型",
    material: "材料",
    mesh: "网格",
    load: "载荷",
    support: "约束",
    viewport: "视图区",
    report: "报告",
    metrics: "求解指标",
    messages: "消息",
    failureReason: "失败原因",
    lastHeartbeat: "最近心跳",
    heartbeatStatus: "心跳状态",
    cancelJob: "取消任务",
    historyPanel: "操作历史",
    undo: "撤销",
    redo: "重做",
    noOperations: "当前还没有可回滚的操作。",
    undoApplied: "已回滚上一个改动。",
    redoApplied: "已恢复刚才回滚的改动。",
    changeStudyType: "切换研究类型",
    editAxialField: "修改轴向研究参数",
    editMaterial: "切换材料预设",
    editParametric: "修改参数化生成器",
    importAction: "导入模型文件",
    sampleAction: "加载样板模型",
    historyAction: "打开历史任务",
    generateAction: "生成参数化桁架",
    applySuggestionAction: "应用诊断修复",
    addNodeAction: "新增节点",
    deleteNodeAction: "删除节点",
    toggleMemberAction: "修改杆件连接",
    deleteMemberAction: "删除杆件",
    dragNodeAction: "拖拽节点",
    editNodeAction: "编辑节点属性",
    editMemberAction: "编辑杆件属性",
    overview: "概览",
    controls: "控制",
    settings: "设置",
    scripts: "脚本",
    assistant: "助手",
    assistantSummary: "上下文",
    assistantStatusReady: "就绪",
    assistantCurrentStudy: "当前研究",
    assistantCurrentRuntime: "运行模式",
    assistantCurrentJob: "当前任务",
    assistantCurrentResult: "当前结果",
    assistantEmpty: "当前工作台状态健康，暂时没有需要助手介入的动作。",
    assistantNeedsProject: "把当前工作区挂到项目里",
    assistantNeedsProjectHint: "挂到项目后，保存模型、任务历史和导出管理都会更顺手。",
    assistantOpenProjects: "打开项目",
    assistantRefreshRuntime: "刷新运行时状态",
    assistantRefreshRuntimeHint: "当前运行时快照缺失或偏旧，可以重新拉一次健康状态和 agent 信息。",
    assistantConfigureDirectMesh: "配置直连 Mesh 节点",
    assistantConfigureDirectMeshHint: "直连 Mesh 模式至少需要一个可达的 host:port 节点才能路由求解。",
    assistantRunStudy: "运行当前研究",
    assistantRunStudyHint: "当前模型已经基本可提交，可以直接发起一次新求解。",
    assistantCancelRun: "取消正在运行的任务",
    assistantCancelRunHint: "当前仍有任务在执行，如果你要改模型或重试参数，可以先停掉它。",
    assistantApplyFix: "应用第一条建议修复",
    assistantApplyFixHint: "二维桁架预检查发现了阻塞问题，而且已经准备好一个安全的首个修复动作。",
    assistantEnterImmersive: "打开沉浸式三维视图",
    assistantEnterImmersiveHint: "编辑三维模型时切到全屏，会更适合选点、导航和建模。",
    backend: "后端",
    protocols: "协议",
    controlPlaneProtocol: "调度面协议",
    solverRpcProtocol: "求解代理协议",
    deploymentMode: "部署模式",
    discoveryMode: "发现方式",
    registeredAgents: "注册代理",
    reachableAgents: "可达代理",
    runtimeMode: "运行模式",
    cluster: "集群",
    clusterSize: "集群规模",
    clusterHealth: "集群健康度",
    peers: "对等节点",
    peerState: "节点状态",
    headless: "无头运行",
    capabilities: "能力",
    methods: "方法",
    protocolAgents: "协议代理",
    noProtocolAgents: "当前还没有可描述的求解代理。",
    watchdog: "看门狗",
    activeJobs: "活跃任务",
    stalledJobs: "卡住任务",
    timedOutJobs: "超时任务",
    scanEvery: "扫描周期",
    staleAfter: "卡住阈值",
    timeoutAfter: "超时阈值",
    dataAdmin: "数据管理",
    orchestrator: "Elixir 编排器",
    solverAgent: "Rust 求解代理",
    ui: "Next.js 界面",
    theme: "主题",
    language: "语言",
    frontendMode: "前端模式",
    directMeshEndpoints: "直连集群节点",
    directMeshEndpointsHelp: "用逗号或换行分隔的 host:port，供局域网直连 mesh GUI 使用。",
    directMeshCompleted: "直连 Mesh 求解已完成",
    directMeshStrategy: "Mesh 策略",
    directMeshLastRoute: "最近路由",
    directMeshLastAgent: "最近节点",
    shortcutHints: "快捷键提示",
    shortcutHintsHelp: "在三维视图区显示键盘快捷键速查卡。",
    immersiveGuard: "沉浸护栏",
    immersiveGuardHelp: "沉浸模式下拦截文本选择、右键菜单和常见复制快捷键。",
    enterImmersive: "沉浸模式",
    exitImmersive: "退出沉浸",
    immersiveModeEnabled: "已进入沉浸式视图区。",
    immersiveModeDisabled: "已退出沉浸式视图区。",
    browserLimitsNote: "网页无法彻底关闭浏览器插件，但可以显著减少选择文本和复制类交互。",
    adminJobs: "任务",
    adminResults: "结果",
    selectRecord: "选择一条记录后即可查看或编辑。",
    adminMessage: "消息",
    adminProjectId: "项目 ID",
    adminModelVersionId: "模型版本",
    adminCaseId: "仿真工况",
    saveRecord: "保存记录",
    deleteRecord: "删除记录",
    exportRecord: "导出记录",
    resultPayload: "结果 JSON",
    resultSaved: "结果记录已更新。",
    resultDeleted: "结果记录已删除。",
    jobSaved: "任务记录已更新。",
    jobDeleted: "任务记录已删除。",
    invalidJson: "JSON 内容无效。",
    databaseRecordCount: "记录数",
    immersiveStudy: "研究",
    immersiveModel: "建模",
    immersiveLibrary: "库",
    immersiveTools: "工具",
    immersiveHelp: "3D 帮助",
    immersiveViewTools: "视图工具",
    immersiveNodeOps: "节点操作",
    immersiveMemberOps: "杆件操作",
    immersiveQuickProps: "快速属性",
    immersiveNodeSelection: "选中节点",
    immersiveNoNodeSelection: "先选中一个三维节点，再编辑它的坐标、约束和载荷。",
    immersiveTransform: "变换",
    immersiveLoads: "批量载荷",
    immersiveUtilities: "实用工具",
    frameSelection: "聚焦选中",
    duplicateNodes: "复制节点",
    mirrorX: "按 X 镜像",
    mirrorY: "按 Y 镜像",
    mirrorZ: "按 Z 镜像",
    applyLoads: "应用载荷",
    clearLoads: "清空载荷",
    nudgeStep: "步进",
    immersiveDrawer: "沉浸抽屉",
    close: "关闭",
    immersiveSamples: "样板",
    immersiveModels: "已保存",
    immersiveJobs: "最近任务",
    immersiveEmptyModels: "当前项目还没有保存模型。",
    immersiveEmptyJobs: "当前还没有任务记录。",
    themes: { linen: "纸面浅色", marine: "海图网格", graphite: "石墨实验室" },
    languages: { en: "English", zh: "中文" },
    frontendModes: {
      orchestrated_gui: "中心调度 GUI",
      direct_mesh_gui: "直连 Mesh GUI",
    },
    directMeshStrategies: {
      healthiest: "优先健康节点",
      first_reachable: "首个可达节点",
    },
    shortcutLegendTitle: "三维控制",
    shortcutLegendRows: [
      "拖拽平移 · Alt+拖拽旋转 · 滚轮缩放",
      "Shift+滚轮可左右平移",
      "1/2/3/4 视角 · P 投影",
      "G 网格 · L 标签 · N 节点",
      "F 聚焦 · R 重置 · WASD/方向键平移",
      "连线模式：依次点击两个节点即可切换杆件",
      "BOX 框选可见节点 · 沉浸按钮可切全屏",
    ],
    length: "跨度 (m)",
    area: "截面积 (m²)",
    modulus: "弹性模量 (GPa)",
    elements: "单元数",
    tipForce: "端部载荷 (N)",
    run: "运行研究",
    running: "运行中...",
    ready: "就绪",
    busy: "处理中",
    tipDisp: "端部位移",
    maxStress: "最大应力",
    reaction: "反力",
    progress: "进度",
    iteration: "迭代",
    residual: "残差",
    worker: "执行器",
    status: "状态",
    nodes: "节点",
    axialElements: "单元结果",
    trussElements: "桁架杆件",
    spatialTrussElements: "空间桁架杆件",
    span: "区间",
    stress: "应力 (Pa)",
    axialForce: "轴力 (N)",
    importedModel: "已导入模型",
    importFailed: "导入失败",
    initialLoaded: "工作台已连接到编排器。",
    initialFailed: "无法连接到编排器。",
    dispatching: "正在向编排器提交研究任务。",
    defaultModel: "手动研究",
    online: "在线",
    offline: "离线",
    historyHint: "任务历史会在编排器重启后保留。",
    historyEmpty: "暂时还没有任务。",
    openJob: "打开",
    refresh: "刷新",
    resultWindow: "结果窗口",
    previousPage: "上一页",
    nextPage: "下一页",
    jumpStart: "开头",
    jumpQuarter: "25%",
    jumpMid: "50%",
    jumpThreeQuarter: "75%",
    jumpEnd: "末尾",
    pageRange: "区间",
    chunkSize: "窗口",
    totalElements: "单元",
    projectLibrary: "项目库",
    projectNameField: "项目名",
    projectDescriptionField: "描述",
    createProject: "新建项目",
    updateProject: "更新项目",
    deleteProject: "删除项目",
    exportProject: "导出项目",
    exportProjectJson: "项目 JSON",
    exportProjectZip: "项目 ZIP",
    importProject: "导入项目",
    importProjectHint: "导入 .kyuubiki.json 或 .kyuubiki 项目包到项目库。",
    projectEmpty: "还没有项目。",
    savedModels: "已保存模型",
    versions: "版本",
    save: "保存",
    saveAs: "另存为",
    deleteSavedModel: "删除模型",
    renameVersion: "重命名版本",
    deleteVersion: "删除版本",
    exportData: "导出数据",
    exportJson: "JSON",
    exportCsv: "CSV",
    noSavedModels: "这个项目里还没有保存的模型。",
    noVersions: "还没有版本记录。",
    defaultProject: "工作区",
    projectCreated: "项目已创建。",
    projectUpdated: "项目已更新。",
    projectDeleted: "项目已删除。",
    projectRequired: "保存模型前请先创建或选择一个项目。",
    projectExported: "项目包已导出。",
    databaseExported: "数据库快照已导出。",
    projectImported: "项目包已导入。",
    modelSaved: "已保存新模型版本。",
    modelCreated: "已保存新模型。",
    modelDeletedStored: "已删除保存的模型。",
    versionLoaded: "已将保存版本加载到工作台。",
    persistedModelLoaded: "已从库中加载保存模型。",
    versionRenamed: "版本已重命名。",
    versionDeleted: "版本已删除。",
    resultJsonDownloaded: "分析结果 JSON 已下载。",
    resultCsvDownloaded: "分析结果 CSV 已下载。",
    jobCancelled: "任务已取消。",
    noResultToExport: "请先运行一次分析再导出结果数据。",
    modelTools: "编辑工具",
    dragHint: "直接在视图区拖拽桁架节点来修改几何。",
    planeHint: "选择平面节点和三角形单元，直接修改约束、载荷和厚度材料。",
    parametric: "参数化生成",
    panelGenerator: "面板生成器",
    spaceStudio: "三维空间工作室",
    tabs: { summary: "概览", controls: "控制", tools: "工具", tree: "对象", samples: "样板", projects: "项目", models: "模型", jobs: "任务" },
    generate: "生成模型",
    generatePanel: "生成面板",
    download: "下载 JSON",
    saveForSolver: "使用当前模型",
    bays: "跨数",
    height: "高度 (m)",
    divisionsX: "X 分段",
    divisionsY: "Y 分段",
    nodeTable: "节点列表",
    dragNode: "当前节点",
    noNodeSelected: "未选择节点",
    loadCase: "载荷工况",
    modelStudioHint: "当前建模页支持二维桁架和二维平面单元。三维研究已经支持导入、求解和结果回看。",
    spaceStudioHint: "通过旋转、平移和缩放来查看空间桁架。三维节点和杆件现在也可以选中并做数值编辑。",
    sourceModel: "来源模型",
    createdAt: "创建时间",
    updatedAt: "更新时间",
    hasResult: "结果",
    yes: "是",
    no: "否",
    dragToEdit: "拖拽编辑",
    historyLoaded: "已从历史记录加载持久化任务。",
    modelDownloaded: "模型 JSON 已下载。",
    generatedModel: "已生成参数化桁架模型。",
    planeElements: "平面单元",
    thickness: "厚度",
    poisson: "泊松比",
    poissonRatio: "泊松比",
    sampleLibrary: "样板库",
    objectTree: "对象树",
    properties: "属性",
    diagnostics: "诊断",
    diagnosticsClear: "当前没有阻塞性的预检查问题。",
    suggestedFixes: "建议修复",
    stabilityScore: "稳定性评分",
    hotspotNodes: "危险热区节点",
    stabilityGood: "稳定",
    stabilityWatch: "需关注",
    stabilityRisk: "高风险",
    translatedSmallDeformation: "这个结构看起来过软，已经超出小变形求解范围。请补约束或增强薄弱跨段。",
    translatedConnectivity: "桁架里可能有连接偏弱或断开的区域。请检查约束、杆件连接和孤立节点。",
    translatedSingular: "刚度矩阵是奇异的。说明模型仍然像机构一样可动，需要更多约束或对角支撑。",
    translatedWatchdogStalled: "任务长时间没有新的进度更新，已经被看门狗判定为卡住。",
    translatedWatchdogTimedOut: "任务运行时间过长，已经被看门狗超时终止。",
    translatedExecutionTimedOut: "求解器响应太久，任务已被安全超时终止。",
    translatedAgentTimeout: "求解代理在规定时间内没有响应。请检查 agent 进程，或稍后重试。",
    heartbeatHealthy: "正常",
    heartbeatQuiet: "安静",
    heartbeatStale: "过期",
    mechanismRisk: "这个拓扑仍然很像机构。建议先增加三角化斜撑或额外支座再求解。",
    addNode: "新增节点",
    addBranchNode: "分支节点",
    linkMode: "连线模式",
    linkModeActive: "连线中",
    linkModeIdle: "在三维视图区中依次选择两个节点，即可连接或断开。",
    linkModeEnabled: "已开启三维连线模式。",
    linkModeDisabled: "已关闭三维连线模式。",
    linkModeCompleted: "已应用三维连线修改。",
    deleteNode: "删除节点",
    addMember: "新增杆件",
    toggleMember: "切换连接",
    deleteMember: "删除杆件",
    selectTwoNodes: "请选择两个节点来创建杆件。",
    memberCreated: "杆件已创建。",
    memberRemoved: "杆件已断开。",
    nodeCreated: "节点已创建。",
    branchCreated: "节点已创建并连接。",
    nodeDeleted: "节点已删除。",
    memberDeleted: "杆件已删除。",
    selectionHint: "可以通过对象树或视图区选择节点和杆件。",
    memberSelection: "杆件选择",
    noElementSelected: "未选择杆件",
    nodeX: "节点 X",
    nodeY: "节点 Y",
    nodeZ: "节点 Z",
    fixX: "固定 X",
    fixY: "固定 Y",
    fixZ: "固定 Z",
    loadX: "载荷 X",
    loadY: "载荷 Y",
    loadZ: "载荷 Z",
    nodeI: "起点节点",
    nodeJ: "终点节点",
    nodeK: "第三节点",
    none: "无",
    precheckPrefix: "预检查失败",
    unstableSupport: "请至少提供两个受限自由度来稳定桁架。",
    isolatedNode: "每个节点在求解前都应该至少连接一根杆件。",
    underconnected: "这个桁架连接数量偏少，可能无法稳定求解。",
    freeRigidBody: "请通过约束两个平移方向来避免整体刚体运动。",
    supportXMissing: "至少增加一个 X 方向约束，避免侧向漂移。",
    supportYMissing: "至少增加一个 Y 方向约束，避免竖向漂移。",
    fixCurrentNodeXAction: "固定当前节点 X",
    fixCurrentNodeYAction: "固定当前节点 Y",
    fixNodeXAction: "固定问题节点 X",
    fixNodeYAction: "固定问题节点 Y",
    releaseX: "释放 X",
    releaseY: "释放 Y",
    releaseZ: "释放 Z",
    connectCurrentNodeAction: "将当前节点连接到最近节点",
    connectNodeAction: "将问题节点连接到最近节点",
    suggestionAppliedSupportX: "已为建议节点补上 X 约束。",
    suggestionAppliedSupportY: "已为建议节点补上 Y 约束。",
    suggestionAppliedLink: "已将该节点连接到最近的可用邻点。",
    suggestionNoLinkTarget: "附近没有可自动连接的有效节点。",
    panelGenerated: "已生成矩形平面网格。",
    planeThickness: "厚度",
    orbitHint: "拖拽平移，按住 Alt 拖拽旋转，滚轮缩放。",
    spaceNodeCreated: "三维节点已创建。",
    spaceBranchCreated: "三维节点已创建并连接。",
    spaceNodeDeleted: "三维节点已删除。",
    spaceMemberDeleted: "三维杆件已删除。",
    switchedTo2dStudio: "已切换到二维工作区。",
    switchedTo3dStudio: "已切换到三维空间工作区。",
  },
} as const;

function humanizeSolverFailure(message: string | null | undefined, languageCopy: (typeof copy)[Language]) {
  if (!message) return null;

  if (message.includes("watchdog marked job stalled")) {
    return languageCopy.translatedWatchdogStalled;
  }

  if (message.includes("watchdog timed out job")) {
    return languageCopy.translatedWatchdogTimedOut;
  }

  if (message.includes("job execution timed out")) {
    return languageCopy.translatedExecutionTimedOut;
  }

  if (message.includes("small-deformation limit")) {
    return `${languageCopy.translatedSmallDeformation} ${languageCopy.translatedConnectivity}`;
  }

  if (message.includes("system is singular")) {
    return `${languageCopy.translatedSingular} ${languageCopy.translatedConnectivity}`;
  }

  if (message.includes("supports or connectivity")) {
    return languageCopy.translatedConnectivity;
  }

  if (message.includes(":timeout") || message.includes("all_agents_failed")) {
    return languageCopy.translatedAgentTimeout;
  }

  return message;
}

function formatJobMessage(job: JobEnvelope["job"] | null, fallback: string, languageCopy: (typeof copy)[Language]) {
  if (!job) return fallback;
  if (job.status === "failed" && job.message) {
    return `${job.job_id} failed: ${humanizeSolverFailure(job.message, languageCopy) ?? job.message}`;
  }
  return `${job.job_id} ${job.status}`;
}

function summarizeTrussStability(model: Truss2dJobInput, diagnostics: TrussDiagnostics): StabilitySummary {
  const nodeIssueEntries = Object.entries(diagnostics.nodeIssues);
  const issueCount = diagnostics.blockingMessages.length + nodeIssueEntries.reduce((sum, [, issues]) => sum + issues.length, 0);
  const supportCount = model.nodes.reduce((sum, node) => sum + (node.fix_x ? 1 : 0) + (node.fix_y ? 1 : 0), 0);
  const structuralScore = Math.max(0, 100 - issueCount * 14 - Math.max(0, model.nodes.length - model.elements.length - 1) * 8);
  const supportBoost = Math.min(12, supportCount * 3);
  const score = Math.max(0, Math.min(100, structuralScore + supportBoost));
  const hotspotNodes = nodeIssueEntries
    .sort((left, right) => right[1].length - left[1].length)
    .slice(0, 3)
    .map(([nodeIndex]) => Number(nodeIndex));

  if (score >= 80) return { score, tone: "good", hotspotNodes };
  if (score >= 55) return { score, tone: "watch", hotspotNodes };
  return { score, tone: "risk", hotspotNodes };
}

function isTruss3dResult(value: unknown): value is Truss3dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && Array.isArray((value as Truss3dResult).nodes) && (value as Truss3dResult).nodes.some((node) => "z" in node);
}

function buildDisplayTruss3dNodes(
  model: Truss3dJobInput,
  result: Truss3dResult | null,
  windowNodes?: Truss3dResult["nodes"],
): DisplayTruss3dNode[] {
  const nodes = windowNodes ?? result?.nodes;

  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      z: node.z,
      ux: node.ux,
      uy: node.uy,
      uz: node.uz,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    z: node.z,
    ux: 0,
    uy: 0,
    uz: 0,
  }));
}

function buildDisplayTruss3dElements(
  model: Truss3dJobInput,
  result: Truss3dResult | null,
  windowElements?: Truss3dResult["elements"],
): DisplayTruss3dElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      ...element,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);
    const dz = (nodeJ?.z ?? 0) - (nodeI?.z ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy + dz * dz),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}

function projectTruss3dPoint(node: { x: number; y: number; z: number }, bounds: ReturnType<typeof getTrussBounds>) {
  const isoX = node.x - node.y * 0.55;
  const isoY = node.z + node.y * 0.35;
  return toSvgPoint({ x: isoX, y: isoY }, bounds);
}

function localMaterialLabel(value: string, language: Language): string {
  const labels = {
    en: {
      "210": "Steel",
      "70": "Aluminum",
      "116": "Titanium",
      "30": "Concrete",
      "135": "Carbon fiber",
      custom: "Custom",
    },
    zh: {
      "210": "钢",
      "70": "铝",
      "116": "钛",
      "30": "混凝土",
      "135": "碳纤维",
      custom: "自定义",
    },
  } as const;

  return labels[language][value as keyof (typeof labels)[Language]] ?? labels[language].custom;
}

const MATERIAL_COLOR_STOPS = [
  "#1677a3",
  "#ff8a3d",
  "#4a9c61",
  "#915fe2",
  "#c7547a",
  "#8a6f3b",
];

function materialColorByIndex(index: number) {
  return MATERIAL_COLOR_STOPS[index % MATERIAL_COLOR_STOPS.length];
}

function isAxialResult(value: unknown): value is AxialBarResult {
  return typeof value === "object" && value !== null && "tip_displacement" in value;
}

function isTrussResult(value: unknown): value is Truss2dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && !("tip_displacement" in value) && Array.isArray((value as Truss2dResult).nodes) && !(value as Truss2dResult).nodes.some((node) => "z" in node);
}

function isPlaneTriangleResult(value: unknown): value is PlaneTriangle2dResult {
  return typeof value === "object" && value !== null && "elements" in value && "nodes" in value && "input" in value && Array.isArray((value as PlaneTriangle2dResult).elements) && (value as PlaneTriangle2dResult).elements.some((element) => "node_k" in element);
}

function buildDisplayTrussNodes(
  model: Truss2dJobInput,
  result: Truss2dResult | null,
  windowNodes?: Truss2dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;

  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      ux: node.ux,
      uy: node.uy,
      fix_x: model.nodes[node.index]?.fix_x ?? false,
      fix_y: model.nodes[node.index]?.fix_y ?? false,
      load_x: model.nodes[node.index]?.load_x ?? 0,
      load_y: model.nodes[node.index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    ux: 0,
    uy: 0,
    fix_x: node.fix_x,
    fix_y: node.fix_y,
    load_x: node.load_x,
    load_y: node.load_y,
  }));
}

function buildDisplayTrussElements(
  model: Truss2dJobInput,
  result: Truss2dResult | null,
  windowElements?: Truss2dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.strain,
      stress: element.stress,
      axial_force: element.axial_force,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}

function getTrussBounds(nodes: Array<{ x: number; y: number }>) {
  const xs = nodes.map((node) => node.x);
  const ys = nodes.map((node) => node.y);
  const minX = Math.min(...xs, 0);
  const maxX = Math.max(...xs, 1);
  const minY = Math.min(...ys, 0);
  const maxY = Math.max(...ys, 1);
  return {
    minX,
    maxX,
    minY,
    maxY,
    width: Math.max(maxX - minX, 1),
    height: Math.max(maxY - minY, 1),
  };
}

function toSvgPoint(node: { x: number; y: number }, bounds: ReturnType<typeof getTrussBounds>) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;

  return {
    x: paddingX + ((node.x - bounds.minX) / bounds.width) * usableWidth,
    y: 460 - paddingY - ((node.y - bounds.minY) / bounds.height) * usableHeight,
  };
}

function fromSvgPoint(clientX: number, clientY: number, rect: DOMRect, bounds: ReturnType<typeof getTrussBounds>) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;
  const x = ((clientX - rect.left) / rect.width) * 980;
  const y = ((clientY - rect.top) / rect.height) * 460;

  const normalizedX = Math.min(Math.max((x - paddingX) / usableWidth, 0), 1);
  const normalizedY = Math.min(Math.max((460 - paddingY - y) / usableHeight, 0), 1);

  return {
    x: round(bounds.minX + normalizedX * bounds.width),
    y: round(bounds.minY + normalizedY * bounds.height),
  };
}

function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}

function downloadTextFile(filename: string, contents: string) {
  const blob = new Blob([contents], { type: "application/json;charset=utf-8" });
  downloadBlobFile(filename, blob);
}

function downloadBlobFile(filename: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

function resetActiveResult(
  setResult: Dispatch<SetStateAction<AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | null>>,
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>,
) {
  setResult(null);
  setJob(null);
}

function pushNodeIssue(nodeIssues: Record<number, string[]>, nodeIndex: number, issue: string) {
  const issues = nodeIssues[nodeIndex] ?? [];
  if (!issues.includes(issue)) {
    nodeIssues[nodeIndex] = [...issues, issue];
  }
}

function heartbeatStatus(
  job: JobEnvelope["job"] | null,
  languageCopy: (typeof copy)[Language],
) {
  if (!job?.updated_at) return "--";

  const updatedAt = new Date(job.updated_at);
  if (Number.isNaN(updatedAt.getTime())) return "--";

  const active =
    job.status === "queued" ||
    job.status === "preprocessing" ||
    job.status === "partitioning" ||
    job.status === "solving" ||
    job.status === "postprocessing";

  if (!active) return languageCopy.heartbeatHealthy;

  const ageMs = Math.max(0, Date.now() - updatedAt.getTime());

  if (ageMs < 6_000) return languageCopy.heartbeatHealthy;
  if (ageMs < 20_000) return `${languageCopy.heartbeatQuiet} ${Math.round(ageMs / 1000)}s`;
  return `${languageCopy.heartbeatStale} ${Math.round(ageMs / 1000)}s`;
}

function heartbeatTone(job: JobEnvelope["job"] | null): "healthy" | "quiet" | "stale" {
  if (!job?.updated_at) return "quiet";

  const updatedAt = new Date(job.updated_at);
  if (Number.isNaN(updatedAt.getTime())) return "quiet";

  const active =
    job.status === "queued" ||
    job.status === "preprocessing" ||
    job.status === "partitioning" ||
    job.status === "solving" ||
    job.status === "postprocessing";

  if (!active) return "healthy";

  const ageMs = Math.max(0, Date.now() - updatedAt.getTime());
  if (ageMs < 6_000) return "healthy";
  if (ageMs < 20_000) return "quiet";
  return "stale";
}

function findNearestConnectableNode(model: Truss2dJobInput, nodeIndex: number): number | null {
  const origin = model.nodes[nodeIndex];
  if (!origin) return null;

  let bestIndex: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;

  for (const [candidateIndex, candidate] of model.nodes.entries()) {
    if (candidateIndex === nodeIndex) continue;

    const alreadyLinked = model.elements.some(
      (element) =>
        (element.node_i === nodeIndex && element.node_j === candidateIndex) ||
        (element.node_i === candidateIndex && element.node_j === nodeIndex),
    );

    if (alreadyLinked) continue;

    const distance = Math.hypot(candidate.x - origin.x, candidate.y - origin.y);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = candidateIndex;
    }
  }

  return bestIndex;
}

function analyzeTrussModel(
  model: Truss2dJobInput,
  languageCopy: (typeof copy)[Language],
  selectedNode: number | null,
): TrussDiagnostics {
  const nodeCount = model.nodes.length;
  const elementCount = model.elements.length;
  const fixedXCount = model.nodes.filter((node) => node.fix_x).length;
  const fixedYCount = model.nodes.filter((node) => node.fix_y).length;
  const constrainedDofs = model.nodes.reduce(
    (count, node) => count + (node.fix_x ? 1 : 0) + (node.fix_y ? 1 : 0),
    0,
  );
  const blockingMessages: string[] = [];
  const nodeIssues: Record<number, string[]> = {};
  const suggestions: TrussSuggestion[] = [];
  const suggestionIds = new Set<string>();
  const connectionCounts = new Array(nodeCount).fill(0);
  const supportTarget = selectedNode ?? 0;

  const addSuggestion = (suggestion: TrussSuggestion) => {
    if (!suggestionIds.has(suggestion.id)) {
      suggestionIds.add(suggestion.id);
      suggestions.push(suggestion);
    }
  };

  if (constrainedDofs < 2) {
    blockingMessages.push(languageCopy.unstableSupport);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.unstableSupport);
    }
  }

  if (fixedXCount === 0) {
    blockingMessages.push(languageCopy.supportXMissing);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.supportXMissing);
      addSuggestion({
        id: `fix-x-${supportTarget}`,
        kind: "fix_support",
        axis: "x",
        nodeIndex: supportTarget,
        label: selectedNode !== null ? languageCopy.fixCurrentNodeXAction : languageCopy.fixNodeXAction,
      });
    }
  }

  if (fixedYCount === 0) {
    blockingMessages.push(languageCopy.supportYMissing);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.supportYMissing);
      addSuggestion({
        id: `fix-y-${supportTarget}`,
        kind: "fix_support",
        axis: "y",
        nodeIndex: supportTarget,
        label: selectedNode !== null ? languageCopy.fixCurrentNodeYAction : languageCopy.fixNodeYAction,
      });
    }
  }

  if (fixedXCount === 0 || fixedYCount === 0) {
    blockingMessages.push(languageCopy.freeRigidBody);
  }

  if (elementCount < nodeCount - 1) {
    blockingMessages.push(languageCopy.underconnected);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, selectedNode ?? 0, languageCopy.underconnected);
    }
  }

  if (elementCount + constrainedDofs < nodeCount * 2) {
    blockingMessages.push(languageCopy.mechanismRisk);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, selectedNode ?? 0, languageCopy.mechanismRisk);
    }
  }

  for (const element of model.elements) {
    if (element.node_i < nodeCount) connectionCounts[element.node_i] += 1;
    if (element.node_j < nodeCount) connectionCounts[element.node_j] += 1;
  }

  const isolatedNodes = connectionCounts.flatMap((count, index) => (count === 0 ? [index] : []));
  for (const nodeIndex of isolatedNodes) {
    pushNodeIssue(nodeIssues, nodeIndex, languageCopy.isolatedNode);
  }

  if (isolatedNodes.length > 0) {
    blockingMessages.push(languageCopy.isolatedNode);
  }

  const connectTarget =
    (selectedNode !== null && (isolatedNodes.includes(selectedNode) || connectionCounts[selectedNode] < 2)
      ? selectedNode
      : isolatedNodes[0] ?? selectedNode) ?? null;

  if (connectTarget !== null && nodeCount > 1) {
    addSuggestion({
      id: `connect-${connectTarget}`,
      kind: "connect_nearest",
      nodeIndex: connectTarget,
      label: selectedNode !== null ? languageCopy.connectCurrentNodeAction : languageCopy.connectNodeAction,
    });
  }

  return {
    blockingMessages: [...new Set(blockingMessages)],
    nodeIssues,
    suggestions,
  };
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

function formatProtocolMethodLabel(method: string) {
  return method.replaceAll("_", " ");
}

function clusterHealthTone(score: number | null | undefined) {
  if (score == null) return "quiet";
  if (score >= 85) return "healthy";
  if (score >= 55) return "watch";
  return "stale";
}

function formatPeerStatus(status: string | undefined, languageCopy: (typeof copy)[Language]) {
  if (!status) return "--";
  switch (status) {
    case "healthy":
      return languageCopy.heartbeatHealthy;
    case "degraded":
      return languageCopy.heartbeatQuiet;
    case "unreachable":
      return languageCopy.heartbeatStale;
    case "seed":
      return languageCopy.ready;
    default:
      return status.replaceAll("_", " ");
  }
}

export function Workbench() {
  const [studyKind, setStudyKind] = useState<StudyKind>("axial_bar_1d");
  const [axialForm, setAxialForm] = useState<AxialFormState>(defaultAxial);
  const [trussModel, setTrussModel] = useState<Truss2dJobInput>(defaultTruss);
  const [truss3dModel, setTruss3dModel] = useState<Truss3dJobInput>(defaultTruss3d);
  const [planeModel, setPlaneModel] = useState<PlaneTriangle2dJobInput>(defaultPlane);
  const [parametric, setParametric] = useState<ParametricTrussConfig>(defaultParametric);
  const [panelParametric, setPanelParametric] = useState<ParametricPanelConfig>(defaultPanelParametric);
  const [activeMaterial, setActiveMaterial] = useState("210");
  const [result, setResult] = useState<AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | null>(null);
  const [resultWindow, setResultWindow] = useState<ResultWindowState | null>(null);
  const [resultWindowOffset, setResultWindowOffset] = useState(0);
  const [resultWindowLimit, setResultWindowLimit] = useState(RESULT_WINDOW_BASE_SIZE);
  const [canvasViewportWidth, setCanvasViewportWidth] = useState(980);
  const [job, setJob] = useState<JobEnvelope["job"] | null>(null);
  const [jobHistory, setJobHistory] = useState<JobState[]>([]);
  const [resultRecords, setResultRecords] = useState<ResultRecord[]>([]);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [selectedVersionId, setSelectedVersionId] = useState<string | null>(null);
  const [modelVersions, setModelVersions] = useState<ModelVersionRecord[]>([]);
  const [projectNameDraft, setProjectNameDraft] = useState<string>(copy.en.defaultProject);
  const [projectDescriptionDraft, setProjectDescriptionDraft] = useState<string>("");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [protocolAgents, setProtocolAgents] = useState<ProtocolAgentDescriptor[]>([]);
  const [loadedModelName, setLoadedModelName] = useState<string>(copy.en.defaultModel);
  const [message, setMessage] = useState<string>(copy.en.initialLoaded);
  const [language, setLanguage] = useState<Language>("en");
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
    truss_2d: [],
    truss_3d: [],
    plane_triangle_2d: [],
  });
  const [sidebarSection, setSidebarSection] = useState<SidebarSection>("study");
  const [studyTab, setStudyTab] = useState<StudyPanelTab>("summary");
  const [modelTab, setModelTab] = useState<ModelPanelTab>("tools");
  const [libraryTab, setLibraryTab] = useState<LibraryPanelTab>("samples");
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
  const [selectedAdminJobId, setSelectedAdminJobId] = useState<string | null>(null);
  const [selectedAdminResultJobId, setSelectedAdminResultJobId] = useState<string | null>(null);
  const [scriptActionLog, setScriptActionLog] = useState<WorkbenchScriptActionLogEntry[]>([]);
  const [assistantTransactions, setAssistantTransactions] = useState<AssistantTransactionEntry[]>([]);
  const [adminJobMessage, setAdminJobMessage] = useState("");
  const [adminJobProjectId, setAdminJobProjectId] = useState("");
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
  const chunkScrollFrameRef = useRef<number | null>(null);
  const chunkScrollLeftRef = useRef(0);
  const chunkScrollDirectionRef = useRef<-1 | 0 | 1>(0);
  const chunkCacheRef = useRef<Map<string, ResultChunkPayload<Record<string, unknown>>>>(new Map());
  const t = copy[language];

  useEffect(() => {
    setResultWindowOffset(0);
    setResultWindowLimit(RESULT_WINDOW_BASE_SIZE);
    chunkCacheRef.current.clear();
  }, [job?.job_id, studyKind]);

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
    return () => {
      if (chunkScrollFrameRef.current !== null) {
        window.cancelAnimationFrame(chunkScrollFrameRef.current);
      }
    };
  }, []);

  useEffect(() => {
    const syncCanvasViewportWidth = () => {
      const nextWidth = canvasStageRef.current?.clientWidth ?? 980;
      setCanvasViewportWidth((current) => (current === nextWidth ? current : nextWidth));
    };

    syncCanvasViewportWidth();
    window.addEventListener("resize", syncCanvasViewportWidth);

    return () => {
      window.removeEventListener("resize", syncCanvasViewportWidth);
    };
  }, []);

  useEffect(() => {
    if (!job?.job_id || !result || isAxialResult(result)) {
      setResultWindow(null);
      return;
    }

    const totalNodes = Array.isArray(result.nodes) ? result.nodes.length : 0;
    const totalElements = Array.isArray(result.elements) ? result.elements.length : 0;
    const totalItems = Math.max(totalNodes, totalElements);

    if (totalNodes <= RESULT_WINDOW_THRESHOLD && totalElements <= RESULT_WINDOW_THRESHOLD) {
      setResultWindow(null);
      return;
    }

    const limit = computeResultWindowSize(totalItems, canvasViewportWidth);
    if (resultWindowLimit !== limit) {
      setResultWindowLimit(limit);
    }

    const nextStudyKind: Exclude<StudyKind, "axial_bar_1d"> = isTrussResult(result)
      ? "truss_2d"
      : isTruss3dResult(result)
        ? "truss_3d"
        : "plane_triangle_2d";

    let cancelled = false;

    (async () => {
      try {
        const safeOffset = clampChunkOffset(resultWindowOffset, totalItems, limit);
        const chunkFetcher =
          frontendRuntimeMode === "direct_mesh_gui" ? fetchDirectMeshResultChunk : fetchResultChunk;
        const fetchChunk = async (kind: "nodes" | "elements", offset: number) => {
          const key = chunkCacheKey(frontendRuntimeMode, job.job_id, kind, offset, limit);
          const cached = readChunkCache(chunkCacheRef.current, key);
          if (cached) return cached;

          const chunk = await chunkFetcher(job.job_id, kind, { offset, limit });
          writeChunkCache(chunkCacheRef.current, key, chunk as ResultChunkPayload<Record<string, unknown>>);
          return chunk;
        };

        const nodesKey = chunkCacheKey(frontendRuntimeMode, job.job_id, "nodes", safeOffset, limit);
        const elementsKey = chunkCacheKey(frontendRuntimeMode, job.job_id, "elements", safeOffset, limit);
        const cachedNodes = readChunkCache(chunkCacheRef.current, nodesKey);
        const cachedElements = readChunkCache(chunkCacheRef.current, elementsKey);

        if (cachedNodes && cachedElements) {
          setResultWindow({
            jobId: job.job_id,
            studyKind: nextStudyKind,
            nodes: cachedNodes.items,
            elements: cachedElements.items,
            totalNodes: cachedNodes.total,
            totalElements: cachedElements.total,
            limit,
          });
        }

        const [nodesChunk, elementsChunk] = await Promise.all([
          fetchChunk("nodes", safeOffset),
          fetchChunk("elements", safeOffset),
        ]);

        if (cancelled) return;

        setResultWindow({
          jobId: job.job_id,
          studyKind: nextStudyKind,
          nodes: nodesChunk.items,
          elements: elementsChunk.items,
          totalNodes: nodesChunk.total,
          totalElements: elementsChunk.total,
          limit,
        });

        const directionalOffsets =
          chunkScrollDirectionRef.current > 0
            ? [safeOffset + limit, safeOffset + limit * 2, safeOffset - limit]
            : chunkScrollDirectionRef.current < 0
              ? [safeOffset - limit, safeOffset - limit * 2, safeOffset + limit]
              : [safeOffset - limit, safeOffset + limit];

        const prefetchOffsets = directionalOffsets
          .map((offset) => clampChunkOffset(offset, totalItems, limit))
          .filter((offset, index, values) => offset !== safeOffset && values.indexOf(offset) === index);

        void Promise.all(
          prefetchOffsets.flatMap((offset) => [
            fetchChunk("nodes", offset),
            fetchChunk("elements", offset),
          ]),
        ).catch(() => undefined);
      } catch {
        if (!cancelled) {
          setResultWindow(null);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [canvasViewportWidth, frontendRuntimeMode, job?.job_id, result, resultWindowLimit, resultWindowOffset]);

  useEffect(() => {
    if (!resultWindow) return;

    const totalItems = Math.max(resultWindow.totalNodes, resultWindow.totalElements);
    const nextOffset = clampChunkOffset(resultWindowOffset, totalItems, resultWindowLimit);

    if (nextOffset !== resultWindowOffset) {
      setResultWindowOffset(nextOffset);
    }
  }, [resultWindow, resultWindowLimit, resultWindowOffset]);

  useEffect(() => {
    const stored = safeStorageGet();
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
    if (stored.language === "en" || stored.language === "zh") {
      setLanguage(stored.language);
      setLoadedModelName(copy[stored.language].defaultModel);
      setMessage(copy[stored.language].initialLoaded);
    }
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    if (typeof window !== "undefined") {
      window.localStorage.setItem(
        WORKBENCH_SETTINGS_KEY,
        JSON.stringify(sanitizeWorkbenchSettings({
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
        })),
      );
    }
  }, [theme, language, showShortcutHints, immersiveGuardrails, frontendRuntimeMode, directMeshEndpointsText, directMeshSelectionMode, controlPlaneApiToken, clusterApiToken, directMeshApiToken, assistantMode, assistantApiBaseUrl, assistantApiKey, assistantModel]);

  useEffect(() => {
    return () => {
      if (dragFrameRef.current !== null) {
        window.cancelAnimationFrame(dragFrameRef.current);
      }
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
    void refreshHealth();
    void refreshJobHistory();
    void refreshResults();
    void refreshProjects(true);
  }, []);

  useEffect(() => {
    void refreshHealth();
  }, [frontendRuntimeMode, directMeshEndpointsText, directMeshSelectionMode]);

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

  useEffect(() => {
    if (!selectedModelId) {
      setModelVersions([]);
      return;
    }

    void refreshVersions(selectedModelId);
  }, [selectedModelId]);

  async function refreshHealth() {
    if (frontendRuntimeMode === "direct_mesh_gui") {
      try {
        const endpoints = parseDirectMeshEndpoints(directMeshEndpointsText);
        const nextDirect = await fetchDirectMeshAgents(endpoints);
        const directMethods = [...new Set(
          nextDirect.agents.flatMap((agent) => agent.descriptor?.protocol?.methods ?? []),
        )];

        setProtocolAgents(nextDirect.agents);
        setHealth({
          service: "kyuubiki-frontend-direct-mesh",
          status: nextDirect.agents.length > 0 ? "ok" : "degraded",
          protocol: {
            program: "kyuubiki-frontend",
            role: "gui",
            protocol: {
              name: "kyuubiki.direct-mesh/http-v1",
              version: 1,
              transport: { kind: "http", encoding: "json" },
            },
            compatible_solver_rpc: {
              name: "kyuubiki.solver-rpc/v1",
              rpc_version: 1,
              transport: {
                kind: "tcp",
                framing: "length_prefixed_u32",
                encoding: "json",
              },
              methods: directMethods,
            },
          },
          deployment: {
            mode: "direct_mesh",
            discovery: nextDirect.discovery,
            endpoint_count: nextDirect.endpoint_count,
          },
          remote_solver_registry: {
            active_agents: nextDirect.agents.length,
          },
        });
      } catch {
        setHealth(null);
        setProtocolAgents([]);
      }
      return;
    }

    try {
      const [nextHealth, nextProtocolAgents] = await Promise.all([
        fetchHealth(),
        fetchProtocolAgents().catch(() => ({ agents: [] })),
      ]);

      setHealth(nextHealth);
      setProtocolAgents(nextProtocolAgents.agents);
    } catch {
      setHealth(null);
      setProtocolAgents([]);
    }
  }

  async function refreshJobHistory() {
    try {
      const payload = await fetchJobHistory();
      setJobHistory(payload.jobs);
      setSelectedAdminJobId((current) =>
        current && payload.jobs.some((entry) => entry.job_id === current) ? current : payload.jobs[0]?.job_id ?? null,
      );
    } catch {
      setJobHistory([]);
      setSelectedAdminJobId(null);
    }
  }

  async function refreshResults() {
    try {
      const payload = await fetchResults();
      setResultRecords(payload.results);
      setSelectedAdminResultJobId((current) =>
        current && payload.results.some((entry) => entry.job_id === current) ? current : payload.results[0]?.job_id ?? null,
      );
    } catch {
      setResultRecords([]);
      setSelectedAdminResultJobId(null);
    }
  }

  async function refreshProjects(bootstrap = false) {
    try {
      const payload = await fetchProjects();
      let nextProjects = payload.projects;

      if (bootstrap && nextProjects.length === 0) {
        const created = await createProject({ name: copy.en.defaultProject, description: "Local workspace" });
        nextProjects = [created.project];
      }

      setProjects(nextProjects);

      const nextProjectId =
        selectedProjectId && nextProjects.some((project) => project.project_id === selectedProjectId)
          ? selectedProjectId
          : nextProjects[0]?.project_id ?? null;

      setSelectedProjectId(nextProjectId);

      const nextModelId =
        selectedModelId &&
        nextProjects.some((project) => (project.models ?? []).some((model) => model.model_id === selectedModelId))
          ? selectedModelId
          : (nextProjects.find((project) => project.project_id === nextProjectId)?.models ?? [])[0]?.model_id ?? null;

      setSelectedModelId(nextModelId);
      if (!nextModelId) {
        setSelectedVersionId(null);
      }
    } catch {
      setProjects([]);
    }
  }

  async function refreshVersions(modelId: string) {
    try {
      const payload = await fetchModelVersions(modelId);
      setModelVersions(payload.versions);
    } catch {
      setModelVersions([]);
    }
  }

  const runAnalysis = () => {
    if (studyKind === "truss_2d") {
      const precheckErrors = trussDiagnostics?.blockingMessages ?? [];
      if (precheckErrors.length > 0) {
        setMessage(`${t.precheckPrefix}: ${precheckErrors[0]}`);
        resetActiveResult(setResult, setJob);
        return;
      }
    }

    setMessage(t.dispatching);
    setResult(null);

    startTransition(async () => {
      try {
        if (frontendRuntimeMode === "direct_mesh_gui") {
          const endpoints = parseDirectMeshEndpoints(directMeshEndpointsText);
          if (endpoints.length === 0) {
            throw new Error(t.directMeshEndpointsHelp);
          }

          const created =
            studyKind === "axial_bar_1d"
              ? await createDirectMeshSolve<AxialBarResult>(
                  "axial_bar_1d",
                  toAxialInput(axialForm),
                  endpoints,
                  directMeshSelectionMode,
                )
              : studyKind === "truss_2d"
                ? await createDirectMeshSolve<Truss2dResult>(
                    "truss_2d",
                    resolveTruss2dJobInput(trussModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
                : studyKind === "truss_3d"
                  ? await createDirectMeshSolve<Truss3dResult>(
                      "truss_3d",
                      resolveTruss3dJobInput(truss3dModel) as unknown as Record<string, unknown>,
                      endpoints,
                      directMeshSelectionMode,
                    )
                  : await createDirectMeshSolve<PlaneTriangle2dResult>(
                      "plane_triangle_2d",
                      resolvePlaneTriangle2dJobInput(planeModel) as unknown as Record<string, unknown>,
                      endpoints,
                      directMeshSelectionMode,
                    );

          setJob(created.job);
          if (created.result) {
            setResult(created.result);
          }
          setDirectMeshExecution({
            endpoint: created.direct_mesh.endpoint,
            strategy: created.direct_mesh.strategy,
            at: new Date().toISOString(),
          });
          setMessage(`${t.directMeshCompleted}: ${created.job.worker_id ?? "direct-mesh"}`);
          return;
        }

        const jobContext = {
          ...(selectedProjectId ? { project_id: selectedProjectId } : {}),
          ...(selectedVersionId ? { model_version_id: selectedVersionId } : {}),
        };

        const created =
          studyKind === "axial_bar_1d"
            ? await createAxialBarJob({ ...toAxialInput(axialForm), ...jobContext })
            : studyKind === "truss_2d"
              ? await createTruss2dJob(resolveTruss2dJobInput({ ...trussModel, ...jobContext }))
              : studyKind === "truss_3d"
                ? await createTruss3dJob(resolveTruss3dJobInput({ ...truss3dModel, ...jobContext }))
              : await createPlaneTriangle2dJob(resolvePlaneTriangle2dJobInput({ ...planeModel, ...jobContext }));

        setJob(created.job);
        await refreshJobHistory();
        await pollJob(created.job.job_id, studyKind);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const handleCanvasStageScroll = (event: ReactUIEvent<HTMLDivElement>) => {
    if (!activeResultWindow) return;
    if (chunkScrollFrameRef.current !== null) return;

    const target = event.currentTarget;
    if (target.clientWidth > 0) {
      setCanvasViewportWidth((current) => (current === target.clientWidth ? current : target.clientWidth));
    }
    const previousLeft = chunkScrollLeftRef.current;
    chunkScrollDirectionRef.current =
      target.scrollLeft > previousLeft ? 1 : target.scrollLeft < previousLeft ? -1 : 0;
    chunkScrollLeftRef.current = target.scrollLeft;

    chunkScrollFrameRef.current = window.requestAnimationFrame(() => {
      chunkScrollFrameRef.current = null;

      const maxScrollLeft = Math.max(0, target.scrollWidth - target.clientWidth);
      if (maxScrollLeft <= 0) return;

      const nextOffset = computeVisibleResultWindowOffset(
        resultWindowMaxTotal,
        resultWindowLimit,
        target.clientWidth,
        target.scrollLeft,
        target.scrollWidth,
      );

      setResultWindowOffset((current) => (current === nextOffset ? current : nextOffset));
    });
  };

  const pollJob = async (jobId: string, kind: StudyKind) => {
    for (let attempt = 0; attempt < 40; attempt += 1) {
      const payload =
        kind === "axial_bar_1d"
          ? await fetchJobStatus<AxialBarResult>(jobId)
          : kind === "truss_2d"
            ? await fetchJobStatus<Truss2dResult>(jobId)
            : kind === "truss_3d"
              ? await fetchJobStatus<Truss3dResult>(jobId)
            : await fetchJobStatus<PlaneTriangle2dResult>(jobId);

      setJob(payload.job);

      if (payload.result) {
        setResult(payload.result);
      }

      setMessage(formatJobMessage(payload.job, `${jobId} ${payload.job.status}`, t));

      if (payload.job.status === "completed" || payload.job.status === "failed" || payload.job.status === "cancelled") {
        await refreshJobHistory();
        return;
      }

      await new Promise((resolve) => window.setTimeout(resolve, 250));
    }
  };

  const openHistoryJob = (jobId: string) => {
    startTransition(async () => {
      try {
        const payload = await fetchJobStatus<AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult>(jobId);
        setJob(payload.job);

        if (payload.result) {
          setResult(payload.result);

          if (isAxialResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("axial_bar_1d");
            setAxialForm({
              length: payload.result.input.length,
              area: payload.result.input.area,
              elements: payload.result.input.elements,
              tipForce: payload.result.input.tip_force,
              material: activeMaterial,
              youngsModulusGpa: round(payload.result.input.youngs_modulus / 1.0e9),
            });
          }

          if (isTrussResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("truss_2d");
            setTrussModel(ensureTrussModelMaterials(payload.result.input, activeMaterial));
            setSidebarSection("study");
          }

          if (isTruss3dResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("truss_3d");
            setTruss3dModel(ensureTruss3dModelMaterials(payload.result.input, activeMaterial));
            setSidebarSection("study");
          }

          if (isPlaneTriangleResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("plane_triangle_2d");
            setPlaneModel(ensurePlaneModelMaterials(payload.result.input, activeMaterial));
            setSidebarSection("study");
          }
        }

        setMessage(payload.job.status === "failed" ? formatJobMessage(payload.job, t.historyLoaded, t) : t.historyLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const cancelCurrentJob = () => {
    if (!job?.job_id || !jobIsActive) return;

    startTransition(async () => {
      try {
        const payload = await cancelJob(job.job_id);
        setJob(payload.job);
        setMessage(t.jobCancelled);
        await refreshJobHistory();
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const importModel = async (file: File | undefined) => {
    if (!file) return;

    try {
      const imported = parsePlaygroundModel(await file.text());
      recordHistory(t.importAction);
      setLoadedModelName(imported.name);
      setSelectedModelId(null);
      setSelectedVersionId(null);
      setModelVersions([]);
      setMessage(`${t.importedModel}: ${imported.name}`);

      if (imported.kind === "truss_2d") {
        setStudyKind("truss_2d");
        setTrussModel(ensureTrussModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
        setParametric((current) => ({
          ...current,
          youngsModulusGpa: imported.youngsModulusGpa,
        }));
      } else if (imported.kind === "truss_3d") {
        setStudyKind("truss_3d");
        setTruss3dModel(ensureTruss3dModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
      } else if (imported.kind === "plane_triangle_2d") {
        setStudyKind("plane_triangle_2d");
        setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
      } else {
        setStudyKind("axial_bar_1d");
        setAxialForm({
          length: imported.length,
          area: imported.area,
          elements: imported.elements,
          tipForce: imported.tipForce,
          material: imported.material,
          youngsModulusGpa: imported.youngsModulusGpa,
        });
        setActiveMaterial(imported.material);
      }
    } catch (error) {
      setMessage(error instanceof Error ? `${t.importFailed}: ${error.message}` : t.importFailed);
    }
  };

  const openSample = (href: string) => {
    startTransition(async () => {
      try {
        const response = await fetch(href, { cache: "no-store" });
        const text = await response.text();
        const imported = parsePlaygroundModel(text);
        recordHistory(t.sampleAction);
        setLoadedModelName(imported.name);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);

        if (imported.kind === "plane_triangle_2d") {
          setStudyKind("plane_triangle_2d");
          setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
        } else if (imported.kind === "truss_3d") {
          setStudyKind("truss_3d");
          setTruss3dModel(ensureTruss3dModelMaterials(imported.model, imported.material));
        } else if (imported.kind === "truss_2d") {
          setStudyKind("truss_2d");
          setTrussModel(ensureTrussModelMaterials(imported.model, imported.material));
        } else {
          setStudyKind("axial_bar_1d");
          setAxialForm({
            length: imported.length,
            area: imported.area,
            elements: imported.elements,
            tipForce: imported.tipForce,
            material: imported.material,
            youngsModulusGpa: imported.youngsModulusGpa,
          });
        }

        setMessage(`${t.importedModel}: ${imported.name}`);
      } catch (error) {
        setMessage(error instanceof Error ? `${t.importFailed}: ${error.message}` : t.importFailed);
      }
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
    setLanguage(nextLanguage);
    setLoadedModelName((current) =>
      current === copy.en.defaultModel || current === copy.zh.defaultModel
        ? copy[nextLanguage].defaultModel
        : current,
    );
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
    const nextModel = ensurePlaneModelMaterials(generateRectangularPanelMesh(panelParametric), activeMaterial);
    setStudyKind("plane_triangle_2d");
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
        trussModel,
        truss3dModel,
        planeModel,
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
    if (!result) {
      setMessage(t.noResultToExport);
      return;
    }

    const payload = {
      exported_at: new Date().toISOString(),
      study_kind: studyKind,
      model_name: loadedModelName,
      job,
      result,
    };

    downloadTextFile(`${loadedModelName || "kyuubiki-study"}-result.json`, JSON.stringify(payload, null, 2));
    setMessage(t.resultJsonDownloaded);
  };

  const downloadResultCsv = () => {
    if (!result) {
      setMessage(t.noResultToExport);
      return;
    }

    const csv = serializeResultCsv(studyKind, job, result);
    downloadTextFile(`${loadedModelName || "kyuubiki-study"}-result.csv`, csv);
    setMessage(t.resultCsvDownloaded);
  };

  const buildProjectBundleJson = async () => {
    if (!selectedProject) {
      throw new Error(t.projectRequired);
    }

    const modelDetails = await Promise.all(
      selectedProjectModels.map(async (model) => {
        const modelEnvelope = await fetchModel(model.model_id);
        const versionsEnvelope = await fetchModelVersions(model.model_id);
        return {
          model: modelEnvelope.model,
          versions: versionsEnvelope.versions,
        };
      }),
    );

    const resultCandidates: Array<JobResultRecord | null> = await Promise.all(
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

    const results = resultCandidates.filter((entry): entry is JobResultRecord => entry !== null);

    return exportProjectBundle({
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
        trussModel,
        truss3dModel,
        planeModel,
        parametric,
        round,
      ),
      jobs: jobHistory,
      results,
    });
  };

  const downloadProjectBundleJson = async () => {
    try {
      const bundle = await buildProjectBundleJson();
      downloadTextFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki.json`, bundle);
      setMessage(t.projectExported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const downloadProjectBundleZip = async () => {
    try {
      const bundle = await buildProjectBundleJson();
      const blob = await exportProjectBundleZip(bundle);
      downloadBlobFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki`, blob);
      setMessage(t.projectExported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const downloadDatabaseSnapshot = async () => {
    try {
      const snapshot = await fetchDatabaseExport();
      const timestamp = snapshot.exported_at.replaceAll(":", "-");
      downloadTextFile(`kyuubiki-database-${timestamp}.json`, JSON.stringify(snapshot, null, 2));
      setMessage(t.databaseExported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
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
      try {
        const parsed = JSON.parse(adminResultDraft) as Record<string, unknown>;
        await updateResultRecord(selectedAdminResultJobId, parsed);
        await refreshResults();
        setMessage(t.resultSaved);
      } catch (error) {
        setMessage(error instanceof SyntaxError ? t.invalidJson : error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteAdminResultRecord = () => {
    if (!selectedAdminResultJobId) return;

    startTransition(async () => {
      try {
        await deleteResultRecord(selectedAdminResultJobId);
        await refreshResults();
        setMessage(t.resultDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const exportAdminResultRecord = () => {
    if (!selectedAdminResultJobId) return;

    try {
      const parsed = JSON.parse(adminResultDraft);
      downloadTextFile(`${selectedAdminResultJobId}-result.json`, JSON.stringify(parsed, null, 2));
      setMessage(t.resultJsonDownloaded);
    } catch {
      setMessage(t.invalidJson);
    }
  };

  const importProjectBundle = async (file: File | undefined) => {
    if (!file) return;

    try {
      const bundle = await parseProjectBundleFile(file);
      const createdProject = await createProject({
        name: bundle.project.name,
        description: bundle.project.description ?? "",
      });

      const modelIdMap = new Map<string, string>();

      for (const bundledModel of bundle.models) {
        const bundledVersions = bundle.model_versions
          .filter((version) => version.model_id === bundledModel.model_id)
          .sort((left, right) => left.version_number - right.version_number);

        const baseVersion = bundledVersions[0];
        const createdModel = await createModel(createdProject.project.project_id, {
          name: baseVersion?.name || bundledModel.name,
          kind: bundledModel.kind,
          material: bundledModel.material ?? undefined,
          model_schema_version: bundledModel.model_schema_version,
          payload: (baseVersion?.payload ?? bundledModel.payload) as Record<string, unknown>,
        });

        const newModelId = createdModel.model.model_id;
        modelIdMap.set(bundledModel.model_id, newModelId);

        const initialVersionId = createdModel.model.versions?.[0]?.version_id;
        if (initialVersionId && baseVersion?.name) {
          await updateModelVersion(initialVersionId, { name: baseVersion.name });
        }

        for (const extraVersion of bundledVersions.slice(1)) {
          await createModelVersion(newModelId, {
            name: extraVersion.name,
            kind: extraVersion.kind,
            material: extraVersion.material ?? undefined,
            model_schema_version: extraVersion.model_schema_version,
            payload: extraVersion.payload,
          });
        }
      }

      await refreshProjects();

      const importedActiveModelId =
        (bundle.active_model_id && modelIdMap.get(bundle.active_model_id)) ||
        [...modelIdMap.values()][0] ||
        null;

      setSelectedProjectId(createdProject.project.project_id);
      setSelectedModelId(importedActiveModelId);

      if (bundle.workspace_snapshot) {
        recordHistory(t.importAction);
        applyPersistedPayload(bundle.workspace_snapshot, bundle.project.name);
      }

      if (importedActiveModelId) {
        await refreshVersions(importedActiveModelId);
      } else {
        setModelVersions([]);
      }

      setMessage(t.projectImported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.importFailed);
    }
  };

  const applyPersistedPayload = (payload: Record<string, unknown>, fallbackName?: string) => {
    const imported = parsePlaygroundModel(JSON.stringify(payload));

    if (imported.kind === "plane_triangle_2d") {
      setStudyKind("plane_triangle_2d");
      setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
    } else if (imported.kind === "truss_3d") {
      setStudyKind("truss_3d");
      setTruss3dModel(ensureTruss3dModelMaterials(imported.model, imported.material));
    } else if (imported.kind === "truss_2d") {
      setStudyKind("truss_2d");
      setTrussModel(ensureTrussModelMaterials(imported.model, imported.material));
    } else {
      setStudyKind("axial_bar_1d");
      setAxialForm({
        length: imported.length,
        area: imported.area,
        elements: imported.elements,
        tipForce: imported.tipForce,
        material: imported.material,
        youngsModulusGpa: imported.youngsModulusGpa,
      });
    }

    setLoadedModelName(fallbackName ?? imported.name);
    setActiveMaterial(imported.material);
    resetActiveResult(setResult, setJob);
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
      trussModel,
      truss3dModel,
      planeModel,
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
    startTransition(async () => {
      try {
        const payload = await fetchModel(model.model_id);
        recordHistory(t.historyAction);
        applyPersistedPayload(payload.model.payload, payload.model.name);
        setSelectedProjectId(payload.model.project_id);
        setSelectedModelId(payload.model.model_id);
        setSelectedVersionId(payload.model.latest_version_id ?? null);
        await refreshVersions(payload.model.model_id);
        setMessage(t.persistedModelLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const openSavedVersion = (version: ModelVersionRecord) => {
    startTransition(async () => {
      try {
        const payload = await fetchModelVersion(version.version_id);
        recordHistory(t.historyAction);
        applyPersistedPayload(payload.version.payload, payload.version.name);
        setSelectedModelId(payload.version.model_id);
        setSelectedProjectId(payload.version.project_id);
        setSelectedVersionId(payload.version.version_id);
        setMessage(t.versionLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
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
    { key: "study", label: t.rail.study, symbol: "S" },
    { key: "model", label: t.rail.model, symbol: "M" },
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

  const isAxial = studyKind === "axial_bar_1d";
  const isTruss = studyKind === "truss_2d";
  const isTruss3d = studyKind === "truss_3d";
  const isPlane = studyKind === "plane_triangle_2d";
  const jobIsActive =
    job?.status === "queued" ||
    job?.status === "preprocessing" ||
    job?.status === "partitioning" ||
    job?.status === "solving" ||
    job?.status === "postprocessing";
  const axialResult = isAxial && isAxialResult(result) ? result : null;
  const trussResult = isTruss && isTrussResult(result) ? result : null;
  const truss3dResult = isTruss3d && isTruss3dResult(result) ? result : null;
  const planeResult = isPlane && isPlaneTriangleResult(result) ? result : null;
  const activeResultWindow =
    resultWindow && job?.job_id === resultWindow.jobId && studyKind === resultWindow.studyKind ? resultWindow : null;
  const trussDiagnostics = isTruss ? analyzeTrussModel(trussModel, t, selectedNode) : null;
  const trussStability = isTruss && trussDiagnostics ? summarizeTrussStability(trussModel, trussDiagnostics) : null;
  const axialNodes = axialResult?.nodes ?? [];
  const axialElements = axialResult?.elements ?? [];
  const axialLength = axialResult?.input.length ?? axialForm.length;
  const axialScale = axialResult?.max_displacement ? 140 / axialResult.max_displacement : 1;
  const displayTrussNodes = buildDisplayTrussNodes(
    trussModel,
    trussResult,
    activeResultWindow?.studyKind === "truss_2d" ? (activeResultWindow.nodes as Truss2dResult["nodes"]) : undefined,
  );
  const displayTrussElements = buildDisplayTrussElements(
    trussModel,
    trussResult,
    activeResultWindow?.studyKind === "truss_2d" ? (activeResultWindow.elements as Truss2dResult["elements"]) : undefined,
  );
  const trussBounds = getTrussBounds(displayTrussNodes);
  const displayTruss3dNodes = buildDisplayTruss3dNodes(
    truss3dModel,
    truss3dResult,
    activeResultWindow?.studyKind === "truss_3d" ? (activeResultWindow.nodes as Truss3dResult["nodes"]) : undefined,
  );
  const displayTruss3dElements = buildDisplayTruss3dElements(
    truss3dModel,
    truss3dResult,
    activeResultWindow?.studyKind === "truss_3d" ? (activeResultWindow.elements as Truss3dResult["elements"]) : undefined,
  );
  const planeWindowNodes =
    activeResultWindow?.studyKind === "plane_triangle_2d" ? (activeResultWindow.nodes as PlaneTriangle2dResult["nodes"]) : undefined;
  const planeWindowElements =
    activeResultWindow?.studyKind === "plane_triangle_2d" ? (activeResultWindow.elements as PlaneTriangle2dResult["elements"]) : undefined;
  const planeNodes =
    (planeWindowNodes ?? planeResult?.nodes)?.map((node, index) => ({
      ...planeModel.nodes[node.index ?? index],
      ...node,
      fix_x: planeModel.nodes[node.index ?? index]?.fix_x ?? false,
      fix_y: planeModel.nodes[node.index ?? index]?.fix_y ?? false,
      load_x: planeModel.nodes[node.index ?? index]?.load_x ?? 0,
      load_y: planeModel.nodes[node.index ?? index]?.load_y ?? 0,
    })) ??
    planeModel.nodes.map((node, index) => ({ ...node, index, ux: 0, uy: 0 }));
  const planeElements =
    (planeWindowElements ?? planeResult?.elements)?.map((element) => ({
      ...element,
      material_id: planeModel.elements[element.index]?.material_id,
    })) ??
    planeModel.elements.map((element, index) => ({ ...element, index, area: 0, strain_x: 0, strain_y: 0, gamma_xy: 0, stress_x: 0, stress_y: 0, tau_xy: 0, von_mises: 0 }));
  const planeBounds = getTrussBounds(planeNodes);
  const selectedNodeData = selectedNode !== null ? displayTrussNodes[selectedNode] : null;
  const heartbeatStatusValue = heartbeatStatus(job, t);
  const heartbeatToneValue = heartbeatTone(job);
  const selectedElementData = selectedElement !== null ? displayTrussElements[selectedElement] : null;
  const selectedTruss3dNodeData = selectedNode !== null ? displayTruss3dNodes[selectedNode] : null;
  const selectedTruss3dElementData = selectedElement !== null ? displayTruss3dElements[selectedElement] : null;
  const selectedPlaneNodeData = selectedNode !== null ? planeNodes[selectedNode] : null;
  const selectedPlaneElementData = selectedElement !== null ? planeElements[selectedElement] : null;
  const selectedNodeIssues =
    selectedNode !== null && trussDiagnostics ? trussDiagnostics.nodeIssues[selectedNode] ?? [] : [];
  const translatedFailureReason = humanizeSolverFailure(job?.message, t);
  const securityUi =
    language === "zh"
      ? {
          controlPlaneToken: "控制面 API Token",
          clusterToken: "集群 API Token",
          directMeshToken: "直连网格 Token",
          protectReads: "保护只读接口",
          clusterWindow: "集群时间窗",
          directMeshRoutes: "直连网格路由",
          security: "安全",
          enabled: "已启用",
          disabled: "未启用",
          configured: "已配置",
          notConfigured: "未配置",
          mutatingRoutes: "写入路由保护",
          clusterRoutes: "集群路由保护",
        }
      : {
          controlPlaneToken: "Control-plane API token",
          clusterToken: "Cluster API token",
          directMeshToken: "Direct mesh token",
          protectReads: "Protect reads",
          clusterWindow: "Cluster time window",
          directMeshRoutes: "Direct mesh routes",
          security: "Security",
          enabled: "Enabled",
          disabled: "Disabled",
          configured: "Configured",
          notConfigured: "Not configured",
          mutatingRoutes: "Mutating routes",
          clusterRoutes: "Cluster routes",
        };
  const planeMaxVonMises = Math.max(...planeElements.map((element) => element.von_mises ?? 0), 0);
  const currentMaterials =
    studyKind === "truss_2d"
      ? trussModel.materials ?? []
      : studyKind === "truss_3d"
        ? truss3dModel.materials ?? []
        : studyKind === "plane_triangle_2d"
          ? planeModel.materials ?? []
          : [];
  const hiddenMaterialIds = hiddenMaterials[studyKind] ?? [];
  const materialColorMap = new Map(currentMaterials.map((material, index) => [material.id, materialColorByIndex(index)]));
  const materialOptions = currentMaterials.map((material) => ({
    id: material.id,
    label: `${material.name} (${round(material.youngs_modulus / 1.0e9)} GPa)`,
  }));
  const trussElementColors = displayTrussElements.map((element) => materialColorMap.get(element.material_id ?? "") ?? "#1677a3");
  const truss3dElementColors = displayTruss3dElements.map((element) => materialColorMap.get(element.material_id ?? "") ?? "#1677a3");
  const planeElementColors = planeModel.elements.map((element) => materialColorMap.get(element.material_id ?? "") ?? planeStressFill(0, 1));
  const nodeCount =
    isAxial
      ? axialNodes.length
      : activeResultWindow?.totalNodes ??
        (isTruss ? trussResult?.nodes.length : isTruss3d ? truss3dResult?.nodes.length : planeResult?.nodes.length) ??
        (isTruss ? trussModel.nodes.length : isTruss3d ? truss3dModel.nodes.length : planeModel.nodes.length);
  const resultWindowMaxTotal = activeResultWindow ? Math.max(activeResultWindow.totalNodes, activeResultWindow.totalElements) : 0;
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
  const hasAnyResult = Boolean(axialResult || trussResult || truss3dResult || planeResult);
  const assistantCards: Array<{
    id: string;
    title: string;
    summary: string;
    actionLabel: string;
    tone: "good" | "watch" | "risk";
    onAction: () => void;
  }> = [];

  if (!selectedProjectId) {
    assistantCards.push({
      id: "project",
      title: t.assistantNeedsProject,
      summary: t.assistantNeedsProjectHint,
      actionLabel: t.assistantOpenProjects,
      tone: "watch",
      onAction: () => {
        setSidebarSection("library");
        setLibraryTab("projects");
      },
    });
  }

  if (frontendRuntimeMode === "direct_mesh_gui" && directMeshEndpoints.length === 0) {
    assistantCards.push({
      id: "direct-mesh",
      title: t.assistantConfigureDirectMesh,
      summary: t.assistantConfigureDirectMeshHint,
      actionLabel: t.settings,
      tone: "risk",
      onAction: () => {
        setSidebarSection("system");
        setSystemPanelTab("config");
      },
    });
  }

  if (!health) {
    assistantCards.push({
      id: "runtime",
      title: t.assistantRefreshRuntime,
      summary: t.assistantRefreshRuntimeHint,
      actionLabel: t.refresh,
      tone: "watch",
      onAction: () => {
        void refreshHealth();
      },
    });
  }

  if (jobIsActive) {
    assistantCards.push({
      id: "job",
      title: t.assistantCancelRun,
      summary: t.assistantCancelRunHint,
      actionLabel: t.cancelJob,
      tone: "watch",
      onAction: cancelCurrentJob,
    });
  } else if (isTruss && trussDiagnostics?.blockingMessages.length && trussDiagnostics.suggestions[0]) {
    assistantCards.push({
      id: "fix",
      title: t.assistantApplyFix,
      summary: `${t.assistantApplyFixHint} ${trussDiagnostics.blockingMessages[0]}`,
      actionLabel: t.assistantApplyFix,
      tone: "risk",
      onAction: () => applyTrussSuggestion(trussDiagnostics.suggestions[0]),
    });
  } else if (!hasAnyResult) {
    assistantCards.push({
      id: "run",
      title: t.assistantRunStudy,
      summary: t.assistantRunStudyHint,
      actionLabel: t.run,
      tone: "good",
      onAction: runAnalysis,
    });
  }

  if (isTruss3d && !immersiveViewport) {
    assistantCards.push({
      id: "immersive",
      title: t.assistantEnterImmersive,
      summary: t.assistantEnterImmersiveHint,
      actionLabel: t.enterImmersive,
      tone: "good",
      onAction: () => {
        void toggleImmersiveViewport();
      },
    });
  }

  const requestLlmAssistantPlan = async (prompt: string): Promise<AssistantPlan> =>
    requestWorkbenchAssistantPlan({
      baseUrl: assistantApiBaseUrl,
      apiKey: assistantApiKey,
      model: assistantModel,
      prompt,
      snapshot: getScriptSnapshot(),
      localHints: assistantCards.map((card) => ({
        id: card.id,
        title: card.title,
        summary: card.summary,
        actionLabel: card.actionLabel,
      })),
    });

  const scriptSnapshot: WorkbenchScriptSnapshot = {
    studyKind,
    sidebarSection,
    studyTab,
    modelTab,
    libraryTab,
    systemPanelTab,
    language,
    theme,
    frontendRuntimeMode,
    selectedProjectId,
    selectedModelId,
    selectedVersionId,
    loadedModelName,
    activeMaterial,
    selectedNode,
    selectedElement,
    hasResult: hasAnyResult,
    jobStatus: job?.status ?? null,
    projectCount: projects.length,
    jobHistoryCount: jobHistory.length,
    resultCount: resultRecords.length,
    protocolAgentCount: protocolAgents.length,
    healthStatus: health?.status ?? null,
    message,
  };

  const appendScriptActionLog = (entry: Omit<WorkbenchScriptActionLogEntry, "id" | "at">) => {
    setScriptActionLog((current) => [
      {
        id: `${Date.now()}-${current.length}`,
        at: new Date().toISOString(),
        ...entry,
      },
      ...current,
    ].slice(0, 40));
  };

  const getScriptSnapshot = (): WorkbenchScriptSnapshot => ({
    studyKind,
    sidebarSection,
    studyTab,
    modelTab,
    libraryTab,
    systemPanelTab,
    language,
    theme,
    frontendRuntimeMode,
    selectedProjectId,
    selectedModelId,
    selectedVersionId,
    loadedModelName,
    activeMaterial,
    selectedNode,
    selectedElement,
    hasResult: Boolean(result),
    jobStatus: job?.status ?? null,
    projectCount: projects.length,
    jobHistoryCount: jobHistory.length,
    resultCount: resultRecords.length,
    protocolAgentCount: protocolAgents.length,
    healthStatus: health?.status ?? null,
    message,
  });

  const invokeScriptAction = async (action: string, payload: Record<string, unknown> = {}) => {
    appendScriptActionLog({ action, status: "started", summary: JSON.stringify(payload) });

    try {
      let resultPayload: Record<string, unknown>;
      switch (action) {
        case "nav/setSidebarSection": {
          const section = payload.section;
          if (section === "study" || section === "model" || section === "library" || section === "system") {
            setSidebarSection(section);
          }
          resultPayload = { ok: true, action, section };
          break;
        }
        case "nav/setStudyKind": {
          const nextStudyKind = payload.studyKind;
          if (
            nextStudyKind === "axial_bar_1d" ||
            nextStudyKind === "truss_2d" ||
            nextStudyKind === "truss_3d" ||
            nextStudyKind === "plane_triangle_2d"
          ) {
            recordHistory(t.changeStudyType);
            setStudyKind(nextStudyKind);
          }
          resultPayload = { ok: true, action, studyKind: nextStudyKind };
          break;
        }
        case "settings/patch": {
          if (payload.language === "en" || payload.language === "zh") {
            handleLanguageChange(payload.language);
          }
          if (payload.theme === "linen" || payload.theme === "marine" || payload.theme === "graphite") {
            setTheme(payload.theme);
          }
          if (payload.frontendRuntimeMode === "orchestrated_gui" || payload.frontendRuntimeMode === "direct_mesh_gui") {
            setFrontendRuntimeMode(payload.frontendRuntimeMode);
          }
          if (typeof payload.directMeshEndpointsText === "string") {
            setDirectMeshEndpointsText(payload.directMeshEndpointsText);
          }
          if (payload.directMeshSelectionMode === "healthiest" || payload.directMeshSelectionMode === "first_reachable") {
            setDirectMeshSelectionMode(payload.directMeshSelectionMode);
          }
          resultPayload = { ok: true, action };
          break;
        }
        case "runtime/refreshAll": {
          await Promise.all([refreshHealth(), refreshJobHistory(), refreshResults(), refreshProjects()]);
          resultPayload = { ok: true, action };
          break;
        }
        case "project/create": {
          const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : t.defaultProject;
          const description = typeof payload.description === "string" ? payload.description : "";
          const created = await createProject({ name, description });
          setSelectedProjectId(created.project.project_id);
          setProjectNameDraft(created.project.name);
          setProjectDescriptionDraft(created.project.description ?? "");
          await refreshProjects();
          setMessage(t.projectCreated);
          resultPayload = { ok: true, action, projectId: created.project.project_id };
          break;
        }
        case "project/select": {
          const projectId = typeof payload.projectId === "string" ? payload.projectId : null;
          if (projectId) {
            setSelectedProjectId(projectId);
          }
          resultPayload = { ok: true, action, projectId };
          break;
        }
        case "project/updateSelected": {
          if (!selectedProjectId) {
            throw new Error(t.projectRequired);
          }
          const name = typeof payload.name === "string" && payload.name.trim() ? payload.name.trim() : projectNameDraft || t.defaultProject;
          const description = typeof payload.description === "string" ? payload.description : projectDescriptionDraft;
          await updateProject(selectedProjectId, { name, description });
          setProjectNameDraft(name);
          setProjectDescriptionDraft(description);
          await refreshProjects();
          setMessage(t.projectUpdated);
          resultPayload = { ok: true, action, projectId: selectedProjectId };
          break;
        }
        case "project/deleteSelected": {
          if (!selectedProjectId) {
            throw new Error(t.projectRequired);
          }
          await deleteProject(selectedProjectId);
          setSelectedProjectId(null);
          setSelectedModelId(null);
          setSelectedVersionId(null);
          await refreshProjects();
          setMessage(t.projectDeleted);
          resultPayload = { ok: true, action };
          break;
        }
        case "project/exportJson": {
          await downloadProjectBundleJson();
          resultPayload = { ok: true, action };
          break;
        }
        case "project/exportZip": {
          await downloadProjectBundleZip();
          resultPayload = { ok: true, action };
          break;
        }
        case "model/generateTruss": {
          generateModel();
          resultPayload = { ok: true, action };
          break;
        }
        case "model/generatePanel": {
          generatePanelModel();
          resultPayload = { ok: true, action };
          break;
        }
        case "model/save":
        case "model/saveAs": {
          if (!selectedProjectId) {
            throw new Error(t.projectRequired);
          }
          const payloadModel = serializeCurrentModel(
            studyKind,
            loadedModelName,
            activeMaterial,
            axialForm,
            trussModel,
            truss3dModel,
            planeModel,
            parametric,
            round,
          );
          if (!selectedModelId || action === "model/saveAs") {
            const created = await createModel(selectedProjectId, {
              name: loadedModelName,
              kind: studyKind,
              material: activeMaterial,
              model_schema_version: String(payloadModel.model_schema_version ?? "kyuubiki.model/v1"),
              payload: payloadModel,
            });
            setSelectedModelId(created.model.model_id);
            setSelectedVersionId(created.model.latest_version_id ?? null);
            await refreshProjects();
            await refreshVersions(created.model.model_id);
            setMessage(t.modelCreated);
            resultPayload = { ok: true, action, modelId: created.model.model_id };
            break;
          }

          await updateModel(selectedModelId, {
            name: loadedModelName,
            kind: studyKind,
            material: activeMaterial,
            model_schema_version: String(payloadModel.model_schema_version ?? "kyuubiki.model/v1"),
            payload: payloadModel,
          });
          const version = await createModelVersion(selectedModelId, {
            name: loadedModelName,
            kind: studyKind,
            material: activeMaterial,
            model_schema_version: String(payloadModel.model_schema_version ?? "kyuubiki.model/v1"),
            payload: payloadModel,
          });
          setSelectedVersionId(version.version.version_id);
          await refreshProjects();
          await refreshVersions(selectedModelId);
          setMessage(t.modelSaved);
          resultPayload = { ok: true, action, versionId: version.version.version_id };
          break;
        }
        case "model/deleteSelected": {
          if (!selectedModelId) {
            throw new Error(t.noSavedModels);
          }
          await deleteModel(selectedModelId);
          setSelectedModelId(null);
          setSelectedVersionId(null);
          setModelVersions([]);
          await refreshProjects();
          setMessage(t.modelDeletedStored);
          resultPayload = { ok: true, action };
          break;
        }
        case "model/renameSelectedVersion": {
          if (!selectedVersionId) {
            throw new Error(t.noVersions);
          }
          await updateModelVersion(selectedVersionId, { name: loadedModelName });
          await refreshVersions(selectedModelId ?? "");
          setMessage(t.versionRenamed);
          resultPayload = { ok: true, action, versionId: selectedVersionId };
          break;
        }
        case "model/deleteSelectedVersion": {
          if (!selectedVersionId) {
            throw new Error(t.noVersions);
          }
          await deleteModelVersion(selectedVersionId);
          setSelectedVersionId(null);
          if (selectedModelId) {
            await refreshVersions(selectedModelId);
          }
          await refreshProjects();
          setMessage(t.versionDeleted);
          resultPayload = { ok: true, action };
          break;
        }
        case "model/setWorkspaceMeta": {
          if (typeof payload.loadedModelName === "string") {
            setLoadedModelName(payload.loadedModelName);
          }
          if (typeof payload.activeMaterial === "string") {
            setActiveMaterial(payload.activeMaterial);
          }
          resultPayload = { ok: true, action };
          break;
        }
        case "state/setParametric": {
          recordHistory(t.editParametric);
          setParametric((current) => ({ ...current, ...(payload as Partial<ParametricTrussConfig>) }));
          resultPayload = { ok: true, action };
          break;
        }
        case "state/setPanelParametric": {
          recordHistory(t.editParametric);
          setPanelParametric((current) => ({ ...current, ...(payload as Partial<ParametricPanelConfig>) }));
          resultPayload = { ok: true, action };
          break;
        }
        case "state/replaceTruss2dModel": {
          recordHistory(t.importAction);
          setStudyKind("truss_2d");
          setTrussModel(resolveTruss2dJobInput(payload as unknown as Truss2dJobInput));
          resetActiveResult(setResult, setJob);
          resultPayload = { ok: true, action };
          break;
        }
        case "state/replaceTruss3dModel": {
          recordHistory(t.importAction);
          setStudyKind("truss_3d");
          setTruss3dModel(resolveTruss3dJobInput(payload as unknown as Truss3dJobInput));
          resetActiveResult(setResult, setJob);
          resultPayload = { ok: true, action };
          break;
        }
        case "state/replacePlaneModel": {
          recordHistory(t.importAction);
          setStudyKind("plane_triangle_2d");
          setPlaneModel(resolvePlaneTriangle2dJobInput(payload as unknown as PlaneTriangle2dJobInput));
          resetActiveResult(setResult, setJob);
          resultPayload = { ok: true, action };
          break;
        }
        case "selection/set": {
          setSelectedNode(typeof payload.nodeIndex === "number" ? payload.nodeIndex : null);
          setSelectedElement(typeof payload.elementIndex === "number" ? payload.elementIndex : null);
          resultPayload = { ok: true, action };
          break;
        }
        case "job/run": {
          runAnalysis();
          resultPayload = { ok: true, action };
          break;
        }
        case "job/cancel": {
          cancelCurrentJob();
          resultPayload = { ok: true, action };
          break;
        }
        case "history/undo": {
          handleUndo();
          resultPayload = { ok: true, action };
          break;
        }
        case "history/redo": {
          handleRedo();
          resultPayload = { ok: true, action };
          break;
        }
        case "viewport/toggleImmersive": {
          await toggleImmersiveViewport();
          resultPayload = { ok: true, action };
          break;
        }
        case "viewport/set3dView": {
          if (payload.preset === "iso" || payload.preset === "front" || payload.preset === "right" || payload.preset === "top") {
            setTruss3dViewPreset(payload.preset);
          }
          if (payload.projection === "ortho" || payload.projection === "persp") {
            setTruss3dProjectionMode(payload.projection);
          }
          resultPayload = { ok: true, action };
          break;
        }
        case "viewport/focus3d": {
          setTruss3dFocusRequestVersion((current) => current + 1);
          resultPayload = { ok: true, action };
          break;
        }
        case "viewport/reset3d": {
          setTruss3dResetRequestVersion((current) => current + 1);
          resultPayload = { ok: true, action };
          break;
        }
        case "viewport/toggleFlags": {
          if (typeof payload.grid === "boolean") {
            setTruss3dShowGrid(payload.grid);
          }
          if (typeof payload.labels === "boolean") {
            setTruss3dShowLabels(payload.labels);
          }
          if (typeof payload.nodes === "boolean") {
            setTruss3dShowNodes(payload.nodes);
          }
          resultPayload = { ok: true, action };
          break;
        }
        case "data/exportDatabase": {
          await downloadDatabaseSnapshot();
          resultPayload = { ok: true, action };
          break;
        }
        default:
          throw new Error(`Unknown script action: ${action}`);
      }

      appendScriptActionLog({ action, status: "completed", summary: JSON.stringify(resultPayload) });
      return resultPayload;
    } catch (error) {
      const summary = error instanceof Error ? error.message : String(error);
      appendScriptActionLog({ action, status: "failed", summary });
      throw error;
    }
  };

  const buildSnapshot = (): WorkbenchSnapshot => ({
    ...buildWorkbenchSnapshot({
      studyKind,
      axialForm,
      trussModel,
      truss3dModel,
      planeModel,
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
        setTrussModel,
        setTruss3dModel,
        setPlaneModel,
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

  const recordAssistantTransaction = (summary: string, executedActions: string[]) => {
    const entry: AssistantTransactionEntry = createAssistantTransactionEntry(
      summary,
      executedActions,
      buildSnapshot(),
    );
    setAssistantTransactions((current) => [entry, ...current].slice(0, 12));
    return entry.id;
  };

  const rollbackAssistantTransaction = (transactionId: string) => {
    const entry = assistantTransactions.find((transaction) => transaction.id === transactionId);
    if (!entry) return;
    restoreSnapshot(entry.snapshot);
    setAssistantTransactions((current) => current.filter((transaction) => transaction.id !== transactionId));
    setMessage(language === "zh" ? "已回滚上一轮助手事务。" : "Rolled back the last assistant transaction.");
  };

  const executeAssistantPlan = async (
    actions: Array<{ action: string; payload?: Record<string, unknown>; reason?: string }>,
    summary: string,
  ) => {
    const transactionId = recordAssistantTransaction(summary, actions.map((entry) => entry.action));
    try {
      for (const entry of actions) {
        await invokeScriptAction(entry.action, entry.payload ?? {});
      }
      setMessage(language === "zh" ? "助手计划已执行。" : "Assistant plan executed.");
      return transactionId;
    } catch (error) {
      rollbackAssistantTransaction(transactionId);
      throw error;
    }
  };

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

  const updateSelectedNode = (key: keyof Truss2dJobInput["nodes"][number], value: number | boolean) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => updateTruss2dNode(current, selectedNode, key, value));
  };

  const updateSelectedElement = (
    key: keyof Truss2dJobInput["elements"][number],
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => updateTruss2dElement(current, selectedElement, key, value));
  };

  const assignSelectedElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => assignTruss2dElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedTruss3dNode = (
    key: keyof Truss3dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => ({
      ...current,
      nodes: current.nodes.map((node, index) =>
        index === selectedNode ? { ...node, [key]: value } : node,
      ),
    }));
  };

  const updateSelectedTruss3dNodes = (
    key: keyof Truss3dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    const targetIndices = selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => updateTruss3dSelectedNodes(current, selectedTruss3dNodes, selectedNode, key, value));
  };

  const nudgeSelectedTruss3dNodes = (axis: "x" | "y" | "z", delta: number) => {
    const targetIndices = selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => nudgeTruss3dSelectedNodes(current, selectedTruss3dNodes, selectedNode, axis, delta, round));
  };

  const applySelectedTruss3dLoads = (mode: "apply" | "clear") => {
    const targetIndices = selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) =>
      applyTruss3dSelectedLoads(current, selectedTruss3dNodes, selectedNode, mode, {
        x: truss3dBatchLoadX,
        y: truss3dBatchLoadY,
        z: truss3dBatchLoadZ,
      }),
    );
  };

  const cloneSelectedTruss3dNodes = (mirrorAxis: "x" | "y" | "z" | null = null) => {
    const targetIndices = selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(t.addNodeAction);
    resetActiveResult(setResult, setJob);

    const nextState = cloneTruss3dSelectedNodes(truss3dModel, selectedTruss3dNodes, selectedNode, round, mirrorAxis);
    setTruss3dModel(nextState.model);

    if (nextState.nextSelection.length > 0) {
      setSelectedTruss3dNodes(nextState.nextSelection);
      setSelectedNode(nextState.nextSelection[0] ?? null);
      setMemberDraftNodes([]);
      setSelectedElement(null);
    }
  };

  const updateSelectedTruss3dElement = (
    key: keyof Truss3dJobInput["elements"][number],
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => updateTruss3dElement(current, selectedElement, key, value));
  };

  const assignSelectedTruss3dElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => assignTruss3dElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedPlaneNode = (
    key: keyof PlaneTriangle2dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setPlaneModel((current) => updatePlaneNode(current, selectedNode, key, value));
  };

  const updateSelectedPlaneElement = (
    key: keyof PlaneTriangle2dJobInput["elements"][number],
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setPlaneModel((current) => updatePlaneElement(current, selectedElement, key, value));
  };

  const assignSelectedPlaneElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setPlaneModel((current) => assignPlaneElementMaterial(current, selectedElement, materialId));
  };

  const addMaterialToCurrentModel = () => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);

    if (studyKind === "truss_2d") {
      setTrussModel((current) => addPresetMaterialToTrussModel(current, activeMaterial));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => addPresetMaterialToTruss3dModel(current, activeMaterial));
      return;
    }

    setPlaneModel((current) => addPresetMaterialToPlaneModel(current, activeMaterial));
  };

  const addCustomMaterialToCurrentModel = () => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(t.editMaterial);
    resetActiveResult(setResult, setJob);

    if (studyKind === "truss_2d") {
      setTrussModel((current) => addCustomMaterialToTrussModel(current));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => addCustomMaterialToTruss3dModel(current));
      return;
    }

    setPlaneModel((current) => addCustomMaterialToPlaneModel(current));
  };

  const applyMaterialToCurrentModel = (materialId: string, mode: "selected" | "all") => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);

    if (studyKind === "truss_2d") {
      setTrussModel((current) => applyMaterialToTrussModel(current, materialId, mode, selectedElement));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => applyMaterialToTruss3dModel(current, materialId, mode, selectedElement));
      return;
    }

    setPlaneModel((current) => applyMaterialToPlaneModel(current, materialId, mode, selectedElement));
  };

  const toggleMaterialVisibility = (materialId: string) => {
    setHiddenMaterials((current) => {
      const hidden = current[studyKind];
      const nextHidden = hidden.includes(materialId)
        ? hidden.filter((entry) => entry !== materialId)
        : [...hidden, materialId];
      return { ...current, [studyKind]: nextHidden };
    });
  };

  const importMaterials = async (file: File | undefined) => {
    if (!file || studyKind === "axial_bar_1d") return;

    try {
      const imported = parseMaterialLibrary(await file.text(), file.name);
      recordHistory(t.editMaterial);
      resetActiveResult(setResult, setJob);

      if (studyKind === "truss_2d") {
        setTrussModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else if (studyKind === "truss_3d") {
        setTruss3dModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else {
        setPlaneModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      }

      setMessage(language === "zh" ? "外部材料库已导入。" : "Imported external material library.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const updateCurrentMaterial = (
    materialId: string,
    field: "name" | "youngs_modulus" | "poisson_ratio",
    value: string | number,
  ) => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);

    if (studyKind === "truss_2d") {
      setTrussModel((current) => updateMaterialInTrussModel(current, materialId, field, value));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => updateMaterialInTruss3dModel(current, materialId, field, value));
      return;
    }

    setPlaneModel((current) => updateMaterialInPlaneModel(current, materialId, field, value));
  };

  const deleteCurrentMaterial = (materialId: string) => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(t.deleteMemberAction);
    resetActiveResult(setResult, setJob);

    if (studyKind === "truss_2d") {
      setTrussModel((current) => deleteMaterialFromTrussModel(current, materialId));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => deleteMaterialFromTruss3dModel(current, materialId));
      return;
    }

    setPlaneModel((current) => deleteMaterialFromPlaneModel(current, materialId));
  };

  const addNode = (connectToSelected: boolean) => {
    recordHistory(t.addNodeAction);
    setStudyKind("truss_2d");
    setSidebarSection("model");
    resetActiveResult(setResult, setJob);
    const nextState = addTruss2dNode(trussModel, connectToSelected, selectedNode, parametric, round);
    setTrussModel(nextState.model);
    setSelectedNode(nextState.nextSelectedNode);
    setSelectedElement(nextState.nextSelectedElement);
    setMemberDraftNodes([]);
    setMessage(nextState.createdBranch ? t.branchCreated : t.nodeCreated);
  };

  const addTruss3dNode = (connectToSelected: boolean) => {
    recordHistory(t.addNodeAction);
    setStudyKind("truss_3d");
    setSidebarSection("model");
    resetActiveResult(setResult, setJob);
    const nextState = addTruss3dNodeCommand(truss3dModel, connectToSelected, selectedNode, round);
    setTruss3dModel(nextState.model);
    setSelectedNode(nextState.nextSelectedNode);
    setSelectedTruss3dNodes([nextState.nextSelectedNode]);
    setSelectedElement(nextState.nextSelectedElement);
    setMemberDraftNodes([]);
    setMessage(nextState.createdBranch ? t.spaceBranchCreated : t.spaceNodeCreated);
  };

  const deleteSelectedNode = () => {
    if (selectedNode === null) return;
    recordHistory(t.deleteNodeAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => deleteTruss2dNode(current, selectedNode));
    setSelectedNode(null);
    setSelectedTruss3dNodes([]);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setMessage(t.nodeDeleted);
  };

  const deleteSelectedTruss3dNode = () => {
    if (selectedNode === null) return;
    recordHistory(t.deleteNodeAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => deleteTruss3dNodeCommand(current, selectedNode));
    setSelectedNode(null);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setMessage(t.spaceNodeDeleted);
  };

  const toggleDraftNode = (index: number) => {
    setSelectedNode(index);
    setSelectedElement(null);
    setMemberDraftNodes((current) => toggleDraftSelection(current, index));
  };

  const toggleTruss3dLinkMode = () => {
    setTruss3dLinkMode((current) => {
      const next = !current;
      setMemberDraftNodes([]);
      setMessage(next ? t.linkModeEnabled : t.linkModeDisabled);
      return next;
    });
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

  const completeTruss3dLink = (firstNode: number, secondNode: number) => {
    if (firstNode === secondNode) {
      setSelectedNode(firstNode);
      setSelectedTruss3dNodes([firstNode]);
      setMemberDraftNodes([firstNode]);
      return;
    }

    recordHistory(t.toggleMemberAction);

    resetActiveResult(setResult, setJob);
    const nextState = completeTruss3dLinkCommand(truss3dModel, firstNode, secondNode);
    if (nextState.repeatedNode) return;
    setTruss3dModel(nextState.model);
    setSelectedElement(nextState.removedExisting ? null : nextState.nextSelectedElement);
    setSelectedNode(secondNode);
    setSelectedTruss3dNodes([secondNode]);
    setMemberDraftNodes([secondNode]);
    setMessage(nextState.removedExisting ? t.memberRemoved : t.linkModeCompleted);
  };

  const handleTruss3dNodePick = (index: number) => {
    if (!truss3dLinkMode) {
      setSelectedTruss3dNodes([index]);
      toggleDraftNode(index);
      return;
    }

    setSelectedElement(null);
    setSelectedNode(index);
    setSelectedTruss3dNodes([index]);

    if (memberDraftNodes.length === 1) {
      completeTruss3dLink(memberDraftNodes[0], index);
      return;
    }

    setMemberDraftNodes([index]);
  };

  const handleTruss3dNodesBoxSelect = (indices: number[], append: boolean) => {
    const nextSelection = merge3dBoxSelection(selectedTruss3dNodes, indices, append);
    setSelectedTruss3dNodes(nextSelection);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setSelectedNode(nextSelection[0] ?? null);
  };

  const toggleMemberFromDraft = () => {
    if (memberDraftNodes.length !== 2) {
      setMessage(t.selectTwoNodes);
      return;
    }
    recordHistory(t.toggleMemberAction);

    resetActiveResult(setResult, setJob);
    const nextState = toggleTruss2dMember(trussModel, memberDraftNodes, parametric);
    if (!nextState.valid) return;
    setTrussModel(nextState.model);
    setSelectedElement(nextState.removedExisting ? null : nextState.nextSelectedElement);
    setSelectedTruss3dNodes([]);
    setMemberDraftNodes([]);
    setMessage(nextState.removedExisting ? t.memberRemoved : t.memberCreated);
  };

  const toggleTruss3dMemberFromDraft = () => {
    if (memberDraftNodes.length !== 2) {
      setMessage(t.selectTwoNodes);
      return;
    }
    const [nodeI, nodeJ] = memberDraftNodes;
    completeTruss3dLink(nodeI, nodeJ);
  };

  const deleteSelectedElement = () => {
    if (selectedElement === null) return;
    recordHistory(t.deleteMemberAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => deleteTruss2dElement(current, selectedElement));
    setSelectedElement(null);
    setMessage(t.memberDeleted);
  };

  const deleteSelectedTruss3dElement = () => {
    if (selectedElement === null) return;
    recordHistory(t.deleteMemberAction);
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => deleteTruss3dElementCommand(current, selectedElement));
    setSelectedElement(null);
    setSelectedTruss3dNodes([]);
    setMessage(t.spaceMemberDeleted);
  };

  const handleTrussPointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (draggingNode === null || studyKind !== "truss_2d") return;
    if (!dragHistoryCapturedRef.current) {
      recordHistory(t.dragNodeAction);
      dragHistoryCapturedRef.current = true;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const position = fromSvgPoint(event.clientX, event.clientY, rect, trussBounds);
    pendingDragPointRef.current = position;

    if (dragFrameRef.current !== null) {
      return;
    }

    dragFrameRef.current = window.requestAnimationFrame(() => {
      dragFrameRef.current = null;
      const nextPoint = pendingDragPointRef.current;
      if (!nextPoint) return;

      resetActiveResult(setResult, setJob);
      setTrussModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === draggingNode ? { ...node, x: nextPoint.x, y: nextPoint.y } : node,
        ),
      }));
    });
  };

  const stopDraggingNode = () => {
    setDraggingNode(null);
    dragHistoryCapturedRef.current = false;
    pendingDragPointRef.current = null;
    if (dragFrameRef.current !== null) {
      window.cancelAnimationFrame(dragFrameRef.current);
      dragFrameRef.current = null;
    }
  };

  const updateTruss3dNodePosition = (index: number, position: { x: number; y: number; z: number }) => {
    resetActiveResult(setResult, setJob);
    setTruss3dModel((current) => updateTruss3dNodePositionCommand(current, index, position, round));
  };

  const studyKindOptions = [
    { value: "axial_bar_1d" as const, label: t.kinds.axial_bar_1d },
    { value: "truss_2d" as const, label: t.kinds.truss_2d },
    { value: "truss_3d" as const, label: t.kinds.truss_3d },
    { value: "plane_triangle_2d" as const, label: t.kinds.plane_triangle_2d },
  ];

  const studySummaryRows = [
    { label: t.modelName, value: loadedModelName },
    { label: t.material, value: localMaterialLabel(activeMaterial, language) },
    {
      label: t.mesh,
      value: isAxial
        ? axialForm.elements
        : isTruss
          ? trussModel.elements.length
          : isTruss3d
            ? truss3dModel.elements.length
            : planeModel.elements.length,
    },
    {
      label: t.load,
      value: isAxial
        ? `${fixed(axialForm.tipForce, 0)} N`
        : isTruss
          ? `${fixed(trussModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`
          : isTruss3d
            ? `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + node.load_z, 0), 0)} N`
            : `${fixed(planeModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`,
    },
    {
      label: t.support,
      value: isAxial ? "Node 0" : isTruss3d ? "Fixed tripod" : "Pinned base",
    },
  ];

  const studyControlsRows = isAxial
    ? []
    : isTruss
      ? [
          { label: t.nodes, value: trussModel.nodes.length },
          { label: t.trussElements, value: trussModel.elements.length },
          { label: t.material, value: localMaterialLabel(activeMaterial, language) },
          { label: t.sourceModel, value: loadedModelName },
        ]
      : isTruss3d
        ? [
            { label: t.nodes, value: truss3dModel.nodes.length },
            { label: t.spatialTrussElements, value: truss3dModel.elements.length },
            { label: t.load, value: `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + node.load_z, 0), 0)} N` },
            { label: t.sourceModel, value: loadedModelName },
          ]
        : [
            { label: t.nodes, value: planeModel.nodes.length },
            { label: t.planeElements, value: planeModel.elements.length },
            { label: t.thickness, value: fixed(planeModel.elements[0]?.thickness, 3) },
            { label: t.sourceModel, value: loadedModelName },
          ];

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

  const modelToolsContent: ReactNode = (
    <>
      <section className="sidebar-card">
        <div className="card-head">
          <h2>{isTruss3d ? t.spaceStudio : t.sections.model}</h2>
          <span>{isTruss3d ? t.orbitHint : t.dragToEdit}</span>
        </div>
        <p className="card-copy">{isTruss3d ? t.spaceStudioHint : isPlane ? t.planeHint : t.modelStudioHint}</p>
        {isTruss ? (
          <>
            <div className="button-row">
              <button className="ghost-button" onClick={() => addNode(false)} type="button">
                {t.addNode}
              </button>
              <button className="ghost-button" disabled={selectedNode === null} onClick={() => addNode(true)} type="button">
                {t.addBranchNode}
              </button>
              <button className="ghost-button" disabled={selectedNode === null} onClick={deleteSelectedNode} type="button">
                {t.deleteNode}
              </button>
            </div>
            <div className="button-row">
              <button className="ghost-button" onClick={toggleMemberFromDraft} type="button">
                {t.toggleMember}
              </button>
              <button className="ghost-button" disabled={selectedElement === null} onClick={deleteSelectedElement} type="button">
                {t.deleteMember}
              </button>
            </div>
          </>
        ) : null}
        {isTruss3d ? (
          <>
            <div className="button-row">
              <button className="ghost-button" onClick={() => addTruss3dNode(false)} type="button">
                {t.addNode}
              </button>
              <button className="ghost-button" disabled={selectedNode === null} onClick={() => addTruss3dNode(true)} type="button">
                {t.addBranchNode}
              </button>
              <button className="ghost-button" disabled={selectedNode === null} onClick={deleteSelectedTruss3dNode} type="button">
                {t.deleteNode}
              </button>
            </div>
            <div className="button-row">
              <button className={`ghost-button${truss3dLinkMode ? " ghost-button--active" : ""}`} onClick={toggleTruss3dLinkMode} type="button">
                {truss3dLinkMode ? t.linkModeActive : t.linkMode}
              </button>
              <button className="ghost-button" onClick={toggleTruss3dMemberFromDraft} type="button">
                {t.toggleMember}
              </button>
              <button className="ghost-button" disabled={selectedElement === null} onClick={deleteSelectedTruss3dElement} type="button">
                {t.deleteMember}
              </button>
            </div>
            <p className="card-copy">{t.linkModeIdle}</p>
          </>
        ) : null}
        <div className="button-row">
          <button className="ghost-button" disabled={undoStack.length === 0} onClick={handleUndo} type="button">
            {t.undo}
          </button>
          <button className="ghost-button" disabled={redoStack.length === 0} onClick={handleRedo} type="button">
            {t.redo}
          </button>
        </div>
        <div className="button-row">
          <button className="ghost-button" onClick={downloadModel} type="button">
            {t.download}
          </button>
          <button
            className="ghost-button"
            onClick={() => {
              setStudyKind(isPlane ? "plane_triangle_2d" : isTruss3d ? "truss_3d" : "truss_2d");
              setSidebarSection("study");
              setMessage(isPlane ? t.planeHint : isTruss3d ? t.switchedTo3dStudio : t.switchedTo2dStudio);
            }}
            type="button"
          >
            {t.saveForSolver}
          </button>
        </div>
        <p className="card-copy">{t.selectionHint}</p>
      </section>

      {!isAxial ? (
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
      ) : null}

      {!isTruss3d ? (
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
      ) : null}
    </>
  );

  const modelTreeContent: ReactNode = isTruss3d ? (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{t.objectTree}</h2>
        <span>{selectedTruss3dNodes.length > 1 ? `${selectedTruss3dNodes.length} ${t.nodes}` : truss3dLinkMode ? t.linkModeActive : `${memberDraftNodes.length}/2`}</span>
      </div>
      <p className="card-copy">{truss3dLinkMode ? t.linkModeIdle : t.orbitHint}</p>
      <div className="table-like">
        <div className="table-like__head table-like__head--space">
          <span>ID</span>
          <span>X</span>
          <span>Y</span>
          <span>Z</span>
        </div>
        <VirtualList
          className="table-like__body"
          items={truss3dModel.nodes}
          itemHeight={46}
          maxHeight={240}
          itemKey={(node) => node.id}
          renderItem={(node, index) => (
            <button
              className={`table-like__row table-like__row--space${selectedTruss3dNodes.includes(index) || selectedNode === index ? " table-like__row--active" : ""}${memberDraftNodes.includes(index) ? " table-like__row--draft" : ""}`}
              onClick={() => handleTruss3dNodePick(index)}
              type="button"
            >
              <strong>{node.id}</strong>
              <span>{fixed(node.x, 2)}</span>
              <span>{fixed(node.y, 2)}</span>
              <span>{fixed(node.z, 2)}</span>
            </button>
          )}
        />
      </div>
      <div className="table-like model-tree-spacer">
        <div className="table-like__head">
          <span>ID</span>
          <span>{t.nodeI}</span>
          <span>{t.nodeJ}</span>
          <span>{t.area}</span>
        </div>
        <VirtualList
          className="table-like__body"
          items={displayTruss3dElements}
          itemHeight={44}
          maxHeight={240}
          itemKey={(element) => element.id}
          renderItem={(element, index) => (
            <button
              className={`table-like__row${selectedElement === index ? " table-like__row--active" : ""}`}
              onClick={() => {
                setSelectedElement(index);
                setSelectedNode(null);
                setSelectedTruss3dNodes([]);
                setMemberDraftNodes([]);
              }}
              type="button"
            >
              <strong>{element.id}</strong>
              <span>{element.node_i}</span>
              <span>{element.node_j}</span>
              <span>{fixed(truss3dModel.elements[index]?.area, 4)}</span>
            </button>
          )}
        />
      </div>
    </section>
  ) : (
    <WorkbenchObjectTree
      title={t.objectTree}
      countLabel={isTruss ? `${memberDraftNodes.length}/2` : String(planeModel.elements.length)}
      hint={isPlane ? t.planeHint : t.dragHint}
      diagnosticsLabel={t.diagnostics}
      loadCaseLabel={t.loadCase}
      nodeJLabel={t.nodeJ}
      nodeKLabel={t.nodeK}
      nodeRows={(isPlane ? planeModel.nodes : trussModel.nodes).map((node) => ({
        id: node.id,
        x: node.x,
        y: node.y,
        load_y: node.load_y,
      }))}
      elementRows={(isPlane ? planeElements : displayTrussElements).map((element) => ({
        id: element.id,
        node_i: element.node_i,
        node_j: "node_j" in element ? element.node_j : undefined,
        node_k: "node_k" in element ? element.node_k : undefined,
      }))}
      isPlane={isPlane}
      isTruss={isTruss}
      selectedNode={selectedNode}
      selectedElement={selectedElement}
      nodeIssueCounts={Object.fromEntries(
        Object.entries(trussDiagnostics?.nodeIssues ?? {}).map(([key, issues]) => [Number(key), issues.length]),
      )}
      onSelectNode={(index) => {
        if (isPlane) {
          setSelectedNode(index);
          setSelectedElement(null);
        } else {
          toggleDraftNode(index);
        }
      }}
      onSelectElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        if (isTruss) setMemberDraftNodes([]);
      }}
    />
  );

  return (
    <div className="workbench-shell">
      <aside className="app-rail panel">
        <div className="rail-brand">
          <strong>{t.brand}</strong>
          <span>v0.3</span>
        </div>
        <div className="rail-nav">
          {railItems.map((item) => (
            <button
              key={item.key}
              className={`rail-button${sidebarSection === item.key ? " rail-button--active" : ""}`}
              onClick={() => setSidebarSection(item.key)}
              type="button"
            >
              <span>{item.symbol}</span>
              <small>{item.label}</small>
            </button>
          ))}
        </div>
      </aside>

      <aside className="workspace-sidebar panel">
        <div className="sidebar-header">
          <p className="eyebrow">{t.brand}</p>
          <h1>{t.title}</h1>
          <p>{t.subtitle}</p>
        </div>

        {sidebarSection === "study" ? (
          <WorkbenchStudySidebar
            studyTab={studyTab}
            onStudyTabChange={setStudyTab}
            sectionTitle={t.sections.study}
            summaryTabLabel={t.tabs.summary}
            controlsTabLabel={t.tabs.controls}
            loadedModelName={loadedModelName}
            studyTypeLabel={language === "zh" ? "研究类型" : "Study Type"}
            studyKind={studyKind}
            studyKindOptions={studyKindOptions}
            onStudyKindChange={(nextStudyKind) => {
              recordHistory(t.changeStudyType);
              setStudyKind(nextStudyKind);
            }}
            summaryRows={studySummaryRows}
            controlsRows={studyControlsRows}
            controlsContent={studyControlsContent}
            controlsTitle={t.controls}
            readyLabel={t.ready}
            busyLabel={t.busy}
            isPending={isPending}
            runLabel={t.run}
            runningLabel={t.running}
            onRun={runAnalysis}
          />
        ) : null}

        {sidebarSection === "model" ? (
          <WorkbenchModelSidebar
            modelTab={modelTab}
            onModelTabChange={setModelTab}
            isTruss3d={isTruss3d}
            toolsTabLabel={t.tabs.tools}
            treeTabLabel={t.tabs.tree}
            toolsContent={modelToolsContent}
            treeContent={modelTreeContent}
          />
        ) : null}

        {sidebarSection === "library" ? (
          <WorkbenchLibrarySidebar
            libraryTab={libraryTab}
            onLibraryTabChange={setLibraryTab}
            labels={t}
            samples={SAMPLE_LIBRARY}
            projects={projects}
            selectedProjectId={selectedProjectId}
            onSelectedProjectChange={(projectId) => {
              setSelectedProjectId(projectId);
              setSelectedModelId(null);
            }}
            projectNameDraft={projectNameDraft}
            onProjectNameDraftChange={setProjectNameDraft}
            projectDescriptionDraft={projectDescriptionDraft}
            onProjectDescriptionDraftChange={setProjectDescriptionDraft}
            onCreateProject={createProjectRecord}
            onUpdateProject={updateProjectRecord}
            onDeleteProject={deleteProjectRecord}
            onExportProjectJson={() => void downloadProjectBundleJson()}
            onExportProjectZip={() => void downloadProjectBundleZip()}
            onImportProjectBundle={(file) => void importProjectBundle(file)}
            selectedProjectModels={selectedProjectModels}
            deferredProjectModels={deferredProjectModels}
            selectedModelId={selectedModelId}
            loadedModelName={loadedModelName}
            onLoadedModelNameChange={setLoadedModelName}
            onSaveModel={saveModelVersion}
            onDeleteSavedModel={deleteSavedModelRecord}
            onOpenSavedModel={openSavedModel}
            deferredModelVersions={deferredModelVersions}
            modelVersions={modelVersions}
            selectedVersionId={selectedVersionId}
            onRenameSelectedVersion={renameSelectedVersion}
            onDeleteSelectedVersion={deleteSelectedVersion}
            onOpenSavedVersion={openSavedVersion}
            deferredJobHistory={deferredJobHistory}
            jobHistory={jobHistory}
            activeJobId={job?.job_id ?? null}
            onOpenHistoryJob={openHistoryJob}
            onOpenSample={openSample}
            onRefresh={() => {
              void refreshJobHistory();
              void refreshProjects();
            }}
            onImportModel={importModel}
            formatTime={(value) => formatTime(value, language)}
          />
        ) : null}

        {sidebarSection === "system" ? (
          <div className="sidebar-stack panel-scroll-window">
            <div className="panel-tabs panel-tabs--editor">
              <button
                className={`panel-tab${systemPanelTab === "config" ? " panel-tab--active" : ""}`}
                onClick={() => setSystemPanelTab("config")}
                type="button"
              >
                {language === "zh" ? "配置" : "Config"}
              </button>
              <button
                className={`panel-tab${systemPanelTab === "assistant" ? " panel-tab--active" : ""}`}
                onClick={() => setSystemPanelTab("assistant")}
                type="button"
              >
                {t.assistant}
              </button>
              <button
                className={`panel-tab${systemPanelTab === "scripts" ? " panel-tab--active" : ""}`}
                onClick={() => setSystemPanelTab("scripts")}
                type="button"
              >
                {t.scripts}
              </button>
              <button
                className={`panel-tab${systemPanelTab === "runtime" ? " panel-tab--active" : ""}`}
                onClick={() => setSystemPanelTab("runtime")}
                type="button"
              >
                {language === "zh" ? "运行时" : "Runtime"}
              </button>
              <button
                className={`panel-tab${systemPanelTab === "data" ? " panel-tab--active" : ""}`}
                onClick={() => setSystemPanelTab("data")}
                type="button"
              >
                {language === "zh" ? "数据" : "Data"}
              </button>
            </div>
            {systemPanelTab === "config" ? (
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{t.settings}</h2>
                <span>{health?.status === "ok" ? t.online : t.offline}</span>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{t.theme}</span>
                  <select value={theme} onChange={(event) => setTheme(event.target.value as Theme)}>
                    <option value="linen">{t.themes.linen}</option>
                    <option value="marine">{t.themes.marine}</option>
                    <option value="graphite">{t.themes.graphite}</option>
                  </select>
                </label>
                <label>
                  <span>{t.language}</span>
                  <select value={language} onChange={(event) => handleLanguageChange(event.target.value as Language)}>
                    <option value="en">{t.languages.en}</option>
                    <option value="zh">{t.languages.zh}</option>
                  </select>
                </label>
                <label>
                  <span>{t.frontendMode}</span>
                  <select
                    value={frontendRuntimeMode}
                    onChange={(event) => setFrontendRuntimeMode(event.target.value as FrontendRuntimeMode)}
                  >
                    <option value="orchestrated_gui">{t.frontendModes.orchestrated_gui}</option>
                    <option value="direct_mesh_gui">{t.frontendModes.direct_mesh_gui}</option>
                  </select>
                </label>
                <label>
                  <span>{t.directMeshStrategy}</span>
                  <select
                    value={directMeshSelectionMode}
                    onChange={(event) => setDirectMeshSelectionMode(event.target.value as DirectMeshSelectionMode)}
                  >
                    <option value="healthiest">{t.directMeshStrategies.healthiest}</option>
                    <option value="first_reachable">{t.directMeshStrategies.first_reachable}</option>
                  </select>
                </label>
                <label className="field-span-2">
                  <span>{t.directMeshEndpoints}</span>
                  <small className="field-hint">{t.directMeshEndpointsHelp}</small>
                  <textarea
                    rows={3}
                    value={directMeshEndpointsText}
                    onChange={(event) => setDirectMeshEndpointsText(event.target.value)}
                  />
                </label>
                <label className="field-span-2">
                  <span>{securityUi.controlPlaneToken}</span>
                  <small className="field-hint">
                    {language === "zh"
                      ? "用于带 Token 的 control-plane 部署；会作为 x-kyuubiki-token 附加到 /api/v1 请求。"
                      : "Used for token-protected control-plane deployments; sent as x-kyuubiki-token to /api/v1 requests."}
                  </small>
                  <input
                    type="password"
                    value={controlPlaneApiToken}
                    onChange={(event) => setControlPlaneApiToken(event.target.value)}
                    placeholder={language === "zh" ? "可选的控制面令牌" : "Optional control-plane token"}
                  />
                </label>
                <label className="field-span-2">
                  <span>{securityUi.clusterToken}</span>
                  <small className="field-hint">
                    {language === "zh"
                      ? "用于 agent 注册、心跳和摘除这类集群路由；未填写时会回退到控制面令牌。"
                      : "Used for agent register/heartbeat/remove cluster routes; falls back to the control-plane token when empty."}
                  </small>
                  <input
                    type="password"
                    value={clusterApiToken}
                    onChange={(event) => setClusterApiToken(event.target.value)}
                    placeholder={language === "zh" ? "可选的集群专用令牌" : "Optional cluster-only token"}
                  />
                </label>
                <label className="field-span-2">
                  <span>{securityUi.directMeshToken}</span>
                  <small className="field-hint">
                    {language === "zh"
                      ? "用于带 Token 的 direct mesh 路由；会附加到 /api/direct-mesh 请求。"
                      : "Used for token-protected direct mesh routes; sent to /api/direct-mesh requests."}
                  </small>
                  <input
                    type="password"
                    value={directMeshApiToken}
                    onChange={(event) => setDirectMeshApiToken(event.target.value)}
                    placeholder={language === "zh" ? "可选的直连网格令牌" : "Optional direct-mesh token"}
                  />
                </label>
                <label className="toggle-row">
                  <div>
                    <span>{t.shortcutHints}</span>
                    <small className="field-hint">{t.shortcutHintsHelp}</small>
                  </div>
                  <input
                    type="checkbox"
                    checked={showShortcutHints}
                    onChange={(event) => setShowShortcutHints(event.target.checked)}
                  />
                </label>
                <label className="toggle-row">
                  <div>
                    <span>{t.immersiveGuard}</span>
                    <small className="field-hint">{t.immersiveGuardHelp}</small>
                  </div>
                  <input
                    type="checkbox"
                    checked={immersiveGuardrails}
                    onChange={(event) => setImmersiveGuardrails(event.target.checked)}
                  />
                </label>
              </div>
              <p className="card-copy">{t.browserLimitsNote}</p>
              <div className="button-row">
                <button className="ghost-button" onClick={() => void downloadDatabaseSnapshot()} type="button">
                  {language === "zh" ? "导出数据库快照" : "Export database snapshot"}
                </button>
              </div>
            </section>
            ) : null}
            {systemPanelTab === "assistant" ? (
              <WorkbenchAssistantPanel
                currentJobLabel={job?.status ?? t.none}
                currentResultLabel={hasAnyResult ? t.yes : t.no}
                currentRuntimeLabel={t.frontendModes[frontendRuntimeMode]}
                currentStudyLabel={t.kinds[studyKind]}
                language={language}
                llmApiKey={assistantApiKey}
                llmBaseUrl={assistantApiBaseUrl}
                llmModel={assistantModel}
                localCards={assistantCards}
                mode={assistantMode}
                onExecuteLlmAction={async (action, payload, reason) => {
                  await executeAssistantPlan([{ action, payload, reason }], reason ?? action);
                }}
                onExecuteLlmPlan={async (actions, summary) => {
                  await executeAssistantPlan(actions, summary);
                }}
                onLlmApiKeyChange={setAssistantApiKey}
                onLlmBaseUrlChange={setAssistantApiBaseUrl}
                onLlmModelChange={setAssistantModel}
                onModeChange={setAssistantMode}
                onRequestPlan={requestLlmAssistantPlan}
                onRollbackTransaction={rollbackAssistantTransaction}
                transactions={assistantTransactions.map((entry) => ({
                  id: entry.id,
                  summary: entry.summary,
                  createdAt: entry.createdAt,
                  executedActions: entry.executedActions,
                }))}
              />
            ) : null}
            {systemPanelTab === "scripts" ? (
              <WorkbenchScriptPanel
                actionLog={scriptActionLog}
                getSnapshot={getScriptSnapshot}
                language={language}
                onInvokeAction={invokeScriptAction}
                snapshot={scriptSnapshot}
              />
            ) : null}
            {systemPanelTab === "runtime" ? (
            <>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{t.backend}</h2>
                <span>{health?.status ?? t.offline}</span>
              </div>
              <div className="sidebar-list">
                <div>
                  <span>{t.ui}</span>
                  <strong>3000</strong>
                </div>
                <div>
                  <span>{t.orchestrator}</span>
                  <strong>{health ? "4000" : t.offline}</strong>
                </div>
                <div>
                  <span>{t.solverAgent}</span>
                  <strong>{health?.transport?.solver_agent_tcp ?? 5001}</strong>
                </div>
              </div>
            </section>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{t.protocols}</h2>
                <span>{health?.protocol ? t.online : t.offline}</span>
              </div>
              <div className="sidebar-list">
                <div>
                  <span>{t.controlPlaneProtocol}</span>
                  <strong>{health?.protocol?.protocol?.name ?? "--"}</strong>
                </div>
                <div>
                  <span>{t.solverRpcProtocol}</span>
                  <strong>{health?.protocol?.compatible_solver_rpc?.name ?? "--"}</strong>
                </div>
                <div>
                  <span>{t.deploymentMode}</span>
                  <strong>{health?.deployment?.mode ?? "--"}</strong>
                </div>
                <div>
                  <span>{t.discoveryMode}</span>
                  <strong>{health?.deployment?.discovery ?? "--"}</strong>
                </div>
                <div>
                  <span>{t.registeredAgents}</span>
                  <strong>{health?.remote_solver_registry?.active_agents ?? 0}</strong>
                </div>
                <div>
                  <span>{t.reachableAgents}</span>
                  <strong>{protocolAgents.length}</strong>
                </div>
                {frontendRuntimeMode === "direct_mesh_gui" ? (
                  <>
                    <div>
                      <span>{t.directMeshStrategy}</span>
                      <strong>{t.directMeshStrategies[directMeshSelectionMode]}</strong>
                    </div>
                    <div>
                      <span>{t.directMeshLastAgent}</span>
                      <strong>{directMeshExecution?.endpoint ?? "--"}</strong>
                    </div>
                    <div>
                      <span>{t.directMeshLastRoute}</span>
                      <strong>
                        {directMeshExecution
                          ? `${t.directMeshStrategies[directMeshExecution.strategy]} · ${formatTime(directMeshExecution.at, language)}`
                          : "--"}
                      </strong>
                    </div>
                  </>
                ) : null}
              </div>
              {health?.protocol?.compatible_solver_rpc?.methods?.length ? (
                <div className="protocol-chip-row">
                  {health.protocol.compatible_solver_rpc.methods.map((method) => (
                    <span className="protocol-chip" key={method}>
                      {formatProtocolMethodLabel(method)}
                    </span>
                  ))}
                </div>
              ) : null}
            </section>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{securityUi.security}</h2>
                <span>
                  {health?.security?.api_token_configured ? securityUi.configured : securityUi.notConfigured}
                </span>
              </div>
              <div className="sidebar-list">
                <div>
                  <span>{securityUi.controlPlaneToken}</span>
                  <strong>
                    {health?.security?.api_token_configured ? securityUi.configured : securityUi.notConfigured}
                  </strong>
                </div>
                <div>
                  <span>{securityUi.clusterToken}</span>
                  <strong>
                    {health?.security?.cluster_token_configured ? securityUi.configured : securityUi.notConfigured}
                  </strong>
                </div>
                <div>
                  <span>{securityUi.clusterWindow}</span>
                  <strong>{health?.security?.cluster_timestamp_window_ms ?? 30000} ms</strong>
                </div>
                <div>
                  <span>{language === "zh" ? "Agent 白名单" : "Agent allowlist"}</span>
                  <strong>
                    {health?.security?.cluster_agent_allowlist_enabled
                      ? `${securityUi.enabled} · ${health?.security?.cluster_agent_allowlist_count ?? 0}`
                      : securityUi.disabled}
                  </strong>
                </div>
                <div>
                  <span>{language === "zh" ? "Cluster 白名单" : "Cluster allowlist"}</span>
                  <strong>
                    {health?.security?.cluster_cluster_allowlist_enabled
                      ? `${securityUi.enabled} · ${health?.security?.cluster_cluster_allowlist_count ?? 0}`
                      : securityUi.disabled}
                  </strong>
                </div>
                <div>
                  <span>{language === "zh" ? "Fingerprint 绑定" : "Fingerprint binding"}</span>
                  <strong>
                    {health?.security?.cluster_fingerprint_required ? securityUi.enabled : securityUi.disabled}
                  </strong>
                </div>
                <div>
                  <span>{securityUi.protectReads}</span>
                  <strong>{health?.security?.protect_reads ? securityUi.enabled : securityUi.disabled}</strong>
                </div>
                <div>
                  <span>{securityUi.mutatingRoutes}</span>
                  <strong>
                    {health?.security?.mutating_routes_protected ? securityUi.enabled : securityUi.disabled}
                  </strong>
                </div>
                <div>
                  <span>{securityUi.clusterRoutes}</span>
                  <strong>
                    {health?.security?.cluster_routes_protected ? securityUi.enabled : securityUi.disabled}
                  </strong>
                </div>
                <div>
                  <span>{securityUi.directMeshRoutes}</span>
                  <strong>
                    {directMeshApiToken ? securityUi.configured : securityUi.enabled}
                  </strong>
                </div>
              </div>
              <p className="card-copy">
                {language === "zh"
                  ? "运行中的安全状态来自 /api/health；前端输入的 token 只保存在当前浏览器设置里。"
                  : "Runtime security state comes from /api/health; frontend tokens stay only in local browser settings."}
              </p>
            </section>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{t.protocolAgents}</h2>
                <span>{protocolAgents.length}</span>
              </div>
              {protocolAgents.length === 0 ? (
                <p className="card-copy">{t.noProtocolAgents}</p>
              ) : (
                <div className="protocol-agent-list">
                  {protocolAgents.slice(0, 4).map((agent) => (
                    <article className="protocol-agent-card" key={agent.id}>
                      <div className="protocol-agent-card__head">
                        <strong>{agent.id}</strong>
                        <span>
                          {agent.host}:{agent.port}
                        </span>
                      </div>
                      <div className="sidebar-list">
                        <div>
                          <span>{t.runtimeMode}</span>
                          <strong>{agent.descriptor?.runtime?.runtime_mode ?? "--"}</strong>
                        </div>
                        <div>
                          <span>{t.cluster}</span>
                          <strong>{agent.descriptor?.runtime?.cluster_id ?? "--"}</strong>
                        </div>
                        <div>
                          <span>{t.clusterSize}</span>
                          <strong>{agent.descriptor?.runtime?.cluster_size ?? 1}</strong>
                        </div>
                        <div>
                          <span>{t.clusterHealth}</span>
                          <strong>
                            <span
                              className={`status-chip status-chip--${clusterHealthTone(agent.descriptor?.runtime?.health_score)}`}
                            >
                              {agent.descriptor?.runtime?.health_score ?? "--"}
                            </span>
                          </strong>
                        </div>
                        <div>
                          <span>{t.peers}</span>
                          <strong>{agent.descriptor?.runtime?.peers?.length ?? 0}</strong>
                        </div>
                        <div>
                          <span>{t.headless}</span>
                          <strong>{agent.descriptor?.runtime?.headless ? t.yes : t.no}</strong>
                        </div>
                        <div>
                          <span>{t.capabilities}</span>
                          <strong>{agent.descriptor?.capabilities?.length ?? 0}</strong>
                        </div>
                        <div>
                          <span>{t.methods}</span>
                          <strong>{agent.descriptor?.protocol?.methods?.length ?? 0}</strong>
                        </div>
                      </div>
                      {agent.descriptor?.capabilities?.length || agent.descriptor?.runtime?.peers?.length ? (
                        <div className="protocol-chip-row">
                          {agent.descriptor?.capabilities?.flatMap((capability) =>
                            capability.tags.slice(0, 3).map((tag) => (
                              <span className="protocol-chip" key={`${agent.id}-${capability.id}-${tag}`}>
                                {tag}
                              </span>
                            )),
                          ) ?? null}
                          {agent.descriptor?.runtime?.peers?.slice(0, 2).map((peer) => (
                            <span
                              className={`protocol-chip protocol-chip--${clusterHealthTone(
                                peer.status === "healthy"
                                  ? 100
                                  : peer.status === "degraded"
                                    ? 65
                                    : peer.status === "seed"
                                      ? 85
                                      : 25,
                              )}`}
                              key={`${agent.id}-${peer.address}`}
                              title={`${t.peerState}: ${formatPeerStatus(peer.status, t)}`}
                            >
                              {peer.address}
                            </span>
                          )) ?? null}
                        </div>
                      ) : agent.descriptor_error ? (
                        <p className="card-copy">{agent.descriptor_error}</p>
                      ) : null}
                    </article>
                  ))}
                </div>
              )}
            </section>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{t.watchdog}</h2>
                <span>{health?.watchdog ? t.online : t.offline}</span>
              </div>
              <div className="sidebar-list">
                <div>
                  <span>{t.activeJobs}</span>
                  <strong>{health?.watchdog?.active_jobs ?? 0}</strong>
                </div>
                <div>
                  <span>{t.stalledJobs}</span>
                  <strong>{health?.watchdog?.stalled_jobs ?? 0}</strong>
                </div>
                <div>
                  <span>{t.timedOutJobs}</span>
                  <strong>{health?.watchdog?.timed_out_jobs ?? 0}</strong>
                </div>
                <div>
                  <span>{t.scanEvery}</span>
                  <strong>{formatMilliseconds(health?.watchdog?.scan_interval_ms)}</strong>
                </div>
                <div>
                  <span>{t.staleAfter}</span>
                  <strong>{formatMilliseconds(health?.watchdog?.stale_job_ms)}</strong>
                </div>
                <div>
                  <span>{t.timeoutAfter}</span>
                  <strong>{formatMilliseconds(health?.watchdog?.job_timeout_ms)}</strong>
                </div>
              </div>
            </section>
            </>
            ) : null}
            {systemPanelTab === "data" ? (
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{t.dataAdmin}</h2>
                <span>
                  {t.databaseRecordCount}: {jobHistory.length + resultRecords.length}
                </span>
              </div>
              <div className="panel-tabs">
                <button
                  className={`panel-tab${systemDataTab === "jobs" ? " panel-tab--active" : ""}`}
                  onClick={() => setSystemDataTab("jobs")}
                  type="button"
                >
                  {t.adminJobs}
                </button>
                <button
                  className={`panel-tab${systemDataTab === "results" ? " panel-tab--active" : ""}`}
                  onClick={() => setSystemDataTab("results")}
                  type="button"
                >
                  {t.adminResults}
                </button>
              </div>
              {systemDataTab === "jobs" ? (
                <>
                  <VirtualList
                    className="history-list"
                    items={deferredJobHistory}
                    itemHeight={112}
                    maxHeight={220}
                    emptyState={<p className="card-copy">{t.historyEmpty}</p>}
                    itemKey={(historyJob) => historyJob.job_id}
                    renderItem={(historyJob) => (
                      <button
                        className={`history-item${selectedAdminJobId === historyJob.job_id ? " history-item--active" : ""}`}
                        onClick={() => setSelectedAdminJobId(historyJob.job_id)}
                        type="button"
                      >
                        <strong>{historyJob.job_id.slice(0, 8)}</strong>
                        <span>{historyJob.status}</span>
                        <small>{historyJob.project_id}</small>
                        <small>
                          <span className={`heartbeat-badge heartbeat-badge--${heartbeatTone(historyJob as JobEnvelope["job"])}`}>
                            {heartbeatStatus(historyJob as JobEnvelope["job"], t)}
                          </span>
                        </small>
                        <small>{humanizeSolverFailure(historyJob.message, t) ?? historyJob.message ?? historyJob.worker_id ?? "--"}</small>
                      </button>
                    )}
                  />
                  {selectedAdminJob ? (
                    <>
                      <div className="button-row">
                        <button className="ghost-button" disabled={selectedAdminJob.status === "completed" || selectedAdminJob.status === "failed" || selectedAdminJob.status === "cancelled"} onClick={selectedAdminJob.job_id === job?.job_id ? cancelCurrentJob : async () => {
                          try {
                            await cancelJob(selectedAdminJob.job_id);
                            setMessage(t.jobCancelled);
                            await refreshJobHistory();
                          } catch (error) {
                            setMessage(error instanceof Error ? error.message : t.initialFailed);
                          }
                        }} type="button">
                          {t.cancelJob}
                        </button>
                      </div>
                      <div className="form-grid compact">
                        <label>
                          <span>{t.adminMessage}</span>
                          <input value={adminJobMessage} onChange={(event) => setAdminJobMessage(event.target.value)} />
                        </label>
                        <label>
                          <span>{t.adminProjectId}</span>
                          <input value={adminJobProjectId} onChange={(event) => setAdminJobProjectId(event.target.value)} />
                        </label>
                        <label>
                          <span>{t.adminModelVersionId}</span>
                          <input value={adminJobModelVersionId} onChange={(event) => setAdminJobModelVersionId(event.target.value)} />
                        </label>
                        <label>
                          <span>{t.adminCaseId}</span>
                          <input value={adminJobCaseId} onChange={(event) => setAdminJobCaseId(event.target.value)} />
                        </label>
                      </div>
                      <div className="button-row">
                        <button className="ghost-button" onClick={saveAdminJobRecord} type="button">
                          {t.saveRecord}
                        </button>
                        <button className="ghost-button" onClick={deleteAdminJobRecord} type="button">
                          {t.deleteRecord}
                        </button>
                      </div>
                    </>
                  ) : (
                    <p className="card-copy">{t.selectRecord}</p>
                  )}
                </>
              ) : (
                <>
                  <VirtualList
                    className="history-list"
                    items={deferredResultRecords}
                    itemHeight={88}
                    maxHeight={220}
                    emptyState={<p className="card-copy">{t.historyEmpty}</p>}
                    itemKey={(entry) => entry.job_id}
                    renderItem={(entry) => (
                      <button
                        className={`history-item${selectedAdminResultJobId === entry.job_id ? " history-item--active" : ""}`}
                        onClick={() => setSelectedAdminResultJobId(entry.job_id)}
                        type="button"
                      >
                        <strong>{entry.job_id.slice(0, 8)}</strong>
                        <span>{entry.updated_at ? formatTime(entry.updated_at, language) : t.hasResult}</span>
                        <small>{Object.keys(entry.result).join(", ").slice(0, 64) || t.resultPayload}</small>
                      </button>
                    )}
                  />
                  {selectedAdminResult ? (
                    <>
                      <div className="form-grid compact">
                        <label>
                          <span>{t.resultPayload}</span>
                          <textarea
                            className="json-editor"
                            value={adminResultDraft}
                            onChange={(event) => setAdminResultDraft(event.target.value)}
                            rows={10}
                          />
                        </label>
                      </div>
                      <div className="button-row">
                        <button className="ghost-button" onClick={saveAdminResultRecord} type="button">
                          {t.saveRecord}
                        </button>
                        <button className="ghost-button" onClick={exportAdminResultRecord} type="button">
                          {t.exportRecord}
                        </button>
                        <button className="ghost-button" onClick={deleteAdminResultRecord} type="button">
                          {t.deleteRecord}
                        </button>
                      </div>
                    </>
                  ) : (
                    <p className="card-copy">{t.selectRecord}</p>
                  )}
                </>
              )}
            </section>
            ) : null}
          </div>
        ) : null}
      </aside>

      <main className="workspace-main">
        <section ref={viewportPanelRef} className={`panel canvas-panel${immersiveViewport ? " canvas-panel--immersive" : ""}`}>
          <div className="panel-head">
            <h2>{sidebarSection === "model" ? t.sections.model : t.viewport}</h2>
            <div className="panel-head__actions">
              {isTruss3d && immersiveViewport ? (
                <div className="immersive-switches">
                  <button
                    className={`ghost-button ghost-button--compact${sidebarSection === "study" ? " ghost-button--active" : ""}`}
                    onClick={() => setSidebarSection("study")}
                    type="button"
                  >
                    {t.immersiveStudy}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${sidebarSection === "model" ? " ghost-button--active" : ""}`}
                    onClick={() => setSidebarSection("model")}
                    type="button"
                  >
                    {t.immersiveModel}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${sidebarSection === "library" ? " ghost-button--active" : ""}`}
                    onClick={() => setSidebarSection(sidebarSection === "library" ? "model" : "library")}
                    type="button"
                  >
                    {t.immersiveLibrary}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={() => setImmersiveToolDrawerOpen((current) => !current)}
                    type="button"
                  >
                    {t.immersiveTools}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${immersiveHelpDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={() => setImmersiveHelpDrawerOpen((current) => !current)}
                    type="button"
                  >
                    {t.immersiveHelp}
                  </button>
                </div>
              ) : null}
              {isTruss3d && !immersiveViewport ? (
                <div className="immersive-switches">
                  <button
                    className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={() => setImmersiveToolDrawerOpen((current) => !current)}
                    type="button"
                  >
                    {t.immersiveTools}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${immersiveHelpDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={() => setImmersiveHelpDrawerOpen((current) => !current)}
                    type="button"
                  >
                    {t.immersiveHelp}
                  </button>
                </div>
              ) : null}
              {isTruss3d ? (
                <button className={`ghost-button ghost-button--compact${immersiveViewport ? " ghost-button--active" : ""}`} onClick={toggleImmersiveViewport} type="button">
                  {immersiveViewport ? t.exitImmersive : t.enterImmersive}
                </button>
              ) : null}
              <span>{job?.status ?? "idle"}</span>
            </div>
          </div>
          <div className={`canvas-layout${hasViewportDock ? " canvas-layout--split" : ""}`}>
          {hasViewportDock ? (
            <div className="viewport-dock">
              {immersiveViewport && immersiveToolDrawerOpen ? (
                <section className="viewport-dock__card">
                  <div className="card-head">
                    <h2>{t.immersiveTools}</h2>
                    <span>{t.kinds.truss_3d}</span>
                  </div>
                  {showViewportToolStrip ? (
                    <div className="viewport-toolbar-strip viewport-toolbar-strip--dock" role="toolbar" aria-label={t.immersiveViewTools}>
                      {(["iso", "front", "right", "top"] as const).map((preset) => (
                        <button
                          key={preset}
                          className={`ghost-button ghost-button--compact${truss3dViewPreset === preset ? " ghost-button--active" : ""}`}
                          onClick={() => setTruss3dViewPreset(preset)}
                          type="button"
                        >
                          {preset === "iso" ? "ISO" : preset === "front" ? "FR" : preset === "right" ? "RT" : "TP"}
                        </button>
                      ))}
                      <button
                        className={`ghost-button ghost-button--compact${selectedNode !== null || selectedTruss3dNodes.length > 0 ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dFocusRequestVersion((current) => current + 1)}
                        type="button"
                      >
                        FOCUS
                      </button>
                      <button
                        className={`ghost-button ghost-button--compact${selectedTruss3dNodes.length > 0 ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dFocusRequestVersion((current) => current + 1)}
                        type="button"
                      >
                        {t.frameSelection}
                      </button>
                      <button
                        className={`ghost-button ghost-button--compact${truss3dProjectionMode === "persp" ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dProjectionMode((current) => (current === "ortho" ? "persp" : "ortho"))}
                        type="button"
                      >
                        {truss3dProjectionMode === "ortho" ? "TO PERSP" : "TO ORTHO"}
                      </button>
                      <button
                        className={`ghost-button ghost-button--compact${truss3dShowGrid ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dShowGrid((current) => !current)}
                        type="button"
                      >
                        GRID
                      </button>
                      <button
                        className={`ghost-button ghost-button--compact${truss3dShowLabels ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dShowLabels((current) => !current)}
                        type="button"
                      >
                        LABEL
                      </button>
                      <button
                        className={`ghost-button ghost-button--compact${truss3dShowNodes ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dShowNodes((current) => !current)}
                        type="button"
                      >
                        NODE
                      </button>
                      <button
                        className={`ghost-button ghost-button--compact${truss3dBoxSelectMode ? " ghost-button--active" : ""}`}
                        onClick={() => setTruss3dBoxSelectMode((current) => !current)}
                        type="button"
                      >
                        BOX
                      </button>
                      <button
                        className="ghost-button ghost-button--compact"
                        onClick={() => setTruss3dResetRequestVersion((current) => current + 1)}
                        type="button"
                      >
                        RESET
                      </button>
                    </div>
                  ) : null}
                  <div className="panel-tabs viewport-dock__tabs">
                    <button className={`panel-tab${immersiveToolTab === "node" ? " panel-tab--active" : ""}`} onClick={() => setImmersiveToolTab("node")} type="button">
                      {t.immersiveNodeOps}
                    </button>
                    <button className={`panel-tab${immersiveToolTab === "props" ? " panel-tab--active" : ""}`} onClick={() => setImmersiveToolTab("props")} type="button">
                      {t.immersiveQuickProps}
                    </button>
                  </div>
                  <div className="viewport-dock__stack">
                    {immersiveViewport && immersiveToolTab === "node" ? (
                        <div>
                          <div className="card-subhead">
                            <strong>{t.immersiveNodeOps}</strong>
                            <span>{selectedTruss3dNodes.length > 1 ? `${selectedTruss3dNodes.length} ${t.nodes}` : selectedTruss3dNodeData?.id ?? t.none}</span>
                          </div>
                          <div className="button-row">
                            <button className="ghost-button ghost-button--compact" onClick={() => addTruss3dNode(false)} type="button">
                              {t.addNode}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={selectedNode === null} onClick={() => addTruss3dNode(true)} type="button">
                              {t.addBranchNode}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={selectedNode === null} onClick={deleteSelectedTruss3dNode} type="button">
                              {t.deleteNode}
                            </button>
                          </div>
                          <div className="button-row">
                            <button className={`ghost-button ghost-button--compact${truss3dLinkMode ? " ghost-button--active" : ""}`} onClick={toggleTruss3dLinkMode} type="button">
                              {truss3dLinkMode ? t.linkModeActive : t.linkMode}
                            </button>
                            <button className="ghost-button ghost-button--compact" onClick={toggleTruss3dMemberFromDraft} type="button">
                              {t.toggleMember}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={selectedElement === null} onClick={deleteSelectedTruss3dElement} type="button">
                              {t.deleteMember}
                            </button>
                          </div>
                          <div className="button-row">
                            <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes(null)} type="button">
                              {t.duplicateNodes}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes("x")} type="button">
                              {t.mirrorX}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes("y")} type="button">
                              {t.mirrorY}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes("z")} type="button">
                              {t.mirrorZ}
                            </button>
                          </div>
                          <div className="button-row">
                            <button className="ghost-button ghost-button--compact" disabled={undoStack.length === 0} onClick={handleUndo} type="button">
                              {t.undo}
                            </button>
                            <button className="ghost-button ghost-button--compact" disabled={redoStack.length === 0} onClick={handleRedo} type="button">
                              {t.redo}
                            </button>
                          </div>
                          <p className="card-copy">{truss3dLinkMode ? t.linkModeIdle : t.selectionHint}</p>
                        </div>
                    ) : null}
                    {immersiveViewport && immersiveToolTab === "props" ? (
                        <div>
                          <div className="card-subhead">
                            <strong>{t.immersiveQuickProps}</strong>
                            <span>{selectedTruss3dNodes.length > 1 ? `${selectedTruss3dNodes.length} ${t.immersiveNodeSelection}` : selectedTruss3dNodeData?.id ?? t.none}</span>
                          </div>
                          {selectedTruss3dNodeData ? (
                            <>
                              <div className="form-grid compact">
                                <label>
                                  <span>{t.nodeX}</span>
                                  <input
                                    type="number"
                                    step={0.1}
                                    value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.x ?? selectedTruss3dNodeData.x}
                                    onChange={(event) => updateSelectedTruss3dNode("x", Number(event.target.value))}
                                  />
                                </label>
                                <label>
                                  <span>{t.nodeY}</span>
                                  <input
                                    type="number"
                                    step={0.1}
                                    value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.y ?? selectedTruss3dNodeData.y}
                                    onChange={(event) => updateSelectedTruss3dNode("y", Number(event.target.value))}
                                  />
                                </label>
                                <label>
                                  <span>{t.nodeZ}</span>
                                  <input
                                    type="number"
                                    step={0.1}
                                    value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.z ?? selectedTruss3dNodeData.z}
                                    onChange={(event) => updateSelectedTruss3dNode("z", Number(event.target.value))}
                                  />
                                </label>
                                <label>
                                  <span>{t.loadX}</span>
                                  <input
                                    type="number"
                                    step={100}
                                    value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.load_x ?? 0}
                                    onChange={(event) => updateSelectedTruss3dNode("load_x", Number(event.target.value))}
                                  />
                                </label>
                                <label>
                                  <span>{t.loadY}</span>
                                  <input
                                    type="number"
                                    step={100}
                                    value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.load_y ?? 0}
                                    onChange={(event) => updateSelectedTruss3dNode("load_y", Number(event.target.value))}
                                  />
                                </label>
                                <label>
                                  <span>{t.loadZ}</span>
                                  <input
                                    type="number"
                                    step={100}
                                    value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.load_z ?? 0}
                                    onChange={(event) => updateSelectedTruss3dNode("load_z", Number(event.target.value))}
                                  />
                                </label>
                              </div>
                              <div className="button-row">
                                <button
                                  className={`ghost-button ghost-button--compact${truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_x ? " ghost-button--active" : ""}`}
                                  onClick={() => updateSelectedTruss3dNode("fix_x", !(truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_x ?? false))}
                                  type="button"
                                >
                                  {t.fixX}
                                </button>
                                <button
                                  className={`ghost-button ghost-button--compact${truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_y ? " ghost-button--active" : ""}`}
                                  onClick={() => updateSelectedTruss3dNode("fix_y", !(truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_y ?? false))}
                                  type="button"
                                >
                                  {t.fixY}
                                </button>
                                <button
                                  className={`ghost-button ghost-button--compact${truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_z ? " ghost-button--active" : ""}`}
                                  onClick={() => updateSelectedTruss3dNode("fix_z", !(truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_z ?? false))}
                                  type="button"
                                >
                                  {t.fixZ}
                                </button>
                              </div>
                              <div className="card-subhead">
                                <strong>{t.immersiveTransform}</strong>
                                <span>{t.nudgeStep}: {fixed(truss3dNudgeStep, 2)}</span>
                              </div>
                              <div className="button-row">
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", -truss3dNudgeStep)} type="button">X-</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", truss3dNudgeStep)} type="button">X+</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", -truss3dNudgeStep)} type="button">Y-</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", truss3dNudgeStep)} type="button">Y+</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", -truss3dNudgeStep)} type="button">Z-</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", truss3dNudgeStep)} type="button">Z+</button>
                              </div>
                              <label className="inline-field">
                                <span>{t.nudgeStep}</span>
                                <input type="number" min={0.01} step={0.05} value={truss3dNudgeStep} onChange={(event) => setTruss3dNudgeStep(Math.max(0.01, Number(event.target.value) || 0.01))} />
                              </label>
                            </>
                          ) : selectedTruss3dNodes.length > 1 ? (
                            <>
                              <div className="button-row">
                                <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_x", true)} type="button">
                                  {t.fixX}
                                </button>
                                <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_y", true)} type="button">
                                  {t.fixY}
                                </button>
                                <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_z", true)} type="button">
                                  {t.fixZ}
                                </button>
                              </div>
                              <div className="button-row">
                                <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_x", false)} type="button">
                                  {t.releaseX}
                                </button>
                                <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_y", false)} type="button">
                                  {t.releaseY}
                                </button>
                                <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_z", false)} type="button">
                                  {t.releaseZ}
                                </button>
                              </div>
                              <div className="card-subhead">
                                <strong>{t.immersiveTransform}</strong>
                                <span>{t.nudgeStep}: {fixed(truss3dNudgeStep, 2)}</span>
                              </div>
                              <div className="button-row">
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", -truss3dNudgeStep)} type="button">X-</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", truss3dNudgeStep)} type="button">X+</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", -truss3dNudgeStep)} type="button">Y-</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", truss3dNudgeStep)} type="button">Y+</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", -truss3dNudgeStep)} type="button">Z-</button>
                                <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", truss3dNudgeStep)} type="button">Z+</button>
                              </div>
                              <label className="inline-field">
                                <span>{t.nudgeStep}</span>
                                <input type="number" min={0.01} step={0.05} value={truss3dNudgeStep} onChange={(event) => setTruss3dNudgeStep(Math.max(0.01, Number(event.target.value) || 0.01))} />
                              </label>
                              <div className="card-subhead">
                                <strong>{t.immersiveLoads}</strong>
                                <span>{selectedTruss3dNodes.length} {t.nodes}</span>
                              </div>
                              <div className="form-grid compact">
                                <label>
                                  <span>{t.loadX}</span>
                                  <input type="number" step={100} value={truss3dBatchLoadX} onChange={(event) => setTruss3dBatchLoadX(Number(event.target.value))} />
                                </label>
                                <label>
                                  <span>{t.loadY}</span>
                                  <input type="number" step={100} value={truss3dBatchLoadY} onChange={(event) => setTruss3dBatchLoadY(Number(event.target.value))} />
                                </label>
                                <label>
                                  <span>{t.loadZ}</span>
                                  <input type="number" step={100} value={truss3dBatchLoadZ} onChange={(event) => setTruss3dBatchLoadZ(Number(event.target.value))} />
                                </label>
                              </div>
                              <div className="button-row">
                                <button className="ghost-button ghost-button--compact" onClick={() => applySelectedTruss3dLoads("apply")} type="button">
                                  {t.applyLoads}
                                </button>
                                <button className="ghost-button ghost-button--compact" onClick={() => applySelectedTruss3dLoads("clear")} type="button">
                                  {t.clearLoads}
                                </button>
                              </div>
                              <p className="card-copy">{t.selectionHint}</p>
                            </>
                          ) : (
                            <p className="card-copy">{t.immersiveNoNodeSelection}</p>
                          )}
                        </div>
                    ) : null}
                  </div>
                </section>
              ) : null}
              {showShortcutHints && immersiveHelpDrawerOpen ? (
                <section className="viewport-dock__card viewport-dock__card--help">
                  <div className="card-head">
                    <h2>{t.immersiveHelp}</h2>
                    <span>{t.kinds.truss_3d}</span>
                  </div>
                  <div className="viewport-help-list">
                    {t.shortcutLegendRows.map((row) => (
                      <p key={row} className="card-copy">
                        {row}
                      </p>
                    ))}
                  </div>
                </section>
              ) : null}
            </div>
          ) : null}
          {activeResultWindow ? (
            <div className="viewport-window-bar">
              <div className="viewport-window-bar__meta">
                <strong>{t.resultWindow}</strong>
                <span>
                  {t.pageRange}: {resultWindowStart}-{resultWindowEnd}
                </span>
                <span>
                  {t.chunkSize}: {activeResultWindowLimit}
                </span>
                <span>
                  {t.nodes}: {activeResultWindow.totalNodes}
                </span>
                <span>
                  {t.totalElements}: {activeResultWindow.totalElements}
                </span>
              </div>
              <div className="button-row">
                {resultWindowJumps.map((jump) => (
                  <button
                    key={jump.label}
                    className="ghost-button ghost-button--compact"
                    disabled={resultWindowOffset === jump.offset}
                    onClick={() => setResultWindowOffset(jump.offset)}
                    type="button"
                  >
                    {jump.label}
                  </button>
                ))}
                <button
                  className="ghost-button ghost-button--compact"
                  disabled={resultWindowOffset <= 0}
                  onClick={() =>
                    setResultWindowOffset((current) =>
                      clampChunkOffset(current - activeResultWindowLimit, resultWindowMaxTotal, activeResultWindowLimit),
                    )
                  }
                  type="button"
                >
                  {t.previousPage}
                </button>
                <button
                  className="ghost-button ghost-button--compact"
                  disabled={resultWindowOffset + activeResultWindowLimit >= resultWindowMaxTotal}
                  onClick={() =>
                    setResultWindowOffset((current) =>
                      clampChunkOffset(current + activeResultWindowLimit, resultWindowMaxTotal, activeResultWindowLimit),
                    )
                  }
                  type="button"
                >
                  {t.nextPage}
                </button>
              </div>
            </div>
          ) : null}
          <div
            className={`canvas-stage${isTruss3d ? " canvas-stage--space" : ""}${shouldStretchSpaceViewport ? " canvas-stage--space-fluid" : ""}`}
            onScroll={handleCanvasStageScroll}
            ref={canvasStageRef}
          >
          <WorkbenchViewport
            studyKind={studyKind}
            sidebarSection={sidebarSection}
            title={t.sections.model}
            axialTitle={t.kinds.axial_bar_1d}
            trussTitle={t.kinds.truss_2d}
            truss3dTitle={t.kinds.truss_3d}
            planeTitle={t.kinds.plane_triangle_2d}
            axialNodes={axialNodes}
            axialLength={axialLength}
            axialScale={axialScale}
            displayTrussNodes={displayTrussNodes}
            displayTrussElements={displayTrussElements}
            trussElementColors={trussElementColors}
            hiddenTrussMaterialIds={isTruss ? hiddenMaterialIds : []}
            trussBounds={trussBounds}
            trussResult={Boolean(trussResult)}
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
            }}
            onStartTrussNodeDrag={(index) => {
              dragHistoryCapturedRef.current = false;
              setDraggingNode(index);
              toggleDraftNode(index);
            }}
            displayTruss3dNodes={displayTruss3dNodes}
            displayTruss3dElements={displayTruss3dElements}
            truss3dElementColors={truss3dElementColors}
            hiddenTruss3dMaterialIds={isTruss3d ? hiddenMaterialIds : []}
            planeNodes={planeNodes}
            planeElements={planeElements}
            planeElementColors={planeElementColors}
            hiddenPlaneMaterialIds={isPlane ? hiddenMaterialIds : []}
            planeBounds={planeBounds}
            planeResult={Boolean(planeResult)}
            planeMaxVonMises={planeMaxVonMises}
            selectedPlaneNodeId={selectedPlaneNodeData?.id ?? null}
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
            onProjectionModeChange={setTruss3dProjectionMode}
            onShowGridChange={setTruss3dShowGrid}
            onShowLabelsChange={setTruss3dShowLabels}
            onShowNodesChange={setTruss3dShowNodes}
            onBoxSelectModeChange={setTruss3dBoxSelectMode}
            viewportPixelWidth={viewportPixelWidth}
          />
          </div>
          </div>
          {immersiveViewport && sidebarSection === "library" ? (
            <div className="immersive-drawer">
              <section className="immersive-drawer__card">
                <div className="card-head">
                  <h2>{t.immersiveDrawer}</h2>
                  <div className="immersive-drawer__head-actions">
                    <span>{t.immersiveLibrary}</span>
                    <button
                      className="ghost-button ghost-button--compact"
                      onClick={() => setSidebarSection("model")}
                      type="button"
                    >
                      {t.close}
                    </button>
                  </div>
                </div>
                <div className="immersive-drawer__grid">
                  <div className="immersive-drawer__section">
                    <h3>{t.immersiveSamples}</h3>
                    <div className="immersive-drawer__list">
                      {SAMPLE_LIBRARY.slice(0, 4).map((sample) => (
                        <button
                          key={sample.href}
                          className="history-item immersive-drawer__button"
                          onClick={() => openSample(sample.href)}
                          type="button"
                        >
                          <strong>{sample.name}</strong>
                          <span>{sample.kind}</span>
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="immersive-drawer__section">
                    <h3>{t.immersiveModels}</h3>
                    <div className="immersive-drawer__list">
                      {deferredProjectModels.slice(0, 4).length > 0 ? (
                        deferredProjectModels.slice(0, 4).map((model) => (
                          <button
                            key={model.model_id}
                            className="history-item immersive-drawer__button"
                            onClick={() => openSavedModel(model)}
                            type="button"
                          >
                            <strong>{model.name}</strong>
                            <small>{t.updatedAt}: {formatTime(model.updated_at, language)}</small>
                          </button>
                        ))
                      ) : (
                        <p className="card-copy">{t.immersiveEmptyModels}</p>
                      )}
                    </div>
                  </div>
                  <div className="immersive-drawer__section">
                    <h3>{t.immersiveJobs}</h3>
                    <div className="immersive-drawer__list">
                      {deferredJobHistory.slice(0, 4).length > 0 ? (
                        deferredJobHistory.slice(0, 4).map((historyJob) => (
                          <button
                            key={historyJob.job_id}
                            className="history-item immersive-drawer__button"
                            onClick={() => openHistoryJob(historyJob.job_id)}
                            type="button"
                          >
                            <strong>{historyJob.job_id.slice(0, 8)}</strong>
                            <span>{historyJob.status}</span>
                          </button>
                        ))
                      ) : (
                        <p className="card-copy">{t.immersiveEmptyJobs}</p>
                      )}
                    </div>
                  </div>
                </div>
              </section>
            </div>
          ) : null}
        </section>

        <WorkbenchConsole
          sidebarSection={sidebarSection}
          title={sidebarSection === "model" ? t.nodeTable : t.report}
          subtitle={message}
          modelMessageTitle={t.dragNode}
          reportMessageTitle={t.messages}
          message={message}
          dragNodeLabel={t.dragNode}
          noNodeSelectedLabel={t.noNodeSelected}
          loadCaseLabel={t.loadCase}
          diagnosticsLabel={t.diagnostics}
          selectedNodeId={isPlane ? selectedPlaneNodeData?.id ?? null : selectedNodeData?.id ?? null}
          selectedNodeX={isPlane ? selectedPlaneNodeData?.x : selectedNodeData?.x}
          selectedNodeY={isPlane ? selectedPlaneNodeData?.y : selectedNodeData?.y}
          selectedNodeLoadY={isPlane ? selectedPlaneNodeData?.load_y : selectedNodeData?.load_y}
          selectedNodeIssueCount={isPlane ? null : selectedNodeIssues.length > 0 ? selectedNodeIssues.length : null}
          elementTitle={isAxial ? t.axialElements : isTruss ? t.trussElements : isTruss3d ? t.spatialTrussElements : t.planeElements}
          spanLabel={t.span}
          stressLabel={t.stress}
          axialForceLabel={t.axialForce}
          elements={(isAxial ? axialElements : isTruss ? displayTrussElements : isTruss3d ? displayTruss3dElements : planeElements) as Array<{
            index: number;
            x1?: number;
            x2?: number;
            node_i?: number;
            node_j?: number;
            node_k?: number;
            stress?: number;
            axial_force?: number;
            von_mises?: number;
          }>}
        />
      </main>

      <WorkbenchInspector
        t={t}
        sidebarSection={sidebarSection}
        studyKind={studyKind}
        isPending={isPending}
        selectedNodeData={selectedNodeData ? { ...selectedNodeData } : null}
        selectedElementData={selectedElementData ? { ...selectedElementData } : null}
        selectedTruss3dNodeData={selectedTruss3dNodeData ? { ...selectedTruss3dNodeData, ...truss3dModel.nodes[selectedTruss3dNodeData.index] } : null}
        selectedTruss3dElementData={selectedTruss3dElementData ? { ...selectedTruss3dElementData } : null}
        selectedPlaneNodeData={selectedPlaneNodeData ? { ...selectedPlaneNodeData } : null}
        selectedPlaneElementData={selectedPlaneElementData ? { ...selectedPlaneElementData } : null}
        trussElementArea={selectedElementData ? trussModel.elements[selectedElementData.index]?.area ?? 0 : 0}
        trussElementModulusGpa={selectedElementData ? round((trussModel.elements[selectedElementData.index]?.youngs_modulus ?? 0) / 1.0e9) : 0}
        trussElementMaterialId={selectedElementData ? trussModel.elements[selectedElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : ""}
        truss3dElementArea={selectedTruss3dElementData ? truss3dModel.elements[selectedTruss3dElementData.index]?.area ?? 0 : 0}
        truss3dElementModulusGpa={selectedTruss3dElementData ? round((truss3dModel.elements[selectedTruss3dElementData.index]?.youngs_modulus ?? 0) / 1.0e9) : 0}
        truss3dElementMaterialId={selectedTruss3dElementData ? truss3dModel.elements[selectedTruss3dElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : ""}
        planeElementThickness={selectedPlaneElementData ? planeModel.elements[selectedPlaneElementData.index]?.thickness ?? 0 : 0}
        planeElementModulusGpa={selectedPlaneElementData ? round((planeModel.elements[selectedPlaneElementData.index]?.youngs_modulus ?? 0) / 1.0e9) : 0}
        planeElementPoissonRatio={selectedPlaneElementData ? planeModel.elements[selectedPlaneElementData.index]?.poisson_ratio ?? 0.33 : 0.33}
        planeElementMaterialId={selectedPlaneElementData ? planeModel.elements[selectedPlaneElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : ""}
        materialOptions={materialOptions}
        materialLabel={t.material}
        onUpdateSelectedNode={updateSelectedNode}
        onUpdateSelectedElement={updateSelectedElement}
        onAssignSelectedElementMaterial={assignSelectedElementMaterial}
        onUpdateSelectedTruss3dNode={updateSelectedTruss3dNode}
        onUpdateSelectedTruss3dElement={updateSelectedTruss3dElement}
        onAssignSelectedTruss3dElementMaterial={assignSelectedTruss3dElementMaterial}
        onUpdateSelectedPlaneNode={updateSelectedPlaneNode}
        onUpdateSelectedPlaneElement={updateSelectedPlaneElement}
        onAssignSelectedPlaneElementMaterial={assignSelectedPlaneElementMaterial}
        trussDiagnostics={trussDiagnostics}
        trussStability={trussStability}
        hotspotNodeLabels={(trussStability?.hotspotNodes ?? []).map((nodeIndex) => trussModel.nodes[nodeIndex]?.id ?? nodeIndex).join(", ")}
        onApplyTrussSuggestion={(id) => {
          const suggestion = trussDiagnostics?.suggestions.find((entry) => entry.id === id);
          if (suggestion) applyTrussSuggestion(suggestion);
        }}
        undoStack={undoStack}
        redoStack={redoStack}
        onUndo={handleUndo}
        onRedo={handleRedo}
        job={job}
        nodeCount={nodeCount}
        tipDisplacement={isAxial ? scientific(axialResult?.tip_displacement) : isTruss ? scientific(trussResult?.max_displacement) : isTruss3d ? scientific(truss3dResult?.max_displacement) : scientific(planeResult?.max_displacement)}
        maxStressValue={scientific(isAxial ? axialResult?.max_stress : isTruss ? trussResult?.max_stress : isTruss3d ? truss3dResult?.max_stress : planeResult?.max_stress)}
        reactionValue={isAxial ? scientific(axialResult?.reaction_force) : "--"}
        createdAtValue={formatTime(job?.created_at, language)}
        updatedAtValue={formatTime(job?.updated_at, language)}
        heartbeatStatusValue={heartbeatStatusValue}
        heartbeatTone={heartbeatToneValue}
        failureReasonValue={translatedFailureReason ?? job?.message ?? "--"}
        canCancelJob={jobIsActive}
        onCancelJob={cancelCurrentJob}
        onDownloadJson={downloadResultJson}
        onDownloadCsv={downloadResultCsv}
      />
    </div>
  );
}
