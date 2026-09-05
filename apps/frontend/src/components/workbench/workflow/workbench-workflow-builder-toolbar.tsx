"use client";

import { useId, type ChangeEvent, type RefObject } from "react";
import { WorkbenchPanelNotice } from "@/components/workbench/workbench-panel-notice";
import type { WorkbenchNoticeItem, WorkbenchNoticeStateSetter } from "@/components/workbench/workbench-notice-state";
import type { WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowBuilderToolbarProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry;
  canRunDraft: boolean;
  canExportDataset: boolean;
  draftBlockingIssueCount: number;
  draftBlockerMessage: string | null;
  importNotice: WorkbenchNoticeItem | null;
  setImportNotice: WorkbenchNoticeStateSetter;
  graphInputRef: RefObject<HTMLInputElement | null>;
  datasetInputRef: RefObject<HTMLInputElement | null>;
  onRunCatalog: () => void;
  onRunDraft: () => void;
  onLocateDraftBlocker: () => void;
  onSaveDraft: () => void;
  onPromoteDraft: () => void;
  onDuplicateLocalWorkflow: () => void;
  onRenameLocalWorkflow: () => void;
  onDeleteLocalWorkflow: () => void;
  onExportGraph: () => void;
  onExportDataset: () => void;
  onGraphFileChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onDatasetFileChange: (event: ChangeEvent<HTMLInputElement>) => void;
};

