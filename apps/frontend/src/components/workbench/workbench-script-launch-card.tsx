"use client";

import { WorkbenchAlertStrip } from "@/components/workbench/workbench-alert-strip";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";

type RuntimeStatus = "idle" | "loading" | "ready" | "running" | "error";

type WorkbenchScriptLaunchCardProps = {
  clearOutput: () => void;
  copy: WorkbenchScriptPanelCopyEntry;
  loadRuntime: () => void;
  recordingMode: boolean;
  resetScript: () => void;
  runScript: () => void;
  runtimeError: string | null;
  runtimeStatus: RuntimeStatus;
  toggleRecordingMode: () => void;
};

export function WorkbenchScriptLaunchCard({
  clearOutput,
  copy,
  loadRuntime,
  recordingMode,
  resetScript,
  runScript,
  runtimeError,
  runtimeStatus,
  toggleRecordingMode,
}: WorkbenchScriptLaunchCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.launch}</h2>
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
      <p className="card-copy">{copy.title}</p>
      <p className="card-copy">{copy.subtitle}</p>
      <p className="card-copy">{copy.frontendSurfaceHint}</p>
      <p className="card-copy">{copy.firstRun}</p>
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
        <button className="ghost-button" onClick={loadRuntime} type="button">
          {copy.loadRuntime}
        </button>
        <button className="ghost-button" onClick={resetScript} type="button">
          {copy.resetScript}
        </button>
        <button className="ghost-button" onClick={runScript} type="button">
          {copy.runScript}
        </button>
        <button className={`ghost-button${recordingMode ? " ghost-button--active" : ""}`} onClick={toggleRecordingMode} type="button">
          {recordingMode ? copy.stopRecording : copy.startRecording}
        </button>
        <button className="ghost-button" onClick={clearOutput} type="button">
          {copy.clearOutput}
        </button>
      </div>
      {recordingMode ? <p className="card-copy">{copy.recordingActive}</p> : null}
    </section>
  );
}
