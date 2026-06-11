"use client";

import { useEffect, useState } from "react";
import { WorkbenchHeadlessWorkflowPanel } from "@/components/workbench/workbench-headless-workflow-panel";
import type { FrontendMacroAssetRecord } from "@/components/workbench/workbench-headless-workflow-panel";
import { WorkbenchScriptAuthorPanel } from "@/components/workbench/workbench-script-author-panel";
import { WorkbenchScriptCatalogPanel } from "@/components/workbench/workbench-script-catalog-panel";
import { WorkbenchScriptInspectPanel } from "@/components/workbench/workbench-script-inspect-panel";
import { WorkbenchScriptLaunchCard } from "@/components/workbench/workbench-script-launch-card";
import { workbenchScriptPanelCopy, type WorkbenchScriptPanelCopyEntry } from "@/components/workbench/workbench-script-panel-copy";
import {
  buildWorkbenchRecordedMacroDraft,
  buildWorkbenchRecordedMacroDraftFromEntries,
  buildWorkbenchPythonPrelude,
  DEFAULT_WORKBENCH_PYTHON,
  deleteWorkbenchMacroPreset,
  ensurePyodideRuntime,
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

function sanitizePythonComment(value: string | undefined): string | null {
  if (!value) return null;
  const collapsed = value.replace(/\s+/g, " ").trim();
  return collapsed ? collapsed.replaceAll("#", "") : null;
}

function buildTimelineReplaySnippet(entry: WorkbenchScriptActionLogEntry): string {
  const failureReason =
    entry.status !== "failed"
      ? null
      : sanitizePythonComment(entry.note ?? (entry.result ? JSON.stringify(entry.result) : entry.payload ? JSON.stringify(entry.payload) : entry.summary));
  const commentLines = [
    "# Replay snippet from timeline",
    `# action: ${entry.action}`,
    `# source: ${entry.source ?? "unknown"}`,
    `# status: ${entry.status}`,
    `# at: ${entry.at}`,
    sanitizePythonComment(entry.summary) ? `# summary: ${sanitizePythonComment(entry.summary)}` : null,
    sanitizePythonComment(entry.note) ? `# note: ${sanitizePythonComment(entry.note)}` : null,
    failureReason ? `# last failure: ${failureReason}` : null,
  ].filter(Boolean);
  const actionLiteral = JSON.stringify(entry.action);
  const payloadLiteral = stringifyPayload(entry.payload);

  return `${commentLines.join("\n")}
replay_payload = ${payloadLiteral}
replay_result = await ky.invoke(${actionLiteral}, replay_payload)
ky.log("Replay result:", replay_result)
latest_state = ky.state()
ky.log("Replay message:", latest_state.get("message"))
# await ky.sleep(0.25)  # Un-comment if the next UI action needs a short settle window.
`;
}

function buildTimelineContinuationSnippet(actionLog: WorkbenchScriptActionLogEntry[], entry: WorkbenchScriptActionLogEntry): string {
  const entryIndex = actionLog.findIndex((candidate) => candidate.id === entry.id);
  const replayEntries = (entryIndex >= 0 ? actionLog.slice(0, entryIndex + 1) : [entry]).reverse();
  const header = [
    "# Continue timeline from selected action",
    `# start action: ${entry.action}`,
    `# steps: ${replayEntries.length}`,
    "# Re-run the recorded flow in chronological order.",
  ].join("\n");
  const body = replayEntries
    .map((step, index) => {
      const actionLiteral = JSON.stringify(step.action);
      const payloadLiteral = stringifyPayload(step.payload);
      const summary = sanitizePythonComment(step.summary);
      const note = sanitizePythonComment(step.note);

      return [
        `# step ${index + 1}: ${step.action}`,
        summary ? `# summary: ${summary}` : null,
        note ? `# note: ${note}` : null,
        `step_${index + 1}_payload = ${payloadLiteral}`,
        `step_${index + 1}_result = await ky.invoke(${actionLiteral}, step_${index + 1}_payload)`,
        `replay_results.append(step_${index + 1}_result)`,
        `ky.log("Step ${index + 1} result:", step_${index + 1}_result)`,
        "# await ky.sleep(0.25)",
      ]
        .filter(Boolean)
        .join("\n");
    })
    .join("\n\n");

  return `${header}
replay_results = []

${body}

latest_state = ky.state()
ky.log("Continuation message:", latest_state.get("message"))
`;
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

function buildTimelinePresetName(entry: WorkbenchScriptActionLogEntry): string {
  const actionName = entry.action.replaceAll("/", " ");
  const timestamp = entry.at.replace("T", " ").slice(0, 16);
  return `${actionName} ${timestamp}`.trim();
}

function buildFrontendMacroAssetId(source: FrontendMacroAssetRecord["source"]) {
  return `asset_${source}_${Math.random().toString(36).slice(2, 10)}`;
}

function buildDerivedMacroDraftId(baseId: string) {
  const normalized = (baseId.startsWith("macro/") ? baseId.slice("macro/".length) : baseId)
    .replace(/-derived-[a-z0-9]+$/i, "")
    .replace(/[^a-zA-Z0-9/_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^[-/]+|[-/]+$/g, "");
  const suffix = Date.now().toString(36).slice(-6);
  return `macro/${normalized || "frontend-subflow"}-derived-${suffix}`;
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
  }, [snapshot.selectedProjectId]);

  const appendOutput = (line: string) => {
    setOutput((current) => [...current.slice(-199), line]);
  };

  const refreshPresetRecords = () => {
    setPresetRecords(listWorkbenchMacroPresets(snapshot.selectedProjectId));
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
        insertAction={insertAction}
        insertMacro={insertMacro}
        insertPreset={insertPreset}
        language={language}
        macros={WORKBENCH_SCRIPT_MACROS}
        presetName={presetName}
        presetRecords={presetRecords}
        saveCurrentPreset={saveCurrentPreset}
        selectedProjectId={snapshot.selectedProjectId}
        setPresetName={setPresetName}
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
