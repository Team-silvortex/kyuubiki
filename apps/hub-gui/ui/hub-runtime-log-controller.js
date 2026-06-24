export function createHubRuntimeLogController({
  applyDesktopState,
  copySanitizedRuntimeLogToClipboard,
  elements,
  fallbackPollMs,
  inferHotRuntimeStateHelper,
  persistHubHotLogSettings,
  persistHubRuntimeLogSettings,
  refreshHotRuntimeLog,
  refreshObserveRuntimeLog,
  sanitizeRuntimeLogForClipboardHelper,
  setHotRuntimeLogOutput,
  state,
}) {
  let hotRuntimeLogPollHandle = null;
  let observeRuntimeLogPollHandle = null;

  function currentHotRuntimeStatus() {
    return String(elements.hotRuntimeStatus?.textContent || "").trim().toLowerCase();
  }

  function currentHotRuntimeLogService() {
    return elements.hotRuntimeLogService?.value || "hot-stack";
  }

  function currentObserveRuntimeLogService() {
    return elements.observeRuntimeLogService?.value || "frontend";
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

  function shouldPollHotRuntimeLog() {
    return state.activeSection === "runtimes"
      && currentHotRuntimeStatus() === "running"
      && currentHotRuntimeLogAutoRefresh();
  }

  function shouldPollObserveRuntimeLog() {
    return state.activeSection === "observe" && currentObserveRuntimeLogAutoRefresh();
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

  function clearHotRuntimeLogView() {
    setHotRuntimeLogOutput(`Cleared local log view for ${currentHotRuntimeLogService()}. Background tail and log files are unchanged.`);
  }

  function sanitizeRuntimeLogForClipboard(text) {
    return sanitizeRuntimeLogForClipboardHelper(text);
  }

  async function copyHotRuntimeLogView() {
    await copySanitizedRuntimeLogToClipboard(
      String(elements.hotRuntimeLogOutput?.textContent || "").trim(),
    );
  }

  function inferHotRuntimeState(rendered) {
    return inferHotRuntimeStateHelper(rendered, elements.hotRuntimeMode?.textContent?.trim() || "local");
  }

  function renderHotRuntimeLogServiceLabel() {
    const label = currentHotRuntimeLogService();
    if (elements.observeHotLogService) {
      elements.observeHotLogService.textContent = label;
    }
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
    }, currentHotRuntimeLogInterval() || fallbackPollMs);
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
    }, fallbackPollMs);
    renderObserveRuntimeLogFollowState();
  }

  return {
    clearHotRuntimeLogView,
    copyHotRuntimeLogView,
    currentHotRuntimeLogService,
    currentHotRuntimeStatus,
    currentObserveRuntimeLogService,
    inferHotRuntimeState,
    persistCurrentHotLogSettings,
    persistCurrentObserveRuntimeLogSettings,
    renderHotRuntimeLogFollowState,
    renderHotRuntimeLogServiceLabel,
    renderObserveRuntimeLogFollowState,
    sanitizeRuntimeLogForClipboard,
    shouldPollHotRuntimeLog,
    shouldPollObserveRuntimeLog,
    stopHotRuntimeLogPolling,
    stopObserveRuntimeLogPolling,
    syncHotRuntimeLogPolling,
    syncObserveRuntimeLogPolling,
  };
}
