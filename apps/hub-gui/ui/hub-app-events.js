export function bindHubAppEvents({
  answerWithLocalGuide,
  applyDesktopState,
  bindHubLibraryControls,
  bindHubLocalizationPanel,
  bindHubRecentActionControls,
  clearHubWorkloadLibrary,
  elements,
  executeHubAssistantPlan,
  fetchWorkflowCatalog,
  formatHubOperatorError,
  hubCopy,
  hubDynamic,
  importHubWorkloadLibrary,
  loadHubRecents,
  localizedWorkflowCatalogLabel,
  mergeProjectActionHistory,
  normalizeDesktopLanguage,
  persistCurrentHotLogSettings,
  persistCurrentObserveRuntimeLogSettings,
  refreshDesktopStatusOutput,
  refreshHotRuntimeLog,
  refreshObserveRuntimeLog,
  renderAssistantContext,
  renderHubAssistantLocalCards,
  renderHubAssistantPlan,
  renderHubRecents,
  renderHubWorkloadLibrary,
  renderHotRuntimeLogServiceLabel,
  renderToolsPlatformLabel,
  renderWorkflowCatalog,
  requestHubAssistantPlan,
  rerenderLocalizedHubShell,
  runAction,
  saveDesktopLanguagePreference,
  saveHubRecents,
  setSection,
  setAssistantMode,
  setAssistantOutput,
  setAssistantPanelOpen,
  setOperationOutput,
  setPanelPage,
  setProjectBundleOutput,
  setProjectsPage,
  setWorkflowCatalogOutput,
  setWorkloadLibraryOutput,
  state,
  stopHotRuntimeLogPolling,
  syncAssistantSettingsFromInputs,
  syncHotRuntimeLogPolling,
  syncObserveRuntimeLogPolling,
  toggleHubDensityPanel,
  updateAssistantEndpointPolicy,
}) {
  elements.navItems.forEach((item) => {
    item.addEventListener("click", () => setSection(item.dataset.target));
  });
  
  elements.projectsPageButtons.forEach((button) => {
    button.addEventListener("click", () => {
      setProjectsPage(button.dataset.projectsPage || "start");
      applyDesktopState(elements.actionState, "active", { kind: "activity" });
      setOperationOutput(hubDynamic("focusedHomePage", { page: button.dataset.projectsPage || "start" }));
    });
  });
  
  elements.projectsTargetButtons.forEach((button) => {
    button.addEventListener("click", () => {
      setProjectsPage(button.dataset.projectsTarget || "start");
      applyDesktopState(elements.actionState, "active", { kind: "activity" });
      setOperationOutput(hubDynamic("openedHomeTarget", { page: button.dataset.projectsTarget || "start" }));
    });
  });
  
  elements.panelPageButtons.forEach((button) => {
    button.addEventListener("click", () => {
      const group = button.dataset.panelPageGroup || "";
      const page = button.dataset.panelPage || "";
      setPanelPage(group, page);
      applyDesktopState(elements.actionState, "active", { kind: "activity" });
      setOperationOutput(hubDynamic("focusedPanelPage", { page, group }));
    });
  });
  
  elements.assistantFab?.addEventListener("click", () => {
    setAssistantPanelOpen(!state.assistantOpen);
    applyDesktopState(elements.actionState, "active", { kind: "activity" });
    setOperationOutput(state.assistantOpen ? hubDynamic("assistantPanelOpened") : hubDynamic("assistantPanelClosed"));
  });
  
  elements.assistantClose?.addEventListener("click", () => {
    setAssistantPanelOpen(false);
    applyDesktopState(elements.actionState, "idle", { kind: "activity" });
    setOperationOutput(hubDynamic("assistantPanelClosed"));
  });
  
  elements.assistantPanel?.addEventListener("click", (event) => {
    if (event.target !== elements.assistantPanel) {
      return;
    }
    setAssistantPanelOpen(false);
    applyDesktopState(elements.actionState, "idle", { kind: "activity" });
    setOperationOutput(hubDynamic("assistantPanelClosed"));
  });
  
  elements.assistantLocalAsk?.addEventListener("click", () => {
    answerWithLocalGuide();
  });
  
  elements.assistantLocalPrompt?.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }
    event.preventDefault();
    answerWithLocalGuide();
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
    renderToolsPlatformLabel();
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
  
  bindHubRecentActionControls({
    elements,
    state,
    renderHubRecents,
    setProjectBundleOutput,
    loadHubRecents,
    saveHubRecents,
    mergeProjectActionHistory,
    formatHubOperatorError,
  });
  
  bindHubLibraryControls({
    elements,
    state,
    renderHubWorkloadLibrary,
    setWorkloadLibraryOutput,
    renderWorkflowCatalog,
    setWorkflowCatalogOutput,
    localizedWorkflowCatalogLabel,
  });
  
  elements.densityToggleButtons.forEach((button) => {
    button.addEventListener("click", () => {
      toggleHubDensityPanel(button.dataset.densityToggle || "");
    });
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
  
  bindHubLocalizationPanel({
    elements,
    hubCopy,
    rerenderLocalizedHubShell,
    setOperationOutput,
  });
  
  elements.languageSelect?.addEventListener("change", async (event) => {
    state.language = await saveDesktopLanguagePreference(normalizeDesktopLanguage(event.target.value));
    rerenderLocalizedHubShell();
    renderToolsPlatformLabel();
    window.requestAnimationFrame(() => {
      rerenderLocalizedHubShell();
      renderToolsPlatformLabel();
    });
  });
}
