"use client";

import { WorkbenchDataAdminPanel } from "@/components/workbench/system/workbench-data-admin-panel";

type AdminJobRecord = {
  job_id?: string;
  project_id?: string | null;
  model_version_id?: string | null;
  status?: string;
};

type WorkbenchSystemDataMountProps = {
  t: {
    dataAdmin: string;
    databaseRecordCount: string;
    overview: string;
    adminJobs: string;
    adminResults: string;
    adminBrowsePage: string;
    adminEditPage: string;
    historyEmpty: string;
    selectRecord: string;
    cancelJob: string;
    saveRecord: string;
    deleteRecord: string;
    exportRecord: string;
    applyRecordContext: string;
    openLinkedProject: string;
    openLinkedVersion: string;
    filterProject: string;
    filterVersion: string;
    useCurrentProject: string;
    useCurrentVersion: string;
    clearFilters: string;
    adminMessage: string;
    adminProjectId: string;
    adminModelVersionId: string;
    adminCaseId: string;
    resultPayload: string;
    jobCancelled: string;
    initialFailed: string;
  };
  adminJobRows: Array<{
    id: string;
    status: string;
    projectId: string | null;
    heartbeatTone: string;
    heartbeatLabel: string;
    detail: string;
  }>;
  adminResultRows: Array<{
    id: string;
    updatedAt: string;
    projectId: string | null;
    modelVersionId: string | null;
    status: string | null;
    summary: string;
  }>;
  systemDataTab: "jobs" | "results";
  handleSystemDataTabChange: (tab: "jobs" | "results") => void;
  adminFilterProjectId: string;
  handleAdminFilterProjectChange: (value: string) => void;
  adminFilterModelVersionId: string;
  handleAdminFilterModelVersionChange: (value: string) => void;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  useCurrentProjectAsAdminFilter: () => void;
  useCurrentVersionAsAdminFilter: () => void;
  clearAdminFilters: () => void;
  selectedAdminJobId: string | null;
  handleSelectAdminJob: (jobId: string) => void;
  selectedAdminJob: AdminJobRecord | null;
  applySelectedAdminJobContext: () => void;
  openSelectedAdminJobProject: () => void;
  openSelectedAdminJobVersion: () => void;
  jobId: string | null;
  cancelCurrentJob: () => void;
  cancelJob: (jobId: string) => Promise<unknown>;
  setMessage: (value: string) => void;
  refreshJobHistory: () => Promise<void>;
  adminJobMessage: string;
  setAdminJobMessage: (value: string) => void;
  adminJobProjectId: string;
  setAdminJobProjectId: (value: string) => void;
  adminJobModelVersionId: string;
  setAdminJobModelVersionId: (value: string) => void;
  adminJobCaseId: string;
  setAdminJobCaseId: (value: string) => void;
  saveAdminJobRecord: () => void;
  deleteAdminJobRecord: () => void;
  selectedAdminResultJobId: string | null;
  handleSelectAdminResult: (jobId: string) => void;
  jobHistory: Array<{
    job_id: string;
    project_id?: string | null;
    model_version_id?: string | null;
    status: string;
  }>;
  adminResultDraft: string;
  setAdminResultDraft: (value: string) => void;
  saveAdminResultRecord: () => void;
  applySelectedAdminResultContext: () => void;
  openSelectedAdminResultProject: () => void;
  openSelectedAdminResultVersion: () => void;
  exportAdminResultRecord: () => void;
  deleteAdminResultRecord: () => void;
};

