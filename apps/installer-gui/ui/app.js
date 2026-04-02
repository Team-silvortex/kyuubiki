(function () {
  const invoke = async (command, payload = {}) => {
    const tauri = window.__TAURI__;
    if (!tauri?.core?.invoke) {
      throw new Error("Tauri runtime is not available. Run this UI inside the desktop installer.");
    }

    return tauri.core.invoke(command, payload);
  };

  const output = document.getElementById("output");
  const serviceStatus = document.getElementById("service-status");
  const runtimeLog = document.getElementById("runtime-log");
  const completionBanner = document.getElementById("completion-banner");
  const completionMessage = document.getElementById("completion-message");
  const doctorGrid = document.getElementById("doctor-grid");
  const platformLabel = document.getElementById("platform-label");
  const workspaceLabel = document.getElementById("workspace-label");
  const currentModeLabel = document.getElementById("current-mode-label");
  const serviceModePill = document.getElementById("service-mode-pill");
  const completionGuide = document.getElementById("completion-guide");
  const liveTailToggle = document.getElementById("log-autorefresh");
  const logServiceSelect = document.getElementById("log-service");
  let logRefreshTimer = null;
  let stopLogListener = null;
  let streamedService = null;
  let latestLogSnapshot = "";

  async function listen(eventName, handler) {
    const tauri = window.__TAURI__;
    if (!tauri?.event?.listen) {
      throw new Error("Tauri event API is not available.");
    }

    return tauri.event.listen(eventName, handler);
  }

  function setOutput(value) {
    output.textContent = value;
  }

  function appendOutput(title, value) {
    const chunks = [output.textContent.trim(), `## ${title}\n${value}`].filter(Boolean);
    output.textContent = chunks.join("\n\n");
  }

  function showCompletion(message) {
    completionMessage.textContent = message;
    completionBanner.hidden = false;
    if (completionGuide) {
      completionGuide.textContent = message;
    }
  }

  function currentEnvPayload() {
    return {
      storageBackend: document.getElementById("storage-mode").value,
      sqliteDatabasePath: document.getElementById("sqlite-path").value.trim(),
      databaseUrl: document.getElementById("database-url").value.trim(),
      agentEndpoints: document.getElementById("agent-endpoints").value.trim(),
    };
  }

  function currentMode() {
    return document.getElementById("storage-mode").value || "sqlite";
  }

  function setModeCard(mode) {
    document.querySelectorAll("[data-mode-card]").forEach((card) => {
      card.classList.toggle("mode-card--active", card.dataset.modeCard === mode);
    });
    currentModeLabel.textContent = mode;
    serviceModePill.textContent = `${mode} profile`;
  }

  function hydrateEnv(form) {
    if (!form) return;
    document.getElementById("storage-mode").value = form.storage_backend || "sqlite";
    document.getElementById("sqlite-path").value =
      form.sqlite_database_path || "/Users/Shared/chroot/dev/kyuubiki/tmp/data/kyuubiki_dev.sqlite3";
    document.getElementById("database-url").value = form.database_url || "";
    document.getElementById("agent-endpoints").value =
      form.agent_endpoints || "127.0.0.1:5001,127.0.0.1:5002";
    setModeCard(form.storage_backend || "sqlite");
  }

  function applyPreset(mode) {
    const storageMode = document.getElementById("storage-mode");
    if (mode === "sqlite") {
      storageMode.value = "sqlite";
      if (!document.getElementById("sqlite-path").value.trim()) {
        document.getElementById("sqlite-path").value =
          "/Users/Shared/chroot/dev/kyuubiki/tmp/data/kyuubiki_dev.sqlite3";
      }
    } else {
      storageMode.value = "postgres";
      if (!document.getElementById("database-url").value.trim()) {
        document.getElementById("database-url").value =
          "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev";
      }
    }
    if (!document.getElementById("agent-endpoints").value.trim()) {
      document.getElementById("agent-endpoints").value = "127.0.0.1:5001,127.0.0.1:5002";
    }
    setModeCard(mode);
  }

  function renderDoctor(report) {
    platformLabel.textContent = report.platform;
    workspaceLabel.textContent = report.workspace;
    doctorGrid.innerHTML = "";

    report.checks.forEach((check) => {
      const card = document.createElement("article");
      card.className = "doctor-card";
      card.innerHTML = `
        <strong>${check.label}</strong>
        <span class="doctor-state ${check.ok ? "ok" : "missing"}">${check.ok ? "ok" : "missing"}</span>
      `;
      doctorGrid.appendChild(card);
    });
  }

  function renderServiceStatus(rendered) {
    serviceStatus.textContent = rendered;
  }

  function renderRuntimeLog(rendered) {
    latestLogSnapshot = rendered;
    runtimeLog.textContent = rendered;
    runtimeLog.scrollTop = runtimeLog.scrollHeight;
  }

  async function runAction(name, callback, options = {}) {
    try {
      const result = await callback();
      if (typeof result === "string" && !options.skipOutput) {
        appendOutput(name, result);
      }
      return result;
    } catch (error) {
      const message = error.message || String(error);
      appendOutput(name, message);
      throw error;
    }
  }

  async function refreshDoctor() {
    const report = await invoke("doctor_report");
    renderDoctor(report);
    return report.rendered;
  }

  async function refreshServiceStatus() {
    const report = await invoke("service_status");
    renderServiceStatus(report.rendered);
    return report.rendered;
  }

  async function refreshRuntimeLog() {
    const report = await invoke("read_runtime_log", {
      service: logServiceSelect.value,
    });
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

  document.querySelectorAll(".sidebar-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document.querySelectorAll(".sidebar-tab").forEach((item) => item.classList.remove("active"));
      document.querySelectorAll(".panel").forEach((panel) => panel.classList.remove("panel-visible"));
      tab.classList.add("active");
      document.querySelector(`[data-panel="${tab.dataset.tab}"]`)?.classList.add("panel-visible");
    });
  });

  document.getElementById("storage-mode").addEventListener("change", (event) => {
    setModeCard(event.target.value);
  });

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

  document.querySelectorAll("[data-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const action = button.dataset.action;

      switch (action) {
        case "doctor":
          await runAction("doctor", refreshDoctor);
          break;
        case "prepare-layout":
          await runAction("prepare-layout", () => invoke("prepare_layout"));
          break;
        case "bootstrap":
          await runAction("bootstrap", async () => {
            const result = await invoke("bootstrap");
            await refreshDoctor();
            showCompletion("Bootstrap complete. You can validate env or start services next.");
            return result;
          });
          break;
        case "init-env":
          await runAction("init-env", async () => {
            const result = await invoke("init_env", { force: false });
            hydrateEnv(await invoke("read_env_file"));
            return result;
          });
          break;
        case "validate-env":
          await runAction("validate-env", () => invoke("validate_env"));
          break;
        case "write-env":
          await runAction("write-env", async () => {
            const result = await invoke("write_env_file", currentEnvPayload());
            hydrateEnv(await invoke("read_env_file"));
            showCompletion("Environment saved. Next step: validate and start the active profile.");
            return result;
          });
          break;
        case "reload-env":
          await runAction("reload-env", async () => {
            hydrateEnv(await invoke("read_env_file"));
            return "reloaded current environment";
          });
          break;
        case "use-local-mode":
          applyPreset("sqlite");
          showCompletion("Local SQLite profile selected.");
          appendOutput("mode", "selected local SQLite profile");
          break;
        case "use-cloud-mode":
          applyPreset("postgres");
          showCompletion("Cloud PostgreSQL profile selected.");
          appendOutput("mode", "selected cloud PostgreSQL profile");
          break;
        case "service-status":
          await runAction("service-status", refreshServiceStatus);
          break;
        case "service-start-local":
          await runAction("service-start-local", async () => {
            const result = await invoke("service_start", { mode: "local" });
            await refreshServiceStatus();
            showCompletion("Local services started.");
            return result;
          });
          break;
        case "service-restart-local":
          await runAction("service-restart-local", async () => {
            const result = await invoke("service_restart", { mode: "local" });
            await refreshServiceStatus();
            showCompletion("Local services restarted.");
            return result;
          });
          break;
        case "service-start-cloud":
          await runAction("service-start-cloud", async () => {
            const result = await invoke("service_start", { mode: "cloud" });
            await refreshServiceStatus();
            showCompletion("Cloud services started.");
            return result;
          });
          break;
        case "service-restart-cloud":
          await runAction("service-restart-cloud", async () => {
            const result = await invoke("service_restart", { mode: "cloud" });
            await refreshServiceStatus();
            showCompletion("Cloud services restarted.");
            return result;
          });
          break;
        case "service-stop":
          await runAction("service-stop", async () => {
            const result = await invoke("service_stop");
            await refreshServiceStatus();
            showCompletion("All services stopped.");
            return result;
          });
          break;
        case "load-log":
          await runAction("load-log", async () => {
            if (liveTailToggle.checked) {
              await startRuntimeLogStream();
              return `attached live tail to ${logServiceSelect.value}`;
            }
            return refreshRuntimeLog();
          });
          break;
        case "wizard-start-active":
          await runAction("wizard-start-active", async () => {
            const mode = currentMode() === "postgres" ? "cloud" : "local";
            const result = await invoke("service_start", { mode });
            await refreshServiceStatus();
            showCompletion(`Started ${mode} profile.`);
            return result;
          });
          break;
        case "stage-release":
          await runAction("stage-release", () =>
            invoke("stage_release", {
              platform: document.getElementById("release-platform").value,
              targetDir: document.getElementById("release-target").value.trim() || null,
            }),
          );
          showCompletion("Release scaffold staged. You can export launch config or build the installer next.");
          break;
        case "build-installer":
          await runAction("build-installer", () =>
            invoke("build_installer_bundle", {
              bundleMode:
                document.getElementById("release-build-mode")?.value ||
                document.getElementById("build-mode")?.value ||
                "debug-check",
            }),
          );
          showCompletion("Installer build completed.");
          break;
        case "export-launch":
          await runAction("export-launch", () =>
            invoke("export_launch", {
              platform: document.getElementById("release-platform").value,
            }),
          );
          break;
        case "clear-output":
          setOutput("");
          break;
        default:
          break;
      }
    });
  });

  runAction(
    "startup",
    async () => {
      const [doctor, envForm, status] = await Promise.all([
        invoke("doctor_report"),
        invoke("read_env_file").catch(() => null),
        invoke("service_status").catch(() => ({ rendered: "service status unavailable" })),
      ]);

      renderDoctor(doctor);
      if (envForm) {
        hydrateEnv(envForm);
      } else {
        applyPreset("sqlite");
      }
      renderServiceStatus(status.rendered);
      await refreshRuntimeLog().catch(() => {
        renderRuntimeLog("runtime log unavailable");
      });
      if (liveTailToggle.checked) {
        await startRuntimeLogStream().catch(() => {});
      }
      showCompletion("Installer GUI ready. Pick a profile, write env, then start services and watch live logs here.");

      return "installer gui ready";
    },
    { skipOutput: false },
  );
})();
