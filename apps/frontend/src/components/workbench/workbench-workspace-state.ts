"use client";

import { useState } from "react";
import {
  defaultAxial,
  defaultBeam1d,
  defaultElectrostaticPlaneQuad,
  defaultElectrostaticPlaneTriangle,
  defaultFrame2d,
  defaultHeatBar1d,
  defaultHeatPlaneTriangle,
  defaultPanelParametric,
  defaultParametric,
  defaultPlaneTriangle,
  defaultSpring1d,
  defaultSpring2d,
  defaultSpring3d,
  defaultThermalBar1d,
  defaultThermalBeam1d,
  defaultThermalFrame2d,
  defaultThermalTruss2d,
  defaultThermalTruss3d,
  defaultTorsion1d,
  defaultTruss,
  defaultTruss3d,
  type AxialFormState,
  type DirectMeshExecutionState,
  type PlaneResultField,
} from "@/components/workbench/workbench-defaults";
import type { ViewportRenderStrategy } from "@/components/workbench/workbench-render-diagnostics";
import {
  type BeamResultField,
  type FrameResultField,
  type LibraryPanelTab,
  type ModelPanelTab,
  type SidebarSection,
  type StudyKind,
  type StudyPanelTab,
  type SystemDataTab,
  type SystemPanelTab,
} from "@/components/workbench/workbench-types";
import type { ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";
import type { ResultWindowState } from "@/components/workbench/workbench-result-window-controller";
import { RESULT_WINDOW_BASE_SIZE } from "@/lib/workbench/result-window";
import {
  type HistoryEntry,
} from "@/lib/workbench/history";
import {
  type AxialBarResult,
  type Beam1dResult,
  type ElectrostaticPlaneQuad2dResult,
  type ElectrostaticPlaneTriangle2dResult,
  type Frame2dResult,
  type HeatBar1dResult,
  type HeatPlaneQuad2dResult,
  type HeatPlaneTriangle2dResult,
  type JobEnvelope,
  type ModelVersionRecord,
  type PlaneQuad2dResult,
  type PlaneTriangle2dResult,
  type ProjectRecord,
  type ResultRecord,
  type Spring1dResult,
  type Spring2dResult,
  type Spring3dResult,
  type ThermalBar1dResult,
  type ThermalBeam1dResult,
  type ThermalFrame2dResult,
  type ThermalPlaneQuad2dResult,
  type ThermalPlaneTriangle2dResult,
  type ThermalTruss2dResult,
  type ThermalTruss3dResult,
  type Torsion1dResult,
  type Truss2dJobInput,
  type Truss2dResult,
  type Truss3dJobInput,
  type Truss3dResult,
} from "@/lib/api";
import type {
  BeamStudyJobInput,
  FrameStudyJobInput,
  HeatBarStudyJobInput,
  HeatPlaneStudyJobInput,
  PlaneStudyJobInput,
  Spring2dStudyJobInput,
  Spring3dStudyJobInput,
  SpringStudyJobInput,
  ThermalBarStudyJobInput,
  ThermalBeamStudyJobInput,
  ThermalFrameStudyJobInput,
  ThermalTruss2dStudyJobInput,
  ThermalTruss3dStudyJobInput,
} from "@/components/workbench/workbench-types";
import type { ParametricPanelConfig, ParametricTrussConfig } from "@/lib/models";

type WorkbenchResult =
  | AxialBarResult
  | Beam1dResult
  | Frame2dResult
  | HeatBar1dResult
  | ElectrostaticPlaneQuad2dResult
  | ElectrostaticPlaneTriangle2dResult
  | HeatPlaneQuad2dResult
  | HeatPlaneTriangle2dResult
  | PlaneQuad2dResult
  | PlaneTriangle2dResult
  | Spring1dResult
  | Spring2dResult
  | Spring3dResult
  | ThermalBar1dResult
  | ThermalBeam1dResult
  | ThermalFrame2dResult
  | ThermalPlaneQuad2dResult
  | ThermalPlaneTriangle2dResult
  | ThermalTruss2dResult
  | ThermalTruss3dResult
  | Torsion1dResult
  | Truss2dResult
  | Truss3dResult
  | null;

export function useWorkbenchWorkspaceState(params: {
  defaultLoadedModelName: string;
  defaultMessage: string;
  defaultProjectLabel: string;
}) {
  const { defaultLoadedModelName, defaultMessage, defaultProjectLabel } = params;
  const [studyKind, setStudyKind] = useState<StudyKind>("axial_bar_1d");
  const [axialForm, setAxialForm] = useState<AxialFormState>(defaultAxial);
  const [heatBarModel, setHeatBarModel] = useState<HeatBarStudyJobInput>(defaultHeatBar1d);
  const [heatPlaneModel, setHeatPlaneModel] = useState<HeatPlaneStudyJobInput>(defaultHeatPlaneTriangle);
  const [thermalBarModel, setThermalBarModel] = useState<ThermalBarStudyJobInput>(defaultThermalBar1d);
  const [thermalBeamModel, setThermalBeamModel] = useState<ThermalBeamStudyJobInput>(defaultThermalBeam1d);
  const [thermalFrameModel, setThermalFrameModel] = useState<ThermalFrameStudyJobInput>(defaultThermalFrame2d);
  const [thermalTrussModel, setThermalTrussModel] = useState<ThermalTruss2dStudyJobInput>(defaultThermalTruss2d);
  const [trussModel, setTrussModel] = useState<Truss2dJobInput>(defaultTruss);
  const [thermalTruss3dModel, setThermalTruss3dModel] =
    useState<ThermalTruss3dStudyJobInput>(defaultThermalTruss3d);
  const [truss3dModel, setTruss3dModel] = useState<Truss3dJobInput>(defaultTruss3d);
  const [planeModel, setPlaneModel] = useState<PlaneStudyJobInput>(defaultPlaneTriangle);
  const [frameModel, setFrameModel] = useState<FrameStudyJobInput>(defaultFrame2d);
  const [beamModel, setBeamModel] = useState<BeamStudyJobInput>(defaultBeam1d);
  const [torsionModel, setTorsionModel] = useState(defaultTorsion1d);
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
  const [renderStrategy, setRenderStrategy] = useState<ViewportRenderStrategy>("auto");
  const [focusedPlaneElement, setFocusedPlaneElement] = useState<number | null>(null);
  const [focusedFrameElement, setFocusedFrameElement] = useState<number | null>(null);
  const [result, setResult] = useState<WorkbenchResult>(null);
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
  const [projectNameDraft, setProjectNameDraft] = useState(defaultProjectLabel);
  const [projectDescriptionDraft, setProjectDescriptionDraft] = useState("");
  const [loadedModelName, setLoadedModelName] = useState(defaultLoadedModelName);
  const [message, setMessage] = useState(defaultMessage);
  const [hiddenMaterials, setHiddenMaterials] = useState<Record<StudyKind, string[]>>({
    axial_bar_1d: [],
    heat_bar_1d: [],
    electrostatic_plane_triangle_2d: [],
    electrostatic_plane_quad_2d: [],
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

  return {
    studyKind,
    setStudyKind,
    axialForm,
    setAxialForm,
    heatBarModel,
    setHeatBarModel,
    heatPlaneModel,
    setHeatPlaneModel,
    thermalBarModel,
    setThermalBarModel,
    thermalBeamModel,
    setThermalBeamModel,
    thermalFrameModel,
    setThermalFrameModel,
    thermalTrussModel,
    setThermalTrussModel,
    trussModel,
    setTrussModel,
    thermalTruss3dModel,
    setThermalTruss3dModel,
    truss3dModel,
    setTruss3dModel,
    planeModel,
    setPlaneModel,
    frameModel,
    setFrameModel,
    beamModel,
    setBeamModel,
    torsionModel,
    setTorsionModel,
    springModel,
    setSpringModel,
    spring2dModel,
    setSpring2dModel,
    spring3dModel,
    setSpring3dModel,
    parametric,
    setParametric,
    panelParametric,
    setPanelParametric,
    activeMaterial,
    setActiveMaterial,
    planeResultField,
    setPlaneResultField,
    frameResultField,
    setFrameResultField,
    beamResultField,
    setBeamResultField,
    planeHotspotLimit,
    setPlaneHotspotLimit,
    renderStrategy,
    setRenderStrategy,
    focusedPlaneElement,
    setFocusedPlaneElement,
    focusedFrameElement,
    setFocusedFrameElement,
    result,
    setResult,
    resultWindow,
    setResultWindow,
    resultWindowOffset,
    setResultWindowOffset,
    resultWindowLimit,
    setResultWindowLimit,
    canvasViewportWidth,
    setCanvasViewportWidth,
    job,
    setJob,
    resultRecords,
    setResultRecords,
    projects,
    setProjects,
    selectedProjectId,
    setSelectedProjectId,
    selectedModelId,
    setSelectedModelId,
    selectedVersionId,
    setSelectedVersionId,
    modelVersions,
    setModelVersions,
    projectNameDraft,
    setProjectNameDraft,
    projectDescriptionDraft,
    setProjectDescriptionDraft,
    loadedModelName,
    setLoadedModelName,
    message,
    setMessage,
    hiddenMaterials,
    setHiddenMaterials,
    sidebarSection,
    setSidebarSection,
    studyTab,
    setStudyTab,
    modelTab,
    setModelTab,
    modelToolsPage,
    setModelToolsPage,
    libraryTab,
    setLibraryTab,
    systemDataTab,
    setSystemDataTab,
    systemPanelTab,
    setSystemPanelTab,
    draggingNode,
    setDraggingNode,
    truss3dLinkMode,
    setTruss3dLinkMode,
    selectedTruss3dNodes,
    setSelectedTruss3dNodes,
    selectedNode,
    setSelectedNode,
    selectedElement,
    setSelectedElement,
    memberDraftNodes,
    setMemberDraftNodes,
    undoStack,
    setUndoStack,
    redoStack,
    setRedoStack,
  };
}
