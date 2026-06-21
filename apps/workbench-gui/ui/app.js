import {
  applyDesktopState,
  invokeTauri,
  loadDesktopBrand,
  loadDesktopLanguagePreference,
  normalizeDesktopLanguage,
  saveDesktopLanguagePreference,
  setText,
  syncDesktopStates,
} from "./shared/tauri-bridge.js";
import { normalizeDesktopPlatform } from "./shared/platform.js";
import { formatRuntimeStatusReport, renderRuntimeStatusPlane } from "./shared/runtime-status-summary.js";

const shellCopy = {
  en: {
    language: "Language",
    shellPages: { control: "Control", workbench: "Workbench" },
    runtime: "Runtime",
    runtimeConsole: "Runtime Console",
    viewerControls: "Viewer controls",
    status: "Status",
    logs: "Logs",
    startLocal: "Start local",
    restartLocal: "Restart local",
    refresh: "Refresh",
    stop: "Stop",
    refreshLog: "Refresh log",
    reloadFrame: "Reload frame",
    loadLocalUi: "Load local UI",
    openWorkbenchPage: "Open Workbench page",
    embeddedWorkbench: "Embedded workbench",
    back: "Back",
  },
  zh: {
    language: "语言",
    shellPages: { control: "控制", workbench: "分析" },
    runtime: "运行时",
    runtimeConsole: "运行控制台",
    viewerControls: "视图控制",
    status: "状态",
    logs: "日志",
    startLocal: "启动本地",
    restartLocal: "重启本地",
    refresh: "刷新",
    stop: "停止",
    refreshLog: "刷新日志",
    reloadFrame: "重载界面",
    loadLocalUi: "载入本地界面",
    openWorkbenchPage: "打开分析页",
    embeddedWorkbench: "内嵌 Workbench",
    back: "返回",
  },
  ja: {
    language: "言語",
    shellPages: { control: "コントロール", workbench: "解析" },
    runtime: "ランタイム",
    runtimeConsole: "ランタイムコンソール",
    viewerControls: "ビュー制御",
    status: "状態",
    logs: "ログ",
    startLocal: "ローカル起動",
    restartLocal: "ローカル再起動",
    refresh: "更新",
    stop: "停止",
    refreshLog: "ログ更新",
    reloadFrame: "画面再読み込み",
    loadLocalUi: "ローカル UI を開く",
    openWorkbenchPage: "解析ページを開く",
    embeddedWorkbench: "埋め込み Workbench",
    back: "戻る",
  },
  es: {
    language: "Idioma",
    shellPages: { control: "Control", workbench: "Análisis" },
    runtime: "Runtime",
    runtimeConsole: "Consola de runtime",
    viewerControls: "Controles de vista",
    status: "Estado",
    logs: "Registros",
    startLocal: "Iniciar local",
    restartLocal: "Reiniciar local",
    refresh: "Actualizar",
    stop: "Detener",
    refreshLog: "Actualizar registro",
    reloadFrame: "Recargar interfaz",
    loadLocalUi: "Abrir UI local",
    openWorkbenchPage: "Abrir análisis",
    embeddedWorkbench: "Workbench incrustado",
    back: "Volver",
  },
};

const state = {
  workbenchUrl: "http://127.0.0.1:3000",
  orchestratorUrl: "http://127.0.0.1:4000",
  shellPage: "control",
  consoleTab: "status",
  logService: "frontend",
  releaseVersion: "",
  releaseCodename: "",
  language: "en",
};

const elements = {
  shellRoot: document.getElementById("shell-root"),
  workbenchUrl: document.getElementById("workbench-url"),
  orchestratorUrl: document.getElementById("orchestrator-url"),
  deploymentMode: document.getElementById("deployment-mode"),
  statusPlane: document.getElementById("status-plane"),
  statusOutput: document.getElementById("status-output"),
  logsPanel: document.getElementById("logs-panel"),
  logOutput: document.getElementById("log-output"),
  frame: document.getElementById("workbench-frame"),
  viewerCaption: document.getElementById("viewer-caption"),
  languageLabel: document.getElementById("shell-language-label"),
  languageSelect: document.getElementById("shell-language-select"),
  shellTabs: Array.from(document.querySelectorAll("[data-shell-page]")),
  shellTargets: Array.from(document.querySelectorAll("[data-shell-target]")),
  shellPanes: Array.from(document.querySelectorAll("[data-shell-pane]")),
};

