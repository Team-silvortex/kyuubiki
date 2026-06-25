(function () {
  function targetLabel(target) {
    if (!target) {
      return "unknown";
    }
    const element = target.closest?.("[data-action], button, select, input, textarea, a") || target;
    const action = element.dataset?.action;
    const id = element.id;
    const text = element.textContent?.trim()?.replace(/\s+/g, " ")?.slice(0, 64);
    return action || id || text || element.tagName || "unknown";
  }

  function write(message, meta) {
    const messageNode = document.getElementById("hub-event-message");
    const metaNode = document.getElementById("hub-event-meta");
    if (messageNode) {
      messageNode.textContent = message;
    }
    if (metaNode) {
      metaNode.textContent = meta;
    }
  }

  function mark(kind, event) {
    const label = targetLabel(event.target);
    write(`${kind}: ${label}`, `boot-probe:${event.type}`);
  }

  function invoke(command, payload) {
    const tauriInvoke = window.__TAURI__?.core?.invoke;
    if (!tauriInvoke) {
      write("Tauri invoke unavailable.", "boot-probe:tauri-missing");
      return Promise.reject(new Error("Tauri invoke unavailable"));
    }
    return tauriInvoke(command, payload || {});
  }

  function fallbackForAction(action) {
    switch (action) {
      case "open-workbench":
        return () => invoke("launch_workbench_gui");
      case "open-installer":
        return () => invoke("launch_installer_gui");
      case "validate-env":
        return () => invoke("guarded_mutation_action", { payload: { action: "validate_env" } });
      case "start-local":
        return () => invoke("guarded_mutation_action", { payload: { action: "service_start", mode: "local" } });
      case "desktop-status":
        return () => invoke("desktop_status");
      case "run-doctor":
        return () => invoke("doctor_report");
      case "open-docs-index":
        return () => invoke("open_docs_index");
      case "open-current-line-doc":
        return () => invoke("open_current_line_doc");
      case "open-operations-doc":
        return () => invoke("open_operations_doc");
      case "open-troubleshooting-doc":
        return () => invoke("open_troubleshooting_doc");
      default:
        return null;
    }
  }

  function wasHandledByApp(action) {
    const startedAt = Number(window.__kyuubikiHubActionStartedAt || 0);
    const completedAt = Number(window.__kyuubikiHubActionCompletedAt || 0);
    const lastAction = window.__kyuubikiHubLastAction;
    const lastCompletedAction = window.__kyuubikiHubLastCompletedAction;
    const actionStarted =
      lastAction === action && Number.isFinite(startedAt) && Date.now() - startedAt < 1200;
    const actionCompleted =
      lastCompletedAction === action && Number.isFinite(completedAt) && Date.now() - completedAt < 1200;
    return actionStarted || actionCompleted;
  }

  function scheduleFallback(action) {
    const runner = fallbackForAction(action);
    if (!runner) {
      return;
    }

    window.setTimeout(async () => {
      if (wasHandledByApp(action)) {
        write(`app handled: ${action}`, "boot-probe:app-handled");
        return;
      }

      try {
        write(`fallback running: ${action}`, "boot-probe:fallback");
        const result = await runner();
        write(`fallback complete: ${action}`, typeof result === "string" ? result : "ok");
      } catch (error) {
        write(`fallback failed: ${action}`, error?.message || String(error));
      }
    }, 250);
  }

  function scheduleLanguageFallback(value) {
    window.setTimeout(async () => {
      if (window.__kyuubikiHubLanguageChangeAt && Date.now() - window.__kyuubikiHubLanguageChangeAt < 1200) {
        write(`app handled language: ${value}`, "boot-probe:app-handled");
        return;
      }

      try {
        write(`fallback language save: ${value}`, "boot-probe:fallback");
        const payload = await invoke("set_global_language_preference", {
          payload: { language: value || "en" },
        });
        write(`fallback language saved: ${payload?.language || value || "en"}`, "language:fallback");
      } catch (error) {
        write(`fallback language failed: ${value}`, error?.message || String(error));
      }
    }, 250);
  }

  window.addEventListener("error", (event) => {
    write(`js error: ${event.message || "unknown error"}`, "boot-probe:error");
  });

  window.addEventListener("unhandledrejection", (event) => {
    const reason = event.reason?.message || String(event.reason || "unknown rejection");
    write(`promise rejection: ${reason}`, "boot-probe:rejection");
  });

  document.addEventListener("pointerdown", (event) => mark("pointer", event), true);
  document.addEventListener(
    "click",
    (event) => {
      mark("click", event);
      const action = event.target?.closest?.("[data-action]")?.dataset?.action;
      if (action) {
        scheduleFallback(action);
      }
    },
    true,
  );
  document.addEventListener(
    "change",
    (event) => {
      mark("change", event);
      if (event.target?.id === "shell-language-select") {
        scheduleLanguageFallback(event.target.value);
      }
    },
    true,
  );

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      write("Boot probe mounted.", "boot-probe:ready");
      window.setTimeout(() => {
        if (!window.__kyuubikiHubAppReadyAt) {
          write("Hub app module did not report ready.", "boot-probe:app-missing");
        }
      }, 900);
    });
  } else {
    write("Boot probe mounted.", "boot-probe:ready");
    window.setTimeout(() => {
      if (!window.__kyuubikiHubAppReadyAt) {
        write("Hub app module did not report ready.", "boot-probe:app-missing");
      }
    }, 900);
  }
})();
