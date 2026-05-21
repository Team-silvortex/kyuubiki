import {
  applyDesktopState,
  invokeTauri,
  loadDesktopBrand,
  setText,
  syncDesktopStates,
} from "./shared/tauri-bridge.js";

const sectionModel = {
  projects: {
    title: "Home",
    copy: "Start with one clear path: bring work in, inspect it once, then move into Workbench.",
  },
  runtimes: {
    title: "Runtimes",
    copy: "Start the right loop, check runtime health, and keep logs close.",
  },
  deploy: {
    title: "Deploy",
    copy: "Choose the target posture, validate the workstation, and prepare release paths.",
  },
  observe: {
    title: "Observe",
    copy: "Scan health, tails, and recent risk signals without leaving the desktop shell.",
  },
  tools: {
    title: "Tools",
    copy: "Run diagnostics, packaging, and verification from one operator surface.",
  },
};

const HUB_RECENTS_KEY = "kyuubiki.hub.recents.v1";
const HUB_WORKLOAD_LIBRARY_KEY = "kyuubiki.hub.workloads.v1";
const HUB_ASSISTANT_SETTINGS_KEY = "kyuubiki.hub.assistant.settings.v1";
const HUB_ASSISTANT_SECRETS_KEY = "kyuubiki.hub.assistant.secrets.v1";
const HUB_ASSISTANT_AUDIT_KEY = "kyuubiki.hub.assistant.audit.v1";
const HUB_HOT_LOG_SETTINGS_KEY = "kyuubiki.hub.hot-log-settings.v1";
const HUB_RUNTIME_LOG_SETTINGS_KEY = "kyuubiki.hub.runtime-log-settings.v1";
const HUB_DENSITY_SETTINGS_KEY = "kyuubiki.hub.density-settings.v1";
const HUB_RECENTS_LIMIT = 6;
const HUB_ACTION_HISTORY_LIMIT = 8;
const HUB_ASSISTANT_AUDIT_LIMIT = 16;
const HUB_WORKLOAD_LIBRARY_LIMIT = 32;
const HUB_HOT_LOG_POLL_MS = 4000;
const HUB_ASSISTANT_MODEL_PRESETS = ["gpt-5", "gpt-5-mini", "gpt-4.1", "custom"];
const HUB_ASSISTANT_ACTION_RISK = {
  "hub/focusSection": "low",
  "hub/openWorkbench": "low",
  "hub/openInstaller": "sensitive",
  "hub/startLocal": "sensitive",
  "hub/validateEnv": "low",
  "hub/desktopStage": "sensitive",
  "hub/desktopBuildHost": "high",
  "hub/desktopVerify": "sensitive",
  "hub/setBundleContext": "low",
  "hub/projectInspect": "low",
  "hub/projectValidate": "low",
  "hub/projectNormalize": "sensitive",
  "hub/projectUnpack": "sensitive",
  "hub/projectPack": "high",
  "hub/projectDiff": "low",
};
const PROJECT_ACTION_LABELS = {
  "project inspect": "project-inspect",
  "project validate": "project-validate",
  "project normalize": "project-normalize",
  "project unpack": "project-unpack",
  "project pack": "project-pack",
  "project diff": "project-diff",
};
const HUB_ASSISTANT_ACTIONS = [
  { id: "hub/focusSection", summary: "Focus a Hub section.", payloadExample: { section: "projects" } },
  { id: "hub/openWorkbench", summary: "Open the Workbench desktop shell.", payloadExample: {} },
  { id: "hub/openInstaller", summary: "Open the Installer desktop shell.", payloadExample: {} },
  { id: "hub/startLocal", summary: "Start the local stack.", payloadExample: {} },
  { id: "hub/validateEnv", summary: "Validate the desktop environment.", payloadExample: {} },
  { id: "hub/desktopStage", summary: "Prepare desktop manifests for the selected platform.", payloadExample: {} },
  { id: "hub/desktopBuildHost", summary: "Build host bundles for the current machine.", payloadExample: {} },
  { id: "hub/desktopVerify", summary: "Verify the current desktop release staging area.", payloadExample: {} },
  { id: "hub/setBundleContext", summary: "Fill Hub bundle path inputs.", payloadExample: { path: "", comparePath: "", out: "" } },
  { id: "hub/projectInspect", summary: "Inspect the selected project bundle.", payloadExample: { path: "" } },
  { id: "hub/projectValidate", summary: "Validate the selected project bundle.", payloadExample: { path: "" } },
  { id: "hub/projectNormalize", summary: "Normalize the selected project bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectUnpack", summary: "Unpack the selected project bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectPack", summary: "Pack a project directory into a bundle.", payloadExample: { path: "", out: "" } },
  { id: "hub/projectDiff", summary: "Diff two project bundles.", payloadExample: { leftPath: "", rightPath: "" } },
];
const HUB_DENSITY_DEFAULTS = {
  "projects-workflow": false,
  "runtimes-remote-targets": false,
  "deploy-suggested-flow": false,
  "tools-output": false,
  "side-current-mode": false,
};

const state = {
  hostPlatform: "macos",
  activeSection: "projects",
  projectsPage: "start",
  panelPages: {
    runtimes: "local",
    observe: "health",
    tools: "packages",
  },
  isBusy: false,
  historyFilter: "all",
  workloadFilter: "all",
  workloadFamilyFilter: "all",
  assistantMode: "local",
  assistantPlan: null,
  hotLogRefreshInFlight: false,
  runtimeLogRefreshInFlight: false,
  density: { ...HUB_DENSITY_DEFAULTS },
  releaseVersion: "",
  releaseCodename: "",
};

let hotRuntimeLogPollHandle = null;
let observeRuntimeLogPollHandle = null;

const elements = {
  title: document.getElementById("section-title"),
  copy: document.getElementById("section-copy"),
  navItems: Array.from(document.querySelectorAll(".hub-nav__item")),
  panels: Array.from(document.querySelectorAll(".hub-panel")),
  projectsPageButtons: Array.from(document.querySelectorAll("[data-projects-page]")),
  projectsTargetButtons: Array.from(document.querySelectorAll("[data-projects-target]")),
  projectsPanes: Array.from(document.querySelectorAll("[data-projects-pane]")),
  panelPageButtons: Array.from(document.querySelectorAll("[data-panel-page-group][data-panel-page]")),
  panelPanes: Array.from(document.querySelectorAll("[data-panel-pane-group][data-panel-pane]")),
  releasePlatform: document.getElementById("release-platform"),
  projectBundlePath: document.getElementById("project-bundle-path"),
  projectBundleComparePath: document.getElementById("project-bundle-compare-path"),
  projectBundleOutPath: document.getElementById("project-bundle-out-path"),
  projectBundleOutput: document.getElementById("project-bundle-output"),
  workloadCatalogUrl: document.getElementById("workload-catalog-url"),
  workloadLabel: document.getElementById("workload-label"),
  workloadImportInput: document.getElementById("workload-import-input"),
  workloadLibraryList: document.getElementById("workload-library-list"),
  workloadLibraryOutput: document.getElementById("workload-library-output"),
  workloadFilterButtons: Array.from(document.querySelectorAll("[data-workload-filter]")),
  workloadFamilyFilterButtons: Array.from(document.querySelectorAll("[data-workload-family-filter]")),
  historyImportInput: document.getElementById("history-import-input"),
  recentBundleList: document.getElementById("recent-bundle-list"),
  recentCompareList: document.getElementById("recent-compare-list"),
  recentOutputList: document.getElementById("recent-output-list"),
  favoriteActionList: document.getElementById("favorite-action-list"),
  recentActionList: document.getElementById("recent-action-list"),
  operationOutput: document.getElementById("hub-operation-output"),
  runtimeStatusOutput: document.getElementById("runtime-status-output"),
  localRuntimeStatus: document.getElementById("local-runtime-status"),
  observeRuntimeStatusOutput: document.getElementById("observe-runtime-status-output"),
  observeRuntimeStatus: document.getElementById("observe-runtime-status"),
  hotRuntimeStatusOutput: document.getElementById("hot-runtime-status-output"),
  hotRuntimeStatus: document.getElementById("hot-runtime-status"),
  hotRuntimeMode: document.getElementById("hot-runtime-mode"),
  hotRuntimeLogService: document.getElementById("hot-runtime-log-service"),
  hotRuntimeLogAuto: document.getElementById("hot-runtime-log-auto"),
  hotRuntimeLogInterval: document.getElementById("hot-runtime-log-interval"),
  hotRuntimeLogFollowState: document.getElementById("hot-runtime-log-follow-state"),
  hotRuntimeLogOutput: document.getElementById("hot-runtime-log-output"),
  observeHotStatus: document.getElementById("observe-hot-status"),
  observeHotMode: document.getElementById("observe-hot-mode"),
  observeHotFollowState: document.getElementById("observe-hot-follow-state"),
  observeHotLogService: document.getElementById("observe-hot-log-service"),
  observeHotLogOutput: document.getElementById("observe-hot-log-output"),
  observeRuntimeLogService: document.getElementById("observe-runtime-log-service"),
  observeRuntimeLogAuto: document.getElementById("observe-runtime-log-auto"),
  observeRuntimeLogFollowState: document.getElementById("observe-runtime-log-follow-state"),
  observeRuntimeLogOutput: document.getElementById("observe-runtime-log-output"),
  workbenchUrl: document.getElementById("local-workbench-url"),
  orchestratorUrl: document.getElementById("local-orchestrator-url"),
  currentRuntimeMode: document.getElementById("current-runtime-mode"),
  currentProfile: document.getElementById("current-profile"),
  actionState: document.getElementById("hub-action-state"),
  desktopStatusOutput: document.getElementById("hub-desktop-status-output"),
  actionButtons: Array.from(document.querySelectorAll("[data-action]")),
  sectionJumpButtons: Array.from(document.querySelectorAll("[data-target-section]")),
  historyFilterButtons: Array.from(document.querySelectorAll("[data-history-filter]")),
  historyManageButtons: Array.from(document.querySelectorAll("[data-history-manage]")),
  assistantModeButtons: Array.from(document.querySelectorAll("[data-assistant-mode]")),
  assistantEngineState: document.getElementById("assistant-engine-state"),
  assistantContextSection: document.getElementById("assistant-context-section"),
  assistantContextRuntime: document.getElementById("assistant-context-runtime"),
  assistantContextBundle: document.getElementById("assistant-context-bundle"),
  assistantLocalPanel: document.getElementById("assistant-local-panel"),
  assistantLocalCards: document.getElementById("assistant-local-cards"),
  assistantLlmPanel: document.getElementById("assistant-llm-panel"),
  assistantBaseUrl: document.getElementById("assistant-base-url"),
  assistantApiKey: document.getElementById("assistant-api-key"),
  assistantModelPreset: document.getElementById("assistant-model-preset"),
  assistantModelName: document.getElementById("assistant-model-name"),
  assistantPrompt: document.getElementById("assistant-prompt"),
  assistantEndpointPolicy: document.getElementById("assistant-endpoint-policy"),
  assistantRequestPlan: document.getElementById("assistant-request-plan"),
  assistantApprovePlan: document.getElementById("assistant-approve-plan"),
  assistantExecutePlan: document.getElementById("assistant-execute-plan"),
  assistantPlanActions: document.getElementById("assistant-plan-actions"),
  assistantOutput: document.getElementById("assistant-output"),
  assistantAuditList: document.getElementById("assistant-audit-list"),
  densityToggleButtons: Array.from(document.querySelectorAll("[data-density-toggle]")),
  densityPanels: Array.from(document.querySelectorAll("[data-density-panel]")),
};

function loadHubRecents() {
  try {
    const raw = window.localStorage.getItem(HUB_RECENTS_KEY);
    if (!raw) {
      return { bundles: [], compares: [], outputs: [] };
    }

    const parsed = JSON.parse(raw);
    return {
      bundles: Array.isArray(parsed?.bundles) ? parsed.bundles : [],
      compares: Array.isArray(parsed?.compares) ? parsed.compares : [],
      outputs: Array.isArray(parsed?.outputs) ? parsed.outputs : [],
      actions: Array.isArray(parsed?.actions) ? parsed.actions : [],
    };
  } catch {
    return { bundles: [], compares: [], outputs: [], actions: [] };
  }
}

function persistHubRecents(recents) {
  window.localStorage.setItem(HUB_RECENTS_KEY, JSON.stringify(recents));
}

