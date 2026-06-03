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
  type UIEvent as ReactUIEvent,
  type SetStateAction,
} from "react";
import brand from "../../../../../assets/brand/brand.json";
import { WorkbenchAssistantPanel } from "@/components/workbench/workbench-assistant-panel";
import { WorkbenchConsole } from "@/components/workbench/workbench-console";
import { WorkbenchInspector } from "@/components/workbench/workbench-inspector";
import { WorkbenchObjectTree } from "@/components/workbench/workbench-object-tree";
import { WorkbenchScriptPanel } from "@/components/workbench/workbench-script-panel";
import { WorkbenchViewportPanel } from "@/components/workbench/workbench-viewport-panel";
import { WorkbenchViewport } from "@/components/workbench/workbench-viewport";
import { WorkbenchLibrarySidebar } from "@/components/workbench/library/workbench-library-sidebar";
import { WorkbenchMaterialLibraryCard } from "@/components/workbench/model/workbench-material-library-card";
import { WorkbenchModelSidebar, type ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";
import { WorkbenchModelToolsCard } from "@/components/workbench/model/workbench-model-tools-card";
import { WorkbenchParametricCard } from "@/components/workbench/model/workbench-parametric-card";
import { WorkbenchTruss3dTreeCard } from "@/components/workbench/model/workbench-truss3d-tree-card";
import { WorkbenchStudySidebar } from "@/components/workbench/study/workbench-study-sidebar";
import { WorkbenchDataAdminPanel } from "@/components/workbench/system/workbench-data-admin-panel";
import { WorkbenchSystemConfigCard } from "@/components/workbench/system/workbench-system-config-card";
import { WorkbenchSystemRuntimePanel } from "@/components/workbench/system/workbench-system-runtime-panel";
import { WorkbenchSystemSidebar } from "@/components/workbench/system/workbench-system-sidebar";
import {
  isWorkflowGraphResult,
  summarizeWorkflowArtifacts,
  upsertWorkflowRunRecord,
  useWorkbenchWorkflowController,
} from "@/components/workbench/workflow/workbench-workflow-controller";
import { WorkbenchWorkflowSidebar } from "@/components/workbench/workflow/workbench-workflow-sidebar";
import type {
  WorkflowSurfaceTab,
} from "@/components/workbench/workflow/workbench-workflow-types";
import { requestWorkbenchAssistantPlan, type AssistantPlan } from "@/lib/assistant/openai-compatible";
import { parseMaterialLibrary } from "@/lib/materials";
import { createMaterialDefinition, MATERIAL_PRESETS } from "@/lib/materials";
import { parsePlaygroundModel } from "@/lib/models";
import { exportProjectBundleZip, parseProjectBundleFile } from "@/lib/projects";
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
  WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
  type WorkbenchLanguagePack,
} from "@/lib/workbench/helpers";
import { serializeResultCsv } from "@/lib/workbench/result-csv";
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
  createSecurityAuditEntry,
  readSecurityAuditLog,
  writeSecurityAuditLog,
  type WorkbenchSecurityAuditEntry,
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
  buildStudyControlsRows,
  buildStudyKindOptionGroups,
  buildStudySummaryRows,
  buildTruss3dTreeRows,
} from "@/lib/workbench/view-models";
import {
  addCustomMaterialToFrameModel,
  addCustomMaterialToPlaneModel,
  addPresetMaterialToFrameModel,
  addCustomMaterialToTruss3dModel,
  addCustomMaterialToTrussModel,
  applyMaterialToFrameModel,
  addPresetMaterialToPlaneModel,
  addPresetMaterialToTruss3dModel,
  addPresetMaterialToTrussModel,
  applyMaterialToPlaneModel,
  applyMaterialToTruss3dModel,
  applyMaterialToTrussModel,
  deleteMaterialFromFrameModel,
  deleteMaterialFromPlaneModel,
  deleteMaterialFromTruss3dModel,
  deleteMaterialFromTrussModel,
  ensurePlaneModelMaterials,
  ensureTruss3dModelMaterials,
  ensureTrussModelMaterials,
  mergeImportedMaterials,
  updateMaterialInFrameModel,
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
  addFrame2dNode,
  assignFrame2dElementMaterial,
  deleteFrame2dElement,
  deleteFrame2dNode,
  toggleFrame2dMember,
  updateFrame2dElement,
  updateFrame2dNode,
} from "@/lib/workbench/frame2d-commands";
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
  generateRectangularQuadPanelMesh,
  generateRectangularPanelMesh,
  type ParametricPanelConfig,
  type ParametricTrussConfig,
} from "@/lib/models";
import { SAMPLE_LIBRARY } from "@/lib/models";
import {
  getWorkbenchScriptActionDefinition,
  getWorkbenchScriptMacroDefinition,
  listWorkbenchMacroPresets,
  resolveWorkbenchMacroPayloadTemplates,
  saveWorkbenchMacroPreset,
  type WorkbenchScriptActionLogEntry,
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
  createSecurityEvent,
  createDirectMeshSolve,
  createFrame2dJob,
  createPlaneQuad2dJob,
  createPlaneTriangle2dJob,
  createModel,
  createModelVersion,
  createProject,
  fetchSecurityEvents,
  exportSecurityEvents,
  exportSecurityEventsCsv,
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
  type ResultChunkPayload,
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

type Language = "en" | "zh" | "ja" | "es";
type Theme = "linen" | "marine" | "graphite";
type SidebarSection = "study" | "model" | "workflow" | "library" | "system";
type StudyKind = "axial_bar_1d" | "heat_bar_1d" | "heat_plane_triangle_2d" | "heat_plane_quad_2d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_frame_2d" | "thermal_truss_2d" | "thermal_truss_3d" | "thermal_plane_triangle_2d" | "thermal_plane_quad_2d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";
type PlaneStudyJobInput = PlaneTriangle2dJobInput | PlaneQuad2dJobInput | ThermalPlaneTriangle2dJobInput | ThermalPlaneQuad2dJobInput;
type HeatPlaneStudyJobInput = HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput;
type FrameStudyJobInput = Frame2dJobInput;
type ThermalFrameStudyJobInput = ThermalFrame2dJobInput;
type BeamStudyJobInput = Beam1dJobInput;
type ThermalBeamStudyJobInput = ThermalBeam1dJobInput;
type ThermalBarStudyJobInput = ThermalBar1dJobInput;
type HeatBarStudyJobInput = HeatBar1dJobInput;
type ThermalTruss2dStudyJobInput = ThermalTruss2dJobInput;
type ThermalTruss3dStudyJobInput = ThermalTruss3dJobInput;
type SpringStudyJobInput = Spring1dJobInput;
type Spring2dStudyJobInput = Spring2dJobInput;
type Spring3dStudyJobInput = Spring3dJobInput;
type LineResultField =
  | "axial_stress"
  | "max_bending_stress"
  | "max_combined_stress"
  | "moment"
  | "shear_force"
  | "average_temperature_delta"
  | "temperature_gradient_y"
  | "thermal_curvature";
type FrameResultField = Exclude<LineResultField, "shear_force">;
type BeamResultField = Extract<LineResultField, "max_bending_stress" | "moment" | "shear_force" | "temperature_gradient_y" | "thermal_curvature">;
type StudyPanelTab = "summary" | "controls";
type ModelPanelTab = "tools" | "tree";
type LibraryPanelTab = "jobs" | "results" | "models" | "projects" | "samples";
type WorkflowPanelTab = WorkflowSurfaceTab;
type ImmersiveToolTab = "node" | "props";
type SystemDataTab = "jobs" | "results";
type SystemPanelTab = "config" | "assistant" | "scripts" | "runtime" | "data";
type AssistantMode = "local" | "llm";
type SecurityEventWindow = "" | "1h" | "24h" | "7d" | "30d";

const SECURITY_EVENT_WINDOW_MS: Record<Exclude<SecurityEventWindow, "">, number> = {
  "1h": 60 * 60 * 1_000,
  "24h": 24 * 60 * 60 * 1_000,
  "7d": 7 * 24 * 60 * 60 * 1_000,
  "30d": 30 * 24 * 60 * 60 * 1_000,
};

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
  axial_stress?: number;
  max_bending_stress?: number;
  max_combined_stress?: number;
  axial_force_i?: number;
  shear_force_i?: number;
  moment_i?: number;
  axial_force_j?: number;
  shear_force_j?: number;
  moment_j?: number;
  average_temperature_delta?: number;
  temperature_gradient_y?: number;
  thermal_curvature?: number;
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

type PlaneResultField =
  | "von_mises"
  | "principal_stress_1"
  | "max_in_plane_shear"
  | "average_temperature"
  | "average_temperature_delta"
  | "temperature_gradient_x"
  | "temperature_gradient_y"
  | "heat_flux_x"
  | "heat_flux_y"
  | "heat_flux_magnitude"
  | "thermal_strain"
  | "mechanical_strain";

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

const defaultPlaneTriangle: PlaneTriangle2dJobInput = {
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

const defaultPlaneQuad: PlaneQuad2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 1, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -800 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: false, load_x: 0, load_y: -800 },
  ],
  elements: [
    {
      id: "q0",
      node_i: 0,
      node_j: 1,
      node_k: 2,
      node_l: 3,
      thickness: 0.02,
      youngs_modulus: 70e9,
      poisson_ratio: 0.33,
      material_id: "mat-1",
    },
  ],
};

