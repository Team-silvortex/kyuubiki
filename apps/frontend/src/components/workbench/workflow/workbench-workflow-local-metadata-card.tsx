"use client";

import { useEffect, useState } from "react";
import type { WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowLocalMetadataCardProps = {
  labels: WorkflowSidebarLabels;
  workflow: WorkflowCatalogEntry;
  onSave: (summary: string, notes: string) => void;
};

export function WorkbenchWorkflowLocalMetadataCard({
  labels,
  workflow,
  onSave,
}: WorkbenchWorkflowLocalMetadataCardProps) {
  const [summary, setSummary] = useState(workflow.summary);
  const [notes, setNotes] = useState(workflow.local?.notes ?? "");

  useEffect(() => {
    setSummary(workflow.summary);
    setNotes(workflow.local?.notes ?? "");
  }, [workflow]);

  if (!workflow.local) return null;

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.localWorkflowMetadataTitle}</h2>
        <span className="status-pill status-pill--watch">{labels.localWorkflowBadgeLabel}</span>
      </div>
      <label className="field-label">
        <span>{labels.localWorkflowSummaryLabel}</span>
        <input onChange={(event) => setSummary(event.target.value)} type="text" value={summary} />
      </label>
      <label className="field-label">
        <span>{labels.localWorkflowNotesLabel}</span>
        <textarea onChange={(event) => setNotes(event.target.value)} rows={4} value={notes} />
      </label>
      <div className="button-row">
        <button onClick={() => onSave(summary, notes)} type="button">
          {labels.localWorkflowSaveMetadataLabel}
        </button>
      </div>
    </section>
  );
}
