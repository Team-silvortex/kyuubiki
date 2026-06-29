"use client";
import { useState } from "react";
import { buildWorkbenchLanguageOptions } from "@/components/workbench/workbench-language-options";
import { WorkbenchScriptPanel } from "@/components/workbench/workbench-script-panel";
import { WorkbenchSystemDataMount } from "@/components/workbench/workbench-system-data-mount";
import type { SecurityEventWindow } from "@/components/workbench/workbench-types";
import type { WorkbenchSystemSidebarMountProps } from "@/components/workbench/workbench-system-sidebar-mount-types";
import { WorkbenchSystemConfigCard } from "@/components/workbench/system/workbench-system-config-card";
import { buildWorkbenchSystemBackendTargetCopy } from "@/components/workbench/system/workbench-system-backend-target-copy";
import { buildWorkbenchSystemControlModeCopy, buildWorkbenchSystemControlTopologySummary, buildWorkbenchSystemTopologySnapshot } from "@/components/workbench/system/workbench-system-control-mode-contract";
import { WorkbenchSystemInstallLayoutCard } from "@/components/workbench/system/workbench-system-install-layout-card";
import { WorkbenchSystemInstallPolicyMount } from "@/components/workbench/system/workbench-system-install-policy-mount";
import { buildWorkbenchLanguagePackPresentation } from "@/components/workbench/system/workbench-system-language-pack-presentation";
import { WorkbenchSystemRuntimePanel } from "@/components/workbench/system/workbench-system-runtime-panel";
import { WorkbenchSystemSidebar } from "@/components/workbench/system/workbench-system-sidebar";
import { applyWorkbenchGovernancePatch, buildWorkbenchGovernanceConfig, buildWorkbenchGovernanceRows } from "@/lib/workbench/governance";
import {
  persistWorkbenchApiBaseUrl,
  readPersistedWorkbenchApiBaseUrl,
  resolveWorkbenchBackendTarget,
} from "@/lib/api/backend-target";
import type { ProtocolAgentDescriptor } from "@/lib/api";
import type { WorkbenchSecurityAuditRisk, WorkbenchSecurityAuditSource } from "@/lib/workbench/security-audit";