const defaultThermalPlaneTriangle: ThermalPlaneTriangle2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "n1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "n2", x: 1, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
  ],
  elements: [
    { id: "tp0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, thermal_expansion: 12e-6, material_id: "mat-1" },
    { id: "tp1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, thermal_expansion: 12e-6, material_id: "mat-1" },
  ],
};

const defaultThermalPlaneQuad: ThermalPlaneQuad2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1", poisson_ratio: 0.33 })],
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    { id: "n1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    { id: "n2", x: 1, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
  ],
  elements: [
    { id: "tq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33, thermal_expansion: 11e-6, material_id: "mat-1" },
  ],
};

const defaultHeatPlaneTriangle: HeatPlaneTriangle2dJobInput = {
  nodes: [
    { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
    { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
    { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
    { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
  ],
  elements: [
    { id: "hp0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, conductivity: 45 },
    { id: "hp1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, conductivity: 45 },
  ],
};

const defaultHeatPlaneQuad: HeatPlaneQuad2dJobInput = {
  nodes: [
    { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
    { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
    { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
    { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
  ],
  elements: [
    { id: "hq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, conductivity: 45 },
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

const defaultBeam1d: Beam1dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "b0", x: 0, fix_y: true, fix_rz: true, load_y: 0, moment_z: 0 },
    { id: "b1", x: 2.4, fix_y: false, fix_rz: false, load_y: -12000, moment_z: 0 },
  ],
  elements: [
    {
      id: "m0",
      node_i: 0,
      node_j: 1,
      youngs_modulus: 210e9,
      moment_of_inertia: 1.2e-4,
      section_modulus: 1.1e-3,
      distributed_load_y: 0,
      material_id: "mat-1",
    },
  ],
};

const defaultThermalBeam1d: ThermalBeam1dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "tb0", x: 0, fix_y: true, fix_rz: true, load_y: 0, moment_z: 0 },
    { id: "tb1", x: 2.4, fix_y: false, fix_rz: false, load_y: 0, moment_z: 0 },
  ],
  elements: [
    {
      id: "tm0",
      node_i: 0,
      node_j: 1,
      youngs_modulus: 210e9,
      moment_of_inertia: 1.2e-4,
      section_modulus: 1.1e-3,
      thermal_expansion: 1.2e-5,
      section_depth: 0.3,
      distributed_load_y: 0,
      temperature_gradient_y: 45,
      material_id: "mat-1",
    },
  ],
};

const defaultTorsion1d: Torsion1dJobInput = {
  nodes: [
    { id: "t0", x: 0, fix_rz: true, torque_z: 0 },
    { id: "t1", x: 1.5, fix_rz: false, torque_z: 2500 },
  ],
  elements: [
    {
      id: "s0",
      node_i: 0,
      node_j: 1,
      shear_modulus: 79e9,
      polar_moment: 1.8e-6,
      section_modulus: 1.2e-4,
    },
  ],
};

const defaultSpring1d: Spring1dJobInput = {
  nodes: [
    { id: "s0", x: 0, fix_x: true, load_x: 0 },
    { id: "s1", x: 1.2, fix_x: false, load_x: 0 },
    { id: "s2", x: 2.4, fix_x: false, load_x: 1200 },
  ],
  elements: [
    { id: "k0", node_i: 0, node_j: 1, stiffness: 35000 },
    { id: "k1", node_i: 1, node_j: 2, stiffness: 20000 },
  ],
};

const defaultThermalBar1d: ThermalBar1dJobInput = {
  nodes: [
    { id: "t0", x: 0, fix_x: true, load_x: 0, temperature_delta: 40 },
    { id: "t1", x: 1.5, fix_x: true, load_x: 0, temperature_delta: 40 },
  ],
  elements: [
    { id: "tb0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210e9, thermal_expansion: 1.2e-5 },
  ],
};

const defaultHeatBar1d: HeatBar1dJobInput = {
  nodes: [
    { id: "h0", x: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
    { id: "h1", x: 1, fix_temperature: false, temperature: 0, heat_load: 0 },
    { id: "h2", x: 2, fix_temperature: true, temperature: 20, heat_load: 0 },
  ],
  elements: [
    { id: "he0", node_i: 0, node_j: 1, area: 0.01, conductivity: 45 },
    { id: "he1", node_i: 1, node_j: 2, area: 0.01, conductivity: 45 },
  ],
};

const defaultThermalTruss2d: ThermalTruss2dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "tt0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "tt1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    { id: "tt2", x: 0.5, y: 0.8, fix_x: false, fix_y: false, load_x: 0, load_y: -400, temperature_delta: 25 },
  ],
  elements: [
    { id: "tte0", node_i: 0, node_j: 2, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte2", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
  ],
};

const defaultThermalTruss3d: ThermalTruss3dJobInput = {
  materials: [createMaterialDefinition("70", 1, { id: "mat-1" })],
  nodes: [
    { id: "tb0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0, temperature_delta: 35 },
    { id: "tb1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0, temperature_delta: 35 },
    { id: "tb2", x: 0.1, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0, temperature_delta: 35 },
    { id: "tb3", x: 0.35, y: 0.3, z: 1.0, fix_x: false, fix_y: false, fix_z: false, load_x: 0, load_y: 0, load_z: -900, temperature_delta: 15 },
  ],
  elements: [
    { id: "tte0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte2", node_i: 2, node_j: 0, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte3", node_i: 0, node_j: 3, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte4", node_i: 1, node_j: 3, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
    { id: "tte5", node_i: 2, node_j: 3, area: 0.01, youngs_modulus: 70e9, thermal_expansion: 1.2e-5, material_id: "mat-1" },
  ],
};

const defaultSpring2d: Spring2dJobInput = {
  nodes: [
    { id: "s0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "s1", x: 1.2, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "s2", x: 1.2, y: 1.2, fix_x: false, fix_y: false, load_x: 1200, load_y: -600 },
    { id: "s3", x: 0, y: 1.2, fix_x: true, fix_y: false, load_x: 0, load_y: 0 },
  ],
  elements: [
    { id: "k0", node_i: 0, node_j: 1, stiffness: 28000 },
    { id: "k1", node_i: 1, node_j: 2, stiffness: 18000 },
    { id: "k2", node_i: 2, node_j: 3, stiffness: 22000 },
    { id: "k3", node_i: 3, node_j: 0, stiffness: 18000 },
    { id: "k4", node_i: 0, node_j: 2, stiffness: 12000 },
  ],
};

const defaultSpring3d: Spring3dJobInput = {
  nodes: [
    { id: "s0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "s1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "s2", x: 0, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "top", x: 0.45, y: 0.35, z: 1.1, fix_x: false, fix_y: false, fix_z: false, load_x: 250, load_y: 0, load_z: -1100 },
  ],
  elements: [
    { id: "k0", node_i: 0, node_j: 3, stiffness: 18000 },
    { id: "k1", node_i: 1, node_j: 3, stiffness: 22000 },
    { id: "k2", node_i: 2, node_j: 3, stiffness: 16000 },
    { id: "k3", node_i: 0, node_j: 1, stiffness: 9000 },
    { id: "k4", node_i: 1, node_j: 2, stiffness: 9000 },
    { id: "k5", node_i: 2, node_j: 0, stiffness: 9000 },
  ],
};

const defaultFrame2d: Frame2dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "f0", x: 0, y: 0, fix_x: true, fix_y: true, fix_rz: true, load_x: 0, load_y: 0, moment_z: 0 },
    { id: "f1", x: 0, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0 },
    { id: "f2", x: 3.2, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: -12000, moment_z: 0 },
    { id: "f3", x: 3.2, y: 0, fix_x: false, fix_y: true, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0 },
  ],
  elements: [
    { id: "c0", node_i: 0, node_j: 1, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, material_id: "mat-1" },
    { id: "b0", node_i: 1, node_j: 2, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.2e-4, section_modulus: 1.1e-3, material_id: "mat-1" },
    { id: "c1", node_i: 2, node_j: 3, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, material_id: "mat-1" },
  ],
};

const defaultThermalFrame2d: ThermalFrame2dJobInput = {
  materials: [createMaterialDefinition("210", 1, { id: "mat-1" })],
  nodes: [
    { id: "tf0", x: 0, y: 0, fix_x: true, fix_y: true, fix_rz: true, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 20 },
    { id: "tf1", x: 0, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 40 },
    { id: "tf2", x: 3.2, y: 2.4, fix_x: false, fix_y: false, fix_rz: false, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 40 },
    { id: "tf3", x: 3.2, y: 0, fix_x: true, fix_y: true, fix_rz: true, load_x: 0, load_y: 0, moment_z: 0, temperature_delta: 20 },
  ],
  elements: [
    { id: "tc0", node_i: 0, node_j: 1, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, thermal_expansion: 1.2e-5, section_depth: 0.32, temperature_gradient_y: 0, material_id: "mat-1" },
    { id: "tb0", node_i: 1, node_j: 2, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.2e-4, section_modulus: 1.1e-3, thermal_expansion: 1.2e-5, section_depth: 0.28, temperature_gradient_y: 30, material_id: "mat-1" },
    { id: "tc1", node_i: 2, node_j: 3, area: 0.018, youngs_modulus: 210e9, moment_of_inertia: 1.8e-4, section_modulus: 1.6e-3, thermal_expansion: 1.2e-5, section_depth: 0.32, temperature_gradient_y: 0, material_id: "mat-1" },
  ],
};

const copyEn = {
    brand: brand.productName,
    shortTitle: brand.workbenchShortName ?? brand.applicationName.replace(/^Kyuubiki\s+/u, ""),
    roleLabel: brand.workbenchRoleLabel ?? "Analysis shell",
    title: brand.workbenchShortName ?? brand.applicationName.replace(/^Kyuubiki\s+/u, ""),
    subtitle: brand.workbenchDescription,
    rail: { study: "Study", model: "Workspace", workflow: "Workflow", library: "History", system: "System" },
    sections: { study: "Study Setup", model: "Workspace", workflow: "Workflow Studio", library: "Job History", system: "System" },
    kinds: {
      axial_bar_1d: "1D axial bar",
      heat_bar_1d: "1D heat bar",
      heat_plane_triangle_2d: "2D heat plane triangle",
      heat_plane_quad_2d: "2D heat plane quad",
      thermal_bar_1d: "1D thermal bar",
      thermal_beam_1d: "1D thermal beam",
      thermal_frame_2d: "2D thermal frame",
      thermal_truss_2d: "2D thermal truss",
      thermal_truss_3d: "3D thermal space truss",
      thermal_plane_triangle_2d: "2D thermal plane triangle",
      thermal_plane_quad_2d: "2D thermal plane quad",
      spring_1d: "1D spring",
      spring_2d: "2D spring",
      spring_3d: "3D spring",
      beam_1d: "1D beam",
      torsion_1d: "1D torsion shaft",
      truss_2d: "2D truss",
      truss_3d: "3D space truss",
      plane_triangle_2d: "2D plane triangle",
      plane_quad_2d: "2D plane quad",
      frame_2d: "2D frame",
    },
    studyFamilies: {
      axialAndSprings: "Axial & Springs",
      beamsAndFrames: "Beams & Frames",
      trusses: "Trusses",
      planes: "Planes",
    },
    studyDomains: {
      mechanical: "Mechanical",
      thermal: "Thermal",
      thermoMechanical: "Thermo-mechanical",
    },
    familyHints: {
      axialAndSprings: "Line studies focused on stiffness, extension, and axial force paths.",
      beamsAndFrames: "Member studies focused on bending, shear, rotation, and end-force review.",
      trusses: "Connectivity studies focused on axial load paths and structural stability.",
      planes: "Continuum studies focused on stress fields, patches, and hotspot elements.",
    },
    importModel: "Import model",
    importHint: "Load a JSON model for 1D or 2D studies.",
    sampleCatalogPage: "Catalog",
    sampleImportPage: "Import",
    workflowOverviewPage: "Overview",
    workflowCatalogPage: "Catalog",
    workflowBuilderPage: "Builder",
    workflowRunsPage: "Runs",
    workflowCatalogTitle: "Workflow Catalog",
    workflowCatalogHint: "Run named multi-operator workflows here before wiring them into a larger study path.",
    workflowOverviewHint: "Keep composite operator workflows in one dedicated surface instead of hiding them inside samples or results.",
    workflowBuilderHint: "Inspect graph nodes, bridges, extracts, and exports before turning them into a larger study chain.",
    workflowRunsHint: "Track named workflow jobs, current nodes, and exported summaries from one place.",
    workflowCatalogRefresh: "Refresh workflows",
    workflowCatalogRun: "Run reference sample",
    workflowCatalogEmpty: "No named workflows are published yet.",
    workflowCatalogReady: "Workflow catalog ready.",
    workflowCatalogQueued: "Queued workflow job",
    workflowCatalogCompleted: "Workflow completed",
    workflowCatalogUnsupported: "This workflow does not have a built-in sample input yet.",
    workflowCatalogLoaded: "Loaded workflow catalog.",
    workflowCatalogFailed: "Workflow run failed.",
    workflowSelectForBuilder: "Open builder",
    workflowNoSelection: "Select a named workflow first so the builder can show its graph.",
    workflowNodesTitle: "Nodes",
    workflowEdgesTitle: "Edges",
    workflowEntryInputsTitle: "Entry Inputs",
    workflowOutputArtifactsTitle: "Output Artifacts",
    workflowDatasetContractTitle: "Dataset Contract",
    workflowDatasetValuesTitle: "Dataset Values",
    workflowDatasetValueLabel: "Dataset Value",
    workflowDatasetSemanticTypeLabel: "Semantic Type",
    workflowDatasetEncodingLabel: "Encoding",
    workflowDatasetShapeLabel: "Shape",
    workflowDatasetAxesLabel: "Axis",
    workflowDatasetSchemaLabel: "Schema",
    workflowDatasetClassLabel: "Element Type",
    workflowDatasetNoneLabel: "This workflow has not published a dataset contract yet.",
    workflowDatasetDraftHint: "Edit dataset semantics locally here first so ports, edges, and cross-operator values line up before we add persistence.",
    workflowDatasetEditorTitle: "Dataset Editor",
    workflowDatasetValueSelectLabel: "Selected Dataset Value",
    workflowDatasetUnitLabel: "Unit",
    workflowDatasetMetadataLabel: "Metadata",
    workflowDatasetPortMappingsTitle: "Port Mappings",
    workflowDatasetEdgeMappingsTitle: "Edge Mappings",
    workflowDatasetDraftLocalLabel: "local draft",
    workflowDatasetUnassignedLabel: "Unassigned",
    workflowExportGraphLabel: "Export Graph JSON",
    workflowExportDatasetContractLabel: "Export Dataset JSON",
    workflowOperatorLabel: "Operator",
    workflowKindLabel: "Kind",
    workflowProgressLabel: "Progress",
    workflowCurrentNodeLabel: "Current node",
    workflowLatestSummaryLabel: "Latest summary",
    workflowOpenRunLabel: "Open run",
    workflowRunsEmpty: "No workflow runs yet.",
    projectManagePage: "Manage",
    projectExchangePage: "Exchange",
    modelSavedPage: "Saved",
    modelVersionsPage: "Versions",
    studyDomain: "Physics domain",
    noDomainStudies: "No studies are published in this domain yet.",
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
    result: "Result",
    actions: "Actions",
    details: "Details",
    controls: "Controls",
    controlsSetupPage: "Setup",
    controlsReviewPage: "Review",
    studyTypeLabel: "Study Type",
    modelStudyPage: "Study",
    modelOverviewPage: "Overview",
    modelStudioPage: "Studio",
    modelMaterialsPage: "Materials",
    modelGeneratePage: "Generate",
    workspaceStudyHint: "Choose the study family, physics domain, and solver-ready setup here.",
    workspaceStudioHint: "Build and edit nodes, members, boundaries, and geometry here.",
    workspaceMaterialsHint: "Keep starter material values and reusable material choices together here.",
    workspaceGenerateHint: "Use generators and parametric helpers to create repeatable models here.",
    workspaceBrowseHint: "Browse objects, highlights, and result-linked selections here.",
    settings: "Settings",
    scripts: "Scripts",
    settingsConfigHint: "Theme, language, routing, access tokens, and language packs stay here.",
    settingsScriptsHint: "WASM Python, macro recording, and action catalogs stay here.",
    assistant: "Assistant",
    config: "Config",
    runtime: "Runtime",
    data: "Data",
    packs: "Language packs",
    workspace: "Workspace",
    routing: "Routing",
    access: "Access",
    stack: "Stack",
    security: "Security",
    agents: "Agents",
    audit: "Audit",
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
    assistantOpenSample: "Open the official sample for this study",
    assistantOpenSampleHint: "If you are still learning this study family, start with the closest official sample before editing your own model.",
    assistantReviewControls: "Review supports, loads, and materials",
    assistantReviewControlsHint: "Before the first run, check the study controls and confirm the basic physical story makes sense.",
    assistantReviewReport: "Review the current result report",
    assistantReviewReportHint: "Open the report first, then read hotspots, summary metrics, and result-field meaning before changing the model.",
    assistantExportResult: "Export the current result data",
    assistantExportResultHint: "Export CSV once the result looks plausible so you have a stable trail for comparison and review.",
    assistantPromptExplain: "Explain this study to a beginner",
    assistantPromptMaterial: "Help me choose starter material values",
    assistantPromptBoundary: "Help me choose supports and loads",
    assistantPromptResults: "Help me read these result fields",
    assistantLauncherHint: "Need a guide?",
    assistantFloatingTitle: "Ask the workbench",
    assistantFloatingSubtitle: "Keep the main workspace clear, then pull a guide in only when you need one.",
    assistantOpen: "Open assistant",
    assistantClose: "Close assistant",
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
    languagePacksTitle: "Installed language packs",
    languagePacksHint: "Import local JSON packs now, and keep this page ready for future remote pack delivery.",
    languagePacksEmptyLabel: "No custom language packs are installed yet.",
    languagePackName: "Name",
    languagePackVersion: "Version",
    languagePackSourceImported: "Imported",
    languagePackSourceDownloaded: "Downloaded",
    languagePackDownloadTemplate: "Download template",
    languagePackExportInstalled: "Export installed pack",
    languagePackImport: "Import pack",
    languagePackRemove: "Remove",
    languagePackCatalogTitle: "Future catalog",
    languagePackCatalogHint: "These slots stay visible now so future remote pack download can plug into the same surface.",
    languagePackCatalogAction: "Coming soon",
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
    controlPlaneTokenHelp:
      "Used for token-protected control-plane deployments; sent as x-kyuubiki-token to /api/v1 requests.",
    controlPlaneTokenPlaceholder: "Optional control-plane token",
    clusterTokenHelp:
      "Used for agent register/heartbeat/remove cluster routes; falls back to the control-plane token when empty.",
    clusterTokenPlaceholder: "Optional cluster-only token",
    directMeshTokenHelp:
      "Used for token-protected direct mesh routes; sent to /api/direct-mesh requests.",
    directMeshTokenPlaceholder: "Optional direct-mesh token",
    exportDatabase: "Export database snapshot",
    runtimeSecurityFooter:
      "Runtime security state comes from /api/health; frontend tokens stay only in the current browser session.",
    securityAudit: "Security audit",
    auditSessionLabel:
      "Shows the control-plane persisted automation and assistant event stream, including Workbench assistant, scripting, and Hub assistant actions.",
    auditWindow: "Window",
    auditSource: "Source",
    auditRisk: "Risk",
    auditStatus: "Status",
    auditAction: "Action",
    auditSummaryTitle: "Event summary",
    auditTrendTitle: "Event trend",
    auditTrendEmptyLabel: "No trend data is available for the current window.",
    auditSourceStatusTitle: "Source × status",
    auditStudyFacetTitle: "Study facets",
    auditProjectFacetTitle: "Project facets",
    auditModelVersionFacetTitle: "Version facets",
    auditFacetEmptyLabel: "No facets are available for the current window.",
    auditRefreshLabel: "Refresh events",
    auditExportLabel: "Export bundle",
    auditExportCsvLabel: "Export CSV",
    auditWindowOptions: { all: "All time", h1: "Last 1 hour", h24: "Last 24 hours", d7: "Last 7 days", d30: "Last 30 days" },
    auditSourceOptions: { all: "All", assistant: "Assistant", hubAssistant: "Hub assistant", script: "Script" },
    auditRiskOptions: { all: "All", low: "Low", sensitive: "Sensitive", high: "High", destructive: "Destructive" },
    auditStatusOptions: { all: "All", prompted: "Prompted", confirmed: "Confirmed", cancelled: "Cancelled", completed: "Completed", failed: "Failed" },
    adminJobs: "Jobs",
    adminResults: "Results",
    adminBrowsePage: "Browse",
    adminEditPage: "Edit",
    selectRecord: "Select a record to inspect or edit.",
    adminMessage: "Message",
    adminProjectId: "Project ID",
    adminModelVersionId: "Model version",
    adminCaseId: "Simulation case",
    saveRecord: "Save record",
    deleteRecord: "Delete record",
    exportRecord: "Export record",
    applyRecordContext: "Apply record context",
    openLinkedProject: "Open linked project",
    openLinkedVersion: "Open linked version",
    filterProject: "Filter project",
    filterVersion: "Filter version",
    useCurrentProject: "Use current project",
    useCurrentVersion: "Use current version",
    clearFilters: "Clear filters",
    resultPayload: "Result JSON",
    resultSaved: "Result record updated.",
    resultDeleted: "Result record deleted.",
    jobSaved: "Job record updated.",
    jobDeleted: "Job record deleted.",
    recordContextApplied: "Applied the record context to the workbench.",
    linkedProjectOpened: "Opened the linked project context.",
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
    languages: { en: "English", zh: "中文", ja: "日本語", es: "Español" },
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
    maxAxialForce: "Max axial force",
    maxShearForce: "Max shear force",
    displacementMagnitude: "Displacement magnitude",
    fixRz: "Fix Rz",
    momentZ: "Moment Z",
    rotationZ: "Rotation Z",
    frameElements: "Frame members",
    memberEndForces: "Member end forces",
    momentOfInertia: "Moment of inertia",
    sectionModulus: "Section modulus",
    distributedLoadY: "Distributed load Y",
    temperatureDelta: "Temperature delta",
    temperature: "Temperature",
    averageTemperature: "Average temperature",
    maxTemperature: "Max temperature",
    conductivity: "Conductivity",
    fixTemperature: "Fix temperature",
    heatLoad: "Heat load",
    temperatureGradientX: "Temperature gradient X",
    temperatureGradientY: "Temperature gradient Y",
    thermalIntent: "Thermal intent",
    thermalBoundary: "Thermal boundary",
    prescribedTemperatureNodes: "prescribed-T nodes",
    sourceNodes: "source nodes",
    heatedNodes: "heated nodes",
    gradientMembers: "gradient members",
    restrainedSupports: "restrained supports",
    thermalMembers: "thermal members",
    conductionField: "conduction field",
    heatSourceField: "heat-source field",
    nodalTemperatureRise: "nodal temperature rise",
    memberTemperatureGradient: "member temperature gradient",
    thermalBarResponse: "restrained bar response",
    thermalBeamResponse: "beam thermal response",
    thermalFrameResponse: "frame thermal response",
    thermalTrussResponse: "truss thermal response",
    thermoelasticPlaneResponse: "thermoelastic plane response",
    maxHeatFlux: "Max heat flux",
    heatFluxX: "Heat flux X",
    heatFluxY: "Heat flux Y",
    thermalCurvature: "Thermal curvature",
    thermalStrain: "Thermal strain",
    mechanicalStrain: "Mechanical strain",
    totalStrain: "Total strain",
    thermalExpansion: "Thermal expansion",
    bendingStress: "Max bending stress",
    combinedStress: "Combined stress",
    maxMoment: "Max moment",
    maxTorque: "Max torque",
    torsionStress: "Torsion stress",
    torqueZ: "Torque Z",
    torsionHint: "Shaft studies focus on twist, torque transmission, and shear stress along the axis.",
    maxRotation: "Max rotation",
    sortBy: "Sort by",
    shearForce: "Shear force",
    forceI: "Axial @ i",
    shearI: "Shear @ i",
    momentI: "Moment @ i",
    forceJ: "Axial @ j",
    shearJ: "Shear @ j",
    momentJ: "Moment @ j",
    principalStress1: "Principal stress 1",
    principalStress2: "Principal stress 2",
    maxInPlaneShear: "Max in-plane shear",
    currentField: "Current field",
    planeHotspots: "Top hot elements",
    topN: "Top N",
    exportHotspots: "Export hotspots",
    memberForceTable: "Member force table",
    elementHeatTable: "Element heat table",
    exportMemberForces: "Export member forces",
    planeResultLegend: "Fill: von Mises · Overlay: deformed shape",
    planeViewVonMises: "von Mises",
    planeViewPrincipal1: "Principal 1",
    planeViewMaxShear: "Max shear",
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
    projectHeatToThermo: "Use temperatures in thermo study",
    projectHeatToThermoAction: "Mapped heat result into thermo study",
    projectedHeatToThermo: "Mapped the heat result into a thermo-mechanical study. Review supports and materials before solving.",
    noSavedModels: "No saved models in this project yet.",
    noVersions: "No saved versions yet.",
    defaultProject: "Workspace",
    projectCreated: "Project created.",
    projectUpdated: "Project updated.",
    projectDeleted: "Project deleted.",
    projectRequired: "Create or select a project before saving models.",
    projectExported: "Project bundle exported.",
    projectExportedPartial: "Project bundle exported with partial data. Some models or results could not be fetched in time.",
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
    tabs: { summary: "Summary", controls: "Controls", tools: "Tools", tree: "Browse", jobs: "Jobs", results: "Results", models: "Models", projects: "Projects", samples: "Samples" },
    generate: "Generate",
    generatePanel: "Generate Panel",
    download: "Download JSON",
    saveForSolver: "Use current model",
    bays: "Bays",
    height: "Height (m)",
    divisionsX: "Divisions X",
    divisionsY: "Divisions Y",
    nodeTable: "Object browser",
    dragNode: "Selected node",
    noNodeSelected: "No node selected",
    loadCase: "Load case",
    modelStudioHint: "2D truss and 2D plane studies can be edited directly here. 3D studies can already be imported, solved, and reviewed.",
    frameEditorHint: "Frame studies are ready to import, solve, inspect, and review here. Rich member editing is the next step.",
    spaceStudioHint: "Use orbit, pan, and zoom to navigate the space frame. 3D nodes and members can be selected and edited numerically.",
    sourceModel: "Source model",
    createdAt: "Created",
    updatedAt: "Updated",
    hasResult: "Result",
    yes: "Yes",
    no: "No",
    jobWorkspaceTitle: "Job Workspace",
    jobWorkspaceHint: "Track solver runs first, then drill into a specific record only when you need the details.",
    resultWorkspaceTitle: "Results Workspace",
    resultWorkspaceHint: "Open the newest solved run here, then move into reports, exports, and linked records.",
    modelWorkspaceTitle: "Model Workspace",
    modelWorkspaceHint: "Keep saved studies and reusable versions together so you can reopen or branch them quickly.",
    openLatestJob: "Open latest job",
    openLatestResult: "Open latest result",
    openLatestModel: "Open latest model",
    waitingJobs: "Waiting result",
    readyResults: "Ready results",
    savedCount: "Saved models",
    versionCount: "Versions",
    dragToEdit: "Drag to edit",
    historyLoaded: "Loaded a persisted study from history.",
    modelDownloaded: "Model JSON downloaded.",
    generatedModel: "Generated a parametric truss model.",
    planeElements: "Plane elements",
    thickness: "Thickness",
    poisson: "Poisson ratio",
    poissonRatio: "Poisson ratio",
    sampleLibrary: "Sample Library",
    objectTree: "Objects",
    geometry: "Geometry",
    results: "Results",
    summary: "Summary",
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
    requestTimedOut: "The request took too long. The workbench kept the last stable state.",
    pollingDetached: "Live polling paused to keep the UI responsive. Refresh the job state when ready.",
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
  };

const copyZh = {
    brand: brand.productName,
    shortTitle: brand.workbenchShortName ?? "Workbench",
    roleLabel: "分析工作台",
    title: "结构分析工作台",
    subtitle: "更正式的前端工作台，统一建模、编排与求解回看。",
    rail: { study: "研究", model: "工作区", workflow: "工作流", library: "历史", system: "系统" },
    sections: { study: "研究设置", model: "工作区", workflow: "工作流工作台", library: "任务历史", system: "系统" },
    kinds: {
      axial_bar_1d: "一维轴向杆",
      heat_bar_1d: "一维导热杆",
      heat_plane_triangle_2d: "二维导热三角形单元",
      heat_plane_quad_2d: "二维导热四边形单元",
      thermal_bar_1d: "一维热轴杆",
      thermal_beam_1d: "一维热梁",
      thermal_frame_2d: "二维热刚架",
      thermal_truss_2d: "二维热桁架",
      thermal_truss_3d: "三维热空间桁架",
      thermal_plane_triangle_2d: "二维热三角形单元",
      thermal_plane_quad_2d: "二维热四边形单元",
      spring_1d: "一维弹簧",
      spring_2d: "二维弹簧",
      spring_3d: "三维弹簧",
      beam_1d: "一维梁",
      torsion_1d: "一维扭转轴",
      truss_2d: "二维桁架",
      truss_3d: "三维空间桁架",
      plane_triangle_2d: "二维三角形单元",
      plane_quad_2d: "二维四边形单元",
      frame_2d: "二维刚架",
    },
    studyFamilies: {
      axialAndSprings: "轴向与弹簧",
      beamsAndFrames: "梁与刚架",
      trusses: "桁架",
      planes: "平面单元",
    },
    studyDomains: {
      mechanical: "纯力学",
      thermal: "纯热",
      thermoMechanical: "力-热",
    },
    familyHints: {
      axialAndSprings: "这一类主要看刚度、伸长和轴力传递路径。",
      beamsAndFrames: "这一类主要看弯曲、剪力、转角和构件端力。",
      trusses: "这一类主要看连接关系、轴力路径和整体稳定性。",
      planes: "这一类主要看应力场、单元斑块和热点区域。",
    },
    importModel: "导入模型",
    importHint: "导入 1D 或 2D 研究 JSON 模型。",
    sampleCatalogPage: "目录",
    sampleImportPage: "导入",
    workflowOverviewPage: "概览",
    workflowCatalogPage: "目录",
    workflowBuilderPage: "搭建",
    workflowRunsPage: "运行",
    workflowCatalogTitle: "工作流目录",
    workflowCatalogHint: "先在这里运行命名多算子工作流，再决定要不要把它接进更大的研究路径。",
    workflowOverviewHint: "把复合算子工作流放在独立工作面里，而不是继续藏在样板或结果页里。",
    workflowBuilderHint: "先看清 graph 的节点、桥接、提取和导出，再决定怎么把它接进更大的研究链。",
    workflowRunsHint: "在一个地方看命名工作流任务、当前节点和导出摘要。",
    workflowCatalogRefresh: "刷新工作流",
    workflowCatalogRun: "运行参考样板",
    workflowCatalogEmpty: "当前还没有发布命名工作流。",
    workflowCatalogReady: "工作流目录已就绪。",
    workflowCatalogQueued: "工作流任务已排队",
    workflowCatalogCompleted: "工作流已完成",
    workflowCatalogUnsupported: "这条工作流目前还没有内建参考输入。",
    workflowCatalogLoaded: "已加载工作流目录。",
    workflowCatalogFailed: "工作流运行失败。",
    workflowSelectForBuilder: "打开搭建面",
    workflowNoSelection: "先选择一条命名工作流，搭建面才会显示它的 graph。",
    workflowNodesTitle: "节点",
    workflowEdgesTitle: "连线",
    workflowEntryInputsTitle: "入口输入",
    workflowOutputArtifactsTitle: "输出产物",
    workflowDatasetContractTitle: "数据契约",
    workflowDatasetValuesTitle: "数据值",
    workflowDatasetValueLabel: "数据值",
    workflowDatasetSemanticTypeLabel: "语义类型",
    workflowDatasetEncodingLabel: "编码",
    workflowDatasetShapeLabel: "形状",
    workflowDatasetAxesLabel: "轴",
    workflowDatasetSchemaLabel: "Schema",
    workflowDatasetClassLabel: "元素类型",
    workflowDatasetNoneLabel: "这条工作流还没有发布 dataset contract。",
    workflowDatasetDraftHint: "先在这里本地调整 dataset 语义，让 port、edge 和跨算子值先对齐，后面再接持久化。",
    workflowDatasetEditorTitle: "数据编辑器",
    workflowDatasetValueSelectLabel: "当前数据值",
    workflowDatasetUnitLabel: "单位",
    workflowDatasetMetadataLabel: "元信息",
    workflowDatasetPortMappingsTitle: "端口映射",
    workflowDatasetEdgeMappingsTitle: "连线映射",
    workflowDatasetDraftLocalLabel: "本地草稿",
    workflowDatasetUnassignedLabel: "未分配",
    workflowExportGraphLabel: "导出 Graph JSON",
    workflowExportDatasetContractLabel: "导出 Dataset JSON",
    workflowOperatorLabel: "算子",
    workflowKindLabel: "类型",
    workflowProgressLabel: "进度",
    workflowCurrentNodeLabel: "当前节点",
    workflowLatestSummaryLabel: "最近摘要",
    workflowOpenRunLabel: "打开运行",
    workflowRunsEmpty: "还没有工作流运行记录。",
    projectManagePage: "管理",
    projectExchangePage: "交换",
    modelSavedPage: "已保存",
    modelVersionsPage: "版本",
    studyDomain: "物理类别",
    noDomainStudies: "这个大类里的研究还没正式开放。",
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
    result: "结果",
    actions: "动作",
    details: "细节",
    controls: "控制",
    controlsSetupPage: "设置",
    controlsReviewPage: "复查",
    studyTypeLabel: "研究类型",
    modelStudyPage: "研究",
    modelOverviewPage: "概览",
    modelStudioPage: "工作区",
    modelMaterialsPage: "材料",
    modelGeneratePage: "生成",
    workspaceStudyHint: "在这里选择研究族、物理域和可直接求解的 setup。",
    workspaceStudioHint: "在这里建模和编辑节点、单元、边界与几何。",
    workspaceMaterialsHint: "把起步材料参数和可复用材料选择集中放在这里。",
    workspaceGenerateHint: "在这里用生成器和参数化助手快速生成可重复模型。",
    workspaceBrowseHint: "在这里浏览对象、热点以及与结果联动的选择。",
    settings: "设置",
    scripts: "脚本",
    settingsConfigHint: "主题、语言、路由、访问令牌和语言包都放在这里。",
    settingsScriptsHint: "WASM Python、宏录制和动作目录都放在这里。",
    assistant: "助手",
    config: "配置",
    runtime: "运行时",
    data: "数据",
    packs: "语言包",
    workspace: "工作区",
    routing: "路由",
    access: "访问",
    stack: "栈",
    security: "安全",
    agents: "代理",
    audit: "审计",
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
    assistantOpenSample: "打开这个研究的官方样板",
    assistantOpenSampleHint: "如果你还在熟悉这个研究家族，先从最接近的官方样板开始，再改成自己的模型会更稳。",
    assistantReviewControls: "先检查约束、载荷和材料",
    assistantReviewControlsHint: "第一次运行前，先确认 study controls 里的物理设定讲得通，再去求解。",
    assistantReviewReport: "先看当前结果报告",
    assistantReviewReportHint: "先看 summary、热点和结果场含义，再决定要不要继续改模型。",
    assistantExportResult: "导出当前结果数据",
    assistantExportResultHint: "如果这次结果基本可信，先导出一份 CSV，后面更方便对比和复查。",
    assistantPromptExplain: "用小白能懂的话解释这个研究",
    assistantPromptMaterial: "帮我选一套起步材料参数",
    assistantPromptBoundary: "帮我判断支撑和载荷怎么设",
    assistantPromptResults: "帮我解释这些结果字段怎么读",
    assistantLauncherHint: "需要带路吗？",
    assistantFloatingTitle: "问问工作台",
    assistantFloatingSubtitle: "先把主工作面留干净，需要时再把助手拉出来。",
    assistantOpen: "打开助手",
    assistantClose: "关闭助手",
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
    languagePacksTitle: "已安装语言包",
    languagePacksHint: "现在先支持导入本地 JSON 语言包，这个页面也会为将来的远程下载入口保留位置。",
    languagePacksEmptyLabel: "当前还没有安装自定义语言包。",
    languagePackName: "名称",
    languagePackVersion: "版本",
    languagePackSourceImported: "本地导入",
    languagePackSourceDownloaded: "远程下载",
    languagePackDownloadTemplate: "下载模板",
    languagePackExportInstalled: "导出当前语言包",
    languagePackImport: "导入语言包",
    languagePackRemove: "移除",
    languagePackCatalogTitle: "未来目录",
    languagePackCatalogHint: "这些槽位现在先作为预留，后面接远程语言包下载时会复用同一块界面。",
    languagePackCatalogAction: "即将支持",
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
    controlPlaneTokenHelp: "用于带 Token 的 control-plane 部署；会作为 x-kyuubiki-token 附加到 /api/v1 请求。",
    controlPlaneTokenPlaceholder: "可选的控制面令牌",
    clusterTokenHelp: "用于 agent 注册、心跳和摘除这类集群路由；未填写时会回退到控制面令牌。",
    clusterTokenPlaceholder: "可选的集群专用令牌",
    directMeshTokenHelp: "用于带 Token 的 direct mesh 路由；会附加到 /api/direct-mesh 请求。",
    directMeshTokenPlaceholder: "可选的直连网格令牌",
    exportDatabase: "导出数据库快照",
    runtimeSecurityFooter: "运行中的安全状态来自 /api/health；前端输入的 token 只保存在当前浏览器会话里。",
    securityAudit: "安全审计",
    auditSessionLabel: "这里展示控制面持久化的自动化与助手事件流，包括 Workbench 助手、脚本和 Hub 助手的高风险动作。",
    auditWindow: "时间窗",
    auditSource: "来源",
    auditRisk: "风险",
    auditStatus: "状态",
    auditAction: "动作",
    auditSummaryTitle: "事件摘要",
    auditTrendTitle: "时间趋势",
    auditTrendEmptyLabel: "当前窗口下还没有趋势数据。",
    auditSourceStatusTitle: "来源 × 状态",
    auditStudyFacetTitle: "Study 分面",
    auditProjectFacetTitle: "Project 分面",
    auditModelVersionFacetTitle: "Version 分面",
    auditFacetEmptyLabel: "当前窗口下没有可用分面。",
    auditRefreshLabel: "刷新事件",
    auditExportLabel: "导出分析包",
    auditExportCsvLabel: "导出 CSV",
    auditWindowOptions: { all: "全部时间", h1: "最近 1 小时", h24: "最近 24 小时", d7: "最近 7 天", d30: "最近 30 天" },
    auditSourceOptions: { all: "全部", assistant: "助手", hubAssistant: "Hub 助手", script: "脚本" },
    auditRiskOptions: { all: "全部", low: "低", sensitive: "敏感", high: "高", destructive: "高风险" },
    auditStatusOptions: { all: "全部", prompted: "待确认", confirmed: "已确认", cancelled: "已取消", completed: "已执行", failed: "失败" },
    adminJobs: "任务",
    adminResults: "结果",
    adminBrowsePage: "浏览",
    adminEditPage: "编辑",
    selectRecord: "选择一条记录后即可查看或编辑。",
    adminMessage: "消息",
    adminProjectId: "项目 ID",
    adminModelVersionId: "模型版本",
    adminCaseId: "仿真工况",
    saveRecord: "保存记录",
    deleteRecord: "删除记录",
    exportRecord: "导出记录",
    applyRecordContext: "应用记录上下文",
    openLinkedProject: "打开关联项目",
    openLinkedVersion: "打开关联版本",
    filterProject: "筛选项目",
    filterVersion: "筛选版本",
    useCurrentProject: "使用当前项目",
    useCurrentVersion: "使用当前版本",
    clearFilters: "清空筛选",
    resultPayload: "结果 JSON",
    resultSaved: "结果记录已更新。",
    resultDeleted: "结果记录已删除。",
    jobSaved: "任务记录已更新。",
    jobDeleted: "任务记录已删除。",
    recordContextApplied: "已将记录上下文应用到工作台。",
    linkedProjectOpened: "已打开关联项目上下文。",
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
    languages: { en: "English", zh: "中文", ja: "日本語", es: "Español" },
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
    maxAxialForce: "最大轴力",
    maxShearForce: "最大剪力",
    displacementMagnitude: "位移模",
    fixRz: "约束 Rz",
    momentZ: "节点弯矩",
    rotationZ: "转角 Rz",
    frameElements: "刚架杆件",
    memberEndForces: "杆端内力",
    momentOfInertia: "惯性矩",
    sectionModulus: "截面模量",
    distributedLoadY: "均布载荷 Y",
    temperatureDelta: "温升",
    temperature: "温度",
    averageTemperature: "平均温度",
    maxTemperature: "最高温度",
    conductivity: "导热系数",
    fixTemperature: "固定温度",
    heatLoad: "热载荷",
    temperatureGradientX: "温差梯度 X",
    temperatureGradientY: "温差梯度 Y",
    thermalIntent: "热载荷意图",
    thermalBoundary: "热边界模式",
    prescribedTemperatureNodes: "固定温度节点",
    sourceNodes: "热源节点",
    heatedNodes: "受温升节点",
    gradientMembers: "温差梯度杆件",
    restrainedSupports: "受约束支撑",
    thermalMembers: "热杆件",
    conductionField: "导热场",
    heatSourceField: "热源场",
    nodalTemperatureRise: "节点温升",
    memberTemperatureGradient: "杆件温差梯度",
    thermalBarResponse: "受约束热杆响应",
    thermalBeamResponse: "热梁响应",
    thermalFrameResponse: "热刚架响应",
    thermalTrussResponse: "热桁架响应",
    thermoelasticPlaneResponse: "热弹性平面响应",
    maxHeatFlux: "最大热流",
    heatFluxX: "热流 X",
    heatFluxY: "热流 Y",
    thermalCurvature: "热曲率",
    thermalStrain: "热应变",
    mechanicalStrain: "机械应变",
    totalStrain: "总应变",
    thermalExpansion: "热膨胀系数",
    bendingStress: "最大弯曲应力",
    combinedStress: "组合应力",
    maxMoment: "最大弯矩",
    maxTorque: "最大扭矩",
    torsionStress: "扭转应力",
    torqueZ: "扭矩 Z",
    torsionHint: "扭转轴研究主要看转角、扭矩传递和沿轴的剪应力。",
    maxRotation: "最大转角",
    sortBy: "排序方式",
    shearForce: "剪力",
    forceI: "i端轴力",
    shearI: "i端剪力",
    momentI: "i端弯矩",
    forceJ: "j端轴力",
    shearJ: "j端剪力",
    momentJ: "j端弯矩",
    principalStress1: "主应力 1",
    principalStress2: "主应力 2",
    maxInPlaneShear: "最大面内剪应力",
    currentField: "当前结果场",
    planeHotspots: "热点单元",
    topN: "热点数",
    exportHotspots: "导出热点",
    memberForceTable: "杆端内力表",
    elementHeatTable: "单元热量表",
    exportMemberForces: "导出杆端内力",
    planeResultLegend: "填色：von Mises · 叠加：变形后形状",
    planeViewVonMises: "von Mises",
    planeViewPrincipal1: "主应力 1",
    planeViewMaxShear: "最大剪应力",
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
    projectHeatToThermo: "将温度场用于力-热研究",
    projectHeatToThermoAction: "已把热结果映射到力-热研究",
    projectedHeatToThermo: "已将热结果映射到力-热研究。求解前请再检查约束和材料设置。",
    noSavedModels: "这个项目里还没有保存的模型。",
    noVersions: "还没有版本记录。",
    defaultProject: "工作区",
    projectCreated: "项目已创建。",
    projectUpdated: "项目已更新。",
    projectDeleted: "项目已删除。",
    projectRequired: "保存模型前请先创建或选择一个项目。",
    projectExported: "项目包已导出。",
    projectExportedPartial: "项目包已导出，但只包含部分数据；有些模型或结果未能及时读取。",
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
    tabs: { summary: "概览", controls: "控制", tools: "工具", tree: "浏览", jobs: "任务", results: "结果", models: "模型", projects: "项目", samples: "样板" },
    generate: "生成模型",
    generatePanel: "生成面板",
    download: "下载 JSON",
    saveForSolver: "使用当前模型",
    bays: "跨数",
    height: "高度 (m)",
    divisionsX: "X 分段",
    divisionsY: "Y 分段",
    nodeTable: "对象浏览",
    dragNode: "当前节点",
    noNodeSelected: "未选择节点",
    loadCase: "载荷工况",
    modelStudioHint: "当前建模页支持二维桁架和二维平面单元。三维研究已经支持导入、求解和结果回看。",
    frameEditorHint: "二维刚架现在已经支持导入、求解、检查和结果回看；更完整的构件编辑会继续补上。",
    spaceStudioHint: "通过旋转、平移和缩放来查看空间桁架。三维节点和杆件现在也可以选中并做数值编辑。",
    sourceModel: "来源模型",
    createdAt: "创建时间",
    updatedAt: "更新时间",
    hasResult: "结果",
    yes: "是",
    no: "否",
    jobWorkspaceTitle: "任务工作台",
    jobWorkspaceHint: "先盯 solver 运行，再在需要时钻进某一条任务记录看细节。",
    resultWorkspaceTitle: "结果工作台",
    resultWorkspaceHint: "这里先打开最新已完成结果，再继续进入报告、导出和关联记录。",
    modelWorkspaceTitle: "模型工作台",
    modelWorkspaceHint: "把保存过的研究和可复用版本放在一起，方便快速重开或分支。",
    openLatestJob: "打开最新任务",
    openLatestResult: "打开最新结果",
    openLatestModel: "打开最新模型",
    waitingJobs: "待出结果",
    readyResults: "已就绪结果",
    savedCount: "已存模型",
    versionCount: "版本数",
    dragToEdit: "拖拽编辑",
    historyLoaded: "已从历史记录加载持久化任务。",
    modelDownloaded: "模型 JSON 已下载。",
    generatedModel: "已生成参数化桁架模型。",
    planeElements: "平面单元",
    thickness: "厚度",
    poisson: "泊松比",
    poissonRatio: "泊松比",
    sampleLibrary: "样板库",
    objectTree: "对象",
    geometry: "几何",
    results: "结果",
    summary: "概览",
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
    requestTimedOut: "请求等待过久，工作台已保留最近一次稳定状态。",
    pollingDetached: "为了保持界面流畅，实时轮询已暂停；你可以稍后手动刷新任务状态。",
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
  };

const copy = {
  en: copyEn,
  zh: copyZh,
  ja: {
    ...copyEn,
    roleLabel: "解析シェル",
    title: "構造解析ワークベンチ",
    subtitle: "モデリング、オーケストレーション、結果レビューを一つにまとめた作業面です。",
    rail: { study: "解析", model: "ワークスペース", workflow: "ワークフロー", library: "履歴", system: "システム" },
    sections: { study: "解析設定", model: "ワークスペース", workflow: "ワークフロースタジオ", library: "ジョブ履歴", system: "システム" },
    studyFamilies: {
      axialAndSprings: "軸・ばね",
      beamsAndFrames: "梁・フレーム",
      trusses: "トラス",
      planes: "平面要素",
    },
    studyDomains: {
      mechanical: "力学",
      thermal: "熱",
      thermoMechanical: "熱応力",
    },
    importModel: "モデルを読み込む",
    importHint: "1D / 2D 解析用の JSON モデルを読み込みます。",
    sampleCatalogPage: "カタログ",
    sampleImportPage: "インポート",
    workflowOverviewPage: "概要",
    workflowCatalogPage: "カタログ",
    workflowBuilderPage: "ビルダー",
    workflowRunsPage: "実行",
    workflowCatalogTitle: "ワークフローカタログ",
    workflowCatalogHint: "より大きな study path に組み込む前に、命名済みの多算子 workflow をここで実行します。",
    workflowOverviewHint: "複合算子 workflow を専用作業面に集め、samples や results に埋め込まないようにします。",
    workflowBuilderHint: "graph のノード、bridge、extract、export を確認してから大きな study chain に接続します。",
    workflowRunsHint: "命名 workflow ジョブ、現在ノード、書き出し済み summary を一か所で追います。",
    workflowCatalogRefresh: "ワークフローを更新",
    workflowCatalogRun: "参照サンプルを実行",
    workflowCatalogEmpty: "まだ公開済みの命名 workflow はありません。",
    workflowCatalogReady: "ワークフローカタログの準備ができました。",
    workflowCatalogQueued: "ワークフロージョブを投入しました",
    workflowCatalogCompleted: "ワークフローが完了しました",
    workflowCatalogUnsupported: "この workflow にはまだ内蔵サンプル入力がありません。",
    workflowCatalogLoaded: "ワークフローカタログを読み込みました。",
    workflowCatalogFailed: "ワークフロー実行に失敗しました。",
    workflowSelectForBuilder: "ビルダーを開く",
    workflowNoSelection: "まず命名 workflow を選ぶと、ビルダーに graph が表示されます。",
    workflowNodesTitle: "ノード",
    workflowEdgesTitle: "エッジ",
    workflowEntryInputsTitle: "入口入力",
    workflowOutputArtifactsTitle: "出力アーティファクト",
    workflowDatasetContractTitle: "データセット契約",
    workflowDatasetValuesTitle: "データセット値",
    workflowDatasetValueLabel: "データセット値",
    workflowDatasetSemanticTypeLabel: "意味型",
    workflowDatasetEncodingLabel: "エンコード",
    workflowDatasetShapeLabel: "形状",
    workflowDatasetAxesLabel: "軸",
    workflowDatasetSchemaLabel: "スキーマ",
    workflowDatasetClassLabel: "要素型",
    workflowDatasetNoneLabel: "この workflow にはまだ dataset contract がありません。",
    workflowDatasetDraftHint: "まずここで dataset の意味付けをローカル草稿として調整し、port・edge・跨算子値を揃えてから永続化に進みます。",
    workflowDatasetEditorTitle: "データセットエディタ",
    workflowDatasetValueSelectLabel: "選択中のデータセット値",
    workflowDatasetUnitLabel: "単位",
    workflowDatasetMetadataLabel: "メタ情報",
    workflowDatasetPortMappingsTitle: "ポート対応",
    workflowDatasetEdgeMappingsTitle: "エッジ対応",
    workflowDatasetDraftLocalLabel: "ローカル草稿",
    workflowDatasetUnassignedLabel: "未割り当て",
    workflowExportGraphLabel: "Graph JSON を書き出す",
    workflowExportDatasetContractLabel: "Dataset JSON を書き出す",
    workflowOperatorLabel: "算子",
    workflowKindLabel: "種別",
    workflowProgressLabel: "進捗",
    workflowCurrentNodeLabel: "現在ノード",
    workflowLatestSummaryLabel: "最新サマリー",
    workflowOpenRunLabel: "実行を開く",
    workflowRunsEmpty: "まだ workflow 実行はありません。",
    projectManagePage: "管理",
    projectExchangePage: "入出力",
    modelSavedPage: "保存済み",
    modelVersionsPage: "バージョン",
    studyDomain: "物理ドメイン",
    noDomainStudies: "このドメインにはまだ公開済み study がありません。",
    jobWorkspaceTitle: "ジョブ作業面",
    jobWorkspaceHint: "まずは solver 実行を追い、必要になった時だけ個別ジョブに掘り下げます。",
    resultWorkspaceTitle: "結果作業面",
    resultWorkspaceHint: "ここでは最新の完了結果を開き、そこからレポートやエクスポートに進みます。",
    modelWorkspaceTitle: "モデル作業面",
    modelWorkspaceHint: "保存済み study と再利用用バージョンをまとめて、すぐに再開や分岐ができるようにします。",
    openLatestJob: "最新ジョブを開く",
    openLatestResult: "最新結果を開く",
    openLatestModel: "最新モデルを開く",
    waitingJobs: "結果待ち",
    readyResults: "利用可能な結果",
    savedCount: "保存済みモデル",
    versionCount: "バージョン数",
    overview: "概要",
    result: "結果",
    actions: "操作",
    details: "詳細",
    controls: "設定",
    controlsSetupPage: "編集",
    controlsReviewPage: "確認",
    studyTypeLabel: "解析タイプ",
    modelStudyPage: "解析",
    modelOverviewPage: "概要",
    modelStudioPage: "スタジオ",
    modelMaterialsPage: "材料",
    modelGeneratePage: "生成",
    workspaceStudyHint: "Study ファミリー、物理ドメイン、solver 向け setup をここで整えます。",
    workspaceStudioHint: "ノード、部材、境界、幾何の編集はここで行います。",
    workspaceMaterialsHint: "初期材料値と再利用する材料選択をここにまとめます。",
    workspaceGenerateHint: "ジェネレーターとパラメトリック補助で再現可能なモデルを作ります。",
    workspaceBrowseHint: "オブジェクト、ハイライト、結果に結び付く選択をここで参照します。",
    settings: "設定",
    scripts: "スクリプト",
    settingsConfigHint: "テーマ、言語、ルーティング、アクセストークン、言語パックをここで管理します。",
    settingsScriptsHint: "WASM Python、マクロ記録、アクションカタログをここで扱います。",
    assistant: "アシスタント",
    config: "構成",
    runtime: "ランタイム",
    data: "データ",
    packs: "言語パック",
    workspace: "ワークスペース",
    routing: "ルーティング",
    access: "アクセス",
    stack: "スタック",
    security: "セキュリティ",
    agents: "エージェント",
    audit: "監査",
    backend: "バックエンド",
    protocols: "プロトコル",
    dataAdmin: "データ管理",
    languagePacksTitle: "インストール済み言語パック",
    languagePacksHint: "今はローカル JSON パックの取り込みを先にサポートし、この画面を将来のリモート配布入口として残しておきます。",
    languagePacksEmptyLabel: "まだカスタム言語パックはありません。",
    languagePackName: "名称",
    languagePackVersion: "バージョン",
    languagePackSourceImported: "ローカル取込",
    languagePackSourceDownloaded: "リモート取得",
    languagePackDownloadTemplate: "テンプレートを出力",
    languagePackExportInstalled: "現在の言語パックを出力",
    languagePackImport: "言語パックを取込",
    languagePackRemove: "削除",
    languagePackCatalogTitle: "将来のカタログ",
    languagePackCatalogHint: "この欄は将来のリモート言語パック配布を同じ画面で扱えるように先に用意しています。",
    languagePackCatalogAction: "今後対応",
    adminJobs: "ジョブ",
    adminResults: "結果",
    adminBrowsePage: "参照",
    adminEditPage: "編集",
    themes: { linen: "リネン", marine: "マリン", graphite: "グラファイト" },
    languages: { en: "English", zh: "中文", ja: "日本語", es: "Español" },
    frontendModes: {
      orchestrated_gui: "オーケストレーション GUI",
      direct_mesh_gui: "ダイレクトメッシュ GUI",
    },
    directMeshStrategies: {
      healthiest: "最も健全なノード",
      first_reachable: "最初に到達可能なノード",
    },
    tabs: { summary: "概要", controls: "設定", tools: "ツール", tree: "参照", jobs: "ジョブ", results: "結果", models: "モデル", projects: "プロジェクト", samples: "サンプル" },
    ready: "準備完了",
    busy: "処理中",
    run: "解析を実行",
    running: "実行中...",
    historyEmpty: "まだジョブはありません。",
    refresh: "更新",
    online: "オンライン",
    offline: "オフライン",
    controlPlaneTokenHelp:
      "トークン保護された control-plane 配備向けです。/api/v1 リクエストに x-kyuubiki-token として送信されます。",
    controlPlaneTokenPlaceholder: "任意の control-plane トークン",
    clusterTokenHelp:
      "agent の登録・heartbeat・削除などの cluster ルートに使います。未入力なら control-plane token を使います。",
    clusterTokenPlaceholder: "任意の cluster 専用トークン",
    directMeshTokenHelp:
      "トークン保護された direct mesh ルート向けです。/api/direct-mesh リクエストに送信されます。",
    directMeshTokenPlaceholder: "任意の direct-mesh トークン",
    exportDatabase: "データベーススナップショットを出力",
    runtimeSecurityFooter:
      "実行中のセキュリティ状態は /api/health から取得され、フロントエンドのトークンは現在のブラウザセッションにのみ保持されます。",
    securityAudit: "セキュリティ監査",
    auditSessionLabel:
      "Workbench アシスタント、スクリプト、Hub アシスタントを含む control-plane 永続化イベント列を表示します。",
    auditWindow: "期間",
    auditSource: "ソース",
    auditRisk: "リスク",
    auditStatus: "状態",
    auditAction: "操作",
    auditSummaryTitle: "イベント要約",
    auditTrendTitle: "時間推移",
    auditTrendEmptyLabel: "現在の期間ではトレンドデータがありません。",
    auditSourceStatusTitle: "ソース × 状態",
    auditStudyFacetTitle: "Study ファセット",
    auditProjectFacetTitle: "Project ファセット",
    auditModelVersionFacetTitle: "Version ファセット",
    auditFacetEmptyLabel: "現在の期間では利用できるファセットがありません。",
    auditRefreshLabel: "イベントを更新",
    auditExportLabel: "分析バンドルを出力",
    auditExportCsvLabel: "CSV を出力",
    auditWindowOptions: { all: "全期間", h1: "直近 1 時間", h24: "直近 24 時間", d7: "直近 7 日", d30: "直近 30 日" },
    auditSourceOptions: { all: "すべて", assistant: "アシスタント", hubAssistant: "Hub アシスタント", script: "スクリプト" },
    auditRiskOptions: { all: "すべて", low: "低", sensitive: "機微", high: "高", destructive: "破壊的" },
    auditStatusOptions: { all: "すべて", prompted: "確認待ち", confirmed: "確認済み", cancelled: "取消済み", completed: "完了", failed: "失敗" },
    assistantLauncherHint: "ガイドが必要ですか？",
    assistantFloatingTitle: "ワークベンチに質問",
    assistantFloatingSubtitle: "作業面は軽く保ち、必要なときだけガイドを開きます。",
    assistantOpen: "アシスタントを開く",
    assistantClose: "アシスタントを閉じる",
    modelName: "モデル",
    material: "材料",
    load: "荷重",
    support: "拘束",
    viewport: "ビュー",
    report: "レポート",
    summary: "概要",
    messages: "メッセージ",
    nodeTable: "オブジェクト参照",
    objectTree: "オブジェクト",
  },
} as const;

type WorkbenchCopy = typeof copyEn;
const copyByLanguage: Record<Language, WorkbenchCopy> = {
  en: copy.en,
  zh: copy.zh,
  ja: copy.ja,
  es: {
    ...copy.en,
    language: "Idioma",
    languages: {
      ...copy.en.languages,
      es: "Español",
    },
  },
};

function humanizeSolverFailure(message: string | null | undefined, languageCopy: WorkbenchCopy) {
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

function formatJobMessage(job: JobEnvelope["job"] | null, fallback: string, languageCopy: WorkbenchCopy) {
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

  const localized = language === "zh" ? labels.zh : labels.en;
  return localized[value as keyof typeof localized] ?? localized.custom;
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

function isThermalBar1dResult(value: unknown): value is ThermalBar1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isHeatBar1dResult(value: unknown): value is HeatBar1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature" in value &&
    "max_heat_flux" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isHeatPlaneTriangle2dResult(value: unknown): value is HeatPlaneTriangle2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature" in value &&
    "max_heat_flux" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as HeatPlaneTriangle2dResult).elements) &&
    (value as HeatPlaneTriangle2dResult).elements.every((element) => !("node_l" in element))
  );
}

function isHeatPlaneQuad2dResult(value: unknown): value is HeatPlaneQuad2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature" in value &&
    "max_heat_flux" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as HeatPlaneQuad2dResult).elements) &&
    (value as HeatPlaneQuad2dResult).elements.some((element) => "node_l" in element)
  );
}

function isThermalTruss2dResult(value: unknown): value is ThermalTruss2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as ThermalTruss2dResult).nodes) &&
    !(value as ThermalTruss2dResult).nodes.some((node) => "z" in node)
  );
}

