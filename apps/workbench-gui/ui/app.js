const { invoke } = window.__TAURI__.core;

const state = {
  workbenchUrl: "http://127.0.0.1:3000",
  orchestratorUrl: "http://127.0.0.1:4000",
  consoleTab: "status",
  logService: "frontend",
};

const elements = {
  workbenchUrl: document.getElementById("workbench-url"),
  orchestratorUrl: document.getElementById("orchestrator-url"),
  deploymentMode: document.getElementById("deployment-mode"),
  statusOutput: document.getElementById("status-output"),
  logsPanel: document.getElementById("logs-panel"),
  logOutput: document.getElementById("log-output"),
  frame: document.getElementById("workbench-frame"),
  viewerCaption: document.getElementById("viewer-caption"),
};

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
  const environment = await invoke("workbench_environment");
  state.workbenchUrl = environment.workbench_url;
  state.orchestratorUrl = environment.orchestrator_url;
  elements.workbenchUrl.textContent = environment.workbench_url;
  elements.orchestratorUrl.textContent = environment.orchestrator_url;
  elements.deploymentMode.textContent = environment.deployment_mode;
}

function loadWorkbenchFrame() {
  elements.frame.src = state.workbenchUrl;
  elements.viewerCaption.textContent = state.workbenchUrl;
}

function isShortcutModifier(event) {
  return event.metaKey || event.ctrlKey;
}

async function refreshStatus() {
  try {
    const payload = await invoke("service_status");
    elements.statusOutput.textContent = payload.rendered;
  } catch (error) {
    elements.statusOutput.textContent = String(error);
  }
}

async function refreshLog() {
  try {
    const payload = await invoke("read_runtime_log", { payload: { service: state.logService } });
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
      elements.statusOutput.textContent = await invoke("service_stop");
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
      return;
    }

    if (action === "start-local") {
      elements.statusOutput.textContent = await invoke("service_start", { payload: { mode: "local" } });
      loadWorkbenchFrame();
      if (state.consoleTab === "logs") {
        await refreshLog();
      }
      return;
    }

    if (action === "restart-local") {
      elements.statusOutput.textContent = await invoke("service_restart", { payload: { mode: "local" } });
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
  }
});

async function boot() {
  await loadEnvironment();
  loadWorkbenchFrame();
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
  elements.statusOutput.textContent = String(error);
});
