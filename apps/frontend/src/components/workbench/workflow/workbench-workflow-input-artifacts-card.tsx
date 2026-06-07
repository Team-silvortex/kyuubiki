"use client";

import type { WorkflowCatalogEntryArtifact } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowInputArtifactsCardProps = {
  labels: WorkflowSidebarLabels;
  entryInputs: WorkflowCatalogEntryArtifact[];
  inputTexts: Record<string, string>;
  invalidKeys: string[];
  onChangeInputText: (nodeId: string, value: string) => void;
};

export function WorkbenchWorkflowInputArtifactsCard({
  labels,
  entryInputs,
  inputTexts,
  invalidKeys,
  onChangeInputText,
}: WorkbenchWorkflowInputArtifactsCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.inputArtifactsTitle}</h2>
        <span className="status-pill status-pill--watch">{labels.artifactDraftLocalLabel}</span>
      </div>
      <p className="card-copy">{labels.inputArtifactsHint}</p>
      {entryInputs.length === 0 ? <p className="card-copy">{labels.emptyCatalogLabel}</p> : null}
      <div className="runtime-overview-grid">
        {entryInputs.map((artifact) => {
          const isInvalid = invalidKeys.includes(artifact.node_id);
          return (
            <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={artifact.node_id}>
              <div className="card-head">
                <h2>{artifact.node_id}</h2>
                <span className={`status-pill status-pill--${isInvalid ? "risk" : "good"}`}>
                  {artifact.artifact_type}
                </span>
              </div>
              {artifact.description ? <p className="card-copy">{artifact.description}</p> : null}
              <textarea
                className="shell-textarea"
                onChange={(event) => onChangeInputText(artifact.node_id, event.target.value)}
                rows={8}
                value={inputTexts[artifact.node_id] ?? ""}
              />
              {isInvalid ? <p className="card-copy">{labels.runDraftInvalidInputsLabel}</p> : null}
            </section>
          );
        })}
      </div>
    </section>
  );
}
