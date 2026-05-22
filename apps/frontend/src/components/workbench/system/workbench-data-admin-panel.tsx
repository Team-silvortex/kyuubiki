"use client";

import { memo, useState } from "react";

import { VirtualList } from "@/components/ui/virtual-list";

type DataTab = "jobs" | "results";
type DataPage = "overview" | "browse" | "edit";

type AdminJobRow = {
  id: string;
  status: string;
  projectId: string | null;
  heartbeatTone: string;
  heartbeatLabel: string;
  detail: string;
};

type AdminResultRow = {
  id: string;
  updatedAt: string;
  projectId: string | null;
  modelVersionId: string | null;
  status: string | null;
  summary: string;
};

type WorkbenchDataAdminPanelProps = {
  title: string;
  recordCountLabel: string;
  overviewTabLabel: string;
  jobsTabLabel: string;
  resultsTabLabel: string;
  browsePageLabel: string;
  editPageLabel: string;
  historyEmptyLabel: string;
  selectRecordLabel: string;
  cancelJobLabel: string;
  saveRecordLabel: string;
  deleteRecordLabel: string;
  exportRecordLabel: string;
  applyRecordContextLabel: string;
  openLinkedProjectLabel: string;
  openLinkedVersionLabel: string;
  filterProjectLabel: string;
  filterVersionLabel: string;
  useCurrentProjectLabel: string;
  useCurrentVersionLabel: string;
  clearFiltersLabel: string;
  filterProjectValue: string;
  onFilterProjectChange: (value: string) => void;
  filterVersionValue: string;
  onFilterVersionChange: (value: string) => void;
  canUseCurrentProject: boolean;
  canUseCurrentVersion: boolean;
  onUseCurrentProject: () => void;
  onUseCurrentVersion: () => void;
  onClearFilters: () => void;
  adminMessageLabel: string;
  adminProjectIdLabel: string;
  adminModelVersionIdLabel: string;
  adminCaseIdLabel: string;
  resultPayloadLabel: string;
  activeTab: DataTab;
  onTabChange: (tab: DataTab) => void;
  jobRows: AdminJobRow[];
  selectedAdminJobId: string | null;
  onSelectAdminJob: (jobId: string) => void;
  selectedAdminJob: boolean;
  selectedAdminJobHasVersion: boolean;
  selectedAdminJobHasProject: boolean;
  selectedAdminJobHasContext: boolean;
  canCancelSelectedJob: boolean;
  onCancelSelectedJob: () => void;
  onApplySelectedJobContext: () => void;
  onOpenSelectedJobProject: () => void;
  onOpenSelectedJobVersion: () => void;
  adminJobMessage: string;
  onAdminJobMessageChange: (value: string) => void;
  adminJobProjectId: string;
  onAdminJobProjectIdChange: (value: string) => void;
  adminJobModelVersionId: string;
  onAdminJobModelVersionIdChange: (value: string) => void;
  adminJobCaseId: string;
  onAdminJobCaseIdChange: (value: string) => void;
  onSaveAdminJob: () => void;
  onDeleteAdminJob: () => void;
  resultRows: AdminResultRow[];
  selectedAdminResultJobId: string | null;
  onSelectAdminResult: (jobId: string) => void;
  selectedAdminResult: boolean;
  selectedAdminResultHasProject: boolean;
  selectedAdminResultHasVersion: boolean;
  selectedAdminResultHasContext: boolean;
  adminResultDraft: string;
  onAdminResultDraftChange: (value: string) => void;
  onSaveAdminResult: () => void;
  onApplySelectedResultContext: () => void;
  onOpenSelectedResultProject: () => void;
  onOpenSelectedResultVersion: () => void;
  onExportAdminResult: () => void;
  onDeleteAdminResult: () => void;
};