function shellText() {
  return shellCopy[state.language] || shellCopy.en;
}

function renderLanguage() {
  const t = shellText();
  document.documentElement.lang = state.language;
  if (elements.languageLabel) elements.languageLabel.textContent = t.language;
  if (elements.languageSelect) elements.languageSelect.value = state.language;

  const shellPageLabels = {
    control: t.shellPages.control,
    workbench: t.shellPages.workbench,
  };

  document.querySelector('[data-shell-page="control"]')?.replaceChildren(t.shellPages.control);
  document.querySelector('[data-shell-page="workbench"]')?.replaceChildren(t.shellPages.workbench);
  document.querySelector(".panel:nth-of-type(1) .panel__title")?.replaceChildren(t.runtime);
  document.querySelector(".panel:nth-of-type(2) .panel__title")?.replaceChildren(t.runtimeConsole);
  document.querySelector(".panel:nth-of-type(3) .panel__title")?.replaceChildren(t.viewerControls);
  document.querySelector('[data-console-tab="status"]')?.replaceChildren(t.status);
  document.querySelector('[data-console-tab="logs"]')?.replaceChildren(t.logs);
  document.querySelector('[data-action="start-local"]')?.replaceChildren(t.startLocal);
  document.querySelector('[data-action="restart-local"]')?.replaceChildren(t.restartLocal);
  document.querySelector('[data-action="refresh"]')?.replaceChildren(t.refresh);
  document.querySelector('[data-action="stop"]')?.replaceChildren(t.stop);
  document.querySelector('[data-action="refresh-log"]')?.replaceChildren(t.refreshLog);
  document.querySelectorAll('[data-action="reload-frame"]').forEach((button) => button.replaceChildren(t.reloadFrame));
  document.querySelector('[data-action="open-local"]')?.replaceChildren(t.loadLocalUi);
  document.querySelector('[data-shell-target="workbench"]')?.replaceChildren(t.openWorkbenchPage);
  document.querySelector(".viewer__headline strong")?.replaceChildren(t.embeddedWorkbench);
  document.querySelector(".viewer__back-button")?.replaceChildren(t.back);

  for (const button of elements.shellTabs) {
    const page = button.dataset.shellPage;
    if (page && shellPageLabels[page]) {
      button.textContent = shellPageLabels[page];
    }
  }
}

function postLanguageToWorkbench() {
  elements.frame.contentWindow?.postMessage(
    {
      type: "kyuubiki:set-language",
      language: state.language,
    },
    "*",
  );
}

function renderShellPages() {
  for (const button of elements.shellTabs) {
    button.classList.toggle("is-active", button.dataset.shellPage === state.shellPage);
  }

  for (const pane of elements.shellPanes) {
    pane.classList.toggle("hidden", pane.dataset.shellPane !== state.shellPage);
  }

  elements.shellRoot?.classList.toggle("shell-shell--workbench", state.shellPage === "workbench");
}

function setShellPage(page) {
  state.shellPage = page === "workbench" ? "workbench" : "control";
  renderShellPages();
}

function renderConsoleTabs() {
  for (const button of document.querySelectorAll("[data-console-tab]")) {
    button.classList.toggle("is-active", button.dataset.consoleTab === state.consoleTab);
  }

  const showLogs = state.consoleTab === "logs";
  elements.statusOutput.classList.toggle("is-hidden", showLogs);
  elements.logsPanel.classList.toggle("is-hidden", !showLogs);
}

function renderLogServiceTabs() {
  for (const button of document.querySelectorAll("[data-log-service]")) {
    button.classList.toggle("is-active", button.dataset.logService === state.logService);
  }
}

