import {
  invokeTauri as invoke,
  listenTauri as listen,
  loadDesktopBrand,
  loadDesktopLanguagePreference,
  normalizeDesktopLanguage,
  saveDesktopLanguagePreference,
  setText,
  syncDesktopStates,
  watchDesktopLanguagePreference,
} from "./shared/tauri-bridge.js";
import {
  desktopReleaseRootPattern,
  normalizeDesktopPlatform,
  populateDesktopPlatformSelect,
  syncDesktopReleaseTargetInput,
} from "./shared/platform.js";
import {
  applyPreset,
  applyRemoteNodeToForm,
  currentRemoteAgentPayload,
  currentRemoteBootstrapPayload,
  currentRemoteNodeRegistryPayload,
  currentRemotePolicyPayload,
  hydrateRemoteNodeRegistry,
  hydrateRemotePolicy,
  renderDoctor,
  withRemoteNodeStatus,
} from "./installer-workflows.js";
import { mountCertificatePanel } from "./certificate-panel.js";
import { renderRegressionGateReport } from "./regression-gate-panel.js";
import { mountRemotePanel } from "./remote-panel.js";
import { mountRemoteNodePanel } from "./remote-node-panel.js";
import { createRuntimeLogController } from "./runtime-log-panel.js";
import {
  bindInstallerActionHandlers,
  bindInstallerSensitiveFields,
  bindInstallerSidebarTabs,
} from "./installer-event-bindings.js";
import { runInstallerStartup } from "./installer-startup.js";
import { mountIntegrityPanel, renderIntegrityReport } from "./integrity-panel.js";
import {
  currentUpdateSourcePayload,
  hydrateUpdateSourceConfig,
  mountUpdatePanel,
  renderLatestAppliedUpdate,
  renderLatestDownloadedUpdate,
  renderLatestStagedUpdate,
  renderUpdatePlan,
  renderUpdatePreview,
  selectedUpdateChannel,
} from "./update-panel.js";
import { formatRuntimeStatusReport, renderRuntimeStatusPlane } from "./shared/runtime-status-summary.js";

