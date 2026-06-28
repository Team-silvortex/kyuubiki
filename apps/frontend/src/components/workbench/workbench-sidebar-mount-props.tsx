"use client";

import { lazy, Suspense, type ReactNode } from "react";
import type { WorkbenchUiChunkId } from "@/components/workbench/workbench-ui-streaming";

const WorkbenchLibrarySectionMount = lazy(() =>
  import("@/components/workbench/workbench-library-section-mount").then((module) => ({
    default: module.WorkbenchLibrarySectionMount,
  })),
);
const WorkbenchModelSectionMount = lazy(() =>
  import("@/components/workbench/workbench-model-section-mount").then((module) => ({
    default: module.WorkbenchModelSectionMount,
  })),
);
const WorkbenchStudySectionMount = lazy(() =>
  import("@/components/workbench/workbench-study-section-mount").then((module) => ({
    default: module.WorkbenchStudySectionMount,
  })),
);
const WorkbenchSystemSidebarMount = lazy(() =>
  import("@/components/workbench/workbench-system-sidebar-mount").then((module) => ({
    default: module.WorkbenchSystemSidebarMount,
  })),
);
const WorkbenchWorkflowSectionMount = lazy(() =>
  import("@/components/workbench/workbench-workflow-section-mount").then((module) => ({
    default: module.WorkbenchWorkflowSectionMount,
  })),
);