function loadHubWorkloadLibrary() {
  try {
    const raw = window.localStorage.getItem(HUB_WORKLOAD_LIBRARY_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persistHubWorkloadLibrary(entries) {
  window.localStorage.setItem(
    HUB_WORKLOAD_LIBRARY_KEY,
    JSON.stringify(entries.slice(0, HUB_WORKLOAD_LIBRARY_LIMIT)),
  );
}

function appendTextElement(parent, tagName, text, className) {
  const element = document.createElement(tagName);
  if (className) {
    element.className = className;
  }
  element.textContent = text;
  parent.appendChild(element);
  return element;
}

function appendAssistantCardHeader(parent, title, badgeText, badgeClassName) {
  const header = document.createElement("div");
  header.className = "desktop-shell-section-header";
  appendTextElement(header, "strong", title);
  appendTextElement(header, "span", badgeText, badgeClassName);
  parent.appendChild(header);
  return header;
}

function workloadIdentity(entry) {
  return [
    String(entry?.sourceKind || "").trim(),
    String(entry?.bundlePath || "").trim(),
    String(entry?.downloadUrl || "").trim(),
    String(entry?.projectId || "").trim(),
  ].join("::");
}

function normalizeHubWorkloadEntry(entry) {
  const label = String(entry?.label || entry?.projectName || "").trim();
  const sourceKind = String(entry?.sourceKind || "").trim() || "local-bundle";
  const bundlePath = String(entry?.bundlePath || "").trim();
  const downloadUrl = String(entry?.downloadUrl || "").trim();
  const projectId = String(entry?.projectId || "").trim();
  const projectName = String(entry?.projectName || "").trim();

  if (!label && !bundlePath && !downloadUrl && !projectId) {
    return null;
  }

  return {
    id: String(entry?.id || `workload-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    label: label || projectName || bundlePath || downloadUrl || "workload",
    note: String(entry?.note || "").trim(),
    sourceKind,
    sourceLabel: String(entry?.sourceLabel || "").trim(),
    bundlePath,
    downloadUrl,
    projectId,
    projectName,
    schema: String(entry?.schema || "").trim(),
    layout: String(entry?.layout || "").trim(),
    modelCount: Number.isFinite(Number(entry?.modelCount)) ? Number(entry.modelCount) : 0,
    versionCount: Number.isFinite(Number(entry?.versionCount)) ? Number(entry.versionCount) : 0,
    jobCount: Number.isFinite(Number(entry?.jobCount)) ? Number(entry.jobCount) : 0,
    resultCount: Number.isFinite(Number(entry?.resultCount)) ? Number(entry.resultCount) : 0,
    analysisDomains: Array.isArray(entry?.analysisDomains)
      ? entry.analysisDomains.filter((value) => typeof value === "string")
      : Array.isArray(entry?.analysis_domains)
        ? entry.analysis_domains.filter((value) => typeof value === "string")
        : [],
    analysisFamilies: Array.isArray(entry?.analysisFamilies)
      ? entry.analysisFamilies.filter((value) => typeof value === "string")
      : Array.isArray(entry?.analysis_families)
        ? entry.analysis_families.filter((value) => typeof value === "string")
        : [],
    thermalIntents: Array.isArray(entry?.thermalIntents)
      ? entry.thermalIntents.filter((value) => typeof value === "string")
      : Array.isArray(entry?.thermal_intents)
        ? entry.thermal_intents.filter((value) => typeof value === "string")
        : [],
    downloadedAt: String(entry?.downloadedAt || "").trim(),
    attachedAt: String(entry?.attachedAt || "").trim(),
    addedAt: String(entry?.addedAt || "").trim() || new Date().toISOString(),
    updatedAt: String(entry?.updatedAt || "").trim() || new Date().toISOString(),
  };
}

function mergeHubWorkloadLibrary(existingEntries, incomingEntries) {
  const merged = [];

  for (const candidate of [...incomingEntries, ...existingEntries]) {
    const normalized = normalizeHubWorkloadEntry(candidate);
    if (!normalized) {
      continue;
    }

    const duplicateIndex = merged.findIndex((entry) => workloadIdentity(entry) === workloadIdentity(normalized));
    if (duplicateIndex >= 0) {
      continue;
    }

    merged.push(normalized);
    if (merged.length >= HUB_WORKLOAD_LIBRARY_LIMIT) {
      break;
    }
  }

  return merged;
}

function loadHubAssistantSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_ASSISTANT_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return {
      mode: parsed?.mode === "llm" ? "llm" : "local",
      baseUrl: String(parsed?.baseUrl || ""),
      modelPreset: HUB_ASSISTANT_MODEL_PRESETS.includes(String(parsed?.modelPreset || "")) ? parsed.modelPreset : "gpt-5",
      model: String(parsed?.model || "gpt-5"),
    };
  } catch {
    return { mode: "local", baseUrl: "", modelPreset: "gpt-5", model: "gpt-5" };
  }
}

function persistHubAssistantSettings(settings) {
  window.localStorage.setItem(HUB_ASSISTANT_SETTINGS_KEY, JSON.stringify(settings));
}

function loadHubAssistantSecrets() {
  try {
    const raw = window.sessionStorage.getItem(HUB_ASSISTANT_SECRETS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return {
      apiKey: String(parsed?.apiKey || ""),
    };
  } catch {
    return { apiKey: "" };
  }
}

function persistHubAssistantSecrets(secrets) {
  window.sessionStorage.setItem(HUB_ASSISTANT_SECRETS_KEY, JSON.stringify(secrets));
}

function loadHubAssistantAudit() {
  try {
    const raw = window.sessionStorage.getItem(HUB_ASSISTANT_AUDIT_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persistHubAssistantAudit(entries) {
  window.sessionStorage.setItem(HUB_ASSISTANT_AUDIT_KEY, JSON.stringify(entries.slice(0, HUB_ASSISTANT_AUDIT_LIMIT)));
}

function loadHubHotLogSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_HOT_LOG_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const interval = String(parsed?.interval || "4000");
    return {
      service: String(parsed?.service || "hot-stack"),
      autoRefresh: parsed?.autoRefresh !== false,
      interval: ["2000", "4000", "8000"].includes(interval) ? interval : "4000",
    };
  } catch {
    return { service: "hot-stack", autoRefresh: true, interval: "4000" };
  }
}

function persistHubHotLogSettings(settings) {
  window.localStorage.setItem(HUB_HOT_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

function loadHubRuntimeLogSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_RUNTIME_LOG_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const service = String(parsed?.service || "frontend");
    return {
      service: ["frontend", "orchestrator", "agent-5001", "agent-5002"].includes(service) ? service : "frontend",
      autoRefresh: parsed?.autoRefresh !== false,
    };
  } catch {
    return { service: "frontend", autoRefresh: true };
  }
}

function persistHubRuntimeLogSettings(settings) {
  window.localStorage.setItem(HUB_RUNTIME_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

function loadHubDensitySettings() {
  try {
    const raw = window.localStorage.getItem(HUB_DENSITY_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return Object.fromEntries(
      Object.entries(HUB_DENSITY_DEFAULTS).map(([key, defaultExpanded]) => [
        key,
        typeof parsed?.[key] === "boolean" ? parsed[key] : defaultExpanded,
      ]),
    );
  } catch {
    return { ...HUB_DENSITY_DEFAULTS };
  }
}

function persistHubDensitySettings() {
  window.localStorage.setItem(HUB_DENSITY_SETTINGS_KEY, JSON.stringify(state.density));
}

function assistantRiskLevel(action) {
  return HUB_ASSISTANT_ACTION_RISK[action] || "low";
}

function assistantRiskStateClass(risk) {
  switch (risk) {
    case "high":
      return "desktop-shell-state desktop-shell-state--danger";
    case "sensitive":
      return "desktop-shell-state desktop-shell-state--warning";
    default:
      return "desktop-shell-state desktop-shell-state--healthy";
  }
}

function assistantStatusStateClass(status) {
  switch (status) {
    case "failed":
    case "cancelled":
      return "desktop-shell-state desktop-shell-state--danger";
    case "prompted":
    case "confirmed":
      return "desktop-shell-state desktop-shell-state--warning";
    case "completed":
      return "desktop-shell-state desktop-shell-state--healthy";
    default:
      return "desktop-shell-state desktop-shell-state--idle";
  }
}

function assistantDeliveryStateClass(delivery) {
  switch (delivery) {
    case "synced":
      return "desktop-shell-state desktop-shell-state--healthy";
    case "sync_failed":
      return "desktop-shell-state desktop-shell-state--danger";
    default:
      return "desktop-shell-state desktop-shell-state--idle";
  }
}

function formatAssistantAuditTime(value) {
  const timestamp = new Date(String(value || "").trim());
  if (Number.isNaN(timestamp.getTime())) {
    return String(value || "").trim();
  }

  return timestamp.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function renderHubAssistantAudit(entries = loadHubAssistantAudit()) {
  if (!elements.assistantAuditList) {
    return;
  }

  elements.assistantAuditList.innerHTML = "";
  if (!entries.length) {
    renderEmptyHistoryState(elements.assistantAuditList, "No assistant actions recorded in this session.");
    return;
  }

  entries.forEach((entry) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    const header = document.createElement("div");
    header.className = "desktop-shell-section-header";
    appendTextElement(header, "strong", entry.action);
    const badges = document.createElement("div");
    badges.className = "desktop-shell-action-row";
    appendTextElement(badges, "span", entry.risk, assistantRiskStateClass(entry.risk));
    appendTextElement(badges, "span", entry.status, assistantStatusStateClass(entry.status));
    appendTextElement(badges, "span", entry.delivery || "local", assistantDeliveryStateClass(entry.delivery || "local"));
    header.appendChild(badges);
    article.appendChild(header);
    appendTextElement(
      article,
      "p",
      `${formatAssistantAuditTime(entry.createdAt)} · ${entry.source}${entry.note ? ` · ${entry.note}` : ""}`,
      "desktop-shell-note",
    );
    elements.assistantAuditList.appendChild(article);
  });
}

function rememberHubAssistantAudit(entry) {
  const normalized = {
    auditId: String(entry?.auditId || `hub-assistant-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    action: String(entry?.action || "").trim(),
    risk: String(entry?.risk || "low").trim(),
    status: String(entry?.status || "idle").trim(),
    source: String(entry?.source || "assistant").trim(),
    note: String(entry?.note || "").trim(),
    createdAt: new Date().toISOString(),
    delivery: String(entry?.delivery || "local").trim(),
  };

  if (!normalized.action) {
    return loadHubAssistantAudit();
  }

  const next = [normalized, ...loadHubAssistantAudit()].slice(0, HUB_ASSISTANT_AUDIT_LIMIT);
  persistHubAssistantAudit(next);
  renderHubAssistantAudit(next);
  if (entry?.sync !== false) {
    void mirrorHubAssistantAuditToSecurityEvents(normalized);
  }
  return next;
}

function currentOrchestratorBaseUrl() {
  const text = String(elements.orchestratorUrl?.textContent || "").trim();
  return text || "http://127.0.0.1:4000";
}

function currentLocalWorkloadCatalogUrl() {
  return `${currentOrchestratorBaseUrl().replace(/\/+$/u, "")}/api/v1/workloads/catalog`;
}

function ensureDefaultWorkloadCatalogUrl(force = false) {
  if (!elements.workloadCatalogUrl) {
    return "";
  }

  if (!force && String(elements.workloadCatalogUrl.value || "").trim()) {
    return String(elements.workloadCatalogUrl.value || "").trim();
  }

  const next = currentLocalWorkloadCatalogUrl();
  elements.workloadCatalogUrl.value = next;
  return next;
}

function currentAssistantAuditContext() {
  return {
    section: state.activeSection,
    runtime: String(elements.currentRuntimeMode?.textContent || "").trim(),
    profile: String(elements.currentProfile?.textContent || "").trim(),
    bundle_path: String(elements.projectBundlePath?.value || "").trim(),
    compare_path: String(elements.projectBundleComparePath?.value || "").trim(),
    output_path: String(elements.projectBundleOutPath?.value || "").trim(),
  };
}

function updateHubAssistantAuditDelivery(auditId, delivery, noteSuffix = "") {
  const entries = loadHubAssistantAudit();
  const next = entries.map((entry) => {
    if (entry.auditId !== auditId) {
      return entry;
    }
    return {
      ...entry,
      delivery,
      note: noteSuffix ? `${entry.note}${entry.note ? " · " : ""}${noteSuffix}` : entry.note,
    };
  });
  persistHubAssistantAudit(next);
  renderHubAssistantAudit(next);
}

async function mirrorHubAssistantAuditToSecurityEvents(entry) {
  const baseUrl = currentOrchestratorBaseUrl().replace(/\/+$/, "");
  try {
    const response = await fetch(`${baseUrl}/api/v1/security-events`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        event_id: entry.auditId,
        event_type: "hub.assistant.action",
        source: "hub-assistant",
        action: entry.action,
        risk: entry.risk,
        status: entry.status,
        note: entry.note || null,
        context: {
          ...currentAssistantAuditContext(),
          assistant_source: entry.source,
          delivery: "hub-session",
        },
        occurred_at: entry.createdAt,
      }),
    });

    if (!response.ok) {
      throw new Error(`control-plane sync failed (${response.status})`);
    }

    updateHubAssistantAuditDelivery(entry.auditId, "synced");
  } catch (error) {
    updateHubAssistantAuditDelivery(
      entry.auditId,
      "sync_failed",
      error instanceof Error ? error.message : String(error),
    );
  }
}

function saveHubRecents(recents) {
  persistHubRecents(recents);
  renderHubRecents(recents);
}

function setWorkloadLibraryOutput(value) {
  if (elements.workloadLibraryOutput) {
    elements.workloadLibraryOutput.textContent = value;
  }
}

function rawErrorMessage(error) {
  return error instanceof Error ? error.message : String(error || "");
}

function formatHubOperatorError(error, options = {}) {
  const raw = rawErrorMessage(error).trim();
  const actionLabel = String(options?.actionLabel || "This action").trim();
  const service = String(options?.service || "").trim();
  const context = String(options?.context || "").trim();

  if (/request timed out:/i.test(raw)) {
    return `${actionLabel} timed out. Check runtime health and agent availability, then try again.`;
  }

  if (context === "log-read") {
    return `Couldn't read the ${service || "selected"} log right now. Check whether the runtime is running, then refresh the log again.`;
  }

  if (context === "desktop-status") {
    return "Couldn't refresh desktop packaging status right now. Check the local runtime tools and try again.";
  }

  if (/operation not permitted|permission denied|access denied|denied|eperm/i.test(raw)) {
    return `${actionLabel} needs additional local access. Check desktop permissions and try again.`;
  }

  if (/invalid analysis_domains|invalid analysis_families|invalid thermal_intents|missing label/i.test(raw)) {
    return `The workload catalog format is not valid for ${actionLabel.toLowerCase()}. Check the catalog entry and try again.`;
  }

  if (!raw) {
    return `${actionLabel} didn't complete. Try again after checking runtime state and inputs.`;
  }

  return `${actionLabel} didn't complete: ${raw}`;
}

function inferDownloadFilename(url, fallback = "kyuubiki-workload.kyuubiki") {
  try {
    const parsed = new URL(String(url || "").trim());
    const pathname = parsed.pathname.split("/").filter(Boolean).at(-1);
    return pathname || fallback;
  } catch {
    return fallback;
  }
}

function downloadHubBlob(filename, blob) {
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}

function workloadSourceBadge(entry) {
  if (entry.sourceKind === "remote-catalog" && entry.bundlePath) {
    return ["attached local", "desktop-shell-state desktop-shell-state--healthy"];
  }

  if (entry.sourceKind === "remote-catalog" && entry.downloadedAt) {
    return ["downloaded", "desktop-shell-state desktop-shell-state--warning"];
  }

  switch (entry.sourceKind) {
    case "remote-catalog":
      return ["remote catalog", "desktop-shell-state desktop-shell-state--healthy"];
    case "imported-library":
      return ["imported", "desktop-shell-state desktop-shell-state--warning"];
    default:
      return ["local bundle", "desktop-shell-state desktop-shell-state--idle"];
  }
}

function workloadProvenanceLabel(entry) {
  if (entry.sourceKind === "remote-catalog") {
    if (entry.sourceLabel === "Kyuubiki Control Plane") {
      return "first-party control plane catalog";
    }
    const hostHint = workloadProvenanceHost(entry.sourceLabel || entry.downloadUrl || "");
    if (hostHint) {
      return `custom remote catalog · ${hostHint}`;
    }
    return `custom remote catalog${entry.sourceLabel ? ` · ${entry.sourceLabel}` : ""}`;
  }

  if (entry.sourceKind === "imported-library") {
    return "imported library snapshot";
  }

  if (entry.sourceLabel) {
    return entry.sourceLabel;
  }

  return "Hub local registration";
}

function workloadProvenanceHost(value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return "";
  }

  try {
    return new URL(normalized).host;
  } catch {
    return "";
  }
}

