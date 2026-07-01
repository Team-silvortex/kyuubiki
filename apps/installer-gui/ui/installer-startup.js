function afterFirstPaint(task) {
  const schedule = window.requestAnimationFrame
    ? (callback) => window.requestAnimationFrame(() => window.requestAnimationFrame(callback))
    : (callback) => window.setTimeout(callback, 0);

  return new Promise((resolve) => {
    schedule(() => resolve(task()));
  });
}

function duringIdle(task) {
  return new Promise((resolve) => {
    const run = () => resolve(task());
    if (window.requestIdleCallback) {
      window.requestIdleCallback(run, { timeout: 1200 });
      return;
    }
    window.setTimeout(run, 48);
  });
}

async function settleInstallerStartup(label, task) {
  try {
    await task();
  } catch (error) {
    console.warn(`Installer startup phase failed: ${label}`, error);
  }
}

export async function runInstallerStartup({
  invoke,
  runAction,
  loadDesktopLanguagePreference,
  loadDesktopBrand,
  renderDesktopLanguagePreference,
  setCurrentLanguage,
  applyBrandConfig,
  syncDesktopStates,
  renderDoctor,
  platformLabel,
  workspaceLabel,
  doctorGrid,
  renderIntegrityReport,
  renderUpdatePlan,
  hydrateUpdateSourceConfig,
  renderUpdatePreview,
  renderLatestDownloadedUpdate,
  renderLatestAppliedUpdate,
  renderLatestStagedUpdate,
  hydrateEnv,
  applyPreset,
  defaultPreset,
  setModeCard,
  hydrateRemotePolicy,
  hydrateCertificateAuthority,
  hydrateRemoteNodeRegistry,
  releasePlatformSelect,
  populateDesktopPlatformSelect,
  normalizeDesktopPlatform,
  syncReleaseTarget,
  renderServiceStatus,
  refreshRuntimeLog,
  renderRuntimeLog,
  renderRegressionGateReport,
  liveTailToggle,
  startRuntimeLogStream,
  showCompletion,
  brandConfigName,
}) {
  const startupResult = await runAction(
    "startup",
    async () => {
      const [
        doctor,
        envForm,
        status,
        language,
        brand,
      ] = await Promise.all([
        invoke("doctor_report"),
        invoke("read_env_file").catch(() => null),
        invoke("service_status").catch(() => ({ rendered: "service status unavailable" })),
        loadDesktopLanguagePreference().catch(() => "en"),
        loadDesktopBrand().catch(() => null),
      ]);

      setCurrentLanguage(language);
      renderDesktopLanguagePreference();
      if (brand) {
        applyBrandConfig(brand);
      }
      syncDesktopStates();
      renderDoctor(doctor, platformLabel, workspaceLabel, doctorGrid);
      if (envForm) {
        hydrateEnv(envForm);
      } else {
        applyPreset("local", defaultPreset);
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
      showCompletion(`${brandConfigName()} ready. Heavy reports continue loading in the background.`);
      return "installer gui ready";
    },
    { skipOutput: false },
  );

  void afterFirstPaint(async () => {
    let integrityReport = null;
    let regressionGate = null;

    await duringIdle(() =>
      settleInstallerStartup("integrity-report", async () => {
        integrityReport = await invoke("installation_integrity_report");
        renderIntegrityReport(integrityReport);
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("update-plan", async () => {
        renderUpdatePlan(await invoke("unified_update_plan", { channel: "stable" }));
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("update-source", async () => {
        hydrateUpdateSourceConfig(await invoke("update_source_config"));
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("update-preview", async () => {
        renderUpdatePreview(await invoke("unified_update_preview", { channel: "stable" }));
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("update-records", async () => {
        const [downloadedUpdate, appliedUpdate, stagedUpdate] = await Promise.all([
          invoke("latest_downloaded_update_record").catch(() => null),
          invoke("latest_applied_update_record").catch(() => null),
          invoke("latest_staged_update_record").catch(() => null),
        ]);
        renderLatestDownloadedUpdate(downloadedUpdate);
        renderLatestAppliedUpdate(appliedUpdate);
        renderLatestStagedUpdate(stagedUpdate);
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("remote-policy", async () => {
        hydrateRemotePolicy(await invoke("remote_deploy_policy"));
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("certificate-authority", async () => {
        hydrateCertificateAuthority(await invoke("certificate_authority_policy"));
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("remote-nodes", async () => {
        hydrateRemoteNodeRegistry(await invoke("remote_node_registry"));
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("regression-gate", async () => {
        regressionGate = await invoke("regression_gate_report");
        renderRegressionGateReport(regressionGate);
      }),
    );
    await duringIdle(() =>
      settleInstallerStartup("runtime-log", async () => {
        await refreshRuntimeLog().catch(() => {
          renderRuntimeLog("runtime log unavailable");
        });
        if (liveTailToggle.checked) {
          await startRuntimeLogStream().catch(() => {});
        }
      }),
    );

    const readyMessage =
      regressionGate && regressionGate.overall_gate_status && regressionGate.overall_gate_status !== "pass"
        ? `${brandConfigName()} ready. Unified regression gate is ${regressionGate.overall_gate_status}; review benchmark or workflow drift before packaging or rollout.`
        : integrityReport && Array.isArray(integrityReport.issues) && integrityReport.issues.length > 0
          ? `${brandConfigName()} ready. Integrity panel has flagged install contract drift; clear that before packaging a release.`
          : `${brandConfigName()} ready. Pick a profile, write env, then start services and watch live logs here.`;
    showCompletion(readyMessage);
  });

  return startupResult;
}
