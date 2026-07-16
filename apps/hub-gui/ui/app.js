import {
  applyDesktopState,
  invokeTauri,
  loadDesktopLanguagePreference,
  normalizeDesktopLanguage,
  saveDesktopLanguagePreference,
  setText,
  syncDesktopStates,
  watchDesktopLanguagePreference,
} from "./shared/tauri-bridge.js";
import { loadDesktopLanguagePack } from "./shared/language-pack-loader.js";
import { createHubWorkflowPanel } from "./hub-workflow-panel.js";
import { createHubAssistantAuditPanel } from "./hub-assistant-audit-panel.js";
import { createHubActionRunner } from "./hub-action-runner.js";
import { createHubAssistantPanel } from "./hub-assistant-panel.js";
import {
  inferDownloadFilename,
  mergeHubWorkloadLibrary as mergeStoredHubWorkloadLibrary,
  normalizeHubWorkloadEntry as normalizeStoredHubWorkloadEntry,
  normalizeRemoteWorkloadCatalogPayload,
  projectSummaryFromInspectPayload,
  validateHubCatalogUrl,
  validateRemoteWorkloadCatalogPayload,
  workloadDomainLabel,
  workloadFamilyLabel,
  workloadIdentity as buildWorkloadIdentity,
  workloadProvenanceHost,
  workloadProvenanceLabel,
  workloadSourceBadge,
} from "./hub-workload-library.js";
import {
  copySanitizedRuntimeLogToClipboard,
  inferHotRuntimeState as inferHotRuntimeStateHelper,
  sanitizeRuntimeLogForClipboard as sanitizeRuntimeLogForClipboardHelper,
} from "./hub-runtime-helpers.js";
import { createHubRuntimeLogController } from "./hub-runtime-log-controller.js";
import { createHubRuntimePanel } from "./hub-runtime-panel.js";
import { HUB_I18N } from "./hub-i18n-core.js";
import { collectHubElements } from "./hub-elements.js";
import { bindHubAppEvents } from "./hub-app-events.js";
import { createHubShellPanel } from "./hub-shell-panel.js";
import { createHubLocalizedShell } from "./hub-localized-shell.js";
import { createHubBrandPanel } from "./hub-brand-panel.js";
import { createHubOutputPanel } from "./hub-output-panel.js";
import { createHubNetworkContext } from "./hub-network-context.js";
import { appendAssistantCardHeader, appendTextElement } from "./hub-dom-builders.js";
import { createHubWorkloadAdapter } from "./hub-workload-adapter.js";
import { createHubState } from "./hub-state.js";
import { hubMessage } from "./hub-message.js";
import { createToolsPlatformLabelRenderer } from "./hub-tools-platform-label.js";
import {
  clearLegacyHubAssistantSecrets,
  loadHubAssistantAudit,
  loadHubAssistantSettings,
  loadHubDensitySettings,
  loadHubHotLogSettings,
  loadHubRecents,
  loadHubRuntimeLogSettings,
  loadHubWorkloadLibrary,
  persistHubAssistantAudit,
  persistHubAssistantSettings,
  persistHubAssistantTrustedHosts,
  persistHubHotLogSettings,
  persistHubRecents,
  persistHubRuntimeLogSettings,
  persistHubTrustedHosts,
  persistHubWorkloadLibrary,
} from "./hub-storage.js";
import {
  HUB_ASSISTANT_ACTION_RISK,
  HUB_ASSISTANT_ACTIONS,
  HUB_ASSISTANT_AUDIT_LIMIT,
  HUB_ASSISTANT_MODEL_PRESETS,
  HUB_DENSITY_DEFAULTS,
  HUB_DENSITY_SETTINGS_KEY,
  HUB_DIRECT_ACTION_RISK,
  HUB_HOT_LOG_POLL_MS,
  HUB_REMOTE_TRUSTED_HOSTS_KEY,
  HUB_WORKLOAD_LIBRARY_LIMIT,
  PROJECT_ACTION_LABELS,
} from "./hub-app-config.js";
import { bindHubLibraryControls } from "./hub-library-controls.js";
import { bindHubRecentActionControls } from "./hub-recent-actions.js";
import {
  formatProjectActionTime,
  mergeProjectActionHistory,
} from "./hub-project-history.js";
import { createHubProjectHistoryPanel } from "./hub-project-history-panel.js";
import { createHubWorkloadPanel } from "./hub-workload-panel.js";
import { importHubCopyPayload, resolveHubCopy } from "./hub-copy-registry.js";
import { downloadHubBlob, downloadHubJson } from "./hub-downloads.js";
import { formatHubOperatorError } from "./hub-operator-errors.js";
import {
  loadRegressionGateReportPanel,
  renderDirectMeshRegressionSnapshot,
  renderRegressionGateReport,
} from "./hub-guides-panel.js";
import {
  bindHubLocalizationPanel,
} from "./hub-localization-panel.js";
import { runHubStartupPhases } from "./hub-startup-phases.js";
import { setupHubStreamingRuntime } from "./hub-streaming-setup.js";

