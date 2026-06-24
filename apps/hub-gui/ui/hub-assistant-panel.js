import {
  applyAssistantBundlePayload as applyAssistantBundlePayloadEngine,
  assistantHostRequiresTrust as assistantHostRequiresTrustEngine,
  assistantTrustHostOrigin as assistantTrustHostOriginEngine,
  confirmHubAssistantAction as confirmHubAssistantActionEngine,
  ensureAssistantHostTrust as ensureAssistantHostTrustEngine,
  ensureRemoteHostTrust as ensureRemoteHostTrustEngine,
  executeHubAssistantAction as executeHubAssistantActionEngine,
  executeHubAssistantPlan as executeHubAssistantPlanEngine,
  renderHubAssistantPlan as renderHubAssistantPlanEngine,
  requestHubAssistantPlan as requestHubAssistantPlanEngine,
} from "./hub-assistant-engine.js";
import {
  buildHubAssistantLocalCards as buildHubAssistantLocalCardsModule,
  buildLocalGuideResponse as buildLocalGuideResponseModule,
  extractAssistantJsonBlock as extractAssistantJsonBlockModule,
  renderHubAssistantLocalCards as renderHubAssistantLocalCardsModule,
} from "./hub-assistant-local.js";

export function createHubAssistantPanel(context) {
  function renderAssistantPanel() {
    const open = context.state.assistantOpen === true;
    context.elements.assistantPanel?.classList.toggle("hidden", !open);
    context.elements.assistantPanel?.setAttribute("aria-hidden", String(!open));
    context.elements.assistantFab?.setAttribute("aria-expanded", String(open));
  }

  function setAssistantPanelOpen(open) {
    context.state.assistantOpen = open === true;
    renderAssistantPanel();
  }

  function currentAssistantSnapshot() {
    return {
      activeSection: context.state.activeSection,
      runtimeStatus: context.elements.localRuntimeStatus?.textContent?.trim() || "unknown",
      profile: context.elements.currentProfile?.textContent?.trim() || "unknown",
      bundlePath: context.elements.projectBundlePath?.value?.trim() || "",
      comparePath: context.elements.projectBundleComparePath?.value?.trim() || "",
      outputPath: context.elements.projectBundleOutPath?.value?.trim() || "",
      favorites: context.loadHubRecents().actions?.filter((entry) => entry.pinned).length ?? 0,
    };
  }

  function renderAssistantContext() {
    const snapshot = currentAssistantSnapshot();
    context.setText(context.elements.assistantContextSection, snapshot.activeSection);
    context.setText(context.elements.assistantContextRuntime, snapshot.runtimeStatus);
    context.setText(context.elements.assistantContextBundle, snapshot.bundlePath || "--");
  }

  function setAssistantMode(mode) {
    context.state.assistantMode = mode === "llm" ? "llm" : "local";
    context.elements.assistantModeButtons.forEach((button) => {
      const active = button.dataset.assistantMode === context.state.assistantMode;
      button.classList.toggle("desktop-shell-button-primary", active);
      button.classList.toggle("desktop-shell-button-ghost", !active);
      button.setAttribute("aria-pressed", String(active));
    });
    context.elements.assistantLocalPanel?.classList.toggle("hidden", context.state.assistantMode !== "local");
    context.elements.assistantLlmPanel?.classList.toggle("hidden", context.state.assistantMode !== "llm");
    context.applyDesktopState(
      context.elements.assistantEngineState,
      context.state.assistantMode === "llm" ? "remote model" : "local guide",
      { kind: "activity" },
    );
    context.persistHubAssistantSettings({
      ...context.loadHubAssistantSettings(),
      mode: context.state.assistantMode,
      baseUrl: context.elements.assistantBaseUrl?.value || "",
      modelPreset: context.elements.assistantModelPreset?.value || "gpt-5",
      model: context.elements.assistantModelName?.value || "gpt-5",
    });
  }

  function buildHubAssistantLocalCards() {
    return buildHubAssistantLocalCardsModule(localCardContext());
  }

  function renderHubAssistantLocalCards() {
    return renderHubAssistantLocalCardsModule({
      assistantLocalCards: context.elements.assistantLocalCards,
      renderEmptyHistoryState: context.renderEmptyHistoryState,
      appendAssistantCardHeader: context.appendAssistantCardHeader,
      appendTextElement: context.appendTextElement,
      ...localCardContext(),
    });
  }

  function localCardContext() {
    return {
      currentAssistantSnapshot,
      hubDynamic: context.hubDynamic,
      homeBundlesTitle: context.hubCopy().home.quick.bundlesTitle,
      shellStartLocal: context.hubCopy().shell.startLocal,
      bundleInspect: context.hubCopy().bundles.inspect,
      bundleNormalize: context.hubCopy().bundles.normalize,
      bundleDiff: context.hubCopy().bundles.diff,
      homeGuidesTitle: context.hubCopy().home.tabs.guides,
      shellOpenWorkbench: context.hubCopy().shell.openWorkbench,
      setSection: context.setSection,
      setProjectsPage: context.setProjectsPage,
      projectBundlePath: context.elements.projectBundlePath,
      setProjectBundleOutput: context.setProjectBundleOutput,
      runAction: context.runAction,
    };
  }

  function buildLocalGuideResponse(query) {
    return buildLocalGuideResponseModule(query, {
      currentAssistantSnapshot,
      hubDynamic: context.hubDynamic,
    });
  }

  function answerWithLocalGuide() {
    const query = context.elements.assistantLocalPrompt?.value || "";
    context.setAssistantLocalOutput(buildLocalGuideResponse(query));
  }

  function extractAssistantJsonBlock(value) {
    return extractAssistantJsonBlockModule(value);
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

    if (protocol === "https:" || (protocol === "http:" && isLoopback)) {
      return { ok: true, normalized: baseUrl };
    }

    return {
      ok: false,
      reason: "Assistant base URL must use https, or http only for localhost / 127.0.0.1 / ::1.",
    };
  }

  function updateAssistantEndpointPolicy() {
    if (!context.elements.assistantEndpointPolicy || !context.elements.assistantBaseUrl) {
      return;
    }

    const baseUrl = context.elements.assistantBaseUrl.value.trim();
    if (!baseUrl) {
      context.elements.assistantEndpointPolicy.textContent = context.hubDynamic("endpointPolicyDefault");
      return;
    }

    const validation = validateAssistantBaseUrl(baseUrl);
    if (!validation.ok) {
      context.elements.assistantEndpointPolicy.textContent =
        `${validation.reason} The API key is sent directly to the configured base URL.`;
      return;
    }

    context.elements.assistantEndpointPolicy.textContent = context.hubDynamic("endpointPolicyAllowed");
  }

  function assistantTrustHostOrigin(baseUrl) {
    return assistantTrustHostOriginEngine(baseUrl);
  }

  function assistantHostRequiresTrust(baseUrl) {
    return assistantHostRequiresTrustEngine(baseUrl);
  }

  function ensureAssistantHostTrust(baseUrl, apiKey) {
    return ensureAssistantHostTrustEngine(baseUrl, apiKey, {
      trustedHosts: context.state.assistantTrustedHosts,
      persistTrustedHosts: context.persistHubAssistantTrustedHosts,
      confirm: window.confirm.bind(window),
    });
  }

  function ensureRemoteHostTrust(baseUrl, label) {
    return ensureRemoteHostTrustEngine(baseUrl, label, {
      trustedHosts: context.state.remoteTrustedHosts,
      persistTrustedHosts: (trustedHosts) => context.persistHubTrustedHosts(context.remoteTrustedHostsKey, trustedHosts),
      confirm: window.confirm.bind(window),
    });
  }

  async function requestHubAssistantPlan() {
    return requestHubAssistantPlanEngine({
      assistantBaseUrl: context.elements.assistantBaseUrl,
      assistantModelName: context.elements.assistantModelName,
      assistantPrompt: context.elements.assistantPrompt,
      assistantApiKey: context.elements.assistantApiKey,
      validateAssistantBaseUrl,
      assistantTrustedHosts: context.state.assistantTrustedHosts,
      persistAssistantTrustedHosts: context.persistHubAssistantTrustedHosts,
      confirm: window.confirm.bind(window),
      currentAssistantSnapshot,
      assistantActions: context.assistantActions,
      buildHubAssistantLocalCards,
      extractAssistantJsonBlock,
    });
  }

  function renderHubAssistantPlan() {
    return renderHubAssistantPlanEngine({
      assistantPlanActions: context.elements.assistantPlanActions,
      assistantPlan: context.state.assistantPlan,
      renderEmptyHistoryState: context.renderEmptyHistoryState,
      hubDynamic: context.hubDynamic,
      appendAssistantCardHeader: context.appendAssistantCardHeader,
      appendTextElement: context.appendTextElement,
      assistantRiskLevel: context.assistantRiskLevel,
      assistantRiskStateClass: context.assistantRiskStateClass,
      executeHubAssistantAction,
    });
  }

  function confirmHubAssistantAction(action, source = "assistant") {
    return confirmHubAssistantActionEngine(action, source, {
      assistantRiskLevel: context.assistantRiskLevel,
      rememberHubAssistantAudit: context.rememberHubAssistantAudit,
      confirm: window.confirm.bind(window),
    });
  }

  function applyAssistantBundlePayload(payload) {
    return applyAssistantBundlePayloadEngine(payload, {
      projectBundlePath: context.elements.projectBundlePath,
      projectBundleComparePath: context.elements.projectBundleComparePath,
      projectBundleOutPath: context.elements.projectBundleOutPath,
    });
  }

  async function executeHubAssistantAction(action, payload = {}, source = "assistant") {
    return executeHubAssistantActionEngine(action, payload, source, {
      assistantRiskLevel: context.assistantRiskLevel,
      setAssistantOutput: context.setAssistantOutput,
      hubDynamic: context.hubDynamic,
      setSection: context.setSection,
      rememberHubAssistantAudit: context.rememberHubAssistantAudit,
      runActionWithOptions: context.runActionWithOptions,
      renderAssistantContext,
      projectBundlePath: context.elements.projectBundlePath,
      projectBundleComparePath: context.elements.projectBundleComparePath,
      projectBundleOutPath: context.elements.projectBundleOutPath,
      confirm: window.confirm.bind(window),
    });
  }

  async function executeHubAssistantPlan() {
    return executeHubAssistantPlanEngine({
      assistantPlan: context.state.assistantPlan,
      assistantApprovePlan: context.elements.assistantApprovePlan,
      setAssistantOutput: context.setAssistantOutput,
      hubDynamic: context.hubDynamic,
      executeHubAssistantAction,
      rememberHubAssistantAudit: context.rememberHubAssistantAudit,
      assistantRiskLevel: context.assistantRiskLevel,
    });
  }

  function syncAssistantSettingsFromInputs() {
    context.state.assistantApiKey = context.elements.assistantApiKey?.value || "";
    context.persistHubAssistantSettings({
      mode: context.state.assistantMode,
      baseUrl: context.elements.assistantBaseUrl?.value || "",
      modelPreset: context.elements.assistantModelPreset?.value || "gpt-5",
      model: context.elements.assistantModelName?.value || "gpt-5",
    });
  }

  function applyAssistantSettings() {
    context.clearLegacyHubAssistantSecrets();
    const settings = context.loadHubAssistantSettings();
    context.state.assistantMode = settings.mode;
    if (context.elements.assistantBaseUrl) {
      context.elements.assistantBaseUrl.value = settings.baseUrl;
    }
    if (context.elements.assistantModelPreset) {
      context.elements.assistantModelPreset.value = settings.modelPreset;
    }
    if (context.elements.assistantModelName) {
      context.elements.assistantModelName.value = settings.model;
    }
    if (context.elements.assistantApiKey) {
      context.elements.assistantApiKey.value = context.state.assistantApiKey || "";
    }
    setAssistantMode(settings.mode);
    updateAssistantEndpointPolicy();
    renderAssistantContext();
    renderHubAssistantLocalCards();
    renderHubAssistantPlan();
    context.renderHubAssistantAudit();
  }

  return {
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
  };
}
