import { formatRuntimeStatusReport, renderRuntimeStatusPlane } from "./shared/runtime-status-summary.js";

export function sanitizeRuntimeLogForClipboard(text) {
  return String(text || "")
    .replace(/(authorization\s*[:=]\s*)(bearer\s+)?([^\s]+)/giu, "$1[redacted]")
    .replace(/(api[_-]?key\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(token\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(password\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]")
    .replace(/(secret\s*[:=]\s*)([^\s]+)/giu, "$1[redacted]");
}

export async function copySanitizedRuntimeLogToClipboard(text) {
  await navigator.clipboard.writeText(sanitizeRuntimeLogForClipboard(text));
}

export function inferHotRuntimeState(rendered, fallbackMode = "local") {
  const text = String(rendered || "");
  const running = /hot-loop:\s+running/i.test(text);
  const stopped = /hot-loop:\s+stopped/i.test(text);
  const modeMatch =
    /started managed hot-reload loop \((cloud|distributed|local)\)/i.exec(text) ||
    /Mode\W*(cloud|distributed|local)/i.exec(text);

  return {
    status: running ? "running" : stopped ? "idle" : "unknown",
    mode: modeMatch?.[1] || fallbackMode,
  };
}

async function fetchMeshRuntimeHealth(orchestratorBaseUrl) {
  const baseUrl = String(orchestratorBaseUrl || "").trim().replace(/\/+$/u, "");
  if (!baseUrl) return null;

  const response = await fetch(`${baseUrl}/api/health`);
  if (!response.ok) {
    throw new Error(`mesh runtime health request failed with ${response.status}`);
  }

  return response.json();
}

export async function refreshRuntimeStatusPanel({
  invokeTauri,
  orchestratorBaseUrl,
  setRuntimeStatusOutput,
  applyDesktopState,
  localRuntimeStatus,
  observeRuntimeStatus,
  runtimeStatusPlane,
}) {
  try {
    const payload = await invokeTauri("service_status");
    const meshRuntime = await fetchMeshRuntimeHealth(orchestratorBaseUrl).catch(() => null);
    renderRuntimeStatusPlane(runtimeStatusPlane, payload.summary, meshRuntime);
    setRuntimeStatusOutput(
      formatRuntimeStatusReport({
        title: "Kyuubiki Hub Runtime",
        rendered: payload.rendered,
        summary: payload.summary,
      }, meshRuntime),
    );
    applyDesktopState(localRuntimeStatus, payload.rendered, { kind: "health" });
    applyDesktopState(observeRuntimeStatus, payload.rendered, { kind: "health" });
  } catch (error) {
    const message = String(error);
    renderRuntimeStatusPlane(runtimeStatusPlane, null);
    setRuntimeStatusOutput(message);
    applyDesktopState(localRuntimeStatus, message, { kind: "health" });
    applyDesktopState(observeRuntimeStatus, message, { kind: "health" });
  }
}

export async function refreshHotRuntimeStatusPanel({
  invokeTauri,
  setHotRuntimeStatusOutput,
  applyDesktopState,
  hotRuntimeStatus,
  observeHotStatus,
  hotRuntimeMode,
  observeHotMode,
  syncHotRuntimeLogPolling,
  refreshHotRuntimeLog,
}) {
  try {
    const payload = await invokeTauri("hot_service_status");
    setHotRuntimeStatusOutput(payload.rendered);
    const inferred = inferHotRuntimeState(
      payload.rendered,
      hotRuntimeMode?.textContent?.trim() || "local",
    );
    applyDesktopState(hotRuntimeStatus, inferred.status, { kind: "activity" });
    applyDesktopState(observeHotStatus, inferred.status, { kind: "activity" });
    if (hotRuntimeMode) {
      hotRuntimeMode.textContent = inferred.mode;
    }
    if (observeHotMode) {
      observeHotMode.textContent = inferred.mode;
    }
    syncHotRuntimeLogPolling();
    await refreshHotRuntimeLog({ silent: true });
  } catch (error) {
    setHotRuntimeStatusOutput(String(error));
    applyDesktopState(hotRuntimeStatus, "failed", { kind: "activity" });
    applyDesktopState(observeHotStatus, "failed", { kind: "activity" });
    syncHotRuntimeLogPolling();
  }
}

export async function refreshRuntimeLogPanel({
  invokeTauri,
  state,
  inFlightKey,
  service,
  silent = false,
  setOutput,
  hubDynamic,
  formatHubOperatorError,
}) {
  if (state[inFlightKey]) {
    return;
  }

  state[inFlightKey] = true;

  try {
    const payload = await invokeTauri("read_runtime_log", {
      payload: { service },
    });
    const rendered = String(payload?.rendered || "").trim();
    setOutput(rendered || hubDynamic("noLogLines", { service }));
  } catch (error) {
    if (!silent) {
      setOutput(
        formatHubOperatorError(error, {
          actionLabel: "Reading runtime logs",
          context: "log-read",
          service,
        }),
      );
    }
  } finally {
    state[inFlightKey] = false;
  }
}

export async function refreshDesktopStatusPanel({
  invokeTauri,
  platform,
  setDesktopStatusOutput,
  formatHubOperatorError,
}) {
  try {
    setDesktopStatusOutput(
      await invokeTauri("desktop_status", {
        payload: { platform },
      }),
    );
  } catch (error) {
    setDesktopStatusOutput(
      formatHubOperatorError(error, {
        actionLabel: "Refreshing desktop packaging status",
        context: "desktop-status",
      }),
    );
  }
}
