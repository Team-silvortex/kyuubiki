"use client";

import { memo } from "react";
import { VirtualList } from "@/components/virtual-list";

type SidebarSection = "study" | "model" | "library" | "system";
type StudyKind = "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";

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

type PlaneNodeSelection = {
  id: string;
  x: number;
  y: number;
  load_x: number;
  load_y: number;
  fix_x: boolean;
  fix_y: boolean;
};

type PlaneElementSelection = {
  id: string;
  index: number;
  node_i: number;
  node_j: number;
  node_k: number;
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
  loadX: string;
  loadY: string;
  fixX: string;
  fixY: string;
  memberSelection: string;
  nodeI: string;
  nodeJ: string;
  nodeK: string;
  area: string;
  modulus: string;
  planeThickness: string;
  poissonRatio: string;
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
  reaction: string;
  createdAt: string;
  updatedAt: string;
  hasResult: string;
  failureReason: string;
  yes: string;
  no: string;
};

type WorkbenchInspectorProps = {
  t: InspectorLabels;
  sidebarSection: SidebarSection;
  studyKind: StudyKind;
  isPending: boolean;
  selectedNodeData: TrussNodeSelection | null;
  selectedElementData: TrussElementSelection | null;
  selectedPlaneNodeData: PlaneNodeSelection | null;
  selectedPlaneElementData: PlaneElementSelection | null;
  trussElementArea: number;
  trussElementModulusGpa: number;
  planeElementThickness: number;
  planeElementModulusGpa: number;
  planeElementPoissonRatio: number;
  onUpdateSelectedNode: (field: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y", value: number | boolean) => void;
  onUpdateSelectedElement: (field: "area" | "youngs_modulus", value: number) => void;
  onUpdateSelectedPlaneNode: (field: "x" | "y" | "load_x" | "load_y" | "fix_x" | "fix_y", value: number | boolean) => void;
  onUpdateSelectedPlaneElement: (field: "thickness" | "youngs_modulus" | "poisson_ratio", value: number) => void;
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
  reactionValue: string;
  createdAtValue: string;
  updatedAtValue: string;
  failureReasonValue: string;
  onDownloadJson: () => void;
  onDownloadCsv: () => void;
};

function WorkbenchInspectorInner({
  t,
  sidebarSection,
  studyKind,
  isPending,
  selectedNodeData,
  selectedElementData,
  selectedPlaneNodeData,
  selectedPlaneElementData,
  trussElementArea,
  trussElementModulusGpa,
  planeElementThickness,
  planeElementModulusGpa,
  planeElementPoissonRatio,
  onUpdateSelectedNode,
  onUpdateSelectedElement,
  onUpdateSelectedPlaneNode,
  onUpdateSelectedPlaneElement,
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
  reactionValue,
  createdAtValue,
  updatedAtValue,
  failureReasonValue,
  onDownloadJson,
  onDownloadCsv,
}: WorkbenchInspectorProps) {
  const isTruss = studyKind === "truss_2d";
  const isPlane = studyKind === "plane_triangle_2d";
  const historyRows = [
    ...undoStack.slice(-4).reverse().map((entry) => ({ key: `undo-${entry.label}`, label: entry.label, kind: t.undo })),
    ...redoStack.slice(-2).reverse().map((entry) => ({ key: `redo-${entry.label}`, label: entry.label, kind: t.redo })),
  ];

  return (
    <aside className="workspace-inspector panel">
      <div className="panel-head">
        <h2>{t.overview}</h2>
        <span>{isPending ? t.busy : t.ready}</span>
      </div>
      <div className="inspector-stack">
        {sidebarSection === "model" ? (
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
                <label><span>{t.area}</span><input type="number" step={0.0001} value={trussElementArea} onChange={(event) => onUpdateSelectedElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={trussElementModulusGpa} onChange={(event) => onUpdateSelectedElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
              </div>
            ) : isPlane && selectedPlaneNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedPlaneNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedPlaneNodeData.x} onChange={(event) => onUpdateSelectedPlaneNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedPlaneNodeData.y} onChange={(event) => onUpdateSelectedPlaneNode("y", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedPlaneNodeData.load_x} onChange={(event) => onUpdateSelectedPlaneNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedPlaneNodeData.load_y} onChange={(event) => onUpdateSelectedPlaneNode("load_y", Number(event.target.value))} /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_x} onChange={(event) => onUpdateSelectedPlaneNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_y} onChange={(event) => onUpdateSelectedPlaneNode("fix_y", event.target.checked)} /></label>
              </div>
            ) : isPlane && selectedPlaneElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedPlaneElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedPlaneElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedPlaneElementData.node_j} readOnly /></label>
                <label><span>{t.nodeK}</span><input value={selectedPlaneElementData.node_k} readOnly /></label>
                <label><span>{t.planeThickness}</span><input type="number" step={0.001} value={planeElementThickness} onChange={(event) => onUpdateSelectedPlaneElement("thickness", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={planeElementModulusGpa} onChange={(event) => onUpdateSelectedPlaneElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.poissonRatio}</span><input type="number" step={0.01} min={0.01} max={0.49} value={planeElementPoissonRatio} onChange={(event) => onUpdateSelectedPlaneElement("poisson_ratio", Number(event.target.value))} /></label>
              </div>
            ) : (
              <p className="card-copy">{t.selectionHint}</p>
            )}
          </section>
        ) : null}

        {sidebarSection === "model" && isTruss ? (
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

        <section className="info-card">
          <h3>{t.report}</h3>
          <div className="button-row">
            <button className="ghost-button" onClick={onDownloadJson} type="button">{t.exportData} {t.exportJson}</button>
            <button className="ghost-button" onClick={onDownloadCsv} type="button">{t.exportData} {t.exportCsv}</button>
          </div>
          <div className="metric-grid">
            <div><span>{t.tipDisp}</span><strong>{tipDisplacement}</strong></div>
            <div><span>{t.maxStress}</span><strong>{maxStressValue}</strong></div>
            <div><span>{t.reaction}</span><strong>{reactionValue}</strong></div>
            <div><span>{t.createdAt}</span><strong>{createdAtValue}</strong></div>
            <div><span>{t.updatedAt}</span><strong>{updatedAtValue}</strong></div>
            <div><span>{t.hasResult}</span><strong>{job?.has_result ? t.yes : t.no}</strong></div>
            <div><span>{t.failureReason}</span><strong>{failureReasonValue}</strong></div>
          </div>
        </section>
      </div>
    </aside>
  );
}

export const WorkbenchInspector = memo(WorkbenchInspectorInner);
