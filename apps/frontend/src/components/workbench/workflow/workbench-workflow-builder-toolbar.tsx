"use client";

import type { ChangeEvent, RefObject } from "react";
import type { WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowBuilderToolbarProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry;
  canRunDraft: boolean;
  canExportDataset: boolean;
  importMessage: string | null;
  graphInputRef: RefObject<HTMLInputElement | null>;
  datasetInputRef: RefObject<HTMLInputElement | null>;
  onRunCatalog: () => void;
  onRunDraft: () => void;
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
  importMessage,
  graphInputRef,
  datasetInputRef,
  onRunCatalog,
  onRunDraft,
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
  const promotedAt = selectedWorkflow.local?.promoted_at
    ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(
        new Date(selectedWorkflow.local.promoted_at),
      )
    : null;
  return (
    <>
      <div className="card-head">
        <h2>{selectedWorkflow.name}</h2>
        <span className="status-pill status-pill--good">{selectedWorkflow.version}</span>
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
        </div>
      ) : null}
      <div className="button-row">
        <button onClick={onRunCatalog} type="button">{labels.runLabel}</button>
        <button disabled={!canRunDraft} onClick={onRunDraft} type="button">{labels.runDraftLabel}</button>
        <button onClick={onSaveDraft} type="button">{labels.saveDraftLabel}</button>
        <button disabled={!canRunDraft} onClick={onPromoteDraft} type="button">{labels.promoteDraftLabel}</button>
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
      <input accept="application/json,.json" hidden onChange={onGraphFileChange} ref={graphInputRef} type="file" />
      <input accept="application/json,.json" hidden onChange={onDatasetFileChange} ref={datasetInputRef} type="file" />
      {importMessage ? <p className="card-copy">{importMessage}</p> : null}
    </>
  );
}
