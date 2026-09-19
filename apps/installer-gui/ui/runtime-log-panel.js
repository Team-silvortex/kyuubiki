import { builtinInstallerSetupCopy } from "./installer-setup-copy.js";

export function createRuntimeLogController({
  invoke,
  listen,
  logServiceSelect,
  liveTailToggle,
  renderRuntimeLog,
  showCompletion,
  getCopy = () => builtinInstallerSetupCopy.en,
}) {
  let logRefreshTimer = null;
  let stopLogListener = null;
  let streamedService = null;
  const message = (key, service) => getCopy()[key].replaceAll("{service}", () => service);

  async function refreshRuntimeLog() {
    const report = await invoke("read_runtime_log", { service: logServiceSelect.value });
    renderRuntimeLog(report.rendered || message("logEmpty", report.service));
    return message("logLoaded", report.service);
  }

  async function stopRuntimeLogStream() {
    if (logRefreshTimer) {
      clearInterval(logRefreshTimer);
      logRefreshTimer = null;
    }
    if (streamedService) {
      await invoke("stop_log_stream", { service: streamedService }).catch(() => {});
      streamedService = null;
    }
    if (stopLogListener) {
      stopLogListener();
      stopLogListener = null;
    }
  }

  async function startRuntimeLogStream() {
    await stopRuntimeLogStream();
    const service = logServiceSelect.value;

    try {
      stopLogListener = await listen("runtime-log-update", (event) => {
        const payload = event.payload || {};
        if (payload.service === service) {
          renderRuntimeLog(payload.rendered || message("logEmpty", service));
        }
      });
      await invoke("start_log_stream", { service });
      streamedService = service;
      await refreshRuntimeLog();
    } catch (error) {
      await stopRuntimeLogStream();
      logRefreshTimer = window.setInterval(() => {
        refreshRuntimeLog().catch(() => {});
      }, 3000);
      const status = message("logPolling", service);
      showCompletion(status);
      return status;
    }
    const status = message("logAttached", service);
    showCompletion(status);
    return status;
  }

  liveTailToggle.addEventListener("change", async (event) => {
    if (event.target.checked) {
      await startRuntimeLogStream();
    } else {
      await stopRuntimeLogStream();
      await refreshRuntimeLog().catch(() => {});
    }
  });

  logServiceSelect.addEventListener("change", async () => {
    if (liveTailToggle.checked) {
      await startRuntimeLogStream();
    } else {
      await refreshRuntimeLog().catch(() => {});
    }
  });

  return {
    refreshRuntimeLog,
    startRuntimeLogStream,
    stopRuntimeLogStream,
  };
}