(function () {
  mountRemotePanel();
  const DEFAULT_PRESET = {
    agentManifestPath: "./deploy/agents.local.example.json",
    distributedAgentManifestPath: "./deploy/agents.distributed.example.json",
    sqliteDatabasePath: "./tmp/data/kyuubiki_dev.sqlite3",
  };
  const ids = (id) => document.getElementById(id);
  const ui = {
    output: ids("output"),
    serviceStatus: ids("service-status"),
    serviceStatusPlane: ids("service-status-plane"),
    runtimeLog: ids("runtime-log"),
    completionBanner: ids("completion-banner"),
    completionMessage: ids("completion-message"),
    doctorGrid: ids("doctor-grid"),
    platformLabel: ids("platform-label"),
    releasePlatformSelect: ids("release-platform"),
    releaseTargetInput: ids("release-target"),
    workspaceLabel: ids("workspace-label"),
    currentModeLabel: ids("current-mode-label"),
    languageLabel: ids("shell-language-label"), languageSelect: ids("shell-language-select"),
    serviceModePill: ids("service-mode-pill"), completionGuide: ids("completion-guide"),
    regressionGateTitle: ids("regression-gate-title"), regressionGateCopy: ids("regression-gate-copy"),
    regressionGateStatus: ids("regression-gate-status"), regressionGateWarningCount: ids("regression-gate-warning-count"),
    regressionGateFailingCount: ids("regression-gate-failing-count"), regressionGateCatalogPath: ids("regression-gate-catalog-path"),
    regressionGateSummary: ids("regression-gate-summary"), regressionGateReasons: ids("regression-gate-reasons"),
    liveTailToggle: ids("log-autorefresh"), logServiceSelect: ids("log-service"),
  };
  let brandConfig = null;
  let currentLanguage = "en";
  const sensitiveEnvFieldIds = ["database-url", "api-token", "cluster-api-token", "direct-mesh-token"];

  mountIntegrityPanel(); mountUpdatePanel(); populateDesktopPlatformSelect(ui.releasePlatformSelect);
  const { currentCertificateIssuePayload, currentCertificatePolicyPayload, currentCertificateRevokePayload, getActiveCertificates, hydrateCertificateAuthority } = mountCertificatePanel();

  const releaseLabel = () => {
    const version = String(brandConfig?.releaseVersion || "").replace(/^v/u, "");
    const codename = String(brandConfig?.releaseCodename || "").trim();
    return [codename, version].filter(Boolean).join(" ") ? `Kyuubiki Installer · ${[codename, version].filter(Boolean).join(" ")}` : "Kyuubiki Installer";
  };

  const renderDesktopLanguagePreference = () => {
    document.documentElement.lang = currentLanguage;
    if (ui.languageLabel) {
      ui.languageLabel.textContent =
        currentLanguage === "zh" ? "语言" : currentLanguage === "ja" ? "言語" : currentLanguage === "es" ? "Idioma" : "Language";
    }
    if (ui.languageSelect) ui.languageSelect.value = currentLanguage;
  };

  const applyBrandConfig = (brand) => {
    brandConfig = brand;
    if (brand?.installerName) {
      const version = String(brand.releaseVersion || "").replace(/^v/u, "");
      const codename = String(brand.releaseCodename || "").trim();
      const tag = [codename, version].filter(Boolean).join(" ");
      document.title = tag ? `${brand.installerName} · ${tag}` : brand.installerName;
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
      const tag = [
        String(brand.releaseCodename || "").trim(),
        String(brand.releaseVersion || "").replace(/^v/u, ""),
      ].filter(Boolean).join(" ");
      setText("brand-installer-version", tag);
    }
  };

  const setOutput = (value) => {
    ui.output.textContent = value;
  };

  const appendOutput = (title, value) => {
    const chunks = [ui.output.textContent.trim(), `## ${title}\n${value}`].filter(Boolean);
    ui.output.textContent = chunks.join("\n\n");
  };

  const showCompletion = (message) => {
    ui.completionMessage.textContent = message;
    ui.completionBanner.hidden = false;
    if (ui.completionGuide) ui.completionGuide.textContent = message;
  };

  const invokeGuardedMutation = (action, payload = {}) =>
    invoke("guarded_mutation_action", { payload: { action, ...payload } });

  const setSensitiveFieldState = (input, configured, placeholders) => {
    if (!input) return;
    input.value = "";
    input.dataset.configured = configured ? "true" : "false";
    input.placeholder = configured ? placeholders.configured : placeholders.empty;
  };

  const currentMode = () => ids("deployment-mode").value || "local";

  const setModeCard = (mode) => {
    document.querySelectorAll("[data-mode-card]").forEach((card) => {
      card.classList.toggle("mode-card--active", card.dataset.modeCard === mode);
    });
    applyDesktopState(ui.currentModeLabel, mode, { kind: "activity" });
    applyDesktopState(ui.serviceModePill, `${mode} profile`, { kind: "activity" });
  };

  const currentEnvPayload = () => {
    const databaseUrlInput = ids("database-url");
    const apiTokenInput = ids("api-token");
    const clusterApiTokenInput = ids("cluster-api-token");
    const directMeshTokenInput = ids("direct-mesh-token");
    return {
      deploymentMode: ids("deployment-mode").value,
      agentDiscovery: ids("agent-discovery").value,
      agentManifestPath: ids("agent-manifest-path").value.trim(),
      storageBackend: ids("storage-mode").value,
      sqliteDatabasePath: ids("sqlite-path").value.trim(),
      databaseUrl: databaseUrlInput.value.trim(),
      databaseUrlConfigured: databaseUrlInput.dataset.configured === "true",
      agentEndpoints: ids("agent-endpoints").value.trim(),
      kyuubikiApiToken: apiTokenInput.value.trim(),
      kyuubikiApiTokenConfigured: apiTokenInput.dataset.configured === "true",
      kyuubikiClusterApiToken: clusterApiTokenInput.value.trim(),
      kyuubikiClusterApiTokenConfigured: clusterApiTokenInput.dataset.configured === "true",
      kyuubikiClusterAllowedAgentIds: ids("cluster-allowed-agent-ids").value.trim(),
      kyuubikiClusterAllowedClusterIds: ids("cluster-allowed-cluster-ids").value.trim(),
      kyuubikiClusterRequireFingerprint: ids("cluster-require-fingerprint").value === "true",
      kyuubikiClusterTimestampWindowMs: ids("cluster-timestamp-window").value.trim() || "30000",
      kyuubikiProtectReads: ids("protect-reads").value === "true",
      kyuubikiDirectMeshEnabled: ids("direct-mesh-enabled").value === "true",
      kyuubikiDirectMeshToken: directMeshTokenInput.value.trim(),
      kyuubikiDirectMeshTokenConfigured: directMeshTokenInput.dataset.configured === "true",
    };
  };

  const hydrateEnv = (form) => {
    if (!form) return;
    ids("deployment-mode").value = form.deployment_mode || "local";
    ids("agent-discovery").value = form.agent_discovery || "static";
    ids("agent-manifest-path").value = form.agent_manifest_path || DEFAULT_PRESET.agentManifestPath;
    ids("storage-mode").value = form.storage_backend || "sqlite";
    ids("sqlite-path").value = form.sqlite_database_path || DEFAULT_PRESET.sqliteDatabasePath;
    setSensitiveFieldState(ids("database-url"), form.database_url_configured === true, {
      configured: "configured; leave blank to keep current value",
      empty: "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev",
    });
    ids("agent-endpoints").value = form.agent_endpoints || "127.0.0.1:5001,127.0.0.1:5002";
    setSensitiveFieldState(ids("api-token"), form.kyuubiki_api_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional shared token",
    });
    setSensitiveFieldState(ids("cluster-api-token"), form.kyuubiki_cluster_api_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional cluster-only token",
    });
    ids("cluster-allowed-agent-ids").value = form.kyuubiki_cluster_allowed_agent_ids || "";
    ids("cluster-allowed-cluster-ids").value = form.kyuubiki_cluster_allowed_cluster_ids || "";
    ids("cluster-require-fingerprint").value = form.kyuubiki_cluster_require_fingerprint ? "true" : "false";
    ids("cluster-timestamp-window").value = form.kyuubiki_cluster_timestamp_window_ms || "30000";
    ids("protect-reads").value = form.kyuubiki_protect_reads ? "true" : "false";
    ids("direct-mesh-enabled").value = form.kyuubiki_direct_mesh_enabled === false ? "false" : "true";
    setSensitiveFieldState(ids("direct-mesh-token"), form.kyuubiki_direct_mesh_token_configured === true, {
      configured: "configured; leave blank to keep current token",
      empty: "optional direct-mesh token",
    });
    setModeCard(form.deployment_mode || "local");
  };

  const renderServiceStatus = (report) => {
    renderRuntimeStatusPlane(ui.serviceStatusPlane, report?.summary);
    ui.serviceStatus.textContent = formatRuntimeStatusReport({
      title: releaseLabel(),
      rendered: report?.rendered,
      summary: report?.summary,
    });
  };

  const renderRuntimeLog = (rendered) => {
    ui.runtimeLog.textContent = rendered;
    ui.runtimeLog.scrollTop = ui.runtimeLog.scrollHeight;
  };

  const syncReleaseTarget = (platform = ui.releasePlatformSelect?.value) => {
    syncDesktopReleaseTargetInput(ui.releaseTargetInput, normalizeDesktopPlatform(platform));
    if (ui.releaseTargetInput && !ui.releaseTargetInput.dataset.desktopPlaceholderBound) {
      ui.releaseTargetInput.placeholder = desktopReleaseRootPattern();
      ui.releaseTargetInput.dataset.desktopPlaceholderBound = "true";
    }
  };

  const runAction = async (name, callback, options = {}) => {
    try {
      const result = await callback();
      if (typeof result === "string" && !options.skipOutput) appendOutput(name, result);
      return result;
    } catch (error) {
      appendOutput(name, error.message || String(error));
      throw error;
    }
  };

  const refreshDoctor = async () => {
    const report = await invoke("doctor_report");
    renderDoctor(report, ui.platformLabel, ui.workspaceLabel, ui.doctorGrid);
    return report.rendered;
  };
  const refreshIntegrityReport = async () => {
    const report = await invoke("installation_integrity_report");
    renderIntegrityReport(report, brandConfig);
    return report.rendered;
  };
  const refreshUpdatePlan = async () => {
    const report = await invoke("unified_update_plan", { channel: selectedUpdateChannel() });
    renderUpdatePlan(report);
    return report.rendered;
  };
  const refreshUpdateSourceConfig = async () => {
    const config = await invoke("update_source_config");
    hydrateUpdateSourceConfig(config);
    return config.rendered;
  };
  const refreshUpdatePreview = async () => {
    const report = await invoke("unified_update_preview", { channel: selectedUpdateChannel() });
    renderUpdatePreview(report);
    return report.rendered;
  };
  const refreshLatestStagedUpdate = async () => {
    const record = await invoke("latest_staged_update_record").catch(() => null);
    renderLatestStagedUpdate(record);
    return record?.rendered || "no staged update record";
  };
  const refreshLatestDownloadedUpdate = async () => {
    const record = await invoke("latest_downloaded_update_record").catch(() => null);
    renderLatestDownloadedUpdate(record);
    return record?.rendered || "no downloaded update record";
  };
  const refreshLatestAppliedUpdate = async () => {
    const record = await invoke("latest_applied_update_record").catch(() => null);
    renderLatestAppliedUpdate(record);
    return record?.rendered || "no applied update record";
  };
  const refreshUpdateState = async () => {
    const values = await Promise.all([
      refreshUpdatePlan(),
      refreshUpdatePreview(),
      refreshLatestStagedUpdate(),
      refreshLatestDownloadedUpdate(),
      refreshLatestAppliedUpdate(),
    ]);
    return values.filter(Boolean).join("\n\n");
  };
  const refreshServiceStatus = async () => {
    const report = await invoke("service_status");
    renderServiceStatus(report);
    return report.rendered;
  };
  const refreshRemotePolicy = async () => {
    const policy = await invoke("remote_deploy_policy");
    hydrateRemotePolicy(policy);
    return policy.rendered;
  };
  const refreshCertificateAuthority = async () => {
    const policy = await invoke("certificate_authority_policy");
    hydrateCertificateAuthority(policy);
    return policy.rendered;
  };

  const { refreshRuntimeLog, startRuntimeLogStream } = createRuntimeLogController({
    invoke,
    listen,
    logServiceSelect: ui.logServiceSelect,
    liveTailToggle: ui.liveTailToggle,
    renderRuntimeLog,
    showCompletion,
  });
  const { renderRemoteNodeCards } = mountRemoteNodePanel({
    invoke,
    runAction,
    invokeGuardedMutation,
    applyRemoteNodeToForm,
    getActiveCertificates,
    showCompletion,
    currentRemoteBootstrapPayload,
    currentRemoteAgentPayload,
    currentRemoteNodeRegistryPayload,
    hydrateRemoteNodeRegistry,
    withRemoteNodeStatus,
  });
  const applyRemoteRegistry = (registry) => {
    hydrateRemoteNodeRegistry(registry);
    renderRemoteNodeCards(registry?.nodes || []);
  };
  const refreshRemoteNodes = async () => {
    const registry = await invoke("remote_node_registry");
    applyRemoteRegistry(registry);
    return registry.rendered;
  };

  ui.languageSelect?.addEventListener("change", async (event) => {
    currentLanguage = await saveDesktopLanguagePreference(normalizeDesktopLanguage(event.target.value));
    renderDesktopLanguagePreference();
  });
  watchDesktopLanguagePreference({ getCurrentLanguage: () => currentLanguage, onChange: (language) => { currentLanguage = language; renderDesktopLanguagePreference(); } });
  bindInstallerSidebarTabs();
  ids("storage-mode").addEventListener("change", (event) => setModeCard(event.target.value));
  ui.releasePlatformSelect?.addEventListener("change", (event) => syncReleaseTarget(event.target.value));
  bindInstallerSensitiveFields(ids, sensitiveEnvFieldIds);

  const actionHandlers = {
    doctor: () => runAction("doctor", refreshDoctor),
    "prepare-layout": () => runAction("prepare-layout", () => invokeGuardedMutation("prepare_layout")),
    bootstrap: () => runAction("bootstrap", async () => {
      const result = await invokeGuardedMutation("bootstrap");
      await refreshDoctor();
      showCompletion("Bootstrap complete. You can validate env or start services next.");
      return result;
    }),
    "init-env": () => runAction("init-env", async () => {
      const result = await invokeGuardedMutation("init_env", { force: false });
      hydrateEnv(await invoke("read_env_file"));
      return result;
    }),
    "validate-env": () => runAction("validate-env", () => invokeGuardedMutation("validate_env")),
    "refresh-integrity": () => runAction("refresh-integrity", refreshIntegrityReport),
    "repair-installation": () => runAction("repair-installation", async () => {
      const result = await invokeGuardedMutation("repair_installation");
      await refreshIntegrityReport();
      showCompletion("Installation contract repaired and residue cleanup completed.");
      return result;
    }),
    "refresh-update-plan": () => runAction("refresh-update-plan", refreshUpdatePlan),
    "refresh-update-source": () => runAction("refresh-update-source", refreshUpdateSourceConfig),
    "refresh-update-preview": () => runAction("refresh-update-preview", refreshUpdatePreview),
    "save-update-source": () => runAction("save-update-source", async () => {
      const result = await invokeGuardedMutation("write_update_source_config", currentUpdateSourcePayload());
      await refreshUpdateSourceConfig();
      showCompletion("Update source saved. Refresh the channel plan to validate the selected catalog.");
      return result;
    }),
    "download-update": () => runAction("download-update", async () => {
      const result = await invokeGuardedMutation("download_update", {
        channel: selectedUpdateChannel(),
        platform: ui.releasePlatformSelect?.value || "macos",
      });
      await refreshLatestDownloadedUpdate();
      showCompletion("Selected channel artifacts downloaded into the configured update cache.");
      return result;
    }),
    "refresh-downloaded-update": () => runAction("refresh-downloaded-update", refreshLatestDownloadedUpdate),
    "apply-downloaded-update": () => runAction("apply-downloaded-update", async () => {
      const result = await invokeGuardedMutation("apply_downloaded_update");
      await refreshLatestAppliedUpdate();
      showCompletion("Downloaded update promoted into the applied-update handoff record.");
      return result;
    }),
    "refresh-applied-update": () => runAction("refresh-applied-update", refreshLatestAppliedUpdate),
    "refresh-staged-update": () => runAction("refresh-staged-update", refreshLatestStagedUpdate),
    "prepare-update": () => runAction("prepare-update", async () => {
      const result = await invokeGuardedMutation("prepare_staged_update", {
        channel: selectedUpdateChannel(),
        platform: ui.releasePlatformSelect?.value || "macos",
        targetDir: ui.releaseTargetInput?.value.trim() || null,
      });
      await Promise.all([refreshIntegrityReport(), refreshUpdateState()]);
      showCompletion("Staged update prepared. Review the refreshed integrity and channel state before distributing artifacts.");
      return result;
    }),
    "reprepare-update": () => runAction("prepare-update", async () => {
      const result = await invokeGuardedMutation("prepare_staged_update", {
        channel: selectedUpdateChannel(),
        platform: ui.releasePlatformSelect?.value || "macos",
        targetDir: ui.releaseTargetInput?.value.trim() || null,
      });
      await Promise.all([refreshIntegrityReport(), refreshUpdateState()]);
      showCompletion("Staged update prepared. Review the refreshed integrity and channel state before distributing artifacts.");
      return result;
    }),
    "write-env": () => runAction("write-env", async () => {
      const result = await invokeGuardedMutation("write_env_file", { envPayload: currentEnvPayload() });
      hydrateEnv(await invoke("read_env_file"));
      showCompletion("Environment saved. Next step: validate and start the active profile.");
      return result;
    }),
    "reload-env": () => runAction("reload-env", async () => {
      hydrateEnv(await invoke("read_env_file"));
      return "reloaded current environment";
    }),
    "use-local-mode": () => { applyPreset("local", DEFAULT_PRESET); setModeCard("local"); showCompletion("Local SQLite profile selected."); appendOutput("mode", "selected local SQLite profile"); },
    "use-cloud-mode": () => { applyPreset("cloud", DEFAULT_PRESET); setModeCard("cloud"); showCompletion("Cloud PostgreSQL profile selected."); appendOutput("mode", "selected cloud PostgreSQL profile"); },
    "use-distributed-mode": () => { applyPreset("distributed", DEFAULT_PRESET); setModeCard("distributed"); showCompletion("Distributed control-plane profile selected."); appendOutput("mode", "selected distributed control-plane profile"); },
    "service-status": () => runAction("service-status", refreshServiceStatus),
    "service-start-local": () => runAction("service-start-local", async () => { const result = await invokeGuardedMutation("service_start", { mode: "local" }); await refreshServiceStatus(); showCompletion("Local services started."); return result; }),
    "service-restart-local": () => runAction("service-restart-local", async () => { const result = await invokeGuardedMutation("service_restart", { mode: "local" }); await refreshServiceStatus(); showCompletion("Local services restarted."); return result; }),
    "service-start-cloud": () => runAction("service-start-cloud", async () => { const result = await invokeGuardedMutation("service_start", { mode: "cloud" }); await refreshServiceStatus(); showCompletion("Cloud services started."); return result; }),
    "service-start-distributed": () => runAction("service-start-distributed", async () => { const result = await invokeGuardedMutation("service_start", { mode: "distributed" }); await refreshServiceStatus(); showCompletion("Distributed control plane started."); return result; }),
    "service-restart-cloud": () => runAction("service-restart-cloud", async () => { const result = await invokeGuardedMutation("service_restart", { mode: "cloud" }); await refreshServiceStatus(); showCompletion("Cloud services restarted."); return result; }),
    "service-stop": () => runAction("service-stop", async () => { const result = await invokeGuardedMutation("service_stop"); await refreshServiceStatus(); showCompletion("All services stopped."); return result; }),
    "load-log": () => runAction("load-log", () => ui.liveTailToggle.checked ? startRuntimeLogStream().then(() => `attached live tail to ${ui.logServiceSelect.value}`) : refreshRuntimeLog()),
    "wizard-start-active": () => runAction("wizard-start-active", async () => {
      const mode = currentMode() === "distributed" ? "distributed" : currentMode() === "cloud" ? "cloud" : "local";
      const result = await invokeGuardedMutation("service_start", { mode });
      await refreshServiceStatus();
      showCompletion(`Started ${mode} profile.`);
      return result;
    }),
    "remote-bootstrap": () => runAction("remote-bootstrap", async () => {
      const result = await invokeGuardedMutation("remote_bootstrap", { remoteBootstrap: currentRemoteBootstrapPayload() });
      showCompletion("Remote workspace bootstrapped.");
      return result;
    }),
    "remote-start-agent": () => runAction("remote-start-agent", async () => {
      const result = await invokeGuardedMutation("remote_start_agent", { remoteAgent: currentRemoteAgentPayload() });
      showCompletion("Remote solver agent started.");
      return result;
    }),
    "refresh-remote-policy": () => runAction("refresh-remote-policy", refreshRemotePolicy),
    "save-remote-policy": () => runAction("save-remote-policy", async () => {
      const result = await invokeGuardedMutation("write_remote_policy", { remotePolicy: currentRemotePolicyPayload() });
      await refreshRemotePolicy();
      showCompletion("Remote deployment policy saved.");
      return result;
    }),
    "refresh-certificate-policy": () => runAction("refresh-certificate-policy", refreshCertificateAuthority),
    "save-certificate-policy": () => runAction("save-certificate-policy", async () => {
      const result = await invokeGuardedMutation("write_certificate_policy", {
        certificatePolicy: currentCertificatePolicyPayload(),
      });
      await refreshCertificateAuthority();
      showCompletion("Certificate policy saved. PKI storage and enforcement toggles are now aligned.");
      return result;
    }),
    "initialize-certificate-authority": () => runAction("initialize-certificate-authority", async () => {
      const result = await invokeGuardedMutation("initialize_certificate_authority");
      await refreshCertificateAuthority();
      showCompletion("Certificate authority initialized. You can now issue node certificates.");
      return result;
    }),
    "issue-node-certificate": () => runAction("issue-node-certificate", async () => {
      const result = await invokeGuardedMutation("issue_node_certificate", {
        certificateIssue: currentCertificateIssuePayload(),
      });
      await refreshCertificateAuthority();
      showCompletion("Node certificate issued and added to the installer inventory.");
      return result;
    }),
    "revoke-node-certificate": () => runAction("revoke-node-certificate", async () => {
      const result = await invokeGuardedMutation("revoke_node_certificate", {
        certificateRevoke: currentCertificateRevokePayload(),
      });
      await refreshCertificateAuthority();
      showCompletion("Selected certificate marked as revoked in the installer inventory.");
      return result;
    }),
    "refresh-remote-nodes": () => runAction("refresh-remote-nodes", refreshRemoteNodes),
    "save-remote-nodes": () => runAction("save-remote-nodes", async () => {
      const result = await invokeGuardedMutation("write_remote_nodes", { remoteNodes: currentRemoteNodeRegistryPayload() });
      await refreshRemoteNodes();
      showCompletion("Remote node registry saved.");
      return result;
    }),
    "use-first-remote-node": () => runAction("use-first-remote-node", async () => {
      const registry = await invoke("remote_node_registry");
      if (!Array.isArray(registry.nodes) || registry.nodes.length === 0) throw new Error("no remote nodes configured");
      applyRemoteNodeToForm(registry.nodes[0]);
      showCompletion(`Loaded remote node ${registry.nodes[0].label || registry.nodes[0].target_host}.`);
      return `loaded ${registry.nodes[0].label || registry.nodes[0].target_host}`;
    }),
    "probe-remote-node": () => runAction("probe-remote-node", async () => {
      const result = await invokeGuardedMutation("probe_remote_node", { remoteBootstrap: currentRemoteBootstrapPayload() });
      showCompletion("Remote node probe completed.");
      return result;
    }),
    "stage-release": async () => {
      await runAction("stage-release", () => invokeGuardedMutation("stage_release", {
        platform: ui.releasePlatformSelect.value,
        targetDir: ui.releaseTargetInput.value.trim() || null,
      }));
      showCompletion("Release scaffold staged. You can export launch config or build the installer next.");
    },
    "build-installer": async () => {
      await runAction("build-installer", () => invokeGuardedMutation("build_installer_bundle", {
        bundleMode: ids("release-build-mode")?.value || ids("build-mode")?.value || "debug-check",
      }));
      showCompletion("Installer build completed.");
    },
    "export-launch": () => runAction("export-launch", () => invoke("export_launch", { platform: ui.releasePlatformSelect.value })),
    "clear-output": () => setOutput(""),
  };

  bindInstallerActionHandlers(actionHandlers);

  runInstallerStartup({
    invoke,
    runAction,
    loadDesktopLanguagePreference,
    loadDesktopBrand,
    renderDesktopLanguagePreference,
    setCurrentLanguage: (language) => { currentLanguage = language || currentLanguage; },
    applyBrandConfig,
    syncDesktopStates,
    renderDoctor,
    platformLabel: ui.platformLabel,
    workspaceLabel: ui.workspaceLabel,
    doctorGrid: ui.doctorGrid,
    renderIntegrityReport,
    renderUpdatePlan,
    hydrateUpdateSourceConfig,
    renderUpdatePreview,
    renderLatestDownloadedUpdate,
    renderLatestAppliedUpdate,
    renderLatestStagedUpdate,
    hydrateEnv,
    applyPreset,
    defaultPreset: DEFAULT_PRESET,
    setModeCard,
    hydrateRemotePolicy,
    hydrateCertificateAuthority,
    hydrateRemoteNodeRegistry: applyRemoteRegistry,
    releasePlatformSelect: ui.releasePlatformSelect,
    populateDesktopPlatformSelect,
    normalizeDesktopPlatform,
    syncReleaseTarget,
    renderServiceStatus,
    refreshRuntimeLog,
    renderRuntimeLog,
    liveTailToggle: ui.liveTailToggle,
    startRuntimeLogStream,
    renderRegressionGateReport: (report) => renderRegressionGateReport(ui, report),
    showCompletion,
    brandConfigName: () => brandConfig?.installerName || "Installer GUI",
  });
})();
