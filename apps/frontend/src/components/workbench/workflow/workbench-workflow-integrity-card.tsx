"use client";

import type {
  WorkflowIntegrityIssue,
  WorkflowIntegrityReport,
} from "@/components/workbench/workflow/workbench-workflow-integrity";

type WorkbenchWorkflowIntegrityCardProps = {
  report: WorkflowIntegrityReport;
  onLocateIssue?: (issue: WorkflowIntegrityIssue) => void;
};

export function WorkbenchWorkflowIntegrityCard({
  report,
  onLocateIssue,
}: WorkbenchWorkflowIntegrityCardProps) {
  const previewIssues = report.issues.slice(0, 4);
  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card" data-workflow-integrity-card="card">
      <div className="card-head">
        <h2>Component integrity</h2>
        <span className={`status-pill status-pill--${report.issues.length === 0 ? "good" : "watch"}`}>
          {report.issues.length}
        </span>
      </div>
      <div className="sidebar-list">
        <div className="sidebar-list__row"><span>local storage linked</span><strong>{report.localWorkflowFound ? "yes" : "no"}</strong></div>
        <div className="sidebar-list__row"><span>snapshots indexed</span><strong>{report.snapshotCount}</strong></div>
        <div className="sidebar-list__row"><span>summary-only snapshots</span><strong>{report.summaryOnlySnapshotCount}</strong></div>
      </div>
      {previewIssues.length > 0 ? (
        <div style={{ display: "grid", gap: "0.35rem", marginTop: "0.75rem" }}>
          {previewIssues.map((issue) => (
            <div key={issue.id} style={{ display: "grid", gap: "0.15rem" }}>
              <strong style={{ fontSize: "0.9rem" }}>{issue.scope}</strong>
              <span className="card-copy">{issue.message}</span>
              {issue.detail ? <span className="card-copy">{issue.detail}</span> : null}
              {issue.locate ? <div className="button-row"><button onClick={() => onLocateIssue?.(issue)} type="button">locate</button></div> : null}
            </div>
          ))}
        </div>
      ) : (
        <p className="card-copy" style={{ marginTop: "0.75rem" }}>
          Workflow storage, graph contract, dataset bindings, and snapshot chain look consistent.
        </p>
      )}
    </section>
  );
}
