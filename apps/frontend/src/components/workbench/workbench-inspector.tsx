"use client";

import { memo, useState } from "react";
import { VirtualList } from "@/components/ui/virtual-list";

type SidebarSection = "study" | "model" | "library" | "system";
type StudyKind = "axial_bar_1d" | "thermal_bar_1d" | "thermal_beam_1d" | "thermal_frame_2d" | "thermal_truss_2d" | "thermal_truss_3d" | "spring_1d" | "spring_2d" | "spring_3d" | "beam_1d" | "torsion_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d" | "plane_quad_2d" | "frame_2d";

type TrussSuggestion = {
  id: string;
  label: string;
};

type TrussDiagnostics = {
  blockingMessages: string[];
  suggestions: TrussSuggestion[];
};

type StabilitySummary = {
  score: number;
  tone: "good" | "watch" | "risk";
  hotspotNodes: number[];
};

type HistoryEntry = {
  label: string;
};

type TrussNodeSelection = {
  id: string;
  x: number;
  y: number;
  load_x: number;
  load_y: number;
  fix_x: boolean;
  fix_y: boolean;
};

type TrussElementSelection = {
  id: string;
  index: number;
  node_i: number;
  node_j: number;
};

type Truss3dNodeSelection = {
  id: string;
  x: number;
  y: number;
  z: number;
  load_x: number;
  load_y: number;
  load_z: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_z: boolean;
};

type Truss3dElementSelection = {
  id: string;
  index: number;
  node_i: number;
  node_j: number;
};

type PlaneNodeSelection = {
  id: string;
  x: number;
  y: number;
  load_x: number;
  load_y: number;
  displacement_magnitude?: number;
  fix_x: boolean;
  fix_y: boolean;
};

type PlaneElementSelection = {
  id: string;
  index: number;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l?: number;
  principal_stress_1?: number;
  principal_stress_2?: number;
  max_in_plane_shear?: number;
};

type FrameNodeSelection = {
  id: string;
  x: number;
  y: number;
  load_x: number;
  load_y: number;
  moment_z: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_rz: boolean;
  displacement_magnitude?: number;
  rz?: number;
};

type FrameElementSelection = {
  id: string;
  index: number;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  moment_of_inertia: number;
  section_modulus: number;
  distributed_load_y?: number;
  axial_stress?: number;
  max_bending_stress?: number;
  max_combined_stress?: number;
  axial_force_i?: number;
  shear_force_i?: number;
  moment_i?: number;
  axial_force_j?: number;
  shear_force_j?: number;
  moment_j?: number;
};

type JobLike = {
  status?: string | null;
  worker_id?: string | null;
  progress?: number | null;
  iteration?: number | null;
  residual?: number | null;
  has_result?: boolean | null;
  created_at?: string | undefined;
  updated_at?: string | undefined;
  message?: string | null;
};

type InspectorLabels = {
  overview: string;
  busy: string;
  ready: string;
  properties: string;
  dragNode: string;
  nodeX: string;
  nodeY: string;
  nodeZ: string;
  loadX: string;
  loadY: string;
  loadZ: string;
  fixX: string;
  fixY: string;
  fixZ: string;
  memberSelection: string;
  nodeI: string;
  nodeJ: string;
  nodeK: string;
  area: string;
  modulus: string;
  planeThickness: string;
  poissonRatio: string;
  fixRz: string;
  momentZ: string;
  torqueZ: string;
  rotationZ: string;
  frameElements: string;
  memberEndForces: string;
  momentOfInertia: string;
  sectionModulus: string;
  distributedLoadY: string;
  bendingStress: string;
  combinedStress: string;
  maxMoment: string;
  maxTorque: string;
  torsionStress: string;
  maxRotation: string;
  sortBy: string;
  shearForce: string;
  forceI: string;
  shearI: string;
  momentI: string;
  forceJ: string;
  shearJ: string;
  momentJ: string;
  selectionHint: string;
  diagnostics: string;
  stabilityScore: string;
  stabilityGood: string;
  stabilityWatch: string;
  stabilityRisk: string;
  hotspotNodes: string;
  suggestedFixes: string;
  diagnosticsClear: string;
  historyPanel: string;
  undo: string;
  redo: string;
  noOperations: string;
  metrics: string;
  status: string;
  worker: string;
  progress: string;
  iteration: string;
  residual: string;
  nodes: string;
  report: string;
  exportData: string;
  exportJson: string;
  exportCsv: string;
  tipDisp: string;
  maxStress: string;
  axialForce: string;
  maxAxialForce: string;
  maxShearForce: string;
  reaction: string;
  displacementMagnitude: string;
  principalStress1: string;
  principalStress2: string;
  maxInPlaneShear: string;
  currentField: string;
  planeHotspots: string;
  topN: string;
  exportHotspots: string;
  memberForceTable: string;
  exportMemberForces: string;
  createdAt: string;
  updatedAt: string;
  lastHeartbeat: string;
  heartbeatStatus: string;
  hasResult: string;
  failureReason: string;
  cancelJob: string;
  yes: string;
  no: string;
};