function isThermalTruss3dResult(value: unknown): value is ThermalTruss3dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_axial_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as ThermalTruss3dResult).nodes) &&
    (value as ThermalTruss3dResult).nodes.some((node) => "z" in node)
  );
}

function isTrussResult(value: unknown): value is Truss2dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && !("tip_displacement" in value) && Array.isArray((value as Truss2dResult).nodes) && !(value as Truss2dResult).nodes.some((node) => "z" in node);
}

function isBeam1dResult(value: unknown): value is Beam1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Beam1dResult).nodes) &&
    !(value as Beam1dResult).nodes.some((node) => "y" in node)
  );
}

function isThermalBeam1dResult(value: unknown): value is ThermalBeam1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_gradient" in value &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as ThermalBeam1dResult).nodes) &&
    !(value as ThermalBeam1dResult).nodes.some((node) => "y" in node)
  );
}

function isTorsion1dResult(value: unknown): value is Torsion1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_torque" in value &&
    "max_rotation" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Torsion1dResult).nodes)
  );
}

function isSpring1dResult(value: unknown): value is Spring1dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring1dResult).nodes)
  );
}

function isSpring2dResult(value: unknown): value is Spring2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring2dResult).nodes) &&
    (value as Spring2dResult).nodes.some((node) => "y" in node)
  );
}

function isSpring3dResult(value: unknown): value is Spring3dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_force" in value &&
    "nodes" in value &&
    "elements" in value &&
    Array.isArray((value as Spring3dResult).nodes) &&
    (value as Spring3dResult).nodes.some((node) => "z" in node)
  );
}

function isFrame2dResult(value: unknown): value is Frame2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_rotation" in value &&
    "max_moment" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isThermalFrame2dResult(value: unknown): value is ThermalFrame2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "max_temperature_delta" in value &&
    "max_temperature_gradient" in value &&
    "max_rotation" in value &&
    "nodes" in value &&
    "elements" in value
  );
}

function isPlaneResult(
  value: unknown,
): value is
  | PlaneTriangle2dResult
  | PlaneQuad2dResult
  | ThermalPlaneTriangle2dResult
  | ThermalPlaneQuad2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "elements" in value &&
    "nodes" in value &&
    "input" in value &&
    Array.isArray(
      (
        value as
          | PlaneTriangle2dResult
          | PlaneQuad2dResult
          | ThermalPlaneTriangle2dResult
          | ThermalPlaneQuad2dResult
      ).elements,
    ) &&
    (
      value as
        | PlaneTriangle2dResult
        | PlaneQuad2dResult
        | ThermalPlaneTriangle2dResult
        | ThermalPlaneQuad2dResult
    ).elements.some((element) => "node_k" in element)
  );
}

function ensureBeamModelMaterials<T extends { materials?: Array<{ id: string }>; elements: Array<{ material_id?: string | undefined }> }>(
  model: T,
  materialValue: string,
): T {
  const existingMaterials = model.materials?.length
    ? model.materials
    : [createMaterialDefinition(materialValue, 1, { id: "mat-1" })];
  const defaultMaterialId = existingMaterials[0]?.id ?? "mat-1";
  return {
    ...model,
    materials: existingMaterials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

function ensureFrameModelMaterials(model: Frame2dJobInput, materialValue: string): Frame2dJobInput {
  const existingMaterials = model.materials?.length
    ? model.materials
    : [createMaterialDefinition(materialValue, 1, { id: "mat-1" })];
  const defaultMaterialId = existingMaterials[0]?.id ?? "mat-1";
  return {
    ...model,
    materials: existingMaterials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

function buildThermalBarFromHeatResult(
  sourceModel: HeatBar1dJobInput,
  result: HeatBar1dResult,
  fallbackModel: ThermalBar1dJobInput,
): ThermalBar1dJobInput {
  const sameTopology =
    fallbackModel.nodes.length === sourceModel.nodes.length &&
    fallbackModel.elements.length === sourceModel.elements.length;
  const defaultElement = defaultThermalBar1d.elements[0];

  return {
    project_id: sourceModel.project_id,
    model_version_id: sourceModel.model_version_id,
    nodes: sourceModel.nodes.map((node, index) => ({
      id: node.id,
      x: node.x,
      fix_x: sameTopology ? fallbackModel.nodes[index]?.fix_x ?? node.fix_temperature : node.fix_temperature,
      load_x: sameTopology ? fallbackModel.nodes[index]?.load_x ?? 0 : 0,
      temperature_delta: result.nodes[index]?.temperature ?? node.temperature ?? 0,
    })),
    elements: sourceModel.elements.map((element, index) => ({
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      area: element.area,
      youngs_modulus: sameTopology ? fallbackModel.elements[index]?.youngs_modulus ?? defaultElement.youngs_modulus : defaultElement.youngs_modulus,
      thermal_expansion: sameTopology ? fallbackModel.elements[index]?.thermal_expansion ?? defaultElement.thermal_expansion : defaultElement.thermal_expansion,
    })),
  };
}

function buildThermalPlaneTriangleFromHeatResult(
  sourceModel: HeatPlaneTriangle2dJobInput,
  result: HeatPlaneTriangle2dResult,
  fallbackModel: ThermalPlaneTriangle2dJobInput,
  fallbackMaterial: string,
): ThermalPlaneTriangle2dJobInput {
  const sameTopology =
    fallbackModel.nodes.length === sourceModel.nodes.length &&
    fallbackModel.elements.length === sourceModel.elements.length;
  const fallbackMaterials = sameTopology && fallbackModel.materials?.length ? fallbackModel.materials : defaultThermalPlaneTriangle.materials;

  return ensurePlaneModelMaterials(
    {
      materials: fallbackMaterials,
      project_id: sourceModel.project_id,
      model_version_id: sourceModel.model_version_id,
      nodes: sourceModel.nodes.map((node, index) => ({
        id: node.id,
        x: node.x,
        y: node.y,
        fix_x: sameTopology ? fallbackModel.nodes[index]?.fix_x ?? node.fix_temperature : node.fix_temperature,
        fix_y: sameTopology ? fallbackModel.nodes[index]?.fix_y ?? node.fix_temperature : node.fix_temperature,
        load_x: sameTopology ? fallbackModel.nodes[index]?.load_x ?? 0 : 0,
        load_y: sameTopology ? fallbackModel.nodes[index]?.load_y ?? 0 : 0,
        temperature_delta: result.nodes[index]?.temperature ?? node.temperature ?? 0,
      })),
      elements: sourceModel.elements.map((element, index) => {
        const template = sameTopology ? fallbackModel.elements[index] : defaultThermalPlaneTriangle.elements[Math.min(index, defaultThermalPlaneTriangle.elements.length - 1)];
        return {
          id: element.id,
          node_i: element.node_i,
          node_j: element.node_j,
          node_k: element.node_k,
          thickness: element.thickness,
          youngs_modulus: template?.youngs_modulus ?? defaultThermalPlaneTriangle.elements[0].youngs_modulus,
          poisson_ratio: template?.poisson_ratio ?? defaultThermalPlaneTriangle.elements[0].poisson_ratio,
          thermal_expansion: template?.thermal_expansion ?? defaultThermalPlaneTriangle.elements[0].thermal_expansion,
          material_id: template?.material_id ?? fallbackMaterials?.[0]?.id,
        };
      }),
    },
    fallbackMaterial,
  ) as ThermalPlaneTriangle2dJobInput;
}

function buildThermalPlaneQuadFromHeatResult(
  sourceModel: HeatPlaneQuad2dJobInput,
  result: HeatPlaneQuad2dResult,
  fallbackModel: ThermalPlaneQuad2dJobInput,
  fallbackMaterial: string,
): ThermalPlaneQuad2dJobInput {
  const sameTopology =
    fallbackModel.nodes.length === sourceModel.nodes.length &&
    fallbackModel.elements.length === sourceModel.elements.length;
  const fallbackMaterials = sameTopology && fallbackModel.materials?.length ? fallbackModel.materials : defaultThermalPlaneQuad.materials;

  return ensurePlaneModelMaterials(
    {
      materials: fallbackMaterials,
      project_id: sourceModel.project_id,
      model_version_id: sourceModel.model_version_id,
      nodes: sourceModel.nodes.map((node, index) => ({
        id: node.id,
        x: node.x,
        y: node.y,
        fix_x: sameTopology ? fallbackModel.nodes[index]?.fix_x ?? node.fix_temperature : node.fix_temperature,
        fix_y: sameTopology ? fallbackModel.nodes[index]?.fix_y ?? node.fix_temperature : node.fix_temperature,
        load_x: sameTopology ? fallbackModel.nodes[index]?.load_x ?? 0 : 0,
        load_y: sameTopology ? fallbackModel.nodes[index]?.load_y ?? 0 : 0,
        temperature_delta: result.nodes[index]?.temperature ?? node.temperature ?? 0,
      })),
      elements: sourceModel.elements.map((element, index) => {
        const template = sameTopology ? fallbackModel.elements[index] : defaultThermalPlaneQuad.elements[Math.min(index, defaultThermalPlaneQuad.elements.length - 1)];
        return {
          id: element.id,
          node_i: element.node_i,
          node_j: element.node_j,
          node_k: element.node_k,
          node_l: element.node_l,
          thickness: element.thickness,
          youngs_modulus: template?.youngs_modulus ?? defaultThermalPlaneQuad.elements[0].youngs_modulus,
          poisson_ratio: template?.poisson_ratio ?? defaultThermalPlaneQuad.elements[0].poisson_ratio,
          thermal_expansion: template?.thermal_expansion ?? defaultThermalPlaneQuad.elements[0].thermal_expansion,
          material_id: template?.material_id ?? fallbackMaterials?.[0]?.id,
        };
      }),
    },
    fallbackMaterial,
  ) as ThermalPlaneQuad2dJobInput;
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

function buildDisplayFrameNodes(
  model: Frame2dJobInput,
  result: Frame2dResult | null,
  windowNodes?: Frame2dResult["nodes"],
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

function buildDisplayThermalFrameNodes(
  model: ThermalFrame2dJobInput,
  result: ThermalFrame2dResult | null,
  windowNodes?: ThermalFrame2dResult["nodes"],
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

function buildDisplayBeamNodes(
  model: Beam1dJobInput,
  result: Beam1dResult | null,
  windowNodes?: Beam1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;

  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: 0,
      uy: node.uy,
      fix_x: false,
      fix_y: model.nodes[node.index]?.fix_y ?? false,
      load_x: 0,
      load_y: model.nodes[node.index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: false,
    fix_y: node.fix_y,
    load_x: 0,
    load_y: node.load_y,
  }));
}

function buildDisplayThermalBeamNodes(
  model: ThermalBeam1dJobInput,
  result: ThermalBeam1dResult | null,
  windowNodes?: ThermalBeam1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;

  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: 0,
      uy: node.uy,
      fix_x: false,
      fix_y: model.nodes[node.index]?.fix_y ?? false,
      load_x: 0,
      load_y: model.nodes[node.index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: false,
    fix_y: node.fix_y,
    load_x: 0,
    load_y: node.load_y,
  }));
}

function buildDisplayTorsionNodes(
  model: Torsion1dJobInput,
  result: Torsion1dResult | null,
  windowNodes?: Torsion1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;

  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: 0,
      uy: 0,
      fix_x: false,
      fix_y: true,
      load_x: 0,
      load_y: model.nodes[node.index]?.torque_z ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: false,
    fix_y: true,
    load_x: 0,
    load_y: node.torque_z,
  }));
}

function buildDisplaySpringNodes(
  model: Spring1dJobInput,
  result: Spring1dResult | null,
  windowNodes?: Spring1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;

  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: node.ux,
      uy: 0,
      fix_x: model.nodes[node.index]?.fix_x ?? false,
      fix_y: true,
      load_x: model.nodes[node.index]?.load_x ?? 0,
      load_y: 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: node.fix_x,
    fix_y: true,
    load_x: node.load_x,
    load_y: 0,
  }));
}

function buildDisplayBeamElements(
  model: Beam1dJobInput,
  result: Beam1dResult | null,
  windowElements?: Beam1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: 0,
      stress: element.max_bending_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      max_bending_stress: element.max_bending_stress,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}

function buildDisplayThermalBeamElements(
  model: ThermalBeam1dJobInput,
  result: ThermalBeam1dResult | null,
  windowElements?: ThermalBeam1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.thermal_curvature,
      stress: element.max_bending_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      temperature_gradient_y: element.temperature_gradient_y,
      thermal_curvature: element.thermal_curvature,
      max_bending_stress: element.max_bending_stress,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      temperature_gradient_y: element.temperature_gradient_y,
      thermal_curvature: 0,
      max_bending_stress: 0,
      shear_force_i: 0,
      moment_i: 0,
      shear_force_j: 0,
      moment_j: 0,
      material_id: element.material_id,
    };
  });
}

function buildDisplayTorsionElements(
  model: Torsion1dJobInput,
  result: Torsion1dResult | null,
  windowElements?: Torsion1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.twist,
      stress: element.shear_stress,
      axial_force: Math.abs(element.torque),
      max_bending_stress: element.shear_stress,
      moment_i: element.torque,
      moment_j: element.torque,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      max_bending_stress: 0,
      moment_i: 0,
      moment_j: 0,
    };
  });
}

function buildDisplaySpringElements(
  model: Spring1dJobInput,
  result: Spring1dResult | null,
  windowElements?: Spring1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.extension,
      stress: element.force,
      axial_force: element.force,
      axial_stress: element.force,
      axial_force_i: element.force,
      axial_force_j: element.force,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      axial_stress: 0,
      axial_force_i: 0,
      axial_force_j: 0,
    };
  });
}

function buildDisplayThermalBarNodes(
  model: ThermalBar1dJobInput,
  result: ThermalBar1dResult | null,
  windowNodes?: ThermalBar1dResult["nodes"],
): DisplayTrussNode[] {
  const source = windowNodes ?? result?.nodes;
  return (source ?? model.nodes.map((node, index) => ({ index, id: node.id, x: node.x, ux: 0, temperature_delta: node.temperature_delta }))).map((node, index) => ({
    index: node.index ?? index,
    id: model.nodes[node.index ?? index]?.id ?? node.id,
    x: model.nodes[node.index ?? index]?.x ?? node.x,
    y: 0,
    ux: node.ux ?? 0,
    uy: 0,
    fix_x: model.nodes[node.index ?? index]?.fix_x ?? false,
    fix_y: true,
    load_x: model.nodes[node.index ?? index]?.load_x ?? 0,
    load_y: 0,
  }));
}

function buildDisplayThermalBarElements(
  model: ThermalBar1dJobInput,
  result: ThermalBar1dResult | null,
  windowElements?: ThermalBar1dResult["elements"],
): DisplayTrussElement[] {
  const source = windowElements ?? result?.elements;
  return (source ??
    model.elements.map((element, index) => ({
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(model.nodes[element.node_j].x - model.nodes[element.node_i].x),
      stress: 0,
      axial_force: 0,
      average_temperature_delta: 0,
    } as ThermalBar1dResult["elements"][number]))).map((element, index) => ({
    index: element.index ?? index,
    id: model.elements[element.index ?? index]?.id ?? element.id,
    node_i: element.node_i,
    node_j: element.node_j,
    length: element.length,
    strain: element.total_strain ?? 0,
    stress: element.stress,
    axial_force: element.axial_force,
    axial_stress: element.stress,
    axial_force_i: element.axial_force,
    shear_force_i: 0,
    moment_i: 0,
    axial_force_j: element.axial_force,
    shear_force_j: 0,
    moment_j: 0,
  }));
}

function buildDisplayHeatBarNodes(
  model: HeatBar1dJobInput,
  result: HeatBar1dResult | null,
  windowNodes?: HeatBar1dResult["nodes"],
): DisplayTrussNode[] {
  const source = windowNodes ?? result?.nodes;
  return (
    source ??
    model.nodes.map((node, index) => ({
      index,
      id: node.id,
      x: node.x,
      temperature: node.temperature,
      heat_load: node.heat_load,
    }))
  ).map((node, index) => ({
    index: node.index ?? index,
    id: model.nodes[node.index ?? index]?.id ?? node.id,
    x: model.nodes[node.index ?? index]?.x ?? node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: model.nodes[node.index ?? index]?.fix_temperature ?? false,
    fix_y: true,
    load_x: model.nodes[node.index ?? index]?.heat_load ?? 0,
    load_y: 0,
    temperature_delta: node.temperature ?? model.nodes[node.index ?? index]?.temperature ?? 0,
  }));
}

function buildDisplayHeatBarElements(
  model: HeatBar1dJobInput,
  result: HeatBar1dResult | null,
  windowElements?: HeatBar1dResult["elements"],
): DisplayTrussElement[] {
  const source = windowElements ?? result?.elements;
  return (
    source ??
    model.elements.map((element, index) => ({
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(model.nodes[element.node_j].x - model.nodes[element.node_i].x),
      average_temperature: 0,
      temperature_gradient: 0,
      heat_flux: 0,
    }))
  ).map((element, index) => ({
    index: element.index ?? index,
    id: model.elements[element.index ?? index]?.id ?? element.id,
    node_i: element.node_i,
    node_j: element.node_j,
    length: element.length,
    strain: element.temperature_gradient ?? 0,
    stress: element.heat_flux ?? 0,
    axial_force: element.heat_flux ?? 0,
    axial_stress: element.heat_flux ?? 0,
    average_temperature_delta: element.average_temperature ?? 0,
    axial_force_i: element.heat_flux ?? 0,
    shear_force_i: 0,
    moment_i: 0,
    axial_force_j: element.heat_flux ?? 0,
    shear_force_j: 0,
    moment_j: 0,
  }));
}

function buildDisplayThermalTrussNodes(
  model: ThermalTruss2dJobInput,
  result: ThermalTruss2dResult | null,
  windowNodes?: ThermalTruss2dResult["nodes"],
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

function buildDisplayThermalTrussElements(
  model: ThermalTruss2dJobInput,
  result: ThermalTruss2dResult | null,
  windowElements?: ThermalTruss2dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.total_strain,
      stress: element.stress,
      axial_force: element.axial_force,
      axial_stress: element.stress,
      axial_force_i: element.axial_force,
      shear_force_i: 0,
      moment_i: 0,
      axial_force_j: element.axial_force,
      shear_force_j: 0,
      moment_j: 0,
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
      axial_stress: 0,
      axial_force_i: 0,
      shear_force_i: 0,
      moment_i: 0,
      axial_force_j: 0,
      shear_force_j: 0,
      moment_j: 0,
      material_id: element.material_id,
    };
  });
}

function buildDisplaySpring2dNodes(
  model: Spring2dJobInput,
  result: Spring2dResult | null,
  windowNodes?: Spring2dResult["nodes"],
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

function buildDisplaySpring2dElements(
  model: Spring2dJobInput,
  result: Spring2dResult | null,
  windowElements?: Spring2dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.extension,
      stress: element.force,
      axial_force: element.force,
      axial_stress: element.force,
      axial_force_i: element.force,
      axial_force_j: element.force,
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
      axial_stress: 0,
      axial_force_i: 0,
      axial_force_j: 0,
    };
  });
}

function buildDisplaySpring3dNodes(
  model: Spring3dJobInput,
  result: Spring3dResult | null,
  windowNodes?: Spring3dResult["nodes"],
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

function buildDisplaySpring3dElements(
  model: Spring3dJobInput,
  result: Spring3dResult | null,
  windowElements?: Spring3dResult["elements"],
): DisplayTruss3dElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.extension,
      stress: element.force,
      axial_force: element.force,
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
    };
  });
}

