"use client";

import { memo, useState } from "react";
import type { WorkflowResolvedSummaryArtifact } from "@/components/workbench/workflow/workbench-workflow-summary-contract";
import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkflowLineageEntry = NonNullable<WorkflowRunRecord["artifactLineage"]>[number];

type WorkbenchWorkflowSummaryArtifactCardProps = {
  artifact: WorkflowResolvedSummaryArtifact;
  lineageEntry?: WorkflowLineageEntry;
  previousArtifact?: WorkflowResolvedSummaryArtifact;
  previousRunJobId?: string;
  runJobId: string;
  onSelectLineage?: (entry: WorkflowLineageEntry) => void;
  onSelectNode?: (nodeId: string) => void;
};

function renderStatusPill(label: string) {
  return <span className="status-pill status-pill--watch">{label}</span>;
}

function formatSummaryValue(value: string | number | boolean | null) {
  return typeof value === "number" ? value.toExponential(4) : String(value);
}

function renderFieldPreview(artifact: WorkflowResolvedSummaryArtifact) {
  return Object.entries(artifact.payload.fields)
    .slice(0, 2)
    .map(([key, value]) => `${key}=${typeof value === "number" ? value.toExponential(2) : String(value)}`)
    .join(", ") || "--";
}

function SummaryArtifactDetails({
  artifact,
  previousArtifact,
}: Pick<WorkbenchWorkflowSummaryArtifactCardProps, "artifact" | "previousArtifact">) {
  return (
    <div className="workflow-trace-panel-card__details">
      <span className="card-copy">contract {artifact.payload.contract_version}</span>
      <span className="card-copy">namespace {artifact.payload.field_namespace ?? "--"}</span>
      <div className="workflow-trace-panel-card__details-grid">
        {Object.entries(artifact.payload.fields).map(([key, value]) => (
          <span className="card-copy" key={`${artifact.artifactKey}:field:${key}`}>
            {`${key}: ${formatSummaryValue(value)}`}
          </span>
        ))}
      </div>
      <div className="workflow-trace-panel-card__details-grid">
        {previousArtifact ? (
          Object.entries(artifact.payload.fields).map(([key, value]) => {
            const previousValue = previousArtifact.payload.fields[key];
            if (typeof value === "number" && typeof previousValue === "number") {
              const delta = value - previousValue;
              const ratio = previousValue !== 0 ? value / previousValue : null;
              return (
                <span className="card-copy" key={`${artifact.artifactKey}:delta:${key}`}>
                  {`${key} delta: ${delta.toExponential(4)}${ratio !== null ? ` (${ratio.toFixed(3)}x)` : ""}`}
                </span>
              );
            }
            return (
              <span className="card-copy" key={`${artifact.artifactKey}:delta:${key}`}>
                {`${key} previous: ${previousValue === undefined ? "--" : formatSummaryValue(previousValue)}`}
              </span>
            );
          })
        ) : (
          <span className="card-copy">previous run comparison --</span>
        )}
      </div>
      <div className="workflow-trace-panel-card__details-grid">
        {Object.entries(artifact.payload.metadata ?? {}).length === 0 ? (
          <span className="card-copy">metadata --</span>
        ) : (
          Object.entries(artifact.payload.metadata ?? {}).map(([key, value]) => (
            <span className="card-copy" key={`${artifact.artifactKey}:meta:${key}`}>
              {`${key}: ${String(value)}`}
            </span>
          ))
        )}
      </div>
    </div>
  );
}

export const WorkbenchWorkflowSummaryArtifactCard = memo(function WorkbenchWorkflowSummaryArtifactCard({
  artifact,
  lineageEntry,
  previousArtifact,
  previousRunJobId,
  runJobId,
  onSelectLineage,
  onSelectNode,
}: WorkbenchWorkflowSummaryArtifactCardProps) {
  const [showDetails, setShowDetails] = useState(false);
  return (
    <div className="workflow-trace-panel-card workflow-trace-panel-card--summary" key={`${runJobId}:${artifact.artifactKey}`}>
      <button className="workflow-trace-panel-card__button" onClick={() => lineageEntry ? onSelectLineage?.(lineageEntry) : onSelectNode?.(artifact.nodeId ?? "")} style={{ cursor: onSelectLineage || onSelectNode ? "pointer" : "default" }} type="button">
        <strong>{artifact.payload.summary_kind ?? artifact.artifactType}</strong>
      </button>
      <button className="workflow-trace-panel-card__button" onClick={() => lineageEntry ? onSelectLineage?.(lineageEntry) : undefined} style={{ cursor: onSelectLineage && lineageEntry ? "pointer" : "default" }} type="button">
        <span className="card-copy">{artifact.artifactKey}</span>
      </button>
      <span className="workflow-trace-panel-card__meta">
        {renderStatusPill(String(Object.keys(artifact.payload.fields).length))}
        <button className="workflow-trace-panel-card__button" onClick={() => artifact.nodeId ? onSelectNode?.(artifact.nodeId) : undefined} style={{ cursor: onSelectNode && artifact.nodeId ? "pointer" : "default" }} type="button">
          <span>{artifact.payload.source_operator_id ?? "unknown operator"}</span>
        </button>
      </span>
      <span className="card-copy">{renderFieldPreview(artifact)}</span>
      <span className="card-copy">
        {previousArtifact ? `vs previous ${previousRunJobId ?? "--"}` : "no previous summary match"}
      </span>
      <details onToggle={(event) => setShowDetails(event.currentTarget.open)}>
        <summary className="card-copy" style={{ cursor: "pointer" }}>
          contract details
        </summary>
        {showDetails ? (
          <SummaryArtifactDetails artifact={artifact} previousArtifact={previousArtifact} />
        ) : null}
      </details>
    </div>
  );
});
