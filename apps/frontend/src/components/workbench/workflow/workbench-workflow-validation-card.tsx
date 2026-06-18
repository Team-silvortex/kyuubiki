"use client";

import { useMemo, useState } from "react";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import {
  publishWorkflowPolicyActionFeedback,
  useWorkflowPolicyAction,
} from "@/components/workbench/workflow/workbench-workflow-policy-actions";
import type { WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowValidationCardProps = {
  activeFilter?: "all" | "fixable" | "review";
  labels: WorkflowSidebarLabels;
  validationIssues: WorkflowGraphValidationIssue[];
  recentFixSummary?: WorkflowValidationFixSummaryEntry[];
  onApplyValidationFix: (issueId: string) => void;
  onApplyAllValidationFixes: () => void;
  onLocateValidationIssue: (issueId: string) => void;
};

export function WorkbenchWorkflowValidationCard({
  activeFilter = "all",
  labels,
  validationIssues,
  recentFixSummary = [],
  onApplyValidationFix,
  onApplyAllValidationFixes,
  onLocateValidationIssue,
}: WorkbenchWorkflowValidationCardProps) {
  const [previewOpen, setPreviewOpen] = useState(false);
  const filteredIssues =
    activeFilter === "fixable"
      ? validationIssues.filter((issue) => issue.fix)
      : activeFilter === "review"
        ? validationIssues.filter((issue) => !issue.fix)
        : validationIssues;
  const fixableIssues = filteredIssues.filter((issue) => issue.fix);
  const fixableIssueCount = fixableIssues.length;
  const previewIssues = fixableIssues.slice(0, 3);
  const remainingPreviewCount = fixableIssueCount - previewIssues.length;
  const recentSummaryPreview = recentFixSummary.slice(0, 4);
  const recentSummaryMoreCount = recentFixSummary.length - recentSummaryPreview.length;
  const previewDetail = useMemo(
    () =>
      fixableIssueCount > 0
        ? labels.validationFixPreviewLabel.replace("{count}", String(fixableIssueCount))
        : labels.validationOkLabel,
    [fixableIssueCount, labels.validationFixPreviewLabel, labels.validationOkLabel],
  );
  useWorkflowPolicyAction((action) => {
    if (action !== "preview-validation-fixes") return;
    setPreviewOpen(fixableIssueCount > 0);
    publishWorkflowPolicyActionFeedback(
      "preview-validation-fixes",
      fixableIssueCount > 0 ? "ready" : "complete",
      previewDetail,
    );
  });
  return (
    <section className="sidebar-card sidebar-card--compact" data-workflow-validation-card="card">
      <div className="card-head">
        <h2>{labels.validationTitle}</h2>
        <span className={`status-pill status-pill--${filteredIssues.length > 0 ? "watch" : "good"}`}>
          {filteredIssues.length}
        </span>
      </div>
      {recentSummaryPreview.length > 0 ? (
        <div className="sidebar-stack">
          <p className="card-copy">{labels.validationLatestFixSummaryLabel}</p>
          <div className="sidebar-list">
            {recentSummaryPreview.map((item) => (
              <div className="sidebar-list__row" key={`recent:${item.id}`}>
                <strong>{item.title}</strong>
                <span>{item.detail}</span>
              </div>
            ))}
          </div>
          {recentSummaryMoreCount > 0 ? (
            <p className="card-copy">{labels.validationLatestFixSummaryMoreLabel.replace("{count}", String(recentSummaryMoreCount))}</p>
          ) : null}
        </div>
      ) : null}
      {filteredIssues.length > 0 ? (
        <div className="sidebar-stack">
          {previewOpen && fixableIssueCount > 0 ? (
            <div className="sidebar-card sidebar-card--compact workflow-preview-panel">
              <div className="card-head">
                <h2>{labels.validationPreviewLabel}</h2>
                <span className="status-pill status-pill--watch">{fixableIssueCount}</span>
              </div>
              <p className="card-copy">{labels.validationFixPreviewLabel.replace("{count}", String(fixableIssueCount))}</p>
              <div className="sidebar-list">
                {previewIssues.map((issue) => (
                  <div className="sidebar-list__row" key={`preview:${issue.id}`}>
                    <strong>{issue.message}</strong>
                    {issue.fix?.kind === "sync_node_template_from_operator" ? (
                      <span>{`会按 ${issue.fix.operatorId} 的节点模板重建端口并尽量保留现有配置。`}</span>
                    ) : null}
                  </div>
                ))}
              </div>
              {remainingPreviewCount > 0 ? (
                <p className="card-copy">{labels.validationFixPreviewMoreLabel.replace("{count}", String(remainingPreviewCount))}</p>
              ) : null}
              <div className="button-row button-row--adaptive">
                <button onClick={() => { onApplyAllValidationFixes(); setPreviewOpen(false); }} type="button">{labels.validationFixAllLabel}</button>
                <button onClick={() => setPreviewOpen(false)} type="button">{labels.packageInstallRulesPreviewCancelLabel}</button>
              </div>
            </div>
          ) : null}
          <div className="sidebar-list">
            {filteredIssues.map((issue) => (
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
          {fixableIssueCount > 1 ? (
            <div className="button-row">
              <button onClick={() => setPreviewOpen(true)} type="button">{labels.validationPreviewLabel}</button>
            </div>
          ) : null}
        </div>
      ) : (
        <p className="card-copy">{labels.validationOkLabel}</p>
      )}
    </section>
  );
}
