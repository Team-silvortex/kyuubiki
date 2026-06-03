"use client";

import { useEffect, useState } from "react";
import { workbenchScriptPanelCopy } from "@/components/workbench/workbench-script-panel-copy";
import {
  buildWorkbenchRecordedMacroDraft,
  buildWorkbenchPythonPrelude,
  DEFAULT_WORKBENCH_PYTHON,
  deleteWorkbenchMacroPreset,
  ensurePyodideRuntime,
  isWorkbenchScriptActionHighRisk,
  listWorkbenchMacroPresets,
  parseWorkbenchRecordedMacroDraft,
  saveWorkbenchMacroPreset,
  serializeWorkbenchRecordedMacroDraft,
  serializeWorkbenchMacroPythonSnippet,
  WORKBENCH_SCRIPT_MACROS,
  WORKBENCH_SCRIPT_ACTIONS,
  type WorkbenchScriptActionDefinition,
  type WorkbenchScriptActionLogEntry,
  type WorkbenchScriptLanguage,
  type WorkbenchMacroPresetRecord,
  type WorkbenchScriptSnapshot,
} from "@/lib/scripting/workbench-script-runtime";

type WorkbenchScriptPanelProps = {
  language: WorkbenchScriptLanguage;
  snapshot: WorkbenchScriptSnapshot;
  getSnapshot: () => WorkbenchScriptSnapshot;
  actionLog: WorkbenchScriptActionLogEntry[];
  recordingMode: boolean;
  onToggleRecordingMode: () => void;
  onInvokeAction: (action: string, payload?: Record<string, unknown>) => Promise<unknown>;
};

type RuntimeStatus = "idle" | "loading" | "ready" | "running" | "error";

const STORAGE_KEY = "kyuubiki-workbench-python-panel";

function safeStorageGet() {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as { code?: string }) : null;
  } catch {
    return null;
  }
}

function stringifyPayload(payload: Record<string, unknown> | undefined): string {
  return payload ? JSON.stringify(payload) : "{}";
}