const state = createHubState();

let hubActionRunner;
let hubStreamingRuntime;

function hubCopy() {
  return resolveHubCopy(HUB_I18N, state.language);
}

async function ensureHubLanguagePack(language) {
  const result = await loadDesktopLanguagePack("hub", normalizeDesktopLanguage(language));
  if (result.status === "loaded" && result.pack) {
    importHubCopyPayload(result.pack);
  }
  return result;
}

const elements = collectHubElements(document);

const renderToolsPlatformLabel = createToolsPlatformLabelRenderer({ elements, hubCopy, state });

const {
  currentLocalWorkloadCatalogUrl,
  currentOrchestratorBaseUrl,
  currentWorkflowCatalogUrl,
  ensureDefaultWorkloadCatalogUrl,
} = createHubNetworkContext({ elements });

const {
  mergeHubWorkloadLibrary,
  normalizeHubWorkloadEntry,
  workloadIdentity,
} = createHubWorkloadAdapter({
  buildWorkloadIdentity,
  libraryLimit: HUB_WORKLOAD_LIBRARY_LIMIT,
  mergeStoredHubWorkloadLibrary,
  normalizeStoredHubWorkloadEntry,
});

const {
  applyBrand,
  formatRuntimeReport,
} = createHubBrandPanel({ state });

const {
  localizedHistoryFilterLabel,
  localizedWorkflowCatalogLabel,
  localizedWorkloadFamilyFilterLabel,
  localizedWorkloadFilterLabel,
  renderDesktopLanguagePreference,
  renderPanelLanguage,
  rerenderLocalizedHubShell,
} = createHubLocalizedShell({
  applyDesktopState,
  elements,
  fallbackCopy: HUB_I18N.en,
  hubCopy,
  renderAssistantContext: (...args) => renderAssistantContext(...args),
  renderAssistantPanel: (...args) => renderAssistantPanel(...args),
  renderHubAssistantAudit: (...args) => renderHubAssistantAudit(...args),
  renderHubAssistantLocalCards: (...args) => renderHubAssistantLocalCards(...args),
  renderHubRecents: (...args) => renderHubRecents(...args),
  renderToolsPlatformLabel,
  renderWorkflowCatalog: (...args) => renderWorkflowCatalog(...args),
  setText,
  state,
});

const {
  setDesktopStatusOutput,
  setEventMessage,
  setHotRuntimeLogOutput,
  setHotRuntimeStatusOutput,
  setObserveRuntimeLogOutput,
  setOperationOutput,
  setRuntimeStatusOutput,
} = createHubOutputPanel({
  elements,
  formatRuntimeReport,
});

const loadRegressionGateReport = () => loadRegressionGateReportPanel({
  applyDesktopState, elements, hubCopy, invokeTauri,
  renderDirectMeshRegressionSnapshot, renderRegressionGateReport, state,
});

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

const {
  renderHubAssistantAudit,
  rememberHubAssistantAudit,
  updateHubAssistantAuditDelivery,
} = createHubAssistantAuditPanel({
  appendTextElement,
  assistantRiskStateClass,
  auditLimit: HUB_ASSISTANT_AUDIT_LIMIT,
  currentOrchestratorBaseUrl,
  elements,
  loadHubAssistantAudit,
  persistHubAssistantAudit,
  renderEmptyHistoryState: (...args) => renderEmptyHistoryState(...args),
  state,
});

function saveHubRecents(recents) {
  persistHubRecents(recents);
  renderHubRecents(recents);
}

function setWorkloadLibraryOutput(value) {
  if (elements.workloadLibraryOutput) {
    elements.workloadLibraryOutput.textContent = value;
  }
}

