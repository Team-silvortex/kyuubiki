"use client";

import { useEffect, useState, type ChangeEvent } from "react";
import type {
  WorkbenchSystemControlModeCopy,
  WorkbenchSystemControlTopologySummary,
  WorkbenchSystemTopologySnapshot,
  WorkbenchSystemTopologySnapshotSource,
} from "@/components/workbench/system/workbench-system-control-mode-contract";

type WindowMode = "orchestrated" | "direct" | "mesh";

type WorkbenchSystemControlModeWindowProps = {
  copy: WorkbenchSystemControlModeCopy;
  topology: WorkbenchSystemControlTopologySummary;
  snapshot: WorkbenchSystemTopologySnapshot;
  snapshotSource: WorkbenchSystemTopologySnapshotSource;
  onImportSnapshot: (file: File) => void;
  onResetSnapshotSource: () => void;
};

function deriveWindowMode(mode: WorkbenchSystemControlTopologySummary["mode"]): WindowMode {
  return mode;
}

export function WorkbenchSystemControlModeWindow({
  copy,
  topology,
  snapshot,
  snapshotSource,
  onImportSnapshot,
  onResetSnapshotSource,
}: WorkbenchSystemControlModeWindowProps) {
  const [windowMode, setWindowMode] = useState<WindowMode>(deriveWindowMode(topology.mode));

  useEffect(() => {
    setWindowMode((current) => (current === "mesh" ? current : deriveWindowMode(topology.mode)));
  }, [topology.mode]);

  function handleExportSnapshot() {
    const blob = new Blob([JSON.stringify(snapshot, null, 2)], { type: "application/json;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `mesh-topology-${snapshot.observed_at.replace(/[:.]/g, "-")}.json`;
    link.click();
    URL.revokeObjectURL(url);
  }

  function handleSnapshotFileChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    onImportSnapshot(file);
    event.target.value = "";
  }

  return (
    <section
      className="sidebar-card sidebar-card--compact"
      data-workbench-control-source={snapshotSource.kind}
      data-workbench-control-window="root"
      data-workbench-surface="built-in"
    >
      <div className="card-head">
        <h2>{copy.title}</h2>
        <span>{copy.activeRuntimeModeLabel}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide" data-workbench-control-window="tabs">
        <button className={`panel-tab${windowMode === "orchestrated" ? " panel-tab--active" : ""}`} data-workbench-control-mode-tab="orchestrated" onClick={() => setWindowMode("orchestrated")} type="button">
          {copy.tabs.orchestrated}
        </button>
        <button className={`panel-tab${windowMode === "direct" ? " panel-tab--active" : ""}`} data-workbench-control-mode-tab="direct" onClick={() => setWindowMode("direct")} type="button">
          {copy.tabs.direct}
        </button>
        <button className={`panel-tab${windowMode === "mesh" ? " panel-tab--active" : ""}`} data-workbench-control-mode-tab="mesh" onClick={() => setWindowMode("mesh")} type="button">
          {copy.tabs.mesh}
        </button>
      </div>
      <div className="card-copy">
        <strong>{copy.topologyWindowLabel}</strong>
        <p>{copy.topologyWindowHint}</p>
      </div>
      <article className="control-mode-window">
        <header className="control-mode-window__head">
          <div>
            <strong>{copy.windows[windowMode].title}</strong>
            <span>{copy.modeLabel}</span>
          </div>
          <span className="status-chip status-chip--watch">{copy.tabs[windowMode]}</span>
        </header>
        <div className="sidebar-list sidebar-list--metrics" data-workbench-control-window="snapshot-meta">
          <div className="sidebar-list__row">
            <span>source</span>
            <strong>{snapshotSource.label}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.snapshotVersionLabel}</span>
            <strong>{snapshot.schema.version}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.snapshotObservedAtLabel}</span>
            <strong>{snapshot.observed_at}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.rows.safeModeLabel}</span>
            <strong>{topology.safeModeActive ? copy.statuses.ready : copy.statuses.open}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.rows.downgradeReasonLabel}</span>
            <strong>{topology.downgradeReason}</strong>
          </div>
        </div>
        <p className="control-mode-window__hint">{copy.windows[windowMode].hint}</p>
        <div className="sidebar-list sidebar-list--metrics" data-workbench-control-window="metrics">
          <div className="sidebar-list__row">
            <span>{copy.rows.currentRuntimeLabel}</span>
            <strong>{topology.runtimeLabel}</strong>
          </div>
          {windowMode === "orchestrated" ? (
            <>
              <div className="sidebar-list__row">
                <span>{copy.rows.protocolStatusLabel}</span>
                <strong>{topology.protocolOnline ? copy.statuses.online : copy.statuses.offline}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.securityStatusLabel}</span>
                <strong>{topology.securityConfigured ? copy.statuses.ready : copy.statuses.open}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.auditCountLabel}</span>
                <strong>{topology.auditCount}</strong>
              </div>
            </>
          ) : null}
          {windowMode === "direct" ? (
            <>
              <div className="sidebar-list__row">
                <span>{copy.rows.directStrategyLabel}</span>
                <strong>{topology.directStrategyLabel}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.endpointCountLabel}</span>
                <strong>{topology.endpointCount}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.agentCountLabel}</span>
                <strong>{topology.visibleAgentCount}</strong>
              </div>
            </>
          ) : null}
          {windowMode === "mesh" ? (
            <>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshEntryLabel}</span>
                <strong>{topology.entryAgentId}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshEntryHealthLabel}</span>
                <strong>{topology.entryHealthLabel}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshPeersLabel}</span>
                <strong>{topology.peerCount}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshGraphLabel}</span>
                <strong>{topology.graphSummaryLabel}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshRouteTraceLabel}</span>
                <strong>{topology.routeTraceLabel}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshLastSeenLabel}</span>
                <strong>{topology.lastSeenLabel}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshHopLabel}</span>
                <strong>{topology.estimatedHopCount}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshRoutingLabel}</span>
                <strong>{topology.routingPolicy}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshFallbackLabel}</span>
                <strong>{topology.fallbackPolicy}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{copy.rows.meshFailoverReasonLabel}</span>
                <strong>{topology.failoverReason}</strong>
              </div>
            </>
          ) : null}
        </div>
        <div className="button-row" data-workbench-control-window="actions">
          <button data-workbench-control-action="export-snapshot" onClick={handleExportSnapshot} type="button">
            {copy.exportSnapshotLabel}
          </button>
          <label className="ghost-button">
            <input
              accept="application/json"
              data-workbench-control-action="import-snapshot"
              onChange={handleSnapshotFileChange}
              style={{ display: "none" }}
              type="file"
            />
            <span>Load snapshot</span>
          </label>
          {snapshotSource.kind === "imported_snapshot" ? (
            <button data-workbench-control-action="reset-snapshot-source" onClick={onResetSnapshotSource} type="button">
              Use live derived
            </button>
          ) : null}
        </div>
        {windowMode === "mesh" ? <p className="control-mode-window__foot">{copy.meshPlannedHint}</p> : null}
      </article>
    </section>
  );
}