export function WorkbenchSystemDataMount({
  t,
  adminJobRows,
  adminResultRows,
  systemDataTab,
  handleSystemDataTabChange,
  adminFilterProjectId,
  handleAdminFilterProjectChange,
  adminFilterModelVersionId,
  handleAdminFilterModelVersionChange,
  selectedProjectId,
  selectedVersionId,
  useCurrentProjectAsAdminFilter,
  useCurrentVersionAsAdminFilter,
  clearAdminFilters,
  selectedAdminJobId,
  handleSelectAdminJob,
  selectedAdminJob,
  applySelectedAdminJobContext,
  openSelectedAdminJobProject,
  openSelectedAdminJobVersion,
  jobId,
  cancelCurrentJob,
  cancelJob,
  setMessage,
  refreshJobHistory,
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
}: WorkbenchSystemDataMountProps) {
  return (
    <WorkbenchDataAdminPanel
      title={t.dataAdmin}
      recordCountLabel={`${t.databaseRecordCount}: ${adminJobRows.length + adminResultRows.length}`}
      overviewTabLabel={t.overview}
      jobsTabLabel={t.adminJobs}
      resultsTabLabel={t.adminResults}
      browsePageLabel={t.adminBrowsePage}
      editPageLabel={t.adminEditPage}
      historyEmptyLabel={t.historyEmpty}
      selectRecordLabel={t.selectRecord}
      cancelJobLabel={t.cancelJob}
      saveRecordLabel={t.saveRecord}
      deleteRecordLabel={t.deleteRecord}
      exportRecordLabel={t.exportRecord}
      applyRecordContextLabel={t.applyRecordContext}
      openLinkedProjectLabel={t.openLinkedProject}
      openLinkedVersionLabel={t.openLinkedVersion}
      filterProjectLabel={t.filterProject}
      filterVersionLabel={t.filterVersion}
      useCurrentProjectLabel={t.useCurrentProject}
      useCurrentVersionLabel={t.useCurrentVersion}
      clearFiltersLabel={t.clearFilters}
      filterProjectValue={adminFilterProjectId}
      onFilterProjectChange={handleAdminFilterProjectChange}
      filterVersionValue={adminFilterModelVersionId}
      onFilterVersionChange={handleAdminFilterModelVersionChange}
      canUseCurrentProject={Boolean(selectedProjectId)}
      canUseCurrentVersion={Boolean(selectedVersionId)}
      onUseCurrentProject={useCurrentProjectAsAdminFilter}
      onUseCurrentVersion={useCurrentVersionAsAdminFilter}
      onClearFilters={clearAdminFilters}
      adminMessageLabel={t.adminMessage}
      adminProjectIdLabel={t.adminProjectId}
      adminModelVersionIdLabel={t.adminModelVersionId}
      adminCaseIdLabel={t.adminCaseId}
      resultPayloadLabel={t.resultPayload}
      activeTab={systemDataTab}
      onTabChange={handleSystemDataTabChange}
      jobRows={adminJobRows}
      selectedAdminJobId={selectedAdminJobId}
      onSelectAdminJob={handleSelectAdminJob}
      selectedAdminJob={Boolean(selectedAdminJob)}
      selectedAdminJobHasVersion={Boolean(selectedAdminJob?.model_version_id)}
      selectedAdminJobHasProject={Boolean(selectedAdminJob?.project_id)}
      selectedAdminJobHasContext={Boolean(selectedAdminJob?.project_id || selectedAdminJob?.model_version_id)}
      canCancelSelectedJob={Boolean(
        selectedAdminJob &&
          selectedAdminJob.status !== "completed" &&
          selectedAdminJob.status !== "failed" &&
          selectedAdminJob.status !== "cancelled",
      )}
      onApplySelectedJobContext={applySelectedAdminJobContext}
      onOpenSelectedJobProject={openSelectedAdminJobProject}
      onOpenSelectedJobVersion={openSelectedAdminJobVersion}
      onCancelSelectedJob={() => {
        if (!selectedAdminJob) return;
        if (selectedAdminJob.job_id === jobId) {
          cancelCurrentJob();
          return;
        }
        void (async () => {
          try {
            if (selectedAdminJob.job_id) {
              await cancelJob(selectedAdminJob.job_id);
            }
            setMessage(t.jobCancelled);
            await refreshJobHistory();
          } catch (error) {
            setMessage(error instanceof Error ? error.message : t.initialFailed);
          }
        })();
      }}
      adminJobMessage={adminJobMessage}
      onAdminJobMessageChange={setAdminJobMessage}
      adminJobProjectId={adminJobProjectId}
      onAdminJobProjectIdChange={setAdminJobProjectId}
      adminJobModelVersionId={adminJobModelVersionId}
      onAdminJobModelVersionIdChange={setAdminJobModelVersionId}
      adminJobCaseId={adminJobCaseId}
      onAdminJobCaseIdChange={setAdminJobCaseId}
      onSaveAdminJob={saveAdminJobRecord}
      onDeleteAdminJob={deleteAdminJobRecord}
      resultRows={adminResultRows}
      selectedAdminResultJobId={selectedAdminResultJobId}
      onSelectAdminResult={handleSelectAdminResult}
      selectedAdminResult={Boolean(selectedAdminResultJobId)}
      selectedAdminResultHasProject={Boolean(
        jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.project_id,
      )}
      selectedAdminResultHasVersion={Boolean(
        jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.model_version_id,
      )}
      selectedAdminResultHasContext={Boolean(
        jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.project_id ||
          jobHistory.find((entry) => entry.job_id === selectedAdminResultJobId)?.model_version_id,
      )}
      adminResultDraft={adminResultDraft}
      onAdminResultDraftChange={setAdminResultDraft}
      onSaveAdminResult={saveAdminResultRecord}
      onApplySelectedResultContext={applySelectedAdminResultContext}
      onOpenSelectedResultProject={openSelectedAdminResultProject}
      onOpenSelectedResultVersion={openSelectedAdminResultVersion}
      onExportAdminResult={exportAdminResultRecord}
      onDeleteAdminResult={deleteAdminResultRecord}
    />
  );
}
