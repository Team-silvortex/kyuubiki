import {
  copySanitizedRuntimeLogToClipboard,
  refreshDesktopStatusPanel,
  refreshHotRuntimeStatusPanel,
  refreshRuntimeLogPanel,
  refreshRuntimeStatusPanel,
} from "./hub-runtime-helpers.js";
import {
  renderDirectMeshRegressionLoadError,
  renderDirectMeshRegressionSnapshot,
} from "./hub-guides-panel.js";
import {
  normalizeDesktopPlatform,
  populateDesktopPlatformSelect,
} from "./shared/platform.js";

export function createHubRuntimePanel(context) {
  async function loadEnvironment() {
    const environment = await context.invokeTauri("hub_environment");
    context.state.hostPlatform = normalizeDesktopPlatform(environment.host_platform);

    populateDesktopPlatformSelect(context.elements.releasePlatform, {
      includeAll: true,
      fallback: context.state.hostPlatform,
    });

    if (context.elements.releasePlatform && !context.elements.releasePlatform.value) {
      context.elements.releasePlatform.value = context.state.hostPlatform;
    }
    context.renderToolsPlatformLabel();

    if (context.elements.workbenchUrl) {
      context.elements.workbenchUrl.textContent = environment.workbench_url;
    }

    if (context.elements.orchestratorUrl) {
      context.elements.orchestratorUrl.textContent = environment.orchestrator_url;
    }

    context.ensureDefaultWorkloadCatalogUrl();
    context.applyDesktopState(context.elements.currentRuntimeMode, "orchestrated_gui", { kind: "activity" });
    context.applyDesktopState(context.elements.currentProfile, environment.deployment_mode, { kind: "activity" });
    context.renderAssistantContext();
  }

  async function loadDirectMeshRegressionSnapshot() {
    try {
      context.state.directMeshRegressionSnapshot = await context.invokeTauri("hub_direct_mesh_regression_snapshot");
      renderDirectMeshRegressionSnapshot({
        elements: context.elements,
        snapshot: context.state.directMeshRegressionSnapshot,
        copy: context.hubCopy(),
        regressionGateReport: context.state.regressionGateReport,
        applyDesktopState: context.applyDesktopState,
      });
    } catch (error) {
      renderDirectMeshRegressionLoadError({
        elements: context.elements,
        copy: context.hubCopy(),
        error,
        applyDesktopState: context.applyDesktopState,
        formatHubOperatorError: context.formatHubOperatorError,
      });
    }
  }

  async function refreshRuntimeStatus() {
    await refreshRuntimeStatusPanel({
      invokeTauri: context.invokeTauri,
      orchestratorBaseUrl: context.currentOrchestratorBaseUrl(),
      setRuntimeStatusOutput: context.setRuntimeStatusOutput,
      applyDesktopState: context.applyDesktopState,
      localRuntimeStatus: context.elements.localRuntimeStatus,
      observeRuntimeStatus: context.elements.observeRuntimeStatus,
      runtimeStatusPlane: context.elements.runtimeStatusPlane,
    });
    context.renderAssistantContext();
    context.renderHubAssistantLocalCards();
  }

  async function refreshHotRuntimeStatus() {
    await refreshHotRuntimeStatusPanel({
      invokeTauri: context.invokeTauri,
      setHotRuntimeStatusOutput: context.setHotRuntimeStatusOutput,
      applyDesktopState: context.applyDesktopState,
      hotRuntimeStatus: context.elements.hotRuntimeStatus,
      observeHotStatus: context.elements.observeHotStatus,
      hotRuntimeMode: context.elements.hotRuntimeMode,
      observeHotMode: context.elements.observeHotMode,
      syncHotRuntimeLogPolling: context.syncHotRuntimeLogPolling,
      refreshHotRuntimeLog,
    });
  }

  async function refreshHotRuntimeLog(options = {}) {
    await refreshRuntimeLogPanel({
      invokeTauri: context.invokeTauri,
      state: context.state,
      inFlightKey: "hotLogRefreshInFlight",
      service: context.currentHotRuntimeLogService(),
      silent: options?.silent === true,
      setOutput: context.setHotRuntimeLogOutput,
      hubDynamic: context.hubDynamic,
      formatHubOperatorError: context.formatHubOperatorError,
    });
  }

  async function refreshObserveRuntimeLog(options = {}) {
    await refreshRuntimeLogPanel({
      invokeTauri: context.invokeTauri,
      state: context.state,
      inFlightKey: "runtimeLogRefreshInFlight",
      service: context.currentObserveRuntimeLogService(),
      silent: options?.silent === true,
      setOutput: context.setObserveRuntimeLogOutput,
      hubDynamic: context.hubDynamic,
      formatHubOperatorError: context.formatHubOperatorError,
    });
  }

  async function copyObserveRuntimeLogView() {
    await copySanitizedRuntimeLogToClipboard(
      String(context.elements.observeRuntimeLogOutput?.textContent || "").trim(),
    );
  }

  async function refreshDesktopStatusOutput() {
    await refreshDesktopStatusPanel({
      invokeTauri: context.invokeTauri,
      platform: context.elements.releasePlatform?.value || context.state.hostPlatform,
      setDesktopStatusOutput: context.setDesktopStatusOutput,
      formatHubOperatorError: context.formatHubOperatorError,
    });
  }

  return {
    copyObserveRuntimeLogView,
    loadDirectMeshRegressionSnapshot,
    loadEnvironment,
    refreshDesktopStatusOutput,
    refreshHotRuntimeLog,
    refreshHotRuntimeStatus,
    refreshObserveRuntimeLog,
    refreshRuntimeStatus,
  };
}