type WorkbenchInspectorProps = {
  t: InspectorLabels;
  reportScopeLabel?: string;
  reportScopeHint?: string;
  sidebarSection: SidebarSection;
  studyKind: StudyKind;
  isPending: boolean;
  selectedNodeData: TrussNodeSelection | null;
  selectedElementData: TrussElementSelection | null;
  selectedTruss3dNodeData: Truss3dNodeSelection | null;
  selectedTruss3dElementData: Truss3dElementSelection | null;
  selectedPlaneNodeData: PlaneNodeSelection | null;
  selectedPlaneElementData: PlaneElementSelection | null;
  selectedFrameNodeData: FrameNodeSelection | null;
  selectedFrameElementData: FrameElementSelection | null;
  trussElementArea: number;
  trussElementModulusGpa: number;
  trussElementMaterialId: string;
  truss3dElementArea: number;
  truss3dElementModulusGpa: number;
  truss3dElementMaterialId: string;
  planeElementThickness: number;
  planeElementModulusGpa: number;
  planeElementPoissonRatio: number;
  planeElementMaterialId: string;
  frameElementMaterialId: string;
  materialOptions: Array<{ id: string; label: string }>;
  materialLabel: string;
  onUpdateSelectedNode: (field: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y", value: number | boolean) => void;
  onUpdateSelectedElement: (field: "area" | "youngs_modulus", value: number) => void;
  onAssignSelectedElementMaterial: (materialId: string) => void;
  onUpdateSelectedTruss3dNode: (field: "x" | "y" | "z" | "load_x" | "load_y" | "load_z" | "fix_x" | "fix_y" | "fix_z", value: number | boolean) => void;
  onUpdateSelectedTruss3dElement: (field: "area" | "youngs_modulus", value: number) => void;
  onAssignSelectedTruss3dElementMaterial: (materialId: string) => void;
  onUpdateSelectedPlaneNode: (field: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y", value: number | boolean) => void;
  onUpdateSelectedPlaneElement: (field: "thickness" | "youngs_modulus" | "poisson_ratio", value: number) => void;
  onAssignSelectedPlaneElementMaterial: (materialId: string) => void;
  onUpdateSelectedFrameNode: (field: "x" | "y" | "load_x" | "load_y" | "moment_z" | "fix_x" | "fix_y" | "fix_rz", value: number | boolean) => void;
  onUpdateSelectedFrameElement: (
    field: "area" | "youngs_modulus" | "moment_of_inertia" | "section_modulus" | "distributed_load_y",
    value: number,
  ) => void;
  onAssignSelectedFrameElementMaterial: (materialId: string) => void;
  trussDiagnostics: TrussDiagnostics | null;
  trussStability: StabilitySummary | null;
  hotspotNodeLabels: string;
  onApplyTrussSuggestion: (id: string) => void;
  undoStack: HistoryEntry[];
  redoStack: HistoryEntry[];
  onUndo: () => void;
  onRedo: () => void;
  job: JobLike | null;
  nodeCount: number;
  tipDisplacement: string;
  maxStressValue: string;
  frameMaxAxialForceValue?: string;
  frameMaxShearForceValue?: string;
  reactionValue: string;
  frameMaxRotationValue?: string;
  planeHotspotFieldLabel?: string;
  planeHotspotElements: Array<{ id: string; value: string; index: number; active?: boolean }>;
  frameHotspotFieldLabel?: string;
  frameHotspotElements: Array<{ id: string; value: string; index: number; active?: boolean; summary?: string }>;
  frameForceRows: Array<{
    id: string;
    index: number;
    active?: boolean;
    sortAxial: number;
    sortShear: number;
    sortMoment: number;
    axialForceI: string;
    shearForceI: string;
    momentI: string;
    axialForceJ: string;
    shearForceJ: string;
    momentJ: string;
  }>;
  planeHotspotLimit: number;
  createdAtValue: string;
  updatedAtValue: string;
  heartbeatStatusValue: string;
  heartbeatTone: "healthy" | "quiet" | "stale";
  failureReasonValue: string;
  canCancelJob: boolean;
  onCancelJob: () => void;
  onDownloadJson: () => void;
  onDownloadCsv: () => void;
  onDownloadPlaneHotspots: () => void;
  onDownloadFrameHotspots: () => void;
  onDownloadFrameForces: () => void;
  onSelectPlaneHotspot: (index: number) => void;
  onSelectFrameHotspot: (index: number) => void;
  onPlaneHotspotLimitChange: (limit: number) => void;
};

type InspectorTab = "properties" | "diagnostics" | "history" | "report";
type FrameForceSort = "index" | "axial" | "shear" | "moment";

function WorkbenchInspectorInner({
  t,
  reportScopeLabel,
  reportScopeHint,
  sidebarSection,
  studyKind,
  isPending,
  selectedNodeData,
  selectedElementData,
  selectedTruss3dNodeData,
  selectedTruss3dElementData,
  selectedPlaneNodeData,
  selectedPlaneElementData,
  selectedFrameNodeData,
  selectedFrameElementData,
  trussElementArea,
  trussElementModulusGpa,
  trussElementMaterialId,
  truss3dElementArea,
  truss3dElementModulusGpa,
  truss3dElementMaterialId,
  planeElementThickness,
  planeElementModulusGpa,
  planeElementPoissonRatio,
  planeElementMaterialId,
  frameElementMaterialId,
  materialOptions,
  materialLabel,
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
  trussDiagnostics,
  trussStability,
  hotspotNodeLabels,
  onApplyTrussSuggestion,
  undoStack,
  redoStack,
  onUndo,
  onRedo,
  job,
  nodeCount,
  tipDisplacement,
  maxStressValue,
  frameMaxAxialForceValue,
  frameMaxShearForceValue,
  reactionValue,
  frameMaxRotationValue,
  planeHotspotFieldLabel,
  planeHotspotElements,
  frameHotspotFieldLabel,
  frameHotspotElements,
  frameForceRows,
  planeHotspotLimit,
  createdAtValue,
  updatedAtValue,
  heartbeatStatusValue,
  heartbeatTone,
  failureReasonValue,
  canCancelJob,
  onCancelJob,
  onDownloadJson,
  onDownloadCsv,
  onDownloadPlaneHotspots,
  onDownloadFrameHotspots,
  onDownloadFrameForces,
  onSelectPlaneHotspot,
  onSelectFrameHotspot,
  onPlaneHotspotLimitChange,
}: WorkbenchInspectorProps) {
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("report");
  const [frameForceSort, setFrameForceSort] = useState<FrameForceSort>("index");
  const isTruss = studyKind === "truss_2d" || studyKind === "thermal_truss_2d";
  const isTruss3d = studyKind === "truss_3d" || studyKind === "thermal_truss_3d";
  const isSpring3d = studyKind === "spring_3d";
  const isPlane = studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d";
  const isThermal = studyKind === "thermal_bar_1d" || studyKind === "thermal_truss_2d" || studyKind === "thermal_truss_3d";
  const isSpring = studyKind === "spring_1d" || studyKind === "spring_2d" || studyKind === "spring_3d";
  const isBeam = studyKind === "beam_1d" || studyKind === "thermal_beam_1d";
  const isTorsion = studyKind === "torsion_1d";
  const isFrame = studyKind === "frame_2d" || studyKind === "thermal_frame_2d";
  const historyRows = [
    ...undoStack.slice(-4).reverse().map((entry) => ({ key: `undo-${entry.label}`, label: entry.label, kind: t.undo })),
    ...redoStack.slice(-2).reverse().map((entry) => ({ key: `redo-${entry.label}`, label: entry.label, kind: t.redo })),
  ];
  const sortedFrameForceRows =
    frameForceSort === "axial"
      ? [...frameForceRows].sort((left, right) => right.sortAxial - left.sortAxial)
      : frameForceSort === "shear"
        ? [...frameForceRows].sort((left, right) => right.sortShear - left.sortShear)
        : frameForceSort === "moment"
          ? [...frameForceRows].sort((left, right) => right.sortMoment - left.sortMoment)
          : frameForceRows;

  return (
    <aside className="workspace-inspector panel">
      <div className="panel-head">
        <h2>{t.overview}</h2>
        <span>{isPending ? t.busy : t.ready}</span>
      </div>
      <div className="inspector-stack panel-scroll-window">
        <div className="panel-tabs panel-tabs--wide">
          <button className={`panel-tab${inspectorTab === "properties" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("properties")} type="button">{t.properties}</button>
          <button className={`panel-tab${inspectorTab === "diagnostics" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("diagnostics")} type="button">{t.diagnostics}</button>
          <button className={`panel-tab${inspectorTab === "history" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("history")} type="button">{t.historyPanel}</button>
          <button className={`panel-tab${inspectorTab === "report" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("report")} type="button">{t.report}</button>
        </div>
        {sidebarSection === "model" && inspectorTab === "properties" ? (
          <section className="info-card">
            <h3>{t.properties}</h3>
            {isTruss && selectedNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedNodeData.x} onChange={(event) => onUpdateSelectedNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedNodeData.y} onChange={(event) => onUpdateSelectedNode("y", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedNodeData.load_x} onChange={(event) => onUpdateSelectedNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedNodeData.load_y} onChange={(event) => onUpdateSelectedNode("load_y", Number(event.target.value))} /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedNodeData.fix_x} onChange={(event) => onUpdateSelectedNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedNodeData.fix_y} onChange={(event) => onUpdateSelectedNode("fix_y", event.target.checked)} /></label>
              </div>
            ) : isTruss && selectedElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedElementData.node_j} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={trussElementMaterialId} onChange={(event) => onAssignSelectedElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={trussElementArea} onChange={(event) => onUpdateSelectedElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={trussElementModulusGpa} onChange={(event) => onUpdateSelectedElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
              </div>
            ) : (isTruss3d || isSpring3d) && selectedTruss3dNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedTruss3dNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedTruss3dNodeData.x} onChange={(event) => onUpdateSelectedTruss3dNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedTruss3dNodeData.y} onChange={(event) => onUpdateSelectedTruss3dNode("y", Number(event.target.value))} /></label>
                <label><span>{t.nodeZ}</span><input type="number" step={0.1} value={selectedTruss3dNodeData.z} onChange={(event) => onUpdateSelectedTruss3dNode("z", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedTruss3dNodeData.load_x} onChange={(event) => onUpdateSelectedTruss3dNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedTruss3dNodeData.load_y} onChange={(event) => onUpdateSelectedTruss3dNode("load_y", Number(event.target.value))} /></label>
                <label><span>{t.loadZ}</span><input type="number" step={100} value={selectedTruss3dNodeData.load_z} onChange={(event) => onUpdateSelectedTruss3dNode("load_z", Number(event.target.value))} /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedTruss3dNodeData.fix_x} onChange={(event) => onUpdateSelectedTruss3dNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedTruss3dNodeData.fix_y} onChange={(event) => onUpdateSelectedTruss3dNode("fix_y", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixZ}</span><input type="checkbox" checked={selectedTruss3dNodeData.fix_z} onChange={(event) => onUpdateSelectedTruss3dNode("fix_z", event.target.checked)} /></label>
              </div>
            ) : isTruss3d && selectedTruss3dElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedTruss3dElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedTruss3dElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedTruss3dElementData.node_j} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={truss3dElementMaterialId} onChange={(event) => onAssignSelectedTruss3dElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={truss3dElementArea} onChange={(event) => onUpdateSelectedTruss3dElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={truss3dElementModulusGpa} onChange={(event) => onUpdateSelectedTruss3dElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
              </div>
            ) : isPlane && selectedPlaneNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedPlaneNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedPlaneNodeData.x} onChange={(event) => onUpdateSelectedPlaneNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedPlaneNodeData.y} onChange={(event) => onUpdateSelectedPlaneNode("y", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedPlaneNodeData.load_x} onChange={(event) => onUpdateSelectedPlaneNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedPlaneNodeData.load_y} onChange={(event) => onUpdateSelectedPlaneNode("load_y", Number(event.target.value))} /></label>
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedPlaneNodeData.displacement_magnitude === "number" ? selectedPlaneNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_x} onChange={(event) => onUpdateSelectedPlaneNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_y} onChange={(event) => onUpdateSelectedPlaneNode("fix_y", event.target.checked)} /></label>
              </div>
            ) : isPlane && selectedPlaneElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedPlaneElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedPlaneElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedPlaneElementData.node_j} readOnly /></label>
                <label><span>{t.nodeK}</span><input value={selectedPlaneElementData.node_k} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={planeElementMaterialId} onChange={(event) => onAssignSelectedPlaneElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.planeThickness}</span><input type="number" step={0.001} value={planeElementThickness} onChange={(event) => onUpdateSelectedPlaneElement("thickness", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={planeElementModulusGpa} onChange={(event) => onUpdateSelectedPlaneElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.poissonRatio}</span><input type="number" step={0.01} min={0.01} max={0.49} value={planeElementPoissonRatio} onChange={(event) => onUpdateSelectedPlaneElement("poisson_ratio", Number(event.target.value))} /></label>
                <label><span>{t.principalStress1}</span><input value={typeof selectedPlaneElementData.principal_stress_1 === "number" ? selectedPlaneElementData.principal_stress_1.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.principalStress2}</span><input value={typeof selectedPlaneElementData.principal_stress_2 === "number" ? selectedPlaneElementData.principal_stress_2.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxInPlaneShear}</span><input value={typeof selectedPlaneElementData.max_in_plane_shear === "number" ? selectedPlaneElementData.max_in_plane_shear.toExponential(3) : "--"} readOnly /></label>
              </div>
            ) : isBeam && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input value={selectedFrameNodeData.x} readOnly /></label>
                <label><span>{t.nodeY}</span><input value={selectedFrameNodeData.y} readOnly /></label>
                <label><span>{t.loadY}</span><input value={selectedFrameNodeData.load_y} readOnly /></label>
                <label><span>{t.momentZ}</span><input value={selectedFrameNodeData.moment_z} readOnly /></label>
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.rotationZ}</span><input value={typeof selectedFrameNodeData.rz === "number" ? selectedFrameNodeData.rz.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedFrameNodeData.fix_y} readOnly /></label>
                <label className="toggle-row"><span>{t.fixRz}</span><input type="checkbox" checked={selectedFrameNodeData.fix_rz} readOnly /></label>
              </div>
            ) : isTorsion && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedFrameNodeData.x} onChange={(event) => onUpdateSelectedFrameNode("x", Number(event.target.value))} /></label>
                <label><span>{t.torqueZ}</span><input type="number" step={100} value={selectedFrameNodeData.moment_z} onChange={(event) => onUpdateSelectedFrameNode("moment_z", Number(event.target.value))} /></label>
                <label><span>{t.rotationZ}</span><input value={typeof selectedFrameNodeData.rz === "number" ? selectedFrameNodeData.rz.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixRz}</span><input type="checkbox" checked={selectedFrameNodeData.fix_rz} onChange={(event) => onUpdateSelectedFrameNode("fix_rz", event.target.checked)} /></label>
              </div>
            ) : isFrame && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedFrameNodeData.x} onChange={(event) => onUpdateSelectedFrameNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedFrameNodeData.y} onChange={(event) => onUpdateSelectedFrameNode("y", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedFrameNodeData.load_x} onChange={(event) => onUpdateSelectedFrameNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedFrameNodeData.load_y} onChange={(event) => onUpdateSelectedFrameNode("load_y", Number(event.target.value))} /></label>
                <label><span>{t.momentZ}</span><input type="number" step={100} value={selectedFrameNodeData.moment_z} onChange={(event) => onUpdateSelectedFrameNode("moment_z", Number(event.target.value))} /></label>
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.rotationZ}</span><input value={typeof selectedFrameNodeData.rz === "number" ? selectedFrameNodeData.rz.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedFrameNodeData.fix_x} onChange={(event) => onUpdateSelectedFrameNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedFrameNodeData.fix_y} onChange={(event) => onUpdateSelectedFrameNode("fix_y", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixRz}</span><input type="checkbox" checked={selectedFrameNodeData.fix_rz} onChange={(event) => onUpdateSelectedFrameNode("fix_rz", event.target.checked)} /></label>
              </div>
            ) : isBeam && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={(selectedFrameElementData.youngs_modulus / 1.0e9).toFixed(3)} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.momentOfInertia}</span><input type="number" step={0.000001} value={selectedFrameElementData.moment_of_inertia} onChange={(event) => onUpdateSelectedFrameElement("moment_of_inertia", Number(event.target.value))} /></label>
                <label><span>{t.sectionModulus}</span><input type="number" step={0.000001} value={selectedFrameElementData.section_modulus} onChange={(event) => onUpdateSelectedFrameElement("section_modulus", Number(event.target.value))} /></label>
                <label><span>{t.distributedLoadY}</span><input type="number" step={100} value={selectedFrameElementData.distributed_load_y ?? 0} onChange={(event) => onUpdateSelectedFrameElement("distributed_load_y", Number(event.target.value))} /></label>
                <label><span>{t.bendingStress}</span><input value={typeof selectedFrameElementData.max_bending_stress === "number" ? selectedFrameElementData.max_bending_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearI}</span><input value={typeof selectedFrameElementData.shear_force_i === "number" ? selectedFrameElementData.shear_force_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentI}</span><input value={typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearJ}</span><input value={typeof selectedFrameElementData.shear_force_j === "number" ? selectedFrameElementData.shear_force_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentJ}</span><input value={typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxMoment}</span><input value={Math.max(Math.abs(selectedFrameElementData.moment_i ?? 0), Math.abs(selectedFrameElementData.moment_j ?? 0)).toExponential(3)} readOnly /></label>
              </div>
            ) : isTorsion && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={(selectedFrameElementData.youngs_modulus / 1.0e9).toFixed(3)} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.momentOfInertia}</span><input type="number" step={0.000001} value={selectedFrameElementData.moment_of_inertia} onChange={(event) => onUpdateSelectedFrameElement("moment_of_inertia", Number(event.target.value))} /></label>
                <label><span>{t.sectionModulus}</span><input type="number" step={0.000001} value={selectedFrameElementData.section_modulus} onChange={(event) => onUpdateSelectedFrameElement("section_modulus", Number(event.target.value))} /></label>
                <label><span>{t.torsionStress}</span><input value={typeof selectedFrameElementData.max_bending_stress === "number" ? selectedFrameElementData.max_bending_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentI}</span><input value={typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentJ}</span><input value={typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxTorque}</span><input value={Math.max(Math.abs(selectedFrameElementData.moment_i ?? 0), Math.abs(selectedFrameElementData.moment_j ?? 0)).toExponential(3)} readOnly /></label>
              </div>
            ) : isFrame && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={frameElementMaterialId} onChange={(event) => onAssignSelectedFrameElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={selectedFrameElementData.area} onChange={(event) => onUpdateSelectedFrameElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={(selectedFrameElementData.youngs_modulus / 1.0e9).toFixed(3)} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.momentOfInertia}</span><input type="number" step={0.000001} value={selectedFrameElementData.moment_of_inertia} onChange={(event) => onUpdateSelectedFrameElement("moment_of_inertia", Number(event.target.value))} /></label>
                <label><span>{t.sectionModulus}</span><input type="number" step={0.000001} value={selectedFrameElementData.section_modulus} onChange={(event) => onUpdateSelectedFrameElement("section_modulus", Number(event.target.value))} /></label>
                <label><span>{t.principalStress1}</span><input value={typeof selectedFrameElementData.axial_stress === "number" ? selectedFrameElementData.axial_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.bendingStress}</span><input value={typeof selectedFrameElementData.max_bending_stress === "number" ? selectedFrameElementData.max_bending_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.combinedStress}</span><input value={typeof selectedFrameElementData.max_combined_stress === "number" ? selectedFrameElementData.max_combined_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.forceI}</span><input value={typeof selectedFrameElementData.axial_force_i === "number" ? selectedFrameElementData.axial_force_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearI}</span><input value={typeof selectedFrameElementData.shear_force_i === "number" ? selectedFrameElementData.shear_force_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentI}</span><input value={typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.forceJ}</span><input value={typeof selectedFrameElementData.axial_force_j === "number" ? selectedFrameElementData.axial_force_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearJ}</span><input value={typeof selectedFrameElementData.shear_force_j === "number" ? selectedFrameElementData.shear_force_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentJ}</span><input value={typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxMoment}</span><input value={Math.max(Math.abs(selectedFrameElementData.moment_i ?? 0), Math.abs(selectedFrameElementData.moment_j ?? 0)).toExponential(3)} readOnly /></label>
              </div>
            ) : (
              <p className="card-copy">{t.selectionHint}</p>
            )}
          </section>
        ) : null}

        {sidebarSection === "model" && isTruss && inspectorTab === "diagnostics" ? (
          <section className="info-card">
            <h3>{t.diagnostics}</h3>
            {trussDiagnostics && trussDiagnostics.blockingMessages.length > 0 ? (
              <div className="diagnostic-list">
                {trussStability ? (
                  <div className={`stability-badge stability-badge--${trussStability.tone}`}>
                    <strong>{t.stabilityScore}</strong>
                    <span>{trussStability.score}/100</span>
                    <small>{trussStability.tone === "good" ? t.stabilityGood : trussStability.tone === "watch" ? t.stabilityWatch : t.stabilityRisk}</small>
                  </div>
                ) : null}
                {trussDiagnostics.blockingMessages.map((issue) => (
                  <div key={issue} className="diagnostic-item"><strong>{issue}</strong></div>
                ))}
                {trussStability && trussStability.hotspotNodes.length > 0 ? (
                  <div className="diagnostic-item"><strong>{t.hotspotNodes}: {hotspotNodeLabels}</strong></div>
                ) : null}
                {trussDiagnostics.suggestions.length > 0 ? <p className="card-copy">{t.suggestedFixes}</p> : null}
                <div className="diagnostic-actions">
                  {trussDiagnostics.suggestions.map((suggestion) => (
                    <button key={suggestion.id} className="ghost-button" onClick={() => onApplyTrussSuggestion(suggestion.id)} type="button">
                      {suggestion.label}
                    </button>
                  ))}
                </div>
              </div>
            ) : (
              <p className="card-copy">{t.diagnosticsClear}</p>
            )}
          </section>
        ) : null}

        {inspectorTab === "history" ? (
        <section className="info-card">
          <h3>{t.historyPanel}</h3>
          <div className="button-row">
            <button className="ghost-button" disabled={undoStack.length === 0} onClick={onUndo} type="button">{t.undo}</button>
            <button className="ghost-button" disabled={redoStack.length === 0} onClick={onRedo} type="button">{t.redo}</button>
          </div>
          {historyRows.length === 0 ? (
            <p className="card-copy">{t.noOperations}</p>
          ) : (
            <VirtualList
              className="history-list"
              items={historyRows}
              itemHeight={74}
              maxHeight={190}
              itemKey={(entry) => entry.key}
              renderItem={(entry) => (
                <div className="history-item">
                  <strong>{entry.label}</strong>
                  <small>{entry.kind}</small>
                </div>
              )}
            />
          )}
        </section>
        ) : null}

        {inspectorTab === "report" ? (
        <section className="info-card">
          <h3>{t.metrics}</h3>
          <div className="metric-grid">
            <div><span>{t.status}</span><strong>{job?.status ?? "--"}</strong></div>
            <div><span>{t.worker}</span><strong>{job?.worker_id ?? "--"}</strong></div>
            <div><span>{t.progress}</span><strong>{typeof job?.progress === "number" ? `${Math.round(job.progress * 100)}%` : "--"}</strong></div>
            <div><span>{t.iteration}</span><strong>{job?.iteration ?? "--"}</strong></div>
            <div><span>{t.residual}</span><strong>{typeof job?.residual === "number" ? job.residual.toExponential(3) : "--"}</strong></div>
            <div><span>{t.nodes}</span><strong>{nodeCount}</strong></div>
          </div>
        </section>
        ) : null}

        {inspectorTab === "report" ? (
        <section className="info-card">
          <h3>{t.report}</h3>
          {reportScopeLabel || reportScopeHint ? (
            <p className="card-copy">
              {[reportScopeLabel, reportScopeHint].filter(Boolean).join(" · ")}
            </p>
          ) : null}
          <div className="button-row">
            <button className="ghost-button" disabled={!canCancelJob} onClick={onCancelJob} type="button">{t.cancelJob}</button>
            <button className="ghost-button" onClick={onDownloadJson} type="button">{t.exportData} {t.exportJson}</button>
            <button className="ghost-button" onClick={onDownloadCsv} type="button">{t.exportData} {t.exportCsv}</button>
          </div>
          <div className="metric-grid">
            <div><span>{t.tipDisp}</span><strong>{tipDisplacement}</strong></div>
            <div><span>{t.maxStress}</span><strong>{maxStressValue}</strong></div>
            {isFrame || isSpring || isThermal ? <div><span>{t.maxAxialForce}</span><strong>{frameMaxAxialForceValue ?? "--"}</strong></div> : null}
            {(isFrame || isBeam) ? <div><span>{t.maxShearForce}</span><strong>{frameMaxShearForceValue ?? "--"}</strong></div> : null}
            <div><span>{t.reaction}</span><strong>{reactionValue}</strong></div>
            {(isFrame || isBeam || isTorsion) ? <div><span>{t.maxRotation}</span><strong>{frameMaxRotationValue ?? "--"}</strong></div> : null}
            <div><span>{t.createdAt}</span><strong>{createdAtValue}</strong></div>
            <div><span>{t.updatedAt}</span><strong>{updatedAtValue}</strong></div>
            <div><span>{t.lastHeartbeat}</span><strong>{updatedAtValue}</strong></div>
            <div><span>{t.heartbeatStatus}</span><strong><span className={`heartbeat-badge heartbeat-badge--${heartbeatTone}`}>{heartbeatStatusValue}</span></strong></div>
            <div><span>{t.hasResult}</span><strong>{job?.has_result ? t.yes : t.no}</strong></div>
            <div><span>{t.failureReason}</span><strong>{failureReasonValue}</strong></div>
          </div>
          {isPlane ? (
            <div className="diagnostic-list">
              <div className="diagnostic-item">
                <strong>{t.currentField}: {planeHotspotFieldLabel ?? "--"}</strong>
              </div>
              <div className="button-row">
                <span className="card-copy">{t.topN}</span>
                {[3, 5, 10].map((limit) => (
                  <button
                    key={limit}
                    className={`ghost-button ghost-button--compact${planeHotspotLimit === limit ? " ghost-button--active" : ""}`}
                    onClick={() => onPlaneHotspotLimitChange(limit)}
                    type="button"
                  >
                    {limit}
                  </button>
                ))}
                <button className="ghost-button ghost-button--compact" onClick={onDownloadPlaneHotspots} type="button">
                  {t.exportHotspots}
                </button>
              </div>
              {planeHotspotElements.length > 0 ? (
                <>
                  <p className="card-copy">{t.planeHotspots}</p>
                  {planeHotspotElements.map((entry) => (
                    <button
                      key={entry.id}
                      className={`history-item${entry.active ? " history-item--active" : ""}`}
                      onClick={() => onSelectPlaneHotspot(entry.index)}
                      type="button"
                    >
                      <strong>{entry.id}</strong>
                      <small>{entry.value}</small>
                    </button>
                  ))}
                </>
              ) : (
                <p className="card-copy">--</p>
              )}
            </div>
          ) : isFrame || isBeam || isTorsion || isSpring ? (
            <div className="diagnostic-list">
              <div className="diagnostic-item">
                <strong>{t.currentField}: {frameHotspotFieldLabel ?? "--"}</strong>
              </div>
              <div className="button-row">
                <span className="card-copy">{t.topN}</span>
                {[3, 5, 10].map((limit) => (
                  <button
                    key={limit}
                    className={`ghost-button ghost-button--compact${planeHotspotLimit === limit ? " ghost-button--active" : ""}`}
                    onClick={() => onPlaneHotspotLimitChange(limit)}
                    type="button"
                  >
                    {limit}
                  </button>
                ))}
                <button className="ghost-button ghost-button--compact" onClick={onDownloadFrameHotspots} type="button">
                  {t.exportHotspots}
                </button>
              </div>
              {frameHotspotElements.length > 0 ? (
                <>
                  <p className="card-copy">{t.frameElements}</p>
                  {frameHotspotElements.map((entry) => (
                    <button
                      key={entry.id}
                      className={`history-item${entry.active ? " history-item--active" : ""}`}
                      onClick={() => onSelectFrameHotspot(entry.index)}
                      type="button"
                    >
                      <strong>{entry.id}</strong>
                      <small>{entry.value}</small>
                      {entry.summary ? <small>{entry.summary}</small> : null}
                    </button>
                  ))}
                </>
              ) : (
                <p className="card-copy">--</p>
              )}
              {selectedFrameElementData ? (
                <>
                  <p className="card-copy">{t.memberEndForces}</p>
                  <div className="metric-grid">
                    {!isBeam && !isTorsion ? <div><span>{t.forceI}</span><strong>{typeof selectedFrameElementData.axial_force_i === "number" ? selectedFrameElementData.axial_force_i.toExponential(3) : "--"}</strong></div> : null}
                    {!isSpring && !isThermal && !isTorsion ? <div><span>{t.shearI}</span><strong>{typeof selectedFrameElementData.shear_force_i === "number" ? selectedFrameElementData.shear_force_i.toExponential(3) : "--"}</strong></div> : null}
                    {!isSpring && !isThermal ? <div><span>{t.momentI}</span><strong>{typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"}</strong></div> : null}
                    {!isBeam && !isTorsion ? <div><span>{t.forceJ}</span><strong>{typeof selectedFrameElementData.axial_force_j === "number" ? selectedFrameElementData.axial_force_j.toExponential(3) : "--"}</strong></div> : null}
                    {!isSpring && !isThermal && !isTorsion ? <div><span>{t.shearJ}</span><strong>{typeof selectedFrameElementData.shear_force_j === "number" ? selectedFrameElementData.shear_force_j.toExponential(3) : "--"}</strong></div> : null}
                    {!isSpring && !isThermal ? <div><span>{t.momentJ}</span><strong>{typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"}</strong></div> : null}
                  </div>
                </>
              ) : null}
              {frameForceRows.length > 0 ? (
                <div className="table-like table-like--console">
                  <h4>{t.memberForceTable}</h4>
                  <div className="button-row">
                    <span className="card-copy">{t.sortBy}</span>
                    <button className={`ghost-button ghost-button--compact${frameForceSort === "index" ? " ghost-button--active" : ""}`} onClick={() => setFrameForceSort("index")} type="button">#</button>
                    {!isBeam && !isTorsion ? <button className={`ghost-button ghost-button--compact${frameForceSort === "axial" ? " ghost-button--active" : ""}`} onClick={() => setFrameForceSort("axial")} type="button">{t.axialForce}</button> : null}
                    {!isSpring && !isTorsion ? <button className={`ghost-button ghost-button--compact${frameForceSort === "shear" ? " ghost-button--active" : ""}`} onClick={() => setFrameForceSort("shear")} type="button">{t.shearForce}</button> : null}
                    {!isSpring ? <button className={`ghost-button ghost-button--compact${frameForceSort === "moment" ? " ghost-button--active" : ""}`} onClick={() => setFrameForceSort("moment")} type="button">{t.maxMoment}</button> : null}
                    <button className="ghost-button ghost-button--compact" onClick={onDownloadFrameForces} type="button">{t.exportMemberForces}</button>
                  </div>
                  <div className="table-like__head table-like__head--frame-forces">
                    <span>#</span>
                    {!isBeam && !isTorsion ? <span>{t.forceI}</span> : null}
                    {!isSpring && !isTorsion ? <span>{t.shearI}</span> : null}
                    {!isSpring ? <span>{t.momentI}</span> : null}
                    {!isBeam && !isTorsion ? <span>{t.forceJ}</span> : null}
                    {!isSpring && !isTorsion ? <span>{t.shearJ}</span> : null}
                    {!isSpring ? <span>{t.momentJ}</span> : null}
                  </div>
                  <VirtualList
                    className="table-like__body"
                    items={sortedFrameForceRows}
                    itemHeight={46}
                    maxHeight={240}
                    itemKey={(entry) => `${entry.id}-${entry.index}`}
                    renderItem={(entry) => (
                      <button
                        className={`table-like__row table-like__row--frame-forces${entry.active ? " history-item--active" : ""}`}
                        onClick={() => onSelectFrameHotspot(entry.index)}
                        type="button"
                      >
                        <strong>{entry.id}</strong>
                        {!isBeam && !isTorsion ? <span>{entry.axialForceI}</span> : null}
                        {!isSpring && !isTorsion ? <span>{entry.shearForceI}</span> : null}
                        {!isSpring ? <span>{entry.momentI}</span> : null}
                        {!isBeam && !isTorsion ? <span>{entry.axialForceJ}</span> : null}
                        {!isSpring && !isTorsion ? <span>{entry.shearForceJ}</span> : null}
                        {!isSpring ? <span>{entry.momentJ}</span> : null}
                      </button>
                    )}
                  />
                </div>
              ) : null}
            </div>
          ) : null}
        </section>
        ) : null}
      </div>
    </aside>
  );
}

export const WorkbenchInspector = memo(WorkbenchInspectorInner);
