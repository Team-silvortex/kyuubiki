import { countHubUiPerf } from "./hub-ui-performance.js";

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
  ensureHubLanguagePack,
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
  setEventMessage,
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
  const scheduleFrame = (callback) => {
    let frame = 0;
    return () => {
      if (frame) {
        return;
      }
      frame = window.requestAnimationFrame(() => {
        frame = 0;
        callback();
        countHubUiPerf("assistant-context-renders");
      });
    };
  };
  const scheduleAssistantContextRender = scheduleFrame(() => {
    renderAssistantContext();
    renderHubAssistantLocalCards();
  });
  const scheduleCapturedClickMessage = (() => {
    let frame = 0;
    let nextMessage = null;
    return (message, token) => {
      nextMessage = [message, token];
      if (frame) {
        return;
      }
      frame = window.requestAnimationFrame(() => {
        frame = 0;
        const [queuedMessage, queuedToken] = nextMessage || [];
        nextMessage = null;
        if (queuedMessage) {
          setEventMessage?.(queuedMessage, queuedToken);
          countHubUiPerf("coalesced-click-messages");
        }
      });
    };
  })();

  document.addEventListener(
    "click",
    (event) => {
      const target = event.target?.closest?.("[data-action], button, select, input, textarea, a");
      if (!target) {
        return;
      }
      const label =
        target.dataset?.action ||
        target.id ||
        target.getAttribute?.("aria-label") ||
        target.textContent?.trim()?.slice(0, 48) ||
        target.tagName;
      scheduleCapturedClickMessage(`captured click: ${label}`, `target:${target.tagName?.toLowerCase?.() || "unknown"}`);
    },
    true,
  );

  document.addEventListener("click", async (event) => {
    const button = event.target?.closest?.("[data-mainline-action]");
    if (!button) {
      return;
    }

    const action = button.dataset.mainlineAction;
    setEventMessage?.(`mainline step: ${action}`, "mainline:click");
    if (action === "runtimes") {
      setSection("runtimes");
      return;
    }
    if (action === "library" || action === "bundles" || action === "guides") {
      setProjectsPage(action);
      return;
    }
    if (action === "open-workbench") {
      await runAction("open-workbench");
    }
  });

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
      window.__kyuubikiHubDomClickAt = Date.now();
      setEventMessage?.(`button click: ${button.dataset.action}`, "dom:click");
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
    scheduleAssistantContextRender();
  });
  elements.projectBundleComparePath?.addEventListener("input", () => {
    scheduleAssistantContextRender();
  });
  elements.projectBundleOutPath?.addEventListener("input", () => {
    scheduleAssistantContextRender();
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

  function languageChangeSummary(language, packResult) {
    const copy = hubCopy();
    const status = packResult?.status || "builtin";
    const statusLabel =
      state.language === "zh"
        ? status === "loaded"
          ? "语言包已加载"
          : status === "missing"
            ? "语言包缺失，暂用内建文案"
            : "内建文案已应用"
        : state.language === "ja"
          ? status === "loaded"
            ? "言語パックを読み込みました"
            : status === "missing"
              ? "言語パック未同梱、組み込み文言を使用"
              : "組み込み文言を適用しました"
          : state.language === "es"
            ? status === "loaded"
              ? "paquete de idioma cargado"
              : status === "missing"
                ? "paquete no incluido, usando copy integrado"
                : "copy integrado aplicado"
            : state.language === "en"
              ? status === "loaded"
                ? "language pack loaded"
                : status === "missing"
                  ? "language pack missing, using built-in copy"
                  : "built-in copy applied"
              : status === "missing"
                ? "◇ moxi 2.x"
                : "✓ moxi 2.x";
    const refreshHint =
      state.language === "zh"
        ? "如已打开其他桌面外壳且未刷新，请重启。"
        : state.language === "ja"
          ? "他のデスクトップシェルが更新されない場合は再起動してください。"
          : state.language === "es"
            ? "Reinicia otros shells de escritorio si no se actualizan."
            : state.language === "en"
              ? "Restart open desktop shells if they were already running."
              : "↻";
    return `${copy.shell.language}: ${language} · ${statusLabel}. ${refreshHint}`;
  }
  
  elements.languageSelect?.addEventListener("change", async (event) => {
    window.__kyuubikiHubLanguageChangeAt = Date.now();
    setEventMessage?.(`language select changed: ${event.target.value}`, "dom:change");
    state.language = await saveDesktopLanguagePreference(normalizeDesktopLanguage(event.target.value));
    const packResult = await ensureHubLanguagePack?.(state.language);
    rerenderLocalizedHubShell();
    renderToolsPlatformLabel();
    applyDesktopState(elements.actionState, "ready", { kind: "activity" });
    setOperationOutput(languageChangeSummary(state.language, packResult));
    setEventMessage?.(`language applied: ${state.language} · ${packResult?.status || "builtin"}`, "language:complete");
    window.requestAnimationFrame(() => {
      rerenderLocalizedHubShell();
      renderToolsPlatformLabel();
    });
  });
}