async function loadEnvironment() {
  const environment = await invokeTauri("workbench_environment");
  state.workbenchUrl = environment.workbench_url;
  state.orchestratorUrl = environment.orchestrator_url;
  elements.workbenchUrl.textContent = environment.workbench_url;
  elements.orchestratorUrl.textContent = environment.orchestrator_url;
  applyDesktopState(elements.deploymentMode, environment.deployment_mode, { kind: "activity" });
  if (elements.shellRoot) {
    elements.shellRoot.dataset.hostPlatform = normalizeDesktopPlatform(environment.host_platform);
  }
}

function loadWorkbenchFrame() {
  const nextUrl = new URL(state.workbenchUrl);
  nextUrl.searchParams.set("desktopLanguage", state.language);
  elements.frame.src = nextUrl.toString();
  elements.viewerCaption.textContent = state.workbenchUrl;
}

function isShortcutModifier(event) {
  return event.metaKey || event.ctrlKey;
}

function releaseLabel() {
  const releaseTag = [state.releaseCodename, state.releaseVersion].filter(Boolean).join(" ");
  return releaseTag ? `Kyuubiki Workbench · ${releaseTag}` : "Kyuubiki Workbench";
}

function formatStatusReport(rendered, summary, meshRuntime) {
  return formatRuntimeStatusReport({
    title: releaseLabel(),
    rendered,
    summary,
  }, meshRuntime);
}

async function fetchMeshRuntimeHealth(orchestratorBaseUrl) {
  const baseUrl = String(orchestratorBaseUrl || "").trim().replace(/\/+$/u, "");
  if (!baseUrl) return null;

  const response = await fetch(`${baseUrl}/api/health`);
  if (!response.ok) {
    throw new Error(`mesh runtime health request failed with ${response.status}`);
  }

  return response.json();
}

function invokeGuardedMutation(action, payload = {}) {
  return invokeTauri("guarded_mutation_action", {
    payload: {
      action,
      ...payload,
    },
  });
}

async function refreshStatus() {
  try {
    const payload = await invokeTauri("service_status");
    const meshRuntime = await fetchMeshRuntimeHealth(state.orchestratorUrl).catch(() => null);
    renderRuntimeStatusPlane(elements.statusPlane, payload.summary, meshRuntime);
    elements.statusOutput.textContent = formatStatusReport(payload.rendered, payload.summary, meshRuntime);
  } catch (error) {
    renderRuntimeStatusPlane(elements.statusPlane, null);
    elements.statusOutput.textContent = formatStatusReport(String(error));
  }
}

async function refreshLog() {
  try {
    const payload = await invokeTauri("read_runtime_log", { payload: { service: state.logService } });
    elements.logOutput.textContent = payload.rendered || `${state.logService} log is empty.`;
  } catch (error) {
    elements.logOutput.textContent = String(error);
  }
}

async function runAction(action) {
  try {
    if (action === "refresh") {
      await refreshStatus();
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
      return;
    }

    if (action === "reload-frame") {
      loadWorkbenchFrame();
      return;
    }

    if (action === "open-local") {
      loadWorkbenchFrame();
      await refreshStatus();
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
      return;
    }

    if (action === "refresh-log") {
      await refreshLog();
      return;
    }

    if (action === "stop") {
      await invokeGuardedMutation("service_stop");
      await refreshStatus();
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
      return;
    }

    if (action === "start-local") {
      await invokeGuardedMutation("service_start", { mode: "local" });
      await refreshStatus();
      loadWorkbenchFrame();
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
      return;
    }

    if (action === "restart-local") {
      await invokeGuardedMutation("service_restart", { mode: "local" });
      await refreshStatus();
      loadWorkbenchFrame();
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
    }
  } catch (error) {
    elements.statusOutput.textContent = String(error);
  }
}

for (const button of document.querySelectorAll("[data-action]")) {
  button.addEventListener("click", () => runAction(button.dataset.action));
}