const {
  enhanceHubAccessibility,
  renderHubDensityToggles,
  renderPanelPages,
  renderProjectsPages,
  setPanelPage,
  setProjectsPage,
  setSection,
  toggleHubDensityPanel,
} = createHubShellPanel({
  densityDefaults: HUB_DENSITY_DEFAULTS,
  elements,
  fetchWorkflowCatalog: (...args) => fetchWorkflowCatalog(...args),
  hubCopy,
  persistHubDensitySettings,
  refreshHotRuntimeLog: (...args) => refreshHotRuntimeLog(...args),
  refreshObserveRuntimeLog: (...args) => refreshObserveRuntimeLog(...args),
  renderAssistantContext: (...args) => renderAssistantContext(...args),
  renderHubAssistantLocalCards: (...args) => renderHubAssistantLocalCards(...args),
  state,
  streamingRuntime: () => hubStreamingRuntime,
  syncHotRuntimeLogPolling: (...args) => syncHotRuntimeLogPolling(...args),
  syncObserveRuntimeLogPolling: (...args) => syncObserveRuntimeLogPolling(...args),
});

const {
  answerWithLocalGuide,
  applyAssistantBundlePayload,
  applyAssistantSettings,
  assistantHostRequiresTrust,
  assistantTrustHostOrigin,
  buildHubAssistantLocalCards,
  buildLocalGuideResponse,
  confirmHubAssistantAction,
  currentAssistantSnapshot,
  ensureAssistantHostTrust,
  ensureRemoteHostTrust,
  executeHubAssistantAction,
  executeHubAssistantPlan,
  extractAssistantJsonBlock,
  renderAssistantContext,
  renderAssistantPanel,
  renderHubAssistantLocalCards,
  renderHubAssistantPlan,
  requestHubAssistantPlan,
  setAssistantMode,
  setAssistantPanelOpen,
  syncAssistantSettingsFromInputs,
  updateAssistantEndpointPolicy,
  validateAssistantBaseUrl,
} = createHubAssistantPanel({
  appendAssistantCardHeader,
  appendTextElement,
  applyDesktopState,
  assistantActions: HUB_ASSISTANT_ACTIONS,
  assistantRiskLevel,
  assistantRiskStateClass,
  clearLegacyHubAssistantSecrets,
  elements,
  hubCopy,
  hubDynamic,
  loadHubAssistantSettings,
  loadHubRecents,
  persistHubAssistantSettings,
  persistHubAssistantTrustedHosts,
  persistHubTrustedHosts,
  rememberHubAssistantAudit,
  remoteTrustedHostsKey: HUB_REMOTE_TRUSTED_HOSTS_KEY,
  renderEmptyHistoryState: (...args) => renderEmptyHistoryState(...args),
  renderHubAssistantAudit,
  runAction: (...args) => runAction(...args),
  runActionWithOptions: (...args) => runActionWithOptions(...args),
  setAssistantLocalOutput,
  setAssistantOutput,
  setProjectBundleOutput,
  setProjectsPage,
  setSection,
  setText,
  state,
  streamingRuntime: () => hubStreamingRuntime,
});

const {
  renderEmptyHistoryState,
  renderHistoryFilters,
  renderHubRecents,
  runProjectBundleAction,
  saveProjectBundleRecents,
} = createHubProjectHistoryPanel({
  elements,
  formatProjectActionTime,
  hubCopy,
  hubI18n: HUB_I18N,
  hubMessage,
  invokeTauri,
  loadHubRecents,
  localizedHistoryFilterLabel,
  projectActionLabels: PROJECT_ACTION_LABELS,
  renderAssistantContext,
  renderHubAssistantLocalCards,
  renderHubWorkloadLibrary: (...args) => renderHubWorkloadLibrary(...args),
  runAction,
  saveHubRecents,
  setBusy,
  setProjectBundleOutput,
  state,
});

const {
  clearHubWorkloadLibrary,
  exportHubWorkloadLibrary,
  importHubWorkloadLibrary,
  registerCurrentBundleAsWorkload,
  renderHubWorkloadLibrary,
  saveHubWorkloadLibrary,
  syncLocalControlPlaneWorkloads,
  syncRemoteWorkloadCatalog,
} = createHubWorkloadPanel({
  appendTextElement,
  downloadHubBlob,
  downloadHubJson,
  elements,
  ensureDefaultWorkloadCatalogUrl,
  ensureRemoteHostTrust,
  formatHubOperatorError,
  formatProjectActionTime,
  hubCopy,
  hubI18n: HUB_I18N,
  hubMessage,
  inferDownloadFilename,
  invokeTauri,
  loadHubWorkloadLibrary,
  localizedWorkloadFamilyFilterLabel,
  localizedWorkloadFilterLabel,
  mergeHubWorkloadLibrary,
  normalizeHubWorkloadEntry,
  normalizeRemoteWorkloadCatalogPayload,
  persistHubWorkloadLibrary,
  projectSummaryFromInspectPayload,
  renderAssistantContext,
  renderEmptyHistoryState,
  renderHubAssistantLocalCards,
  runAction,
  setWorkloadLibraryOutput,
  state,
  validateHubCatalogUrl,
  validateRemoteWorkloadCatalogPayload,
  workloadDomainLabel,
  workloadFamilyLabel,
  workloadIdentity,
  workloadProvenanceLabel,
  workloadSourceBadge,
});