export const WorkbenchDataAdminPanel = memo(function WorkbenchDataAdminPanel({
  title,
  recordCountLabel,
  overviewTabLabel,
  jobsTabLabel,
  resultsTabLabel,
  browsePageLabel,
  editPageLabel,
  historyEmptyLabel,
  selectRecordLabel,
  cancelJobLabel,
  saveRecordLabel,
  deleteRecordLabel,
  exportRecordLabel,
  applyRecordContextLabel,
  openLinkedProjectLabel,
  openLinkedVersionLabel,
  filterProjectLabel,
  filterVersionLabel,
  useCurrentProjectLabel,
  useCurrentVersionLabel,
  clearFiltersLabel,
  filterProjectValue,
  onFilterProjectChange,
  filterVersionValue,
  onFilterVersionChange,
  canUseCurrentProject,
  canUseCurrentVersion,
  onUseCurrentProject,
  onUseCurrentVersion,
  onClearFilters,
  adminMessageLabel,
  adminProjectIdLabel,
  adminModelVersionIdLabel,
  adminCaseIdLabel,
  resultPayloadLabel,
  activeTab,
  onTabChange,
  jobRows,
  selectedAdminJobId,
  onSelectAdminJob,
  selectedAdminJob,
  selectedAdminJobHasVersion,
  selectedAdminJobHasProject,
  selectedAdminJobHasContext,
  canCancelSelectedJob,
  onCancelSelectedJob,
  onApplySelectedJobContext,
  onOpenSelectedJobProject,
  onOpenSelectedJobVersion,
  adminJobMessage,
  onAdminJobMessageChange,
  adminJobProjectId,
  onAdminJobProjectIdChange,
  adminJobModelVersionId,
  onAdminJobModelVersionIdChange,
  adminJobCaseId,
  onAdminJobCaseIdChange,
  onSaveAdminJob,
  onDeleteAdminJob,
  resultRows,
  selectedAdminResultJobId,
  onSelectAdminResult,
  selectedAdminResult,
  selectedAdminResultHasProject,
  selectedAdminResultHasVersion,
  selectedAdminResultHasContext,
  adminResultDraft,
  onAdminResultDraftChange,
  onSaveAdminResult,
  onApplySelectedResultContext,
  onOpenSelectedResultProject,
  onOpenSelectedResultVersion,
  onExportAdminResult,
  onDeleteAdminResult,
}: WorkbenchDataAdminPanelProps) {
  const [dataPage, setDataPage] = useState<DataPage>("overview");
  const latestJob = jobRows[0];
  const waitingJobsCount = jobRows.filter((row) => row.status !== "completed").length;
  const latestResult = resultRows[0];
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{recordCountLabel}</span>
      </div>
      <div className="panel-tabs">
        <button className={`panel-tab${activeTab === "jobs" ? " panel-tab--active" : ""}`} onClick={() => onTabChange("jobs")} type="button">
          {jobsTabLabel}
        </button>
        <button className={`panel-tab${activeTab === "results" ? " panel-tab--active" : ""}`} onClick={() => onTabChange("results")} type="button">
          {resultsTabLabel}
        </button>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${dataPage === "overview" ? " panel-tab--active" : ""}`} onClick={() => setDataPage("overview")} type="button">
          {overviewTabLabel}
        </button>
        <button className={`panel-tab${dataPage === "browse" ? " panel-tab--active" : ""}`} onClick={() => setDataPage("browse")} type="button">
          {browsePageLabel}
        </button>
        <button className={`panel-tab${dataPage === "edit" ? " panel-tab--active" : ""}`} onClick={() => setDataPage("edit")} type="button">
          {editPageLabel}
        </button>
      </div>
      {dataPage === "overview" ? (
        <div className="runtime-overview-grid">
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{jobsTabLabel}</h2>
              <span>{String(jobRows.length)}</span>
            </div>
            <div className="sidebar-list sidebar-list--metrics">
              <div className="sidebar-list__row">
                <span>{recordCountLabel}</span>
                <strong>{jobRows.length}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{jobsTabLabel}</span>
                <strong>{latestJob?.status ?? "--"}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{browsePageLabel}</span>
                <strong>{waitingJobsCount}</strong>
              </div>
            </div>
            <div className="button-row">
              <button
                onClick={() => {
                  onTabChange("jobs");
                  setDataPage("browse");
                }}
                type="button"
              >
                {jobsTabLabel}
              </button>
            </div>
          </section>

          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{resultsTabLabel}</h2>
              <span>{String(resultRows.length)}</span>
            </div>
            <div className="sidebar-list sidebar-list--metrics">
              <div className="sidebar-list__row">
                <span>{recordCountLabel}</span>
                <strong>{resultRows.length}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{resultsTabLabel}</span>
                <strong>{latestResult?.status ?? "--"}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{filterVersionLabel}</span>
                <strong>{latestResult?.modelVersionId ?? "--"}</strong>
              </div>
            </div>
            <div className="button-row">
              <button
                onClick={() => {
                  onTabChange("results");
                  setDataPage("browse");
                }}
                type="button"
              >
                {resultsTabLabel}
              </button>
            </div>
          </section>
        </div>
      ) : null}
      {dataPage === "browse" ? (
        <>
          <div className="form-grid compact">
            <label>
              <span>{filterProjectLabel}</span>
              <input value={filterProjectValue} onChange={(event) => onFilterProjectChange(event.target.value)} />
            </label>
            <label>
              <span>{filterVersionLabel}</span>
              <input value={filterVersionValue} onChange={(event) => onFilterVersionChange(event.target.value)} />
            </label>
          </div>
          <div className="button-row">
            <button className="ghost-button ghost-button--compact" disabled={!canUseCurrentProject} onClick={onUseCurrentProject} type="button">
              {useCurrentProjectLabel}
            </button>
            <button className="ghost-button ghost-button--compact" disabled={!canUseCurrentVersion} onClick={onUseCurrentVersion} type="button">
              {useCurrentVersionLabel}
            </button>
            <button className="ghost-button ghost-button--compact" onClick={onClearFilters} type="button">
              {clearFiltersLabel}
            </button>
          </div>
        </>
      ) : null}
      {activeTab === "jobs" ? (
        <>
          {dataPage === "browse" ? (
            <VirtualList
              className="history-list"
              items={jobRows}
              itemHeight={112}
              maxHeight={220}
              emptyState={<p className="card-copy">{historyEmptyLabel}</p>}
              itemKey={(entry) => entry.id}
              renderItem={(entry) => (
                <button
                  className={`history-item${selectedAdminJobId === entry.id ? " history-item--active" : ""}`}
                  onClick={() => onSelectAdminJob(entry.id)}
                  type="button"
                >
                  <strong>{entry.id.slice(0, 8)}</strong>
                  <span>{entry.status}</span>
                  <small>{entry.projectId}</small>
                  <small>
                    <span className={`heartbeat-badge heartbeat-badge--${entry.heartbeatTone}`}>{entry.heartbeatLabel}</span>
                  </small>
                  <small>{entry.detail}</small>
                </button>
              )}
            />
          ) : null}
          {dataPage === "edit" && selectedAdminJob ? (
            <>
              <div className="button-row">
                <button className="ghost-button" disabled={!canCancelSelectedJob} onClick={onCancelSelectedJob} type="button">
                  {cancelJobLabel}
                </button>
                <button
                  className="ghost-button"
                  disabled={!selectedAdminJobHasContext}
                  onClick={onApplySelectedJobContext}
                  type="button"
                >
                  {applyRecordContextLabel}
                </button>
                <button
                  className="ghost-button"
                  disabled={!selectedAdminJobHasProject}
                  onClick={onOpenSelectedJobProject}
                  type="button"
                >
                  {openLinkedProjectLabel}
                </button>
                <button
                  className="ghost-button"
                  disabled={!selectedAdminJobHasVersion}
                  onClick={onOpenSelectedJobVersion}
                  type="button"
                >
                  {openLinkedVersionLabel}
                </button>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{adminMessageLabel}</span>
                  <input value={adminJobMessage} onChange={(event) => onAdminJobMessageChange(event.target.value)} />
                </label>
                <label>
                  <span>{adminProjectIdLabel}</span>
                  <input value={adminJobProjectId} onChange={(event) => onAdminJobProjectIdChange(event.target.value)} />
                </label>
                <label>
                  <span>{adminModelVersionIdLabel}</span>
                  <input value={adminJobModelVersionId} onChange={(event) => onAdminJobModelVersionIdChange(event.target.value)} />
                </label>
                <label>
                  <span>{adminCaseIdLabel}</span>
                  <input value={adminJobCaseId} onChange={(event) => onAdminJobCaseIdChange(event.target.value)} />
                </label>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={onSaveAdminJob} type="button">
                  {saveRecordLabel}
                </button>
                <button className="ghost-button" onClick={onDeleteAdminJob} type="button">
                  {deleteRecordLabel}
                </button>
              </div>
            </>
          ) : dataPage === "edit" ? (
            <p className="card-copy">{selectRecordLabel}</p>
          ) : null}
        </>
      ) : (
        <>
          {dataPage === "browse" ? (
            <VirtualList
              className="history-list"
              items={resultRows}
              itemHeight={88}
              maxHeight={220}
              emptyState={<p className="card-copy">{historyEmptyLabel}</p>}
              itemKey={(entry) => entry.id}
              renderItem={(entry) => (
                <button
                  className={`history-item${selectedAdminResultJobId === entry.id ? " history-item--active" : ""}`}
                  onClick={() => onSelectAdminResult(entry.id)}
                  type="button"
                >
                  <strong>{entry.id.slice(0, 8)}</strong>
                  <span>{entry.updatedAt}</span>
                  <small>{entry.status}</small>
                  <small>{entry.projectId}</small>
                  <small>{entry.modelVersionId}</small>
                  <small>{entry.summary}</small>
                </button>
              )}
            />
          ) : null}
          {dataPage === "edit" && selectedAdminResult ? (
            <>
              <div className="form-grid compact">
                <label>
                  <span>{resultPayloadLabel}</span>
                  <textarea
                    className="json-editor"
                    value={adminResultDraft}
                    onChange={(event) => onAdminResultDraftChange(event.target.value)}
                    rows={10}
                  />
                </label>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={onSaveAdminResult} type="button">
                  {saveRecordLabel}
                </button>
                <button
                  className="ghost-button"
                  disabled={!selectedAdminResultHasContext}
                  onClick={onApplySelectedResultContext}
                  type="button"
                >
                  {applyRecordContextLabel}
                </button>
                <button
                  className="ghost-button"
                  disabled={!selectedAdminResultHasProject}
                  onClick={onOpenSelectedResultProject}
                  type="button"
                >
                  {openLinkedProjectLabel}
                </button>
                <button
                  className="ghost-button"
                  disabled={!selectedAdminResultHasVersion}
                  onClick={onOpenSelectedResultVersion}
                  type="button"
                >
                  {openLinkedVersionLabel}
                </button>
                <button className="ghost-button" onClick={onExportAdminResult} type="button">
                  {exportRecordLabel}
                </button>
                <button className="ghost-button" onClick={onDeleteAdminResult} type="button">
                  {deleteRecordLabel}
                </button>
              </div>
            </>
          ) : dataPage === "edit" ? (
            <p className="card-copy">{selectRecordLabel}</p>
          ) : null}
        </>
      )}
    </section>
  );
});
