"use client";

import { VirtualList } from "@/components/ui/virtual-list";
import type {
  ActionsPage,
  HistoryEntry,
  InspectorLabels,
  ResultPage,
  StabilitySummary,
  StatusPage,
  TrussDiagnostics,
} from "./workbench-inspector-types";

type InspectorTabChromeProps = {
  t: InspectorLabels;
  inspectorTab: "status" | "result" | "actions";
  statusPage: StatusPage;
  actionsPage: ActionsPage;
  resultPage: ResultPage;
  onStatusPageChange: (page: StatusPage) => void;
  onActionsPageChange: (page: ActionsPage) => void;
  onResultPageChange: (page: ResultPage) => void;
};

type DiagnosticsPanelProps = {
  t: InspectorLabels;
  isTruss: boolean;
  trussDiagnostics: TrussDiagnostics | null;
  trussStability: StabilitySummary | null;
  hotspotNodeLabels: string;
  onApplyTrussSuggestion: (id: string) => void;
};

type HistoryPanelProps = {
  t: InspectorLabels;
  undoStack: HistoryEntry[];
  redoStack: HistoryEntry[];
  onUndo: () => void;
  onRedo: () => void;
};

export function WorkbenchInspectorTabChrome({
  t,
  inspectorTab,
  statusPage,
  actionsPage,
  resultPage,
  onStatusPageChange,
  onActionsPageChange,
  onResultPageChange,
}: InspectorTabChromeProps) {
  return (
    <>
      {inspectorTab === "status" ? (
        <section className="info-card">
          <div className="panel-tabs panel-tabs--wide">
            <button className={`panel-tab${statusPage === "properties" ? " panel-tab--active" : ""}`} onClick={() => onStatusPageChange("properties")} type="button">{t.properties}</button>
            <button className={`panel-tab${statusPage === "diagnostics" ? " panel-tab--active" : ""}`} onClick={() => onStatusPageChange("diagnostics")} type="button">{t.diagnostics}</button>
          </div>
        </section>
      ) : null}
      {inspectorTab === "actions" ? (
        <section className="info-card">
          <div className="panel-tabs panel-tabs--wide">
            <button className={`panel-tab${actionsPage === "history" ? " panel-tab--active" : ""}`} onClick={() => onActionsPageChange("history")} type="button">{t.historyPanel}</button>
            <button className={`panel-tab${actionsPage === "exports" ? " panel-tab--active" : ""}`} onClick={() => onActionsPageChange("exports")} type="button">{t.exportData}</button>
          </div>
        </section>
      ) : null}
      {inspectorTab === "result" ? (
        <section className="info-card">
          <div className="panel-tabs panel-tabs--wide">
            <button className={`panel-tab${resultPage === "summary" ? " panel-tab--active" : ""}`} onClick={() => onResultPageChange("summary")} type="button">{t.summary}</button>
            <button className={`panel-tab${resultPage === "details" ? " panel-tab--active" : ""}`} onClick={() => onResultPageChange("details")} type="button">{t.details}</button>
          </div>
        </section>
      ) : null}
    </>
  );
}

export function WorkbenchInspectorDiagnosticsPanel({
  t,
  isTruss,
  trussDiagnostics,
  trussStability,
  hotspotNodeLabels,
  onApplyTrussSuggestion,
}: DiagnosticsPanelProps) {
  return (
    <section className="info-card">
      <h3>{t.diagnostics}</h3>
      {isTruss && trussDiagnostics && trussDiagnostics.blockingMessages.length > 0 ? (
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
      ) : isTruss ? (
        <p className="card-copy">{t.diagnosticsClear}</p>
      ) : (
        <p className="card-copy">{t.selectionHint}</p>
      )}
    </section>
  );
}

export function WorkbenchInspectorHistoryPanel({
  t,
  undoStack,
  redoStack,
  onUndo,
  onRedo,
}: HistoryPanelProps) {
  const historyRows = [
    ...undoStack.slice(-4).reverse().map((entry) => ({ key: `undo-${entry.label}`, label: entry.label, kind: t.undo })),
    ...redoStack.slice(-2).reverse().map((entry) => ({ key: `redo-${entry.label}`, label: entry.label, kind: t.redo })),
  ];

  return (
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
  );
}
