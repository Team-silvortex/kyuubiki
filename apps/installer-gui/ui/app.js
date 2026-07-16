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
  DEFAULT_PRESET,
  SENSITIVE_ENV_FIELD_IDS,
  createInstallerEnvState,
} from "./installer-env-state.js";
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

const installerShellCopy = {
  en: {
    language: "Language",
    platform: "Platform",
    workspace: "Workspace",
    currentMode: "Current mode",
    roleChip: "Bootstrap shell",
    description: "Visual installer for local SQLite workflows, cloud PostgreSQL deployment, and release staging.",
    actions: {
      doctor: "Run doctor",
      bootstrap: "Bootstrap workspace",
      serviceStatus: "Check services",
      writeEnv: "Write env",
      validateEnv: "Validate env",
      stageRelease: "Stage release",
      buildInstaller: "Build installer app",
      clearOutput: "Clear",
    },
    tabs: ["Wizard", "Setup", "Integrity", "Updates", "Services", "Remote", "Release", "Output"],
    headings: {
      wizard: "Desktop install wizard",
      setup: "Choose your deployment mode",
      environment: "Environment",
      doctor: "Doctor checks",
      services: "Run the whole stack",
      status: "Current service status",
      logs: "Runtime logs",
      release: "Stage a release directory",
      output: "Installer output",
    },
    completion: "Setup flow updated.",
    ready: "Ready for the next step.",
  },
  zh: {
    language: "语言",
    platform: "平台",
    workspace: "工作区",
    currentMode: "当前模式",
    roleChip: "引导外壳",
    description: "用于本地 SQLite、云端 PostgreSQL 部署和发布暂存的可视化安装器。",
    actions: {
      doctor: "运行诊断",
      bootstrap: "引导工作区",
      serviceStatus: "检查服务",
      writeEnv: "写入环境",
      validateEnv: "验证环境",
      stageRelease: "暂存发布",
      buildInstaller: "构建安装器",
      clearOutput: "清空",
    },
    tabs: ["向导", "设置", "完整性", "更新", "服务", "远程", "发布", "输出"],
    headings: {
      wizard: "桌面安装向导",
      setup: "选择部署模式",
      environment: "环境",
      doctor: "诊断检查",
      services: "运行整套服务",
      status: "当前服务状态",
      logs: "运行日志",
      release: "暂存发布目录",
      output: "安装器输出",
    },
    completion: "安装流程已更新。",
    ready: "可以进行下一步。",
  },
  ja: {
    language: "言語",
    platform: "プラットフォーム",
    workspace: "ワークスペース",
    currentMode: "現在のモード",
    roleChip: "ブートストラップシェル",
    description: "ローカル SQLite、クラウド PostgreSQL、リリースステージング用のビジュアルインストーラ。",
    actions: {
      doctor: "診断を実行",
      bootstrap: "ワークスペース初期化",
      serviceStatus: "サービス確認",
      writeEnv: "環境を書き込む",
      validateEnv: "環境を検証",
      stageRelease: "リリースを準備",
      buildInstaller: "インストーラをビルド",
      clearOutput: "クリア",
    },
    tabs: ["ウィザード", "設定", "整合性", "更新", "サービス", "リモート", "リリース", "出力"],
    headings: {
      wizard: "デスクトップインストールウィザード",
      setup: "デプロイモードを選択",
      environment: "環境",
      doctor: "診断チェック",
      services: "スタックを実行",
      status: "現在のサービス状態",
      logs: "ランタイムログ",
      release: "リリースディレクトリを準備",
      output: "インストーラ出力",
    },
    completion: "セットアップ手順を更新しました。",
    ready: "次の手順に進めます。",
  },
  es: {
    language: "Idioma",
    platform: "Plataforma",
    workspace: "Espacio de trabajo",
    currentMode: "Modo actual",
    roleChip: "Shell de arranque",
    description: "Instalador visual para SQLite local, despliegue PostgreSQL en nube y staging de releases.",
    actions: {
      doctor: "Ejecutar diagnóstico",
      bootstrap: "Preparar workspace",
      serviceStatus: "Comprobar servicios",
      writeEnv: "Escribir entorno",
      validateEnv: "Validar entorno",
      stageRelease: "Preparar release",
      buildInstaller: "Compilar instalador",
      clearOutput: "Limpiar",
    },
    tabs: ["Asistente", "Config", "Integridad", "Updates", "Servicios", "Remoto", "Release", "Salida"],
    headings: {
      wizard: "Asistente de instalación",
      setup: "Elegir modo de despliegue",
      environment: "Entorno",
      doctor: "Chequeos doctor",
      services: "Ejecutar el stack",
      status: "Estado actual del servicio",
      logs: "Logs de runtime",
      release: "Preparar directorio de release",
      output: "Salida del instalador",
    },
    completion: "Flujo de setup actualizado.",
    ready: "Listo para el siguiente paso.",
  },
};

