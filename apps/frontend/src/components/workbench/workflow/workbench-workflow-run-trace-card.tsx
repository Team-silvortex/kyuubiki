"use client";

import type { WorkflowRunRecord, WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowRunTraceCardProps = {
  labels: WorkflowSidebarLabels;
  run: WorkflowRunRecord;
  onSelectNode?: (nodeId: string) => void;
};

function renderInlineList(values: string[] | undefined, empty = "--") {
  if (!values || values.length === 0) return <span className="card-copy">{empty}</span>;
  return (
    <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
      {values.slice(0, 4).map((value) => (
        <span className="status-pill status-pill--watch" key={value}>{value}</span>
      ))}
    </div>
  );
}

export function WorkbenchWorkflowRunTraceCard({
  labels,
  run,
  onSelectNode,
}: WorkbenchWorkflowRunTraceCardProps) {
  const latestBranch = run.branchDecisions?.[run.branchDecisions.length - 1] ?? null;
  const latestSkipped = run.skippedNodes?.slice(0, 3) ?? [];
  const recentNodes = run.nodeRuns?.slice(-3).reverse() ?? [];
  const recentLineage = run.artifactLineage?.slice(-3).reverse() ?? [];
  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>{run.workflowId}</h2>
        <span className="status-pill status-pill--watch">trace</span>
      </div>
      <div className="sidebar-list">
        <div className="sidebar-list__row"><span>{labels.progressLabel}</span><strong>{Math.round(run.progress * 100)}%</strong></div>
        <div className="sidebar-list__row"><span>{labels.currentNodeLabel}</span><strong>{run.currentNode ?? "--"}</strong></div>
        <div className="sidebar-list__row"><span>skipped</span><strong>{run.skippedNodes?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>node runs</span><strong>{run.nodeRuns?.length ?? 0}</strong></div>
      </div>
      <div style={{ display: "grid", gap: "0.55rem", marginTop: "0.75rem" }}>
        <div>
          <p className="card-copy">latest branch</p>
          <p className="card-copy">
            {latestBranch
              ? `${latestBranch.node_id} -> ${latestBranch.chosen_output} (${latestBranch.predicate_result ? "true" : "false"})`
              : "--"}
          </p>
        </div>
        <div>
          <p className="card-copy">skipped nodes</p>
          {renderInlineList(latestSkipped)}
        </div>
        <div style={{ display: "grid", gap: "0.35rem" }}>
          <p className="card-copy">recent artifact lineage</p>
          {recentLineage.length === 0 ? <span className="card-copy">--</span> : null}
          {recentLineage.map((entry) => (
            <div key={`${run.jobId}:${entry.artifact_key}`} style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
              <strong style={{ fontSize: "0.92rem" }}>{entry.artifact_key}</strong>
              <button onClick={() => onSelectNode?.(entry.node_id)} style={{ all: "unset", cursor: onSelectNode ? "pointer" : "default" }} type="button">
                <span className="card-copy">{entry.node_id}.{entry.port_id}</span>
              </button>
              <span className="card-copy">{(entry.source_artifacts?.length ?? 0) > 0 ? `from ${entry.source_artifacts?.slice(0, 2).join(", ")}` : "source input / root artifact"}</span>
            </div>
          ))}
        </div>
        <div style={{ display: "grid", gap: "0.35rem" }}>
          <p className="card-copy">recent node activity</p>
          {recentNodes.length === 0 ? <span className="card-copy">--</span> : null}
          {recentNodes.map((entry) => (
            <div key={`${run.jobId}:${entry.node_id}:${entry.status}`} style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
              <button onClick={() => onSelectNode?.(entry.node_id)} style={{ all: "unset", cursor: onSelectNode ? "pointer" : "default", justifySelf: "start" }} type="button">
                <strong style={{ fontSize: "0.92rem" }}>{entry.node_id}</strong>
              </button>
              <span className="card-copy">{entry.status} · {entry.kind}{entry.operator_id ? ` · ${entry.operator_id}` : ""}</span>
              <span className="card-copy">in {entry.consumed_artifacts?.length ?? 0} / out {entry.produced_artifacts?.length ?? 0}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
