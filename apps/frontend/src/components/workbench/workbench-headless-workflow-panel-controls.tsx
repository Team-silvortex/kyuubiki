"use client";

import { buildHeadlessWorkflowPanelCopy } from "@/components/workbench/workbench-headless-workflow-panel-helpers";
import type { WorkbenchRecordedMacroDraft } from "@/lib/scripting/workbench-script-runtime";

type HeadlessPanelCopy = ReturnType<typeof buildHeadlessWorkflowPanelCopy>;

export function WorkbenchHeadlessWorkflowPanelControls({
  draft,
  executionLog,
  executionRunning,
  onExportBatch,
  onExportDispatch,
  onExportHandoff,
  onExportHtml,
  onExportJson,
  onImportJson,
  onInsert,
  onRefreshHandoff,
  onRefreshHistory,
  onRunBatch,
  onSubmitHandoff,
  ui,
}: {
  draft: WorkbenchRecordedMacroDraft | null;
  executionLog: string[];
  executionRunning: boolean;
  onExportBatch(): void;
  onExportDispatch(): void;
  onExportHandoff(): void;
  onExportHtml(): void;
  onExportJson(): void;
  onImportJson(file: File | undefined): void;
  onInsert(): void;
  onRefreshHandoff(): void;
  onRefreshHistory(): void;
  onRunBatch(): void;
  onSubmitHandoff(): void;
  ui: HeadlessPanelCopy;
}) {
  return (
    <>
      <div className="button-row">
        <label className="ghost-button" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
          {ui.importJson}
          <input
            accept="application/json,.json"
            hidden
            onChange={(event) => {
              onImportJson(event.target.files?.[0]);
              event.currentTarget.value = "";
            }}
            type="file"
          />
        </label>
        <button className="ghost-button" disabled={executionRunning} onClick={onRunBatch} type="button">
          {ui.runBatch}
        </button>
        <button className="ghost-button" onClick={onExportJson} type="button">
          {ui.exportJson}
        </button>
        <button className="ghost-button" onClick={onExportBatch} type="button">
          {ui.exportBatch}
        </button>
        <button className="ghost-button" onClick={onExportDispatch} type="button">
          {ui.exportDispatch}
        </button>
        <button className="ghost-button" onClick={onExportHandoff} type="button">
          {ui.exportHandoff}
        </button>
        <button className="ghost-button" onClick={onSubmitHandoff} type="button">
          {ui.submitHandoff}
        </button>
        <button className="ghost-button" onClick={onRefreshHandoff} type="button">
          {ui.refreshHandoff}
        </button>
        <button className="ghost-button" onClick={onRefreshHistory} type="button">
          {ui.refreshHistory}
        </button>
        <button className="ghost-button" onClick={onExportHtml} type="button">
          {ui.exportHtml}
        </button>
        <button className="ghost-button" onClick={onInsert} type="button">
          {ui.insert}
        </button>
      </div>
      {executionLog.length > 0 ? (
        <label className="field-label">
          <span>{ui.executionLog}</span>
          <pre className="script-panel__snapshot">{executionLog.join("\n")}</pre>
        </label>
      ) : null}
      <pre className="script-panel__snapshot">{draft ? JSON.stringify(draft, null, 2) : "{\n  \"error\": \"invalid payload json\"\n}"}</pre>
    </>
  );
}
