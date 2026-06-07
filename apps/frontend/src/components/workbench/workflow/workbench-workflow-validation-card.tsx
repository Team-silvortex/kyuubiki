"use client";

import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowValidationCardProps = {
  labels: WorkflowSidebarLabels;
  validationIssues: WorkflowGraphValidationIssue[];
  onApplyValidationFix: (issueId: string) => void;
  onLocateValidationIssue: (issueId: string) => void;
};

export function WorkbenchWorkflowValidationCard({
  labels,
  validationIssues,
  onApplyValidationFix,
  onLocateValidationIssue,
}: WorkbenchWorkflowValidationCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.validationTitle}</h2>
        <span
          className={`status-pill status-pill--${validationIssues.length > 0 ? "watch" : "good"}`}
        >
          {validationIssues.length}
        </span>
      </div>
      {validationIssues.length > 0 ? (
        <div className="sidebar-list">
          {validationIssues.map((issue) => (
            <div className="sidebar-list__row" key={issue.id}>
              <span>{issue.level}</span>
              <strong>{issue.message}</strong>
              {issue.locate ? (
                <button onClick={() => onLocateValidationIssue(issue.id)} type="button">
                  {labels.validationLocateLabel}
                </button>
              ) : null}
              {issue.fix ? (
                <button onClick={() => onApplyValidationFix(issue.id)} type="button">
                  {labels.validationFixLabel}
                </button>
              ) : null}
            </div>
          ))}
        </div>
      ) : (
        <p className="card-copy">{labels.validationOkLabel}</p>
      )}
    </section>
  );
}
