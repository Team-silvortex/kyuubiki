"use client";

import { useEffect, useState } from "react";
import { WorkbenchHeadlessWorkflowPanel } from "@/components/workbench/workbench-headless-workflow-panel";
import type { FrontendMacroAssetRecord } from "@/components/workbench/workbench-headless-workflow-panel";
import {
  buildDerivedMacroDraftId,
  buildFrontendMacroAssetId,
  buildTimelineContinuationSnippet,
  buildTimelinePresetName,
  buildTimelineReplaySnippet,
  downloadTextFile,
} from "@/components/workbench/workbench-script-panel-helpers";
import { WorkbenchScriptAuthorPanel } from "@/components/workbench/workbench-script-author-panel";
import { WorkbenchScriptCatalogPanel } from "@/components/workbench/workbench-script-catalog-panel";
import { WorkbenchScriptInspectPanel } from "@/components/workbench/workbench-script-inspect-panel";
import { WorkbenchScriptLaunchCard } from "@/components/workbench/workbench-script-launch-card";
import { workbenchScriptPanelCopy, type WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";
import {
  buildWorkbenchRecordedMacroDraft,
  buildWorkbenchRecordedMacroDraftFromEntries,
  buildWorkbenchUiAutomationContractSnapshot,
  buildWorkbenchPythonPrelude,
  DEFAULT_WORKBENCH_PYTHON,
  deleteWorkbenchMacroPreset,
  deleteWorkbenchSnippetPreset,
  ensurePyodideRuntime,
  listWorkbenchMacroPresets,
  listWorkbenchSnippetPresets,
  parseWorkbenchRecordedMacroDraft,
  parseWorkbenchSnippetPresetRecord,
  renderWorkbenchScriptSnippet,
  saveWorkbenchMacroPreset,
  saveWorkbenchSnippetPreset,
  serializeWorkbenchSnippetPresetRecord,
  serializeWorkbenchRecordedMacroDraft,
  serializeWorkbenchMacroPythonSnippet,
  WORKBENCH_SCRIPT_MACROS,
  WORKBENCH_SCRIPT_SNIPPETS,
  WORKBENCH_SCRIPT_ACTIONS,
  type WorkbenchScriptActionDefinition,
  type WorkbenchScriptActionLogEntry,
  type WorkbenchScriptLanguage,
  type WorkbenchMacroPresetRecord,
  type WorkbenchScriptSnippetDefinition,
  type WorkbenchScriptSnippetParameters,
  type WorkbenchScriptSnippetPresetRecord,
  type WorkbenchScriptSnapshot,
} from "@/lib/scripting/workbench-script-runtime";
import { serializeWorkbenchPythonLiteral } from "@/lib/scripting/workbench-script-python-format";

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
  return serializeWorkbenchPythonLiteral(payload ?? {});
}


