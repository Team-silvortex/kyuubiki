export function createRuntimeLogController({
  invoke,
  listen,
  logServiceSelect,
  liveTailToggle,
  renderRuntimeLog,
  showCompletion,
}) {
  let logRefreshTimer = null;
  let stopLogListener = null;
  let streamedService = null;

  async function refreshRuntimeLog() {
    const report = await invoke("read_runtime_log", { service: logServiceSelect.value });
    renderRuntimeLog(report.rendered || `${report.service} log is empty`);
    return `loaded ${report.service} log`;
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
          renderRuntimeLog(payload.rendered || `${service} log is empty`);
        }
      });
      await invoke("start_log_stream", { service });
      streamedService = service;
      showCompletion(`Live tail attached to ${service}.`);
      await refreshRuntimeLog();
    } catch (error) {
      if (stopLogListener) {
        stopLogListener();
        stopLogListener = null;
      }
      logRefreshTimer = window.setInterval(() => {
        refreshRuntimeLog().catch(() => {});
      }, 3000);
      showCompletion(`Live tail API unavailable. Falling back to timed refresh for ${service}.`);
    }
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
