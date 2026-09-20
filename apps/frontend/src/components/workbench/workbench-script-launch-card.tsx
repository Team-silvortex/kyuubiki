"use client";

import { WorkbenchAlertStrip } from "@/components/workbench/workbench-alert-strip";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";

type RuntimeStatus = "idle" | "loading" | "ready" | "running" | "error";

type WorkbenchScriptLaunchCardProps = {
  copy: WorkbenchScriptPanelCopyEntry;
  loadRuntime: () => void;
  resetScript: () => void;
  runScript: () => void;
  runtimeError: string | null;
  runtimeStatus: RuntimeStatus;
};

export function WorkbenchScriptLaunchCard({
  copy,
  loadRuntime,
  resetScript,
  runScript,
  runtimeError,
  runtimeStatus,
}: WorkbenchScriptLaunchCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact pwdt-launch-card">
      <div className="card-head">
        <h2>{copy.runtime}</h2>
        <span className={`status-chip status-chip--${runtimeStatus === "error" ? "risk" : runtimeStatus === "ready" ? "good" : "watch"}`}>
          {runtimeStatus === "loading"
            ? copy.loading
            : runtimeStatus === "ready"
              ? copy.ready
              : runtimeStatus === "running"
                ? copy.running
                : runtimeStatus === "error"
                  ? copy.error
                  : copy.idle}
        </span>
      </div>
      <WorkbenchAlertStrip
        alerts={
          runtimeError
            ? [
                {
                  id: "script-runtime-error",
                  message: runtimeError,
                  tone: "error",
                },
              ]
            : []
        }
      />
      <div className="button-row">
        <button className="ghost-button" disabled={runtimeStatus === "loading" || runtimeStatus === "running"} onClick={loadRuntime} type="button">
          {copy.loadRuntime}
        </button>
        <button className="ghost-button" onClick={resetScript} type="button">
          {copy.resetScript}
        </button>
        <button className="ghost-button" disabled={runtimeStatus === "loading" || runtimeStatus === "running"} onClick={runScript} type="button">
          {copy.runScript}
        </button>
      </div>
    </section>
  );
}