function downloadTextFile(filename: string, contents: string) {
  const blob = new Blob([contents], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function WorkbenchScriptPanel({ language, snapshot, getSnapshot, actionLog, recordingMode, onToggleRecordingMode, onInvokeAction }: WorkbenchScriptPanelProps) {
  const t = workbenchScriptPanelCopy[language];
  const [scriptCode, setScriptCode] = useState(DEFAULT_WORKBENCH_PYTHON);
  const [output, setOutput] = useState<string[]>([]);
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus>("idle");
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [presetName, setPresetName] = useState("");
  const [presetRecords, setPresetRecords] = useState<WorkbenchMacroPresetRecord[]>([]);
  const [macroDraftBuffer, setMacroDraftBuffer] = useState<ReturnType<typeof buildWorkbenchRecordedMacroDraft> | null>(null);

  useEffect(() => {
    const stored = safeStorageGet();
    if (stored?.code) {
      setScriptCode(stored.code);
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        code: scriptCode,
      }),
    );
  }, [scriptCode]);

  useEffect(() => {
    setPresetRecords(listWorkbenchMacroPresets(snapshot.selectedProjectId));
  }, [snapshot.selectedProjectId]);

  const appendOutput = (line: string) => {
    setOutput((current) => [...current.slice(-199), line]);
  };

  const refreshPresetRecords = () => {
    setPresetRecords(listWorkbenchMacroPresets(snapshot.selectedProjectId));
  };

  const resolveCurrentDraft = () => macroDraftBuffer ?? buildWorkbenchRecordedMacroDraft(actionLog);

  const loadRuntime = async () => {
    setRuntimeError(null);
    setRuntimeStatus("loading");

    try {
      await ensurePyodideRuntime();
      setRuntimeStatus("ready");
      appendOutput(`[runtime] ${t.ready}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setRuntimeError(message);
      setRuntimeStatus("error");
      appendOutput(`[runtime] ${message}`);
    }
  };

  const runScript = async () => {
    setRuntimeError(null);
    setRuntimeStatus("running");

    try {
      const pyodide = await ensurePyodideRuntime();
      const bridge = {
        invoke: async (action: string, payloadJson?: string) => {
          const payload =
            payloadJson && payloadJson.trim().length > 0
              ? (JSON.parse(payloadJson) as Record<string, unknown>)
              : {};
          const result = await onInvokeAction(action, payload);
          return JSON.stringify(result ?? { ok: true, action });
        },
        state_json: () => JSON.stringify(getSnapshot()),
        actions_json: () => JSON.stringify(WORKBENCH_SCRIPT_ACTIONS),
        macros_json: () => JSON.stringify(WORKBENCH_SCRIPT_MACROS),
        log: (message: string) => appendOutput(message),
        sleep: async (seconds = 0) =>
          new Promise<void>((resolve) => {
            window.setTimeout(resolve, Math.max(0, seconds) * 1000);
          }),
      };

      window.__kyuubikiBridge = bridge;
      appendOutput(`[script] ${t.running}`);
      await pyodide.runPythonAsync(`${buildWorkbenchPythonPrelude()}\n${scriptCode}`);
      setRuntimeStatus("ready");
      appendOutput(`[script] ${t.ready}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setRuntimeError(message);
      setRuntimeStatus("error");
      appendOutput(`[error] ${message}`);
    }
  };

  const insertAction = (action: WorkbenchScriptActionDefinition) => {
    const payload = stringifyPayload(action.payloadExample);
    setScriptCode((current) => `${current.trimEnd()}\n\nawait ky.invoke("${action.id}", ${payload})\n`);
  };

  const insertMacro = (macroId: string, payload?: Record<string, unknown>) => {
    setScriptCode((current) => `${current.trimEnd()}\n\nawait ky.run_macro("${macroId}", ${stringifyPayload(payload)})\n`);
  };

  const insertMacroDraftFromLog = () => {
    const draft = buildWorkbenchRecordedMacroDraft(actionLog);

    if (!draft) {
      appendOutput(`[macro] ${t.noMacroDraftSource}`);
      return;
    }

    setMacroDraftBuffer(draft);
    const snippet = serializeWorkbenchMacroPythonSnippet(draft);
    setScriptCode((current) => `${current.trimEnd()}\n\n${snippet}\n`);
    appendOutput(`[macro] ${t.macroDraftInserted}`);
  };

  const exportMacroDraftJson = () => {
    const draft = buildWorkbenchRecordedMacroDraft(actionLog);

    if (!draft) {
      appendOutput(`[macro] ${t.noMacroDraftSource}`);
      return;
    }

    setMacroDraftBuffer(draft);
    downloadTextFile(`${draft.id}.json`, serializeWorkbenchRecordedMacroDraft(draft));
    appendOutput(`[macro] ${t.macroJsonExported}`);
  };

  const importMacroJson = async (file: File | undefined) => {
    if (!file) return;

    try {
      const parsed = parseWorkbenchRecordedMacroDraft(JSON.parse(await file.text()) as unknown);
      setMacroDraftBuffer(parsed);
      const snippet = serializeWorkbenchMacroPythonSnippet(parsed);
      setScriptCode((current) => `${current.trimEnd()}\n\n${snippet}\n`);
      appendOutput(`[macro] ${t.macroJsonImported}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendOutput(`[macro] ${message}`);
      setRuntimeError(message);
    }
  };

  const saveCurrentPreset = () => {
    if (!snapshot.selectedProjectId) {
      appendOutput(`[preset] ${t.noProjectSelected}`);
      return;
    }

    const draft = resolveCurrentDraft();
    if (!draft) {
      appendOutput(`[preset] ${t.noPresetDraft}`);
      return;
    }

    try {
      const saved = saveWorkbenchMacroPreset({
        projectId: snapshot.selectedProjectId,
        name: presetName || draft.id.replace(/^macro\//, "").replaceAll("-", " "),
        macro: draft,
      });
      setMacroDraftBuffer(saved.macro);
      setPresetName(saved.name);
      refreshPresetRecords();
      appendOutput(`[preset] ${t.presetSaved}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendOutput(`[preset] ${message}`);
      setRuntimeError(message);
    }
  };

  const insertPreset = (preset: WorkbenchMacroPresetRecord) => {
    setMacroDraftBuffer(preset.macro);
    setPresetName(preset.name);
    const snippet = serializeWorkbenchMacroPythonSnippet(preset.macro);
    setScriptCode((current) => `${current.trimEnd()}\n\n${snippet}\n`);
    appendOutput(`[preset] ${t.presetInserted}`);
  };

  const exportPresetJson = (preset: WorkbenchMacroPresetRecord) => {
    setMacroDraftBuffer(preset.macro);
    downloadTextFile(`${preset.name.replace(/\s+/g, "-").toLowerCase() || preset.presetId}.json`, serializeWorkbenchRecordedMacroDraft(preset.macro));
    appendOutput(`[macro] ${t.macroJsonExported}`);
  };

  const deletePreset = (preset: WorkbenchMacroPresetRecord) => {
    deleteWorkbenchMacroPreset(preset.presetId);
    refreshPresetRecords();
    appendOutput(`[preset] ${t.presetDeleted}`);
  };

  return (
    <>
      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.title}</h2>
          <span className={`status-chip status-chip--${runtimeStatus === "error" ? "risk" : runtimeStatus === "ready" ? "good" : "watch"}`}>
            {runtimeStatus === "loading"
              ? t.loading
              : runtimeStatus === "ready"
                ? t.ready
                : runtimeStatus === "running"
                  ? t.running
                  : runtimeStatus === "error"
                    ? t.error
                    : t.idle}
          </span>
        </div>
        <p className="card-copy">{t.subtitle}</p>
        <p className="card-copy">{t.firstRun}</p>
        {runtimeError ? <p className="card-copy">{runtimeError}</p> : null}
        <div className="button-row">
          <button className="ghost-button" onClick={() => void loadRuntime()} type="button">
            {t.loadRuntime}
          </button>
          <button className="ghost-button" onClick={() => setScriptCode(DEFAULT_WORKBENCH_PYTHON)} type="button">
            {t.resetScript}
          </button>
          <button className="ghost-button" onClick={() => void runScript()} type="button">
            {t.runScript}
          </button>
          <button className={`ghost-button${recordingMode ? " ghost-button--active" : ""}`} onClick={onToggleRecordingMode} type="button">
            {recordingMode ? t.stopRecording : t.startRecording}
          </button>
          <button className="ghost-button" onClick={exportMacroDraftJson} type="button">
            {t.exportMacroJson}
          </button>
          <label className="ghost-button" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
            {t.importMacroJson}
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
          <button className="ghost-button" onClick={() => setOutput([])} type="button">
            {t.clearOutput}
          </button>
        </div>
        {recordingMode ? <p className="card-copy">{t.recordingActive}</p> : null}
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.projectPresets}</h2>
          <span>{presetRecords.length}</span>
        </div>
        {!snapshot.selectedProjectId ? <p className="card-copy">{t.noProjectSelected}</p> : null}
        <label className="field-label">
          <span>{t.presetName}</span>
          <input
            className="text-input"
            onChange={(event) => setPresetName(event.target.value)}
            placeholder={t.presetNamePlaceholder}
            type="text"
            value={presetName}
          />
        </label>
        <div className="button-row">
          <button className="ghost-button" disabled={!snapshot.selectedProjectId} onClick={saveCurrentPreset} type="button">
            {t.savePreset}
          </button>
        </div>
        {presetRecords.length === 0 ? (
          <p className="card-copy">{snapshot.selectedProjectId ? t.noPresets : t.noProjectSelected}</p>
        ) : (
          <div className="script-panel__catalog">
            {presetRecords.map((preset) => (
              <article className="script-panel__action" key={preset.presetId}>
                <div className="script-panel__action-head">
                  <strong>{preset.name}</strong>
                  <span>{preset.updatedAt}</span>
                </div>
                <div className="script-panel__payload">
                  <span>ID</span>
                  <code>{preset.macro.id}</code>
                </div>
                <div className="script-panel__payload">
                  <span>Steps</span>
                  <code>{String(preset.macro.steps.length)}</code>
                </div>
                <div className="button-row">
                  <button className="ghost-button ghost-button--compact" onClick={() => insertPreset(preset)} type="button">
                    {t.insertPreset}
                  </button>
                  <button className="ghost-button ghost-button--compact" onClick={() => exportPresetJson(preset)} type="button">
                    {t.exportPresetJson}
                  </button>
                  <button className="ghost-button ghost-button--compact" onClick={() => deletePreset(preset)} type="button">
                    {t.deletePreset}
                  </button>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.snapshot}</h2>
          <span>{snapshot.studyKind}</span>
        </div>
        <div className="sidebar-list">
          <div>
            <span>{t.stateKeys}</span>
            <strong>{Object.keys(snapshot).length}</strong>
          </div>
          <div>
            <span>{t.actionCount}</span>
            <strong>{WORKBENCH_SCRIPT_ACTIONS.length + WORKBENCH_SCRIPT_MACROS.length}</strong>
          </div>
          <div>
            <span>{t.lineCount}</span>
            <strong>{scriptCode.split("\n").length}</strong>
          </div>
        </div>
        <pre className="script-panel__snapshot">{JSON.stringify(snapshot, null, 2)}</pre>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.editor}</h2>
          <span>Pyodide</span>
        </div>
        <textarea
          className="script-panel__editor"
          rows={18}
          spellCheck={false}
          value={scriptCode}
          onChange={(event) => setScriptCode(event.target.value)}
        />
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.output}</h2>
          <span>{output.length}</span>
        </div>
        {output.length === 0 ? (
          <p className="card-copy">{t.noOutput}</p>
        ) : (
          <pre className="script-panel__output">{output.join("\n")}</pre>
        )}
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.actionLog}</h2>
          <span>{actionLog.length}</span>
        </div>
        <div className="button-row">
          <button className="ghost-button ghost-button--compact" onClick={insertMacroDraftFromLog} type="button">
            {t.insertMacroDraft}
          </button>
        </div>
        {actionLog.length === 0 ? (
          <p className="card-copy">{t.noActionLog}</p>
        ) : (
          <div className="script-panel__catalog">
            {actionLog.map((entry) => (
              <article className="script-panel__action" key={entry.id}>
                <div className="script-panel__action-head">
                  <strong>{entry.action}</strong>
                  <span>{entry.status}</span>
                </div>
                <p className="card-copy">{entry.summary}</p>
                {entry.source ? (
                  <div className="script-panel__payload">
                    <span>{t.actionSource}</span>
                    <code>{entry.source}</code>
                  </div>
                ) : null}
                {entry.payload ? (
                  <div className="script-panel__payload">
                    <span>{t.actionPayload}</span>
                    <code>{JSON.stringify(entry.payload)}</code>
                  </div>
                ) : null}
                {entry.result ? (
                  <div className="script-panel__payload">
                    <span>{t.actionResult}</span>
                    <code>{JSON.stringify(entry.result)}</code>
                  </div>
                ) : null}
                {entry.note ? (
                  <div className="script-panel__payload">
                    <span>{t.actionNote}</span>
                    <code>{entry.note}</code>
                  </div>
                ) : null}
                <div className="script-panel__payload">
                  <span>Time</span>
                  <code>{entry.at}</code>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.actionCatalog}</h2>
          <span>{WORKBENCH_SCRIPT_ACTIONS.length}</span>
        </div>
        <div className="script-panel__catalog">
          {WORKBENCH_SCRIPT_ACTIONS.map((action) => (
            <article className="script-panel__action" key={action.id}>
              <div className="script-panel__action-head">
                <strong>{action.id}</strong>
                <span>
                  {t.categories[action.category as keyof typeof t.categories] ?? action.category}
                  {action.risk === "destructive"
                    ? ` · ${t.riskDestructive}`
                    : action.risk === "sensitive"
                      ? ` · ${t.riskSensitive}`
                      : ` · ${t.riskNormal}`}
                </span>
              </div>
              <p className="card-copy">{language === "zh" ? action.summary.zh : action.summary.en}</p>
              {isWorkbenchScriptActionHighRisk(action.id) ? <p className="card-copy">{t.confirmationRequired}</p> : null}
              <div className="script-panel__payload">
                <span>{t.payload}</span>
                <code>{stringifyPayload(action.payloadExample)}</code>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => insertAction(action)} type="button">
                  {t.insertLabel}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.macroCatalog}</h2>
          <span>{WORKBENCH_SCRIPT_MACROS.length}</span>
        </div>
        <div className="script-panel__catalog">
          {WORKBENCH_SCRIPT_MACROS.map((macro) => (
            <article className="script-panel__action" key={macro.id}>
              <div className="script-panel__action-head">
                <strong>{macro.id}</strong>
                <span>
                  {t.categories[macro.category as keyof typeof t.categories] ?? macro.category}
                  {macro.risk === "destructive"
                    ? ` · ${t.riskDestructive}`
                    : macro.risk === "sensitive"
                      ? ` · ${t.riskSensitive}`
                      : ` · ${t.riskNormal}`}
                </span>
              </div>
              <p className="card-copy">{language === "zh" ? macro.summary.zh : macro.summary.en}</p>
              {macro.requiresConfirmation ? <p className="card-copy">{t.confirmationRequired}</p> : null}
              <div className="script-panel__payload">
                <span>{t.payload}</span>
                <code>{stringifyPayload(macro.payloadExample)}</code>
              </div>
              <div className="script-panel__payload">
                <span>{t.stepsLabel}</span>
                <code>{macro.steps.map((step) => step.action).join(" -> ")}</code>
              </div>
              {macro.payloadExample ? (
                <p className="card-copy">{t.macroPayloadHint}</p>
              ) : null}
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => insertMacro(macro.id, macro.payloadExample)} type="button">
                  {t.insertLabel}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}