const installerLanguageOptions = [
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
  { value: "ja", label: "日本語" },
  { value: "es", label: "Español" },
  { value: "ar", label: "العربية · Arabic" },
  { value: "bn", label: "বাংলা · Bengali" },
  { value: "cs", label: "Čeština · Czech" },
  { value: "da", label: "Dansk · Danish" },
  { value: "de", label: "Deutsch · German" },
  { value: "el", label: "Ελληνικά · Greek" },
  { value: "fa", label: "فارسی · Persian" },
  { value: "fi", label: "Suomi · Finnish" },
  { value: "fr", label: "Français · French" },
  { value: "he", label: "עברית · Hebrew" },
  { value: "hi", label: "हिन्दी · Hindi" },
  { value: "id", label: "Bahasa Indonesia · Indonesian" },
  { value: "it", label: "Italiano · Italian" },
  { value: "ko", label: "한국어 · Korean" },
  { value: "ms", label: "Bahasa Melayu · Malay" },
  { value: "nl", label: "Nederlands · Dutch" },
  { value: "no", label: "Norsk · Norwegian" },
  { value: "pl", label: "Polski · Polish" },
  { value: "pt-BR", label: "Português (Brasil)" },
  { value: "ro", label: "Română · Romanian" },
  { value: "ru", label: "Русский · Russian" },
  { value: "sv", label: "Svenska · Swedish" },
  { value: "sw", label: "Kiswahili · Swahili" },
  { value: "ta", label: "தமிழ் · Tamil" },
  { value: "th", label: "ไทย · Thai" },
  { value: "tr", label: "Türkçe · Turkish" },
  { value: "uk", label: "Українська · Ukrainian" },
  { value: "ur", label: "اردو · Urdu" },
  { value: "vi", label: "Tiếng Việt · Vietnamese" },
  { value: "zh-TW", label: "繁體中文 · Traditional Chinese" },
];

function installerShellCopyFor(language) {
  const normalized = normalizeDesktopLanguage(language);
  if (installerShellCopy[normalized]) return installerShellCopy[normalized];
  if (normalized.toLowerCase().startsWith("zh")) return installerShellCopy.zh;
  return installerShellCopy.en;
}

function populateInstallerLanguageSelect(select, language) {
  if (!select) return;
  const selected = normalizeDesktopLanguage(language);
  select.replaceChildren(...installerLanguageOptions.map((option) => {
    const element = document.createElement("option");
    element.value = option.value;
    element.textContent = option.label;
    return element;
  }));
  if (!installerLanguageOptions.some((option) => option.value === selected)) {
    const custom = document.createElement("option");
    custom.value = selected;
    custom.textContent = selected;
    select.appendChild(custom);
  }
  select.value = selected;
}

