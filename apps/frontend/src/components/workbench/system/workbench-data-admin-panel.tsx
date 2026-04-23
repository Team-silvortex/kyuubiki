"use client";

import { memo } from "react";

import { VirtualList } from "@/components/ui/virtual-list";

type DataTab = "jobs" | "results";

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
  summary: string;
};

type WorkbenchDataAdminPanelProps = {
  title: string;
  recordCountLabel: string;
  jobsTabLabel: string;
  resultsTabLabel: string;
  historyEmptyLabel: string;
  selectRecordLabel: string;
  cancelJobLabel: string;
  saveRecordLabel: string;
  deleteRecordLabel: string;
  exportRecordLabel: string;
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
  canCancelSelectedJob: boolean;
  onCancelSelectedJob: () => void;
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
  adminResultDraft: string;
  onAdminResultDraftChange: (value: string) => void;
  onSaveAdminResult: () => void;
  onExportAdminResult: () => void;
  onDeleteAdminResult: () => void;
};

export const WorkbenchDataAdminPanel = memo(function WorkbenchDataAdminPanel({
  title,
  recordCountLabel,
  jobsTabLabel,
  resultsTabLabel,
  historyEmptyLabel,
  selectRecordLabel,
  cancelJobLabel,
  saveRecordLabel,
  deleteRecordLabel,
  exportRecordLabel,
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
  canCancelSelectedJob,
  onCancelSelectedJob,
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
  adminResultDraft,
  onAdminResultDraftChange,
  onSaveAdminResult,
  onExportAdminResult,
  onDeleteAdminResult,
}: WorkbenchDataAdminPanelProps) {
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
      {activeTab === "jobs" ? (
        <>
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
          {selectedAdminJob ? (
            <>
              <div className="button-row">
                <button className="ghost-button" disabled={!canCancelSelectedJob} onClick={onCancelSelectedJob} type="button">
                  {cancelJobLabel}
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
          ) : (
            <p className="card-copy">{selectRecordLabel}</p>
          )}
        </>
      ) : (
        <>
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
                <small>{entry.summary}</small>
              </button>
            )}
          />
          {selectedAdminResult ? (
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
                <button className="ghost-button" onClick={onExportAdminResult} type="button">
                  {exportRecordLabel}
                </button>
                <button className="ghost-button" onClick={onDeleteAdminResult} type="button">
                  {deleteRecordLabel}
                </button>
              </div>
            </>
          ) : (
            <p className="card-copy">{selectRecordLabel}</p>
          )}
        </>
      )}
    </section>
  );
});