function workloadDomainLabel(domain) {
  switch (domain) {
    case "mechanical":
      return "Mechanical";
    case "thermal":
      return "Thermal";
    case "thermo_mechanical":
      return "Thermo-mechanical";
    default:
      return String(domain || "").trim();
  }
}

function workloadFamilyLabel(family) {
  switch (family) {
    case "axial_and_springs":
      return "Axial & Springs";
    case "beams_and_frames":
      return "Beams & Frames";
    case "trusses":
      return "Trusses";
    case "planes":
      return "Planes";
    default:
      return String(family || "").trim();
  }
}

function markHubWorkloadDownloaded(entry) {
  const next = loadHubWorkloadLibrary().map((candidate) => {
    if (workloadIdentity(candidate) !== workloadIdentity(entry)) {
      return candidate;
    }

    return {
      ...candidate,
      downloadedAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  });
  saveHubWorkloadLibrary(next);
}

function updateHubWorkloadEntry(entry, updater) {
  const next = loadHubWorkloadLibrary()
    .map((candidate) => {
      if (workloadIdentity(candidate) !== workloadIdentity(entry)) {
        return candidate;
      }

      return normalizeHubWorkloadEntry(
        updater({
          ...candidate,
        }),
      );
    })
    .filter(Boolean);
  saveHubWorkloadLibrary(next);
}

async function downloadRemoteWorkloadBundle(entry) {
  const validation = validateHubCatalogUrl(entry.downloadUrl || "");
  if (!validation.ok) {
    throw new Error(validation.reason);
  }

  const response = await fetch(validation.normalized);
  if (!response.ok) {
    throw new Error(`bundle download failed (${response.status})`);
  }

  const blob = await response.blob();
  const filename = inferDownloadFilename(validation.normalized);
  downloadHubBlob(filename, blob);
  markHubWorkloadDownloaded(entry);
  setWorkloadLibraryOutput(`downloaded ${entry.label} as ${filename}`);
}

async function openWorkloadInWorkbench(entry) {
  if (!entry.bundlePath) {
    throw new Error("This workload does not have a local bundle path yet.");
  }

  elements.projectBundlePath.value = entry.bundlePath;
  renderAssistantContext();
  renderHubAssistantLocalCards();
  setWorkloadLibraryOutput(`loaded ${entry.label} into the bundle path and opening Workbench`);
  await runAction("open-workbench");
}

async function attachCurrentBundleToWorkload(entry) {
  const bundlePath = String(elements.projectBundlePath?.value || "").trim();
  if (!bundlePath) {
    throw new Error("Fill in the current bundle path before attaching it to this workload.");
  }

  const inspectRaw = await invokeTauri("project_bundle_inspect", { payload: { path: bundlePath } });
  const summary = projectSummaryFromInspectPayload(inspectRaw);
  updateHubWorkloadEntry(entry, (candidate) => ({
    ...candidate,
    bundlePath,
    projectId: summary.projectId || candidate.projectId,
    projectName: summary.projectName || candidate.projectName,
    schema: summary.schema || candidate.schema,
    layout: summary.layout || candidate.layout,
    modelCount: summary.modelCount,
    versionCount: summary.versionCount,
    jobCount: summary.jobCount,
    resultCount: summary.resultCount,
    analysisDomains: summary.analysisDomains,
    analysisFamilies: summary.analysisFamilies,
    thermalIntents: summary.thermalIntents,
    attachedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  }));
  setWorkloadLibraryOutput(`attached local bundle ${bundlePath} to ${entry.label}`);
}

function saveHubWorkloadLibrary(entries) {
  persistHubWorkloadLibrary(entries);
  renderHubWorkloadLibrary(entries);
}

function matchesWorkloadFilter(entry) {
  if (state.workloadFilter === "all") {
    return matchesWorkloadFamilyFilter(entry);
  }
  return entry.analysisDomains.includes(state.workloadFilter) && matchesWorkloadFamilyFilter(entry);
}

function matchesWorkloadFamilyFilter(entry) {
  if (state.workloadFamilyFilter === "all") {
    return true;
  }
  return entry.analysisFamilies.includes(state.workloadFamilyFilter);
}

function renderWorkloadFilters() {
  elements.workloadFilterButtons.forEach((button) => {
    const matches = button.dataset.workloadFilter === state.workloadFilter;
    button.classList.toggle("desktop-shell-button-primary", matches);
    button.classList.toggle("desktop-shell-button-ghost", !matches);
  });
  elements.workloadFamilyFilterButtons.forEach((button) => {
    const matches = button.dataset.workloadFamilyFilter === state.workloadFamilyFilter;
    button.classList.toggle("desktop-shell-button-primary", matches);
    button.classList.toggle("desktop-shell-button-ghost", !matches);
  });
}

function renderHubWorkloadLibrary(entries = loadHubWorkloadLibrary()) {
  if (!elements.workloadLibraryList) {
    return;
  }

  renderWorkloadFilters();
  elements.workloadLibraryList.innerHTML = "";
  if (!entries.length) {
    renderEmptyHistoryState(
      elements.workloadLibraryList,
      "No managed workloads yet. Register a current bundle or sync a remote catalog.",
    );
    return;
  }

  const filteredEntries = entries.filter((entry) => matchesWorkloadFilter(entry));
  if (!filteredEntries.length) {
    const domainLabel = state.workloadFilter === "all" ? "all domains" : state.workloadFilter;
    const familyLabel = state.workloadFamilyFilter === "all" ? "all families" : state.workloadFamilyFilter;
    renderEmptyHistoryState(
      elements.workloadLibraryList,
      `No workloads match ${domainLabel} / ${familyLabel}.`,
    );
    return;
  }

  filteredEntries.forEach((entry) => {
    const shell = document.createElement("div");
    shell.className = "hub-history-item";

    const summary = document.createElement("button");
    summary.type = "button";
    summary.className = "hub-history-item__summary desktop-shell-button-ghost";
    const [sourceLabel, sourceClass] = workloadSourceBadge(entry);
    const metaBits = [
      entry.projectId ? `project ${entry.projectId}` : "",
      entry.schema || "",
      entry.layout || "",
      entry.attachedAt ? `attached ${formatProjectActionTime(entry.attachedAt)}` : "",
      entry.downloadedAt ? `downloaded ${formatProjectActionTime(entry.downloadedAt)}` : "",
    ].filter(Boolean);
    const heading = document.createElement("div");
    heading.className = "hub-history-item__heading";
    appendTextElement(heading, "strong", entry.label);
    const meta = document.createElement("div");
    meta.className = "hub-history-item__meta";
    appendTextElement(meta, "span", sourceLabel, sourceClass);
    entry.analysisDomains.forEach((domain) => {
      appendTextElement(meta, "span", workloadDomainLabel(domain), "desktop-shell-chip");
    });
    entry.analysisFamilies.forEach((family) => {
      appendTextElement(meta, "span", workloadFamilyLabel(family), "desktop-shell-chip");
    });
    heading.appendChild(meta);
    summary.appendChild(heading);
    appendTextElement(summary, "span", metaBits.join(" · ") || "workload entry", "hub-history-item__alias");
    appendTextElement(summary, "span", entry.note || entry.bundlePath || entry.downloadUrl || "--");
    appendTextElement(summary, "span", workloadProvenanceLabel(entry), "hub-history-item__provenance");
    if (entry.thermalIntents.length) {
      appendTextElement(summary, "span", `thermal: ${entry.thermalIntents.join(", ")}`, "desktop-shell-note");
    }
    summary.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
      }
      if (entry.downloadUrl && elements.workloadCatalogUrl) {
        elements.workloadCatalogUrl.value = entry.downloadUrl;
      }
      setWorkloadLibraryOutput(`restored workload context for ${entry.label}`);
      renderAssistantContext();
      renderHubAssistantLocalCards();
    });

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";

    const useButton = document.createElement("button");
    useButton.type = "button";
    useButton.className = "desktop-shell-button-ghost";
    useButton.textContent = "Use";
    useButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
      }
      setWorkloadLibraryOutput(`loaded ${entry.label} into the bundle path`);
      renderAssistantContext();
      renderHubAssistantLocalCards();
    });

    const workbenchButton = document.createElement("button");
    workbenchButton.type = "button";
    workbenchButton.className = "desktop-shell-button-ghost";
    workbenchButton.textContent = "Open in Workbench";
    workbenchButton.disabled = !entry.bundlePath;
    workbenchButton.addEventListener("click", () => {
      void openWorkloadInWorkbench(entry).catch((error) => {
        setWorkloadLibraryOutput(formatHubOperatorError(error, {
          actionLabel: "Opening this workload in Workbench",
        }));
      });
    });

    const inspectButton = document.createElement("button");
    inspectButton.type = "button";
    inspectButton.className = "desktop-shell-button-ghost";
    inspectButton.textContent = "Inspect";
    inspectButton.disabled = !entry.bundlePath;
    inspectButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
        void runAction("project-inspect");
      }
    });

    const validateButton = document.createElement("button");
    validateButton.type = "button";
    validateButton.className = "desktop-shell-button-ghost";
    validateButton.textContent = "Validate";
    validateButton.disabled = !entry.bundlePath;
    validateButton.addEventListener("click", () => {
      if (entry.bundlePath) {
        elements.projectBundlePath.value = entry.bundlePath;
        void runAction("project-validate");
      }
    });

    const downloadButton = document.createElement("button");
    downloadButton.type = "button";
    downloadButton.className = "desktop-shell-button-ghost";
    downloadButton.textContent = "Download";
    downloadButton.disabled = !entry.downloadUrl;
    downloadButton.addEventListener("click", () => {
      void downloadRemoteWorkloadBundle(entry).catch((error) => {
        setWorkloadLibraryOutput(formatHubOperatorError(error, {
          actionLabel: "Downloading this workload",
        }));
      });
    });

    const attachButton = document.createElement("button");
    attachButton.type = "button";
    attachButton.className = "desktop-shell-button-ghost";
    attachButton.textContent = entry.bundlePath ? "Reattach bundle" : "Attach current bundle";
    attachButton.addEventListener("click", () => {
      void attachCurrentBundleToWorkload(entry).catch((error) => {
        setWorkloadLibraryOutput(formatHubOperatorError(error, {
          actionLabel: "Attaching the current bundle",
        }));
      });
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "desktop-shell-button-ghost";
    removeButton.textContent = "Remove";
    removeButton.addEventListener("click", () => {
      const next = loadHubWorkloadLibrary().filter((candidate) => workloadIdentity(candidate) !== workloadIdentity(entry));
      saveHubWorkloadLibrary(next);
      setWorkloadLibraryOutput(`removed ${entry.label} from the workload library`);
    });

    controls.append(useButton, workbenchButton, inspectButton, validateButton, downloadButton, attachButton, removeButton);
    shell.append(summary, controls);
    elements.workloadLibraryList.appendChild(shell);
  });
}

function projectSummaryFromInspectPayload(raw) {
  const parsed = JSON.parse(raw);
  return {
    projectId: String(parsed?.project_id || "").trim(),
    projectName: String(parsed?.project_name || "").trim(),
    schema: String(parsed?.schema || "").trim(),
    layout: String(parsed?.layout || "").trim(),
    modelCount: Number(parsed?.model_count || 0),
    versionCount: Number(parsed?.version_count || 0),
    jobCount: Number(parsed?.job_count || 0),
    resultCount: Number(parsed?.result_count || 0),
    analysisDomains: Array.isArray(parsed?.analysis_domains) ? parsed.analysis_domains.filter((value) => typeof value === "string") : [],
    analysisFamilies: Array.isArray(parsed?.analysis_families) ? parsed.analysis_families.filter((value) => typeof value === "string") : [],
    thermalIntents: Array.isArray(parsed?.thermal_intents) ? parsed.thermal_intents.filter((value) => typeof value === "string") : [],
  };
}

async function registerCurrentBundleAsWorkload() {
  const bundlePath = String(elements.projectBundlePath?.value || "").trim();
  if (!bundlePath) {
    throw new Error("Fill in a bundle path before registering a workload.");
  }

  const inspectRaw = await invokeTauri("project_bundle_inspect", { payload: { path: bundlePath } });
  const summary = projectSummaryFromInspectPayload(inspectRaw);
  const note = String(elements.workloadLabel?.value || "").trim();
  const entry = normalizeHubWorkloadEntry({
    label: note || summary.projectName || summary.projectId || bundlePath,
    note: note || `Registered from local bundle ${bundlePath}`,
    sourceKind: "local-bundle",
    sourceLabel: "Hub local registration",
    bundlePath,
    projectId: summary.projectId,
    projectName: summary.projectName,
    schema: summary.schema,
    layout: summary.layout,
    modelCount: summary.modelCount,
    versionCount: summary.versionCount,
    jobCount: summary.jobCount,
    resultCount: summary.resultCount,
    analysisDomains: summary.analysisDomains,
    analysisFamilies: summary.analysisFamilies,
    thermalIntents: summary.thermalIntents,
  });

  const next = mergeHubWorkloadLibrary(loadHubWorkloadLibrary(), [entry]);
  saveHubWorkloadLibrary(next);
  setWorkloadLibraryOutput(`registered ${entry.label} in the workload library`);
}

function validateHubCatalogUrl(value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return { ok: false, reason: "Fill in a workload catalog URL first." };
  }

  try {
    const parsed = new URL(normalized);
    const protocol = parsed.protocol.toLowerCase();
    const hostname = parsed.hostname.toLowerCase();
    const isLoopback =
      hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
    if (protocol === "https:" || (protocol === "http:" && isLoopback)) {
      return { ok: true, normalized };
    }
    return {
      ok: false,
      reason: "Catalog URL must use https, or http only for localhost / 127.0.0.1 / ::1.",
    };
  } catch {
    return { ok: false, reason: "Catalog URL must be a valid absolute URL." };
  }
}