(function () {
  mountRemotePanel();
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

  mountIntegrityPanel(); mountUpdatePanel(); populateDesktopPlatformSelect(ui.releasePlatformSelect);
  const { currentCertificateIssuePayload, currentCertificatePolicyPayload, currentCertificateRevokePayload, getActiveCertificates, hydrateCertificateAuthority } = mountCertificatePanel();
  const {
    currentEnvPayload,
    currentMode,
    hydrateEnv,
    setModeCard,
  } = createInstallerEnvState({ ids, ui, applyPreset });

  const releaseLabel = () => {
    const version = String(brandConfig?.releaseVersion || "").replace(/^v/u, "");
    const codename = String(brandConfig?.releaseCodename || "").trim();
    return [codename, version].filter(Boolean).join(" ") ? `Kyuubiki Installer · ${[codename, version].filter(Boolean).join(" ")}` : "Kyuubiki Installer";
  };

  const renderDesktopLanguagePreference = () => {
    const copy = installerShellCopyFor(currentLanguage);
    document.documentElement.lang = currentLanguage;
    setText(ui.languageLabel, copy.language);
    populateInstallerLanguageSelect(ui.languageSelect, currentLanguage);
    setText("brand-installer-role-chip", copy.roleChip);
    setText("brand-installer-description", copy.description);
    setText(document.querySelector(".hero-meta .meta-card:nth-child(2) > span"), copy.platform);
    setText(document.querySelector(".hero-meta .meta-card:nth-child(3) > span"), copy.workspace);
    setText(document.querySelector(".hero-meta .meta-card:nth-child(4) > span"), copy.currentMode);
    document.querySelectorAll(".sidebar-tab").forEach((tab, index) => {
      setText(tab.querySelector("span:last-child"), copy.tabs[index]);
    });
    setText(document.querySelector(".completion-banner strong"), copy.completion);
    setText("completion-message", copy.ready);
    setText(document.querySelector('[data-panel="wizard"] h2'), copy.headings.wizard);
    setText(document.querySelector('[data-panel="setup"] .section-header h2'), copy.headings.setup);
    setText(document.querySelector(".form-shell .panel-header h2"), copy.headings.environment);
    setText(document.querySelector(".doctor-shell .panel-header h2"), copy.headings.doctor);
    setText(document.querySelector('[data-panel="services"] .section-header h2'), copy.headings.services);
    setText(document.querySelector(".status-shell .panel-header h2"), copy.headings.status);
    setText(document.querySelector(".log-shell .panel-header h2"), copy.headings.logs);
    setText(document.querySelector('[data-panel="release"] .section-header h2'), copy.headings.release);
    setText(document.querySelector('[data-panel="output"] .section-header h2'), copy.headings.output);
    document.querySelectorAll('[data-action="doctor"]').forEach((node) => setText(node, copy.actions.doctor));
    document.querySelectorAll('[data-action="bootstrap"]').forEach((node) => setText(node, copy.actions.bootstrap));
    document.querySelectorAll('[data-action="service-status"]').forEach((node) => setText(node, copy.actions.serviceStatus));
    document.querySelectorAll('[data-action="write-env"]').forEach((node) => setText(node, copy.actions.writeEnv));
    document.querySelectorAll('[data-action="validate-env"]').forEach((node) => setText(node, copy.actions.validateEnv));
    document.querySelectorAll('[data-action="stage-release"]').forEach((node) => setText(node, copy.actions.stageRelease));
    document.querySelectorAll('[data-action="build-installer"]').forEach((node) => setText(node, copy.actions.buildInstaller));
    document.querySelectorAll('[data-action="clear-output"]').forEach((node) => setText(node, copy.actions.clearOutput));
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
    renderDesktopLanguagePreference();
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

  const highImpactGuardedActions = new Set([
    "remote_bootstrap",
    "remote_start_agent",
    "write_remote_policy",
    "write_remote_nodes",
    "write_certificate_policy",
    "initialize_certificate_authority",
    "issue_node_certificate",
    "revoke_node_certificate",
    "stage_release",
    "prepare_staged_update",
    "write_update_source_config",
    "download_update",
    "apply_downloaded_update",
    "build_installer_bundle",
  ]);

  const invokeGuardedMutation = (action, payload = {}) =>
    invoke("guarded_mutation_action", {
      payload: {
        action,
        ...payload,
        ...(highImpactGuardedActions.has(action) ? { confirmationNonce: `confirmed:${action}` } : {}),
      },
    });

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
  bindInstallerSensitiveFields(ids, SENSITIVE_ENV_FIELD_IDS);

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
