"use client";

import { useEffect, useState } from "react";
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

const copy = {
  en: {
    title: "WASM Python",
    subtitle: "Automate frontend workflows through a Pyodide bridge and the registered action catalog.",
    runtime: "Runtime",
    loading: "Loading",
    ready: "Ready",
    idle: "Idle",
    running: "Running",
    error: "Error",
    firstRun: "First run downloads the Pyodide runtime into the browser cache.",
    loadRuntime: "Load runtime",
    resetScript: "Reset template",
    runScript: "Run script",
    clearOutput: "Clear output",
    startRecording: "Start recording",
    stopRecording: "Stop recording",
    recordingActive: "Recording manual UI actions into the DSL event stream.",
    exportMacroJson: "Export macro JSON",
    importMacroJson: "Import macro JSON",
    macroJsonExported: "Exported the current recorded macro as JSON.",
    macroJsonImported: "Imported a macro JSON draft into the editor.",
    projectPresets: "Project presets",
    presetName: "Preset name",
    presetNamePlaceholder: "review current result context",
    savePreset: "Save preset",
    insertPreset: "Insert preset",
    exportPresetJson: "Export preset JSON",
    deletePreset: "Delete preset",
    noProjectSelected: "Select a project first to save presets against project context.",
    noPresetDraft: "Build or import a macro draft first so it can be saved as a preset.",
    noPresets: "No saved presets for the current project yet.",
    presetSaved: "Saved the current macro draft into project presets.",
    presetDeleted: "Deleted the selected project preset.",
    presetInserted: "Inserted the selected project preset into the editor.",
    snapshot: "Snapshot",
    actionCatalog: "Action catalog",
    macroCatalog: "Macro catalog",
    editor: "Python script",
    output: "Output",
    noOutput: "Script output will appear here.",
    lineCount: "Lines",
    stateKeys: "State fields",
    actionCount: "Actions",
    payload: "Payload",
    actionLog: "Action log",
    noActionLog: "No scripted actions yet.",
    insertMacroDraft: "Insert macro draft",
    macroDraftInserted: "Inserted a macro draft from recent action log.",
    noMacroDraftSource: "Run a few actions first so the panel can assemble a draft macro.",
    actionSource: "Source",
    actionPayload: "Payload",
    actionResult: "Result",
    actionNote: "Note",
    riskNormal: "Normal",
    riskSensitive: "Sensitive",
    riskDestructive: "Destructive",
    confirmationRequired: "Requires confirmation before execution.",
    categories: {
      navigation: "Navigation",
      settings: "Settings",
      runtime: "Runtime",
      project: "Project",
      model: "Model",
      state: "State",
      selection: "Selection",
      job: "Job",
      history: "History",
      viewport: "Viewport",
      data: "Data",
      macro: "Macro",
    },
  },
  zh: {
    title: "WASM Python",
    subtitle: "通过 Pyodide 桥和动作目录，把前端流程脚本化，为后续自动化打基础。",
    runtime: "运行时",
    loading: "加载中",
    ready: "就绪",
    idle: "空闲",
    running: "执行中",
    error: "错误",
    firstRun: "首次运行会把 Pyodide 运行时下载到浏览器缓存里。",
    loadRuntime: "加载运行时",
    resetScript: "重置模板",
    runScript: "运行脚本",
    clearOutput: "清空输出",
    startRecording: "开始录制",
    stopRecording: "停止录制",
    recordingActive: "正在把手动 UI 操作记录进 DSL 事件流。",
    exportMacroJson: "导出宏 JSON",
    importMacroJson: "导入宏 JSON",
    macroJsonExported: "已把当前录制宏导出为 JSON。",
    macroJsonImported: "已把宏 JSON 草稿导入编辑器。",
    projectPresets: "项目预设",
    presetName: "预设名称",
    presetNamePlaceholder: "复查当前结果上下文",
    savePreset: "保存预设",
    insertPreset: "插入预设",
    exportPresetJson: "导出预设 JSON",
    deletePreset: "删除预设",
    noProjectSelected: "先选中一个项目，预设才能挂到项目上下文下面。",
    noPresetDraft: "先生成或导入一个宏草稿，才能把它保存成预设。",
    noPresets: "当前项目还没有保存的预设。",
    presetSaved: "已把当前宏草稿保存到项目预设。",
    presetDeleted: "已删除选中的项目预设。",
    presetInserted: "已把选中的项目预设插入编辑器。",
    snapshot: "状态快照",
    actionCatalog: "动作目录",
    macroCatalog: "宏动作目录",
    editor: "Python 脚本",
    output: "输出",
    noOutput: "脚本输出会显示在这里。",
    lineCount: "行数",
    stateKeys: "状态字段",
    actionCount: "动作数",
    payload: "参数",
    actionLog: "动作日志",
    noActionLog: "还没有脚本动作记录。",
    insertMacroDraft: "插入宏草稿",
    macroDraftInserted: "已根据最近动作日志插入宏草稿。",
    noMacroDraftSource: "先执行几条动作，面板才能帮你拼出宏草稿。",
    actionSource: "来源",
    actionPayload: "参数",
    actionResult: "结果",
    actionNote: "备注",
    riskNormal: "普通",
    riskSensitive: "敏感",
    riskDestructive: "高风险",
    confirmationRequired: "执行前需要额外确认。",
    categories: {
      navigation: "导航",
      settings: "设置",
      runtime: "运行时",
      project: "项目",
      model: "模型",
      state: "状态",
      selection: "选择",
      job: "任务",
      history: "历史",
      viewport: "视图",
      data: "数据",
      macro: "宏动作",
    },
  },
  ja: {
    title: "WASM Python",
    subtitle: "Pyodide ブリッジとアクションカタログを使って、フロントエンド作業を自動化します。",
    runtime: "ランタイム",
    loading: "読込中",
    ready: "準備完了",
    idle: "待機",
    running: "実行中",
    error: "エラー",
    firstRun: "初回実行では Pyodide ランタイムをブラウザキャッシュへダウンロードします。",
    loadRuntime: "ランタイムを読み込む",
    resetScript: "テンプレートに戻す",
    runScript: "スクリプトを実行",
    clearOutput: "出力をクリア",
    startRecording: "記録開始",
    stopRecording: "記録停止",
    recordingActive: "手動 UI 操作を DSL イベントとして記録中です。",
    exportMacroJson: "マクロ JSON を書き出す",
    importMacroJson: "マクロ JSON を読み込む",
    macroJsonExported: "現在の記録マクロを JSON で書き出しました。",
    macroJsonImported: "マクロ JSON 下書きをエディタへ読み込みました。",
    projectPresets: "プロジェクトプリセット",
    presetName: "プリセット名",
    presetNamePlaceholder: "現在の結果コンテキストを確認",
    savePreset: "プリセット保存",
    insertPreset: "プリセット挿入",
    exportPresetJson: "プリセット JSON 書き出し",
    deletePreset: "プリセット削除",
    noProjectSelected: "プリセットを保存するには先にプロジェクトを選択してください。",
    noPresetDraft: "先にマクロ下書きを作成または読み込んでください。",
    noPresets: "このプロジェクトにはまだ保存済みプリセットがありません。",
    presetSaved: "現在のマクロ下書きをプロジェクトプリセットに保存しました。",
    presetDeleted: "選択したプリセットを削除しました。",
    presetInserted: "選択したプリセットをエディタに挿入しました。",
    snapshot: "スナップショット",
    actionCatalog: "アクションカタログ",
    macroCatalog: "マクロカタログ",
    editor: "Python スクリプト",
    output: "出力",
    noOutput: "スクリプト出力がここに表示されます。",
    lineCount: "行数",
    stateKeys: "状態キー",
    actionCount: "アクション数",
    payload: "ペイロード",
    actionLog: "アクションログ",
    noActionLog: "まだスクリプトアクションはありません。",
    insertMacroDraft: "マクロ下書きを挿入",
    macroDraftInserted: "最近のアクションログからマクロ下書きを挿入しました。",
    noMacroDraftSource: "先にいくつかのアクションを実行すると、下書きを組み立てられます。",
    actionSource: "ソース",
    actionPayload: "ペイロード",
    actionResult: "結果",
    actionNote: "メモ",
    riskNormal: "通常",
    riskSensitive: "注意",
    riskDestructive: "高リスク",
    confirmationRequired: "実行前に確認が必要です。",
    categories: {
      navigation: "ナビゲーション",
      settings: "設定",
      runtime: "ランタイム",
      project: "プロジェクト",
      model: "モデル",
      state: "状態",
      selection: "選択",
      job: "ジョブ",
      history: "履歴",
      viewport: "ビュー",
      data: "データ",
      macro: "マクロ",
    },
  },
} as const;

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
  const t = copy[language];
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
                  {language === "zh" ? "插入" : language === "ja" ? "挿入" : "Insert"}
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
                <span>{language === "zh" ? "步骤" : language === "ja" ? "ステップ" : "Steps"}</span>
                <code>{macro.steps.map((step) => step.action).join(" -> ")}</code>
              </div>
              {macro.payloadExample ? (
                <p className="card-copy">
                  {language === "zh"
                    ? "这个宏支持通过 payload 覆盖模板参数。"
                    : "This macro supports payload-driven template parameters."}
                </p>
              ) : null}
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => insertMacro(macro.id, macro.payloadExample)} type="button">
                  {language === "zh" ? "插入" : language === "ja" ? "挿入" : "Insert"}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}