function validateRemoteWorkloadCatalogPayload(payload) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return { ok: false, reason: "Catalog payload must be a JSON object." };
  }

  if (payload.schema_version !== "kyuubiki.workload-catalog/v1") {
    return {
      ok: false,
      reason: "Catalog schema_version must be kyuubiki.workload-catalog/v1.",
    };
  }

  if (!Array.isArray(payload.workloads)) {
    return { ok: false, reason: "Catalog workloads must be an array." };
  }

  for (const [index, workload] of payload.workloads.entries()) {
    if (!workload || typeof workload !== "object" || Array.isArray(workload)) {
      return { ok: false, reason: `Workload ${index + 1} must be an object.` };
    }

    if (!String(workload.label || "").trim()) {
      return { ok: false, reason: `Workload ${index + 1} is missing label.` };
    }

    const hasRequiredLocator =
      String(workload.download_url || "").trim() ||
      String(workload.bundle_path || "").trim() ||
      String(workload.project_id || "").trim();
    if (!hasRequiredLocator) {
    return {
      ok: false,
      reason: `Workload ${index + 1} must define download_url, bundle_path, or project_id.`,
    };
  }

    if (
      workload.analysis_domains !== undefined &&
      (!Array.isArray(workload.analysis_domains) ||
        workload.analysis_domains.some(
          (value) =>
            typeof value !== "string" ||
            !["mechanical", "thermal", "thermo_mechanical"].includes(value),
        ))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid analysis_domains.`,
      };
    }

    if (
      workload.analysis_families !== undefined &&
      (!Array.isArray(workload.analysis_families) ||
        workload.analysis_families.some(
          (value) =>
            typeof value !== "string" ||
            !["axial_and_springs", "beams_and_frames", "trusses", "planes"].includes(value),
        ))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid analysis_families.`,
      };
    }

    if (
      workload.thermal_intents !== undefined &&
      (!Array.isArray(workload.thermal_intents) ||
        workload.thermal_intents.some((value) => typeof value !== "string"))
    ) {
      return {
        ok: false,
        reason: `Workload ${index + 1} has invalid thermal_intents.`,
      };
    }
  }

  return { ok: true };
}

function normalizeRemoteWorkloadCatalogPayload(payload, catalogUrl) {
  const list = Array.isArray(payload)
    ? payload
    : Array.isArray(payload?.workloads)
      ? payload.workloads
      : Array.isArray(payload?.items)
        ? payload.items
        : [];

  return list
    .map((entry) =>
      normalizeHubWorkloadEntry({
        label: entry?.label || entry?.name || entry?.projectName || entry?.project_name,
        note: entry?.note || entry?.description || `Synced from ${catalogUrl}`,
        sourceKind: "remote-catalog",
        sourceLabel: entry?.sourceLabel || payload?.sourceLabel || catalogUrl,
        bundlePath: entry?.bundlePath || entry?.bundle_path || "",
        downloadUrl: entry?.downloadUrl || entry?.download_url || catalogUrl,
        projectId: entry?.projectId || entry?.project_id || "",
        projectName: entry?.projectName || entry?.project_name || "",
        schema: entry?.schema || "",
        layout: entry?.layout || "",
        modelCount: entry?.modelCount || entry?.model_count || 0,
        versionCount: entry?.versionCount || entry?.version_count || 0,
        jobCount: entry?.jobCount || entry?.job_count || 0,
        resultCount: entry?.resultCount || entry?.result_count || 0,
        analysisDomains: entry?.analysisDomains || entry?.analysis_domains || [],
        analysisFamilies: entry?.analysisFamilies || entry?.analysis_families || [],
        thermalIntents: entry?.thermalIntents || entry?.thermal_intents || [],
      }),
    )
    .filter(Boolean);
}

async function syncRemoteWorkloadCatalog(urlOverride = "") {
  const selectedUrl =
    String(urlOverride || "").trim() || String(elements.workloadCatalogUrl?.value || "").trim();
  const validation = validateHubCatalogUrl(selectedUrl);
  if (!validation.ok) {
    throw new Error(validation.reason);
  }

  if (elements.workloadCatalogUrl) {
    elements.workloadCatalogUrl.value = validation.normalized;
  }

  const response = await fetch(validation.normalized);
  if (!response.ok) {
    throw new Error(`catalog sync failed (${response.status})`);
  }

  const payload = await response.json();
  const payloadValidation = validateRemoteWorkloadCatalogPayload(payload);
  if (!payloadValidation.ok) {
    throw new Error(payloadValidation.reason);
  }
  const normalized = normalizeRemoteWorkloadCatalogPayload(payload, validation.normalized);
  const next = mergeHubWorkloadLibrary(loadHubWorkloadLibrary(), normalized);
  saveHubWorkloadLibrary(next);
  setWorkloadLibraryOutput(`synced ${normalized.length} workload entries from remote catalog`);
}

async function syncLocalControlPlaneWorkloads() {
  const catalogUrl = ensureDefaultWorkloadCatalogUrl(true);
  await syncRemoteWorkloadCatalog(catalogUrl);
}

function exportHubWorkloadLibrary() {
  const payload = {
    exportedAt: new Date().toISOString(),
    workloadCount: loadHubWorkloadLibrary().length,
    workloads: loadHubWorkloadLibrary(),
  };
  downloadHubJson("kyuubiki-hub-workloads.json", payload);
  setWorkloadLibraryOutput(`exported ${payload.workloadCount} workload entries as JSON`);
}

async function importHubWorkloadLibrary(file) {
  if (!file) {
    return;
  }

  const raw = await file.text();
  const parsed = JSON.parse(raw);
  const imported = Array.isArray(parsed?.workloads) ? parsed.workloads : [];
  const normalized = imported
    .map((entry) =>
      normalizeHubWorkloadEntry({
        ...entry,
        sourceKind: entry?.sourceKind || "imported-library",
      }),
    )
    .filter(Boolean);
  const next = mergeHubWorkloadLibrary(loadHubWorkloadLibrary(), normalized);
  saveHubWorkloadLibrary(next);
  setWorkloadLibraryOutput(`imported ${normalized.length} workload entries into the Hub library`);
}

function clearHubWorkloadLibrary() {
  saveHubWorkloadLibrary([]);
  setWorkloadLibraryOutput("cleared the Hub workload library");
}

function pushRecentValue(values, value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return values.slice(0, HUB_RECENTS_LIMIT);
  }

  return [normalized, ...values.filter((entry) => entry !== normalized)].slice(0, HUB_RECENTS_LIMIT);
}

function summarizeProjectActionResult(value) {
  const normalized = String(value || "").replace(/\s+/g, " ").trim();
  if (!normalized) {
    return "";
  }

  return normalized.length > 140 ? `${normalized.slice(0, 137)}...` : normalized;
}

function formatProjectActionTime(value) {
  const normalized = String(value || "").trim();
  if (!normalized) {
    return "";
  }

  const timestamp = new Date(normalized);
  if (Number.isNaN(timestamp.getTime())) {
    return normalized;
  }

  return timestamp.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function projectActionStateClass(status) {
  switch (String(status || "").trim()) {
    case "ok":
      return "desktop-shell-state desktop-shell-state--healthy";
    case "failed":
      return "desktop-shell-state desktop-shell-state--danger";
    default:
      return "desktop-shell-state desktop-shell-state--idle";
  }
}

function rememberProjectBundleAction(
  action,
  { bundlePath = "", comparePath = "", outputPath = "", status = "idle", note = "", executedAt = "" } = {},
) {
  const normalizedAction = String(action || "").trim();
  if (!normalizedAction) {
    return [];
  }

  const recents = loadHubRecents();
  const existingEntry = (recents.actions ?? []).find((entry) => {
    return (
      entry.action === normalizedAction &&
      String(entry.bundlePath || "").trim() === String(bundlePath || "").trim() &&
      String(entry.comparePath || "").trim() === String(comparePath || "").trim() &&
      String(entry.outputPath || "").trim() === String(outputPath || "").trim()
    );
  });
  const nextEntry = {
    action: normalizedAction,
    bundlePath: String(bundlePath || "").trim(),
    comparePath: String(comparePath || "").trim(),
    outputPath: String(outputPath || "").trim(),
    status: String(status || "idle").trim() || "idle",
    note: summarizeProjectActionResult(note),
    executedAt: String(executedAt || "").trim() || new Date().toISOString(),
    pinned: Boolean(existingEntry?.pinned),
    favoriteLabel: String(existingEntry?.favoriteLabel || "").trim(),
  };

  return [
    nextEntry,
    ...(recents.actions ?? []).filter((entry) => {
      return !(
        entry.action === nextEntry.action &&
        entry.bundlePath === nextEntry.bundlePath &&
        entry.comparePath === nextEntry.comparePath &&
        entry.outputPath === nextEntry.outputPath
      );
    }),
  ].slice(0, HUB_ACTION_HISTORY_LIMIT);
}

function normalizeImportedProjectAction(entry) {
  const normalizedAction = String(entry?.action || "").trim();
  if (!normalizedAction) {
    return null;
  }

  return {
    action: normalizedAction,
    bundlePath: String(entry?.bundlePath || "").trim(),
    comparePath: String(entry?.comparePath || "").trim(),
    outputPath: String(entry?.outputPath || "").trim(),
    status: String(entry?.status || "idle").trim() || "idle",
    note: summarizeProjectActionResult(entry?.note || ""),
    executedAt: String(entry?.executedAt || "").trim() || new Date().toISOString(),
    pinned: Boolean(entry?.pinned),
    favoriteLabel: String(entry?.favoriteLabel || "").trim(),
  };
}

function mergeProjectActionHistory(existingActions, importedActions) {
  const merged = [];

  for (const entry of [...importedActions, ...existingActions]) {
    const normalized = normalizeImportedProjectAction(entry);
    if (!normalized) {
      continue;
    }

    const duplicateIndex = merged.findIndex((candidate) => {
      return (
        candidate.action === normalized.action &&
        candidate.bundlePath === normalized.bundlePath &&
        candidate.comparePath === normalized.comparePath &&
        candidate.outputPath === normalized.outputPath
      );
    });

    if (duplicateIndex >= 0) {
      continue;
    }

    merged.push(normalized);
    if (merged.length >= HUB_ACTION_HISTORY_LIMIT) {
      break;
    }
  }

  return merged;
}

function actionIdentity(entry) {
  return [
    String(entry?.action || "").trim(),
    String(entry?.bundlePath || "").trim(),
    String(entry?.comparePath || "").trim(),
    String(entry?.outputPath || "").trim(),
  ].join("::");
}

function shellQuote(value) {
  const normalized = String(value || "");
  if (!normalized) {
    return "''";
  }

  return `'${normalized.replace(/'/g, `'\\''`)}'`;
}

function buildProjectCliCommand(entry) {
  const action = String(entry?.action || "").trim();
  const bundlePath = String(entry?.bundlePath || "").trim();
  const comparePath = String(entry?.comparePath || "").trim();
  const outputPath = String(entry?.outputPath || "").trim();

  switch (action) {
    case "project inspect":
      return `kyuubiki project inspect ${shellQuote(bundlePath)} --json`;
    case "project validate":
      return `kyuubiki project validate ${shellQuote(bundlePath)} --json`;
    case "project normalize":
      return `kyuubiki project normalize ${shellQuote(bundlePath)} --out ${shellQuote(outputPath)}`;
    case "project unpack":
      return `kyuubiki project unpack ${shellQuote(bundlePath)} --out ${shellQuote(outputPath)}`;
    case "project pack":
      return `kyuubiki project pack ${shellQuote(bundlePath)} --out ${shellQuote(outputPath)}`;
    case "project diff":
      return `kyuubiki project diff ${shellQuote(bundlePath)} ${shellQuote(comparePath)} --json`;
    default:
      return "";
  }
}

function buildPythonMacroStub(entry) {
  const action = String(entry?.action || "").trim();
  const bundlePath = JSON.stringify(String(entry?.bundlePath || "").trim());
  const comparePath = JSON.stringify(String(entry?.comparePath || "").trim());
  const outputPath = JSON.stringify(String(entry?.outputPath || "").trim());
  const label = JSON.stringify(String(entry?.favoriteLabel || entry?.action || "favorite-workflow").trim());

  switch (action) {
    case "project inspect":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectInspect", {"path": ${bundlePath}})\n`;
    case "project validate":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectValidate", {"path": ${bundlePath}})\n`;
    case "project normalize":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectNormalize", {"path": ${bundlePath}, "out": ${outputPath}})\n`;
    case "project unpack":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectUnpack", {"path": ${bundlePath}, "out": ${outputPath}})\n`;
    case "project pack":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectPack", {"path": ${bundlePath}, "out": ${outputPath}})\n`;
    case "project diff":
      return `# ${JSON.parse(label)}\nawait ky.invoke("hub/projectDiff", {"leftPath": ${bundlePath}, "rightPath": ${comparePath}})\n`;
    default:
      return "";
  }
}

async function copyProjectCliCommand(entry) {
  const command = buildProjectCliCommand(entry);
  if (!command) {
    setProjectBundleOutput(`cannot build CLI command for ${entry.action}`);
    return;
  }

  await navigator.clipboard.writeText(command);
  setProjectBundleOutput(`copied CLI command for ${entry.favoriteLabel || entry.action}`);
}

async function copyPythonMacroStub(entry) {
  const snippet = buildPythonMacroStub(entry);
  if (!snippet) {
    setProjectBundleOutput(`cannot build Python stub for ${entry.action}`);
    return;
  }

  await navigator.clipboard.writeText(snippet);
  setProjectBundleOutput(`copied Python stub for ${entry.favoriteLabel || entry.action}`);
}

function sortProjectActionHistory(actions) {
  return [...actions].sort((left, right) => {
    if (Boolean(left?.pinned) !== Boolean(right?.pinned)) {
      return left?.pinned ? -1 : 1;
    }

    const leftTime = new Date(String(left?.executedAt || "")).getTime();
    const rightTime = new Date(String(right?.executedAt || "")).getTime();
    return (Number.isNaN(rightTime) ? 0 : rightTime) - (Number.isNaN(leftTime) ? 0 : leftTime);
  });
}

function saveProjectBundleRecents({
  action = "",
  bundlePath = "",
  comparePath = "",
  outputPath = "",
  status = "idle",
  note = "",
  executedAt = "",
} = {}) {
  const next = loadHubRecents();
  next.bundles = pushRecentValue(next.bundles, bundlePath);
  next.compares = pushRecentValue(next.compares, comparePath);
  next.outputs = pushRecentValue(next.outputs, outputPath);
  next.actions = rememberProjectBundleAction(action, { bundlePath, comparePath, outputPath, status, note, executedAt });
  saveHubRecents(next);
}

function renderRecentPathList(container, values, input) {
  if (!container) {
    return;
  }

  container.innerHTML = "";
  if (!values.length) {
    const empty = document.createElement("div");
    empty.className = "hub-recent-empty";
    empty.textContent = "No recent entries yet.";
    container.appendChild(empty);
    return;
  }

  values.forEach((value) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "hub-recent-item desktop-shell-button-ghost";
    button.textContent = value;
    button.title = value;
    button.addEventListener("click", () => {
      input.value = value;
      input.focus();
    });
    container.appendChild(button);
  });
}