for (const button of document.querySelectorAll("[data-console-tab]")) {
  button.addEventListener("click", async () => {
    state.consoleTab = button.dataset.consoleTab;
    renderConsoleTabs();
    if (state.consoleTab === "logs") {
      await refreshLog();
    }
  });
}

for (const button of document.querySelectorAll("[data-log-service]")) {
  button.addEventListener("click", async () => {
    state.logService = button.dataset.logService;
    renderLogServiceTabs();
    await refreshLog();
  });
}

for (const button of elements.shellTabs) {
  button.addEventListener("click", () => {
    setShellPage(button.dataset.shellPage);
  });
}

for (const button of elements.shellTargets) {
  button.addEventListener("click", () => {
    setShellPage(button.dataset.shellTarget);
  });
}

elements.languageSelect?.addEventListener("change", async (event) => {
  const nextLanguage = normalizeDesktopLanguage(event.target.value);
  state.language = await saveDesktopLanguagePreference(nextLanguage);
  renderLanguage();
  postLanguageToWorkbench();
});

window.addEventListener("message", async (event) => {
  if (event.source !== elements.frame.contentWindow) return;
  if (event.data?.type !== "kyuubiki:language-changed") return;
  const nextLanguage = normalizeDesktopLanguage(event.data.language);
  if (nextLanguage === state.language) return;
  state.language = await saveDesktopLanguagePreference(nextLanguage);
  renderLanguage();
});

window.addEventListener("keydown", async (event) => {
  if (!isShortcutModifier(event)) return;

  const key = event.key.toLowerCase();

  if (key === "r" && event.shiftKey) {
    event.preventDefault();
    await runAction("restart-local");
    return;
  }

  if (key === "r") {
    event.preventDefault();
    await runAction("reload-frame");
    return;
  }

  if (key === "1") {
    event.preventDefault();
    state.consoleTab = "status";
    renderConsoleTabs();
    await refreshStatus();
    return;
  }

  if (key === "2") {
    event.preventDefault();
    state.consoleTab = "logs";
    renderConsoleTabs();
    await refreshLog();
    return;
  }

  if (key === "l") {
    event.preventDefault();
    state.consoleTab = "logs";
    renderConsoleTabs();
    await refreshLog();
    return;
  }

  if (key === "3") {
    event.preventDefault();
    setShellPage("workbench");
  }
});

async function boot() {
  const brand = await loadDesktopBrand();
  state.language = await loadDesktopLanguagePreference();
  if (brand) {
    if (brand.applicationName) {
      const releaseVersion = String(brand.releaseVersion || "").replace(/^v/u, "");
      const releaseCodename = String(brand.releaseCodename || "").trim();
      const releaseTag = [releaseCodename, releaseVersion].filter(Boolean).join(" ");
      state.releaseVersion = releaseVersion;
      state.releaseCodename = releaseCodename;
      document.title = releaseTag ? `${brand.applicationName} · ${releaseTag}` : brand.applicationName;
      setText("brand-workbench-name", brand.workbenchShortName || "Workbench");
    }
    setText("brand-workbench-role-chip", brand?.shellRoleLabel);

    if (brand.releaseVersion || brand.releaseCodename) {
      const releaseTag = [
        String(brand.releaseCodename || "").trim(),
        String(brand.releaseVersion || "").replace(/^v/u, ""),
      ]
        .filter(Boolean)
        .join(" ");
      setText("brand-workbench-version", releaseTag);
    }

    setText("brand-workbench-description", brand.workbenchShellDescription);
    setText("brand-workbench-focus", brand.shellFocusLabel);
  }

  await loadEnvironment();
  renderLanguage();
  loadWorkbenchFrame();
  elements.frame.addEventListener("load", () => {
    postLanguageToWorkbench();
  });
  syncDesktopStates();
  renderShellPages();
  renderConsoleTabs();
  renderLogServiceTabs();
  await refreshStatus();
  window.setInterval(async () => {
    await refreshStatus();
    if (state.consoleTab === "logs") {
      await refreshLog();
    }
  }, 5000);
}

boot().catch((error) => {
  elements.statusOutput.textContent = formatStatusReport(String(error));
});