export function WorkbenchScriptPanel({ language, snapshot, getSnapshot, actionLog, recordingMode, onToggleRecordingMode, onInvokeAction }: WorkbenchScriptPanelProps) {
  const t = workbenchScriptPanelCopy[language] as WorkbenchScriptPanelCopyEntry;
  const [headlessFrontendMacroAssets, setHeadlessFrontendMacroAssets] = useState<FrontendMacroAssetRecord[]>([]);
  const [scriptCode, setScriptCode] = useState(DEFAULT_WORKBENCH_PYTHON);
  const [output, setOutput] = useState<string[]>([]);
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus>("idle");
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [presetName, setPresetName] = useState("");
  const [presetRecords, setPresetRecords] = useState<WorkbenchMacroPresetRecord[]>([]);
  const [snippetPresetRecords, setSnippetPresetRecords] = useState<WorkbenchScriptSnippetPresetRecord[]>([]);
  const [macroDraftBuffer, setMacroDraftBuffer] = useState<ReturnType<typeof buildWorkbenchRecordedMacroDraft> | null>(null);

  useEffect(() => {
    const stored = safeStorageGet();
    if (stored?.code) setScriptCode(stored.code);
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ code: scriptCode }));
  }, [scriptCode]);

  useEffect(() => {
    setPresetRecords(listWorkbenchMacroPresets(snapshot.selectedProjectId));
    setSnippetPresetRecords(listWorkbenchSnippetPresets(snapshot.selectedProjectId));
  }, [snapshot.selectedProjectId]);

  const appendOutput = (line: string) => {
    setOutput((current) => [...current.slice(-199), line]);
  };

  const refreshPresetRecords = () => {
    setPresetRecords(listWorkbenchMacroPresets(snapshot.selectedProjectId));
  };
  const refreshSnippetPresetRecords = () => {
    setSnippetPresetRecords(listWorkbenchSnippetPresets(snapshot.selectedProjectId));
  };
  const pushHeadlessFrontendMacroAsset = (draft: ReturnType<typeof parseWorkbenchRecordedMacroDraft>, source: FrontendMacroAssetRecord["source"]) => {
    const nextRecord: FrontendMacroAssetRecord = {
      assetId: buildFrontendMacroAssetId(source),
      draft,
      source,
      updatedAt: new Date().toISOString(),
    };
    setHeadlessFrontendMacroAssets((current) => [nextRecord, ...current].slice(0, 16));
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
      window.__kyuubikiBridge = {
        invoke: async (action: string, payloadJson?: string) => {
          const payload = payloadJson && payloadJson.trim().length > 0 ? (JSON.parse(payloadJson) as Record<string, unknown>) : {};
          const result = await onInvokeAction(action, payload);
          return JSON.stringify(result ?? { ok: true, action });
        },
        state_json: () => JSON.stringify(getSnapshot()),
        actions_json: () => JSON.stringify(WORKBENCH_SCRIPT_ACTIONS),
        macros_json: () => JSON.stringify(WORKBENCH_SCRIPT_MACROS),
        ui_contract_json: () => JSON.stringify(buildWorkbenchUiAutomationContractSnapshot()),
        log: (message: string) => appendOutput(message),
        sleep: async (seconds = 0) => new Promise<void>((resolve) => window.setTimeout(resolve, Math.max(0, seconds) * 1000)),
      };
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
    setScriptCode((current) => `${current.trimEnd()}\n\nawait ky.invoke("${action.id}", ${stringifyPayload(action.payloadExample)})\n`);
  };

  const insertMacro = (macroId: string, payload?: Record<string, unknown>) => {
    setScriptCode((current) => `${current.trimEnd()}\n\nawait ky.run_macro("${macroId}", ${stringifyPayload(payload)})\n`);
  };

  const insertSnippet = (snippet: WorkbenchScriptSnippetDefinition, parameters?: WorkbenchScriptSnippetParameters) => {
    setScriptCode((current) => `${current.trimEnd()}\n\n${renderWorkbenchScriptSnippet(snippet, parameters)}`);
    appendOutput(`[snippet] ${snippet.id}`);
  };

  const saveSnippetPreset = (snippet: WorkbenchScriptSnippetDefinition, parameters: WorkbenchScriptSnippetParameters) => {
    if (!snapshot.selectedProjectId) {
      appendOutput(`[preset] ${t.noProjectSelected}`);
      return;
    }
    try {
      const timestamp = new Date().toISOString().replace("T", " ").slice(0, 16);
      saveWorkbenchSnippetPreset({
        projectId: snapshot.selectedProjectId,
        snippetId: snippet.id,
        name: `${snippet.id.replace(/^snippet\//, "")} ${timestamp}`,
        parameters,
      });
      refreshSnippetPresetRecords();
      appendOutput(`[preset] Saved snippet preset for ${snippet.id}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendOutput(`[preset] ${message}`);
      setRuntimeError(message);
    }
  };

  const insertSnippetPreset = (preset: WorkbenchScriptSnippetPresetRecord) => {
    const snippet = WORKBENCH_SCRIPT_SNIPPETS.find((entry) => entry.id === preset.snippetId);
    if (!snippet) {
      appendOutput(`[preset] Missing snippet definition for ${preset.snippetId}`);
      return;
    }
    insertSnippet(snippet, preset.parameters);
  };

  const deleteSnippetPreset = (preset: WorkbenchScriptSnippetPresetRecord) => {
    deleteWorkbenchSnippetPreset(preset.presetId);
    refreshSnippetPresetRecords();
    appendOutput(`[preset] Deleted snippet preset ${preset.name}`);
  };

  const exportSnippetPresetJson = (preset: WorkbenchScriptSnippetPresetRecord) => {
    downloadTextFile(`${preset.name.replace(/\s+/g, "-").toLowerCase() || preset.presetId}.json`, serializeWorkbenchSnippetPresetRecord(preset));
    appendOutput(`[preset] Exported snippet preset ${preset.name}`);
  };

  const importSnippetPresetJson = async (snippet: WorkbenchScriptSnippetDefinition, file: File | undefined) => {
    if (!file) return;
    if (!snapshot.selectedProjectId) {
      appendOutput(`[preset] ${t.noProjectSelected}`);
      return;
    }
    try {
      const parsed = parseWorkbenchSnippetPresetRecord(JSON.parse(await file.text()) as unknown);
      if (parsed.snippetId !== snippet.id) {
        throw new Error(`Snippet preset targets ${parsed.snippetId}, not ${snippet.id}.`);
      }
      saveWorkbenchSnippetPreset({
        projectId: snapshot.selectedProjectId,
        snippetId: parsed.snippetId,
        name: parsed.name,
        parameters: parsed.parameters,
      });
      refreshSnippetPresetRecords();
      appendOutput(`[preset] Imported snippet preset for ${snippet.id}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendOutput(`[preset] ${message}`);
      setRuntimeError(message);
    }
  };

  const insertMacroDraftFromLog = () => {
    const draft = buildWorkbenchRecordedMacroDraft(actionLog);
    if (!draft) {
      appendOutput(`[macro] ${t.noMacroDraftSource}`);
      return;
    }
    setMacroDraftBuffer(draft);
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(draft)}\n`);
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
      setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(parsed)}\n`);
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
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(preset.macro)}\n`);
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

  const insertExternalMacroDraft = (draft: ReturnType<typeof parseWorkbenchRecordedMacroDraft>) => {
    setMacroDraftBuffer(draft);
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(draft)}\n`);
    appendOutput(`[macro] ${t.macroDraftInserted}`);
  };
  const insertFrontendMacroAsset = (asset: FrontendMacroAssetRecord) => {
    setMacroDraftBuffer(asset.draft);
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(asset.draft)}\n`);
    appendOutput(`[macro] ${t.frontendMacroAssetInserted}`);
  };
  const deriveFrontendMacroAsset = (asset: FrontendMacroAssetRecord) => {
    const derivedDraft = {
      id: buildDerivedMacroDraftId(asset.draft.id),
      steps: asset.draft.steps.map((step) => ({
        action: step.action,
        ...(step.payload ? { payload: { ...step.payload } } : {}),
      })),
    };
    setMacroDraftBuffer(derivedDraft);
    pushHeadlessFrontendMacroAsset(derivedDraft, "snapshot_derived");
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(derivedDraft)}\n`);
    appendOutput(`[macro] ${t.frontendMacroAssetDerived}`);
  };
  const restoreBridgeMacroToFrontend = (draft: ReturnType<typeof parseWorkbenchRecordedMacroDraft>) => {
    setMacroDraftBuffer(draft);
    pushHeadlessFrontendMacroAsset(draft, "bridge_restore");
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(draft)}\n`);
    appendOutput(`[macro] ${t.restoreBridgeMacroToFrontend}`);
  };

  const insertTimelineStep = (entry: WorkbenchScriptActionLogEntry) => {
    setScriptCode((current) => `${current.trimEnd()}\n\n${buildTimelineReplaySnippet(entry)}\n`);
    appendOutput(`[macro] ${t.insertActionStep}`);
  };

  const continueTimelineFromEntry = (entry: WorkbenchScriptActionLogEntry) => {
    setScriptCode((current) => `${current.trimEnd()}\n\n${buildTimelineContinuationSnippet(actionLog, entry)}\n`);
    appendOutput(`[macro] ${t.continueFromStep}`);
  };

  const insertTimelineMacroDraft = (entry: WorkbenchScriptActionLogEntry, includedEntryIds?: string[]) => {
    const draft = buildWorkbenchRecordedMacroDraftFromEntries(actionLog, {
      includedEntryIds,
      id: `macro/${entry.action.replaceAll("/", "-")}-selection`,
      maxSteps: 12,
      startEntryId: entry.id,
    });
    if (!draft) {
      appendOutput(`[macro] ${t.noTimelineMacroDraft}`);
      return;
    }
    setMacroDraftBuffer(draft);
    setScriptCode((current) => `${current.trimEnd()}\n\n${serializeWorkbenchMacroPythonSnippet(draft)}\n`);
    appendOutput(`[macro] ${t.timelineMacroDraftInserted}`);
  };

  const saveTimelineMacroPreset = (entry: WorkbenchScriptActionLogEntry, includedEntryIds?: string[]) => {
    if (!snapshot.selectedProjectId) {
      appendOutput(`[preset] ${t.noProjectSelected}`);
      return;
    }
    const draft = buildWorkbenchRecordedMacroDraftFromEntries(actionLog, {
      includedEntryIds,
      id: `macro/${entry.action.replaceAll("/", "-")}-selection`,
      maxSteps: 12,
      startEntryId: entry.id,
    });
    if (!draft) {
      appendOutput(`[preset] ${t.noTimelineMacroDraft}`);
      return;
    }
    try {
      const nextPresetName = buildTimelinePresetName(entry);
      const saved = saveWorkbenchMacroPreset({
        projectId: snapshot.selectedProjectId,
        name: nextPresetName,
        macro: draft,
      });
      setMacroDraftBuffer(saved.macro);
      setPresetName(saved.name);
      refreshPresetRecords();
      appendOutput(`[preset] ${t.timelineMacroPresetSaved}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      appendOutput(`[preset] ${message}`);
      setRuntimeError(message);
    }
  };

  const sendTimelineMacroToHeadless = (entry: WorkbenchScriptActionLogEntry, includedEntryIds?: string[]) => {
    const draft = buildWorkbenchRecordedMacroDraftFromEntries(actionLog, {
      includedEntryIds,
      id: `macro/${entry.action.replaceAll("/", "-")}-frontend-subflow`,
      maxSteps: 12,
      startEntryId: entry.id,
    });
    if (!draft) {
      appendOutput(`[macro] ${t.noTimelineMacroDraft}`);
      return;
    }
    setMacroDraftBuffer(draft);
    pushHeadlessFrontendMacroAsset(draft, "timeline_selection");
    appendOutput(`[macro] ${t.sendTimelineMacroToHeadless}`);
  };

  return (
    <>
      <WorkbenchScriptLaunchCard
        clearOutput={() => setOutput([])}
        copy={t}
        loadRuntime={() => void loadRuntime()}
        recordingMode={recordingMode}
        resetScript={() => setScriptCode(DEFAULT_WORKBENCH_PYTHON)}
        runScript={() => void runScript()}
        runtimeError={runtimeError}
        runtimeStatus={runtimeStatus}
        toggleRecordingMode={onToggleRecordingMode}
      />

      <WorkbenchScriptAuthorPanel
        copy={t}
        deriveFrontendMacroAsset={deriveFrontendMacroAsset}
        exportMacroDraftJson={exportMacroDraftJson}
        frontendMacroAssets={headlessFrontendMacroAssets}
        importMacroJson={importMacroJson}
        insertFrontendMacroAsset={insertFrontendMacroAsset}
        insertMacroDraftFromLog={insertMacroDraftFromLog}
        recordingMode={recordingMode}
        scriptCode={scriptCode}
        setScriptCode={setScriptCode}
      />

      <WorkbenchScriptInspectPanel
        actionCatalogCount={WORKBENCH_SCRIPT_ACTIONS.length}
        actionLog={actionLog}
        continueTimelineFromEntry={continueTimelineFromEntry}
        copy={t}
        insertTimelineStep={insertTimelineStep}
        insertTimelineMacroDraft={insertTimelineMacroDraft}
        sendTimelineMacroToHeadless={sendTimelineMacroToHeadless}
        saveTimelineMacroPreset={saveTimelineMacroPreset}
        macroCatalogCount={WORKBENCH_SCRIPT_MACROS.length}
        output={output}
        scriptCode={scriptCode}
        snapshot={snapshot}
      />

      <WorkbenchScriptCatalogPanel
        actions={WORKBENCH_SCRIPT_ACTIONS}
        copy={t}
        deletePreset={deletePreset}
        exportPresetJson={exportPresetJson}
        exportSnippetPresetJson={exportSnippetPresetJson}
        importSnippetPresetJson={importSnippetPresetJson}
        insertAction={insertAction}
        insertMacro={insertMacro}
        insertPreset={insertPreset}
        insertSnippetPreset={insertSnippetPreset}
        insertSnippet={insertSnippet}
        language={language}
        macros={WORKBENCH_SCRIPT_MACROS}
        deleteSnippetPreset={deleteSnippetPreset}
        presetName={presetName}
        presetRecords={presetRecords}
        saveSnippetPreset={saveSnippetPreset}
        saveCurrentPreset={saveCurrentPreset}
        selectedProjectId={snapshot.selectedProjectId}
        setPresetName={setPresetName}
        snippets={WORKBENCH_SCRIPT_SNIPPETS}
        snippetPresetRecords={snippetPresetRecords}
      />

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.headlessSurface}</h2>
          <span>SDK</span>
        </div>
        <p className="card-copy">{t.headlessSurfaceHint}</p>
      </section>

      <WorkbenchHeadlessWorkflowPanel
        frontendMacroAssets={headlessFrontendMacroAssets}
        language={language}
        onDeriveFrontendMacro={deriveFrontendMacroAsset}
        onInsertMacroDraft={insertExternalMacroDraft}
        onRestoreFrontendMacro={restoreBridgeMacroToFrontend}
      />
    </>
  );
}