function renderHubRecents(recents = loadHubRecents()) {
  renderRecentPathList(elements.recentBundleList, recents.bundles, elements.projectBundlePath);
  renderRecentPathList(elements.recentCompareList, recents.compares, elements.projectBundleComparePath);
  renderRecentPathList(elements.recentOutputList, recents.outputs, elements.projectBundleOutPath);
  renderHistoryFilters();
  renderRecentActionHistory(sortProjectActionHistory(recents.actions ?? []));
  renderHubWorkloadLibrary();
  renderAssistantContext();
  renderHubAssistantLocalCards();
}

function renderRecentActionHistory(actions) {
  if (!elements.recentActionList || !elements.favoriteActionList) {
    return;
  }

  const filteredActions = actions.filter((entry) => matchesHistoryFilter(entry));
  const favoriteActions = filteredActions.filter((entry) => entry.pinned);
  const recentActions = filteredActions.filter((entry) => !entry.pinned);
  elements.favoriteActionList.innerHTML = "";
  elements.recentActionList.innerHTML = "";
  if (!actions.length) {
    renderEmptyHistoryState(elements.favoriteActionList, "No favorite actions yet.");
    renderEmptyHistoryState(elements.recentActionList, "No recent project actions yet.");
    return;
  }

  if (!filteredActions.length) {
    renderEmptyHistoryState(elements.favoriteActionList, `No favorites match the ${state.historyFilter} filter.`);
    renderEmptyHistoryState(elements.recentActionList, `No actions match the ${state.historyFilter} filter.`);
    return;
  }

  if (!favoriteActions.length) {
    renderEmptyHistoryState(elements.favoriteActionList, "No pinned favorites yet.");
  } else {
    renderProjectActionEntries(elements.favoriteActionList, favoriteActions);
  }

  if (!recentActions.length) {
    renderEmptyHistoryState(elements.recentActionList, "No non-pinned actions in this view.");
  } else {
    renderProjectActionEntries(elements.recentActionList, recentActions);
  }
}

function renderEmptyHistoryState(container, message) {
  const empty = document.createElement("div");
  empty.className = "hub-recent-empty";
  empty.textContent = message;
  container.appendChild(empty);
}

function renderProjectActionEntries(container, actions) {
  actions.forEach((entry) => {
    const shell = document.createElement("div");
    shell.className = "hub-history-item";

    const button = document.createElement("button");
    button.type = "button";
    button.className = "hub-history-item__summary desktop-shell-button-ghost";
    const paths = [entry.bundlePath, entry.comparePath, entry.outputPath].filter(Boolean).join("  •  ");
    const badge = `<span class="${projectActionStateClass(entry.status)}">${entry.status || "idle"}</span>`;
    const time = formatProjectActionTime(entry.executedAt);
    const meta = [badge, time ? `<span>${time}</span>` : ""].filter(Boolean).join("");
    const details = summarizeProjectActionResult(entry.note) || paths || "No stored paths";
    const title = entry.pinned && entry.favoriteLabel ? entry.favoriteLabel : entry.action;
    button.innerHTML = `
      <div class="hub-history-item__heading">
        <strong>${title}</strong>
        <div class="hub-history-item__meta">${meta}</div>
      </div>
      ${entry.pinned && entry.favoriteLabel ? `<span class="hub-history-item__alias">${entry.action}</span>` : ""}
      <span>${details}</span>
    `;
    button.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      setProjectBundleOutput(`restored ${entry.action} context`);
    });

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";

    const restoreButton = document.createElement("button");
    restoreButton.type = "button";
    restoreButton.className = "desktop-shell-button-ghost";
    restoreButton.textContent = "Restore";
    restoreButton.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      setProjectBundleOutput(`restored ${entry.action} context`);
    });

    const rerunButton = document.createElement("button");
    rerunButton.type = "button";
    rerunButton.className = "desktop-shell-button-primary";
    rerunButton.textContent = "Re-run";
    rerunButton.addEventListener("click", () => {
      restoreProjectActionContext(entry);
      void rerunProjectActionEntry(entry);
    });

    const pinButton = document.createElement("button");
    pinButton.type = "button";
    pinButton.className = entry.pinned ? "desktop-shell-button-primary" : "desktop-shell-button-ghost";
    pinButton.textContent = entry.pinned ? "Pinned" : "Pin";
    pinButton.addEventListener("click", () => {
      togglePinnedProjectAction(entry);
    });

    controls.append(restoreButton);

    if (entry.pinned) {
      const renameButton = document.createElement("button");
      renameButton.type = "button";
      renameButton.className = "desktop-shell-button-ghost";
      renameButton.textContent = "Label";
      renameButton.addEventListener("click", () => {
        renamePinnedProjectAction(entry);
      });
      controls.append(renameButton);

      const copyButton = document.createElement("button");
      copyButton.type = "button";
      copyButton.className = "desktop-shell-button-ghost";
      copyButton.textContent = "Copy CLI";
      copyButton.addEventListener("click", () => {
        void copyProjectCliCommand(entry);
      });
      controls.append(copyButton);

      const pythonButton = document.createElement("button");
      pythonButton.type = "button";
      pythonButton.className = "desktop-shell-button-ghost";
      pythonButton.textContent = "Copy Python";
      pythonButton.addEventListener("click", () => {
        void copyPythonMacroStub(entry);
      });
      controls.append(pythonButton);
    }

    controls.append(pinButton, rerunButton);
    shell.append(button, controls);
    container.appendChild(shell);
  });
}

function renderHistoryFilters() {
  elements.historyFilterButtons.forEach((button) => {
    const isActive = button.dataset.historyFilter === state.historyFilter;
    button.classList.toggle("desktop-shell-button-primary", isActive);
    button.classList.toggle("desktop-shell-button-ghost", !isActive);
    button.setAttribute("aria-pressed", String(isActive));
  });
}

function matchesHistoryFilter(entry) {
  switch (state.historyFilter) {
    case "failed":
      return entry.status === "failed";
    case "inspect":
      return entry.action === "project inspect";
    case "normalize":
      return entry.action === "project normalize";
    case "diff":
      return entry.action === "project diff";
    case "all":
    default:
      return true;
  }
}

function currentFilteredHistoryActions(actions = loadHubRecents().actions ?? []) {
  return actions.filter((entry) => matchesHistoryFilter(entry));
}

function togglePinnedProjectAction(entry) {
  const recents = loadHubRecents();
  const identity = actionIdentity(entry);
  recents.actions = (recents.actions ?? []).map((candidate) => {
    if (actionIdentity(candidate) !== identity) {
      return candidate;
    }

    return {
      ...candidate,
      pinned: !candidate.pinned,
      favoriteLabel: candidate.pinned ? "" : candidate.favoriteLabel,
    };
  });
  saveHubRecents(recents);
  setProjectBundleOutput(`${entry.pinned ? "unpinned" : "pinned"} ${entry.action}`);
}

function renamePinnedProjectAction(entry) {
  const currentLabel = String(entry.favoriteLabel || "");
  const nextLabel = window.prompt("Favorite label", currentLabel || entry.action);
  if (nextLabel === null) {
    return;
  }

  const recents = loadHubRecents();
  const identity = actionIdentity(entry);
  recents.actions = (recents.actions ?? []).map((candidate) => {
    if (actionIdentity(candidate) !== identity) {
      return candidate;
    }

    return {
      ...candidate,
      favoriteLabel: String(nextLabel || "").trim(),
    };
  });
  saveHubRecents(recents);
  setProjectBundleOutput(`updated label for ${entry.action}`);
}

function downloadHubJson(filename, payload) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}

function exportRecentActionHistory() {
  const recents = loadHubRecents();
  const actions = currentFilteredHistoryActions(recents.actions ?? []);
  const payload = {
    exportedAt: new Date().toISOString(),
    filter: state.historyFilter,
    actionCount: actions.length,
    actions,
  };

  downloadHubJson(`kyuubiki-hub-recent-actions-${state.historyFilter}.json`, payload);
  setProjectBundleOutput(`exported ${actions.length} recent actions as JSON`);
}

async function importRecentActionHistory(file) {
  if (!file) {
    return;
  }

  const raw = await file.text();
  const parsed = JSON.parse(raw);
  const importedActions = Array.isArray(parsed?.actions) ? parsed.actions : [];
  const recents = loadHubRecents();
  recents.actions = mergeProjectActionHistory(recents.actions ?? [], importedActions);
  saveHubRecents(recents);
  setProjectBundleOutput(`imported ${recents.actions.length} recent actions from JSON`);
}

function manageRecentActionHistory(mode) {
  const recents = loadHubRecents();

  switch (mode) {
    case "keep-failed":
      recents.actions = (recents.actions ?? []).filter((entry) => entry.status === "failed");
      saveHubRecents(recents);
      setProjectBundleOutput("kept failed recent actions only");
      return;
    case "import-json":
      elements.historyImportInput?.click();
      return;
    case "clear":
      recents.actions = [];
      saveHubRecents(recents);
      setProjectBundleOutput("cleared recent action history");
      return;
    case "export-json":
      exportRecentActionHistory();
      return;
    default:
      return;
  }
}

function restoreProjectActionContext(entry) {
  elements.projectBundlePath.value = entry.bundlePath || "";
  elements.projectBundleComparePath.value = entry.comparePath || "";
  elements.projectBundleOutPath.value = entry.outputPath || "";
}

async function rerunProjectActionEntry(entry) {
  const action = PROJECT_ACTION_LABELS[entry.action];
  if (!action) {
    setProjectBundleOutput(`cannot re-run unknown action: ${entry.action}`);
    return;
  }

  await runAction(action);
}

async function runProjectBundleAction({ action, command, payload, outputTarget, successOutput }) {
  const executedAt = new Date().toISOString();

  try {
    const result = await invokeTauri(command, { payload });
    saveProjectBundleRecents({
      action,
      bundlePath: elements.projectBundlePath?.value,
      comparePath: elements.projectBundleComparePath?.value,
      outputPath: elements.projectBundleOutPath?.value,
      status: "ok",
      note: result,
      executedAt,
    });
    outputTarget(result);
    setBusy(false, "ready");
  } catch (error) {
    const message = String(error);
    saveProjectBundleRecents({
      action,
      bundlePath: elements.projectBundlePath?.value,
      comparePath: elements.projectBundleComparePath?.value,
      outputPath: elements.projectBundleOutPath?.value,
      status: "failed",
      note: message,
      executedAt,
    });
    outputTarget(message);
    setBusy(false, "failed");
  }
}

async function applyBrand() {
  const brand = await loadDesktopBrand();
  if (!brand) {
    return;
  }

  const releaseVersion = String(brand.releaseVersion || "").replace(/^v/u, "");
  const releaseCodename = String(brand.releaseCodename || "").trim();
  const releaseTag = [releaseCodename, releaseVersion].filter(Boolean).join(" ");

  if (brand.hubName) {
    state.releaseVersion = releaseVersion;
    state.releaseCodename = releaseCodename;
    document.title = releaseTag ? `${brand.hubName} · ${releaseTag}` : brand.hubName;
  }

  setText("brand-hub-title", brand.hubShortName || "Hub");
  setText("brand-hub-role", brand.shellRoleLabel);
  setText("brand-hub-role-chip", brand.shellRoleLabel);
  setText("brand-hub-focus", brand.shellFocusLabel);
  if (releaseTag) {
    setText("brand-hub-version", releaseTag);
  }
}

function releaseLabel() {
  const releaseTag = [state.releaseCodename, state.releaseVersion].filter(Boolean).join(" ");
  return releaseTag ? `Kyuubiki Hub · ${releaseTag}` : "Kyuubiki Hub";
}

function formatRuntimeReport(value) {
  const body = String(value || "").trim();
  return body ? `${releaseLabel()}\n\n${body}` : releaseLabel();
}

function setSection(section) {
  const next = sectionModel[section];
  if (!next) return;

  state.activeSection = section;
  elements.title.textContent = next.title;
  elements.copy.textContent = next.copy;

  elements.navItems.forEach((item) => {
    const active = item.dataset.target === section;
    item.classList.toggle("hub-nav__item--active", active);
    item.setAttribute("aria-current", active ? "page" : "false");
  });

  elements.panels.forEach((panel) => {
    const hidden = panel.id !== `${section}-panel`;
    panel.classList.toggle("hidden", hidden);
    panel.setAttribute("aria-hidden", String(hidden));
  });

  const defaultProjectsPanel = document.getElementById("projects-panel");
  if (defaultProjectsPanel) {
    defaultProjectsPanel.classList.toggle("hidden", section !== "projects");
  }
  if (section === "projects") {
    renderProjectsPages();
  } else if (section in state.panelPages) {
    renderPanelPages(section);
  }

  renderAssistantContext();
  renderHubAssistantLocalCards();
  syncHotRuntimeLogPolling();
  syncObserveRuntimeLogPolling();
  if (section === "runtimes") {
    void refreshHotRuntimeLog({ silent: true });
  }
  if (section === "observe") {
    void refreshObserveRuntimeLog({ silent: true });
  }
}

function enhanceHubAccessibility() {
  elements.title?.setAttribute("tabindex", "-1");

  elements.navItems.forEach((item) => {
    const target = item.dataset.target || "";
    item.setAttribute("aria-controls", `${target}-panel`);
  });

  elements.sectionJumpButtons.forEach((button) => {
    const target = button.dataset.targetSection || "";
    button.setAttribute("aria-controls", `${target}-panel`);
  });

  elements.projectsPageButtons.forEach((button) => {
    const target = button.dataset.projectsPage || "";
    const pane = elements.projectsPanes.find((candidate) => candidate.dataset.projectsPane === target);
    if (!pane) {
      return;
    }

    if (!pane.id) {
      pane.id = `projects-pane-${target}`;
    }
    button.setAttribute("aria-controls", pane.id);
  });

  elements.panelPageButtons.forEach((button) => {
    const group = button.dataset.panelPageGroup || "";
    const target = button.dataset.panelPage || "";
    const pane = elements.panelPanes.find(
      (candidate) => candidate.dataset.panelPaneGroup === group && candidate.dataset.panelPane === target,
    );
    if (!pane) {
      return;
    }

    if (!pane.id) {
      pane.id = `panel-pane-${group}-${target}`;
    }
    button.setAttribute("aria-controls", pane.id);
  });

  elements.densityToggleButtons.forEach((button) => {
    const densityId = button.dataset.densityToggle || "";
    const panel = elements.densityPanels.find((candidate) => candidate.dataset.densityPanel === densityId);
    if (!panel) {
      return;
    }

    if (!panel.id) {
      panel.id = `density-panel-${densityId}`;
    }
    button.setAttribute("aria-controls", panel.id);
  });
}

function setOperationOutput(value) {
  elements.operationOutput.textContent = value;
}

