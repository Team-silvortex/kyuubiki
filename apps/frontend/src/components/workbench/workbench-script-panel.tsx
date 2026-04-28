"use client";

import { useEffect, useState } from "react";
import {
  buildWorkbenchPythonPrelude,
  DEFAULT_WORKBENCH_PYTHON,
  ensurePyodideRuntime,
  isWorkbenchScriptActionHighRisk,
  WORKBENCH_SCRIPT_ACTIONS,
  type WorkbenchScriptActionDefinition,
  type WorkbenchScriptActionLogEntry,
  type WorkbenchScriptLanguage,
  type WorkbenchScriptSnapshot,
} from "@/lib/scripting/workbench-script-runtime";

type WorkbenchScriptPanelProps = {
  language: WorkbenchScriptLanguage;
  snapshot: WorkbenchScriptSnapshot;
  getSnapshot: () => WorkbenchScriptSnapshot;
  actionLog: WorkbenchScriptActionLogEntry[];
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
    snapshot: "Snapshot",
    actionCatalog: "Action catalog",
    editor: "Python script",
    output: "Output",
    noOutput: "Script output will appear here.",
    lineCount: "Lines",
    stateKeys: "State fields",
    actionCount: "Actions",
    payload: "Payload",
    actionLog: "Action log",
    noActionLog: "No scripted actions yet.",
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
    snapshot: "状态快照",
    actionCatalog: "动作目录",
    editor: "Python 脚本",
    output: "输出",
    noOutput: "脚本输出会显示在这里。",
    lineCount: "行数",
    stateKeys: "状态字段",
    actionCount: "动作数",
    payload: "参数",
    actionLog: "动作日志",
    noActionLog: "还没有脚本动作记录。",
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

export function WorkbenchScriptPanel({ language, snapshot, getSnapshot, actionLog, onInvokeAction }: WorkbenchScriptPanelProps) {
  const t = copy[language];
  const [scriptCode, setScriptCode] = useState(DEFAULT_WORKBENCH_PYTHON);
  const [output, setOutput] = useState<string[]>([]);
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus>("idle");
  const [runtimeError, setRuntimeError] = useState<string | null>(null);

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

  const appendOutput = (line: string) => {
    setOutput((current) => [...current.slice(-199), line]);
  };

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
          <button className="ghost-button" onClick={() => setOutput([])} type="button">
            {t.clearOutput}
          </button>
        </div>
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
            <strong>{WORKBENCH_SCRIPT_ACTIONS.length}</strong>
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
              <p className="card-copy">{action.summary[language]}</p>
              {isWorkbenchScriptActionHighRisk(action.id) ? <p className="card-copy">{t.confirmationRequired}</p> : null}
              <div className="script-panel__payload">
                <span>{t.payload}</span>
                <code>{stringifyPayload(action.payloadExample)}</code>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => insertAction(action)} type="button">
                  {language === "zh" ? "插入" : "Insert"}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}
