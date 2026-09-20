"use client";
import { lazy, Suspense, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { workbenchScriptSession } from "@/lib/scripting/workbench-script-session";
import { WorkbenchAlertStrip } from "@/components/workbench/workbench-alert-strip";
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
import { WorkbenchScriptDslCard } from "@/components/workbench/workbench-script-dsl-card";
import { WorkbenchScriptWorkspace, type WorkbenchScriptPage } from "./workbench-script-workspace";
import { WorkbenchScriptLaunchCard } from "@/components/workbench/workbench-script-launch-card";
import { executeWorkbenchPythonSource } from "@/components/workbench/workbench-script-panel-runtime";
import {
  safeWorkbenchPanelStorageGetResult,
  writeWorkbenchPanelStorage,
} from "@/components/workbench/workbench-script-panel-storage";
import { getWorkbenchScriptPanelCopy } from "@/components/workbench/workbench-script-panel-copy";
import {
  buildWorkbenchRecordedMacroDraft,
  buildWorkbenchRecordedMacroDraftFromEntries,
  buildWorkbenchUiAutomationContractSnapshot,
  buildWorkbenchFrontendDslFromRecipe,
  buildWorkbenchPythonPrelude,
  buildWorkbenchFrontendDslFromMacroDraft,
  compileWorkbenchFrontendDslToPython,
  CLOSED_LOOP_TRUSS_WORKBENCH_FRONTEND_DSL,
  DEFAULT_WORKBENCH_FRONTEND_DSL,
  DEFAULT_WORKBENCH_PYTHON,
  deleteWorkbenchMacroPreset,
  deleteWorkbenchSnippetPreset,
  listWorkbenchMacroPresets,
  listWorkbenchSnippetPresets,
  parseWorkbenchMacroImportDocument,
  parseWorkbenchFrontendDslDocument,
  parseWorkbenchRecordedMacroDraft,
  parseWorkbenchSnippetPresetRecord,
  renderWorkbenchScriptSnippet,
  saveWorkbenchMacroPreset,
  saveWorkbenchSnippetPreset,
  serializeWorkbenchSnippetPresetRecord,
  serializeWorkbenchRecordedMacroDraft,
  serializeWorkbenchMacroPythonSnippet,
  WORKBENCH_SCRIPT_MACROS,
  WORKBENCH_SCRIPT_RECIPES,
  WORKBENCH_SCRIPT_SNIPPETS,
  WORKBENCH_SCRIPT_ACTIONS,
  WORKBENCH_FRONTEND_DSL_REPORT_PREFIX,
  type WorkbenchScriptActionDefinition,
  type WorkbenchScriptActionLogEntry,
  type WorkbenchScriptLanguage,
  type WorkbenchMacroPresetRecord,
  type WorkbenchScriptRecipeDefinition,
  type WorkbenchScriptSnippetDefinition,
  type WorkbenchScriptSnippetParameters,
  type WorkbenchScriptSnippetPresetRecord,
  type WorkbenchScriptSnapshot,
} from "@/lib/scripting/workbench-script-runtime";
import {
  describeSensitivePresetSaveError,
} from "@/lib/scripting/workbench-script-preset-security";
import { serializeWorkbenchPythonLiteral } from "@/lib/scripting/workbench-script-python-format";
const WorkbenchScriptCatalogPanel = lazy(() => import("./workbench-script-catalog-panel").then((module) => ({ default: module.WorkbenchScriptCatalogPanel })));
const WorkbenchScriptInspectPanel = lazy(() => import("./workbench-script-inspect-panel").then((module) => ({ default: module.WorkbenchScriptInspectPanel })));
const WorkbenchHeadlessWorkflowPanel = lazy(() => import("./workbench-headless-workflow-panel").then((module) => ({ default: module.WorkbenchHeadlessWorkflowPanel })));
type WorkbenchScriptPanelProps = {
  language: WorkbenchScriptLanguage;
  snapshot: WorkbenchScriptSnapshot;
  getSnapshot: () => WorkbenchScriptSnapshot;
  actionLog: WorkbenchScriptActionLogEntry[];
  recordingMode: boolean;
  onToggleRecordingMode: () => void;
  onInvokeAction: (action: string, payload?: Record<string, unknown>) => Promise<unknown>;
};
type PanelStorageStatus = "loading" | "ready" | "blocked";
const STORAGE_KEY = "kyuubiki-workbench-python-panel", DSL_STORAGE_KEY = "kyuubiki-workbench-dsl-panel";
function stringifyPayload(payload: Record<string, unknown> | undefined): string {
  return serializeWorkbenchPythonLiteral(payload ?? {});
}
export function WorkbenchScriptPanel({ language, snapshot, getSnapshot, actionLog, recordingMode, onToggleRecordingMode, onInvokeAction }: WorkbenchScriptPanelProps) {
  const t = getWorkbenchScriptPanelCopy(language);
  const [headlessFrontendMacroAssets, setHeadlessFrontendMacroAssets] = useState<FrontendMacroAssetRecord[]>([]);
  const [scriptCode, setScriptCode] = useState(DEFAULT_WORKBENCH_PYTHON);
  const [dslCode, setDslCode] = useState(DEFAULT_WORKBENCH_FRONTEND_DSL);
  const [activePage, setActivePage] = useState<WorkbenchScriptPage>("script");
  const { output, runtimeStatus, runtimeError } = useSyncExternalStore(
    workbenchScriptSession.subscribe, workbenchScriptSession.getSnapshot, workbenchScriptSession.getServerSnapshot,
  );
  const { setOutput, setRuntimeStatus, setRuntimeError } = workbenchScriptSession;
  const busy = runtimeStatus === "loading" || runtimeStatus === "running";
  const insertScript = (source: string) => {
    setScriptCode((current) => `${current.trimEnd()}\n\n${source.trimEnd()}\n`);
    setActivePage("script");
  };
  const [dslError, setDslError] = useState<string | null>(null);
  const [scriptStorageStatus, setScriptStorageStatus] = useState<PanelStorageStatus>("loading");
  const [dslStorageStatus, setDslStorageStatus] = useState<PanelStorageStatus>("loading");
  const storageBootstrappedRef = useRef(false);
  const [presetSecurityNotice, setPresetSecurityNotice] = useState<string | null>(null);
  const [presetName, setPresetName] = useState("");
  const [presetRecords, setPresetRecords] = useState<WorkbenchMacroPresetRecord[]>([]);
  const [snippetPresetRecords, setSnippetPresetRecords] = useState<WorkbenchScriptSnippetPresetRecord[]>([]);
  const [macroDraftBuffer, setMacroDraftBuffer] = useState<ReturnType<typeof buildWorkbenchRecordedMacroDraft> | null>(null);
  useEffect(() => {
    if (storageBootstrappedRef.current) return;
    storageBootstrappedRef.current = true;
    const stored = safeWorkbenchPanelStorageGetResult(STORAGE_KEY);
    const storedDsl = safeWorkbenchPanelStorageGetResult(DSL_STORAGE_KEY);
    if (stored.value) setScriptCode(stored.value.code);
    if (storedDsl.value) setDslCode(storedDsl.value.code);
    setScriptStorageStatus(stored.readable ? "ready" : "blocked");
    setDslStorageStatus(storedDsl.readable ? "ready" : "blocked");
    if (!stored.readable || !storedDsl.readable) setRuntimeError(t.sessionStorageFailed);
  }, [t.sessionStorageFailed]);
  useEffect(() => {
    if (scriptStorageStatus !== "ready") return;
    if (!writeWorkbenchPanelStorage(STORAGE_KEY, scriptCode)) {
      setRuntimeError(t.sessionStorageFailed);
    }
  }, [scriptCode, scriptStorageStatus, t.sessionStorageFailed]);
  useEffect(() => {
    if (dslStorageStatus !== "ready") return;
    if (!writeWorkbenchPanelStorage(DSL_STORAGE_KEY, dslCode)) {
      setRuntimeError(t.sessionStorageFailed);
    }
  }, [dslCode, dslStorageStatus, t.sessionStorageFailed]);
  useEffect(() => {
    setPresetRecords(listWorkbenchMacroPresets(snapshot.selectedProjectId));
    setSnippetPresetRecords(listWorkbenchSnippetPresets(snapshot.selectedProjectId));
  }, [snapshot.selectedProjectId]);
  const appendOutput = (line: string) => {
    setOutput((current) => [...current.slice(-199), line]);
  };
  const reportPresetSaveError = (error: unknown) => {
    const sensitive = describeSensitivePresetSaveError(error);
    if (sensitive) {
      const message = `${t.sensitivePresetRejected} ${t.sensitivePresetRejectedDetails} ${sensitive.details}`;
      appendOutput(`[preset] ${message}`);
      setRuntimeError(message);
      setPresetSecurityNotice(message);
      return;
    }
    const message = error instanceof Error ? error.message : String(error);
    appendOutput(`[preset] ${message}`);
    setRuntimeError(message);
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
    if (workbenchScriptSession.isBusy()) return;
    setRuntimeError(null);
    setRuntimeStatus("loading");
    try {
      await executeWorkbenchPythonSource({
        appendOutput,
        source: `${buildWorkbenchPythonPrelude()}\nky.log("Runtime loaded")`,
      });
      setRuntimeStatus("ready");
      appendOutput(`[runtime] ${t.ready}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setRuntimeError(message);
      setRuntimeStatus("error");
      appendOutput(`[runtime] ${message}`);
    }
  };
  const compileDslToScript = () => {
    try {
      const document = parseWorkbenchFrontendDslDocument(dslCode);
      const compiled = compileWorkbenchFrontendDslToPython(document);
      setDslError(null);
      setScriptCode((current) => `${current.trimEnd()}\n\n${compiled}\n`);
      appendOutput(`[dsl] compiled ${document.name}`);
      return compiled;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setDslError(message);
      setRuntimeError(message);
      appendOutput(`[dsl] ${message}`);
      return null;
    }
  };
  const runScript = async () => {
    if (workbenchScriptSession.isBusy()) return;
    setRuntimeError(null);
    setRuntimeStatus("running");
    try {
      appendOutput(`[script] ${t.running}`);
      await executeWorkbenchPythonSource({
        appendOutput,
        source: `${buildWorkbenchPythonPrelude()}\n${scriptCode}`,
      });
      setRuntimeStatus("ready");
      appendOutput(`[script] ${t.ready}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setRuntimeError(message);
      setRuntimeStatus("error");
      appendOutput(`[error] ${message}`);
    }
  };
  const runDsl = async () => {
    if (workbenchScriptSession.isBusy()) return;
    const compiled = compileDslToScript();
    if (!compiled) return;
    setRuntimeError(null);
    setRuntimeStatus("running");
    try {
      appendOutput("[dsl] running");
      await executeWorkbenchPythonSource({
        appendOutput,
        source: `${buildWorkbenchPythonPrelude()}\n${compiled}`,
      });
      setRuntimeStatus("ready");
      appendOutput("[dsl] ready");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const taggedCode = message.match(/\[dsl-code=([a-z_]+)\]/i)?.[1];
      const cleanMessage = message.replace(/^\[dsl-code=[^\]]+\]\s*/i, "");
      setRuntimeError(message);
      setRuntimeStatus("error");
      appendOutput(`${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} reported_at=${new Date().toISOString()} status=failed failure_code=${taggedCode ?? (/timeout/i.test(message) ? "timeout" : /selector/i.test(message) ? "selector_mismatch" : /state/i.test(message) ? "state_mismatch" : "runtime_exception")} failure_reason=${encodeURIComponent(cleanMessage)}`);
      appendOutput(`[dsl] ${message}`);
    }
  };
  const insertAction = (action: WorkbenchScriptActionDefinition) => {
    insertScript(`await ky.invoke("${action.id}", ${stringifyPayload(action.payloadExample)})`);
  };
  const insertMacro = (macroId: string, payload?: Record<string, unknown>) => {
    insertScript(`await ky.run_macro("${macroId}", ${stringifyPayload(payload)})`);
  };
  const insertSnippet = (snippet: WorkbenchScriptSnippetDefinition, parameters?: WorkbenchScriptSnippetParameters) => {
    insertScript(renderWorkbenchScriptSnippet(snippet, parameters));
    appendOutput(`[snippet] ${snippet.id}`);
  };
  const loadRecipeDsl = (recipe: WorkbenchScriptRecipeDefinition) => {
    const document = buildWorkbenchFrontendDslFromRecipe(recipe);
    setDslCode(JSON.stringify(document, null, 2));
    setDslError(null);
    setActivePage("dsl");
    appendOutput(`[recipe] ${recipe.id}`);
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
      setPresetSecurityNotice(null);
      refreshSnippetPresetRecords();
      appendOutput(`[preset] Saved snippet preset for ${snippet.id}`);
    } catch (error) {
      reportPresetSaveError(error);
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
      setPresetSecurityNotice(null);
      refreshSnippetPresetRecords();
      appendOutput(`[preset] Imported snippet preset for ${snippet.id}`);
    } catch (error) {
      reportPresetSaveError(error);
    }
  };
  const insertMacroDraftFromLog = () => {
    const draft = buildWorkbenchRecordedMacroDraft(actionLog);
    if (!draft) {
      appendOutput(`[macro] ${t.noMacroDraftSource}`);
      return;
    }
    setMacroDraftBuffer(draft);
    insertScript(serializeWorkbenchMacroPythonSnippet(draft));
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
      const imported = parseWorkbenchMacroImportDocument(JSON.parse(await file.text()) as unknown);
      setMacroDraftBuffer(imported.draft);
      insertScript(serializeWorkbenchMacroPythonSnippet(imported.draft));
      appendOutput(`[macro] ${imported.source === "headless-workflow" ? t.headlessWorkflowImported : t.macroJsonImported}`);
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
      setPresetSecurityNotice(null);
      setMacroDraftBuffer(saved.macro);
      setPresetName(saved.name);
      refreshPresetRecords();
      appendOutput(`[preset] ${t.presetSaved}`);
    } catch (error) {
      reportPresetSaveError(error);
    }
  };
  const insertPreset = (preset: WorkbenchMacroPresetRecord) => {
    setMacroDraftBuffer(preset.macro);
    setPresetName(preset.name);
    insertScript(serializeWorkbenchMacroPythonSnippet(preset.macro));
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
    insertScript(serializeWorkbenchMacroPythonSnippet(draft));
    appendOutput(`[macro] ${t.macroDraftInserted}`);
  };
  const useCurrentMacroDraftAsDsl = () => {
    const draft = resolveCurrentDraft();
    if (!draft) {
      appendOutput(`[dsl] ${t.noMacroDraftSource}`);
      return;
    }
    const document = buildWorkbenchFrontendDslFromMacroDraft(draft);
    setDslCode(JSON.stringify(document, null, 2));
    setDslError(null);
    setActivePage("dsl");
    appendOutput(`[dsl] ${document.name}`);
  };
  const insertFrontendMacroAsset = (asset: FrontendMacroAssetRecord) => {
    setMacroDraftBuffer(asset.draft);
    insertScript(serializeWorkbenchMacroPythonSnippet(asset.draft));
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
    insertScript(serializeWorkbenchMacroPythonSnippet(derivedDraft));
    appendOutput(`[macro] ${t.frontendMacroAssetDerived}`);
  };
  const restoreBridgeMacroToFrontend = (draft: ReturnType<typeof parseWorkbenchRecordedMacroDraft>) => {
    setMacroDraftBuffer(draft);
    pushHeadlessFrontendMacroAsset(draft, "bridge_restore");
    insertScript(serializeWorkbenchMacroPythonSnippet(draft));
    appendOutput(`[macro] ${t.restoreBridgeMacroToFrontend}`);
  };
  const insertTimelineStep = (entry: WorkbenchScriptActionLogEntry) => {
    insertScript(buildTimelineReplaySnippet(entry));
    appendOutput(`[macro] ${t.insertActionStep}`);
  };
  const continueTimelineFromEntry = (entry: WorkbenchScriptActionLogEntry) => {
    insertScript(buildTimelineContinuationSnippet(actionLog, entry));
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
    insertScript(serializeWorkbenchMacroPythonSnippet(draft));
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
      setPresetSecurityNotice(null);
      setMacroDraftBuffer(saved.macro);
      setPresetName(saved.name);
      refreshPresetRecords();
      appendOutput(`[preset] ${t.timelineMacroPresetSaved}`);
    } catch (error) {
      reportPresetSaveError(error);
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
    setActivePage("headless");
    appendOutput(`[macro] ${t.sendTimelineMacroToHeadless}`);
  };
  const authorProps = {
    copy: t, language, busy, onRunScript: () => void runScript(), onToggleRecording: onToggleRecordingMode,
    deriveFrontendMacroAsset, exportMacroDraftJson, frontendMacroAssets: headlessFrontendMacroAssets,
    importMacroJson, insertFrontendMacroAsset, insertMacroDraftFromLog, recordingMode, scriptCode, setScriptCode,
  };
  const outputCard = (
    <section className="sidebar-card sidebar-card--compact pwdt-output-card">
      <div className="card-head">
        <h2>{t.output}</h2>
        <button className="ghost-button ghost-button--compact" onClick={() => setOutput([])} type="button">{t.clearOutput}</button>
      </div>
      {output.length ? <pre className="script-panel__output">{output.join("\n")}</pre> : <p className="card-copy">{t.noOutput}</p>}
    </section>
  );
  const loading = <p className="card-copy" role="status">{t.loading}</p>;
  return (
    <WorkbenchScriptWorkspace language={language} activePage={activePage} onPageChange={setActivePage}
      help={<><p className="card-copy">{t.subtitle}</p><p className="card-copy">{t.frontendSurfaceHint}</p><p className="card-copy">{t.firstRun}</p></>}
      notice={<>
        <WorkbenchAlertStrip alerts={[
          ...(presetSecurityNotice ? [{ id: "preset-security-notice", message: presetSecurityNotice, tone: "warning" as const }] : []),
          ...(activePage !== "script" && runtimeError && runtimeError !== presetSecurityNotice
            ? [{ id: "script-runtime-error", message: runtimeError, tone: "error" as const }] : []),
        ]} />
        {recordingMode && activePage !== "record" ? <button className="ghost-button ghost-button--compact ghost-button--active"
          onClick={onToggleRecordingMode} title={t.recordingActive} type="button">{t.stopRecording}</button> : null}
      </>}
      pages={{
        script: (
          <div className="pwdt-workspace__python">
            <WorkbenchScriptLaunchCard copy={t} loadRuntime={() => void loadRuntime()}
              resetScript={() => setScriptCode(DEFAULT_WORKBENCH_PYTHON)} runScript={() => void runScript()}
              runtimeError={runtimeError === presetSecurityNotice ? null : runtimeError} runtimeStatus={runtimeStatus} />
            <WorkbenchScriptAuthorPanel {...authorProps} mode="script" />
            {outputCard}
          </div>
        ),
        dsl: (
          <div className="pwdt-workspace__dsl">
            <WorkbenchScriptDslCard dslCode={dslCode} dslError={dslError} language={language} busy={busy}
              onCompileDsl={() => { if (compileDslToScript()) setActivePage("script"); }}
              onLoadDslTemplate={() => { setDslCode(DEFAULT_WORKBENCH_FRONTEND_DSL); setDslError(null); }}
              onLoadRecipeTemplate={() => { setDslCode(CLOSED_LOOP_TRUSS_WORKBENCH_FRONTEND_DSL); setDslError(null); }}
              onRunDsl={() => void runDsl()} onUseCurrentMacroDraft={useCurrentMacroDraftAsDsl} setDslCode={setDslCode} />
            {outputCard}
          </div>
        ),
        record: <WorkbenchScriptAuthorPanel {...authorProps} mode="record" />,
        inspect: (
          <Suspense fallback={loading}>
            <WorkbenchScriptInspectPanel initialMode="timeline"
              actionCatalogCount={WORKBENCH_SCRIPT_ACTIONS.length} actionLog={actionLog}
              continueTimelineFromEntry={continueTimelineFromEntry} copy={t} insertTimelineStep={insertTimelineStep}
              insertTimelineMacroDraft={insertTimelineMacroDraft} sendTimelineMacroToHeadless={sendTimelineMacroToHeadless}
              saveTimelineMacroPreset={saveTimelineMacroPreset} macroCatalogCount={WORKBENCH_SCRIPT_MACROS.length}
              output={output} scriptCode={scriptCode} snapshot={snapshot} />
          </Suspense>
        ),
        catalog: (
          <Suspense fallback={loading}>
            <WorkbenchScriptCatalogPanel actions={WORKBENCH_SCRIPT_ACTIONS} copy={t} deletePreset={deletePreset}
              exportPresetJson={exportPresetJson} exportSnippetPresetJson={exportSnippetPresetJson}
              importSnippetPresetJson={importSnippetPresetJson} insertAction={insertAction} insertMacro={insertMacro}
              insertPreset={insertPreset} insertSnippetPreset={insertSnippetPreset} insertSnippet={insertSnippet}
              language={language} loadRecipeDsl={loadRecipeDsl} macros={WORKBENCH_SCRIPT_MACROS}
              deleteSnippetPreset={deleteSnippetPreset} presetName={presetName} presetRecords={presetRecords}
              saveSnippetPreset={saveSnippetPreset} saveCurrentPreset={saveCurrentPreset} selectedProjectId={snapshot.selectedProjectId}
              setPresetName={setPresetName} recipes={WORKBENCH_SCRIPT_RECIPES} snippets={WORKBENCH_SCRIPT_SNIPPETS}
              snippetPresetRecords={snippetPresetRecords} />
          </Suspense>
        ),
        headless: (
          <Suspense fallback={loading}>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head"><h2>{t.headlessSurface}</h2><span>SDK</span></div>
              <p className="card-copy">{t.headlessSurfaceHint}</p>
            </section>
            <WorkbenchHeadlessWorkflowPanel frontendMacroAssets={headlessFrontendMacroAssets} language={language}
              onDeriveFrontendMacro={deriveFrontendMacroAsset} onInsertMacroDraft={insertExternalMacroDraft}
              onRestoreFrontendMacro={restoreBridgeMacroToFrontend} />
          </Suspense>
        ),
      }} />
  );
}
