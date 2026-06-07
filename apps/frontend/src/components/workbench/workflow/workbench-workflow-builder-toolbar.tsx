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
  onExportGraph,
  onExportDataset,
  onGraphFileChange,
  onDatasetFileChange,
}: WorkbenchWorkflowBuilderToolbarProps) {
  return (
    <>
      <div className="card-head">
        <h2>{selectedWorkflow.name}</h2>
        <span className="status-pill status-pill--good">{selectedWorkflow.version}</span>
      </div>
      <p className="card-copy">{selectedWorkflow.summary}</p>
      <div className="button-row">
        <button onClick={onRunCatalog} type="button">{labels.runLabel}</button>
        <button disabled={!canRunDraft} onClick={onRunDraft} type="button">{labels.runDraftLabel}</button>
        <button onClick={onSaveDraft} type="button">{labels.saveDraftLabel}</button>
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