export function WorkbenchWorkflowBuilderToolbar({
  labels,
  selectedWorkflow,
  canRunDraft,
  canExportDataset,
  draftBlockingIssueCount,
  draftBlockerMessage,
  importNotice,
  setImportNotice,
  graphInputRef,
  datasetInputRef,
  onRunCatalog,
  onRunDraft,
  onLocateDraftBlocker,
  onSaveDraft,
  onPromoteDraft,
  onDuplicateLocalWorkflow,
  onRenameLocalWorkflow,
  onDeleteLocalWorkflow,
  onExportGraph,
  onExportDataset,
  onGraphFileChange,
  onDatasetFileChange,
}: WorkbenchWorkflowBuilderToolbarProps) {
  const blockerDescriptionId = useId();
  const localWorkflowTags = selectedWorkflow.local?.tags?.filter(Boolean).join(", ") ?? null;
  const promotedAt = selectedWorkflow.local?.promoted_at
    ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(
        new Date(selectedWorkflow.local.promoted_at),
      )
    : null;
  const draftStatusTone = canRunDraft ? "good" : "watch";
  const draftStatusLabel = canRunDraft ? labels.statusReadyLabel : draftBlockingIssueCount > 0 ? String(draftBlockingIssueCount) : "--";
  return (
    <section
      className="workflow-builder-toolbar"
      data-workflow-draft-readiness={canRunDraft ? "ready" : draftBlockerMessage ? "blocked" : "unavailable"}
      data-workflow-draft-blocker-count={draftBlockingIssueCount}
    >
      <div className="card-head">
        <h2 title={selectedWorkflow.name}>{selectedWorkflow.name}</h2>
        <span className="workflow-builder-toolbar__status" data-workflow-builder-toolbar="status">
          <span className="status-pill status-pill--good">{selectedWorkflow.version}</span>
          <span className={`status-pill status-pill--${draftStatusTone}`} title={labels.runDraftLabel}>{draftStatusLabel}</span>
        </span>
      </div>
      <p className="card-copy">{selectedWorkflow.summary}</p>
      {selectedWorkflow.local ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.localWorkflowSourceLabel}</span>
            <strong>{selectedWorkflow.local.source_workflow_name ?? selectedWorkflow.local.source_workflow_id ?? "--"}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{labels.localWorkflowPromotedAtLabel}</span>
            <strong>{promotedAt ?? "--"}</strong>
          </div>
          {selectedWorkflow.local.variant_of_workflow_name || selectedWorkflow.local.variant_of_workflow_id ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowVariantOfLabel}</span>
              <strong>{selectedWorkflow.local.variant_of_workflow_name ?? selectedWorkflow.local.variant_of_workflow_id}</strong>
            </div>
          ) : null}
          {selectedWorkflow.local.imported_from_package_id ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowPackageIdLabel}</span>
              <strong>{selectedWorkflow.local.imported_from_package_id}</strong>
            </div>
          ) : null}
          {selectedWorkflow.local.imported_from_package_version ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowPackageVersionLabel}</span>
              <strong>{selectedWorkflow.local.imported_from_package_version}</strong>
            </div>
          ) : null}
          {localWorkflowTags ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowTagsLabel}</span>
              <strong>{localWorkflowTags}</strong>
            </div>
          ) : null}
        </div>
      ) : null}
      {selectedWorkflow.local?.notes ? <p className="card-copy">{selectedWorkflow.local.notes}</p> : null}
      {draftBlockerMessage ? (
        <div className="workflow-builder-blocker" data-workflow-draft-blocker="summary">
          <div aria-live="polite" id={blockerDescriptionId} role="status">
            <strong>{labels.validationTitle}: {draftBlockingIssueCount}</strong>
            <p className="workflow-builder-blocker__message" title={draftBlockerMessage}>{draftBlockerMessage}</p>
          </div>
          <button
            aria-describedby={blockerDescriptionId}
            data-workflow-builder-action="locate-blocker"
            onClick={onLocateDraftBlocker}
            type="button"
          >
            {labels.validationLocateLabel}
          </button>
        </div>
      ) : null}
      <div className="button-row button-row--adaptive" data-workflow-builder-toolbar="actions">
        <button aria-describedby={draftBlockerMessage ? blockerDescriptionId : undefined} data-workflow-builder-action="run-draft" disabled={!canRunDraft} onClick={onRunDraft} type="button">{labels.runDraftLabel}</button>
        <button data-workflow-builder-action="save-draft" onClick={onSaveDraft} type="button">{labels.saveDraftLabel}</button>
      </div>
      <details className="workflow-builder-more-actions" data-workflow-builder-tools="secondary">
        <summary aria-label={`${labels.importGraphLabel} / ${labels.exportGraphLabel}`} title={`${labels.importGraphLabel} / ${labels.exportGraphLabel}`}>···</summary>
        <div className="button-row button-row--adaptive">
          <button data-workflow-builder-action="run-catalog" onClick={onRunCatalog} type="button">{labels.runLabel}</button>
          <button data-workflow-builder-action="promote-draft" disabled={!canRunDraft} onClick={onPromoteDraft} type="button">{labels.promoteDraftLabel}</button>
          {selectedWorkflow.local ? (
            <>
              <button onClick={onDuplicateLocalWorkflow} type="button">{labels.duplicateLocalWorkflowLabel}</button>
              <button onClick={onRenameLocalWorkflow} type="button">{labels.renameLocalWorkflowLabel}</button>
              <button onClick={onDeleteLocalWorkflow} type="button">{labels.localWorkflowDeleteLabel}</button>
            </>
          ) : null}
          <button onClick={() => graphInputRef.current?.click()} type="button">{labels.importGraphLabel}</button>
          <button onClick={() => datasetInputRef.current?.click()} type="button">{labels.importDatasetContractLabel}</button>
          <button onClick={onExportGraph} type="button">{labels.exportGraphLabel}</button>
          <button disabled={!canExportDataset} onClick={onExportDataset} type="button">{labels.exportDatasetContractLabel}</button>
        </div>
      </details>
      <input accept="application/json,.json" hidden onChange={onGraphFileChange} ref={graphInputRef} type="file" />
      <input accept="application/json,.json" hidden onChange={onDatasetFileChange} ref={datasetInputRef} type="file" />
      <WorkbenchPanelNotice
        notice={importNotice}
        setNotice={setImportNotice}
        wrapperProps={{ "data-workflow-import-message": "text" }}
      />
    </section>
  );
}
