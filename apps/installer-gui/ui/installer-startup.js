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
  hydrateRemoteNodeRegistry,
  releasePlatformSelect,
  populateDesktopPlatformSelect,
  normalizeDesktopPlatform,
  syncReleaseTarget,
  renderServiceStatus,
  refreshRuntimeLog,
  renderRuntimeLog,
  liveTailToggle,
  startRuntimeLogStream,
  showCompletion,
  brandConfigName,
}) {
  return runAction(
    "startup",
    async () => {
      const [
        doctor,
        integrityReport,
        updatePlan,
        updateSource,
        updatePreview,
        downloadedUpdate,
        appliedUpdate,
        stagedUpdate,
        envForm,
        status,
        language,
        brand,
        remotePolicy,
        remoteNodes,
      ] = await Promise.all([
        invoke("doctor_report"),
        invoke("installation_integrity_report").catch(() => null),
        invoke("unified_update_plan", { channel: "stable" }).catch(() => null),
        invoke("update_source_config").catch(() => null),
        invoke("unified_update_preview", { channel: "stable" }).catch(() => null),
        invoke("latest_downloaded_update_record").catch(() => null),
        invoke("latest_applied_update_record").catch(() => null),
        invoke("latest_staged_update_record").catch(() => null),
        invoke("read_env_file").catch(() => null),
        invoke("service_status").catch(() => ({ rendered: "service status unavailable" })),
        loadDesktopLanguagePreference().catch(() => "en"),
        loadDesktopBrand().catch(() => null),
        invoke("remote_deploy_policy").catch(() => null),
        invoke("remote_node_registry").catch(() => null),
      ]);

      setCurrentLanguage(language);
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
        applyPreset("local", defaultPreset);
        setModeCard("local");
      }
      if (remotePolicy) {
        hydrateRemotePolicy(remotePolicy);
      }
      if (remoteNodes) {
        hydrateRemoteNodeRegistry(remoteNodes);
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
          ? `${brandConfigName()} ready. Integrity panel has flagged install contract drift; clear that before packaging a release.`
          : `${brandConfigName()} ready. Pick a profile, write env, then start services and watch live logs here.`;
      showCompletion(readyMessage);
      return "installer gui ready";
    },
    { skipOutput: false },
  );
}
