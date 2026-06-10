"use client";

import { useState } from "react";
import type { WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";

type WorkbenchScriptAuthorPanelProps = {
  copy: WorkbenchScriptPanelCopyEntry;
  exportMacroDraftJson: () => void;
  importMacroJson: (file: File | undefined) => Promise<void>;
  insertMacroDraftFromLog: () => void;
  recordingMode: boolean;
  scriptCode: string;
  setScriptCode: (value: string) => void;
};

type AuthorMode = "script" | "record";

export function WorkbenchScriptAuthorPanel({
  copy,
  exportMacroDraftJson,
  importMacroJson,
  insertMacroDraftFromLog,
  recordingMode,
  scriptCode,
  setScriptCode,
}: WorkbenchScriptAuthorPanelProps) {
  const [mode, setMode] = useState<AuthorMode>("script");

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{copy.author}</h2>
        <span>{mode === "script" ? "Pyodide" : copy.recordMode}</span>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${mode === "script" ? " panel-tab--active" : ""}`} onClick={() => setMode("script")} type="button">
          {copy.scriptMode}
        </button>
        <button className={`panel-tab${mode === "record" ? " panel-tab--active" : ""}`} onClick={() => setMode("record")} type="button">
          {copy.recordMode}
        </button>
      </div>
      {mode === "script" ? (
        <>
          <p className="card-copy">{copy.authorScriptHint}</p>
          <textarea
            className="script-panel__editor"
            rows={18}
            spellCheck={false}
            value={scriptCode}
            onChange={(event) => setScriptCode(event.target.value)}
          />
        </>
      ) : (
        <>
          <p className="card-copy">{copy.authorRecordHint}</p>
          <div className="button-row">
            <button className="ghost-button" onClick={insertMacroDraftFromLog} type="button">
              {copy.insertMacroDraft}
            </button>
            <button className="ghost-button" onClick={exportMacroDraftJson} type="button">
              {copy.exportMacroJson}
            </button>
            <label className="ghost-button" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
              {copy.importMacroJson}
              <input
                accept="application/json,.json"
                hidden
                onChange={(event) => {
                  void importMacroJson(event.target.files?.[0]);
                  event.currentTarget.value = "";
                }}
                type="file"
              />
            </label>
          </div>
          {recordingMode ? <p className="card-copy">{copy.recordingActive}</p> : null}
        </>
      )}
    </section>
  );
}