function setDesktopStatusOutput(value) {
  if (elements.desktopStatusOutput) {
    elements.desktopStatusOutput.textContent = formatRuntimeReport(value);
  }
}

function setRuntimeStatusOutput(value) {
  elements.runtimeStatusOutput.textContent = formatRuntimeReport(value);
  if (elements.observeRuntimeStatusOutput) {
    elements.observeRuntimeStatusOutput.textContent = formatRuntimeReport(value);
  }
}

function setHotRuntimeStatusOutput(value) {
  if (elements.hotRuntimeStatusOutput) {
    elements.hotRuntimeStatusOutput.textContent = formatRuntimeReport(value);
  }
  if (elements.observeRuntimeStatusOutput) {
    elements.observeRuntimeStatusOutput.textContent = formatRuntimeReport(value);
  }
}

function setHotRuntimeLogOutput(value) {
  if (elements.hotRuntimeLogOutput) {
    elements.hotRuntimeLogOutput.textContent = value;
  }
  if (elements.observeHotLogOutput) {
    elements.observeHotLogOutput.textContent = value;
  }
}

function setObserveRuntimeLogOutput(value) {
  if (elements.observeRuntimeLogOutput) {
    elements.observeRuntimeLogOutput.textContent = value;
  }
}

function clearHotRuntimeLogView() {
  setHotRuntimeLogOutput(`Cleared local log view for ${currentHotRuntimeLogService()}. Background tail and log files are unchanged.`);
}

