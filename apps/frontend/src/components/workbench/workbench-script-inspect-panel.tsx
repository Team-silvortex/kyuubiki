"use client";

import { useState } from "react";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";
import type { WorkbenchScriptActionLogEntry, WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";

type InspectMode = "output" | "timeline" | "snapshot";

type WorkbenchScriptInspectPanelProps = {
  actionCatalogCount: number;
  actionLog: WorkbenchScriptActionLogEntry[];
  copy: WorkbenchScriptPanelCopyEntry;
  macroCatalogCount: number;
  output: string[];
  scriptCode: string;
  snapshot: WorkbenchScriptSnapshot;
};

export function WorkbenchScriptInspectPanel({
  actionCatalogCount,
  actionLog,
  copy,
  macroCatalogCount,
  output,
  scriptCode,
  snapshot,
}: WorkbenchScriptInspectPanelProps) {
  const [mode, setMode] = useState<InspectMode>("output");

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.inspect}</h2>
        <span>{mode === "output" ? copy.output : mode === "timeline" ? copy.timelineMode : copy.snapshot}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${mode === "output" ? " panel-tab--active" : ""}`} onClick={() => setMode("output")} type="button">
          {copy.outputMode}
        </button>
        <button className={`panel-tab${mode === "timeline" ? " panel-tab--active" : ""}`} onClick={() => setMode("timeline")} type="button">
          {copy.timelineMode}
        </button>
        <button className={`panel-tab${mode === "snapshot" ? " panel-tab--active" : ""}`} onClick={() => setMode("snapshot")} type="button">
          {copy.snapshotMode}
        </button>
      </div>
      <div className="sidebar-list">
        <div>
          <span>{copy.stateKeys}</span>
          <strong>{Object.keys(snapshot).length}</strong>
        </div>
        <div>
          <span>{copy.actionCount}</span>
          <strong>{actionCatalogCount + macroCatalogCount}</strong>
        </div>
        <div>
          <span>{copy.lineCount}</span>
          <strong>{scriptCode.split("\n").length}</strong>
        </div>
      </div>
      {mode === "output" ? (
        <>
          <p className="card-copy">{copy.inspectOutputHint}</p>
          {output.length === 0 ? <p className="card-copy">{copy.noOutput}</p> : <pre className="script-panel__output">{output.join("\n")}</pre>}
        </>
      ) : null}
      {mode === "timeline" ? (
        <>
          <p className="card-copy">{copy.inspectTimelineHint}</p>
          {actionLog.length === 0 ? (
            <p className="card-copy">{copy.noActionLog}</p>
          ) : (
            <div className="script-panel__catalog">
              {actionLog.map((entry, index) => (
                <article className="script-panel__action" key={entry.id}>
                  <div className="script-panel__action-head">
                    <strong>{`${index + 1}. ${entry.action}`}</strong>
                    <span>{entry.status}</span>
                  </div>
                  <p className="card-copy">{entry.summary}</p>
                  <div className="script-panel__payload">
                    <span>{copy.actionSource}</span>
                    <code>{entry.source ?? "--"}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>{copy.actionNote}</span>
                    <code>{entry.note ?? "--"}</code>
                  </div>
                  <div className="script-panel__payload">
                    <span>Time</span>
                    <code>{entry.at}</code>
                  </div>
                </article>
              ))}
            </div>
          )}
        </>
      ) : null}
      {mode === "snapshot" ? (
        <>
          <p className="card-copy">{copy.inspectSnapshotHint}</p>
          <pre className="script-panel__snapshot">{JSON.stringify(snapshot, null, 2)}</pre>
        </>
      ) : null}
    </section>
  );
}
