"use client";

import { useMemo } from "react";
import type { WorkflowCatalogEntry } from "@/lib/api";
import { WorkbenchWorkflowBridgeRuntimeCard } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-card";
import { WorkbenchWorkflowSummaryArtifactCard } from "@/components/workbench/workflow/workbench-workflow-summary-artifact-card";
import { listWorkflowSummaryArtifacts } from "@/components/workbench/workflow/workbench-workflow-summary-contract";
import { WorkbenchWorkflowTraceDiagnosticsPanel } from "@/components/workbench/workflow/workbench-workflow-trace-diagnostics-panel";
import {
  resolveWorkflowTraceBranchPredicateTone,
  resolveWorkflowTraceLineageSourceLabel,
  resolveWorkflowTraceLineageSourceTone,
  resolveWorkflowTraceNodeRunTone,
  resolveWorkflowTraceProgressStageTone,
  type WorkflowTraceStatusTone,
} from "@/components/workbench/workflow/workbench-workflow-trace-status";
import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowRunTraceDeepPanelsProps = {
  run: WorkflowRunRecord;
  previousRun?: WorkflowRunRecord | null;
  workflow?: WorkflowCatalogEntry | null;
  onSelectBranch?: (nodeId: string, outputId: string) => void;
  onSelectLineage?: (entry: NonNullable<WorkflowRunRecord["artifactLineage"]>[number]) => void;
  onSelectNode?: (nodeId: string) => void;
};

function renderStatusPill(label: string, tone: WorkflowTraceStatusTone) {
  return <span className={`status-pill status-pill--${tone}`}>{label}</span>;
}