export function buildWorkbenchSidebarMountProps(props: Record<string, any>) {
  return {
    shortTitle: props.t.shortTitle,
    roleLabel: props.t.roleLabel,
    title: props.t.title,
    subtitle: props.t.subtitle,
    railItems: props.railItems,
    sidebarSection: props.sidebarSection,
    onSidebarSectionChange: props.handleSidebarSectionChange,
    assistantLabel: props.t.assistant,
    assistantOpen: props.assistantWindowOpen,
    onAssistantToggle: () => props.setAssistantWindowOpen((current: boolean) => !current),
    studySection: sectionChunk(
      "section.study",
      <WorkbenchStudySectionMount
        studyTab={props.studyTab}
        onStudyTabChange={props.handleStudyTabChange}
        sectionTitle={props.t.sections.study}
        summaryTabLabel={props.t.tabs.summary}
        controlsTabLabel={props.t.tabs.controls}
        loadedModelName={props.loadedModelName}
        studyTypeLabel={props.t.studyTypeLabel}
        studyKind={props.studyKind}
        studyDomainLabel={props.t.studyDomain}
        studyDomainOptions={props.studyDomainOptions}
        noDomainStudiesLabel={props.t.noDomainStudies}
        studyKindOptionGroups={props.studyKindOptionGroups}
        onStudyKindChange={props.selectStudyKind}
        summaryRows={props.studySummaryRows}
        controlsRows={props.studyControlsRows}
        controlsContent={props.studyControlsContent}
        controlsTitle={props.t.controls}
        controlsSetupPageLabel={props.t.controlsSetupPage}
        controlsReviewPageLabel={props.t.controlsReviewPage}
        readyLabel={props.t.ready}
        busyLabel={props.t.busy}
        isPending={props.isPending}
        runLabel={props.t.run}
        runningLabel={props.t.running}
        onRun={props.runAnalysis}
      />,
    ),
    modelSection: sectionChunk(
      "section.model",
      <WorkbenchModelSectionMount
        modelTab={props.modelTab}
        onModelTabChange={props.handleModelTabChange}
        toolsPage={props.modelToolsPage}
        onToolsPageChange={props.handleModelToolsPageChange}
        isTruss3d={props.isTruss3d}
        toolsTabLabel={props.t.tabs.tools}
        treeTabLabel={props.t.tabs.tree}
        toolsPageOverviewLabel={props.t.modelOverviewPage}
        toolsPageStudyLabel={props.t.modelStudyPage}
        toolsPageStudioLabel={props.t.modelStudioPage}
        toolsPageMaterialsLabel={props.t.modelMaterialsPage}
        toolsPageGenerateLabel={props.t.modelGeneratePage}
        studyOverviewHint={props.t.workspaceStudyHint}
        studioOverviewHint={props.t.workspaceStudioHint}
        materialsOverviewHint={props.t.workspaceMaterialsHint}
        generateOverviewHint={props.t.workspaceGenerateHint}
        browseOverviewHint={props.t.workspaceBrowseHint}
        studyContent={props.modelStudyContent}
        studioContent={props.modelStudioContent}
        materialsContent={props.modelMaterialsContent}
        generateContent={props.modelGenerateContent}
        treeContent={props.modelTreeContent}
      />,
    ),
    workflowSection: sectionChunk(
      "section.workflow",
      <WorkbenchWorkflowSectionMount
        surfaceTab={props.workflowPanelTab}
        onSurfaceTabChange={props.handleWorkflowPanelTabChange}
        labels={props.workflowLabels}
        workflowCatalogEntries={props.workflowCatalog}
        workflowOperatorDescriptors={props.workflowOperatorDescriptors}
        workflowOperatorModules={props.workflowOperatorModules}
        workflowCatalogBusy={props.workflowCatalogBusy}
        selectedWorkflowId={props.selectedWorkflowId}
        selectedWorkflow={props.selectedWorkflow}
        currentStudyKind={props.currentStudyKind}
        currentHeatPlaneModel={props.currentHeatPlaneModel}
        currentPlaneModel={props.currentPlaneModel}
        latestJob={props.job}
        latestWorkflowSummary={props.latestWorkflowSummary}
        workflowRuns={props.workflowRuns}
        protocolAgents={props.protocolAgents}
        frontendRuntimeMode={props.frontendRuntimeMode}
        refreshWorkflowCatalog={props.refreshWorkflowCatalog}
        setSelectedWorkflowId={props.setSelectedWorkflowId}
        setWorkflowRuns={props.setWorkflowRuns}
        runWorkflowCatalogEntry={props.runWorkflowCatalogEntry}
        runWorkflowDraft={props.runWorkflowDraft}
        openHistoryJob={props.openHistoryJob}
        setSystemAlerts={props.setSystemAlerts}
      />,
    ),
    librarySection: sectionChunk(
      "section.library",
      <WorkbenchLibrarySectionMount
        labels={props.t}
        libraryTab={props.libraryTab}
        onLibraryTabChange={props.handleLibraryTabChange}
        sampleRows={props.librarySampleRows}
        workflowCatalogEntries={props.workflowCatalog}
        workflowCatalogBusy={props.workflowCatalogBusy}
        projects={props.projects}
        selectedProjectId={props.selectedProjectId}
        setSelectedProjectId={props.setSelectedProjectId}
        setSelectedModelId={props.setSelectedModelId}
        projectNameDraft={props.projectNameDraft}
        setProjectNameDraft={props.setProjectNameDraft}
        projectDescriptionDraft={props.projectDescriptionDraft}
        setProjectDescriptionDraft={props.setProjectDescriptionDraft}
        createProjectRecord={props.createProjectRecord}
        updateProjectRecord={props.updateProjectRecord}
        deleteProjectRecord={props.deleteProjectRecord}
        downloadProjectBundleJson={props.downloadProjectBundleJson}
        downloadProjectBundleZip={props.downloadProjectBundleZip}
        importProjectBundle={props.importProjectBundle}
        selectedProjectModels={props.selectedProjectModels}
        modelRows={props.libraryModelRows}
        selectedModelId={props.selectedModelId}
        loadedModelName={props.loadedModelName}
        setLoadedModelName={props.setLoadedModelName}
        saveModelVersion={props.saveModelVersion}
        deleteSavedModelRecord={props.deleteSavedModelRecord}
        openSavedModel={props.openSavedModel}
        versionRows={props.libraryVersionRows}
        modelVersions={props.modelVersions}
        selectedVersionId={props.selectedVersionId}
        renameSelectedVersion={props.renameSelectedVersion}
        deleteSelectedVersion={props.deleteSelectedVersion}
        openSavedVersion={props.openSavedVersion}
        jobRows={props.libraryJobRows}
        jobCount={props.jobHistory?.length ?? 0}
        activeJobId={props.job?.job_id ?? null}
        openHistoryJob={props.openHistoryJob}
        openSample={props.openSample}
        refreshWorkflowCatalog={props.refreshWorkflowCatalog}
        runWorkflowCatalogEntry={props.runWorkflowCatalogEntry}
        refreshJobHistory={props.refreshJobHistory}
        refreshProjects={props.refreshProjects}
        importModel={props.importModel}
      />,
    ),
    systemSection: sectionChunk(
      "section.system",
      <WorkbenchSystemSidebarMount
        t={props.t}
        systemPanelTab={props.systemPanelTab === "assistant" ? "config" : props.systemPanelTab}
        handleSystemPanelTabChange={props.handleSystemPanelTabChange}
        setSidebarSection={props.setSidebarSection}
        handleWorkflowPanelTabChange={props.handleWorkflowPanelTabChange}
        runtimeRecoveryCard={props.runtimeRecoveryCard}
        healthStatus={props.health?.status}
        healthProtocolOnline={Boolean(props.health?.protocol)}
        healthWatchdogOnline={Boolean(props.health?.watchdog)}
        healthSecurityApiTokenConfigured={Boolean(props.health?.security?.api_token_configured)}
        runtimeBackendRows={props.runtimeBackendRows}
        runtimeProtocolRows={props.runtimeProtocolRows}
        runtimeProtocolMethods={props.runtimeProtocolMethods}
        securityUi={props.securityUi}
        runtimeSecurityRows={props.runtimeSecurityRows}
        runtimeAuditSummaryRows={props.runtimeAuditSummaryRows}
        runtimeAuditTrendBars={props.runtimeAuditTrendBars}
        runtimeAuditSourceStatusFacets={props.runtimeAuditSourceStatusFacets}
        runtimeAuditStudyFacets={props.runtimeAuditStudyFacets}
        runtimeAuditProjectFacets={props.runtimeAuditProjectFacets}
        runtimeAuditModelVersionFacets={props.runtimeAuditModelVersionFacets}
        securityEventRecords={props.securityEventRecords}
        securityEventWindowFilter={props.securityEventWindowFilter}
        securityEventSourceFilter={props.securityEventSourceFilter}
        securityEventRiskFilter={props.securityEventRiskFilter}
        securityEventStatusFilter={props.securityEventStatusFilter}
        securityEventActionFilter={props.securityEventActionFilter}
        setSecurityEventWindowFilter={props.setSecurityEventWindowFilter}
        setSecurityEventSourceFilter={props.setSecurityEventSourceFilter}
        setSecurityEventRiskFilter={props.setSecurityEventRiskFilter}
        setSecurityEventStatusFilter={props.setSecurityEventStatusFilter}
        setSecurityEventActionFilter={props.setSecurityEventActionFilter}
        refreshSecurityEvents={props.refreshSecurityEvents}
        downloadSecurityEventExport={props.downloadSecurityEventExport}
        downloadSecurityEventCsvExport={props.downloadSecurityEventCsvExport}
        runtimeAuditEntries={props.runtimeAuditEntries}
        protocolAgents={props.protocolAgents}
        protocolAgentCards={props.protocolAgentCards}
        runtimeWatchdogRows={props.runtimeWatchdogRows}
        theme={props.theme}
        language={props.language}
        frontendRuntimeMode={props.frontendRuntimeMode}
        directMeshSelectionMode={props.directMeshSelectionMode}
        directMeshEndpointsText={props.directMeshEndpointsText}
        controlPlaneApiToken={props.controlPlaneApiToken}
        clusterApiToken={props.clusterApiToken}
        directMeshApiToken={props.directMeshApiToken}
        showShortcutHints={props.showShortcutHints}
        immersiveGuardrails={props.immersiveGuardrails}
        languagePacks={props.languagePacks}
        languagePackCatalogRows={props.languagePackCatalogRows}
        setTheme={props.setTheme}
        handleLanguageChange={props.handleLanguageChange}
        handleDownloadLanguagePackTemplate={props.handleDownloadLanguagePackTemplate}
        handleExportInstalledLanguagePack={props.handleExportInstalledLanguagePack}
        handleImportLanguagePack={props.handleImportLanguagePack}
        handleRemoveLanguagePack={props.handleRemoveLanguagePack}
        setFrontendRuntimeMode={props.setFrontendRuntimeMode}
        setDirectMeshSelectionMode={props.setDirectMeshSelectionMode}
        setDirectMeshEndpointsText={props.setDirectMeshEndpointsText}
        setControlPlaneApiToken={props.setControlPlaneApiToken}
        setClusterApiToken={props.setClusterApiToken}
        setDirectMeshApiToken={props.setDirectMeshApiToken}
        setShowShortcutHints={props.setShowShortcutHints}
        setImmersiveGuardrails={props.setImmersiveGuardrails}
        downloadDatabaseSnapshot={props.downloadDatabaseSnapshot}
        scriptActionLog={props.scriptActionLog}
        getScriptSnapshot={props.getScriptSnapshot}
        scriptRecordingMode={props.scriptRecordingMode}
        invokeScriptAction={async (action: string, payload?: Record<string, unknown>) => {
          await props.invokeScriptAction(action, payload);
          return {};
        }}
        setScriptRecordingMode={props.setScriptRecordingMode}
        scriptSnapshot={props.scriptSnapshot}
        systemDataTab={props.systemDataTab}
        handleSystemDataTabChange={props.handleSystemDataTabChange}
        adminJobRows={props.adminJobRows}
        selectedAdminJobId={props.selectedAdminJobId}
        handleSelectAdminJob={props.handleSelectAdminJob}
        selectedAdminJob={props.selectedAdminJob}
        adminJobMessage={props.adminJobMessage}
        setAdminJobMessage={props.setAdminJobMessage}
        adminJobProjectId={props.adminJobProjectId}
        setAdminJobProjectId={props.setAdminJobProjectId}
        adminJobModelVersionId={props.adminJobModelVersionId}
        setAdminJobModelVersionId={props.setAdminJobModelVersionId}
        adminJobCaseId={props.adminJobCaseId}
        setAdminJobCaseId={props.setAdminJobCaseId}
        saveAdminJobRecord={props.saveAdminJobRecord}
        deleteAdminJobRecord={props.deleteAdminJobRecord}
        adminResultRows={props.adminResultRows}
        selectedAdminResultJobId={props.selectedAdminResultJobId}
        handleSelectAdminResult={props.handleSelectAdminResult}
        jobHistory={props.jobHistory}
        adminResultDraft={props.adminResultDraft}
        setAdminResultDraft={props.setAdminResultDraft}
        saveAdminResultRecord={props.saveAdminResultRecord}
        applySelectedAdminResultContext={props.applySelectedAdminResultContext}
        openSelectedAdminResultProject={props.openSelectedAdminResultProject}
        openSelectedAdminResultVersion={props.openSelectedAdminResultVersion}
        exportAdminResultRecord={props.exportAdminResultRecord}
        deleteAdminResultRecord={props.deleteAdminResultRecord}
        adminFilterProjectId={props.adminFilterProjectId}
        handleAdminFilterProjectChange={props.handleAdminFilterProjectChange}
        adminFilterModelVersionId={props.adminFilterModelVersionId}
        handleAdminFilterModelVersionChange={props.handleAdminFilterModelVersionChange}
        selectedProjectId={props.selectedProjectId}
        selectedVersionId={props.selectedVersionId}
        useCurrentProjectAsAdminFilter={props.useCurrentProjectAsAdminFilter}
        useCurrentVersionAsAdminFilter={props.useCurrentVersionAsAdminFilter}
        clearAdminFilters={props.clearAdminFilters}
        applySelectedAdminJobContext={props.applySelectedAdminJobContext}
        openSelectedAdminJobProject={props.openSelectedAdminJobProject}
        openSelectedAdminJobVersion={props.openSelectedAdminJobVersion}
        jobId={props.job?.job_id ?? null}
        cancelCurrentJob={props.cancelCurrentJob}
        cancelJob={props.cancelJob}
        setMessage={props.setMessage}
        refreshJobHistory={props.refreshJobHistory}
      />
    ),
  };
}

function sectionChunk(chunkId: WorkbenchUiChunkId, children: ReactNode) {
  return (
    <Suspense fallback={<SidebarChunkFallback chunkId={chunkId} />}>
      <div className="workbench-ui-chunk" data-workbench-ui-chunk={chunkId}>
        {children}
      </div>
    </Suspense>
  );
}

function SidebarChunkFallback({ chunkId }: { chunkId: WorkbenchUiChunkId }) {
  return (
    <div
      className="sidebar-stack panel-scroll-window"
      data-workbench-ui-chunk={chunkId}
      data-workbench-ui-chunk-loading="true"
    >
      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>Loading</h2>
          <span>{chunkId}</span>
        </div>
        <p className="muted-copy">Loading this workspace panel only when it is needed.</p>
      </section>
    </div>
  );
}
