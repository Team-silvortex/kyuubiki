"use client";

import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowValidationCardProps = {
  labels: WorkflowSidebarLabels;
  validationIssues: WorkflowGraphValidationIssue[];
  recentFixSummary?: string[];
  onApplyValidationFix: (issueId: string) => void;
  onApplyAllValidationFixes: () => void;
  onLocateValidationIssue: (issueId: string) => void;
};

export function WorkbenchWorkflowValidationCard({
  labels,
  validationIssues,
  recentFixSummary = [],
  onApplyValidationFix,
  onApplyAllValidationFixes,
  onLocateValidationIssue,
}: WorkbenchWorkflowValidationCardProps) {
  const fixableIssues = validationIssues.filter((issue) => issue.fix);
  const fixableIssueCount = fixableIssues.length;
  const previewIssues = fixableIssues.slice(0, 3);
  const remainingPreviewCount = fixableIssueCount - previewIssues.length;
  const recentSummaryPreview = recentFixSummary.slice(0, 4);
  const recentSummaryMoreCount = recentFixSummary.length - recentSummaryPreview.length;
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.validationTitle}</h2>
        <span className={`status-pill status-pill--${validationIssues.length > 0 ? "watch" : "good"}`}>
          {validationIssues.length}
        </span>
      </div>
      {recentSummaryPreview.length > 0 ? (
        <div className="sidebar-stack">
          <p className="card-copy">{labels.validationLatestFixSummaryLabel}</p>
          <div className="sidebar-list">
            {recentSummaryPreview.map((item) => (
              <div className="sidebar-list__row" key={`recent:${item}`}>
                <strong>{item}</strong>
              </div>
            ))}
          </div>
          {recentSummaryMoreCount > 0 ? (
            <p className="card-copy">{labels.validationLatestFixSummaryMoreLabel.replace("{count}", String(recentSummaryMoreCount))}</p>
          ) : null}
        </div>
      ) : null}
      {validationIssues.length > 0 ? (
        <div className="sidebar-stack">
          {fixableIssueCount > 1 ? (
            <div className="sidebar-stack">
              <p className="card-copy">{labels.validationFixPreviewLabel.replace("{count}", String(fixableIssueCount))}</p>
              <div className="sidebar-list">
                {previewIssues.map((issue) => (
                  <div className="sidebar-list__row" key={`preview:${issue.id}`}>
                    <strong>{issue.message}</strong>
                  </div>
                ))}
              </div>
              {remainingPreviewCount > 0 ? (
                <p className="card-copy">{labels.validationFixPreviewMoreLabel.replace("{count}", String(remainingPreviewCount))}</p>
              ) : null}
              <div className="button-row">
                <button onClick={onApplyAllValidationFixes} type="button">
                  {labels.validationFixAllLabel}
                </button>
              </div>
            </div>
          ) : null}
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
        </div>
      ) : (
        <p className="card-copy">{labels.validationOkLabel}</p>
      )}
    </section>
  );
}