export function WorkbenchWorkflowRunTraceDeepPanels({
  run,
  previousRun,
  workflow,
  onSelectBranch,
  onSelectLineage,
  onSelectNode,
}: WorkbenchWorkflowRunTraceDeepPanelsProps) {
  const latestBranch = run.branchDecisions?.[run.branchDecisions.length - 1] ?? null;
  const latestSkipped = run.skippedNodes?.slice(0, 3) ?? [];
  const recentProgressEvents = run.traceSummary?.recentProgressEvents ?? [];
  const recentNodes = useMemo(() => run.nodeRuns?.slice(-3).reverse() ?? [], [run.nodeRuns]);
  const recentLineage = useMemo(() => run.artifactLineage?.slice(-3).reverse() ?? [], [run.artifactLineage]);
  const lineageByArtifactKey = useMemo(
    () => new Map((run.artifactLineage ?? []).map((entry) => [entry.artifact_key, entry] as const)),
    [run.artifactLineage],
  );
  const recentSummaryArtifacts = useMemo(
    () => listWorkflowSummaryArtifacts(run.result ?? null).slice(0, 3),
    [run.result],
  );
  const previousSummaryArtifactsByKind = useMemo(
    () => new Map(
      listWorkflowSummaryArtifacts(previousRun?.result ?? null).map((artifact) => [
        artifact.payload.summary_kind ?? artifact.artifactType,
        artifact,
      ] as const),
    ),
    [previousRun?.result],
  );

  return (
    <>
      <div className="workflow-trace-panel-section">
        <details className="workflow-trace-panel-section__details">
          <summary className="card-copy" style={{ cursor: recentProgressEvents.length > 0 ? "pointer" : "default" }}>
            {`recent phase timeline (${recentProgressEvents.length})`}
          </summary>
          <div className="workflow-trace-panel-stack">
            {recentProgressEvents.length === 0 ? <span className="card-copy">--</span> : null}
            {recentProgressEvents.map((event, index) => (
              <div className="workflow-trace-panel-card" key={`${run.jobId}:progress:${index}:${event.stage}`}>
                <span className="workflow-trace-panel-card__meta">
                  {renderStatusPill(event.stage, resolveWorkflowTraceProgressStageTone(event.stage))}
                  <span>{Math.round(event.progress * 100)}%</span>
                  {event.kind ? <span>{event.kind}</span> : null}
                </span>
                <span className="card-copy">{event.label ?? "--"}</span>
                <span className="card-copy">{event.nodeId ? `node ${event.nodeId}` : event.emittedAt ?? "workflow event"}</span>
              </div>
            ))}
          </div>
        </details>
      </div>
      <div className="workflow-trace-panel-section">
        <WorkbenchWorkflowBridgeRuntimeCard
          graph={workflow?.graph ?? null}
          onLocateIssue={(issue) => onSelectNode?.(issue.nodeId)}
          result={run.result ?? null}
        />
      </div>
      <div className="workflow-trace-panel-section">
        <WorkbenchWorkflowTraceDiagnosticsPanel result={run.result ?? null} />
      </div>
      <div className="workflow-trace-panel-section">
        <div className="workflow-trace-panel-section__head">
          <p className="card-copy">control flow</p>
          <span className="status-pill status-pill--watch">{`${run.branchDecisions?.length ?? 0} branches · ${run.skippedNodes?.length ?? 0} skipped`}</span>
        </div>
        <div className="workflow-trace-panel-grid workflow-trace-panel-grid--control">
          <div className="workflow-trace-panel-card">
            <span className="card-copy">latest branch</span>
            {latestBranch ? (
              <button className="workflow-trace-panel-card__button" onClick={() => onSelectBranch?.(latestBranch.node_id, latestBranch.chosen_output)} style={{ cursor: onSelectBranch ? "pointer" : "default" }} type="button">
                <span className="workflow-trace-panel-card__meta">
                  {renderStatusPill(latestBranch.predicate_result ? "true" : "false", resolveWorkflowTraceBranchPredicateTone(latestBranch.predicate_result))}
                  <span>{`${latestBranch.node_id} -> ${latestBranch.chosen_output}`}</span>
                </span>
              </button>
            ) : <span className="card-copy">--</span>}
          </div>
          <div className="workflow-trace-panel-card">
            <span className="card-copy">skipped nodes</span>
            {(latestSkipped?.length ?? 0) > 0 ? (
              <div className="workflow-trace-panel-card__meta">
                {latestSkipped.map((value) => <span className="status-pill status-pill--watch" key={value}>{value}</span>)}
              </div>
            ) : <span className="card-copy">--</span>}
          </div>
        </div>
      </div>
      <div className="workflow-trace-panel-section">
        <div className="workflow-trace-panel-section__head">
          <p className="card-copy">summary artifacts</p>
          <span className="status-pill status-pill--watch">{recentSummaryArtifacts.length}</span>
        </div>
        {recentSummaryArtifacts.length === 0 ? <span className="card-copy">--</span> : null}
        <div className="workflow-trace-panel-stack">
          {recentSummaryArtifacts.map((artifact) => {
            const lineageEntry = lineageByArtifactKey.get(artifact.artifactKey);
            const compareKey = artifact.payload.summary_kind ?? artifact.artifactType;
            return (
              <WorkbenchWorkflowSummaryArtifactCard
                artifact={artifact}
                key={`${run.jobId}:${artifact.artifactKey}`}
                lineageEntry={lineageEntry}
                onSelectLineage={onSelectLineage}
                onSelectNode={onSelectNode}
                previousArtifact={previousSummaryArtifactsByKind.get(compareKey)}
                previousRunJobId={previousRun?.jobId}
                runJobId={run.jobId}
              />
            );
          })}
        </div>
      </div>
      <div className="workflow-trace-panel-section">
        <div className="workflow-trace-panel-section__head">
          <p className="card-copy">trace lanes</p>
          <span className="status-pill status-pill--watch">{`${recentLineage.length} lineage · ${recentNodes.length} node events`}</span>
        </div>
        <div className="workflow-trace-lanes">
          <div className="workflow-trace-panel-section">
            <div className="workflow-trace-panel-section__head">
              <p className="card-copy">recent artifact lineage</p>
              <span className="status-pill status-pill--watch">{recentLineage.length}</span>
            </div>
            {recentLineage.length === 0 ? <span className="card-copy">--</span> : null}
            <div className="workflow-trace-panel-stack">
              {recentLineage.map((entry) => (
                <div className="workflow-trace-panel-card" key={`${run.jobId}:${entry.artifact_key}`}>
                  <button className="workflow-trace-panel-card__button" onClick={() => onSelectLineage?.(entry)} style={{ cursor: onSelectLineage ? "pointer" : "default" }} type="button">
                    <strong>{entry.artifact_key}</strong>
                  </button>
                  <button className="workflow-trace-panel-card__button" onClick={() => onSelectNode?.(entry.node_id)} style={{ cursor: onSelectNode ? "pointer" : "default" }} type="button">
                    <span className="card-copy">{entry.node_id}.{entry.port_id}</span>
                  </button>
                  <span className="workflow-trace-panel-card__meta">
                    {renderStatusPill(resolveWorkflowTraceLineageSourceLabel(entry.source_artifacts), resolveWorkflowTraceLineageSourceTone(entry.source_artifacts))}
                    <span>{(entry.source_artifacts?.length ?? 0) > 0 ? `from ${entry.source_artifacts?.slice(0, 2).join(", ")}` : "source input / root artifact"}</span>
                  </span>
                </div>
              ))}
            </div>
          </div>
          <div className="workflow-trace-panel-section">
            <div className="workflow-trace-panel-section__head">
              <p className="card-copy">recent node activity</p>
              <span className="status-pill status-pill--watch">{recentNodes.length}</span>
            </div>
            {recentNodes.length === 0 ? <span className="card-copy">--</span> : null}
            <div className="workflow-trace-panel-stack">
              {recentNodes.map((entry) => (
                <div className="workflow-trace-panel-card" key={`${run.jobId}:${entry.node_id}:${entry.status}`}>
                  <button className="workflow-trace-panel-card__button" onClick={() => onSelectNode?.(entry.node_id)} style={{ cursor: onSelectNode ? "pointer" : "default" }} type="button">
                    <strong>{entry.node_id}</strong>
                  </button>
                  <span className="workflow-trace-panel-card__meta">
                    {renderStatusPill(entry.status, resolveWorkflowTraceNodeRunTone(entry.status))}
                    <span>{entry.kind}{entry.operator_id ? ` · ${entry.operator_id}` : ""}</span>
                  </span>
                  <span className="card-copy">in {entry.consumed_artifacts?.length ?? 0} / out {entry.produced_artifacts?.length ?? 0}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