async function invokeGuardedMutation(action, payload = {}) {
  return hubActionRunner.invokeGuardedMutation(action, payload);
}

const {
  clearHotRuntimeLogView,
  copyHotRuntimeLogView,
  currentHotRuntimeLogService,
  currentObserveRuntimeLogService,
  inferHotRuntimeState,
  persistCurrentHotLogSettings,
  persistCurrentObserveRuntimeLogSettings,
  renderHotRuntimeLogServiceLabel,
  stopHotRuntimeLogPolling,
  syncHotRuntimeLogPolling,
  syncObserveRuntimeLogPolling,
} = createHubRuntimeLogController({
  applyDesktopState,
  copySanitizedRuntimeLogToClipboard,
  elements,
  fallbackPollMs: HUB_HOT_LOG_POLL_MS,
  inferHotRuntimeStateHelper,
  persistHubHotLogSettings,
  persistHubRuntimeLogSettings,
  refreshHotRuntimeLog: (...args) => refreshHotRuntimeLog(...args),
  refreshObserveRuntimeLog: (...args) => refreshObserveRuntimeLog(...args),
  sanitizeRuntimeLogForClipboardHelper,
  setHotRuntimeLogOutput,
  state,
});

const {
  copyObserveRuntimeLogView,
  loadDirectMeshRegressionSnapshot,
  loadEnvironment,
  refreshDesktopStatusOutput,
  refreshHotRuntimeLog,
  refreshHotRuntimeStatus,
  refreshObserveRuntimeLog,
  refreshRuntimeStatus,
} = createHubRuntimePanel({
  applyDesktopState,
  currentHotRuntimeLogService,
  currentObserveRuntimeLogService,
  currentOrchestratorBaseUrl,
  elements,
  ensureDefaultWorkloadCatalogUrl,
  formatHubOperatorError,
  hubCopy,
  hubDynamic,
  invokeTauri,
  renderAssistantContext,
  renderHubAssistantLocalCards,
  renderToolsPlatformLabel,
  setDesktopStatusOutput,
  setHotRuntimeLogOutput,
  setHotRuntimeStatusOutput,
  setObserveRuntimeLogOutput,
  setRuntimeStatusOutput,
  state,
  syncHotRuntimeLogPolling,
});

function setProjectBundleOutput(value) {
  elements.projectBundleOutput.textContent = value;
}

function setAssistantOutput(value) {
  if (elements.assistantOutput) {
    elements.assistantOutput.textContent = value;
  }
}

function setAssistantLocalOutput(value) {
  if (elements.assistantLocalOutput) {
    elements.assistantLocalOutput.textContent = value;
  }
}

function setWorkflowCatalogOutput(value) {
  if (elements.workflowCatalogOutput) {
    elements.workflowCatalogOutput.textContent = value;
  }
}

const {
  fetchWorkflowCatalog,
  renderWorkflowCatalog,
  runWorkflowCatalogSample,
  waitForWorkflowJob,
} = createHubWorkflowPanel({
  appendTextElement,
  applyDesktopState,
  currentOrchestratorBaseUrl,
  currentWorkflowCatalogUrl,
  elements,
  formatHubOperatorError,
  hubMessage,
  localizedWorkflowCatalogLabel,
  renderEmptyHistoryState,
  setOperationOutput,
  setWorkflowCatalogOutput,
  state,
});

hubStreamingRuntime = setupHubStreamingRuntime({
  elements,
  fetchWorkflowCatalog,
  refreshHotRuntimeLog,
  refreshObserveRuntimeLog,
  setEventMessage,
  state,
});

