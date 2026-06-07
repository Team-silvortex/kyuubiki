"use client";

import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { StoredWorkflowDraft } from "@/components/workbench/workflow/workbench-workflow-draft-storage";

type WorkbenchWorkflowDraftCardProps = {
  labels: WorkflowSidebarLabels;
  drafts: StoredWorkflowDraft[];
  onSaveDraft: () => void;
  onLoadDraft: (draftId: string) => void;
  onDeleteDraft: (draftId: string) => void;
};

function formatSavedAt(savedAt: string) {
  const date = new Date(savedAt);
  if (Number.isNaN(date.getTime())) return savedAt;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function WorkbenchWorkflowDraftCard({
  labels,
  drafts,
  onSaveDraft,
  onLoadDraft,
  onDeleteDraft,
}: WorkbenchWorkflowDraftCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.savedDraftsTitle}</h2>
        <span className="status-pill status-pill--watch">{labels.artifactDraftLocalLabel}</span>
      </div>
      <p className="card-copy">{labels.savedDraftsHint}</p>
      <div className="button-row">
        <button onClick={onSaveDraft} type="button">
          {labels.saveDraftLabel}
        </button>
      </div>
      {drafts.length === 0 ? <p className="card-copy">{labels.savedDraftsEmptyLabel}</p> : null}
      <div className="runtime-overview-grid">
        {drafts.map((draft) => (
          <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={draft.id}>
            <div className="card-head">
              <h2>{draft.name}</h2>
            </div>
            <div className="sidebar-list">
              <div className="sidebar-list__row">
                <span>{labels.latestSummaryLabel}</span>
                <strong>{formatSavedAt(draft.savedAt)}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.nodesTitle}</span>
                <strong>{draft.graph.nodes.length}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.edgesTitle}</span>
                <strong>{draft.graph.edges?.length ?? 0}</strong>
              </div>
            </div>
            <div className="button-row">
              <button onClick={() => onLoadDraft(draft.id)} type="button">
                {labels.loadDraftLabel}
              </button>
              <button onClick={() => onDeleteDraft(draft.id)} type="button">
                {labels.deleteDraftLabel}
              </button>
            </div>
          </section>
        ))}
      </div>
    </section>
  );
}