function buildDisplayThermalTruss3dNodes(
  model: ThermalTruss3dJobInput,
  result: ThermalTruss3dResult | null,
  windowNodes?: ThermalTruss3dResult["nodes"],
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

function buildDisplayThermalTruss3dElements(
  model: ThermalTruss3dJobInput,
  result: ThermalTruss3dResult | null,
  windowElements?: ThermalTruss3dResult["elements"],
): DisplayTruss3dElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.total_strain,
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

function buildDisplayFrameElements(
  model: Frame2dJobInput,
  result: Frame2dResult | null,
  windowElements?: Frame2dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: 0,
      stress: element.max_combined_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      axial_stress: element.axial_stress,
      max_bending_stress: element.max_bending_stress,
      max_combined_stress: element.max_combined_stress,
      axial_force_i: element.axial_force_i,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      axial_force_j: element.axial_force_j,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
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

function buildDisplayThermalFrameElements(
  model: ThermalFrame2dJobInput,
  result: ThermalFrame2dResult | null,
  windowElements?: ThermalFrame2dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;

  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.thermal_curvature,
      stress: element.max_combined_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      axial_stress: element.axial_stress,
      max_bending_stress: element.max_bending_stress,
      max_combined_stress: element.max_combined_stress,
      axial_force_i: element.axial_force_i,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      axial_force_j: element.axial_force_j,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
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
  setResult: Dispatch<
    SetStateAction<
      | AxialBarResult
      | HeatBar1dResult
      | HeatPlaneTriangle2dResult
      | HeatPlaneQuad2dResult
      | ThermalBar1dResult
      | ThermalBeam1dResult
      | ThermalFrame2dResult
      | ThermalTruss2dResult
      | ThermalTruss3dResult
      | ThermalPlaneTriangle2dResult
      | ThermalPlaneQuad2dResult
      | Spring1dResult
      | Spring2dResult
      | Spring3dResult
      | Beam1dResult
      | Torsion1dResult
      | Truss2dResult
      | Truss3dResult
      | PlaneTriangle2dResult
      | PlaneQuad2dResult
      | Frame2dResult
      | null
    >
  >,
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
  languageCopy: WorkbenchCopy,
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
  languageCopy: WorkbenchCopy,
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

function planeResultFieldValue(
  element: {
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
    thermal_strain?: number;
    mechanical_strain_x?: number;
    mechanical_strain_y?: number;
  },
  field: PlaneResultField,
) {
  if (field === "average_temperature") {
    return Math.abs(element.average_temperature ?? 0);
  }
  if (field === "average_temperature_delta") {
    return Math.abs(element.average_temperature_delta ?? 0);
  }
  if (field === "temperature_gradient_x") {
    return Math.abs(element.temperature_gradient_x ?? 0);
  }
  if (field === "temperature_gradient_y") {
    return Math.abs(element.temperature_gradient_y ?? 0);
  }
  if (field === "heat_flux_x") {
    return Math.abs(element.heat_flux_x ?? 0);
  }
  if (field === "heat_flux_y") {
    return Math.abs(element.heat_flux_y ?? 0);
  }
  if (field === "heat_flux_magnitude") {
    return Math.abs(element.heat_flux_magnitude ?? 0);
  }
  if (field === "thermal_strain") {
    return Math.abs(element.thermal_strain ?? 0);
  }
  if (field === "mechanical_strain") {
    return Math.max(Math.abs(element.mechanical_strain_x ?? 0), Math.abs(element.mechanical_strain_y ?? 0));
  }
  if (field === "principal_stress_1") {
    return Math.abs(element.principal_stress_1 ?? 0);
  }
  if (field === "max_in_plane_shear") {
    return Math.abs(element.max_in_plane_shear ?? 0);
  }
  return Math.abs(element.von_mises ?? 0);
}

function lineResultFieldValue(
  element: {
    axial_stress?: number;
    max_bending_stress?: number;
    max_combined_stress?: number;
    shear_force_i?: number;
    shear_force_j?: number;
    moment_i?: number;
    moment_j?: number;
    average_temperature_delta?: number;
    temperature_gradient_y?: number;
    thermal_curvature?: number;
  },
  field: LineResultField,
) {
  if (field === "axial_stress") {
    return Math.abs(element.axial_stress ?? 0);
  }
  if (field === "average_temperature_delta") {
    return Math.abs(element.average_temperature_delta ?? 0);
  }
  if (field === "temperature_gradient_y") {
    return Math.abs(element.temperature_gradient_y ?? 0);
  }
  if (field === "thermal_curvature") {
    return Math.abs(element.thermal_curvature ?? 0);
  }
  if (field === "max_bending_stress") {
    return Math.abs(element.max_bending_stress ?? 0);
  }
  if (field === "shear_force") {
    return Math.max(Math.abs(element.shear_force_i ?? 0), Math.abs(element.shear_force_j ?? 0));
  }
  if (field === "moment") {
    return Math.max(Math.abs(element.moment_i ?? 0), Math.abs(element.moment_j ?? 0));
  }
  return Math.abs(element.max_combined_stress ?? 0);
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

function formatPeerStatus(status: string | undefined, languageCopy: WorkbenchCopy) {
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
      current === copy.en.defaultModel || current === copy.zh.defaultModel || current === copy.ja.defaultModel
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
  const [selectedAdminJobId, setSelectedAdminJobId] = useState<string | null>(null);
  const [selectedAdminResultJobId, setSelectedAdminResultJobId] = useState<string | null>(null);
  const [adminFilterProjectId, setAdminFilterProjectId] = useState("");
  const [adminFilterModelVersionId, setAdminFilterModelVersionId] = useState("");
  const [scriptActionLog, setScriptActionLog] = useState<WorkbenchScriptActionLogEntry[]>([]);
  const [assistantTransactions, setAssistantTransactions] = useState<AssistantTransactionEntry[]>([]);
  const [securityAuditLog, setSecurityAuditLog] = useState<WorkbenchSecurityAuditEntry[]>([]);
  const [securityEventRecords, setSecurityEventRecords] = useState<SecurityEventRecord[]>([]);
  const [scriptRecordingMode, setScriptRecordingMode] = useState(false);
  const [securityEventWindowFilter, setSecurityEventWindowFilter] = useState<SecurityEventWindow>("24h");
  const [securityEventSourceFilter, setSecurityEventSourceFilter] = useState("");
  const [securityEventRiskFilter, setSecurityEventRiskFilter] = useState("");
  const [securityEventStatusFilter, setSecurityEventStatusFilter] = useState("");
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
  const chunkScrollFrameRef = useRef<number | null>(null);
  const chunkScrollLeftRef = useRef(0);
  const chunkScrollDirectionRef = useRef<-1 | 0 | 1>(0);
  const chunkCacheRef = useRef<Map<string, ResultChunkPayload<Record<string, unknown>>>>(new Map());
  const healthRefreshSeqRef = useRef(0);
  const jobHistoryRefreshSeqRef = useRef(0);
  const resultRefreshSeqRef = useRef(0);
  const projectRefreshSeqRef = useRef(0);
  const securityEventsRefreshSeqRef = useRef(0);
  const versionsRefreshSeqRef = useRef(0);
  const jobPollTokenRef = useRef(0);
  const activeLanguagePack = useMemo(
    () => languagePacks.find((pack) => pack.language === language) ?? null,
    [language, languagePacks],
  );
  const t = useMemo(
    () => mergeLanguagePack<WorkbenchCopy>(copyByLanguage[language], activeLanguagePack?.overrides ?? null),
    [activeLanguagePack?.overrides, language],
  );
  const languagePackCatalogRows = useMemo(
    () => [
      { id: "fr-preview", language: "fr", name: "French preview", status: language === "zh" ? "预留远程下载入口" : language === "ja" ? "将来のリモート配布枠" : "Reserved for future remote delivery" },
      { id: "de-preview", language: "de", name: "German preview", status: language === "zh" ? "预留远程下载入口" : language === "ja" ? "将来のリモート配布枠" : "Reserved for future remote delivery" },
    ],
    [language],
  );

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
      : isHeatBar1dResult(result)
        ? "heat_bar_1d"
      : isHeatPlaneQuad2dResult(result)
        ? "heat_plane_quad_2d"
      : isHeatPlaneTriangle2dResult(result)
        ? "heat_plane_triangle_2d"
      : isThermalBar1dResult(result)
        ? "thermal_bar_1d"
        : isThermalBeam1dResult(result)
          ? "thermal_beam_1d"
        : isThermalTruss2dResult(result)
          ? "thermal_truss_2d"
          : isThermalTruss3dResult(result)
            ? "thermal_truss_3d"
        : isTruss3dResult(result)
          ? "truss_3d"
        : isSpring1dResult(result)
          ? "spring_1d"
        : isSpring2dResult(result)
          ? "spring_2d"
        : isSpring3dResult(result)
          ? "spring_3d"
        : isThermalBeam1dResult(result)
          ? "thermal_beam_1d"
        : isBeam1dResult(result)
          ? "beam_1d"
        : isTorsion1dResult(result)
          ? "torsion_1d"
        : isFrame2dResult(result)
          ? "frame_2d"
        : studyKind === "plane_quad_2d"
          ? "plane_quad_2d"
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
      } catch (error) {
        if (!cancelled && error instanceof Error && error.message.startsWith("request timed out:")) {
          setMessage(t.requestTimedOut);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [canvasViewportWidth, frontendRuntimeMode, job?.job_id, result, resultWindowLimit, resultWindowOffset, t.requestTimedOut]);

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
    setSecurityAuditLog(readSecurityAuditLog());
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
    writeSecurityAuditLog(securityAuditLog);
  }, [securityAuditLog]);

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
    void refreshHealth();
    void refreshJobHistory();
    void refreshResults();
    void refreshProjects(true);
    void refreshSecurityEvents();
  }, []);

  useEffect(() => {
    void refreshHealth();
  }, [frontendRuntimeMode, directMeshEndpointsText, directMeshSelectionMode]);

  useEffect(() => {
    void refreshSecurityEvents();
  }, [
    securityEventWindowFilter,
    securityEventSourceFilter,
    securityEventRiskFilter,
    securityEventStatusFilter,
    securityEventActionFilter,
  ]);

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
    const refreshSeq = ++healthRefreshSeqRef.current;

    if (frontendRuntimeMode === "direct_mesh_gui") {
      try {
        const endpoints = parseDirectMeshEndpoints(directMeshEndpointsText);
        const nextDirect = await fetchDirectMeshAgents(endpoints);
        const directMethods = [...new Set(
          nextDirect.agents.flatMap((agent) => agent.descriptor?.protocol?.methods ?? []),
        )];

        if (refreshSeq !== healthRefreshSeqRef.current) return;

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
        if (refreshSeq !== healthRefreshSeqRef.current) return;
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

      if (refreshSeq !== healthRefreshSeqRef.current) return;

      setHealth(nextHealth);
      setProtocolAgents(nextProtocolAgents.agents);
    } catch {
      if (refreshSeq !== healthRefreshSeqRef.current) return;
      setHealth(null);
      setProtocolAgents([]);
    }
  }

  async function refreshJobHistory() {
    const refreshSeq = ++jobHistoryRefreshSeqRef.current;

    try {
      const payload = await fetchJobHistory();
      if (refreshSeq !== jobHistoryRefreshSeqRef.current) return;
      setJobHistory(payload.jobs);
      setSelectedAdminJobId((current) =>
        current && payload.jobs.some((entry) => entry.job_id === current) ? current : payload.jobs[0]?.job_id ?? null,
      );
    } catch {
      if (refreshSeq !== jobHistoryRefreshSeqRef.current) return;
      setJobHistory([]);
      setSelectedAdminJobId(null);
    }
  }

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
    const refreshSeq = ++resultRefreshSeqRef.current;

    try {
      const payload = await fetchResults();
      if (refreshSeq !== resultRefreshSeqRef.current) return;
      setResultRecords(payload.results);
      setSelectedAdminResultJobId((current) =>
        current && payload.results.some((entry) => entry.job_id === current) ? current : payload.results[0]?.job_id ?? null,
      );
    } catch {
      if (refreshSeq !== resultRefreshSeqRef.current) return;
      setResultRecords([]);
      setSelectedAdminResultJobId(null);
    }
  }

  async function refreshProjects(bootstrap = false) {
    const refreshSeq = ++projectRefreshSeqRef.current;

    try {
      const payload = await fetchProjects();
      let nextProjects = payload.projects;

      if (bootstrap && nextProjects.length === 0) {
        const created = await createProject({ name: copy.en.defaultProject, description: "Local workspace" });
        nextProjects = [created.project];
      }

      if (refreshSeq !== projectRefreshSeqRef.current) return;

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
      if (refreshSeq !== projectRefreshSeqRef.current) return;
      setProjects([]);
    }
  }

  async function refreshSecurityEvents() {
    const refreshSeq = ++securityEventsRefreshSeqRef.current;

    try {
      const occurredAfter =
        securityEventWindowFilter && SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter]
          ? new Date(Date.now() - SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter]).toISOString()
          : undefined;
      const payload = await fetchSecurityEvents({
        occurred_after: occurredAfter,
        source: securityEventSourceFilter || undefined,
        risk: securityEventRiskFilter || undefined,
        status: securityEventStatusFilter || undefined,
        action: securityEventActionFilter || undefined,
        limit: 120,
      });
      if (refreshSeq !== securityEventsRefreshSeqRef.current) return;
      setSecurityEventRecords(payload.events);
    } catch {
      if (refreshSeq !== securityEventsRefreshSeqRef.current) return;
      setSecurityEventRecords([]);
    }
  }

  async function refreshVersions(modelId: string) {
    const refreshSeq = ++versionsRefreshSeqRef.current;

    try {
      const payload = await fetchModelVersions(modelId);
      if (refreshSeq !== versionsRefreshSeqRef.current) return;
      setModelVersions(payload.versions);
    } catch {
      if (refreshSeq !== versionsRefreshSeqRef.current) return;
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
    jobPollTokenRef.current += 1;

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
              : studyKind === "heat_bar_1d"
                ? await createDirectMeshSolve<HeatBar1dResult>(
                    "heat_bar_1d",
                    resolveHeatBar1dJobInput(heatBarModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "heat_plane_quad_2d"
                ? await createDirectMeshSolve<HeatPlaneQuad2dResult>(
                    "heat_plane_quad_2d",
                    resolveHeatPlaneQuad2dJobInput(heatPlaneModel as HeatPlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "heat_plane_triangle_2d"
                ? await createDirectMeshSolve<HeatPlaneTriangle2dResult>(
                    "heat_plane_triangle_2d",
                    resolveHeatPlaneTriangle2dJobInput(heatPlaneModel as HeatPlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "thermal_bar_1d"
                ? await createDirectMeshSolve<ThermalBar1dResult>(
                    "thermal_bar_1d",
                    resolveThermalBar1dJobInput(thermalBarModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "thermal_beam_1d"
                ? await createDirectMeshSolve<ThermalBeam1dResult>(
                    "thermal_beam_1d",
                    resolveThermalBeam1dJobInput(thermalBeamModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "thermal_frame_2d"
                ? await createDirectMeshSolve<ThermalFrame2dResult>(
                    "thermal_frame_2d",
                    resolveThermalFrame2dJobInput(thermalFrameModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "thermal_truss_2d"
                ? await createDirectMeshSolve<ThermalTruss2dResult>(
                    "thermal_truss_2d",
                    resolveThermalTruss2dJobInput(thermalTrussModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "thermal_truss_3d"
                ? await createDirectMeshSolve<ThermalTruss3dResult>(
                    "thermal_truss_3d",
                    resolveThermalTruss3dJobInput(thermalTruss3dModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
              : studyKind === "spring_1d"
                ? await createDirectMeshSolve<Spring1dResult>(
                    "spring_1d",
                    resolveSpring1dJobInput(springModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
                : studyKind === "spring_2d"
                  ? await createDirectMeshSolve<Spring2dResult>(
                      "spring_2d",
                      resolveSpring2dJobInput(spring2dModel) as unknown as Record<string, unknown>,
                      endpoints,
                      directMeshSelectionMode,
                    )
              : studyKind === "spring_3d"
                    ? await createDirectMeshSolve<Spring3dResult>(
                        "spring_3d",
                        resolveSpring3dJobInput(spring3dModel) as unknown as Record<string, unknown>,
                        endpoints,
                        directMeshSelectionMode,
                      )
                : studyKind === "torsion_1d"
                  ? await createDirectMeshSolve<Torsion1dResult>(
                      "torsion_1d",
                      resolveTorsion1dJobInput(torsionModel) as unknown as Record<string, unknown>,
                      endpoints,
                      directMeshSelectionMode,
                    )
                : studyKind === "beam_1d"
                ? await createDirectMeshSolve<Beam1dResult>(
                    "beam_1d",
                    resolveBeam1dJobInput(beamModel) as unknown as Record<string, unknown>,
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
                  : studyKind === "frame_2d"
                    ? await createDirectMeshSolve<Frame2dResult>(
                        "frame_2d",
                        resolveFrame2dJobInput(frameModel) as unknown as Record<string, unknown>,
                        endpoints,
                        directMeshSelectionMode,
                      )
                  : studyKind === "thermal_plane_quad_2d"
                    ? await createDirectMeshSolve<ThermalPlaneQuad2dResult>(
                        "thermal_plane_quad_2d",
                        resolveThermalPlaneQuad2dJobInput(planeModel as ThermalPlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                        endpoints,
                        directMeshSelectionMode,
                      )
                  : studyKind === "plane_quad_2d"
                    ? await createDirectMeshSolve<PlaneQuad2dResult>(
                        "plane_quad_2d",
                        resolvePlaneQuad2dJobInput(planeModel as PlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                        endpoints,
                        directMeshSelectionMode,
                      )
                    : studyKind === "thermal_plane_triangle_2d"
                      ? await createDirectMeshSolve<ThermalPlaneTriangle2dResult>(
                          "thermal_plane_triangle_2d",
                          resolveThermalPlaneTriangle2dJobInput(planeModel as ThermalPlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
                          endpoints,
                          directMeshSelectionMode,
                        )
                    : await createDirectMeshSolve<PlaneTriangle2dResult>(
                        "plane_triangle_2d",
                        resolvePlaneTriangle2dJobInput(planeModel as PlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
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
            : studyKind === "heat_bar_1d"
              ? await createHeatBar1dJob(resolveHeatBar1dJobInput({ ...heatBarModel, ...jobContext }))
            : studyKind === "heat_plane_quad_2d"
              ? await createHeatPlaneQuad2dJob(resolveHeatPlaneQuad2dJobInput({ ...(heatPlaneModel as HeatPlaneQuad2dJobInput), ...jobContext }))
            : studyKind === "heat_plane_triangle_2d"
              ? await createHeatPlaneTriangle2dJob(resolveHeatPlaneTriangle2dJobInput({ ...(heatPlaneModel as HeatPlaneTriangle2dJobInput), ...jobContext }))
            : studyKind === "thermal_bar_1d"
              ? await createThermalBar1dJob(resolveThermalBar1dJobInput({ ...thermalBarModel, ...jobContext }))
            : studyKind === "thermal_beam_1d"
              ? await createThermalBeam1dJob(resolveThermalBeam1dJobInput({ ...thermalBeamModel, ...jobContext }))
            : studyKind === "thermal_frame_2d"
              ? await createThermalFrame2dJob(resolveThermalFrame2dJobInput({ ...thermalFrameModel, ...jobContext }))
            : studyKind === "thermal_truss_2d"
              ? await createThermalTruss2dJob(resolveThermalTruss2dJobInput({ ...thermalTrussModel, ...jobContext }))
            : studyKind === "thermal_truss_3d"
              ? await createThermalTruss3dJob(resolveThermalTruss3dJobInput({ ...thermalTruss3dModel, ...jobContext }))
            : studyKind === "spring_1d"
              ? await createSpring1dJob(resolveSpring1dJobInput({ ...springModel, ...jobContext }))
            : studyKind === "spring_2d"
              ? await createSpring2dJob(resolveSpring2dJobInput({ ...spring2dModel, ...jobContext }))
            : studyKind === "spring_3d"
              ? await createSpring3dJob(resolveSpring3dJobInput({ ...spring3dModel, ...jobContext }))
            : studyKind === "torsion_1d"
              ? await createTorsion1dJob(resolveTorsion1dJobInput({ ...torsionModel, ...jobContext }))
            : studyKind === "beam_1d"
              ? await createBeam1dJob(resolveBeam1dJobInput({ ...beamModel, ...jobContext }))
            : studyKind === "truss_2d"
              ? await createTruss2dJob(resolveTruss2dJobInput({ ...trussModel, ...jobContext }))
              : studyKind === "truss_3d"
                ? await createTruss3dJob(resolveTruss3dJobInput({ ...truss3dModel, ...jobContext }))
                : studyKind === "frame_2d"
                  ? await createFrame2dJob(resolveFrame2dJobInput({ ...frameModel, ...jobContext }))
                : studyKind === "thermal_plane_quad_2d"
                  ? await createThermalPlaneQuad2dJob(
                      resolveThermalPlaneQuad2dJobInput({ ...(planeModel as ThermalPlaneQuad2dJobInput), ...jobContext }),
                  )
                : studyKind === "plane_quad_2d"
                  ? await createPlaneQuad2dJob(
                      resolvePlaneQuad2dJobInput({ ...(planeModel as PlaneQuad2dJobInput), ...jobContext }),
                  )
                : studyKind === "thermal_plane_triangle_2d"
                  ? await createThermalPlaneTriangle2dJob(
                      resolveThermalPlaneTriangle2dJobInput({ ...(planeModel as ThermalPlaneTriangle2dJobInput), ...jobContext }),
                  )
                : await createPlaneTriangle2dJob(
                    resolvePlaneTriangle2dJobInput({ ...(planeModel as PlaneTriangle2dJobInput), ...jobContext }),
                  );

        setJob(created.job);
        await refreshJobHistory();
        await pollJob(created.job.job_id, studyKind);
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
    const pollToken = ++jobPollTokenRef.current;
    let consecutiveErrors = 0;

    for (let attempt = 0; attempt < 40; attempt += 1) {
      if (pollToken !== jobPollTokenRef.current) return;

      try {
        const payload =
          kind === "axial_bar_1d"
            ? await fetchJobStatus<AxialBarResult>(jobId)
            : kind === "heat_bar_1d"
              ? await fetchJobStatus<HeatBar1dResult>(jobId)
            : kind === "thermal_bar_1d"
              ? await fetchJobStatus<ThermalBar1dResult>(jobId)
            : kind === "thermal_beam_1d"
              ? await fetchJobStatus<ThermalBeam1dResult>(jobId)
            : kind === "thermal_truss_2d"
              ? await fetchJobStatus<ThermalTruss2dResult>(jobId)
            : kind === "thermal_truss_3d"
              ? await fetchJobStatus<ThermalTruss3dResult>(jobId)
            : kind === "spring_1d"
              ? await fetchJobStatus<Spring1dResult>(jobId)
            : kind === "spring_2d"
              ? await fetchJobStatus<Spring2dResult>(jobId)
            : kind === "spring_3d"
              ? await fetchJobStatus<Spring3dResult>(jobId)
            : kind === "torsion_1d"
              ? await fetchJobStatus<Torsion1dResult>(jobId)
            : kind === "beam_1d"
              ? await fetchJobStatus<Beam1dResult>(jobId)
            : kind === "truss_2d"
              ? await fetchJobStatus<Truss2dResult>(jobId)
              : kind === "truss_3d"
                ? await fetchJobStatus<Truss3dResult>(jobId)
                : kind === "frame_2d"
                  ? await fetchJobStatus<Frame2dResult>(jobId)
                : await fetchJobStatus<PlaneTriangle2dResult | PlaneQuad2dResult>(jobId);

        if (pollToken !== jobPollTokenRef.current) return;

        consecutiveErrors = 0;
        setJob(payload.job);

        if (payload.result) {
          setResult(payload.result);
        }

        setMessage(formatJobMessage(payload.job, `${jobId} ${payload.job.status}`, t));

        if (payload.job.status === "completed" || payload.job.status === "failed" || payload.job.status === "cancelled") {
          await refreshJobHistory();
          return;
        }
      } catch (error) {
        if (pollToken !== jobPollTokenRef.current) return;

        consecutiveErrors += 1;

        if (consecutiveErrors >= 3) {
          throw error;
        }

        await new Promise((resolve) => window.setTimeout(resolve, 500));
        continue;
      }

      await new Promise((resolve) => window.setTimeout(resolve, 250));
    }

    if (pollToken === jobPollTokenRef.current) {
      setMessage(t.pollingDetached);
      await refreshJobHistory();
    }
  };

  const openHistoryJob = (jobId: string) => {
    jobPollTokenRef.current += 1;

    startTransition(async () => {
      try {
        const payload = await fetchJobStatus<
          AxialBarResult | HeatBar1dResult | HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult | ThermalBar1dResult | ThermalBeam1dResult | ThermalTruss2dResult | ThermalTruss3dResult | Spring1dResult | Spring2dResult | Spring3dResult | Beam1dResult | Torsion1dResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | PlaneQuad2dResult | Frame2dResult | ThermalFrame2dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult | WorkflowGraphJobResult
        >(jobId);
        setJob(payload.job);

        if (payload.result) {
          const workflowResult = isWorkflowGraphResult(payload.result) ? payload.result : null;
          if (workflowResult) {
            setResult(null);
            const summary = summarizeWorkflowArtifacts(workflowResult);
            setSidebarSection("workflow");
            setWorkflowPanelTab("runs");
            setSelectedWorkflowId(workflowResult.workflow_id);
            setWorkflowRuns((current) =>
              upsertWorkflowRunRecord(current, {
                jobId: payload.job.job_id,
                workflowId: workflowResult.workflow_id,
                status: payload.job.status,
                progress: payload.job.progress ?? 0,
                currentNode: workflowResult.current_node ?? payload.job.message ?? null,
                summary,
                updatedAt: payload.job.updated_at ?? null,
              }),
            );
            setMessage(
              summary
                ? `${t.workflowCatalogCompleted}: ${workflowResult.workflow_id} (${summary})`
                : `${t.workflowCatalogCompleted}: ${workflowResult.workflow_id}`,
            );
            return;
          }

          const nonWorkflowResult = payload.result as Exclude<typeof payload.result, WorkflowGraphJobResult>;
          setResult(nonWorkflowResult);

          if (isAxialResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("axial_bar_1d");
            setAxialForm({
              length: nonWorkflowResult.input.length,
              area: nonWorkflowResult.input.area,
              elements: nonWorkflowResult.input.elements,
              tipForce: nonWorkflowResult.input.tip_force,
              material: activeMaterial,
              youngsModulusGpa: round(nonWorkflowResult.input.youngs_modulus / 1.0e9),
            });
          }

          if (isThermalBar1dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("thermal_bar_1d");
            setThermalBarModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isHeatBar1dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("heat_bar_1d");
            setHeatBarModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isHeatPlaneTriangle2dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("heat_plane_triangle_2d");
            setHeatPlaneModel(nonWorkflowResult.input);
            setPlaneResultField("average_temperature");
            openWorkspaceStudy("controls");
          }

          if (isHeatPlaneQuad2dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("heat_plane_quad_2d");
            setHeatPlaneModel(nonWorkflowResult.input);
            setPlaneResultField("average_temperature");
            openWorkspaceStudy("controls");
          }

          if (isThermalBeam1dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("thermal_beam_1d");
            setThermalBeamModel(ensureBeamModelMaterials(nonWorkflowResult.input, activeMaterial));
            openWorkspaceStudy("controls");
          }

          if (isThermalTruss2dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("thermal_truss_2d");
            setThermalTrussModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isThermalTruss3dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("thermal_truss_3d");
            setThermalTruss3dModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isSpring1dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("spring_1d");
            setSpringModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isSpring2dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("spring_2d");
            setSpring2dModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isSpring3dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("spring_3d");
            setSpring3dModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isBeam1dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("beam_1d");
            setBeamModel(ensureBeamModelMaterials(nonWorkflowResult.input, activeMaterial));
            openWorkspaceStudy("controls");
          }

          if (isTorsion1dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("torsion_1d");
            setTorsionModel(nonWorkflowResult.input);
            openWorkspaceStudy("controls");
          }

          if (isTrussResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("truss_2d");
            setTrussModel(ensureTrussModelMaterials(nonWorkflowResult.input, activeMaterial));
            openWorkspaceStudy("controls");
          }

          if (isTruss3dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("truss_3d");
            setTruss3dModel(ensureTruss3dModelMaterials(nonWorkflowResult.input, activeMaterial));
            openWorkspaceStudy("controls");
          }

          if (isFrame2dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("frame_2d");
            setFrameModel(ensureFrameModelMaterials(nonWorkflowResult.input, activeMaterial));
            openWorkspaceStudy("controls");
          }

          if (isThermalFrame2dResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind("thermal_frame_2d");
            setThermalFrameModel(ensureFrameModelMaterials(nonWorkflowResult.input, activeMaterial) as ThermalFrameStudyJobInput);
            openWorkspaceStudy("controls");
          }

          if (isPlaneResult(nonWorkflowResult)) {
            recordHistory(t.historyAction);
            setStudyKind(
              nonWorkflowResult.input.elements.some((element) => "node_l" in element)
                ? "plane_quad_2d"
                : "plane_triangle_2d",
            );
            setPlaneModel(ensurePlaneModelMaterials(nonWorkflowResult.input, activeMaterial));
            openWorkspaceStudy("controls");
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
    jobPollTokenRef.current += 1;

    startTransition(async () => {
      try {
        const payload = await cancelJob(job.job_id);
        setJob(payload.job);
        setMessage(t.jobCancelled);
        await refreshJobHistory();
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

  const fetchTextWithTimeout = async (url: string, timeoutMs = 12_000) => {
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(url, {
        cache: "no-store",
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error(`request failed: ${response.status}`);
      }

      return await response.text();
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        throw new Error(t.requestTimedOut);
      }

      throw error;
    } finally {
      window.clearTimeout(timeoutId);
    }
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
      } else if (imported.kind === "spring_1d") {
        setStudyKind("spring_1d");
        setSpringModel(imported.model);
      } else if (imported.kind === "heat_bar_1d") {
        setStudyKind("heat_bar_1d");
        setHeatBarModel(imported.model);
      } else if (imported.kind === "heat_plane_triangle_2d" || imported.kind === "heat_plane_quad_2d") {
        setStudyKind(imported.kind);
        setHeatPlaneModel(imported.model);
        setPlaneResultField("average_temperature");
      } else if (imported.kind === "thermal_bar_1d") {
        setStudyKind("thermal_bar_1d");
        setThermalBarModel(imported.model);
      } else if (imported.kind === "thermal_beam_1d") {
        setStudyKind("thermal_beam_1d");
        setThermalBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
      } else if (imported.kind === "thermal_frame_2d") {
        setStudyKind("thermal_frame_2d");
        setThermalFrameModel(ensureFrameModelMaterials(imported.model, imported.material) as ThermalFrameStudyJobInput);
        setActiveMaterial(imported.material);
      } else if (imported.kind === "thermal_truss_2d") {
        setStudyKind("thermal_truss_2d");
        setThermalTrussModel(imported.model);
      } else if (imported.kind === "thermal_truss_3d") {
        setStudyKind("thermal_truss_3d");
        setThermalTruss3dModel(imported.model);
      } else if (imported.kind === "spring_2d") {
        setStudyKind("spring_2d");
        setSpring2dModel(imported.model);
      } else if (imported.kind === "spring_3d") {
        setStudyKind("spring_3d");
        setSpring3dModel(imported.model);
      } else if (imported.kind === "truss_3d") {
        setStudyKind("truss_3d");
        setTruss3dModel(ensureTruss3dModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
      } else if (imported.kind === "frame_2d") {
        setStudyKind("frame_2d");
        setFrameModel(ensureFrameModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
      } else if (imported.kind === "beam_1d") {
        setStudyKind("beam_1d");
        setBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
        setActiveMaterial(imported.material);
      } else if (imported.kind === "torsion_1d") {
        setStudyKind("torsion_1d");
        setTorsionModel(imported.model);
      } else if (
        imported.kind === "plane_triangle_2d" ||
        imported.kind === "plane_quad_2d" ||
        imported.kind === "thermal_plane_triangle_2d" ||
        imported.kind === "thermal_plane_quad_2d"
      ) {
        setStudyKind(imported.kind);
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
        const text = await fetchTextWithTimeout(href);
        const imported = parsePlaygroundModel(text);
        recordHistory(t.sampleAction);
        setLoadedModelName(imported.name);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);

        if (
          imported.kind === "plane_triangle_2d" ||
          imported.kind === "plane_quad_2d" ||
          imported.kind === "thermal_plane_triangle_2d" ||
          imported.kind === "thermal_plane_quad_2d"
        ) {
          setStudyKind(imported.kind);
          setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
        } else if (imported.kind === "heat_bar_1d") {
          setStudyKind("heat_bar_1d");
          setHeatBarModel(imported.model);
        } else if (imported.kind === "heat_plane_triangle_2d" || imported.kind === "heat_plane_quad_2d") {
          setStudyKind(imported.kind);
          setHeatPlaneModel(imported.model);
          setPlaneResultField("average_temperature");
        } else if (imported.kind === "thermal_bar_1d") {
          setStudyKind("thermal_bar_1d");
          setThermalBarModel(imported.model);
        } else if (imported.kind === "thermal_beam_1d") {
          setStudyKind("thermal_beam_1d");
          setThermalBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
          setActiveMaterial(imported.material);
        } else if (imported.kind === "thermal_frame_2d") {
          setStudyKind("thermal_frame_2d");
          setThermalFrameModel(ensureFrameModelMaterials(imported.model, imported.material) as ThermalFrameStudyJobInput);
          setActiveMaterial(imported.material);
        } else if (imported.kind === "thermal_truss_2d") {
          setStudyKind("thermal_truss_2d");
          setThermalTrussModel(imported.model);
        } else if (imported.kind === "thermal_truss_3d") {
          setStudyKind("thermal_truss_3d");
          setThermalTruss3dModel(imported.model);
        } else if (imported.kind === "spring_1d") {
          setStudyKind("spring_1d");
          setSpringModel(imported.model);
        } else if (imported.kind === "spring_2d") {
          setStudyKind("spring_2d");
          setSpring2dModel(imported.model);
        } else if (imported.kind === "spring_3d") {
          setStudyKind("spring_3d");
          setSpring3dModel(imported.model);
        } else if (imported.kind === "beam_1d") {
          setStudyKind("beam_1d");
          setBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
        } else if (imported.kind === "torsion_1d") {
          setStudyKind("torsion_1d");
          setTorsionModel(imported.model);
        } else if (imported.kind === "frame_2d") {
          setStudyKind("frame_2d");
          setFrameModel(ensureFrameModelMaterials(imported.model, imported.material));
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
    applyLanguagePreference(nextLanguage);
  };

  const triggerJsonDownload = (filename: string, payload: Record<string, unknown>) => {
    if (typeof window === "undefined") return;
    const blob = new Blob([`${JSON.stringify(payload, null, 2)}\n`], { type: "application/json" });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    anchor.click();
    window.URL.revokeObjectURL(url);
  };

  const handleDownloadLanguagePackTemplate = () => {
    triggerJsonDownload(`workbench-language-pack-${language}.json`, {
      schema_version: WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
      id: `${language}-custom-pack`,
      language,
      name: `${t.languages[language]} custom pack`,
      version: "1.0.0",
      source: "imported",
      description:
        language === "zh"
          ? "从这个模板开始覆盖 Workbench 文案。"
          : language === "ja"
            ? "このテンプレートから Workbench 文言を上書きします。"
            : "Start from this template to override Workbench copy.",
      overrides: {},
    });
    setMessage(
      language === "zh"
        ? "语言包模板已下载。"
        : language === "ja"
          ? "言語パックのテンプレートを出力しました。"
          : "Language pack template downloaded.",
    );
  };

  const handleExportInstalledLanguagePack = () => {
    const pack = activeLanguagePack;
    if (!pack) {
      setMessage(
        language === "zh"
          ? "当前语言还没有安装自定义语言包。"
          : language === "ja"
            ? "現在の言語にはまだカスタム言語パックがありません。"
            : "No custom language pack is installed for the current language yet.",
      );
      return;
    }

    triggerJsonDownload(`workbench-language-pack-${pack.language}-${pack.id}.json`, pack);
    setMessage(
      language === "zh"
        ? "当前语言包已导出。"
        : language === "ja"
          ? "現在の言語パックを出力しました。"
          : "Exported the current language pack.",
    );
  };

  const handleImportLanguagePack = async (file: File) => {
    try {
      const raw = JSON.parse(await file.text()) as Partial<WorkbenchLanguagePack> & { overrides?: Record<string, unknown> };
      if (!raw || typeof raw !== "object" || typeof raw.language !== "string" || typeof raw.name !== "string") {
        throw new Error("invalid-pack");
      }

      const nextPack: WorkbenchLanguagePack = {
        schema_version:
          typeof raw.schema_version === "string" && raw.schema_version.trim()
            ? raw.schema_version.trim()
            : WORKBENCH_LANGUAGE_PACK_SCHEMA_VERSION,
        id: typeof raw.id === "string" && raw.id.trim() ? raw.id.trim() : `${raw.language}-${Date.now()}`,
        language: raw.language,
        name: raw.name,
        version: typeof raw.version === "string" && raw.version.trim() ? raw.version.trim() : "1.0.0",
        source: raw.source === "downloaded" ? "downloaded" : "imported",
        updatedAt: new Date().toISOString(),
        description: typeof raw.description === "string" ? raw.description : undefined,
        overrides: raw.overrides && typeof raw.overrides === "object" && !Array.isArray(raw.overrides) ? raw.overrides : {},
      };

      setLanguagePacks((current) => {
        const next = current.filter((pack) => !(pack.id === nextPack.id || (pack.language === nextPack.language && pack.name === nextPack.name)));
        return [nextPack, ...next];
      });

      setMessage(
        language === "zh"
          ? "语言包已导入。"
          : language === "ja"
            ? "言語パックを取り込みました。"
            : "Language pack imported.",
      );
    } catch {
      setMessage(
        language === "zh"
          ? "语言包 JSON 无效。"
          : language === "ja"
            ? "言語パック JSON が無効です。"
            : "Invalid language pack JSON.",
      );
    }
  };

  const handleRemoveLanguagePack = (packId: string) => {
    setLanguagePacks((current) => current.filter((pack) => pack.id !== packId));
    setMessage(
      language === "zh"
        ? "语言包已移除。"
        : language === "ja"
          ? "言語パックを削除しました。"
          : "Language pack removed.",
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

  const downloadPlaneHotspotSummary = () => {
    if (!isPlane || planeHotspotElements.length === 0) {
      setMessage(t.noResultToExport);
      return;
    }

    const lines = [
      ["rank", "id", "field", "value"].join(","),
      ...planeHotspotElements.map((entry, index) => [index + 1, entry.id, planeResultField, entry.value].join(",")),
    ];

    downloadTextFile(`${loadedModelName || "kyuubiki-study"}-${planeResultField}-hotspots.csv`, lines.join("\n"));
    setMessage(t.resultCsvDownloaded);
  };

  const downloadFrameHotspotSummary = () => {
    if (!(isFrameLike || isBeam || isSpring || isThermal) || frameHotspotElements.length === 0) {
      setMessage(t.noResultToExport);
      return;
    }

    const lines = [
      ["rank", "id", "field", "value", "end_forces"].join(","),
      ...frameHotspotElements.map((entry, index) => [index + 1, entry.id, activeLineResultField, entry.value, `"${entry.summary ?? ""}"`].join(",")),
    ];

    downloadTextFile(`${loadedModelName || "kyuubiki-study"}-${activeLineResultField}-hotspots.csv`, lines.join("\n"));
    setMessage(t.resultCsvDownloaded);
  };

  const downloadFrameForceSummary = () => {
    if (!(isFrameLike || isBeam || isSpring || isThermal) || frameForceRows.length === 0) {
      setMessage(t.noResultToExport);
      return;
    }

    const lines = [
      ["id", "node_i", "node_j", "axial_force_i", "shear_force_i", "moment_i", "axial_force_j", "shear_force_j", "moment_j"].join(","),
      ...displayTrussElements.map((element) =>
        [
          element.id,
          element.node_i,
          element.node_j,
          element.axial_force_i ?? "",
          element.shear_force_i ?? "",
          element.moment_i ?? "",
          element.axial_force_j ?? "",
          element.shear_force_j ?? "",
          element.moment_j ?? "",
        ].join(","),
      ),
    ];

    downloadTextFile(
      `${loadedModelName || "kyuubiki-study"}-${isThermalBar ? "thermal-bar" : isThermalTruss2d || isThermalTruss3d ? "thermal-truss" : isSpring ? "spring" : isBeam ? "beam" : "frame"}-member-forces.csv`,
      lines.join("\n"),
    );
    setMessage(t.resultCsvDownloaded);
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
    try {
      const { bundle, partial } = await buildProjectBundleJson();
      downloadTextFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki.json`, bundle);
      setMessage(partial ? t.projectExportedPartial : t.projectExported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const downloadProjectBundleZip = async () => {
    try {
      const { bundle, partial } = await buildProjectBundleJson();
      const blob = await exportProjectBundleZip(bundle);
      downloadBlobFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki`, blob);
      setMessage(partial ? t.projectExportedPartial : t.projectExported);
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

  const downloadSecurityEventExport = async () => {
    try {
      const occurredAfter =
        securityEventWindowFilter && SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter]
          ? new Date(Date.now() - SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter]).toISOString()
          : undefined;
      const snapshot = await exportSecurityEvents({
        occurred_after: occurredAfter,
        source: securityEventSourceFilter || undefined,
        risk: securityEventRiskFilter || undefined,
        status: securityEventStatusFilter || undefined,
        action: securityEventActionFilter || undefined,
        limit: 500,
      });
      const timestamp = snapshot.exported_at.replaceAll(":", "-");
      downloadTextFile(`kyuubiki-security-events-${timestamp}.json`, JSON.stringify(snapshot, null, 2));
      setMessage(language === "zh" ? "安全事件分析包已下载。" : language === "ja" ? "セキュリティイベント分析バンドルを出力しました。" : "Security event export downloaded.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const downloadSecurityEventCsvExport = async () => {
    try {
      const occurredAfter =
        securityEventWindowFilter && SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter]
          ? new Date(Date.now() - SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter]).toISOString()
          : undefined;
      const csv = await exportSecurityEventsCsv({
        occurred_after: occurredAfter,
        source: securityEventSourceFilter || undefined,
        risk: securityEventRiskFilter || undefined,
        status: securityEventStatusFilter || undefined,
        action: securityEventActionFilter || undefined,
        limit: 1000,
      });
      const timestamp = new Date().toISOString().replaceAll(":", "-");
      downloadTextFile(`kyuubiki-security-events-${timestamp}.csv`, csv);
      setMessage(language === "zh" ? "安全事件 CSV 已下载。" : language === "ja" ? "セキュリティイベント CSV を出力しました。" : "Security event CSV downloaded.");
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

      for (const preset of bundle.automation_presets ?? []) {
        try {
          saveWorkbenchMacroPreset({
            projectId: createdProject.project.project_id,
            presetId: preset.presetId,
            name: preset.name,
            macro: preset.macro,
          });
        } catch {
          // Ignore malformed preset payloads so model/project import stays usable.
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

    if (
      imported.kind === "heat_plane_triangle_2d" ||
      imported.kind === "heat_plane_quad_2d" ||
      imported.kind === "plane_triangle_2d" ||
      imported.kind === "plane_quad_2d" ||
      imported.kind === "thermal_plane_triangle_2d" ||
      imported.kind === "thermal_plane_quad_2d"
    ) {
      setStudyKind(imported.kind);
      if (imported.kind === "heat_plane_triangle_2d" || imported.kind === "heat_plane_quad_2d") {
        setPlaneResultField("average_temperature");
      }
      if (imported.kind === "heat_plane_triangle_2d" || imported.kind === "heat_plane_quad_2d") {
        setHeatPlaneModel(imported.model);
      } else {
        setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
      }
    } else if (imported.kind === "spring_1d") {
      setStudyKind("spring_1d");
      setSpringModel(imported.model);
    } else if (imported.kind === "heat_bar_1d") {
      setStudyKind("heat_bar_1d");
      setHeatBarModel(imported.model);
    } else if (imported.kind === "thermal_bar_1d") {
      setStudyKind("thermal_bar_1d");
      setThermalBarModel(imported.model);
    } else if (imported.kind === "thermal_beam_1d") {
      setStudyKind("thermal_beam_1d");
      setThermalBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
      setActiveMaterial(imported.material);
    } else if (imported.kind === "thermal_frame_2d") {
      setStudyKind("thermal_frame_2d");
      setThermalFrameModel(ensureFrameModelMaterials(imported.model, imported.material) as ThermalFrameStudyJobInput);
      setActiveMaterial(imported.material);
    } else if (imported.kind === "thermal_truss_2d") {
      setStudyKind("thermal_truss_2d");
      setThermalTrussModel(imported.model);
    } else if (imported.kind === "thermal_truss_3d") {
      setStudyKind("thermal_truss_3d");
      setThermalTruss3dModel(imported.model);
    } else if (imported.kind === "spring_2d") {
      setStudyKind("spring_2d");
      setSpring2dModel(imported.model);
    } else if (imported.kind === "spring_3d") {
      setStudyKind("spring_3d");
      setSpring3dModel(imported.model);
    } else if (imported.kind === "beam_1d") {
      setStudyKind("beam_1d");
      setBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
    } else if (imported.kind === "torsion_1d") {
      setStudyKind("torsion_1d");
      setTorsionModel(imported.model);
    } else if (imported.kind === "frame_2d") {
      setStudyKind("frame_2d");
      setFrameModel(ensureFrameModelMaterials(imported.model, imported.material));
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
    setActiveMaterial("material" in imported ? imported.material : activeMaterial);
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
    openModelVersionById(version.version_id);
  };

  const openModelVersionById = (versionId: string) => {
    startTransition(async () => {
      try {
        const payload = await fetchModelVersion(versionId);
        recordHistory(t.historyAction);
        applyPersistedPayload(payload.version.payload, payload.version.name);
        setSelectedModelId(payload.version.model_id);
        setSelectedProjectId(payload.version.project_id);
        setSelectedVersionId(payload.version.version_id);
        setMessage(t.versionLoaded);
        setSidebarSection("model");
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

  const openSelectedAdminJobVersion = () => {
    if (!selectedAdminJob?.model_version_id) {
      setMessage(language === "zh" ? "这个任务还没有关联模型版本。" : language === "ja" ? "このジョブには関連するモデルバージョンがまだありません。" : "This job does not have a linked model version.");
      return;
    }

    openModelVersionById(selectedAdminJob.model_version_id);
  };

  const openSelectedAdminResultVersion = () => {
    const linkedJob = jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId);

    if (!linkedJob?.model_version_id) {
      setMessage(language === "zh" ? "这个结果还没有关联模型版本。" : language === "ja" ? "この結果には関連するモデルバージョンがまだありません。" : "This result does not have a linked model version.");
      return;
    }

    openModelVersionById(linkedJob.model_version_id);
  };

  const applyJobContextToWorkbench = (entry: JobState) => {
    setAdminFilterProjectId(entry.project_id ?? "");
    setAdminFilterModelVersionId(entry.model_version_id ?? "");
    setAdminJobCaseId(entry.simulation_case_id ?? "");
    setLibraryTab("projects");

    if (entry.model_version_id) {
      openModelVersionById(entry.model_version_id);
      return;
    }

    if (entry.project_id) {
      openProjectContextById(entry.project_id);
      return;
    }

    setMessage(language === "zh" ? "这条记录还没有可应用的项目或版本上下文。" : language === "ja" ? "このレコードには適用できる project / version の文脈がまだありません。" : "This record does not have a linked project or version context yet.");
  };

  const openProjectContextById = (projectId: string) => {
    const project = projects.find((entry) => entry.project_id === projectId);

    if (!project) {
      setMessage(language === "zh" ? "找不到关联项目。" : language === "ja" ? "関連プロジェクトが見つかりませんでした。" : "Could not find the linked project.");
      return;
    }

    const firstModelId = project.models?.[0]?.model_id ?? null;
    const firstVersionId = project.models?.[0]?.latest_version_id ?? null;

    setSelectedProjectId(project.project_id);
    setSelectedModelId(firstModelId);
    setSelectedVersionId(firstVersionId);
    setSidebarSection("library");

    if (firstModelId) {
      void refreshVersions(firstModelId);
    } else {
      setModelVersions([]);
    }

    setMessage(t.linkedProjectOpened);
  };

  const openSelectedAdminJobProject = () => {
    if (!selectedAdminJob?.project_id) {
      setMessage(language === "zh" ? "这个任务还没有关联项目。" : language === "ja" ? "このジョブには関連プロジェクトがまだありません。" : "This job does not have a linked project.");
      return;
    }

    openProjectContextById(selectedAdminJob.project_id);
  };

  const openSelectedAdminResultProject = () => {
    const linkedJob = jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId);

    if (!linkedJob?.project_id) {
      setMessage(language === "zh" ? "这个结果还没有关联项目。" : language === "ja" ? "この結果には関連プロジェクトがまだありません。" : "This result does not have a linked project.");
      return;
    }

    openProjectContextById(linkedJob.project_id);
  };

  const applySelectedAdminJobContext = () => {
    if (!selectedAdminJob) {
      setMessage(language === "zh" ? "请先选择一条任务记录。" : language === "ja" ? "先にジョブレコードを選択してください。" : "Select a job record first.");
      return;
    }

    applyJobContextToWorkbench(selectedAdminJob);
    if (!selectedAdminJob.model_version_id && !selectedAdminJob.project_id) {
      return;
    }
    setMessage(t.recordContextApplied);
  };

  const applySelectedAdminResultContext = () => {
    const linkedJob = jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId);

    if (!linkedJob) {
      setMessage(language === "zh" ? "找不到这条结果对应的任务记录。" : language === "ja" ? "この結果に対応するジョブレコードが見つかりませんでした。" : "Could not find the job record linked to this result.");
      return;
    }

    applyJobContextToWorkbench(linkedJob);
    if (!linkedJob.model_version_id && !linkedJob.project_id) {
      return;
    }
    setMessage(t.recordContextApplied);
  };

  const resolveScriptLinkedJob = (payload: Record<string, unknown>) => {
    const target = payload.target === "job" || payload.target === "result" ? payload.target : "job";
    const explicitJobId = typeof payload.jobId === "string" ? payload.jobId : null;
    const explicitResultJobId = typeof payload.resultJobId === "string" ? payload.resultJobId : null;

    if (target === "job") {
      const jobId = explicitJobId ?? selectedAdminJobId;
      return jobId ? jobHistory.find((entry) => entry.job_id === jobId) ?? null : null;
    }

    const resultJobId = explicitResultJobId ?? explicitJobId ?? selectedAdminResultJobId;
    return resultJobId ? jobHistory.find((entry) => entry.job_id === resultJobId) ?? null : null;
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
  const jobIsActive =
    job?.status === "queued" ||
    job?.status === "preprocessing" ||
    job?.status === "partitioning" ||
    job?.status === "solving" ||
    job?.status === "postprocessing";
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
  const planeHotspotElements = useMemo(
    () =>
      planeElements
        .map((element) => ({
          index: element.index,
          id: element.id,
          value: planeResultFieldValue(element, planeResultField),
          active: selectedElement === element.index,
          summary: isHeatPlane
            ? `Tavg ${scientific("average_temperature" in element ? element.average_temperature : undefined)} · ∇Ty ${scientific("temperature_gradient_y" in element ? element.temperature_gradient_y : undefined)} · |q| ${scientific("heat_flux_magnitude" in element ? element.heat_flux_magnitude : undefined)}`
            : undefined,
        }))
        .sort((left, right) => right.value - left.value)
        .slice(0, planeHotspotLimit)
        .map((element) => ({
          index: element.index,
          id: element.id,
          value: scientific(element.value),
          active: element.active,
          summary: element.summary,
        })),
    [isHeatPlane, planeElements, planeResultField, selectedElement, planeHotspotLimit],
  );
  const planeThermalRows = useMemo(
    () =>
      isHeatPlane
        ? planeElements.map((element) => {
            const heatElement = element as typeof element & {
              average_temperature?: number;
              temperature_gradient_x?: number;
              temperature_gradient_y?: number;
              heat_flux_x?: number;
              heat_flux_y?: number;
              heat_flux_magnitude?: number;
            };
            return {
              id: element.id,
              index: element.index,
              active: selectedElement === element.index,
              sortTemperature: Math.abs(heatElement.average_temperature ?? 0),
              sortGradient: Math.max(Math.abs(heatElement.temperature_gradient_y ?? 0), Math.abs(heatElement.temperature_gradient_x ?? 0)),
              sortFlux: Math.abs(heatElement.heat_flux_magnitude ?? 0),
              averageTemperature: scientific(heatElement.average_temperature),
              temperatureGradientX: scientific(heatElement.temperature_gradient_x),
              temperatureGradientY: scientific(heatElement.temperature_gradient_y),
              heatFluxX: scientific(heatElement.heat_flux_x),
              heatFluxY: scientific(heatElement.heat_flux_y),
              heatFluxMagnitude: scientific(heatElement.heat_flux_magnitude),
            };
          })
        : [],
    [isHeatPlane, planeElements, selectedElement],
  );
  const frameHotspotElements = useMemo(
    () =>
      displayTrussElements
        .map((element) => ({
          index: element.index,
          id: element.id,
          value: lineResultFieldValue(element, activeLineResultField),
          active: selectedElement === element.index,
          summary: isThermalFrame
            ? `ΔT ${scientific(element.average_temperature_delta)} · ∇T ${scientific(element.temperature_gradient_y)} · κth ${scientific(element.thermal_curvature)}`
            : isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
            ? `Fi ${scientific(element.axial_force_i)} · Fj ${scientific(element.axial_force_j)}`
            : isTorsion
              ? `Ti ${scientific(element.moment_i)} · Tj ${scientific(element.moment_j)}`
            : isBeam
            ? `Vi ${scientific(element.shear_force_i)} · Mi ${scientific(element.moment_i)} · Vj ${scientific(element.shear_force_j)} · Mj ${scientific(element.moment_j)}`
            : `Ai ${scientific(element.axial_force_i)} · Vi ${scientific(element.shear_force_i)} · Mi ${scientific(element.moment_i)} · Aj ${scientific(element.axial_force_j)} · Vj ${scientific(element.shear_force_j)} · Mj ${scientific(element.moment_j)}`,
        }))
        .sort((left, right) => right.value - left.value)
        .slice(0, planeHotspotLimit)
        .map((element) => ({
          index: element.index,
          id: element.id,
          value: scientific(element.value),
          active: element.active,
          summary: element.summary,
        })),
    [activeLineResultField, displayTrussElements, isBeam, isSpring, isThermalBar, isThermalFrame, isThermalTruss2d, isThermalTruss3d, selectedElement, planeHotspotLimit],
  );
  const frameForceRows = useMemo(
    () =>
      displayTrussElements.map((element) => ({
        id: element.id,
        index: element.index,
        active: selectedElement === element.index,
        sortAxial: Math.max(Math.abs(element.axial_force_i ?? 0), Math.abs(element.axial_force_j ?? 0)),
        sortShear: Math.max(Math.abs(element.shear_force_i ?? 0), Math.abs(element.shear_force_j ?? 0)),
        sortMoment: Math.max(Math.abs(element.moment_i ?? 0), Math.abs(element.moment_j ?? 0)),
        axialForceI: scientific(element.axial_force_i),
        shearForceI: scientific(element.shear_force_i),
        momentI: scientific(element.moment_i),
        axialForceJ: scientific(element.axial_force_j),
        shearForceJ: scientific(element.shear_force_j),
        momentJ: scientific(element.moment_j),
      })),
    [displayTrussElements, selectedElement],
  );
  const frameMaxAxialForce = useMemo(
    () => Math.max(...displayTrussElements.map((element) => Math.max(Math.abs(element.axial_force_i ?? 0), Math.abs(element.axial_force_j ?? 0))), 0),
    [displayTrussElements],
  );
  const frameMaxShearForce = useMemo(
    () => Math.max(...displayTrussElements.map((element) => Math.max(Math.abs(element.shear_force_i ?? 0), Math.abs(element.shear_force_j ?? 0))), 0),
    [displayTrussElements],
  );
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
  const selectedNodeData = selectedNode !== null ? displayTrussNodes[selectedNode] : null;
  const heartbeatStatusValue = heartbeatStatus(job, t);
  const heartbeatToneValue = heartbeatTone(job);
  const selectedElementData = selectedElement !== null ? displayTrussElements[selectedElement] : null;
  const selectedTruss3dNodeData = selectedNode !== null ? displayTruss3dNodes[selectedNode] : null;
  const selectedTruss3dElementData = selectedElement !== null ? displayTruss3dElements[selectedElement] : null;
  const selectedPlaneNodeData = selectedNode !== null ? planeNodes[selectedNode] : null;
  const selectedPlaneElementData = selectedElement !== null ? planeElements[selectedElement] : null;
  const selectedFrameNodeData =
    selectedNode !== null && (isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode])
      ? {
          ...(isThermalFrame ? thermalFrameModel.nodes[selectedNode] : frameModel.nodes[selectedNode]),
          displacement_magnitude: isThermalFrame ? thermalFrameResult?.nodes[selectedNode]?.displacement_magnitude : frameResult?.nodes[selectedNode]?.displacement_magnitude,
          rz: isThermalFrame ? thermalFrameResult?.nodes[selectedNode]?.rz : frameResult?.nodes[selectedNode]?.rz,
        }
      : null;
  const selectedFrameElementData =
    selectedElement !== null && (isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement])
      ? {
          index: selectedElement,
          ...(isThermalFrame ? thermalFrameModel.elements[selectedElement] : frameModel.elements[selectedElement]),
          ...(isThermalFrame ? thermalFrameResult?.elements[selectedElement] : frameResult?.elements[selectedElement]),
        }
      : null;
  const selectedBeamNodeData =
    selectedNode !== null && (isThermalBeam ? thermalBeamModel.nodes[selectedNode] : beamModel.nodes[selectedNode])
      ? {
          ...(isThermalBeam ? thermalBeamModel.nodes[selectedNode] : beamModel.nodes[selectedNode]),
          displacement_magnitude: isThermalBeam ? thermalBeamResult?.nodes[selectedNode]?.displacement_magnitude : beamResult?.nodes[selectedNode]?.displacement_magnitude,
          rz: isThermalBeam ? thermalBeamResult?.nodes[selectedNode]?.rz : beamResult?.nodes[selectedNode]?.rz,
          y: 0,
          load_x: 0,
          load_y: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.load_y : beamModel.nodes[selectedNode]?.load_y) ?? 0,
          fix_x: false,
          fix_y: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.fix_y : beamModel.nodes[selectedNode]?.fix_y) ?? false,
          moment_z: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.moment_z : beamModel.nodes[selectedNode]?.moment_z) ?? 0,
          fix_rz: (isThermalBeam ? thermalBeamModel.nodes[selectedNode]?.fix_rz : beamModel.nodes[selectedNode]?.fix_rz) ?? false,
        }
      : null;
  const selectedBeamElementData =
    selectedElement !== null && (isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement])
      ? {
          index: selectedElement,
          ...(isThermalBeam ? thermalBeamModel.elements[selectedElement] : beamModel.elements[selectedElement]),
          area: 0,
          ...(isThermalBeam ? thermalBeamResult?.elements[selectedElement] : beamResult?.elements[selectedElement]),
        }
      : null;
  const selectedTorsionNodeData =
    selectedNode !== null && torsionModel.nodes[selectedNode]
      ? {
          id: torsionModel.nodes[selectedNode].id,
          x: torsionModel.nodes[selectedNode].x,
          y: 0,
          load_x: 0,
          load_y: 0,
          moment_z: torsionModel.nodes[selectedNode].torque_z,
          fix_x: false,
          fix_y: true,
          fix_rz: torsionModel.nodes[selectedNode].fix_rz,
          displacement_magnitude: Math.abs(torsionResult?.nodes[selectedNode]?.rz ?? 0),
          rz: torsionResult?.nodes[selectedNode]?.rz,
        }
      : null;
  const selectedTorsionElementData =
    selectedElement !== null && torsionModel.elements[selectedElement]
      ? {
          index: selectedElement,
          id: torsionModel.elements[selectedElement].id,
          node_i: torsionModel.elements[selectedElement].node_i,
          node_j: torsionModel.elements[selectedElement].node_j,
          area: 0,
          youngs_modulus: torsionModel.elements[selectedElement].shear_modulus,
          moment_of_inertia: torsionModel.elements[selectedElement].polar_moment,
          section_modulus: torsionModel.elements[selectedElement].section_modulus,
          axial_stress: undefined,
          max_bending_stress: torsionResult?.elements[selectedElement]?.shear_stress,
          max_combined_stress: undefined,
          axial_force_i: 0,
          shear_force_i: 0,
          moment_i: torsionResult?.elements[selectedElement]?.torque,
          axial_force_j: 0,
          shear_force_j: 0,
          moment_j: torsionResult?.elements[selectedElement]?.torque,
        }
      : null;
  const selectedThermalNodeData =
    selectedNode !== null && (isHeatBar ? heatBarModel.nodes[selectedNode] : thermalBarModel.nodes[selectedNode])
      ? {
          id: isHeatBar ? heatBarModel.nodes[selectedNode].id : thermalBarModel.nodes[selectedNode].id,
          x: isHeatBar ? heatBarModel.nodes[selectedNode].x : thermalBarModel.nodes[selectedNode].x,
          y: 0,
          load_x: isHeatBar ? heatBarModel.nodes[selectedNode].heat_load : thermalBarModel.nodes[selectedNode].load_x,
          load_y: 0,
          fix_x: isHeatBar ? heatBarModel.nodes[selectedNode].fix_temperature : thermalBarModel.nodes[selectedNode].fix_x,
          fix_y: true,
          moment_z: 0,
          fix_rz: true,
          displacement_magnitude: Math.abs(isHeatBar ? heatBarResult?.nodes[selectedNode]?.temperature ?? 0 : thermalBarResult?.nodes[selectedNode]?.ux ?? 0),
          rz: 0,
          temperature: isHeatBar ? heatBarModel.nodes[selectedNode].temperature ?? 0 : undefined,
          heat_load: isHeatBar ? heatBarModel.nodes[selectedNode].heat_load ?? 0 : undefined,
          fix_temperature: isHeatBar ? heatBarModel.nodes[selectedNode].fix_temperature ?? false : undefined,
        }
      : null;
  const selectedThermalElementData =
    selectedElement !== null && (isHeatBar ? heatBarModel.elements[selectedElement] : thermalBarModel.elements[selectedElement])
      ? {
          index: selectedElement,
          ...(isHeatBar ? heatBarModel.elements[selectedElement] : thermalBarModel.elements[selectedElement]),
          area: isHeatBar ? heatBarModel.elements[selectedElement].area : thermalBarModel.elements[selectedElement].area,
          youngs_modulus: isHeatBar ? heatBarModel.elements[selectedElement].conductivity : thermalBarModel.elements[selectedElement].youngs_modulus,
          moment_of_inertia: 0,
          section_modulus: 0,
          axial_stress: displayTrussElements[selectedElement]?.axial_stress,
          average_temperature: isHeatBar ? heatBarResult?.elements[selectedElement]?.average_temperature ?? 0 : undefined,
          temperature_gradient_x: isHeatBar ? heatBarResult?.elements[selectedElement]?.temperature_gradient ?? 0 : undefined,
          heat_flux_x: isHeatBar ? heatBarResult?.elements[selectedElement]?.heat_flux ?? 0 : undefined,
          heat_flux_y: isHeatBar ? 0 : undefined,
          heat_flux_magnitude: isHeatBar ? Math.abs(heatBarResult?.elements[selectedElement]?.heat_flux ?? 0) : undefined,
          axial_force_i: displayTrussElements[selectedElement]?.axial_force_i,
          shear_force_i: 0,
          moment_i: 0,
          axial_force_j: displayTrussElements[selectedElement]?.axial_force_j,
          shear_force_j: 0,
          moment_j: 0,
        }
      : null;
  const selectedSpringElementData =
    selectedElement !== null && activeSpringModel.elements[selectedElement]
      ? {
          index: selectedElement,
          ...activeSpringModel.elements[selectedElement],
          area: 0,
          youngs_modulus: 0,
          moment_of_inertia: 0,
          section_modulus: 0,
          axial_stress: displayTrussElements[selectedElement]?.axial_stress,
          axial_force_i: displayTrussElements[selectedElement]?.axial_force_i,
          shear_force_i: 0,
          moment_i: 0,
          axial_force_j: displayTrussElements[selectedElement]?.axial_force_j,
          shear_force_j: 0,
          moment_j: 0,
        }
      : null;
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
  const runtimeAuditSummaryRows = useMemo(() => {
    const countByRisk = securityEventRecords.reduce<Record<string, number>>((accumulator, entry) => {
      accumulator[entry.risk] = (accumulator[entry.risk] ?? 0) + 1;
      return accumulator;
    }, {});
    const countByStatus = securityEventRecords.reduce<Record<string, number>>((accumulator, entry) => {
      accumulator[entry.status] = (accumulator[entry.status] ?? 0) + 1;
      return accumulator;
    }, {});

    return [
      {
        label: language === "zh" ? "敏感" : language === "ja" ? "機微" : "Sensitive",
        value: String(countByRisk.sensitive ?? 0),
      },
      {
        label: language === "zh" ? "高风险" : language === "ja" ? "破壊的" : "Destructive",
        value: String(countByRisk.destructive ?? 0),
      },
      {
        label: language === "zh" ? "已执行" : language === "ja" ? "完了" : "Completed",
        value: String(countByStatus.completed ?? 0),
      },
      {
        label: language === "zh" ? "已取消" : language === "ja" ? "取消済み" : "Cancelled",
        value: String(countByStatus.cancelled ?? 0),
      },
      {
        label: language === "zh" ? "失败" : language === "ja" ? "失敗" : "Failed",
        value: String(countByStatus.failed ?? 0),
      },
      {
        label: language === "zh" ? "待确认" : language === "ja" ? "確認待ち" : "Prompted",
        value: String(countByStatus.prompted ?? 0),
      },
    ];
  }, [language, securityEventRecords]);
  const runtimeAuditTrendBars = useMemo(() => {
    if (securityEventRecords.length === 0) return [];

    const bucketCount =
      securityEventWindowFilter === "1h" ? 6 : securityEventWindowFilter === "24h" ? 8 : securityEventWindowFilter === "7d" ? 7 : 6;
    const bucketWindowMs =
      securityEventWindowFilter === "1h"
        ? 10 * 60 * 1_000
        : securityEventWindowFilter === "24h"
          ? 3 * 60 * 60 * 1_000
          : securityEventWindowFilter === "7d"
            ? 24 * 60 * 60 * 1_000
            : securityEventWindowFilter === "30d"
              ? 5 * 24 * 60 * 60 * 1_000
              : 24 * 60 * 60 * 1_000;
    const bucketLabels = Array.from({ length: bucketCount }, (_, index) => {
      if (securityEventWindowFilter === "1h") {
        return language === "zh" ? `${(bucketCount - index) * 10} 分内` : language === "ja" ? `${(bucketCount - index) * 10}分以内` : `${(bucketCount - index) * 10}m`;
      }
      if (securityEventWindowFilter === "24h") {
        return language === "zh" ? `${(bucketCount - index) * 3} 小时内` : language === "ja" ? `${(bucketCount - index) * 3}時間以内` : `${(bucketCount - index) * 3}h`;
      }
      if (securityEventWindowFilter === "7d") {
        return language === "zh" ? `${bucketCount - index} 天内` : language === "ja" ? `${bucketCount - index}日以内` : `${bucketCount - index}d`;
      }
      if (securityEventWindowFilter === "30d") {
        return language === "zh" ? `${(bucketCount - index) * 5} 天内` : language === "ja" ? `${(bucketCount - index) * 5}日以内` : `${(bucketCount - index) * 5}d`;
      }
      return language === "zh" ? `${bucketCount - index} 天内` : language === "ja" ? `${bucketCount - index}日以内` : `${bucketCount - index}d`;
    });
    const now = Date.now();
    const counts = new Array(bucketCount).fill(0);

    securityEventRecords.forEach((entry) => {
      const occurredAt = Date.parse(entry.occurred_at);
      if (Number.isNaN(occurredAt)) return;
      const ageMs = Math.max(now - occurredAt, 0);
      const bucketIndex = Math.min(Math.floor(ageMs / bucketWindowMs), bucketCount - 1);
      counts[bucketCount - bucketIndex - 1] += 1;
    });

    const maxCount = Math.max(...counts, 1);
    return counts.map((count, index) => ({
      key: `${bucketLabels[index]}-${index}`,
      label: bucketLabels[index],
      value: String(count),
      ratio: count / maxCount,
    }));
  }, [language, securityEventRecords, securityEventWindowFilter]);
  const runtimeAuditSourceStatusFacets = useMemo(() => {
    const counts = securityEventRecords.reduce<Map<string, number>>((accumulator, entry) => {
      const key = `${entry.source}:${entry.status}`;
      accumulator.set(key, (accumulator.get(key) ?? 0) + 1);
      return accumulator;
    }, new Map<string, number>());

    return [...counts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .slice(0, 6)
      .map(([key, value]) => {
        const [source, status] = key.split(":");
        const sourceLabel =
          source === "assistant"
            ? language === "zh"
              ? "助手"
              : language === "ja"
                ? "アシスタント"
                : "Assistant"
            : language === "zh"
              ? "脚本"
              : language === "ja"
                ? "スクリプト"
                : "Script";
        const statusLabel =
          status === "prompted"
            ? language === "zh"
              ? "待确认"
              : language === "ja"
                ? "確認待ち"
                : "Prompted"
            : status === "cancelled"
              ? language === "zh"
                ? "已取消"
                : language === "ja"
                  ? "取消済み"
                  : "Cancelled"
              : status === "completed"
                ? language === "zh"
                  ? "已执行"
                  : language === "ja"
                    ? "完了"
                    : "Completed"
                : language === "zh"
                  ? "失败"
                  : language === "ja"
                    ? "失敗"
                    : "Failed";

        return {
          key,
          label: `${sourceLabel} / ${statusLabel}`,
          value: String(value),
        };
      });
  }, [language, securityEventRecords]);
  const runtimeAuditStudyFacets = useMemo(() => {
    const counts = securityEventRecords.reduce<Map<string, number>>((accumulator, entry) => {
      const studyKind = typeof entry.context.study_kind === "string" ? entry.context.study_kind : "";
      if (!studyKind) return accumulator;
      accumulator.set(studyKind, (accumulator.get(studyKind) ?? 0) + 1);
      return accumulator;
    }, new Map<string, number>());

    return [...counts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .slice(0, 4)
      .map(([label, value]) => ({ key: label, label, value: String(value) }));
  }, [securityEventRecords]);
  const runtimeAuditProjectFacets = useMemo(() => {
    const counts = securityEventRecords.reduce<Map<string, number>>((accumulator, entry) => {
      const projectId = typeof entry.context.project_id === "string" ? entry.context.project_id : "";
      if (!projectId) return accumulator;
      const shortId = projectId.slice(0, 8);
      accumulator.set(shortId, (accumulator.get(shortId) ?? 0) + 1);
      return accumulator;
    }, new Map<string, number>());

    return [...counts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .slice(0, 4)
      .map(([label, value]) => ({ key: label, label, value: String(value) }));
  }, [securityEventRecords]);
  const runtimeAuditModelVersionFacets = useMemo(() => {
    const counts = securityEventRecords.reduce<Map<string, number>>((accumulator, entry) => {
      const modelVersionId =
        typeof entry.context.model_version_id === "string" ? entry.context.model_version_id : "";
      if (!modelVersionId) return accumulator;
      const shortId = modelVersionId.slice(0, 8);
      accumulator.set(shortId, (accumulator.get(shortId) ?? 0) + 1);
      return accumulator;
    }, new Map<string, number>());

    return [...counts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .slice(0, 4)
      .map(([label, value]) => ({ key: label, label, value: String(value) }));
  }, [securityEventRecords]);
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
  const hasAnyResult = Boolean(axialResult || heatBarResult || thermalBarResult || thermalBeamResult || thermalFrameResult || thermalTrussResult || thermalTruss3dResult || trussResult || truss3dResult || springResult || spring2dResult || spring3dResult || beamResult || torsionResult || frameResult || planeResult);
  const currentStudyDomain = classifyStudyKindDomain(studyKind);
  const assistantStudyFamily = classifyStudyKindFamily(studyKind);
  const currentStudySample =
    SAMPLE_LIBRARY.find((sample) => sample.kind === studyKind) ??
    SAMPLE_LIBRARY.find(
      (sample) => classifyStudyKindDomain(sample.kind) === currentStudyDomain && classifyStudyKindFamily(sample.kind) === assistantStudyFamily,
    ) ??
    null;
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
    if (currentStudySample) {
      assistantCards.push({
        id: "sample",
        title: t.assistantOpenSample,
        summary: `${t.assistantOpenSampleHint} ${currentStudySample.name}.`,
        actionLabel: currentStudySample.name,
        tone: "good",
        onAction: () => {
          openSample(currentStudySample.href);
        },
      });
    }
    assistantCards.push({
      id: "controls",
      title: t.assistantReviewControls,
      summary: t.assistantReviewControlsHint,
      actionLabel: t.controls,
      tone: "watch",
      onAction: () => {
        openWorkspaceStudy("controls");
      },
    });
    assistantCards.push({
      id: "run",
      title: t.assistantRunStudy,
      summary: t.assistantRunStudyHint,
      actionLabel: t.run,
      tone: "good",
      onAction: runAnalysis,
    });
  } else {
    assistantCards.push({
      id: "report",
      title: t.assistantReviewReport,
      summary: t.assistantReviewReportHint,
      actionLabel: t.overview,
      tone: "good",
      onAction: () => {
        openWorkspaceStudy("summary");
      },
    });
    assistantCards.push({
      id: "export",
      title: t.assistantExportResult,
      summary: t.assistantExportResultHint,
      actionLabel: t.exportCsv,
      tone: "watch",
      onAction: downloadResultCsv,
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

  const assistantPromptPresets = [
    {
      id: "explain",
      label: t.assistantPromptExplain,
      prompt:
        language === "zh"
          ? `我现在在做 ${t.kinds[studyKind]}。请用小白能懂的话解释这个 study 是算什么的、最重要的输入是什么、我第一次运行前应该先检查哪三件事。`
          : language === "ja"
            ? `今は ${t.kinds[studyKind]} を扱っています。初心者にも分かる言葉で、この study が何を解くのか、重要な入力は何か、最初の実行前に確認すべき三つのことを教えてください。`
          : `I am working on ${t.kinds[studyKind]}. Explain this study in beginner-friendly language, name the most important inputs, and tell me the first three things to check before my first run.`,
    },
    {
      id: "materials",
      label: t.assistantPromptMaterial,
      prompt:
        language === "zh"
          ? `我不是材料专业的。针对 ${t.kinds[studyKind]}，请给我一套保守的起步材料参数建议，并说明哪些参数最值得先保持默认。`
          : language === "ja"
            ? `私は材料の専門家ではありません。${t.kinds[studyKind]} に対して、保守的な初期材料値の組み合わせを提案し、まずはどのパラメータを既定値に近いままにしておくのが安全か教えてください。`
          : `I am not a materials specialist. For ${t.kinds[studyKind]}, suggest a conservative starter set of material values and explain which parameters are safest to leave near defaults first.`,
    },
    {
      id: "boundary",
      label: t.assistantPromptBoundary,
      prompt:
        language === "zh"
          ? `请根据当前 ${t.kinds[studyKind]} 的上下文，帮我检查支撑和载荷应该怎么设才更像一个合理的第一轮仿真，并提醒我常见过约束或漏约束风险。`
          : language === "ja"
            ? `現在の ${t.kinds[studyKind]} の文脈に基づいて、最初の試行として妥当な支持条件と荷重の置き方を一緒に確認し、拘束不足や過拘束のよくある失敗も教えてください。`
          : `Given the current ${t.kinds[studyKind]} context, help me choose a sensible first-pass set of supports and loads, and warn me about common under-constrained or over-constrained mistakes.`,
    },
    {
      id: "results",
      label: t.assistantPromptResults,
      prompt:
        language === "zh"
          ? `请按 ${t.kinds[studyKind]} 的结果语义，告诉我当前最应该先看哪些结果字段，以及看到什么数量级时应该保持警惕。`
          : language === "ja"
            ? `${t.kinds[studyKind]} の結果の意味に沿って、まずどの結果フィールドを見るべきか、そしてどんな桁や傾向が出たら注意すべきか教えてください。`
          : `For ${t.kinds[studyKind]}, tell me which result fields I should read first and what kinds of magnitudes or patterns should make me cautious.`,
    },
  ];

  const scriptSnapshot: WorkbenchScriptSnapshot = {
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

  const recordManualDslAction = (action: string, payload: Record<string, unknown>) => {
    if (!scriptRecordingMode) return;

    appendScriptActionLog({
      action,
      source: "manual",
      status: "completed",
      summary: JSON.stringify(payload),
      payload,
      note: language === "zh" ? "手动 UI 录制" : language === "ja" ? "手動 UI 操作から記録" : "Recorded from manual UI interaction",
    });
  };

  const persistSecurityAuditEvent = async (entry: WorkbenchSecurityAuditEntry) => {
    try {
      await createSecurityEvent({
        event_id: entry.id,
        event_type: "security_high_risk_action",
        source: entry.source,
        action: entry.action,
        risk: entry.risk,
        status: entry.status,
        note: entry.note,
        occurred_at: entry.at,
        context: {
          frontend_runtime_mode: frontendRuntimeMode,
          study_kind: studyKind,
          project_id: selectedProjectId,
          model_id: selectedModelId,
          model_version_id: selectedVersionId,
          language,
          immersive_viewport: immersiveViewport,
        },
      });
    } catch {
      // Keep local audit logging available even when the control plane is unreachable.
    }
  };

  const recordSecurityAuditEvent = (entry: Omit<WorkbenchSecurityAuditEntry, "id" | "at">) => {
    const event = createSecurityAuditEntry(entry);
    setSecurityAuditLog((current) => [event, ...current].slice(0, 80));
    void persistSecurityAuditEvent(event);
  };

  const getScriptSnapshot = (): WorkbenchScriptSnapshot => ({
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
    hasResult: Boolean(result),
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
      switch (action) {
        case "macro/run": {
          const macroId = typeof payload.macroId === "string" ? payload.macroId : null;
          const macro = macroId ? getWorkbenchScriptMacroDefinition(macroId) : null;

          if (!macro) {
            throw new Error(language === "zh" ? "找不到指定的宏动作。" : language === "ja" ? "指定されたマクロが見つかりませんでした。" : "Could not find the requested macro.");
          }

          const macroPayload = Object.fromEntries(Object.entries(payload).filter(([key]) => key !== "macroId"));
          const macroSnapshot = getScriptSnapshot();

          for (const step of macro.steps) {
            const nextPayload = resolveWorkbenchMacroPayloadTemplates(step.payload ?? {}, macroPayload, macroSnapshot) as Record<string, unknown>;
            await invokeScriptAction(step.action, nextPayload, source, note ?? (language === "zh" ? macro.summary.zh : macro.summary.en));
          }

          resultPayload = { ok: true, action, macroId: macro.id, stepCount: macro.steps.length };
          break;
        }
        case "nav/setSidebarSection": {
          const section = payload.section;
          if (section === "study" || section === "model" || section === "library" || section === "system") {
            handleSidebarSectionChange(section);
          }
          resultPayload = { ok: true, action, section };
          break;
        }
        case "nav/setStudyKind": {
          const nextStudyKind = payload.studyKind;
          if (
            nextStudyKind === "axial_bar_1d" ||
            nextStudyKind === "heat_bar_1d" ||
            nextStudyKind === "heat_plane_triangle_2d" ||
            nextStudyKind === "heat_plane_quad_2d" ||
            nextStudyKind === "thermal_bar_1d" ||
            nextStudyKind === "thermal_beam_1d" ||
            nextStudyKind === "thermal_frame_2d" ||
            nextStudyKind === "thermal_truss_2d" ||
            nextStudyKind === "thermal_truss_3d" ||
            nextStudyKind === "thermal_plane_triangle_2d" ||
            nextStudyKind === "thermal_plane_quad_2d" ||
            nextStudyKind === "spring_1d" ||
            nextStudyKind === "spring_2d" ||
            nextStudyKind === "spring_3d" ||
            nextStudyKind === "beam_1d" ||
            nextStudyKind === "torsion_1d" ||
            nextStudyKind === "truss_2d" ||
            nextStudyKind === "truss_3d" ||
            nextStudyKind === "plane_triangle_2d" ||
            nextStudyKind === "plane_quad_2d" ||
            nextStudyKind === "frame_2d"
          ) {
            recordHistory(t.changeStudyType);
            if (nextStudyKind === "plane_quad_2d" && studyKind !== "plane_quad_2d") {
              setPlaneModel(ensurePlaneModelMaterials(defaultPlaneQuad, activeMaterial));
            } else if (nextStudyKind === "thermal_plane_quad_2d" && studyKind !== "thermal_plane_quad_2d") {
              setPlaneModel(ensurePlaneModelMaterials(defaultThermalPlaneQuad, activeMaterial));
            } else if (nextStudyKind === "plane_triangle_2d" && studyKind !== "plane_triangle_2d") {
              setPlaneModel(ensurePlaneModelMaterials(defaultPlaneTriangle, activeMaterial));
            } else if (nextStudyKind === "thermal_plane_triangle_2d" && studyKind !== "thermal_plane_triangle_2d") {
              setPlaneModel(ensurePlaneModelMaterials(defaultThermalPlaneTriangle, activeMaterial));
            } else if (nextStudyKind === "heat_bar_1d" && studyKind !== "heat_bar_1d") {
              setHeatBarModel(defaultHeatBar1d);
            } else if (nextStudyKind === "heat_plane_quad_2d" && studyKind !== "heat_plane_quad_2d") {
              setHeatPlaneModel(defaultHeatPlaneQuad);
              setPlaneResultField("average_temperature");
            } else if (nextStudyKind === "heat_plane_triangle_2d" && studyKind !== "heat_plane_triangle_2d") {
              setHeatPlaneModel(defaultHeatPlaneTriangle);
              setPlaneResultField("average_temperature");
            } else if (nextStudyKind === "thermal_bar_1d" && studyKind !== "thermal_bar_1d") {
              setThermalBarModel(defaultThermalBar1d);
            } else if (nextStudyKind === "thermal_beam_1d" && studyKind !== "thermal_beam_1d") {
              setThermalBeamModel(ensureBeamModelMaterials(defaultThermalBeam1d, activeMaterial));
            } else if (nextStudyKind === "thermal_frame_2d" && studyKind !== "thermal_frame_2d") {
              setThermalFrameModel(ensureFrameModelMaterials(defaultThermalFrame2d, activeMaterial) as ThermalFrameStudyJobInput);
            } else if (nextStudyKind === "thermal_truss_2d" && studyKind !== "thermal_truss_2d") {
              setThermalTrussModel(defaultThermalTruss2d);
            } else if (nextStudyKind === "thermal_truss_3d" && studyKind !== "thermal_truss_3d") {
              setThermalTruss3dModel(defaultThermalTruss3d);
            } else if (nextStudyKind === "spring_1d" && studyKind !== "spring_1d") {
              setSpringModel(defaultSpring1d);
            } else if (nextStudyKind === "spring_2d" && studyKind !== "spring_2d") {
              setSpring2dModel(defaultSpring2d);
            } else if (nextStudyKind === "spring_3d" && studyKind !== "spring_3d") {
              setSpring3dModel(defaultSpring3d);
            } else if (nextStudyKind === "beam_1d" && studyKind !== "beam_1d") {
              setBeamModel(ensureBeamModelMaterials(defaultBeam1d, activeMaterial));
            } else if (nextStudyKind === "torsion_1d" && studyKind !== "torsion_1d") {
              setTorsionModel(defaultTorsion1d);
            } else if (nextStudyKind === "frame_2d" && studyKind !== "frame_2d") {
              setFrameModel(ensureFrameModelMaterials(defaultFrame2d, activeMaterial));
            }
            setStudyKind(nextStudyKind);
          }
          resultPayload = { ok: true, action, studyKind: nextStudyKind };
          break;
        }
        case "nav/setTabs": {
          if (payload.studyTab === "summary" || payload.studyTab === "controls") {
            setStudyTab(payload.studyTab);
          }
          if (payload.modelTab === "tools" || payload.modelTab === "tree") {
            setModelTab(payload.modelTab);
          }
          if (
            payload.modelToolsPage === "overview" ||
            payload.modelToolsPage === "study" ||
            payload.modelToolsPage === "studio" ||
            payload.modelToolsPage === "materials" ||
            payload.modelToolsPage === "generate"
          ) {
            setModelToolsPage(payload.modelToolsPage);
          }
          if (
            payload.libraryTab === "results" ||
            payload.libraryTab === "samples" ||
            payload.libraryTab === "projects" ||
            payload.libraryTab === "models" ||
            payload.libraryTab === "jobs"
          ) {
            setLibraryTab(payload.libraryTab);
          }
          if (
            payload.systemPanelTab === "config" ||
            payload.systemPanelTab === "assistant" ||
            payload.systemPanelTab === "scripts" ||
            payload.systemPanelTab === "runtime" ||
            payload.systemPanelTab === "data"
          ) {
            if (payload.systemPanelTab === "assistant") {
              setAssistantWindowOpen(true);
              setSystemPanelTab("config");
            } else {
              setSystemPanelTab(payload.systemPanelTab);
            }
          }
          if (payload.systemDataTab === "jobs" || payload.systemDataTab === "results") {
            setSystemDataTab(payload.systemDataTab);
          }
          resultPayload = { ok: true, action };
          break;
        }
        case "settings/patch": {
          if (payload.language === "en" || payload.language === "zh" || payload.language === "ja" || payload.language === "es") {
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
          await Promise.all([refreshHealth(), refreshJobHistory(), refreshResults(), refreshProjects(), refreshSecurityEvents()]);
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
          const payloadRecord = payload as Record<string, unknown>;
          const nextStudyKind = payloadRecord.study_kind === "plane_quad_2d" ? "plane_quad_2d" : "plane_triangle_2d";
          setStudyKind(nextStudyKind);
          setPlaneModel(
            nextStudyKind === "plane_quad_2d"
              ? resolvePlaneQuad2dJobInput(payload as unknown as PlaneQuad2dJobInput)
              : resolvePlaneTriangle2dJobInput(payload as unknown as PlaneTriangle2dJobInput),
          );
          resetActiveResult(setResult, setJob);
          resultPayload = { ok: true, action, studyKind: nextStudyKind };
          break;
        }
        case "state/projectHeatToThermo": {
          const projectedStudyKind = projectHeatToThermoStudy();
          if (!projectedStudyKind) {
            throw new Error(
              language === "zh"
                ? "当前研究没有可映射的热结果，或暂不支持映射到力-热研究。"
                : language === "ja"
                  ? "現在の study には投影できる熱結果がないか、この熱→熱応力マッピングはまだ未対応です。"
                  : "The current study does not have a usable thermal result, or this thermo-mechanical projection is not supported yet.",
            );
          }
          resultPayload = { ok: true, action, studyKind: projectedStudyKind };
          break;
        }
        case "state/replaceFrameModel": {
          recordHistory(t.importAction);
          setStudyKind("frame_2d");
          setFrameModel(ensureFrameModelMaterials(payload as unknown as Frame2dJobInput, activeMaterial));
          resetActiveResult(setResult, setJob);
          resultPayload = { ok: true, action, studyKind: "frame_2d" };
          break;
        }
        case "state/replaceBeamModel": {
          recordHistory(t.importAction);
          setStudyKind("beam_1d");
          setBeamModel(ensureBeamModelMaterials(payload as unknown as Beam1dJobInput, activeMaterial));
          resetActiveResult(setResult, setJob);
          resultPayload = { ok: true, action, studyKind: "beam_1d" };
          break;
        }
        case "selection/set": {
          setSelectedNode(typeof payload.nodeIndex === "number" ? payload.nodeIndex : null);
          setSelectedElement(typeof payload.elementIndex === "number" ? payload.elementIndex : null);
          resultPayload = { ok: true, action };
          break;
        }
        case "selection/set3d": {
          if (Array.isArray(payload.nodeIndices)) {
            setSelectedTruss3dNodes(payload.nodeIndices.filter((entry): entry is number => typeof entry === "number"));
          }
          if (typeof payload.anchorNodeIndex === "number" || payload.anchorNodeIndex === null) {
            setSelectedNode(typeof payload.anchorNodeIndex === "number" ? payload.anchorNodeIndex : null);
          }
          if (Array.isArray(payload.memberDraftNodeIndices)) {
            setMemberDraftNodes(payload.memberDraftNodeIndices.filter((entry): entry is number => typeof entry === "number"));
          }
          if (typeof payload.linkMode === "boolean") {
            setTruss3dLinkMode(payload.linkMode);
          }
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
        case "viewport/setUiState": {
          if (typeof payload.immersiveViewport === "boolean" && payload.immersiveViewport !== immersiveViewport) {
            await toggleImmersiveViewport();
          }
          if (typeof payload.toolDrawerOpen === "boolean") {
            setImmersiveToolDrawerOpen(payload.toolDrawerOpen);
          }
          if (typeof payload.helpDrawerOpen === "boolean") {
            setImmersiveHelpDrawerOpen(payload.helpDrawerOpen);
          }
          if (typeof payload.boxSelectMode === "boolean") {
            setTruss3dBoxSelectMode(payload.boxSelectMode);
          }
          if (typeof payload.linkMode === "boolean") {
            setTruss3dLinkMode(payload.linkMode);
          }
          resultPayload = { ok: true, action };
          break;
        }
        case "data/setFilters": {
          if (payload.activeTab === "jobs" || payload.activeTab === "results") {
            setSystemDataTab(payload.activeTab);
          }
          if (typeof payload.projectId === "string" || payload.projectId === null) {
            setAdminFilterProjectId(typeof payload.projectId === "string" ? payload.projectId : "");
          }
          if (typeof payload.modelVersionId === "string" || payload.modelVersionId === null) {
            setAdminFilterModelVersionId(typeof payload.modelVersionId === "string" ? payload.modelVersionId : "");
          }
          setSidebarSection("system");
          setSystemPanelTab("data");
          resultPayload = { ok: true, action };
          break;
        }
        case "data/selectRecord": {
          if (payload.activeTab === "jobs" || payload.activeTab === "results") {
            setSystemDataTab(payload.activeTab);
          }
          if (typeof payload.jobId === "string") {
            setSelectedAdminJobId(payload.jobId);
          }
          if (typeof payload.resultJobId === "string") {
            setSelectedAdminResultJobId(payload.resultJobId);
          }
          setSidebarSection("system");
          setSystemPanelTab("data");
          resultPayload = { ok: true, action };
          break;
        }
        case "data/openLinkedContext": {
          const mode =
            payload.mode === "apply" || payload.mode === "project" || payload.mode === "version" ? payload.mode : "apply";
          const linkedJob = resolveScriptLinkedJob(payload);

          if (!linkedJob) {
            throw new Error(language === "zh" ? "找不到关联的数据记录上下文。" : language === "ja" ? "関連するデータレコード文脈を解決できませんでした。" : "Could not resolve the linked data record context.");
          }

          if (mode === "version") {
            if (!linkedJob.model_version_id) {
              throw new Error(language === "zh" ? "这条记录没有关联模型版本。" : language === "ja" ? "このレコードには関連モデルバージョンがありません。" : "This record does not have a linked model version.");
            }
            openModelVersionById(linkedJob.model_version_id);
          } else if (mode === "project") {
            if (!linkedJob.project_id) {
              throw new Error(language === "zh" ? "这条记录没有关联项目。" : language === "ja" ? "このレコードには関連プロジェクトがありません。" : "This record does not have a linked project.");
            }
            openProjectContextById(linkedJob.project_id);
          } else {
            applyJobContextToWorkbench(linkedJob);
          }

          resultPayload = {
            ok: true,
            action,
            mode,
            jobId: linkedJob.job_id,
            projectId: linkedJob.project_id ?? null,
            modelVersionId: linkedJob.model_version_id ?? null,
          };
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
    setMessage(language === "zh" ? "已回滚上一轮助手事务。" : language === "ja" ? "直前のアシスタント操作をロールバックしました。" : "Rolled back the last assistant transaction.");
  };

  const executeAssistantPlan = async (
    actions: Array<{ action: string; payload?: Record<string, unknown>; reason?: string }>,
    summary: string,
  ) => {
    const transactionId = recordAssistantTransaction(summary, actions.map((entry) => entry.action));
    try {
      for (const entry of actions) {
        await invokeScriptAction(entry.action, entry.payload ?? {}, "assistant", entry.reason);
      }
      setMessage(language === "zh" ? "助手计划已执行。" : language === "ja" ? "アシスタントのプランを実行しました。" : "Assistant plan executed.");
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
    if (isThermalTruss2d) {
      setThermalTrussModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode ? { ...node, [key]: value } : node,
        ),
      }));
      return;
    }
    setTrussModel((current) => updateTruss2dNode(current, selectedNode, key, value));
  };

  const updateSelectedElement = (
    key: keyof Truss2dJobInput["elements"][number],
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    if (isThermalTruss2d) {
      setThermalTrussModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }
    setTrussModel((current) => updateTruss2dElement(current, selectedElement, key, value));
  };

  const assignSelectedElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    if (isThermalTruss2d) {
      setThermalTrussModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, material_id: materialId } : element,
        ),
      }));
      return;
    }
    setTrussModel((current) => assignTruss2dElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedTruss3dNode = (
    key: keyof Truss3dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    if (isThermalTruss3d) {
      setThermalTruss3dModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode ? { ...node, [key]: value } : node,
        ),
      }));
      return;
    }
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
    if (isThermalTruss3d) {
      setThermalTruss3dModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }
    setTruss3dModel((current) => updateTruss3dElement(current, selectedElement, key, value));
  };

  const assignSelectedTruss3dElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    if (isThermalTruss3d) {
      setThermalTruss3dModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, material_id: materialId } : element,
        ),
      }));
      return;
    }
    setTruss3dModel((current) => assignTruss3dElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedPlaneNode = (
    key: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y" | "temperature_delta" | "fix_temperature" | "temperature" | "heat_load",
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    if (isHeatPlane) {
      setHeatPlaneModel((current) => updatePlaneNode(current, selectedNode, key, value) as HeatPlaneStudyJobInput);
      return;
    }
    setPlaneModel((current) => updatePlaneNode(current, selectedNode, key, value));
  };

  const updateSelectedPlaneElement = (
    key: "thickness" | "youngs_modulus" | "poisson_ratio" | "thermal_expansion" | "conductivity",
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    if (isHeatPlane) {
      setHeatPlaneModel((current) => updatePlaneElement(current, selectedElement, key, value) as HeatPlaneStudyJobInput);
      return;
    }
    setPlaneModel((current) => updatePlaneElement(current, selectedElement, key, value));
  };

  const assignSelectedPlaneElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    if (isHeatPlane) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setPlaneModel((current) => assignPlaneElementMaterial(current, selectedElement, materialId));
  };

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

    if (isThermalFrame) {
      setThermalFrameModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode ? { ...node, [key]: value } : node,
        ),
      }));
      return;
    }

    setFrameModel((current) => updateFrame2dNode(current, selectedNode, key as keyof Frame2dJobInput["nodes"][number], value));
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

    if (isThermalFrame) {
      setThermalFrameModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }

    setFrameModel((current) =>
      updateFrame2dElement(current, selectedElement, key as keyof Frame2dJobInput["elements"][number], value),
    );
  };

  const assignSelectedFrameElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    if (isTorsion) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    if (isThermalFrame) {
      setThermalFrameModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, material_id: materialId } : element,
        ),
      }));
      return;
    }
    setFrameModel((current) => assignFrame2dElementMaterial(current, selectedElement, materialId));
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

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => addPresetMaterialToFrameModel(current, activeMaterial) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => addPresetMaterialToFrameModel(current, activeMaterial));
      }
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

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => addCustomMaterialToFrameModel(current) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => addCustomMaterialToFrameModel(current));
      }
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

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => applyMaterialToFrameModel(current, materialId, mode, selectedElement) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => applyMaterialToFrameModel(current, materialId, mode, selectedElement));
      }
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
      } else if (studyKind === "frame_2d") {
        setFrameModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else {
        setPlaneModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      }

      setMessage(language === "zh" ? "外部材料库已导入。" : language === "ja" ? "外部材料ライブラリを取り込みました。" : "Imported external material library.");
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

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => updateMaterialInFrameModel(current, materialId, field, value) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => updateMaterialInFrameModel(current, materialId, field, value));
      }
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

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => deleteMaterialFromFrameModel(current, materialId) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => deleteMaterialFromFrameModel(current, materialId));
      }
      return;
    }

    setPlaneModel((current) => deleteMaterialFromPlaneModel(current, materialId));
  };

  const addNode = (connectToSelected: boolean) => {
    if (isFrameLike) {
      recordHistory(t.addNodeAction);
      setStudyKind(studyKind);
      setSidebarSection("model");
      resetActiveResult(setResult, setJob);
      const nextState = addFrame2dNode(activeFrameLikeModel, connectToSelected, selectedNode, round);
      if (isThermalFrame) {
        setThermalFrameModel(nextState.model as ThermalFrameStudyJobInput);
      } else {
        setFrameModel(nextState.model);
      }
      setSelectedNode(nextState.nextSelectedNode);
      setSelectedElement(nextState.nextSelectedElement);
      setMemberDraftNodes([]);
      setMessage(nextState.createdBranch ? t.branchCreated : t.nodeCreated);
      return;
    }
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
    if (isFrameLike) {
      if (isThermalFrame) {
        setThermalFrameModel((current) => deleteFrame2dNode(current, selectedNode) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => deleteFrame2dNode(current, selectedNode));
      }
      setSelectedNode(null);
      setSelectedTruss3dNodes([]);
      setSelectedElement(null);
      setMemberDraftNodes([]);
      setMessage(t.nodeDeleted);
      return;
    }
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
    if (isFrameLike) {
      const nextState = toggleFrame2dMember(activeFrameLikeModel, memberDraftNodes);
      if (!nextState.valid) return;
      if (isThermalFrame) {
        setThermalFrameModel(nextState.model as ThermalFrameStudyJobInput);
      } else {
        setFrameModel(nextState.model);
      }
      setSelectedElement(nextState.removedExisting ? null : nextState.nextSelectedElement);
      setSelectedTruss3dNodes([]);
      setMemberDraftNodes([]);
      setMessage(nextState.removedExisting ? t.memberRemoved : t.memberCreated);
      return;
    }
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
    if (isFrameLike) {
      if (isThermalFrame) {
        setThermalFrameModel((current) => deleteFrame2dElement(current, selectedElement) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => deleteFrame2dElement(current, selectedElement));
      }
      setSelectedElement(null);
      setMessage(t.memberDeleted);
      return;
    }
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
    if (draggingNode === null || (studyKind !== "truss_2d" && studyKind !== "frame_2d" && studyKind !== "thermal_frame_2d")) return;
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
      if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
        if (studyKind === "thermal_frame_2d") {
          setThermalFrameModel((current) =>
            updateFrame2dNode(updateFrame2dNode(current, draggingNode, "x", nextPoint.x), draggingNode, "y", nextPoint.y) as ThermalFrameStudyJobInput,
          );
        } else {
          setFrameModel((current) =>
            updateFrame2dNode(updateFrame2dNode(current, draggingNode, "x", nextPoint.x), draggingNode, "y", nextPoint.y),
          );
        }
        return;
      }

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

  const studyKindOptionGroups = buildStudyKindOptionGroups({
    kinds: t.kinds,
    domains: t.studyDomains,
    families: t.studyFamilies,
  });
  const studyDomainOptions = buildStudyDomainOptions(t.studyDomains);
  const currentStudyFamily = classifyStudyKindFamily(studyKind);
  const currentStudyFamilyLabel = t.studyFamilies[currentStudyFamily];
  const currentStudyFamilyHint = t.familyHints[currentStudyFamily];
  const joinThermalIntent = (...parts: Array<string | null | undefined | false>) => parts.filter(Boolean).join(" + ");
  const countRestrainedFrameLikeNodes = (nodes: Array<{ fix_x?: boolean; fix_y?: boolean; fix_rz?: boolean }>) =>
    nodes.reduce((sum, node) => sum + (node.fix_x || node.fix_y || node.fix_rz ? 1 : 0), 0);
  const countRestrainedTruss2dNodes = (nodes: Array<{ fix_x: boolean; fix_y: boolean }>) =>
    nodes.reduce((sum, node) => sum + (node.fix_x || node.fix_y ? 1 : 0), 0);
  const countRestrainedTruss3dNodes = (nodes: Array<{ fix_x: boolean; fix_y: boolean; fix_z: boolean }>) =>
    nodes.reduce((sum, node) => sum + (node.fix_x || node.fix_y || node.fix_z ? 1 : 0), 0);
  const thermalPlaneHeatedNodeCount =
    isThermalPlaneTriangle || isThermalPlaneQuad
      ? activePlaneInputModel.nodes.reduce((sum, node) => {
          const thermalNode = node as { temperature_delta?: number };
          return sum + (typeof thermalNode.temperature_delta === "number" && Math.abs(thermalNode.temperature_delta) > 0 ? 1 : 0);
        }, 0)
      : 0;
  const thermalPlaneRestrainedNodeCount =
    isThermalPlaneTriangle || isThermalPlaneQuad
      ? activePlaneInputModel.nodes.reduce((sum, node) => {
          const supportNode = node as { fix_x?: boolean; fix_y?: boolean };
          return sum + (supportNode.fix_x || supportNode.fix_y ? 1 : 0);
        }, 0)
      : 0;
  const thermalIntentValue = isHeatBar
    ? joinThermalIntent(t.conductionField, heatBarModel.nodes.some((node) => Math.abs(node.heat_load ?? 0) > 0) && t.heatSourceField)
    : isHeatPlane
      ? joinThermalIntent(
          t.conductionField,
          activePlaneInputModel.nodes.some((node) => "heat_load" in node && Math.abs(node.heat_load ?? 0) > 0) && t.heatSourceField,
        )
      : isThermalBar
        ? joinThermalIntent(t.nodalTemperatureRise, t.thermalBarResponse)
      : isThermalBeam
        ? joinThermalIntent(t.memberTemperatureGradient, t.thermalBeamResponse)
      : isThermalFrame
        ? joinThermalIntent(
            thermalFrameModel.nodes.some((node) => Math.abs(node.temperature_delta ?? 0) > 0) && t.nodalTemperatureRise,
            thermalFrameModel.elements.some((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0) && t.memberTemperatureGradient,
            t.thermalFrameResponse,
          )
      : isThermalTruss2d
        ? joinThermalIntent(t.nodalTemperatureRise, t.thermalTrussResponse)
      : studyKind === "thermal_truss_3d"
        ? joinThermalIntent(t.nodalTemperatureRise, t.thermalTrussResponse)
      : isThermalPlaneTriangle || isThermalPlaneQuad
        ? joinThermalIntent(t.nodalTemperatureRise, t.thermoelasticPlaneResponse)
        : undefined;
  const thermalBoundaryValue = isHeatBar
    ? `${heatBarModel.nodes.filter((node) => node.fix_temperature).length} ${t.prescribedTemperatureNodes} · ${heatBarModel.nodes.filter((node) => Math.abs(node.heat_load ?? 0) > 0).length} ${t.sourceNodes}`
    : isHeatPlane
      ? `${activePlaneInputModel.nodes.reduce((sum, node) => sum + (("fix_temperature" in node && node.fix_temperature) ? 1 : 0), 0)} ${t.prescribedTemperatureNodes} · ${activePlaneInputModel.nodes.reduce((sum, node) => sum + (("heat_load" in node && Math.abs(node.heat_load ?? 0) > 0) ? 1 : 0), 0)} ${t.sourceNodes}`
      : isThermalBar
        ? `${thermalBarModel.nodes.filter((node) => Math.abs(node.temperature_delta) > 0).length} ${t.heatedNodes} · ${thermalBarModel.nodes.filter((node) => node.fix_x).length} ${t.restrainedSupports}`
      : isThermalBeam
        ? `${thermalBeamModel.elements.filter((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0).length} ${t.gradientMembers} · ${countRestrainedFrameLikeNodes(thermalBeamModel.nodes)} ${t.restrainedSupports}`
      : isThermalFrame
        ? `${thermalFrameModel.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length} ${t.heatedNodes} · ${thermalFrameModel.elements.filter((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0).length} ${t.gradientMembers} · ${countRestrainedFrameLikeNodes(thermalFrameModel.nodes)} ${t.restrainedSupports}`
      : isThermalTruss2d
        ? `${thermalTrussModel.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length} ${t.heatedNodes} · ${countRestrainedTruss2dNodes(thermalTrussModel.nodes)} ${t.restrainedSupports}`
      : studyKind === "thermal_truss_3d"
        ? `${thermalTruss3dModel.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length} ${t.heatedNodes} · ${countRestrainedTruss3dNodes(thermalTruss3dModel.nodes)} ${t.restrainedSupports}`
      : isThermalPlaneTriangle || isThermalPlaneQuad
        ? `${thermalPlaneHeatedNodeCount} ${t.heatedNodes} · ${thermalPlaneRestrainedNodeCount} ${t.restrainedSupports}`
        : undefined;
  const studySummaryRows = buildStudySummaryRows({
    labels: {
      modelName: t.modelName,
      material: t.material,
      mesh: t.mesh,
      load: t.load,
      support: t.support,
    },
    loadedModelName,
    materialLabel: isSpring || isHeatBar || isThermalBar || isTorsion ? "--" : localMaterialLabel(activeMaterial, language),
    meshValue: isAxial
      ? axialForm.elements
      : isHeatBar
        ? heatBarModel.elements.length
      : isThermalBar
        ? thermalBarModel.elements.length
      : isThermalBeam
        ? thermalBeamModel.elements.length
      : isThermalTruss2d
        ? thermalTrussModel.elements.length
      : isSpring1d
        ? springModel.elements.length
      : isSpring2d
        ? spring2dModel.elements.length
      : isSpring3d
        ? spring3dModel.elements.length
      : isBeam
        ? beamModel.elements.length
      : isTorsion
        ? torsionModel.elements.length
      : isTruss
        ? trussModel.elements.length
        : isTruss3d
          ? studyKind === "thermal_truss_3d"
            ? thermalTruss3dModel.elements.length
            : truss3dModel.elements.length
          : studyKind === "frame_2d"
            ? frameModel.elements.length
          : activePlaneInputModel.elements.length,
    loadValue: isAxial
      ? `${fixed(axialForm.tipForce, 0)} N`
      : isHeatBar
        ? `${fixed(heatBarModel.nodes.reduce((sum, node) => sum + node.heat_load, 0), 0)} W · T ${fixed(heatBarModel.nodes.reduce((max, node) => Math.max(max, node.temperature), 0), 1)} °`
      : isThermalBar
        ? `${fixed(thermalBarModel.nodes.reduce((sum, node) => sum + node.load_x, 0), 0)} N · ΔT ${fixed(thermalBarModel.nodes.reduce((sum, node) => sum + Math.abs(node.temperature_delta), 0), 1)} °`
      : isThermalBeam
        ? `${fixed(thermalBeamModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N · ${fixed(thermalBeamModel.nodes.reduce((sum, node) => sum + node.moment_z, 0), 0)} N·m · q ${fixed(thermalBeamModel.elements.reduce((sum, element) => sum + Math.abs(element.distributed_load_y ?? 0), 0), 0)} N/m · ΔTy ${fixed(thermalBeamModel.elements.reduce((sum, element) => sum + Math.abs(element.temperature_gradient_y ?? 0), 0), 1)} °`
      : isThermalTruss2d
        ? `${fixed(thermalTrussModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y), 0), 0)} N · ΔT ${fixed(thermalTrussModel.nodes.reduce((sum, node) => sum + Math.abs(node.temperature_delta), 0), 1)} °`
      : isSpring1d
        ? `${fixed(springModel.nodes.reduce((sum, node) => sum + node.load_x, 0), 0)} N`
      : isSpring2d
        ? `${fixed(spring2dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y), 0), 0)} N`
      : isSpring3d
        ? `${fixed(spring3dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y, node.load_z), 0), 0)} N`
      : isBeam
        ? `${fixed(beamModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N · ${fixed(beamModel.nodes.reduce((sum, node) => sum + node.moment_z, 0), 0)} N·m · ${fixed(beamModel.elements.reduce((sum, element) => sum + Math.abs(element.distributed_load_y ?? 0), 0), 0)} N/m`
      : isTorsion
        ? `${fixed(torsionModel.nodes.reduce((sum, node) => sum + Math.abs(node.torque_z), 0), 0)} N·m`
      : isTruss
        ? `${fixed(trussModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`
      : isTruss3d
          ? studyKind === "thermal_truss_3d"
            ? `${fixed(thermalTruss3dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y, node.load_z), 0), 0)} N · ΔT ${fixed(thermalTruss3dModel.nodes.reduce((sum, node) => sum + Math.abs(node.temperature_delta), 0), 1)} °`
            : `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + node.load_z, 0), 0)} N`
          : studyKind === "frame_2d"
            ? `${fixed(frameModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N · ${fixed(frameModel.nodes.reduce((sum, node) => sum + node.moment_z, 0), 0)} N·m`
          : isHeatPlane
            ? `${fixed(activePlaneInputModel.nodes.reduce((sum, node) => sum + ("heat_load" in node ? (node.heat_load ?? 0) : 0), 0), 0)} W · T ${fixed(activePlaneInputModel.nodes.reduce((max, node) => Math.max(max, "temperature" in node ? (node.temperature ?? 0) : 0), 0), 1)} °`
            : `${fixed((activePlaneInputModel.nodes as PlaneTriangle2dJobInput["nodes"]).reduce((sum, node) => sum + node.load_y, 0), 0)} N`,
    supportValue: isAxial
      ? "Node 0"
      : isHeatBar
        ? `${heatBarModel.nodes.filter((node) => node.fix_temperature).length} prescribed T`
      : isThermalBar
        ? "Restrained span"
        : isThermalBeam
          ? "Thermal cantilever"
        : isThermalTruss2d
          ? "Thermal truss anchors"
        : isSpring1d
          ? "Axial anchor"
          : isSpring2d
            ? "Planar anchors"
            : isSpring3d
              ? "Spatial anchors"
              : isTruss3d
                ? "Fixed tripod"
                : isTorsion
                  ? "Fixed shaft end"
                  : isHeatPlane
                    ? `${activePlaneInputModel.nodes.reduce((sum, node) => sum + (("fix_temperature" in node && node.fix_temperature) ? 1 : 0), 0)} prescribed T`
                  : isFrameLike || isBeam
                    ? "Moment-resisting base"
                    : "Pinned base",
  });

  const studyControlsRows = buildStudyControlsRows({
    labels: {
      nodes: t.nodes,
      trussElements: t.trussElements,
      material: t.material,
      sourceModel: t.sourceModel,
      spatialTrussElements: t.spatialTrussElements,
      load: t.load,
      planeElements: t.planeElements,
      frameElements: t.frameElements,
      thickness: t.thickness,
      thermalIntent: t.thermalIntent,
      thermalBoundary: t.thermalBoundary,
    },
    studyKind,
    loadedModelName,
    materialLabel: localMaterialLabel(activeMaterial, language),
    trussNodeCount: isFrameLike ? activeFrameLikeModel.nodes.length : trussModel.nodes.length,
    trussElementCount: isFrameLike ? activeFrameLikeModel.elements.length : trussModel.elements.length,
    truss3dNodeCount: isSpring3d ? spring3dModel.nodes.length : truss3dModel.nodes.length,
    truss3dElementCount: isSpring3d ? spring3dModel.elements.length : truss3dModel.elements.length,
    truss3dLoadValue: isSpring3d
      ? `${fixed(spring3dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y, node.load_z), 0), 0)} N`
      : `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + node.load_z, 0), 0)} N`,
    planeNodeCount:
      studyKind === "frame_2d" || studyKind === "thermal_frame_2d"
        ? activeFrameLikeModel.nodes.length
        : studyKind === "beam_1d"
          ? beamModel.nodes.length
          : studyKind === "thermal_beam_1d"
            ? thermalBeamModel.nodes.length
          : studyKind === "torsion_1d"
            ? torsionModel.nodes.length
          : studyKind === "thermal_bar_1d"
            ? thermalBarModel.nodes.length
          : studyKind === "spring_1d"
            ? springModel.nodes.length
            : studyKind === "spring_2d"
              ? spring2dModel.nodes.length
              : studyKind === "spring_3d"
                ? spring3dModel.nodes.length
            : activePlaneInputModel.nodes.length,
    planeElementCount:
      studyKind === "frame_2d" || studyKind === "thermal_frame_2d"
        ? activeFrameLikeModel.elements.length
        : studyKind === "beam_1d"
          ? beamModel.elements.length
          : studyKind === "thermal_beam_1d"
            ? thermalBeamModel.elements.length
          : studyKind === "torsion_1d"
            ? torsionModel.elements.length
          : studyKind === "heat_bar_1d"
            ? heatBarModel.elements.length
          : studyKind === "thermal_bar_1d"
            ? thermalBarModel.elements.length
          : studyKind === "spring_1d"
            ? springModel.elements.length
            : studyKind === "spring_2d"
              ? spring2dModel.elements.length
              : studyKind === "spring_3d"
                ? spring3dModel.elements.length
            : activePlaneInputModel.elements.length,
    planeThicknessValue:
      studyKind === "frame_2d" ||
      studyKind === "thermal_frame_2d" ||
      studyKind === "beam_1d" ||
      studyKind === "thermal_beam_1d" ||
      studyKind === "torsion_1d" ||
      studyKind === "heat_bar_1d" ||
      studyKind === "thermal_bar_1d" ||
      studyKind === "spring_1d" ||
      studyKind === "spring_2d" ||
      studyKind === "spring_3d"
        ? "--"
        : fixed(activePlaneInputModel.elements[0]?.thickness, 3),
    thermalIntentValue,
    thermalBoundaryValue,
  });
  const truss3dTreeRows = buildTruss3dTreeRows({
    nodes: isSpring3d ? spring3dModel.nodes : truss3dModel.nodes,
    elements: displayTruss3dElements,
    selectedNode,
    selectedTruss3dNodes,
    memberDraftNodes,
    fixed,
  });

  const selectStudyKind = (nextStudyKind: typeof studyKind) => {
    recordHistory(t.changeStudyType);
    if (nextStudyKind === "plane_quad_2d" && studyKind !== "plane_quad_2d") {
      setPlaneModel(ensurePlaneModelMaterials(defaultPlaneQuad, activeMaterial));
    } else if (nextStudyKind === "plane_triangle_2d" && studyKind !== "plane_triangle_2d") {
      setPlaneModel(ensurePlaneModelMaterials(defaultPlaneTriangle, activeMaterial));
    } else if (nextStudyKind === "heat_bar_1d" && studyKind !== "heat_bar_1d") {
      setHeatBarModel(defaultHeatBar1d);
    } else if (nextStudyKind === "heat_plane_quad_2d" && studyKind !== "heat_plane_quad_2d") {
      setHeatPlaneModel(defaultHeatPlaneQuad);
      setPlaneResultField("average_temperature");
    } else if (nextStudyKind === "heat_plane_triangle_2d" && studyKind !== "heat_plane_triangle_2d") {
      setHeatPlaneModel(defaultHeatPlaneTriangle);
      setPlaneResultField("average_temperature");
    } else if (nextStudyKind === "thermal_bar_1d" && studyKind !== "thermal_bar_1d") {
      setThermalBarModel(defaultThermalBar1d);
    } else if (nextStudyKind === "thermal_beam_1d" && studyKind !== "thermal_beam_1d") {
      setThermalBeamModel(ensureBeamModelMaterials(defaultThermalBeam1d, activeMaterial));
    } else if (nextStudyKind === "thermal_truss_2d" && studyKind !== "thermal_truss_2d") {
      setThermalTrussModel(defaultThermalTruss2d);
    } else if (nextStudyKind === "thermal_truss_3d" && studyKind !== "thermal_truss_3d") {
      setThermalTruss3dModel(defaultThermalTruss3d);
    } else if (nextStudyKind === "spring_1d" && studyKind !== "spring_1d") {
      setSpringModel(defaultSpring1d);
    } else if (nextStudyKind === "spring_2d" && studyKind !== "spring_2d") {
      setSpring2dModel(defaultSpring2d);
    } else if (nextStudyKind === "spring_3d" && studyKind !== "spring_3d") {
      setSpring3dModel(defaultSpring3d);
    } else if (nextStudyKind === "beam_1d" && studyKind !== "beam_1d") {
      setBeamModel(ensureBeamModelMaterials(defaultBeam1d, activeMaterial));
    } else if (nextStudyKind === "torsion_1d" && studyKind !== "torsion_1d") {
      setTorsionModel(defaultTorsion1d);
    } else if (nextStudyKind === "frame_2d" && studyKind !== "frame_2d") {
      setFrameModel(ensureFrameModelMaterials(defaultFrame2d, activeMaterial));
    }
    setStudyKind(nextStudyKind);
  };

  const openWorkspaceStudy = (tab: StudyPanelTab = "controls") => {
    setSidebarSection("model");
    setModelTab("tools");
    setModelToolsPage("study");
    setStudyTab(tab);
  };

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
      <aside className="app-rail panel">
        <div className="rail-brand">
          <img alt={`${t.shortTitle} mark`} className="rail-brand__mark" src="/kyuubiki.png" />
          <strong>{t.shortTitle}</strong>
          <span>tamamono 1.0.0</span>
        </div>
        <div className="rail-nav">
          {railItems.map((item) => (
            <button
              key={item.key}
              className={`rail-button${sidebarSection === item.key ? " rail-button--active" : ""}`}
              onClick={() => handleSidebarSectionChange(item.key)}
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
          <div className="sidebar-header__brand">
            <img alt={`${t.shortTitle} mark`} className="sidebar-header__mark" src="/kyuubiki.png" />
            <p className="eyebrow">{t.roleLabel}</p>
          </div>
          <h1>{t.title}</h1>
          <p>{t.subtitle}</p>
        </div>

        {sidebarSection === "study" ? (
          <WorkbenchStudySidebar
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
        ) : null}

        {sidebarSection === "model" ? (
          <WorkbenchModelSidebar
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
        ) : null}

        {sidebarSection === "workflow" ? (
          <WorkbenchWorkflowSidebar
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
            onRefreshWorkflowCatalog={() => void refreshWorkflowCatalog()}
            onSelectWorkflow={setSelectedWorkflowId}
            onRunWorkflowCatalog={runWorkflowCatalogEntry}
            onOpenWorkflowRun={openHistoryJob}
          />
        ) : null}

        {sidebarSection === "library" ? (
          <WorkbenchLibrarySidebar
            libraryTab={libraryTab}
            onLibraryTabChange={handleLibraryTabChange}
            labels={t}
            sampleRows={librarySampleRows}
            workflowCatalogEntries={workflowCatalog}
            workflowCatalogBusy={workflowCatalogBusy}
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
            selectedProjectModelCount={selectedProjectModels.length}
            modelRows={libraryModelRows}
            selectedModelId={selectedModelId}
            loadedModelName={loadedModelName}
            onLoadedModelNameChange={setLoadedModelName}
            onSaveModel={saveModelVersion}
            onDeleteSavedModel={deleteSavedModelRecord}
            onOpenSavedModel={(modelId) => {
              const model = selectedProjectModels.find((entry) => entry.model_id === modelId);
              if (model) openSavedModel(model);
            }}
            versionRows={libraryVersionRows}
            modelVersionCount={modelVersions.length}
            selectedVersionId={selectedVersionId}
            onRenameSelectedVersion={renameSelectedVersion}
            onDeleteSelectedVersion={deleteSelectedVersion}
            onOpenSavedVersion={(versionId) => {
              const version = modelVersions.find((entry) => entry.version_id === versionId);
              if (version) openSavedVersion(version);
            }}
            jobRows={libraryJobRows}
            jobCount={jobHistory.length}
            activeJobId={job?.job_id ?? null}
            onOpenHistoryJob={openHistoryJob}
            onOpenSample={openSample}
            onRefreshWorkflowCatalog={() => void refreshWorkflowCatalog()}
            onRunWorkflowCatalog={runWorkflowCatalogEntry}
            onRefresh={() => {
              void refreshJobHistory();
              void refreshProjects();
            }}
            onImportModel={importModel}
          />
        ) : null}

        {sidebarSection === "system" ? (
          <WorkbenchSystemSidebar
            systemPanelTab={systemPanelTab === "assistant" ? "config" : systemPanelTab}
            onSystemPanelTabChange={handleSystemPanelTabChange}
            settingsTabLabel={t.settings}
            overviewPageLabel={t.overview}
            configPageLabel={t.config}
            scriptsPageLabel={t.scripts}
            runtimeTabLabel={t.runtime}
            dataTabLabel={t.data}
            configOverviewHint={t.settingsConfigHint}
            scriptsOverviewHint={t.settingsScriptsHint}
            configContent={
              <WorkbenchSystemConfigCard
                title={t.settings}
                status={health?.status === "ok" ? t.online : t.offline}
                workspacePageLabel={t.workspace}
                routingPageLabel={t.routing}
                accessPageLabel={t.access}
                packsPageLabel={t.packs}
                themeLabel={t.theme}
                languageLabel={t.language}
                languagePacksTitle={t.languagePacksTitle}
                languagePacksHint={t.languagePacksHint}
                languagePacksEmptyLabel={t.languagePacksEmptyLabel}
                languagePackNameLabel={t.languagePackName}
                languagePackVersionLabel={t.languagePackVersion}
                languagePackSourceImportedLabel={t.languagePackSourceImported}
                languagePackSourceDownloadedLabel={t.languagePackSourceDownloaded}
                languagePackDownloadTemplateLabel={t.languagePackDownloadTemplate}
                languagePackExportInstalledLabel={t.languagePackExportInstalled}
                languagePackImportLabel={t.languagePackImport}
                languagePackRemoveLabel={t.languagePackRemove}
                languagePackCatalogTitle={t.languagePackCatalogTitle}
                languagePackCatalogHint={t.languagePackCatalogHint}
                languagePackCatalogActionLabel={t.languagePackCatalogAction}
                frontendModeLabel={t.frontendMode}
                directMeshStrategyLabel={t.directMeshStrategy}
                directMeshEndpointsLabel={t.directMeshEndpoints}
                directMeshEndpointsHelp={t.directMeshEndpointsHelp}
                controlPlaneTokenLabel={securityUi.controlPlaneToken}
                controlPlaneTokenHelp={t.controlPlaneTokenHelp}
                controlPlaneTokenPlaceholder={t.controlPlaneTokenPlaceholder}
                clusterTokenLabel={securityUi.clusterToken}
                clusterTokenHelp={t.clusterTokenHelp}
                clusterTokenPlaceholder={t.clusterTokenPlaceholder}
                directMeshTokenLabel={securityUi.directMeshToken}
                directMeshTokenHelp={t.directMeshTokenHelp}
                directMeshTokenPlaceholder={t.directMeshTokenPlaceholder}
                shortcutHintsLabel={t.shortcutHints}
                shortcutHintsHelp={t.shortcutHintsHelp}
                immersiveGuardLabel={t.immersiveGuard}
                immersiveGuardHelp={t.immersiveGuardHelp}
                browserLimitsNote={t.browserLimitsNote}
                exportDatabaseLabel={t.exportDatabase}
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
                themeOptions={[
                  { value: "linen", label: t.themes.linen },
                  { value: "marine", label: t.themes.marine },
                  { value: "graphite", label: t.themes.graphite },
                ]}
                languageOptions={[
                  { value: "en", label: t.languages.en },
                  { value: "zh", label: t.languages.zh },
                  { value: "ja", label: t.languages.ja },
                  { value: "es", label: t.languages.es },
                ]}
                installedLanguagePacks={languagePacks}
                catalogLanguagePacks={languagePackCatalogRows}
                frontendModeOptions={[
                  { value: "orchestrated_gui", label: t.frontendModes.orchestrated_gui },
                  { value: "direct_mesh_gui", label: t.frontendModes.direct_mesh_gui },
                ]}
                directMeshStrategyOptions={[
                  { value: "healthiest", label: t.directMeshStrategies.healthiest },
                  { value: "first_reachable", label: t.directMeshStrategies.first_reachable },
                ]}
                onThemeChange={setTheme}
                onLanguageChange={handleLanguageChange}
                onDownloadLanguagePackTemplate={handleDownloadLanguagePackTemplate}
                onExportInstalledLanguagePack={handleExportInstalledLanguagePack}
                onImportLanguagePack={(file) => void handleImportLanguagePack(file)}
                onRemoveLanguagePack={handleRemoveLanguagePack}
                onFrontendRuntimeModeChange={setFrontendRuntimeMode}
                onDirectMeshSelectionModeChange={setDirectMeshSelectionMode}
                onDirectMeshEndpointsTextChange={setDirectMeshEndpointsText}
                onControlPlaneApiTokenChange={setControlPlaneApiToken}
                onClusterApiTokenChange={setClusterApiToken}
                onDirectMeshApiTokenChange={setDirectMeshApiToken}
                onShowShortcutHintsChange={setShowShortcutHints}
                onImmersiveGuardrailsChange={setImmersiveGuardrails}
                onExportDatabase={() => void downloadDatabaseSnapshot()}
              />
            }
            scriptsContent={
              <WorkbenchScriptPanel
                actionLog={scriptActionLog}
                getSnapshot={getScriptSnapshot}
                language={language}
                recordingMode={scriptRecordingMode}
                onInvokeAction={invokeScriptAction}
                onToggleRecordingMode={() => setScriptRecordingMode((current) => !current)}
                snapshot={scriptSnapshot}
              />
            }
            runtimeContent={
              <WorkbenchSystemRuntimePanel
                overviewTabLabel={t.overview}
                stackTabLabel={t.stack}
                securityTabLabel={t.security}
                agentsTabLabel={t.agents}
                auditTabLabel={t.audit}
                watchdogTabLabel={t.watchdog}
                backendTitle={t.backend}
                backendStatus={health?.status ?? t.offline}
                backendRows={runtimeBackendRows}
                protocolsTitle={t.protocols}
                protocolsStatus={health?.protocol ? t.online : t.offline}
                protocolRows={runtimeProtocolRows}
                protocolMethods={runtimeProtocolMethods}
                securityTitle={securityUi.security}
                securityStatus={health?.security?.api_token_configured ? securityUi.configured : securityUi.notConfigured}
                securityRows={runtimeSecurityRows}
                securityFooter={<p className="card-copy">{t.runtimeSecurityFooter}</p>}
                auditTitle={t.securityAudit}
                auditCountLabel={String(securityEventRecords.length)}
                auditEmptyLabel={language === "zh" ? "当前筛选下还没有安全事件。" : language === "ja" ? "現在のフィルターに一致するセキュリティイベントはありません。" : "No security events match the current filters."}
                auditSessionLabel={t.auditSessionLabel}
                auditWindowLabel={t.auditWindow}
                auditSourceLabel={t.auditSource}
                auditRiskLabel={t.auditRisk}
                auditStatusLabel={t.auditStatus}
                auditActionLabel={t.auditAction}
                auditSummaryTitle={t.auditSummaryTitle}
                auditSummaryRows={runtimeAuditSummaryRows}
                auditTrendTitle={t.auditTrendTitle}
                auditTrendEmptyLabel={t.auditTrendEmptyLabel}
                auditTrendBars={runtimeAuditTrendBars}
                auditSourceStatusTitle={t.auditSourceStatusTitle}
                auditSourceStatusFacets={runtimeAuditSourceStatusFacets}
                auditStudyFacetTitle={t.auditStudyFacetTitle}
                auditProjectFacetTitle={t.auditProjectFacetTitle}
                auditModelVersionFacetTitle={t.auditModelVersionFacetTitle}
                auditFacetEmptyLabel={t.auditFacetEmptyLabel}
                auditStudyFacets={runtimeAuditStudyFacets}
                auditProjectFacets={runtimeAuditProjectFacets}
                auditModelVersionFacets={runtimeAuditModelVersionFacets}
                auditRefreshLabel={t.auditRefreshLabel}
                auditExportLabel={t.auditExportLabel}
                auditExportCsvLabel={t.auditExportCsvLabel}
                auditWindowValue={securityEventWindowFilter}
                auditSourceValue={securityEventSourceFilter}
                auditRiskValue={securityEventRiskFilter}
                auditStatusValue={securityEventStatusFilter}
                auditActionValue={securityEventActionFilter}
                auditWindowOptions={[
                  { value: "", label: t.auditWindowOptions.all },
                  { value: "1h", label: t.auditWindowOptions.h1 },
                  { value: "24h", label: t.auditWindowOptions.h24 },
                  { value: "7d", label: t.auditWindowOptions.d7 },
                  { value: "30d", label: t.auditWindowOptions.d30 },
                ]}
                auditSourceOptions={[
                  { value: "", label: t.auditSourceOptions.all },
                  { value: "assistant", label: t.auditSourceOptions.assistant },
                  { value: "hub-assistant", label: t.auditSourceOptions.hubAssistant },
                  { value: "script", label: t.auditSourceOptions.script },
                ]}
                auditRiskOptions={[
                  { value: "", label: t.auditRiskOptions.all },
                  { value: "low", label: t.auditRiskOptions.low },
                  { value: "sensitive", label: t.auditRiskOptions.sensitive },
                  { value: "high", label: t.auditRiskOptions.high },
                  { value: "destructive", label: t.auditRiskOptions.destructive },
                ]}
                auditStatusOptions={[
                  { value: "", label: t.auditStatusOptions.all },
                  { value: "prompted", label: t.auditStatusOptions.prompted },
                  { value: "confirmed", label: t.auditStatusOptions.confirmed },
                  { value: "cancelled", label: t.auditStatusOptions.cancelled },
                  { value: "completed", label: t.auditStatusOptions.completed },
                  { value: "failed", label: t.auditStatusOptions.failed },
                ]}
                onAuditWindowChange={(value) => setSecurityEventWindowFilter(value as SecurityEventWindow)}
                onAuditSourceChange={setSecurityEventSourceFilter}
                onAuditRiskChange={setSecurityEventRiskFilter}
                onAuditStatusChange={setSecurityEventStatusFilter}
                onAuditActionChange={setSecurityEventActionFilter}
                onAuditRefresh={() => void refreshSecurityEvents()}
                onAuditExport={() => void downloadSecurityEventExport()}
                onAuditExportCsv={() => void downloadSecurityEventCsvExport()}
                auditEntries={runtimeAuditEntries}
                protocolAgentsTitle={t.protocolAgents}
                protocolAgentsCountLabel={String(protocolAgents.length)}
                protocolAgentsEmptyLabel={t.noProtocolAgents}
                protocolAgents={protocolAgentCards}
                watchdogTitle={t.watchdog}
                watchdogStatus={health?.watchdog ? t.online : t.offline}
                watchdogRows={runtimeWatchdogRows}
              />
            }
            dataContent={
              <WorkbenchDataAdminPanel
                title={t.dataAdmin}
                recordCountLabel={`${t.databaseRecordCount}: ${jobHistory.length + resultRecords.length}`}
                overviewTabLabel={t.overview}
                jobsTabLabel={t.adminJobs}
                resultsTabLabel={t.adminResults}
                browsePageLabel={t.adminBrowsePage}
                editPageLabel={t.adminEditPage}
                historyEmptyLabel={t.historyEmpty}
                selectRecordLabel={t.selectRecord}
                cancelJobLabel={t.cancelJob}
                saveRecordLabel={t.saveRecord}
                deleteRecordLabel={t.deleteRecord}
                exportRecordLabel={t.exportRecord}
                applyRecordContextLabel={t.applyRecordContext}
                openLinkedProjectLabel={t.openLinkedProject}
                openLinkedVersionLabel={t.openLinkedVersion}
                filterProjectLabel={t.filterProject}
                filterVersionLabel={t.filterVersion}
                useCurrentProjectLabel={t.useCurrentProject}
                useCurrentVersionLabel={t.useCurrentVersion}
                clearFiltersLabel={t.clearFilters}
                filterProjectValue={adminFilterProjectId}
                onFilterProjectChange={handleAdminFilterProjectChange}
                filterVersionValue={adminFilterModelVersionId}
                onFilterVersionChange={handleAdminFilterModelVersionChange}
                canUseCurrentProject={Boolean(selectedProjectId)}
                canUseCurrentVersion={Boolean(selectedVersionId)}
                onUseCurrentProject={useCurrentProjectAsAdminFilter}
                onUseCurrentVersion={useCurrentVersionAsAdminFilter}
                onClearFilters={clearAdminFilters}
                adminMessageLabel={t.adminMessage}
                adminProjectIdLabel={t.adminProjectId}
                adminModelVersionIdLabel={t.adminModelVersionId}
                adminCaseIdLabel={t.adminCaseId}
                resultPayloadLabel={t.resultPayload}
                activeTab={systemDataTab}
                onTabChange={handleSystemDataTabChange}
                jobRows={adminJobRows}
                selectedAdminJobId={selectedAdminJobId}
                onSelectAdminJob={handleSelectAdminJob}
                selectedAdminJob={Boolean(selectedAdminJob)}
                selectedAdminJobHasVersion={Boolean(selectedAdminJob?.model_version_id)}
                selectedAdminJobHasProject={Boolean(selectedAdminJob?.project_id)}
                selectedAdminJobHasContext={Boolean(selectedAdminJob?.project_id || selectedAdminJob?.model_version_id)}
                canCancelSelectedJob={Boolean(
                  selectedAdminJob &&
                    selectedAdminJob.status !== "completed" &&
                    selectedAdminJob.status !== "failed" &&
                    selectedAdminJob.status !== "cancelled",
                )}
                onApplySelectedJobContext={applySelectedAdminJobContext}
                onOpenSelectedJobProject={openSelectedAdminJobProject}
                onOpenSelectedJobVersion={openSelectedAdminJobVersion}
                onCancelSelectedJob={() => {
                  if (!selectedAdminJob) return;
                  if (selectedAdminJob.job_id === job?.job_id) {
                    cancelCurrentJob();
                    return;
                  }
                  void (async () => {
                    try {
                      await cancelJob(selectedAdminJob.job_id);
                      setMessage(t.jobCancelled);
                      await refreshJobHistory();
                    } catch (error) {
                      setMessage(error instanceof Error ? error.message : t.initialFailed);
                    }
                  })();
                }}
                adminJobMessage={adminJobMessage}
                onAdminJobMessageChange={setAdminJobMessage}
                adminJobProjectId={adminJobProjectId}
                onAdminJobProjectIdChange={setAdminJobProjectId}
                adminJobModelVersionId={adminJobModelVersionId}
                onAdminJobModelVersionIdChange={setAdminJobModelVersionId}
                adminJobCaseId={adminJobCaseId}
                onAdminJobCaseIdChange={setAdminJobCaseId}
                onSaveAdminJob={saveAdminJobRecord}
                onDeleteAdminJob={deleteAdminJobRecord}
                resultRows={adminResultRows}
                selectedAdminResultJobId={selectedAdminResultJobId}
                onSelectAdminResult={handleSelectAdminResult}
                selectedAdminResult={Boolean(selectedAdminResult)}
                selectedAdminResultHasProject={Boolean(
                  jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.project_id,
                )}
                selectedAdminResultHasVersion={Boolean(
                  jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.model_version_id,
                )}
                selectedAdminResultHasContext={Boolean(
                  jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.project_id ||
                    jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.model_version_id,
                )}
                adminResultDraft={adminResultDraft}
                onAdminResultDraftChange={setAdminResultDraft}
                onSaveAdminResult={saveAdminResultRecord}
                onApplySelectedResultContext={applySelectedAdminResultContext}
                onOpenSelectedResultProject={openSelectedAdminResultProject}
                onOpenSelectedResultVersion={openSelectedAdminResultVersion}
                onExportAdminResult={exportAdminResultRecord}
                onDeleteAdminResult={deleteAdminResultRecord}
              />
            }
          />
        ) : null}
      </aside>

      <button
        aria-expanded={assistantWindowOpen}
        aria-label={assistantWindowOpen ? t.assistantClose : t.assistantOpen}
        className={`assistant-float-launcher${assistantWindowOpen ? " assistant-float-launcher--open" : ""}`}
        onClick={() => setAssistantWindowOpen((current) => !current)}
        type="button"
      >
        <img alt="" className="assistant-float-launcher__mark" src="/kyuubiki.png" />
        <span className="assistant-float-launcher__copy">
          <strong>{t.assistant}</strong>
          <small>{assistantWindowOpen ? t.assistantClose : t.assistantLauncherHint}</small>
        </span>
      </button>

      {assistantWindowOpen ? (
        <aside aria-label={t.assistant} className="assistant-float-panel panel" role="dialog">
          <div className="assistant-float-panel__header">
            <div className="assistant-float-panel__headline">
              <img alt="" className="assistant-float-panel__mark" src="/kyuubiki.png" />
              <div>
                <strong>{t.assistantFloatingTitle}</strong>
                <p>{t.assistantFloatingSubtitle}</p>
              </div>
            </div>
            <button className="ghost-button ghost-button--compact" onClick={() => setAssistantWindowOpen(false)} type="button">
              {t.assistantClose}
            </button>
          </div>
          <div className="assistant-float-panel__body panel-scroll-window">
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
              promptPresets={assistantPromptPresets}
              transactions={assistantTransactions.map((entry) => ({
                id: entry.id,
                summary: entry.summary,
                createdAt: entry.createdAt,
                executedActions: entry.executedActions,
              }))}
              variant="floating"
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
            />
          </div>
        </aside>
      ) : null}

      <main className="workspace-main">
        <WorkbenchViewportPanel
          viewportPanelRef={viewportPanelRef}
          immersiveViewport={immersiveViewport}
          title={sidebarSection === "model" ? t.sections.model : t.viewport}
          headActions={
            <>
              {isTruss3d && immersiveViewport ? (
                <div className="immersive-switches">
                  <button
                    className={`ghost-button ghost-button--compact${sidebarSection === "model" && modelTab === "tools" && modelToolsPage === "study" ? " ghost-button--active" : ""}`}
                    onClick={() => {
                      handleSidebarSectionChange("model");
                      setModelTab("tools");
                      setModelToolsPage("study");
                    }}
                    type="button"
                  >
                    {t.immersiveStudy}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${sidebarSection === "model" && modelTab === "tools" && modelToolsPage !== "study" ? " ghost-button--active" : ""}`}
                    onClick={() => {
                      handleSidebarSectionChange("model");
                      setModelTab("tools");
                      if (modelToolsPage === "study") {
                        setModelToolsPage("studio");
                      }
                    }}
                    type="button"
                  >
                    {t.immersiveModel}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${sidebarSection === "library" ? " ghost-button--active" : ""}`}
                    onClick={() => handleSidebarSectionChange(sidebarSection === "library" ? "model" : "library")}
                    type="button"
                  >
                    {t.immersiveLibrary}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={handleToggleImmersiveToolDrawer}
                    type="button"
                  >
                    {t.immersiveTools}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${immersiveHelpDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={handleToggleImmersiveHelpDrawer}
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
                    onClick={handleToggleImmersiveToolDrawer}
                    type="button"
                  >
                    {t.immersiveTools}
                  </button>
                  <button
                    className={`ghost-button ghost-button--compact${immersiveHelpDrawerOpen ? " ghost-button--active" : ""}`}
                    onClick={handleToggleImmersiveHelpDrawer}
                    type="button"
                  >
                    {t.immersiveHelp}
                  </button>
                </div>
              ) : null}
              {isTruss3d ? (
                <button className={`ghost-button ghost-button--compact${immersiveViewport ? " ghost-button--active" : ""}`} onClick={() => void handleToggleImmersiveViewport()} type="button">
                  {immersiveViewport ? t.exitImmersive : t.enterImmersive}
                </button>
              ) : null}
              <span>{job?.status ?? "idle"}</span>
            </>
          }
          hasViewportDock={hasViewportDock}
          dockContent={
            <>
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
                          onClick={() => handleTruss3dViewPresetChange(preset)}
                          type="button"
                        >
                          {preset === "iso" ? "ISO" : preset === "front" ? "FR" : preset === "right" ? "RT" : "TP"}
                        </button>
                      ))}
                      <button
                        className={`ghost-button ghost-button--compact${selectedNode !== null || selectedTruss3dNodes.length > 0 ? " ghost-button--active" : ""}`}
                        onClick={handleTruss3dFocusViewport}
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
                        onClick={() => handleTruss3dProjectionModeChange(truss3dProjectionMode === "ortho" ? "persp" : "ortho")}
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
                        onClick={() => handleTruss3dBoxSelectModeChange(!truss3dBoxSelectMode)}
                        type="button"
                      >
                        BOX
                      </button>
                      <button
                        className="ghost-button ghost-button--compact"
                        onClick={handleTruss3dResetViewport}
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
                            <button className={`ghost-button ghost-button--compact${truss3dLinkMode ? " ghost-button--active" : ""}`} onClick={handleToggleTruss3dLinkMode} type="button">
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
            </>
          }
          resultWindowBar={
            activeResultWindow ? (
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
                {isPlane && planeResult ? (
                  <>
                    {isHeatPlane ? (
                      <>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "average_temperature" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("average_temperature")}
                          type="button"
                        >
                          {t.maxTemperature}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "temperature_gradient_x" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("temperature_gradient_x")}
                          type="button"
                        >
                          {t.temperatureGradientX}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "temperature_gradient_y" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("temperature_gradient_y")}
                          type="button"
                        >
                          {t.temperatureGradientY}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "heat_flux_x" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("heat_flux_x")}
                          type="button"
                        >
                          {t.heatFluxX}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "heat_flux_y" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("heat_flux_y")}
                          type="button"
                        >
                          {t.heatFluxY}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "heat_flux_magnitude" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("heat_flux_magnitude")}
                          type="button"
                        >
                          {t.maxHeatFlux}
                        </button>
                      </>
                    ) : (
                      <>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "von_mises" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("von_mises")}
                          type="button"
                        >
                          {t.planeViewVonMises}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "principal_stress_1" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("principal_stress_1")}
                          type="button"
                        >
                          {t.planeViewPrincipal1}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "max_in_plane_shear" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("max_in_plane_shear")}
                          type="button"
                        >
                          {t.planeViewMaxShear}
                        </button>
                      </>
                    )}
                    {isThermalPlaneTriangle || isThermalPlaneQuad ? (
                      <>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "average_temperature_delta" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("average_temperature_delta")}
                          type="button"
                        >
                          {t.temperatureDelta}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "thermal_strain" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("thermal_strain")}
                          type="button"
                        >
                          {t.thermalStrain}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${planeResultField === "mechanical_strain" ? " ghost-button--active" : ""}`}
                          onClick={() => setPlaneResultField("mechanical_strain")}
                          type="button"
                        >
                          {t.mechanicalStrain}
                        </button>
                      </>
                    ) : null}
                  </>
                ) : null}
                {isFrameLike && activeFrameLikeResult ? (
                  <>
                    <button
                      className={`ghost-button ghost-button--compact${frameResultField === "axial_stress" ? " ghost-button--active" : ""}`}
                      onClick={() => setFrameResultField("axial_stress")}
                      type="button"
                    >
                      {t.stress}
                    </button>
                    <button
                      className={`ghost-button ghost-button--compact${frameResultField === "max_bending_stress" ? " ghost-button--active" : ""}`}
                      onClick={() => setFrameResultField("max_bending_stress")}
                      type="button"
                    >
                      {t.bendingStress}
                    </button>
                    <button
                      className={`ghost-button ghost-button--compact${frameResultField === "max_combined_stress" ? " ghost-button--active" : ""}`}
                      onClick={() => setFrameResultField("max_combined_stress")}
                      type="button"
                    >
                      {t.combinedStress}
                    </button>
                    <button
                      className={`ghost-button ghost-button--compact${frameResultField === "moment" ? " ghost-button--active" : ""}`}
                      onClick={() => setFrameResultField("moment")}
                      type="button"
                    >
                      {t.maxMoment}
                    </button>
                    {isThermalFrame ? (
                      <>
                        <button
                          className={`ghost-button ghost-button--compact${frameResultField === "average_temperature_delta" ? " ghost-button--active" : ""}`}
                          onClick={() => setFrameResultField("average_temperature_delta")}
                          type="button"
                        >
                          {t.temperatureDelta}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${frameResultField === "temperature_gradient_y" ? " ghost-button--active" : ""}`}
                          onClick={() => setFrameResultField("temperature_gradient_y")}
                          type="button"
                        >
                          {t.temperatureGradientY}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${frameResultField === "thermal_curvature" ? " ghost-button--active" : ""}`}
                          onClick={() => setFrameResultField("thermal_curvature")}
                          type="button"
                        >
                          {t.thermalCurvature}
                        </button>
                      </>
                    ) : null}
                  </>
                ) : null}
                {isBeam && activeBeamLikeResult ? (
                  <>
                    <button
                      className={`ghost-button ghost-button--compact${beamResultField === "max_bending_stress" ? " ghost-button--active" : ""}`}
                      onClick={() => setBeamResultField("max_bending_stress")}
                      type="button"
                    >
                      {t.bendingStress}
                    </button>
                    <button
                      className={`ghost-button ghost-button--compact${beamResultField === "shear_force" ? " ghost-button--active" : ""}`}
                      onClick={() => setBeamResultField("shear_force")}
                      type="button"
                    >
                      {t.shearForce}
                    </button>
                    <button
                      className={`ghost-button ghost-button--compact${beamResultField === "moment" ? " ghost-button--active" : ""}`}
                      onClick={() => setBeamResultField("moment")}
                      type="button"
                    >
                      {t.maxMoment}
                    </button>
                    {studyKind === "thermal_beam_1d" ? (
                      <>
                        <button
                          className={`ghost-button ghost-button--compact${beamResultField === "temperature_gradient_y" ? " ghost-button--active" : ""}`}
                          onClick={() => setBeamResultField("temperature_gradient_y")}
                          type="button"
                        >
                          {t.temperatureGradientY}
                        </button>
                        <button
                          className={`ghost-button ghost-button--compact${beamResultField === "thermal_curvature" ? " ghost-button--active" : ""}`}
                          onClick={() => setBeamResultField("thermal_curvature")}
                          type="button"
                        >
                          {t.thermalCurvature}
                        </button>
                      </>
                    ) : null}
                  </>
                ) : null}
                {isTorsion && torsionResult ? (
                  <>
                    <button
                      className={`ghost-button ghost-button--compact${frameResultField === "max_bending_stress" ? " ghost-button--active" : ""}`}
                      onClick={() => setFrameResultField("max_bending_stress")}
                      type="button"
                    >
                      {t.torsionStress}
                    </button>
                    <button
                      className={`ghost-button ghost-button--compact${frameResultField === "moment" ? " ghost-button--active" : ""}`}
                      onClick={() => setFrameResultField("moment")}
                      type="button"
                    >
                      {t.maxTorque}
                    </button>
                  </>
                ) : null}
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
            ) : null
          }
          isTruss3d={isTruss3d}
          shouldStretchSpaceViewport={shouldStretchSpaceViewport}
          onCanvasStageScroll={handleCanvasStageScroll}
          canvasStageRef={canvasStageRef}
          viewportContent={
            <WorkbenchViewport
            studyKind={studyKind}
            sidebarSection={sidebarSection}
            title={t.sections.model}
            axialTitle={t.kinds.axial_bar_1d}
            trussTitle={t.kinds[isSpring2d ? "spring_2d" : isSpring1d ? "spring_1d" : isHeatBar ? "heat_bar_1d" : isThermalBar ? "thermal_bar_1d" : isThermalBeam ? "thermal_beam_1d" : isThermalFrame ? "thermal_frame_2d" : isThermalTruss2d ? "thermal_truss_2d" : studyKind === "beam_1d" ? "beam_1d" : isTorsion ? "torsion_1d" : isFrame ? "frame_2d" : "truss_2d"]}
            trussLegend={(isFrameLike && activeFrameLikeResult) || (isBeam && activeBeamLikeResult) || (isTorsion && torsionResult) || (isHeatBar && heatBarResult) || (isThermalBar && thermalBarResult) || (isThermalTruss2d && thermalTrussResult) || (isThermalTruss3d && thermalTruss3dResult) || (isSpring1d && springResult) || (isSpring2d && spring2dResult) || (isSpring3d && spring3dResult) ? frameLegendText : undefined}
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
              if (isBeam || isTorsion || isHeatBar || isThermalBar) {
                setSelectedNode(index);
                setSelectedElement(null);
                setMemberDraftNodes([]);
                return;
              }
              if (isFrameLike) {
                dragHistoryCapturedRef.current = false;
                setDraggingNode(index);
                toggleDraftNode(index);
                return;
              }
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
          />
          }
          immersiveDrawer={
            immersiveViewport && sidebarSection === "library" ? (
              <div className="immersive-drawer">
              <section className="immersive-drawer__card">
                <div className="card-head">
                  <h2>{t.immersiveDrawer}</h2>
                  <div className="immersive-drawer__head-actions">
                    <span>{t.immersiveLibrary}</span>
                    <button
                      className="ghost-button ghost-button--compact"
                      onClick={() => handleSidebarSectionChange("model")}
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
                      {librarySampleRows.slice(0, 4).map((sample) => (
                        <button
                          key={sample.href}
                          className="history-item immersive-drawer__button"
                          onClick={() => openSample(sample.href)}
                          type="button"
                        >
                          <strong>{sample.name}</strong>
                          <span>{sample.kindLabel}</span>
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="immersive-drawer__section">
                    <h3>{t.immersiveModels}</h3>
                    <div className="immersive-drawer__list">
                      {libraryModelRows.slice(0, 4).length > 0 ? (
                        libraryModelRows.slice(0, 4).map((model) => (
                          <button
                            key={model.id}
                            className="history-item immersive-drawer__button"
                            onClick={() => {
                              const modelRecord = selectedProjectModels.find((entry) => entry.model_id === model.id);
                              if (modelRecord) openSavedModel(modelRecord);
                            }}
                            type="button"
                          >
                            <strong>{model.name}</strong>
                            <small>{t.updatedAt}: {model.updatedAt}</small>
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
                      {libraryJobRows.slice(0, 4).length > 0 ? (
                        libraryJobRows.slice(0, 4).map((historyJob) => (
                          <button
                            key={historyJob.id}
                            className="history-item immersive-drawer__button"
                            onClick={() => openHistoryJob(historyJob.id)}
                            type="button"
                          >
                            <strong>{historyJob.shortId}</strong>
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
            ) : null
          }
        />

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
          selectedNodeId={isPlane ? selectedPlaneNodeData?.id ?? null : isBeam ? selectedBeamNodeData?.id ?? null : isTorsion ? selectedTorsionNodeData?.id ?? null : isFrameLike ? selectedFrameNodeData?.id ?? null : isHeatBar || isThermalBar ? selectedThermalNodeData?.id ?? null : isSpring3d || isThermalTruss3d ? selectedTruss3dNodeData?.id ?? null : selectedNodeData?.id ?? null}
          selectedNodeX={isPlane ? selectedPlaneNodeData?.x : isBeam ? selectedBeamNodeData?.x : isTorsion ? selectedTorsionNodeData?.x : isFrameLike ? selectedFrameNodeData?.x : isHeatBar || isThermalBar ? selectedThermalNodeData?.x : isSpring3d || isThermalTruss3d ? selectedTruss3dNodeData?.x : selectedNodeData?.x}
          selectedNodeY={isPlane ? selectedPlaneNodeData?.y : isBeam || isTorsion || isHeatBar || isThermalBar || isSpring1d ? 0 : isFrameLike ? selectedFrameNodeData?.y : isSpring3d || isThermalTruss3d ? selectedTruss3dNodeData?.y : selectedNodeData?.y}
          selectedNodeLoadY={
            isPlane
              ? selectedPlaneNodeData?.load_y
              : isBeam
                ? selectedBeamNodeData?.load_y
                : isTorsion
                  ? selectedTorsionNodeData?.moment_z
                : isFrameLike
                  ? selectedFrameNodeData?.load_y
                  : isHeatBar
                    ? selectedThermalNodeData?.load_x
                  : isThermalBar
                    ? selectedThermalNodeData?.load_x
                  : isSpring1d
                    ? selectedNodeData?.load_x
                    : isSpring2d
                      ? selectedNodeData?.load_y
                      : isSpring3d
                        ? (selectedTruss3dNodeData ? spring3dModel.nodes[selectedTruss3dNodeData.index]?.load_z : undefined)
                        : selectedNodeData?.load_y
          }
          selectedNodeIssueCount={isPlane ? null : selectedNodeIssues.length > 0 ? selectedNodeIssues.length : null}
          elementTitle={isAxial ? t.axialElements : isBeam || isTorsion || isSpring || isHeatBar || isThermalBar ? t.frameElements : isTruss ? t.trussElements : isTruss3d ? t.spatialTrussElements : isFrameLike ? t.frameElements : t.planeElements}
          spanLabel={t.span}
          stressLabel={isHeatPlane ? `${t.maxTemperature} / ${t.temperatureGradientY}` : isPlane ? `${t.maxStress} / ${t.principalStress1}` : isFrameLike ? t.combinedStress : isBeam ? t.bendingStress : isTorsion ? t.torsionStress : isHeatBar ? t.temperatureGradientY : isSpring || isThermalBar ? t.axialForce : t.stress}
          axialForceLabel={isHeatPlane ? t.maxHeatFlux : isPlane ? t.maxInPlaneShear : isFrameLike || isBeam ? t.maxMoment : isTorsion ? t.maxTorque : isHeatBar ? t.maxHeatFlux : isSpring || isThermalBar ? t.axialForce : t.axialForce}
          isFrame={isFrameLike || isBeam || isTorsion}
          elements={(isAxial ? axialElements : isSpring3d || isThermalTruss3d ? displayTruss3dElements : isTruss || isSpring || isHeatBar || isThermal ? displayTrussElements : isTruss3d ? displayTruss3dElements : isFrameLike || isBeam ? displayTrussElements : planeElements) as Array<{
            index: number;
            x1?: number;
            x2?: number;
            node_i?: number;
            node_j?: number;
            node_k?: number;
            node_l?: number;
            stress?: number;
            axial_force?: number;
            von_mises?: number;
            principal_stress_1?: number;
            max_in_plane_shear?: number;
          }>}
        />
      </main>

      <WorkbenchInspector
        t={t}
        reportScopeLabel={currentStudyFamilyLabel}
        reportScopeHint={currentStudyFamilyHint}
        sidebarSection={sidebarSection}
        studyKind={studyKind}
        isPending={isPending}
        selectedNodeData={selectedNodeData ? { ...selectedNodeData } : null}
        selectedElementData={selectedElementData ? { ...selectedElementData } : null}
        selectedTruss3dNodeData={selectedTruss3dNodeData ? { ...selectedTruss3dNodeData, ...(isSpring3d ? spring3dModel.nodes[selectedTruss3dNodeData.index] : isThermalTruss3d ? thermalTruss3dModel.nodes[selectedTruss3dNodeData.index] : truss3dModel.nodes[selectedTruss3dNodeData.index]) } : null}
        selectedTruss3dElementData={selectedTruss3dElementData ? { ...selectedTruss3dElementData } : null}
        selectedPlaneNodeData={selectedPlaneNodeData ? { ...selectedPlaneNodeData } : null}
        selectedPlaneElementData={selectedPlaneElementData ? { ...selectedPlaneElementData } : null}
        selectedFrameNodeData={(isFrameLike ? selectedFrameNodeData : isBeam ? selectedBeamNodeData : isTorsion ? selectedTorsionNodeData : isHeatBar || isThermalBar ? selectedThermalNodeData : null) ? { ...(isFrameLike ? selectedFrameNodeData : isBeam ? selectedBeamNodeData : isTorsion ? selectedTorsionNodeData : selectedThermalNodeData)! } : null}
        selectedFrameElementData={(isFrameLike ? selectedFrameElementData : isBeam ? selectedBeamElementData : isTorsion ? selectedTorsionElementData : isHeatBar || isThermalBar ? selectedThermalElementData : isSpring ? selectedSpringElementData : null) ? { ...(isFrameLike ? selectedFrameElementData : isBeam ? selectedBeamElementData : isTorsion ? selectedTorsionElementData : isHeatBar || isThermalBar ? selectedThermalElementData : selectedSpringElementData)! } : null}
        trussElementArea={selectedElementData ? (isThermalTruss2d ? thermalTrussModel.elements[selectedElementData.index]?.area : trussModel.elements[selectedElementData.index]?.area) ?? 0 : 0}
        trussElementModulusGpa={selectedElementData ? round((((isThermalTruss2d ? thermalTrussModel.elements[selectedElementData.index]?.youngs_modulus : trussModel.elements[selectedElementData.index]?.youngs_modulus) ?? 0) / 1.0e9)) : 0}
        trussElementMaterialId={selectedElementData ? (isThermalTruss2d ? thermalTrussModel.elements[selectedElementData.index]?.material_id : trussModel.elements[selectedElementData.index]?.material_id) ?? materialOptions[0]?.id ?? "" : ""}
        truss3dElementArea={selectedTruss3dElementData ? (studyKind === "thermal_truss_3d" ? thermalTruss3dModel.elements[selectedTruss3dElementData.index]?.area : studyKind === "truss_3d" ? truss3dModel.elements[selectedTruss3dElementData.index]?.area : undefined) ?? 0 : 0}
        truss3dElementModulusGpa={selectedTruss3dElementData ? round(((((studyKind === "thermal_truss_3d" ? thermalTruss3dModel.elements[selectedTruss3dElementData.index]?.youngs_modulus : studyKind === "truss_3d" ? truss3dModel.elements[selectedTruss3dElementData.index]?.youngs_modulus : undefined) ?? 0)) / 1.0e9)) : 0}
        truss3dElementMaterialId={selectedTruss3dElementData ? ((studyKind === "thermal_truss_3d" ? thermalTruss3dModel.elements[selectedTruss3dElementData.index]?.material_id : studyKind === "truss_3d" ? truss3dModel.elements[selectedTruss3dElementData.index]?.material_id : undefined) ?? materialOptions[0]?.id ?? "") : ""}
        planeElementThickness={selectedPlaneElementData ? activePlaneInputModel.elements[selectedPlaneElementData.index]?.thickness ?? 0 : 0}
        planeElementModulusGpa={selectedPlaneElementData && !isHeatPlane ? round((((planeModel.elements[selectedPlaneElementData.index] as PlaneTriangle2dJobInput["elements"][number] | PlaneQuad2dJobInput["elements"][number] | ThermalPlaneTriangle2dJobInput["elements"][number] | ThermalPlaneQuad2dJobInput["elements"][number])?.youngs_modulus ?? 0) / 1.0e9)) : 0}
        planeElementPoissonRatio={selectedPlaneElementData && !isHeatPlane ? (((planeModel.elements[selectedPlaneElementData.index] as PlaneTriangle2dJobInput["elements"][number] | PlaneQuad2dJobInput["elements"][number] | ThermalPlaneTriangle2dJobInput["elements"][number] | ThermalPlaneQuad2dJobInput["elements"][number])?.poisson_ratio) ?? 0.33) : 0.33}
        planeElementMaterialId={selectedPlaneElementData && !isHeatPlane ? ((planeModel.elements[selectedPlaneElementData.index] as PlaneTriangle2dJobInput["elements"][number] | PlaneQuad2dJobInput["elements"][number] | ThermalPlaneTriangle2dJobInput["elements"][number] | ThermalPlaneQuad2dJobInput["elements"][number])?.material_id ?? materialOptions[0]?.id ?? "") : ""}
        frameElementMaterialId={isFrameLike ? selectedFrameElementData ? activeFrameLikeModel.elements[selectedFrameElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : "" : isBeam ? selectedBeamElementData ? activeBeamLikeModel.elements[selectedBeamElementData.index]?.material_id ?? materialOptions[0]?.id ?? "" : "" : ""}
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
        onUpdateSelectedFrameNode={updateSelectedFrameNode}
        onUpdateSelectedFrameElement={updateSelectedFrameElement}
        onAssignSelectedFrameElementMaterial={assignSelectedFrameElementMaterial}
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
        tipDisplacement={isAxial ? scientific(axialResult?.tip_displacement) : isHeatBar ? scientific(heatBarResult?.max_temperature) : isHeatPlane ? scientific(isHeatPlaneTriangle ? heatPlaneTriangleResult?.max_temperature : heatPlaneQuadResult?.max_temperature) : isThermalTruss2d ? scientific(thermalTrussResult?.max_displacement) : studyKind === "thermal_truss_3d" ? scientific(thermalTruss3dResult?.max_displacement) : isTruss ? scientific(trussResult?.max_displacement) : isSpring3d ? scientific(spring3dResult?.max_displacement) : studyKind === "truss_3d" ? scientific(truss3dResult?.max_displacement) : isThermalBar ? scientific(thermalBarResult?.max_displacement) : isSpring ? scientific(activeSpringResult?.max_displacement) : isBeam ? scientific(activeBeamLikeResult?.max_displacement) : isTorsion ? scientific(torsionResult?.max_rotation) : isFrameLike ? scientific(activeFrameLikeResult?.max_displacement) : scientific(planeResult?.max_displacement)}
        maxStressValue={scientific(isAxial ? axialResult?.max_stress : isHeatBar ? heatBarResult?.max_heat_flux : isHeatPlane ? (isHeatPlaneTriangle ? heatPlaneTriangleResult?.max_heat_flux : heatPlaneQuadResult?.max_heat_flux) : isThermalTruss2d ? thermalTrussResult?.max_stress : studyKind === "thermal_truss_3d" ? thermalTruss3dResult?.max_stress : isTruss ? trussResult?.max_stress : isSpring3d ? spring3dResult?.max_force : studyKind === "truss_3d" ? truss3dResult?.max_stress : isThermalBar ? thermalBarResult?.max_stress : isSpring ? activeSpringResult?.max_force : isBeam ? activeBeamLikeResult?.max_stress : isTorsion ? torsionResult?.max_stress : isFrameLike ? activeFrameLikeResult?.max_stress : planeResult?.max_stress)}
        frameMaxAxialForceValue={isFrameLike || isSpring || isHeatBar || isThermalBar || isThermalTruss2d || isThermalTruss3d ? scientific(frameMaxAxialForce) : undefined}
        frameMaxShearForceValue={isFrameLike || isBeam ? scientific(frameMaxShearForce) : undefined}
        reactionValue={isAxial ? scientific(axialResult?.reaction_force) : isHeatBar ? scientific(heatBarResult?.max_heat_flux) : isThermalBar ? scientific(thermalBarResult?.max_axial_force) : isThermalTruss2d ? scientific(thermalTrussResult?.max_axial_force) : isThermalTruss3d ? scientific(thermalTruss3dResult?.max_axial_force) : isSpring ? scientific(activeSpringResult?.max_force) : isTorsion ? scientific(torsionResult?.max_torque) : isFrameLike ? scientific(activeFrameLikeResult?.max_moment) : isBeam ? scientific(activeBeamLikeResult?.max_moment) : "--"}
        frameMaxRotationValue={isFrameLike ? scientific(activeFrameLikeResult?.max_rotation) : isBeam ? scientific(activeBeamLikeResult?.max_rotation) : isTorsion ? scientific(torsionResult?.max_rotation) : undefined}
        thermalPlaneMaxTemperatureDeltaValue={isThermalPlaneTriangle || isThermalPlaneQuad ? scientific(planeResult && "max_temperature_delta" in planeResult ? planeResult.max_temperature_delta : undefined) : undefined}
        thermalFrameMaxTemperatureDeltaValue={isThermalFrame ? scientific(thermalFrameMaxTemperatureDelta) : undefined}
        thermalFrameMaxTemperatureGradientValue={isThermalFrame ? scientific(thermalFrameMaxTemperatureGradient) : undefined}
        thermalBeamMaxTemperatureGradientValue={isThermalBeam ? scientific(thermalBeamMaxTemperatureGradient) : undefined}
        planeHotspotFieldLabel={isPlane ? planeResultFieldLabel : undefined}
        planeHotspotElements={isPlane ? planeHotspotElements : []}
        planeThermalRows={isHeatPlane ? planeThermalRows : []}
        frameHotspotFieldLabel={isFrameLike || isBeam || isSpring || isThermal || isTorsion ? frameResultFieldLabel : undefined}
        frameHotspotElements={isFrameLike || isBeam || isTorsion || isSpring || isThermal ? frameHotspotElements : []}
        frameForceRows={isFrameLike || isBeam || isTorsion || isSpring || isThermal ? frameForceRows : []}
        planeHotspotLimit={planeHotspotLimit}
        createdAtValue={formatTime(job?.created_at, language)}
        updatedAtValue={formatTime(job?.updated_at, language)}
        heartbeatStatusValue={heartbeatStatusValue}
        heartbeatTone={heartbeatToneValue}
        failureReasonValue={translatedFailureReason ?? job?.message ?? "--"}
        canCancelJob={jobIsActive}
        onCancelJob={cancelCurrentJob}
        onDownloadJson={downloadResultJson}
        onDownloadCsv={downloadResultCsv}
        canProjectHeatToThermo={canProjectHeatToThermo}
        projectHeatToThermoLabel={t.projectHeatToThermo}
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
    </div>
  );
}