function sanitizeRuntimeLogForClipboard(text) {
  return String(text || "")
    .replace(/(authorization\s*[:=]\s*)(bearer\s+)?([^\s]+)/giu, "$1[redacted]")
    .replace(/(api[_-]?key\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(token\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(password\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(secret\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]");
}

async function copyHotRuntimeLogView() {
  const text = sanitizeRuntimeLogForClipboard(
    String(elements.hotRuntimeLogOutput?.textContent || "").trim(),
  );
  await navigator.clipboard.writeText(text);
}

function renderHotRuntimeLogFollowState() {
  const label = shouldPollHotRuntimeLog() ? "following" : "frozen";
  applyDesktopState(elements.hotRuntimeLogFollowState, label, { kind: "activity" });
  applyDesktopState(elements.observeHotFollowState, label, { kind: "activity" });
}

function renderObserveRuntimeLogFollowState() {
  const label = shouldPollObserveRuntimeLog() ? "following" : "frozen";
  applyDesktopState(elements.observeRuntimeLogFollowState, label, { kind: "activity" });
}

function inferHotRuntimeState(rendered) {
  const text = String(rendered || "");
  const running = /hot-loop:\s+running/i.test(text);
  const stopped = /hot-loop:\s+stopped/i.test(text);
  const modeMatch =
    /started managed hot-reload loop \((cloud|distributed|local)\)/i.exec(text)
    || /Mode\W*(cloud|distributed|local)/i.exec(text);

  return {
    status: running ? "running" : stopped ? "idle" : "unknown",
    mode: modeMatch?.[1] || elements.hotRuntimeMode?.textContent?.trim() || "local",
  };
}

function currentHotRuntimeStatus() {
  return String(elements.hotRuntimeStatus?.textContent || "").trim().toLowerCase();
}

function currentHotRuntimeLogService() {
  return elements.hotRuntimeLogService?.value || "hot-stack";
}

function currentObserveRuntimeLogService() {
  return elements.observeRuntimeLogService?.value || "frontend";
}

function renderHotRuntimeLogServiceLabel() {
  const label = currentHotRuntimeLogService();
  if (elements.observeHotLogService) {
    elements.observeHotLogService.textContent = label;
  }
}

function currentHotRuntimeLogAutoRefresh() {
  return elements.hotRuntimeLogAuto?.checked !== false;
}

function currentObserveRuntimeLogAutoRefresh() {
  return elements.observeRuntimeLogAuto?.checked !== false;
}

function currentHotRuntimeLogInterval() {
  const value = String(elements.hotRuntimeLogInterval?.value || "4000");
  return ["2000", "4000", "8000"].includes(value) ? Number(value) : 4000;
}

function persistCurrentHotLogSettings() {
  persistHubHotLogSettings({
    service: currentHotRuntimeLogService(),
    autoRefresh: currentHotRuntimeLogAutoRefresh(),
    interval: String(currentHotRuntimeLogInterval()),
  });
}

function persistCurrentObserveRuntimeLogSettings() {
  persistHubRuntimeLogSettings({
    service: currentObserveRuntimeLogService(),
    autoRefresh: currentObserveRuntimeLogAutoRefresh(),
  });
}

function shouldPollHotRuntimeLog() {
  return state.activeSection === "runtimes"
    && currentHotRuntimeStatus() === "running"
    && currentHotRuntimeLogAutoRefresh();
}

function shouldPollObserveRuntimeLog() {
  return state.activeSection === "observe" && currentObserveRuntimeLogAutoRefresh();
}

function stopHotRuntimeLogPolling() {
  if (hotRuntimeLogPollHandle) {
    window.clearInterval(hotRuntimeLogPollHandle);
    hotRuntimeLogPollHandle = null;
  }
  renderHotRuntimeLogFollowState();
}

function stopObserveRuntimeLogPolling() {
  if (observeRuntimeLogPollHandle) {
    window.clearInterval(observeRuntimeLogPollHandle);
    observeRuntimeLogPollHandle = null;
  }
  renderObserveRuntimeLogFollowState();
}

function syncHotRuntimeLogPolling() {
  if (!shouldPollHotRuntimeLog()) {
    stopHotRuntimeLogPolling();
    return;
  }

  if (hotRuntimeLogPollHandle) {
    renderHotRuntimeLogFollowState();
    return;
  }

  hotRuntimeLogPollHandle = window.setInterval(() => {
    void refreshHotRuntimeLog({ silent: true });
  }, currentHotRuntimeLogInterval() || HUB_HOT_LOG_POLL_MS);
  renderHotRuntimeLogFollowState();
}

function syncObserveRuntimeLogPolling() {
  if (!shouldPollObserveRuntimeLog()) {
    stopObserveRuntimeLogPolling();
    return;
  }

  if (observeRuntimeLogPollHandle) {
    renderObserveRuntimeLogFollowState();
    return;
  }

  observeRuntimeLogPollHandle = window.setInterval(() => {
    void refreshObserveRuntimeLog({ silent: true });
  }, HUB_HOT_LOG_POLL_MS);
  renderObserveRuntimeLogFollowState();
}

function setProjectBundleOutput(value) {
  elements.projectBundleOutput.textContent = value;
}

function setAssistantOutput(value) {
  if (elements.assistantOutput) {
    elements.assistantOutput.textContent = value;
  }
}

function renderProjectsPages() {
  elements.projectsPageButtons.forEach((button) => {
    const active = button.dataset.projectsPage === state.projectsPage;
    button.classList.toggle("hub-panel-tab--active", active);
    button.setAttribute("aria-pressed", String(active));
  });

  elements.projectsPanes.forEach((pane) => {
    const active = pane.dataset.projectsPane === state.projectsPage;
    pane.classList.toggle("hidden", !active);
    pane.setAttribute("aria-hidden", String(!active));
  });
}

function setProjectsPage(page) {
  state.projectsPage = page === "library" || page === "bundles" ? page : "start";
  renderProjectsPages();
}

function renderPanelPages(group) {
  const activePage = state.panelPages[group];
  elements.panelPageButtons
    .filter((button) => button.dataset.panelPageGroup === group)
    .forEach((button) => {
      const active = button.dataset.panelPage === activePage;
      button.classList.toggle("hub-panel-tab--active", active);
      button.setAttribute("aria-pressed", String(active));
    });

  elements.panelPanes
    .filter((pane) => pane.dataset.panelPaneGroup === group)
    .forEach((pane) => {
      const active = pane.dataset.panelPane === activePage;
      pane.classList.toggle("hidden", !active);
      pane.setAttribute("aria-hidden", String(!active));
    });
}

function setPanelPage(group, page) {
  if (!(group in state.panelPages)) {
    return;
  }
  state.panelPages[group] = page || state.panelPages[group];
  renderPanelPages(group);
}

function currentProjectBundlePayload() {
  return { path: elements.projectBundlePath?.value || "" };
}

function currentProjectBundleOutputPayload() {
  return {
    path: elements.projectBundlePath?.value || "",
    out: elements.projectBundleOutPath?.value || "",
  };
}

function currentProjectBundleComparePayload() {
  return {
    leftPath: elements.projectBundlePath?.value || "",
    rightPath: elements.projectBundleComparePath?.value || "",
  };
}

function currentAssistantSnapshot() {
  return {
    activeSection: state.activeSection,
    runtimeStatus: elements.localRuntimeStatus?.textContent?.trim() || "unknown",
    profile: elements.currentProfile?.textContent?.trim() || "unknown",
    bundlePath: elements.projectBundlePath?.value?.trim() || "",
    comparePath: elements.projectBundleComparePath?.value?.trim() || "",
    outputPath: elements.projectBundleOutPath?.value?.trim() || "",
    favorites: loadHubRecents().actions?.filter((entry) => entry.pinned).length ?? 0,
  };
}

function renderAssistantContext() {
  const snapshot = currentAssistantSnapshot();
  setText(elements.assistantContextSection, snapshot.activeSection);
  setText(elements.assistantContextRuntime, snapshot.runtimeStatus);
  setText(elements.assistantContextBundle, snapshot.bundlePath || "--");
}

function setAssistantMode(mode) {
  state.assistantMode = mode === "llm" ? "llm" : "local";
  elements.assistantModeButtons.forEach((button) => {
    const active = button.dataset.assistantMode === state.assistantMode;
    button.classList.toggle("desktop-shell-button-primary", active);
    button.classList.toggle("desktop-shell-button-ghost", !active);
    button.setAttribute("aria-pressed", String(active));
  });
  elements.assistantLocalPanel?.classList.toggle("hidden", state.assistantMode !== "local");
  elements.assistantLlmPanel?.classList.toggle("hidden", state.assistantMode !== "llm");
  applyDesktopState(elements.assistantEngineState, state.assistantMode === "llm" ? "remote model" : "local guide", {
    kind: "activity",
  });
  persistHubAssistantSettings({
    ...loadHubAssistantSettings(),
    mode: state.assistantMode,
    baseUrl: elements.assistantBaseUrl?.value || "",
    modelPreset: elements.assistantModelPreset?.value || "gpt-5",
    model: elements.assistantModelName?.value || "gpt-5",
  });
}

function renderHubDensityToggles() {
  elements.densityPanels.forEach((panel) => {
    const densityId = panel.dataset.densityPanel || "";
    const expanded = state.density[densityId] !== false;
    panel.classList.toggle("hidden", !expanded);
  });

  elements.densityToggleButtons.forEach((button) => {
    const densityId = button.dataset.densityToggle || "";
    const expanded = state.density[densityId] !== false;
    button.textContent = expanded ? "Collapse" : "Expand";
    button.setAttribute("aria-expanded", String(expanded));
  });
}

function toggleHubDensityPanel(id) {
  if (!(id in HUB_DENSITY_DEFAULTS)) {
    return;
  }

  state.density[id] = !(state.density[id] !== false);
  persistHubDensitySettings();
  renderHubDensityToggles();
}

function buildHubAssistantLocalCards() {
  const snapshot = currentAssistantSnapshot();
  const cards = [];

  if (!snapshot.bundlePath) {
    cards.push({
      id: "bundle-path",
      title: "Start with a bundle path",
      summary: "Paste a .kyuubiki path first so the Hub can inspect, validate, or normalize it safely.",
      actionLabel: "Open Bundle tools",
      tone: "watch",
      onAction: () => {
        setSection("projects");
        setProjectsPage("bundles");
        elements.projectBundlePath?.focus();
        setProjectBundleOutput("focused the bundle path field");
      },
    });
  }

  if (!/ready|healthy/i.test(snapshot.runtimeStatus)) {
    cards.push({
      id: "start-local",
      title: "Bring the local stack online",
      summary: "The Hub does not currently see a healthy local runtime, so starting the local stack is the safest next step.",
      actionLabel: "Start local stack",
      tone: "risk",
      onAction: () => {
        void runAction("start-local");
      },
    });
  }

  if (snapshot.bundlePath) {
    cards.push({
      id: "inspect-bundle",
      title: "Inspect the selected bundle",
      summary: "Inspecting first gives a quick structural read before we normalize, unpack, or diff anything.",
      actionLabel: "Inspect bundle",
      tone: "good",
      onAction: () => {
        void runAction("project-inspect");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.outputPath) {
    cards.push({
      id: "normalize-bundle",
      title: "Normalize into the target path",
      summary: "You already have both the source and output path, so normalization is ready to run.",
      actionLabel: "Normalize bundle",
      tone: "good",
      onAction: () => {
        void runAction("project-normalize");
      },
    });
  }

  if (snapshot.bundlePath && snapshot.comparePath) {
    cards.push({
      id: "diff-bundles",
      title: "Compare the current pair",
      summary: "Both bundle inputs are present, so the Hub can run a safe diff without more setup.",
      actionLabel: "Diff bundles",
      tone: "watch",
      onAction: () => {
        void runAction("project-diff");
      },
    });
  }

  cards.push({
    id: "open-workbench",
    title: "Jump into Workbench",
    summary: "Open the modeling and analysis surface when you are ready to move past bundle-level prep.",
    actionLabel: "Open Workbench",
    tone: "good",
    onAction: () => {
      void runAction("open-workbench");
    },
  });

  return cards.slice(0, 5);
}

function renderHubAssistantLocalCards() {
  if (!elements.assistantLocalCards) {
    return;
  }

  const cards = buildHubAssistantLocalCards();
  elements.assistantLocalCards.innerHTML = "";
  if (!cards.length) {
    renderEmptyHistoryState(elements.assistantLocalCards, "The local guide does not see an urgent next step right now.");
    return;
  }

  cards.forEach((card) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    appendAssistantCardHeader(
      article,
      card.title,
      card.tone,
      `desktop-shell-state desktop-shell-state--${
        card.tone === "risk" ? "danger" : card.tone === "watch" ? "warning" : "healthy"
      }`,
    );
    appendTextElement(article, "p", card.summary, "desktop-shell-note");
    const buttonRow = document.createElement("div");
    buttonRow.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = card.actionLabel;
    button.addEventListener("click", card.onAction);
    buttonRow.appendChild(button);
    article.appendChild(buttonRow);
    elements.assistantLocalCards.appendChild(article);
  });
}

function extractAssistantJsonBlock(value) {
  const fenced = value.match(/```json\s*([\s\S]*?)```/i);
  if (fenced?.[1]) {
    return fenced[1].trim();
  }

  const firstBrace = value.indexOf("{");
  const lastBrace = value.lastIndexOf("}");
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    return value.slice(firstBrace, lastBrace + 1);
  }

  return value.trim();
}

function validateAssistantBaseUrl(value) {
  const baseUrl = value.trim();
  if (!baseUrl) {
    return { ok: false, reason: "Fill in the assistant base URL before requesting a plan." };
  }

  let parsed;
  try {
    parsed = new URL(baseUrl);
  } catch {
    return { ok: false, reason: "Assistant base URL must be a valid absolute URL." };
  }

  const protocol = parsed.protocol.toLowerCase();
  const hostname = parsed.hostname.toLowerCase();
  const isLoopback =
    hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";

  if (protocol === "https:") {
    return { ok: true, normalized: baseUrl };
  }

  if (protocol === "http:" && isLoopback) {
    return { ok: true, normalized: baseUrl };
  }

  return {
    ok: false,
    reason: "Assistant base URL must use https, or http only for localhost / 127.0.0.1 / ::1.",
  };
}

function updateAssistantEndpointPolicy() {
  if (!elements.assistantEndpointPolicy || !elements.assistantBaseUrl) {
    return;
  }

  const baseUrl = elements.assistantBaseUrl.value.trim();
  if (!baseUrl) {
    elements.assistantEndpointPolicy.textContent =
      "Use https:// for remote providers, or http://localhost / 127.0.0.1 for local gateways. The API key is sent directly to the configured base URL.";
    return;
  }

  const validation = validateAssistantBaseUrl(baseUrl);
  if (!validation.ok) {
    elements.assistantEndpointPolicy.textContent = `${validation.reason} The API key is sent directly to the configured base URL.`;
    return;
  }

  elements.assistantEndpointPolicy.textContent =
    "Assistant endpoint looks allowed. The API key is sent directly to the configured base URL for plan generation.";
}

async function requestHubAssistantPlan() {
  const baseUrl = elements.assistantBaseUrl?.value?.trim() || "";
  const model = elements.assistantModelName?.value?.trim() || "";
  const prompt = elements.assistantPrompt?.value?.trim() || "";
  const apiKey = elements.assistantApiKey?.value?.trim() || "";
  const baseUrlValidation = validateAssistantBaseUrl(baseUrl);

  if (!baseUrlValidation.ok || !model) {
    throw new Error(baseUrlValidation.reason || "Fill in the assistant base URL and model before requesting a plan.");
  }

  const response = await fetch(`${baseUrlValidation.normalized.replace(/\/+$/, "")}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
    },
    body: JSON.stringify({
      model,
      temperature: 0.2,
      response_format: { type: "json_object" },
      messages: [
        {
          role: "system",
          content:
            "You are the Kyuubiki Hub assistant. Return strict JSON with keys summary, rationale, suggested_actions. suggested_actions must be an array of objects with action, payload, reason. Only suggest actions from the provided Hub action catalog. Keep it concise, safe, and onboarding-oriented.",
        },
        {
          role: "user",
          content: JSON.stringify(
            {
              prompt,
              snapshot: currentAssistantSnapshot(),
              action_catalog: HUB_ASSISTANT_ACTIONS,
              local_hints: buildHubAssistantLocalCards().map((card) => ({
                id: card.id,
                title: card.title,
                summary: card.summary,
                actionLabel: card.actionLabel,
              })),
            },
            null,
            2,
          ),
        },
      ],
    }),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`assistant request failed (${response.status}): ${body.slice(0, 240)}`);
  }

  const payload = await response.json();
  const content = payload?.choices?.[0]?.message?.content;
  if (!content) {
    throw new Error("assistant response did not include a message body");
  }

  const parsed = JSON.parse(extractAssistantJsonBlock(content));
  return {
    summary: String(parsed?.summary || ""),
    rationale: String(parsed?.rationale || ""),
    suggested_actions: Array.isArray(parsed?.suggested_actions)
      ? parsed.suggested_actions.map((entry) => ({
          action: String(entry?.action || ""),
          payload: entry && typeof entry.payload === "object" && entry.payload ? entry.payload : {},
          reason: String(entry?.reason || ""),
        }))
      : [],
  };
}

function renderHubAssistantPlan() {
  if (!elements.assistantPlanActions) {
    return;
  }

  const plan = state.assistantPlan;
  elements.assistantPlanActions.innerHTML = "";
  if (!plan) {
    renderEmptyHistoryState(elements.assistantPlanActions, "No model plan yet.");
    return;
  }

  const summaryCard = document.createElement("article");
  summaryCard.className = "hub-list__card";
  appendAssistantCardHeader(summaryCard, plan.summary || "Model plan", `${plan.suggested_actions.length} actions`);
  appendTextElement(
    summaryCard,
    "p",
    plan.rationale || "The connected model returned a concise operational plan.",
    "desktop-shell-note",
  );
  elements.assistantPlanActions.appendChild(summaryCard);

  if (!plan.suggested_actions.length) {
    renderEmptyHistoryState(elements.assistantPlanActions, "The model returned no executable Hub actions.");
    return;
  }

  plan.suggested_actions.forEach((entry) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    appendAssistantCardHeader(
      article,
      entry.action,
      assistantRiskLevel(entry.action),
      assistantRiskStateClass(assistantRiskLevel(entry.action)),
    );
    appendTextElement(article, "p", entry.reason || "No rationale supplied.", "desktop-shell-note");
    appendTextElement(article, "code", JSON.stringify(entry.payload || {}, null, 2));
    const row = document.createElement("div");
    row.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = "Run action";
    button.addEventListener("click", () => {
      void executeHubAssistantAction(entry.action, entry.payload || {});
    });
    row.appendChild(button);
    article.appendChild(row);
    elements.assistantPlanActions.appendChild(article);
  });
}

function confirmHubAssistantAction(action, source = "assistant") {
  const risk = assistantRiskLevel(action);
  if (risk === "low") {
    return true;
  }

  const note = source === "plan" ? "model plan action" : "assistant action";
  rememberHubAssistantAudit({ action, risk, status: "prompted", source, note });
  const message =
    risk === "high"
      ? `High-risk ${note}: ${action}\n\nThis may launch builds or rewrite bundle outputs.\n\nContinue?`
      : `Sensitive ${note}: ${action}\n\nPlease confirm before the Hub continues.\n\nContinue?`;
  const approved = window.confirm(message);
  rememberHubAssistantAudit({
    action,
    risk,
    status: approved ? "confirmed" : "cancelled",
    source,
    note,
  });
  return approved;
}

function applyAssistantBundlePayload(payload) {
  if (typeof payload?.path === "string") {
    elements.projectBundlePath.value = payload.path;
  }
  if (typeof payload?.comparePath === "string" || typeof payload?.rightPath === "string") {
    elements.projectBundleComparePath.value = String(payload.comparePath ?? payload.rightPath ?? "");
  }
  if (typeof payload?.out === "string") {
    elements.projectBundleOutPath.value = payload.out;
  }
}

async function executeHubAssistantAction(action, payload = {}, source = "assistant") {
  const risk = assistantRiskLevel(action);
  if (!confirmHubAssistantAction(action, source)) {
    setAssistantOutput(`Cancelled ${action}.`);
    return;
  }

  switch (action) {
    case "hub/focusSection":
      setSection(typeof payload.section === "string" ? payload.section : "projects");
      setAssistantOutput(`Focused ${typeof payload.section === "string" ? payload.section : "projects"} section.`);
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "focused Hub section" });
      return;
    case "hub/openWorkbench":
      await runAction("open-workbench");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Workbench shell" });
      return;
    case "hub/openInstaller":
      await runAction("open-installer");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer shell" });
      return;
    case "hub/startLocal":
      await runAction("start-local");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "started local stack" });
      return;
    case "hub/validateEnv":
      await runAction("validate-env");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated environment" });
      return;
    case "hub/desktopStage":
      await runAction("desktop-stage");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "staged desktop release" });
      return;
    case "hub/desktopBuildHost":
      await runAction("desktop-build-host");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "built host desktop bundles" });
      return;
    case "hub/desktopVerify":
      await runAction("desktop-verify");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "verified desktop release" });
      return;
    case "hub/setBundleContext":
      applyAssistantBundlePayload(payload);
      renderAssistantContext();
      setAssistantOutput("Updated bundle context in the Hub.");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "updated bundle inputs" });
      return;
    case "hub/projectInspect":
      applyAssistantBundlePayload(payload);
      await runAction("project-inspect");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "inspected project bundle" });
      return;
    case "hub/projectValidate":
      applyAssistantBundlePayload(payload);
      await runAction("project-validate");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated project bundle" });
      return;
    case "hub/projectNormalize":
      applyAssistantBundlePayload(payload);
      await runAction("project-normalize");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "normalized project bundle" });
      return;
    case "hub/projectUnpack":
      applyAssistantBundlePayload(payload);
      await runAction("project-unpack");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "unpacked project bundle" });
      return;
    case "hub/projectPack":
      applyAssistantBundlePayload(payload);
      await runAction("project-pack");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "packed project bundle" });
      return;
    case "hub/projectDiff":
      applyAssistantBundlePayload(payload);
      await runAction("project-diff");
      rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "diffed project bundles" });
      return;
    default:
      rememberHubAssistantAudit({ action, risk, status: "failed", source, note: "unknown assistant action" });
      throw new Error(`Unknown assistant action: ${action}`);
  }
}

async function executeHubAssistantPlan() {
  if (!state.assistantPlan?.suggested_actions?.length) {
    setAssistantOutput("No assistant plan is available to execute.");
    return;
  }

  if (!elements.assistantApprovePlan?.checked) {
    setAssistantOutput("Review the generated plan and confirm execution first.");
    return;
  }

  for (const entry of state.assistantPlan.suggested_actions) {
    try {
      await executeHubAssistantAction(entry.action, entry.payload || {}, "plan");
    } catch (error) {
      rememberHubAssistantAudit({
        action: entry.action,
        risk: assistantRiskLevel(entry.action),
        status: "failed",
        source: "plan",
        note: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }
  setAssistantOutput(`Executed ${state.assistantPlan.suggested_actions.length} assistant actions.`);
}

function setBusy(isBusy, label = "idle") {
  state.isBusy = isBusy;
  applyDesktopState(elements.actionState, label, { kind: "activity" });
  elements.actionButtons.forEach((button) => {
    button.disabled = isBusy;
    button.classList.toggle("is-busy", isBusy);
  });
}

function syncAssistantSettingsFromInputs() {
  persistHubAssistantSettings({
    mode: state.assistantMode,
    baseUrl: elements.assistantBaseUrl?.value || "",
    modelPreset: elements.assistantModelPreset?.value || "gpt-5",
    model: elements.assistantModelName?.value || "gpt-5",
  });
  persistHubAssistantSecrets({
    apiKey: elements.assistantApiKey?.value || "",
  });
}

function applyAssistantSettings() {
  const settings = loadHubAssistantSettings();
  const secrets = loadHubAssistantSecrets();
  state.assistantMode = settings.mode;
  if (elements.assistantBaseUrl) {
    elements.assistantBaseUrl.value = settings.baseUrl;
  }
  if (elements.assistantModelPreset) {
    elements.assistantModelPreset.value = settings.modelPreset;
  }
  if (elements.assistantModelName) {
    elements.assistantModelName.value = settings.model;
  }
  if (elements.assistantApiKey) {
    elements.assistantApiKey.value = secrets.apiKey;
  }
  setAssistantMode(settings.mode);
  updateAssistantEndpointPolicy();
  renderAssistantContext();
  renderHubAssistantLocalCards();
  renderHubAssistantPlan();
  renderHubAssistantAudit();
}

async function loadEnvironment() {
  const environment = await invokeTauri("hub_environment");
  state.hostPlatform = environment.host_platform;

  if (elements.releasePlatform && !elements.releasePlatform.value) {
    elements.releasePlatform.value = environment.host_platform;
  }

  if (elements.workbenchUrl) {
    elements.workbenchUrl.textContent = environment.workbench_url;
  }

  if (elements.orchestratorUrl) {
    elements.orchestratorUrl.textContent = environment.orchestrator_url;
  }

  ensureDefaultWorkloadCatalogUrl();

  applyDesktopState(elements.currentRuntimeMode, "orchestrated_gui", { kind: "activity" });
  applyDesktopState(elements.currentProfile, environment.deployment_mode, { kind: "activity" });
  renderAssistantContext();
}

async function refreshRuntimeStatus() {
  try {
    const payload = await invokeTauri("service_status");
    setRuntimeStatusOutput(payload.rendered);
    applyDesktopState(elements.localRuntimeStatus, payload.rendered, { kind: "health" });
    applyDesktopState(elements.observeRuntimeStatus, payload.rendered, { kind: "health" });
  } catch (error) {
    const message = String(error);
    setRuntimeStatusOutput(message);
    applyDesktopState(elements.localRuntimeStatus, message, { kind: "health" });
    applyDesktopState(elements.observeRuntimeStatus, message, { kind: "health" });
  }
  renderAssistantContext();
  renderHubAssistantLocalCards();
}

async function refreshHotRuntimeStatus() {
  try {
    const payload = await invokeTauri("hot_service_status");
    setHotRuntimeStatusOutput(payload.rendered);
    const inferred = inferHotRuntimeState(payload.rendered);
    applyDesktopState(elements.hotRuntimeStatus, inferred.status, { kind: "activity" });
    applyDesktopState(elements.observeHotStatus, inferred.status, { kind: "activity" });
    if (elements.hotRuntimeMode) {
      elements.hotRuntimeMode.textContent = inferred.mode;
    }
    if (elements.observeHotMode) {
      elements.observeHotMode.textContent = inferred.mode;
    }
    syncHotRuntimeLogPolling();
    await refreshHotRuntimeLog({ silent: true });
  } catch (error) {
    const message = String(error);
    setHotRuntimeStatusOutput(message);
    applyDesktopState(elements.hotRuntimeStatus, "failed", { kind: "activity" });
    applyDesktopState(elements.observeHotStatus, "failed", { kind: "activity" });
    syncHotRuntimeLogPolling();
  }
}

async function refreshHotRuntimeLog(options = {}) {
  const silent = options?.silent === true;
  const service = currentHotRuntimeLogService();

  if (state.hotLogRefreshInFlight) {
    return;
  }

  state.hotLogRefreshInFlight = true;

  try {
    const payload = await invokeTauri("read_runtime_log", {
      payload: { service },
    });
    const rendered = String(payload?.rendered || "").trim();
    setHotRuntimeLogOutput(rendered || `No log lines yet for ${service}.`);
  } catch (error) {
    if (!silent) {
      setHotRuntimeLogOutput(formatHubOperatorError(error, {
        actionLabel: "Reading runtime logs",
        context: "log-read",
        service,
      }));
    }
  } finally {
    state.hotLogRefreshInFlight = false;
  }
}

async function refreshObserveRuntimeLog(options = {}) {
  const silent = options?.silent === true;
  const service = currentObserveRuntimeLogService();

  if (state.runtimeLogRefreshInFlight) {
    return;
  }

  state.runtimeLogRefreshInFlight = true;

  try {
    const payload = await invokeTauri("read_runtime_log", {
      payload: { service },
    });
    const rendered = String(payload?.rendered || "").trim();
    setObserveRuntimeLogOutput(rendered || `No log lines yet for ${service}.`);
  } catch (error) {
    if (!silent) {
      setObserveRuntimeLogOutput(formatHubOperatorError(error, {
        actionLabel: "Reading runtime logs",
        context: "log-read",
        service,
      }));
    }
  } finally {
    state.runtimeLogRefreshInFlight = false;
  }
}

async function copyObserveRuntimeLogView() {
  const text = sanitizeRuntimeLogForClipboard(
    String(elements.observeRuntimeLogOutput?.textContent || "").trim(),
  );
  await navigator.clipboard.writeText(text);
}

async function refreshDesktopStatusOutput() {
  try {
    setDesktopStatusOutput(
      await invokeTauri("desktop_status", {
        payload: { platform: elements.releasePlatform?.value || state.hostPlatform },
      }),
    );
  } catch (error) {
    setDesktopStatusOutput(formatHubOperatorError(error, {
      actionLabel: "Refreshing desktop packaging status",
      context: "desktop-status",
    }));
  }
}

async function runAction(action) {
  if (state.isBusy) {
    return;
  }

  setBusy(true, "running");

  try {
    switch (action) {
      case "open-workbench":
        setOperationOutput(await invokeTauri("launch_workbench_gui"));
        setSection("projects");
        setBusy(false, "ready");
        return;
      case "open-installer":
        setOperationOutput(await invokeTauri("launch_installer_gui"));
        setSection("deploy");
        setBusy(false, "ready");
        return;
      case "project-inspect":
        await runProjectBundleAction({
          action: "project inspect",
          command: "project_bundle_inspect",
          payload: currentProjectBundlePayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-validate":
        await runProjectBundleAction({
          action: "project validate",
          command: "project_bundle_validate",
          payload: currentProjectBundlePayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-normalize":
        await runProjectBundleAction({
          action: "project normalize",
          command: "project_bundle_normalize",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-unpack":
        await runProjectBundleAction({
          action: "project unpack",
          command: "project_bundle_unpack",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-pack":
        await runProjectBundleAction({
          action: "project pack",
          command: "project_bundle_pack",
          payload: currentProjectBundleOutputPayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "project-diff":
        await runProjectBundleAction({
          action: "project diff",
          command: "project_bundle_diff",
          payload: currentProjectBundleComparePayload(),
          outputTarget: setProjectBundleOutput,
        });
        return;
      case "workload-register-local":
        await registerCurrentBundleAsWorkload();
        setBusy(false, "ready");
        return;
      case "workload-sync-local":
        await syncLocalControlPlaneWorkloads();
        setBusy(false, "ready");
        return;
      case "workload-sync-remote":
        await syncRemoteWorkloadCatalog();
        setBusy(false, "ready");
        return;
      case "workload-export-library":
        exportHubWorkloadLibrary();
        setBusy(false, "ready");
        return;
      case "workload-import-library":
        elements.workloadImportInput?.click();
        setBusy(false, "idle");
        return;
      case "workload-clear-library":
        clearHubWorkloadLibrary();
        setBusy(false, "ready");
        return;
      case "start-local":
        setOperationOutput(await invokeTauri("service_start", { payload: { mode: "local" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-local":
        setOperationOutput(await invokeTauri("hot_service_start", { payload: { mode: "local" } }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-cloud":
        setOperationOutput(await invokeTauri("hot_service_start", { payload: { mode: "cloud" } }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-start-distributed":
        setOperationOutput(await invokeTauri("hot_service_start", { payload: { mode: "distributed" } }));
        await refreshHotRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "hot-refresh-status":
        await refreshHotRuntimeStatus();
        setOperationOutput("refreshed hot-reload runtime status");
        setBusy(false, "ready");
        return;
      case "hot-refresh-log":
        await refreshHotRuntimeLog();
        setOperationOutput(`refreshed hot log: ${elements.hotRuntimeLogService?.value || "hot-stack"}`);
        setBusy(false, "ready");
        return;
      case "hot-copy-log-view":
        await copyHotRuntimeLogView();
        setOperationOutput(`copied sanitized hot log tail: ${elements.hotRuntimeLogService?.value || "hot-stack"}`);
        setBusy(false, "ready");
        return;
      case "observe-refresh-runtime-log":
        await refreshObserveRuntimeLog();
        setOperationOutput(`refreshed runtime log: ${elements.observeRuntimeLogService?.value || "frontend"}`);
        setBusy(false, "ready");
        return;
      case "observe-copy-runtime-log":
        await copyObserveRuntimeLogView();
        setOperationOutput(`copied sanitized runtime log tail: ${elements.observeRuntimeLogService?.value || "frontend"}`);
        setBusy(false, "ready");
        return;
      case "hot-clear-log-view":
        clearHotRuntimeLogView();
        setOperationOutput(`cleared hot log view: ${elements.hotRuntimeLogService?.value || "hot-stack"}`);
        setBusy(false, "idle");
        return;
      case "hot-stop":
        setOperationOutput(await invokeTauri("hot_service_stop"));
        await refreshHotRuntimeStatus();
        setBusy(false, "idle");
        return;
      case "start-cloud":
        setOperationOutput(await invokeTauri("service_start", { payload: { mode: "cloud" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "start-distributed":
        setOperationOutput(await invokeTauri("service_start", { payload: { mode: "distributed" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "restart-local":
        setOperationOutput(await invokeTauri("service_restart", { payload: { mode: "local" } }));
        await refreshRuntimeStatus();
        setBusy(false, "ready");
        return;
      case "stop-stack":
        setOperationOutput(await invokeTauri("service_stop"));
        await refreshRuntimeStatus();
        setBusy(false, "idle");
        return;
      case "validate-env":
        setOperationOutput(await invokeTauri("validate_env"));
        setBusy(false, "ready");
        return;
      case "run-doctor": {
        const payload = await invokeTauri("doctor_report");
        setOperationOutput(payload.rendered);
        setBusy(false, "ready");
        return;
      }
      case "desktop-stage":
        setOperationOutput(
          await invokeTauri("desktop_stage", {
            payload: { platform: elements.releasePlatform?.value || state.hostPlatform },
          }),
        );
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      case "desktop-status":
        await refreshDesktopStatusOutput();
        setOperationOutput("refreshed desktop packaging readiness");
        setBusy(false, "ready");
        return;
      case "desktop-verify":
        setOperationOutput(
          await invokeTauri("desktop_verify", {
            payload: { platform: elements.releasePlatform?.value || state.hostPlatform },
          }),
        );
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      case "desktop-build-host":
        setOperationOutput(await invokeTauri("desktop_build_host"));
        await refreshDesktopStatusOutput();
        setBusy(false, "ready");
        return;
      default:
        setBusy(false, "idle");
        return;
    }
  } catch (error) {
    setOperationOutput(formatHubOperatorError(error, {
      actionLabel: "This desktop action",
    }));
    setBusy(false, "failed");
  }
}

elements.navItems.forEach((item) => {
  item.addEventListener("click", () => setSection(item.dataset.target));
});

elements.projectsPageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setProjectsPage(button.dataset.projectsPage || "start");
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`focused ${button.dataset.projectsPage || "start"} home page`);
  });
});

elements.projectsTargetButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setProjectsPage(button.dataset.projectsTarget || "start");
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`opened ${button.dataset.projectsTarget || "start"} from home`);
  });
});

elements.panelPageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    const group = button.dataset.panelPageGroup || "";
    const page = button.dataset.panelPage || "";
    setPanelPage(group, page);
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`focused ${page} ${group} page`);
  });
});

for (const button of document.querySelectorAll("[data-action]")) {
  button.addEventListener("click", async () => {
    await runAction(button.dataset.action);
  });
}

elements.sectionJumpButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setSection(button.dataset.targetSection);
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(`focused ${button.dataset.targetSection} section`);
  });
});

elements.assistantModeButtons.forEach((button) => {
  button.addEventListener("click", () => {
    setAssistantMode(button.dataset.assistantMode || "local");
  });
});

elements.assistantModelPreset?.addEventListener("change", () => {
  const preset = elements.assistantModelPreset.value;
  if (preset !== "custom" && elements.assistantModelName) {
    elements.assistantModelName.value = preset;
  }
  syncAssistantSettingsFromInputs();
});

elements.assistantBaseUrl?.addEventListener("change", () => {
  syncAssistantSettingsFromInputs();
  updateAssistantEndpointPolicy();
});
elements.assistantBaseUrl?.addEventListener("input", updateAssistantEndpointPolicy);
elements.assistantApiKey?.addEventListener("change", syncAssistantSettingsFromInputs);
elements.assistantModelName?.addEventListener("change", syncAssistantSettingsFromInputs);
elements.releasePlatform?.addEventListener("change", () => {
  void refreshDesktopStatusOutput();
});
elements.hotRuntimeLogService?.addEventListener("change", () => {
  persistCurrentHotLogSettings();
  renderHotRuntimeLogServiceLabel();
  void refreshHotRuntimeLog();
});
elements.hotRuntimeLogAuto?.addEventListener("change", () => {
  persistCurrentHotLogSettings();
  syncHotRuntimeLogPolling();
});
elements.hotRuntimeLogInterval?.addEventListener("change", () => {
  persistCurrentHotLogSettings();
  stopHotRuntimeLogPolling();
  syncHotRuntimeLogPolling();
});
elements.observeRuntimeLogService?.addEventListener("change", () => {
  persistCurrentObserveRuntimeLogSettings();
  void refreshObserveRuntimeLog();
});
elements.observeRuntimeLogAuto?.addEventListener("change", () => {
  persistCurrentObserveRuntimeLogSettings();
  syncObserveRuntimeLogPolling();
});
elements.projectBundlePath?.addEventListener("input", () => {
  renderAssistantContext();
  renderHubAssistantLocalCards();
});
elements.projectBundleComparePath?.addEventListener("input", () => {
  renderAssistantContext();
  renderHubAssistantLocalCards();
});
elements.projectBundleOutPath?.addEventListener("input", () => {
  renderAssistantContext();
  renderHubAssistantLocalCards();
});

elements.assistantRequestPlan?.addEventListener("click", async () => {
  try {
    elements.assistantRequestPlan.disabled = true;
    setAssistantOutput("Planning...");
    syncAssistantSettingsFromInputs();
    state.assistantPlan = await requestHubAssistantPlan();
    elements.assistantApprovePlan.checked = false;
    renderHubAssistantPlan();
    setAssistantOutput(state.assistantPlan.summary || "Generated a Hub assistant plan.");
  } catch (error) {
    setAssistantOutput(formatHubOperatorError(error, {
      actionLabel: "The assistant request",
    }));
  } finally {
    elements.assistantRequestPlan.disabled = false;
  }
});

elements.assistantExecutePlan?.addEventListener("click", async () => {
  try {
    elements.assistantExecutePlan.disabled = true;
    await executeHubAssistantPlan();
  } catch (error) {
    setAssistantOutput(formatHubOperatorError(error, {
      actionLabel: "The assistant plan",
    }));
  } finally {
    elements.assistantExecutePlan.disabled = false;
  }
});

elements.historyFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.historyFilter = button.dataset.historyFilter || "all";
    renderHubRecents();
    setProjectBundleOutput(`filtered recent actions: ${state.historyFilter}`);
  });
});

elements.workloadFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.workloadFilter = button.dataset.workloadFilter || "all";
    renderHubWorkloadLibrary();
    setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
  });
});

elements.workloadFamilyFilterButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.workloadFamilyFilter = button.dataset.workloadFamilyFilter || "all";
    renderHubWorkloadLibrary();
    setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
  });
});

elements.historyManageButtons.forEach((button) => {
  button.addEventListener("click", () => {
    manageRecentActionHistory(button.dataset.historyManage || "");
  });
});

elements.densityToggleButtons.forEach((button) => {
  button.addEventListener("click", () => {
    toggleHubDensityPanel(button.dataset.densityToggle || "");
  });
});

elements.historyImportInput?.addEventListener("change", async (event) => {
  const input = event.currentTarget;
  const file = input?.files?.[0];

  try {
    await importRecentActionHistory(file);
  } catch (error) {
    setProjectBundleOutput(formatHubOperatorError(error, {
      actionLabel: "Importing recent action history",
    }));
  } finally {
    if (input) {
      input.value = "";
    }
  }
});

elements.workloadImportInput?.addEventListener("change", async (event) => {
  const input = event.currentTarget;
  const file = input?.files?.[0];

  try {
    await importHubWorkloadLibrary(file);
  } catch (error) {
    setWorkloadLibraryOutput(formatHubOperatorError(error, {
      actionLabel: "Importing the workload library",
    }));
  } finally {
    if (input) {
      input.value = "";
    }
  }
});

await applyBrand();
await loadEnvironment();
enhanceHubAccessibility();
state.density = loadHubDensitySettings();
const hotLogSettings = loadHubHotLogSettings();
const runtimeLogSettings = loadHubRuntimeLogSettings();
if (elements.hotRuntimeLogService) {
  elements.hotRuntimeLogService.value = hotLogSettings.service;
}
if (elements.hotRuntimeLogAuto) {
  elements.hotRuntimeLogAuto.checked = hotLogSettings.autoRefresh;
}
if (elements.hotRuntimeLogInterval) {
  elements.hotRuntimeLogInterval.value = hotLogSettings.interval;
}
if (elements.observeRuntimeLogService) {
  elements.observeRuntimeLogService.value = runtimeLogSettings.service;
}
if (elements.observeRuntimeLogAuto) {
  elements.observeRuntimeLogAuto.checked = runtimeLogSettings.autoRefresh;
}
renderHotRuntimeLogServiceLabel();
syncDesktopStates();
renderHubDensityToggles();
renderPanelPages("runtimes");
renderPanelPages("observe");
renderPanelPages("tools");
renderHubRecents();
applyAssistantSettings();
setSection(state.activeSection);
setBusy(false, "idle");
await refreshRuntimeStatus();
await refreshHotRuntimeStatus();
await refreshDesktopStatusOutput();