hubActionRunner = createHubActionRunner({
  applyDesktopState,
  clearHotRuntimeLogView,
  clearHubWorkloadLibrary,
  copyHotRuntimeLogView,
  copyObserveRuntimeLogView,
  currentHotRuntimeLogService,
  currentObserveRuntimeLogService,
  directActionRisk: HUB_DIRECT_ACTION_RISK,
  elements,
  exportHubWorkloadLibrary,
  fetchWorkflowCatalog,
  formatHubOperatorError,
  hubDynamic,
  invokeTauri,
  refreshDesktopStatusOutput,
  refreshHotRuntimeLog,
  refreshHotRuntimeStatus,
  refreshObserveRuntimeLog,
  refreshRuntimeStatus,
  registerCurrentBundleAsWorkload,
  runProjectBundleAction,
  setBusy,
  setEventMessage,
  setOperationOutput,
  setProjectBundleOutput,
  setProjectsPage,
  setSection,
  state,
  syncLocalControlPlaneWorkloads,
  syncRemoteWorkloadCatalog,
});

function hubDynamic(key, replacements = {}) {
  return hubMessage(hubCopy().dynamic?.[key] || HUB_I18N.en.dynamic?.[key] || "", replacements);
}

function setBusy(isBusy, label = "idle") {
  state.isBusy = isBusy;
  applyDesktopState(elements.actionState, label, { kind: "activity" });
  elements.actionButtons.forEach((button) => {
    button.disabled = isBusy;
    button.classList.toggle("is-busy", isBusy);
  });
}

async function runAction(action) {
  return hubActionRunner.runAction(action);
}

async function runActionWithOptions(action, options = {}) {
  return hubActionRunner.runActionWithOptions(action, options);
}

function buildHubAppEventsContext() {
  return {
    elements, state, applyDesktopState, formatHubOperatorError,
    hubCopy, hubDynamic, localizedWorkflowCatalogLabel, normalizeDesktopLanguage,
    ensureHubLanguagePack,
    bindHubLibraryControls, bindHubLocalizationPanel, bindHubRecentActionControls,
    loadHubRecents, mergeProjectActionHistory, saveDesktopLanguagePreference, saveHubRecents,
    answerWithLocalGuide, executeHubAssistantPlan, requestHubAssistantPlan,
    renderAssistantContext, renderHubAssistantLocalCards, renderHubAssistantPlan,
    setAssistantMode, setAssistantOutput, setAssistantPanelOpen,
    syncAssistantSettingsFromInputs, updateAssistantEndpointPolicy,
    clearHubWorkloadLibrary, importHubWorkloadLibrary, renderHubWorkloadLibrary, setWorkloadLibraryOutput,
    fetchWorkflowCatalog, renderWorkflowCatalog, setWorkflowCatalogOutput,
    refreshDesktopStatusOutput, refreshHotRuntimeLog, refreshObserveRuntimeLog,
    renderHotRuntimeLogServiceLabel, stopHotRuntimeLogPolling,
    persistCurrentHotLogSettings, persistCurrentObserveRuntimeLogSettings,
    syncHotRuntimeLogPolling, syncObserveRuntimeLogPolling,
    renderHubRecents, renderToolsPlatformLabel, rerenderLocalizedHubShell, runAction,
    setEventMessage, setOperationOutput, setPanelPage, setProjectBundleOutput, setProjectsPage, setSection,
    toggleHubDensityPanel,
  };
}

bindHubAppEvents(buildHubAppEventsContext());
watchDesktopLanguagePreference({
  getCurrentLanguage: () => state.language,
  onChange: async (language) => {
    const packResult = await ensureHubLanguagePack(language);
    state.language = language;
    rerenderLocalizedHubShell();
    renderToolsPlatformLabel();
    setEventMessage(`language synced: ${language} · ${packResult.status}`, "language:sync");
  },
});
window.__kyuubikiHubAppReadyAt = Date.now();
setEventMessage("Hub app module ready.", "app:ready");

void runHubStartupPhases({
  applyAssistantSettings, applyBrand, elements, enhanceHubAccessibility,
  ensureHubLanguagePack, fetchWorkflowCatalog, loadDesktopLanguagePreference, loadDirectMeshRegressionSnapshot,
  loadEnvironment, loadHubDensitySettings, loadHubHotLogSettings, loadHubRuntimeLogSettings,
  loadRegressionGateReport, refreshDesktopStatusOutput, refreshHotRuntimeStatus,
  refreshRuntimeStatus, renderAssistantPanel, renderHotRuntimeLogServiceLabel,
  renderHubDensityToggles, renderHubRecents, renderPanelPages, rerenderLocalizedHubShell,
  setBusy, setEventMessage, setSection, state, syncDesktopStates,
});