export function WorkbenchSystemSidebarMount({
  t,
  systemPanelTab,
  handleSystemPanelTabChange,
  setSidebarSection,
  handleWorkflowPanelTabChange,
  runtimeRecoveryCard,
  healthStatus,
  healthProtocolOnline,
  healthWatchdogOnline,
  healthSecurityApiTokenConfigured,
  runtimeBackendRows,
  runtimeProtocolRows,
  runtimeProtocolMethods,
  securityUi,
  runtimeSecurityRows,
  runtimeAuditSummaryRows,
  runtimeAuditTrendBars,
  runtimeAuditSourceStatusFacets,
  runtimeAuditStudyFacets,
  runtimeAuditProjectFacets,
  runtimeAuditModelVersionFacets,
  securityEventRecords,
  securityEventWindowFilter,
  securityEventSourceFilter,
  securityEventRiskFilter,
  securityEventStatusFilter,
  securityEventActionFilter,
  setSecurityEventWindowFilter,
  setSecurityEventSourceFilter,
  setSecurityEventRiskFilter,
  setSecurityEventStatusFilter,
  setSecurityEventActionFilter,
  refreshSecurityEvents,
  downloadSecurityEventExport,
  downloadSecurityEventCsvExport,
  runtimeAuditEntries,
  protocolAgents,
  protocolAgentCards,
  runtimeWatchdogRows,
  theme,
  language,
  frontendRuntimeMode,
  directMeshSelectionMode,
  directMeshEndpointsText,
  controlPlaneApiToken,
  clusterApiToken,
  directMeshApiToken,
  showShortcutHints,
  immersiveGuardrails,
  languagePacks,
  languagePackCatalogRows,
  setTheme,
  handleLanguageChange,
  handleDownloadLanguagePackTemplate,
  handleExportInstalledLanguagePack,
  handleImportLanguagePack,
  handleRemoveLanguagePack,
  setFrontendRuntimeMode,
  setDirectMeshSelectionMode,
  setDirectMeshEndpointsText,
  setControlPlaneApiToken,
  setClusterApiToken,
  setDirectMeshApiToken,
  setShowShortcutHints,
  setImmersiveGuardrails,
  downloadDatabaseSnapshot,
  scriptActionLog,
  getScriptSnapshot,
  scriptRecordingMode,
  invokeScriptAction,
  setScriptRecordingMode,
  scriptSnapshot,
  systemDataTab,
  handleSystemDataTabChange,
  adminJobRows,
  selectedAdminJobId,
  handleSelectAdminJob,
  selectedAdminJob,
  adminJobMessage,
  setAdminJobMessage,
  adminJobProjectId,
  setAdminJobProjectId,
  adminJobModelVersionId,
  setAdminJobModelVersionId,
  adminJobCaseId,
  setAdminJobCaseId,
  saveAdminJobRecord,
  deleteAdminJobRecord,
  adminResultRows,
  selectedAdminResultJobId,
  handleSelectAdminResult,
  jobHistory,
  adminResultDraft,
  setAdminResultDraft,
  saveAdminResultRecord,
  applySelectedAdminResultContext,
  openSelectedAdminResultProject,
  openSelectedAdminResultVersion,
  exportAdminResultRecord,
  deleteAdminResultRecord,
  adminFilterProjectId,
  handleAdminFilterProjectChange,
  adminFilterModelVersionId,
  handleAdminFilterModelVersionChange,
  selectedProjectId,
  selectedVersionId,
  useCurrentProjectAsAdminFilter,
  useCurrentVersionAsAdminFilter,
  clearAdminFilters,
  applySelectedAdminJobContext,
  openSelectedAdminJobProject,
  openSelectedAdminJobVersion,
  jobId,
  cancelCurrentJob,
  cancelJob,
  setMessage,
  refreshJobHistory,
}: WorkbenchSystemSidebarMountProps) {
  const mergedProtocolAgents = protocolAgents as ProtocolAgentDescriptor[];
  const activeLeaseCount = mergedProtocolAgents.filter((agent) => Boolean(agent.active_lease)).length, staleLeaseCount = mergedProtocolAgents.filter((agent) => agent.active_lease?.is_stale).length;
  const protocolAgentCountLabel = language === "zh" ? `${mergedProtocolAgents.length} 台 · ${activeLeaseCount} 租约 · ${staleLeaseCount} 过期` : language === "ja" ? `${mergedProtocolAgents.length} 台 ・ ${activeLeaseCount} リース ・ ${staleLeaseCount} 期限切れ` : `${mergedProtocolAgents.length} agents · ${activeLeaseCount} leases · ${staleLeaseCount} stale`;
  const protocolAgentSummaryRows = language === "zh" ? [{ label: "可达代理", value: mergedProtocolAgents.length }, { label: "活跃租约", value: activeLeaseCount }, { label: "过期租约", value: staleLeaseCount }] : language === "ja" ? [{ label: "到達可能エージェント", value: mergedProtocolAgents.length }, { label: "アクティブリース", value: activeLeaseCount }, { label: "期限切れリース", value: staleLeaseCount }] : [{ label: "Reachable agents", value: mergedProtocolAgents.length }, { label: "Active leases", value: activeLeaseCount }, { label: "Stale leases", value: staleLeaseCount }];
  const controlWindowCopy = buildWorkbenchSystemControlModeCopy(language, frontendRuntimeMode);
  const controlWindowTopology = buildWorkbenchSystemControlTopologySummary({
    frontendRuntimeMode,
    directMeshSelectionMode,
    directMeshEndpointsText,
    protocolAgents: mergedProtocolAgents,
    controlPlaneApiToken,
    clusterApiToken,
    directMeshApiToken,
    protocolOnline: healthProtocolOnline,
    securityConfigured: healthSecurityApiTokenConfigured,
    auditCount: runtimeAuditEntries.length,
    copy: controlWindowCopy,
  });
  const controlWindowSnapshot = buildWorkbenchSystemTopologySnapshot({
    frontendRuntimeMode,
    directMeshSelectionMode,
    directMeshEndpointsText,
    protocolAgents: mergedProtocolAgents,
    topology: controlWindowTopology,
  });
  const governanceConfig = buildWorkbenchGovernanceConfig({
    frontendRuntimeMode, directMeshEndpointsText, controlPlaneApiToken, clusterApiToken, directMeshApiToken,
  });
  const governanceRows = buildWorkbenchGovernanceRows(governanceConfig);
  const installedLanguagePackRows = buildWorkbenchLanguagePackPresentation(language, languagePacks);
  const [backendApiBaseUrl, setBackendApiBaseUrl] = useState(readPersistedWorkbenchApiBaseUrl);
  const backendTarget = resolveWorkbenchBackendTarget();
  const backendTargetCopy = buildWorkbenchSystemBackendTargetCopy(language);

  const handleBackendApiBaseUrlChange = (value: string) => {
    const nextValue = persistWorkbenchApiBaseUrl(value);
    setBackendApiBaseUrl(nextValue);
    setMessage(nextValue ? backendTargetCopy.saved : backendTargetCopy.cleared);
  };

  return (
    <WorkbenchSystemSidebar
      systemPanelTab={systemPanelTab}
      onSystemPanelTabChange={handleSystemPanelTabChange}
      settingsTabLabel={t.settings}
      overviewPageLabel={t.overview}
      configPageLabel={t.config}
      scriptsPageLabel={t.scripts}
      runtimeTabLabel={t.runtime}
      dataTabLabel={t.data}
      configOverviewHint={t.settingsConfigHint}
      scriptsOverviewHint={t.settingsScriptsHint}
      configContent={
        <div className="sidebar-stack">
          <WorkbenchSystemConfigCard
            title={t.settings}
            status={healthStatus === "ok" ? t.online : t.offline}
            workspacePageLabel={t.workspace}
            routingPageLabel={t.routing}
            accessPageLabel={t.access}
            governancePageLabel="governance"
            packsPageLabel={t.packs}
            themeLabel={t.theme}
            languageLabel={t.language}
            languagePacksTitle={t.languagePacksTitle}
            languagePacksHint={t.languagePacksHint}
            languagePacksEmptyLabel={t.languagePacksEmptyLabel}
            languagePackNameLabel={t.languagePackName}
            languagePackVersionLabel={t.languagePackVersion}
            languagePackSourceImportedLabel={t.languagePackSourceImported}
            languagePackSourceDownloadedLabel={t.languagePackSourceDownloaded}
            languagePackDownloadTemplateLabel={t.languagePackDownloadTemplate}
            languagePackExportInstalledLabel={t.languagePackExportInstalled}
            languagePackImportLabel={t.languagePackImport}
            languagePackRemoveLabel={t.languagePackRemove}
            languagePackCatalogTitle={t.languagePackCatalogTitle}
            languagePackCatalogHint={t.languagePackCatalogHint}
            languagePackCatalogActionLabel={t.languagePackCatalogAction}
            frontendModeLabel={t.frontendMode}
            backendTargetLabel={backendTargetCopy.label}
            backendTargetHelp={backendTargetCopy.help}
            backendTargetPlaceholder={backendTargetCopy.placeholder}
            backendTargetSourceLabel={backendTargetCopy.source}
            backendTargetEffectiveLabel={backendTargetCopy.effective}
            directMeshStrategyLabel={t.directMeshStrategy}
            directMeshEndpointsLabel={t.directMeshEndpoints}
            directMeshEndpointsHelp={t.directMeshEndpointsHelp}
            controlPlaneTokenLabel={securityUi.controlPlaneToken}
            controlPlaneTokenHelp={t.controlPlaneTokenHelp}
            controlPlaneTokenPlaceholder={t.controlPlaneTokenPlaceholder}
            clusterTokenLabel={securityUi.clusterToken}
            clusterTokenHelp={t.clusterTokenHelp}
            clusterTokenPlaceholder={t.clusterTokenPlaceholder}
            directMeshTokenLabel={securityUi.directMeshToken}
            directMeshTokenHelp={t.directMeshTokenHelp}
            directMeshTokenPlaceholder={t.directMeshTokenPlaceholder}
            shortcutHintsLabel={t.shortcutHints}
            shortcutHintsHelp={t.shortcutHintsHelp}
            immersiveGuardLabel={t.immersiveGuard}
            immersiveGuardHelp={t.immersiveGuardHelp}
            browserLimitsNote={t.browserLimitsNote}
            exportDatabaseLabel={t.exportDatabase}
            governanceTitle="System governance"
            governanceHint="A persisted, read-only architecture contract for hub, workbench, installer, and agent responsibilities."
            governanceRows={governanceRows}
            governanceJson={JSON.stringify(governanceConfig, null, 2)}
            theme={theme}
            language={language}
            frontendRuntimeMode={frontendRuntimeMode}
            backendApiBaseUrl={backendApiBaseUrl}
            backendTargetSource={backendTargetCopy.sources[backendTarget.source]}
            backendTargetEffectiveUrl={backendTarget.baseUrl}
            directMeshSelectionMode={directMeshSelectionMode}
            directMeshEndpointsText={directMeshEndpointsText}
            controlPlaneApiToken={controlPlaneApiToken}
            clusterApiToken={clusterApiToken}
            directMeshApiToken={directMeshApiToken}
            showShortcutHints={showShortcutHints}
            immersiveGuardrails={immersiveGuardrails}
            themeOptions={[
              { value: "linen", label: t.themes.linen },
              { value: "marine", label: t.themes.marine },
              { value: "graphite", label: t.themes.graphite },
            ]}
            languageOptions={buildWorkbenchLanguageOptions({ copy: t, languagePacks, currentLanguage: language })}
            installedLanguagePacks={installedLanguagePackRows}
            catalogLanguagePacks={languagePackCatalogRows}
            frontendModeOptions={[
              { value: "orchestrated_gui", label: t.frontendModes.orchestrated_gui },
              { value: "direct_mesh_gui", label: t.frontendModes.direct_mesh_gui },
            ]}
            directMeshStrategyOptions={[
              { value: "healthiest", label: t.directMeshStrategies.healthiest },
              { value: "first_reachable", label: t.directMeshStrategies.first_reachable },
            ]}
            onThemeChange={setTheme}
            onLanguageChange={handleLanguageChange}
            onDownloadLanguagePackTemplate={handleDownloadLanguagePackTemplate}
            onExportInstalledLanguagePack={handleExportInstalledLanguagePack}
            onImportLanguagePack={(file) => void handleImportLanguagePack(file)}
            onRemoveLanguagePack={handleRemoveLanguagePack}
            onFrontendRuntimeModeChange={(value) => setFrontendRuntimeMode(applyWorkbenchGovernancePatch({ currentFrontendRuntimeMode: frontendRuntimeMode, currentDirectMeshEndpointsText: directMeshEndpointsText, nextFrontendRuntimeMode: value }).frontendRuntimeMode)}
            onBackendApiBaseUrlChange={handleBackendApiBaseUrlChange}
            onDirectMeshSelectionModeChange={setDirectMeshSelectionMode}
            onDirectMeshEndpointsTextChange={(value) => { const governed = applyWorkbenchGovernancePatch({ currentFrontendRuntimeMode: frontendRuntimeMode, currentDirectMeshEndpointsText: directMeshEndpointsText, nextDirectMeshEndpointsText: value }); setDirectMeshEndpointsText(governed.directMeshEndpointsText); setFrontendRuntimeMode(governed.frontendRuntimeMode); }}
            onControlPlaneApiTokenChange={setControlPlaneApiToken}
            onClusterApiTokenChange={setClusterApiToken}
            onDirectMeshApiTokenChange={setDirectMeshApiToken}
            onShowShortcutHintsChange={setShowShortcutHints}
            onImmersiveGuardrailsChange={setImmersiveGuardrails}
            onExportDatabase={() => void downloadDatabaseSnapshot()}
          />
          <WorkbenchSystemInstallLayoutCard
            title={t.workflowPackageInstallRulesTitle}
            hint={t.workflowPackageManifestNoneLabel}
          />
          <WorkbenchSystemInstallPolicyMount
            handleWorkflowPanelTabChange={handleWorkflowPanelTabChange}
            setSidebarSection={setSidebarSection}
            t={t}
          />
        </div>
      }
      scriptsContent={
        <WorkbenchScriptPanel
          actionLog={scriptActionLog}
          getSnapshot={getScriptSnapshot}
          language={language}
          recordingMode={scriptRecordingMode}
          onInvokeAction={invokeScriptAction}
          onToggleRecordingMode={() => setScriptRecordingMode((current) => !current)}
          snapshot={scriptSnapshot}
        />
      }
      runtimeContent={
        <WorkbenchSystemRuntimePanel
          recoveryCard={runtimeRecoveryCard}
          overviewTabLabel={t.overview}
          stackTabLabel={t.stack}
          securityTabLabel={t.security}
          agentsTabLabel={t.agents}
          auditTabLabel={t.audit}
          watchdogTabLabel={t.watchdog}
          backendTitle={t.backend}
          backendStatus={healthStatus ?? t.offline}
          backendRows={runtimeBackendRows}
          controlWindow={{ copy: controlWindowCopy, topology: controlWindowTopology, snapshot: controlWindowSnapshot }}
          protocolsTitle={t.protocols}
          protocolsStatus={healthProtocolOnline ? t.online : t.offline}
          protocolRows={runtimeProtocolRows}
          protocolMethods={runtimeProtocolMethods}
          securityTitle={securityUi.security}
          securityStatus={healthSecurityApiTokenConfigured ? securityUi.configured : securityUi.notConfigured}
          securityRows={runtimeSecurityRows}
          securityFooter={<p className="card-copy">{t.runtimeSecurityFooter}</p>}
          auditTitle={t.securityAudit}
          auditCountLabel={String(securityEventRecords.length)}
          auditEmptyLabel={language === "zh" ? "当前筛选下还没有安全事件。" : language === "ja" ? "現在のフィルターに一致するセキュリティイベントはありません。" : "No security events match the current filters."}
          auditSessionLabel={t.auditSessionLabel}
          auditWindowLabel={t.auditWindow}
          auditSourceLabel={t.auditSource}
          auditRiskLabel={t.auditRisk}
          auditStatusLabel={t.auditStatus}
          auditActionLabel={t.auditAction}
          auditSummaryTitle={t.auditSummaryTitle}
          auditSummaryRows={runtimeAuditSummaryRows}
          auditTrendTitle={t.auditTrendTitle}
          auditTrendEmptyLabel={t.auditTrendEmptyLabel}
          auditTrendBars={runtimeAuditTrendBars}
          auditSourceStatusTitle={t.auditSourceStatusTitle}
          auditSourceStatusFacets={runtimeAuditSourceStatusFacets}
          auditStudyFacetTitle={t.auditStudyFacetTitle}
          auditProjectFacetTitle={t.auditProjectFacetTitle}
          auditModelVersionFacetTitle={t.auditModelVersionFacetTitle}
          auditFacetEmptyLabel={t.auditFacetEmptyLabel}
          auditStudyFacets={runtimeAuditStudyFacets}
          auditProjectFacets={runtimeAuditProjectFacets}
          auditModelVersionFacets={runtimeAuditModelVersionFacets}
          auditRefreshLabel={t.auditRefreshLabel}
          auditExportLabel={t.auditExportLabel}
          auditExportCsvLabel={t.auditExportCsvLabel}
          auditWindowValue={securityEventWindowFilter}
          auditSourceValue={securityEventSourceFilter}
          auditRiskValue={securityEventRiskFilter}
          auditStatusValue={securityEventStatusFilter}
          auditActionValue={securityEventActionFilter}
          auditWindowOptions={[
            { value: "", label: t.auditWindowOptions.all },
            { value: "1h", label: t.auditWindowOptions.h1 },
            { value: "24h", label: t.auditWindowOptions.h24 },
            { value: "7d", label: t.auditWindowOptions.d7 },
            { value: "30d", label: t.auditWindowOptions.d30 },
          ]}
          auditSourceOptions={[
            { value: "", label: t.auditSourceOptions.all },
            { value: "assistant", label: t.auditSourceOptions.assistant },
            { value: "hub-assistant", label: t.auditSourceOptions.hubAssistant },
            { value: "script", label: t.auditSourceOptions.script },
          ]}
          auditRiskOptions={[
            { value: "", label: t.auditRiskOptions.all },
            { value: "low", label: t.auditRiskOptions.low },
            { value: "sensitive", label: t.auditRiskOptions.sensitive },
            { value: "high", label: t.auditRiskOptions.high },
            { value: "destructive", label: t.auditRiskOptions.destructive },
          ]}
          auditStatusOptions={[
            { value: "", label: t.auditStatusOptions.all },
            { value: "prompted", label: t.auditStatusOptions.prompted },
            { value: "confirmed", label: t.auditStatusOptions.confirmed },
            { value: "cancelled", label: t.auditStatusOptions.cancelled },
            { value: "completed", label: t.auditStatusOptions.completed },
            { value: "failed", label: t.auditStatusOptions.failed },
          ]}
          onAuditWindowChange={(value) => setSecurityEventWindowFilter(value as SecurityEventWindow)}
          onAuditSourceChange={(value) => setSecurityEventSourceFilter(value as WorkbenchSecurityAuditSource | "hub-assistant" | "")}
          onAuditRiskChange={(value) => setSecurityEventRiskFilter(value as WorkbenchSecurityAuditRisk | "")}
          onAuditStatusChange={(value) => setSecurityEventStatusFilter(value as "" | "allowed" | "blocked")}
          onAuditActionChange={setSecurityEventActionFilter}
          onAuditRefresh={() => void refreshSecurityEvents()}
          onAuditExport={() => void downloadSecurityEventExport()}
          onAuditExportCsv={() => void downloadSecurityEventCsvExport()}
          auditEntries={runtimeAuditEntries}
          protocolAgentsTitle={t.protocolAgents}
          protocolAgentsCountLabel={protocolAgentCountLabel}
          protocolAgentsEmptyLabel={t.noProtocolAgents}
          protocolAgentSummaryRows={protocolAgentSummaryRows}
          protocolAgents={protocolAgentCards}
          watchdogTitle={t.watchdog}
          watchdogStatus={healthWatchdogOnline ? t.online : t.offline}
          watchdogRows={runtimeWatchdogRows}
        />
      }
      dataContent={
        <WorkbenchSystemDataMount
          t={t}
          adminJobRows={adminJobRows}
          adminResultRows={adminResultRows}
          systemDataTab={systemDataTab}
          handleSystemDataTabChange={handleSystemDataTabChange}
          adminFilterProjectId={adminFilterProjectId}
          handleAdminFilterProjectChange={handleAdminFilterProjectChange}
          adminFilterModelVersionId={adminFilterModelVersionId}
          handleAdminFilterModelVersionChange={handleAdminFilterModelVersionChange}
          selectedProjectId={selectedProjectId}
          selectedVersionId={selectedVersionId}
          useCurrentProjectAsAdminFilter={useCurrentProjectAsAdminFilter}
          useCurrentVersionAsAdminFilter={useCurrentVersionAsAdminFilter}
          clearAdminFilters={clearAdminFilters}
          selectedAdminJobId={selectedAdminJobId}
          handleSelectAdminJob={handleSelectAdminJob}
          selectedAdminJob={selectedAdminJob}
          applySelectedAdminJobContext={applySelectedAdminJobContext}
          openSelectedAdminJobProject={openSelectedAdminJobProject}
          openSelectedAdminJobVersion={openSelectedAdminJobVersion}
          jobId={jobId}
          cancelCurrentJob={cancelCurrentJob}
          cancelJob={cancelJob}
          setMessage={setMessage}
          refreshJobHistory={refreshJobHistory}
          adminJobMessage={adminJobMessage}
          setAdminJobMessage={setAdminJobMessage}
          adminJobProjectId={adminJobProjectId}
          setAdminJobProjectId={setAdminJobProjectId}
          adminJobModelVersionId={adminJobModelVersionId}
          setAdminJobModelVersionId={setAdminJobModelVersionId}
          adminJobCaseId={adminJobCaseId}
          setAdminJobCaseId={setAdminJobCaseId}
          saveAdminJobRecord={saveAdminJobRecord}
          deleteAdminJobRecord={deleteAdminJobRecord}
          selectedAdminResultJobId={selectedAdminResultJobId}
          handleSelectAdminResult={handleSelectAdminResult}
          jobHistory={jobHistory}
          adminResultDraft={adminResultDraft}
          setAdminResultDraft={setAdminResultDraft}
          saveAdminResultRecord={saveAdminResultRecord}
          applySelectedAdminResultContext={applySelectedAdminResultContext}
          openSelectedAdminResultProject={openSelectedAdminResultProject}
          openSelectedAdminResultVersion={openSelectedAdminResultVersion}
          exportAdminResultRecord={exportAdminResultRecord}
          deleteAdminResultRecord={deleteAdminResultRecord}
        />
      }
    />
  );
}
