import { applyDesktopState, invokeTauri as invoke, listenTauri as listen, loadDesktopBrand, loadDesktopLanguagePreference, normalizeDesktopLanguage, saveDesktopLanguagePreference, setText, syncDesktopStates } from "./shared/tauri-bridge.js";
import { desktopReleaseRootPattern, normalizeDesktopPlatform, populateDesktopPlatformSelect, syncDesktopReleaseTargetInput } from "./shared/platform.js";
import { applyPreset, currentRemoteAgentPayload, currentRemoteBootstrapPayload, renderDoctor } from "./installer-workflows.js";
import { mountIntegrityPanel, renderIntegrityReport } from "./integrity-panel.js";
import { currentUpdateSourcePayload, hydrateUpdateSourceConfig, mountUpdatePanel, renderLatestAppliedUpdate, renderLatestDownloadedUpdate, renderLatestStagedUpdate, renderUpdatePlan, renderUpdatePreview, selectedUpdateChannel } from "./update-panel.js";
import { formatRuntimeStatusReport, renderRuntimeStatusPlane } from "./shared/runtime-status-summary.js";
(function () {
  const DEFAULT_AGENT_MANIFEST_PATH = "./deploy/agents.local.example.json", DEFAULT_DISTRIBUTED_AGENT_MANIFEST_PATH = "./deploy/agents.distributed.example.json", DEFAULT_SQLITE_DATABASE_PATH = "./tmp/data/kyuubiki_dev.sqlite3";
  const DEFAULT_PRESET = {
    agentManifestPath: DEFAULT_AGENT_MANIFEST_PATH,
    distributedAgentManifestPath: DEFAULT_DISTRIBUTED_AGENT_MANIFEST_PATH,
    sqliteDatabasePath: DEFAULT_SQLITE_DATABASE_PATH,
  };
  const output = document.getElementById("output"), serviceStatus = document.getElementById("service-status"), serviceStatusPlane = document.getElementById("service-status-plane"), runtimeLog = document.getElementById("runtime-log"), completionBanner = document.getElementById("completion-banner"), completionMessage = document.getElementById("completion-message"), doctorGrid = document.getElementById("doctor-grid"), platformLabel = document.getElementById("platform-label"), releasePlatformSelect = document.getElementById("release-platform"), releaseTargetInput = document.getElementById("release-target"), workspaceLabel = document.getElementById("workspace-label"), currentModeLabel = document.getElementById("current-mode-label"), languageLabel = document.getElementById("shell-language-label"), languageSelect = document.getElementById("shell-language-select"), serviceModePill = document.getElementById("service-mode-pill"), completionGuide = document.getElementById("completion-guide"), liveTailToggle = document.getElementById("log-autorefresh");
  const logServiceSelect = document.getElementById("log-service"); let logRefreshTimer = null, stopLogListener = null, streamedService = null, latestLogSnapshot = "", brandConfig = null, currentLanguage = "en";
  const sensitiveEnvFieldIds = ["database-url", "api-token", "cluster-api-token", "direct-mesh-token"];

  mountIntegrityPanel();
  mountUpdatePanel();
  populateDesktopPlatformSelect(releasePlatformSelect);

  function releaseLabel() {
    const releaseVersion = String(brandConfig?.releaseVersion || "").replace(/^v/u, "");
    const releaseCodename = String(brandConfig?.releaseCodename || "").trim();
    const releaseTag = [releaseCodename, releaseVersion].filter(Boolean).join(" ");
    return releaseTag ? `Kyuubiki Installer · ${releaseTag}` : "Kyuubiki Installer";
  }

  function renderDesktopLanguagePreference() {
    document.documentElement.lang = currentLanguage;
    if (languageLabel) {
      languageLabel.textContent =
        currentLanguage === "zh" ? "语言" : currentLanguage === "ja" ? "言語" : currentLanguage === "es" ? "Idioma" : "Language";
    }
    if (languageSelect) {
      languageSelect.value = currentLanguage;
    }
  }

  function applyBrandConfig(brand) {
    brandConfig = brand;
    if (brand?.installerName) {
      const releaseVersion = String(brand.releaseVersion || "").replace(/^v/u, "");
      const releaseCodename = String(brand.releaseCodename || "").trim();
      const releaseTag = [releaseCodename, releaseVersion].filter(Boolean).join(" ");
      document.title = releaseTag ? `${brand.installerName} · ${releaseTag}` : brand.installerName;
    }

    setText("brand-page-title", brand?.installerName);
    setText("brand-desktop-setup", brand?.desktopSetupLabel);
    setText("brand-engine-tagline", brand?.engineTagline);
    setText("brand-installer-role-chip", brand?.shellRoleLabel);
    setText("brand-installer-name", brand?.installerShortName || "Installer");
    setText("brand-installer-description", brand?.installerDescription);
    setText("brand-product-name", brand?.installerShortName || "Installer");
    setText("brand-installer-console", brand?.installerConsoleName);
    if (brand?.releaseVersion || brand?.releaseCodename) {
      const releaseTag = [
        String(brand.releaseCodename || "").trim(),
        String(brand.releaseVersion || "").replace(/^v/u, ""),
      ]
        .filter(Boolean)
        .join(" ");
      setText("brand-installer-version", releaseTag);
    }
  }

  languageSelect?.addEventListener("change", async (event) => {
    currentLanguage = await saveDesktopLanguagePreference(normalizeDesktopLanguage(event.target.value));
    renderDesktopLanguagePreference();
  });

  function setOutput(value) {
    output.textContent = value;
  }

  function invokeGuardedMutation(action, payload = {}) {
    return invoke("guarded_mutation_action", {
      payload: {
        action,
        ...payload,
      },
    });
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
    const databaseUrlInput = document.getElementById("database-url");
    const apiTokenInput = document.getElementById("api-token");
    const clusterApiTokenInput = document.getElementById("cluster-api-token");
    const directMeshTokenInput = document.getElementById("direct-mesh-token");
    return {
      deploymentMode: document.getElementById("deployment-mode").value,
      agentDiscovery: document.getElementById("agent-discovery").value,
      agentManifestPath: document.getElementById("agent-manifest-path").value.trim(),
      storageBackend: document.getElementById("storage-mode").value,
      sqliteDatabasePath: document.getElementById("sqlite-path").value.trim(),
      databaseUrl: databaseUrlInput.value.trim(),
      databaseUrlConfigured: databaseUrlInput.dataset.configured === "true",
      agentEndpoints: document.getElementById("agent-endpoints").value.trim(),
      kyuubikiApiToken: apiTokenInput.value.trim(),
      kyuubikiApiTokenConfigured: apiTokenInput.dataset.configured === "true",
      kyuubikiClusterApiToken: clusterApiTokenInput.value.trim(),
      kyuubikiClusterApiTokenConfigured: clusterApiTokenInput.dataset.configured === "true",
      kyuubikiClusterAllowedAgentIds:
        document.getElementById("cluster-allowed-agent-ids").value.trim(),
      kyuubikiClusterAllowedClusterIds:
        document.getElementById("cluster-allowed-cluster-ids").value.trim(),
      kyuubikiClusterRequireFingerprint:
        document.getElementById("cluster-require-fingerprint").value === "true",
      kyuubikiClusterTimestampWindowMs:
        document.getElementById("cluster-timestamp-window").value.trim() || "30000",
      kyuubikiProtectReads: document.getElementById("protect-reads").value === "true",
      kyuubikiDirectMeshEnabled: document.getElementById("direct-mesh-enabled").value === "true",
      kyuubikiDirectMeshToken: directMeshTokenInput.value.trim(),
      kyuubikiDirectMeshTokenConfigured: directMeshTokenInput.dataset.configured === "true",
    };
  }

  function setSensitiveFieldState(input, configured, placeholders) {
    if (!input) return;
    input.value = "";
    input.dataset.configured = configured ? "true" : "false";
    input.placeholder = configured ? placeholders.configured : placeholders.empty;
  }

  function currentMode() {
    return document.getElementById("deployment-mode").value || "local";
  }

  function setModeCard(mode) {
    document.querySelectorAll("[data-mode-card]").forEach((card) => {
      card.classList.toggle("mode-card--active", card.dataset.modeCard === mode);
    });
    applyDesktopState(currentModeLabel, mode, { kind: "activity" });
    applyDesktopState(serviceModePill, `${mode} profile`, { kind: "activity" });
  }

  function hydrateEnv(form) {
    if (!form) return;
    document.getElementById("deployment-mode").value = form.deployment_mode || "local";
    document.getElementById("agent-discovery").value = form.agent_discovery || "static";
    document.getElementById("agent-manifest-path").value =
      form.agent_manifest_path || DEFAULT_AGENT_MANIFEST_PATH;
    document.getElementById("storage-mode").value = form.storage_backend || "sqlite";
    document.getElementById("sqlite-path").value =
      form.sqlite_database_path || DEFAULT_SQLITE_DATABASE_PATH;
    setSensitiveFieldState(document.getElementById("database-url"), form.database_url_configured === true, {
      configured: "configured; leave blank to keep current value",
      empty: "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev",
    });
    document.getElementById("agent-endpoints").value =
      form.agent_endpoints || "127.0.0.1:5001,127.0.0.1:5002";
    setSensitiveFieldState(document.getElementById("api-token"), form.kyuubiki_api_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional shared token",
    });
    setSensitiveFieldState(document.getElementById("cluster-api-token"), form.kyuubiki_cluster_api_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional cluster-only token",
    });
    document.getElementById("cluster-allowed-agent-ids").value =
      form.kyuubiki_cluster_allowed_agent_ids || "";
    document.getElementById("cluster-allowed-cluster-ids").value =
      form.kyuubiki_cluster_allowed_cluster_ids || "";
    document.getElementById("cluster-require-fingerprint").value =
      form.kyuubiki_cluster_require_fingerprint ? "true" : "false";
    document.getElementById("cluster-timestamp-window").value =
      form.kyuubiki_cluster_timestamp_window_ms || "30000";
    document.getElementById("protect-reads").value = form.kyuubiki_protect_reads ? "true" : "false";
    document.getElementById("direct-mesh-enabled").value = form.kyuubiki_direct_mesh_enabled === false ? "false" : "true";
    setSensitiveFieldState(document.getElementById("direct-mesh-token"), form.kyuubiki_direct_mesh_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional direct-mesh token",
    });
    setModeCard(form.deployment_mode || "local");
  }

  function renderServiceStatus(report) {
    renderRuntimeStatusPlane(serviceStatusPlane, report?.summary);
    serviceStatus.textContent = formatRuntimeStatusReport({
      title: releaseLabel(),
      rendered: report?.rendered,
      summary: report?.summary,
    });
  }

  function renderRuntimeLog(rendered) {
    latestLogSnapshot = rendered;
    runtimeLog.textContent = rendered;
    runtimeLog.scrollTop = runtimeLog.scrollHeight;
  }

  function syncReleaseTarget(platform = releasePlatformSelect?.value) {
    syncDesktopReleaseTargetInput(
      releaseTargetInput,
      normalizeDesktopPlatform(platform),
    );
    if (releaseTargetInput && !releaseTargetInput.dataset.desktopPlaceholderBound) {
      releaseTargetInput.placeholder = desktopReleaseRootPattern();
      releaseTargetInput.dataset.desktopPlaceholderBound = "true";
    }
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
    renderDoctor(report, platformLabel, workspaceLabel, doctorGrid);
    return report.rendered;
  }

  async function refreshIntegrityReport() {
    const report = await invoke("installation_integrity_report");
    renderIntegrityReport(report, brandConfig);
    return report.rendered;
  }

  async function refreshUpdatePlan() {
    const report = await invoke("unified_update_plan", { channel: selectedUpdateChannel() });
    renderUpdatePlan(report);
    return report.rendered;
  }

  async function refreshUpdateSourceConfig() { const config = await invoke("update_source_config"); hydrateUpdateSourceConfig(config); return config.rendered; }

  async function refreshUpdatePreview() { const report = await invoke("unified_update_preview", { channel: selectedUpdateChannel() }); renderUpdatePreview(report); return report.rendered; }

  async function refreshLatestStagedUpdate() {
    const record = await invoke("latest_staged_update_record").catch(() => null); renderLatestStagedUpdate(record); return record?.rendered || "no staged update record";
  }

  async function refreshLatestDownloadedUpdate() { const record = await invoke("latest_downloaded_update_record").catch(() => null); renderLatestDownloadedUpdate(record); return record?.rendered || "no downloaded update record"; }
  async function refreshLatestAppliedUpdate() { const record = await invoke("latest_applied_update_record").catch(() => null); renderLatestAppliedUpdate(record); return record?.rendered || "no applied update record"; }

  async function refreshUpdateState() {
    const [plan, preview, latest, downloaded, applied] = await Promise.all([refreshUpdatePlan(), refreshUpdatePreview(), refreshLatestStagedUpdate(), refreshLatestDownloadedUpdate(), refreshLatestAppliedUpdate()]);
    return [plan, preview, latest, downloaded, applied].filter(Boolean).join("\n\n");
  }

  async function refreshServiceStatus() {
    const report = await invoke("service_status");
    renderServiceStatus(report);
    return report.rendered;
  }

  async function refreshRuntimeLog() { const report = await invoke("read_runtime_log", { service: logServiceSelect.value }); renderRuntimeLog(report.rendered || `${report.service} log is empty`); return `loaded ${report.service} log`; }

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

  releasePlatformSelect?.addEventListener("change", (event) => {
    syncReleaseTarget(event.target.value);
  });

  sensitiveEnvFieldIds.forEach((id) => {
    const input = document.getElementById(id);
    input?.addEventListener("input", () => {
      input.dataset.configured = "false";
    });
  });

  document.querySelectorAll("[data-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      const action = button.dataset.action;

      switch (action) {
        case "doctor":
          await runAction("doctor", refreshDoctor);
          break;
        case "prepare-layout":
          await runAction("prepare-layout", () => invokeGuardedMutation("prepare_layout"));
          break;
        case "bootstrap":
          await runAction("bootstrap", async () => {
            const result = await invokeGuardedMutation("bootstrap");
            await refreshDoctor();
            showCompletion("Bootstrap complete. You can validate env or start services next.");
            return result;
          });
          break;
        case "init-env":
          await runAction("init-env", async () => {
            const result = await invokeGuardedMutation("init_env", { force: false });
            hydrateEnv(await invoke("read_env_file"));
            return result;
          });
          break;
        case "validate-env":
          await runAction("validate-env", () => invokeGuardedMutation("validate_env"));
          break;
        case "refresh-integrity":
          await runAction("refresh-integrity", refreshIntegrityReport);
          break;
        case "repair-installation":
          await runAction("repair-installation", async () => {
            const result = await invokeGuardedMutation("repair_installation");
            await refreshIntegrityReport();
            showCompletion("Installation contract repaired and residue cleanup completed.");
            return result;
          });
          break;
        case "refresh-update-plan":
          await runAction("refresh-update-plan", refreshUpdatePlan);
          break;
        case "refresh-update-source": await runAction("refresh-update-source", refreshUpdateSourceConfig); break;
        case "refresh-update-preview":
          await runAction("refresh-update-preview", refreshUpdatePreview);
          break;
        case "save-update-source": await runAction("save-update-source", async () => { const result = await invokeGuardedMutation("write_update_source_config", currentUpdateSourcePayload()); await refreshUpdateSourceConfig(); showCompletion("Update source saved. Refresh the channel plan to validate the selected catalog."); return result; }); break;
        case "download-update": await runAction("download-update", async () => { const result = await invokeGuardedMutation("download_update", { channel: selectedUpdateChannel(), platform: releasePlatformSelect?.value || "macos" }); await refreshLatestDownloadedUpdate(); showCompletion("Selected channel artifacts downloaded into the configured update cache."); return result; }); break;
        case "refresh-downloaded-update": await runAction("refresh-downloaded-update", refreshLatestDownloadedUpdate); break;
        case "apply-downloaded-update": await runAction("apply-downloaded-update", async () => { const result = await invokeGuardedMutation("apply_downloaded_update"); await refreshLatestAppliedUpdate(); showCompletion("Downloaded update promoted into the applied-update handoff record."); return result; }); break;
        case "refresh-applied-update": await runAction("refresh-applied-update", refreshLatestAppliedUpdate); break;
        case "refresh-staged-update": await runAction("refresh-staged-update", refreshLatestStagedUpdate); break;
        case "prepare-update":
        case "reprepare-update":
          await runAction("prepare-update", async () => {
            const result = await invokeGuardedMutation("prepare_staged_update", { channel: selectedUpdateChannel(), platform: releasePlatformSelect?.value || "macos", targetDir: releaseTargetInput?.value.trim() || null });
            await Promise.all([refreshIntegrityReport(), refreshUpdateState()]);
            showCompletion("Staged update prepared. Review the refreshed integrity and channel state before distributing artifacts.");
            return result;
          });
          break;
        case "write-env":
          await runAction("write-env", async () => {
            const result = await invokeGuardedMutation("write_env_file", {
              envPayload: currentEnvPayload(),
            });
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
          applyPreset("local", DEFAULT_PRESET);
          setModeCard("local");
          showCompletion("Local SQLite profile selected.");
          appendOutput("mode", "selected local SQLite profile");
          break;
        case "use-cloud-mode":
          applyPreset("cloud", DEFAULT_PRESET);
          setModeCard("cloud");
          showCompletion("Cloud PostgreSQL profile selected.");
          appendOutput("mode", "selected cloud PostgreSQL profile");
          break;
        case "use-distributed-mode":
          applyPreset("distributed", DEFAULT_PRESET);
          setModeCard("distributed");
          showCompletion("Distributed control-plane profile selected.");
          appendOutput("mode", "selected distributed control-plane profile");
          break;
        case "service-status":
          await runAction("service-status", refreshServiceStatus);
          break;
        case "service-start-local":
          await runAction("service-start-local", async () => {
            const result = await invokeGuardedMutation("service_start", { mode: "local" });
            await refreshServiceStatus();
            showCompletion("Local services started.");
            return result;
          });
          break;
        case "service-restart-local":
          await runAction("service-restart-local", async () => {
            const result = await invokeGuardedMutation("service_restart", { mode: "local" });
            await refreshServiceStatus();
            showCompletion("Local services restarted.");
            return result;
          });
          break;
        case "service-start-cloud":
          await runAction("service-start-cloud", async () => {
            const result = await invokeGuardedMutation("service_start", { mode: "cloud" });
            await refreshServiceStatus();
            showCompletion("Cloud services started.");
            return result;
          });
          break;
        case "service-start-distributed":
          await runAction("service-start-distributed", async () => {
            const result = await invokeGuardedMutation("service_start", { mode: "distributed" });
            await refreshServiceStatus();
            showCompletion("Distributed control plane started.");
            return result;
          });
          break;
        case "service-restart-cloud":
          await runAction("service-restart-cloud", async () => {
            const result = await invokeGuardedMutation("service_restart", { mode: "cloud" });
            await refreshServiceStatus();
            showCompletion("Cloud services restarted.");
            return result;
          });
          break;
        case "service-stop":
          await runAction("service-stop", async () => {
            const result = await invokeGuardedMutation("service_stop");
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
            const mode = currentMode() === "distributed" ? "distributed" : currentMode() === "cloud" ? "cloud" : "local";
            const result = await invokeGuardedMutation("service_start", { mode });
            await refreshServiceStatus();
            showCompletion(`Started ${mode} profile.`);
            return result;
          });
          break;
        case "remote-bootstrap":
          await runAction("remote-bootstrap", async () => {
            const result = await invokeGuardedMutation("remote_bootstrap", {
              remoteBootstrap: currentRemoteBootstrapPayload(),
            });
            showCompletion("Remote workspace bootstrapped.");
            return result;
          });
          break;
        case "remote-start-agent":
          await runAction("remote-start-agent", async () => {
            const result = await invokeGuardedMutation("remote_start_agent", {
              remoteAgent: currentRemoteAgentPayload(),
            });
            showCompletion("Remote solver agent started.");
            return result;
          });
          break;
        case "stage-release":
          await runAction("stage-release", () =>
            invokeGuardedMutation("stage_release", {
              platform: document.getElementById("release-platform").value,
              targetDir: document.getElementById("release-target").value.trim() || null,
            }),
          );
          showCompletion("Release scaffold staged. You can export launch config or build the installer next.");
          break;
        case "build-installer":
          await runAction("build-installer", () =>
            invokeGuardedMutation("build_installer_bundle", {
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
      const [doctor, integrityReport, updatePlan, updateSource, updatePreview, downloadedUpdate, appliedUpdate, stagedUpdate, envForm, status, language, brand] = await Promise.all([invoke("doctor_report"), invoke("installation_integrity_report").catch(() => null), invoke("unified_update_plan", { channel: "stable" }).catch(() => null), invoke("update_source_config").catch(() => null), invoke("unified_update_preview", { channel: "stable" }).catch(() => null), invoke("latest_downloaded_update_record").catch(() => null), invoke("latest_applied_update_record").catch(() => null), invoke("latest_staged_update_record").catch(() => null), invoke("read_env_file").catch(() => null), invoke("service_status").catch(() => ({ rendered: "service status unavailable" })), loadDesktopLanguagePreference().catch(() => "en"), loadDesktopBrand().catch(() => null)]);

      currentLanguage = language || currentLanguage;
      renderDesktopLanguagePreference();

      if (brand) {
        applyBrandConfig(brand);
      }
      syncDesktopStates();
      renderDoctor(doctor, platformLabel, workspaceLabel, doctorGrid);
      if (integrityReport) {
        renderIntegrityReport(integrityReport, brand);
      }
      if (updatePlan) {
        renderUpdatePlan(updatePlan);
      }
      hydrateUpdateSourceConfig(updateSource);
      if (updatePreview) {
        renderUpdatePreview(updatePreview);
      }
      renderLatestDownloadedUpdate(downloadedUpdate);
      renderLatestAppliedUpdate(appliedUpdate);
      renderLatestStagedUpdate(stagedUpdate);
      if (envForm) {
        hydrateEnv(envForm);
      } else {
        applyPreset("local", DEFAULT_PRESET);
        setModeCard("local");
      }
      if (releasePlatformSelect) {
        populateDesktopPlatformSelect(releasePlatformSelect, {
          fallback: normalizeDesktopPlatform(doctor?.platform),
        });
        releasePlatformSelect.value = normalizeDesktopPlatform(doctor?.platform);
      }
      syncReleaseTarget(releasePlatformSelect?.value);
      renderServiceStatus(status.rendered);
      await refreshRuntimeLog().catch(() => {
        renderRuntimeLog("runtime log unavailable");
      });
      if (liveTailToggle.checked) {
        await startRuntimeLogStream().catch(() => {});
      }
      const readyMessage =
        integrityReport && Array.isArray(integrityReport.issues) && integrityReport.issues.length > 0
          ? `${brandConfig?.installerName || "Installer GUI"} ready. Integrity panel has flagged install contract drift; clear that before packaging a release.`
          : `${brandConfig?.installerName || "Installer GUI"} ready. Pick a profile, write env, then start services and watch live logs here.`;
      showCompletion(readyMessage);

      return "installer gui ready";
    },
    { skipOutput: false },
  );
})();
