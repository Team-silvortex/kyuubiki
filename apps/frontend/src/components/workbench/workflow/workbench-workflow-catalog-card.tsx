"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import { deriveWorkflowCatalogHighlights } from "@/components/workbench/workflow/workbench-workflow-catalog-highlights";
import { WorkbenchWorkflowBridgeStatusPill } from "@/components/workbench/workflow/workbench-workflow-bridge-status-pill";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

function formatPromotedAt(value?: string) {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function formatWorkflowTags(tags?: string[]) {
  const normalized = tags?.filter(Boolean) ?? [];
  return normalized.length > 0 ? normalized.join(", ") : null;
}

export function WorkbenchWorkflowCatalogCard(props: {
  bridgeRuntimeSummary?: string | null;
  bridgeRuntimeTone?: "good" | "watch" | "risk" | null;
  contractHealth: string | null;
  isSelected: boolean;
  labels: WorkflowSidebarLabels;
  onDelete?: () => void;
  onRun: () => void;
  onSelectForBuilder: () => void;
  workflow: WorkflowCatalogEntry;
}) {
  const { bridgeRuntimeSummary, bridgeRuntimeTone, contractHealth, isSelected, labels, onDelete, onRun, onSelectForBuilder, workflow } =
    props;
  const localWorkflowTags = formatWorkflowTags(workflow.local?.tags);
  const highlights = deriveWorkflowCatalogHighlights(workflow);

  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>{workflow.name}</h2>
        <span className={`status-pill status-pill--${isSelected ? "good" : "watch"}`}>
          {workflow.local ? labels.localWorkflowBadgeLabel : workflow.version}
        </span>
      </div>
      <p className="card-copy">{workflow.summary}</p>
      {highlights.length > 0 ? (
        <div className="sidebar-list">
          {highlights.map((entry) => (
            <div className="sidebar-list__row" key={`${workflow.id}:${entry.label}`}>
              <span>{entry.label}</span>
              <strong>{entry.value}</strong>
            </div>
          ))}
        </div>
      ) : null}
      {workflow.local ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.localWorkflowSourceLabel}</span>
            <strong>{workflow.local.source_workflow_name ?? workflow.local.source_workflow_id ?? "--"}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{labels.localWorkflowPromotedAtLabel}</span>
            <strong>{formatPromotedAt(workflow.local.promoted_at)}</strong>
          </div>
          {workflow.local.variant_of_workflow_name || workflow.local.variant_of_workflow_id ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowVariantOfLabel}</span>
              <strong>{workflow.local.variant_of_workflow_name ?? workflow.local.variant_of_workflow_id}</strong>
            </div>
          ) : null}
          {workflow.local.imported_from_package_id ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowPackageIdLabel}</span>
              <strong>{workflow.local.imported_from_package_id}</strong>
            </div>
          ) : null}
          {workflow.local.imported_from_package_version ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowPackageVersionLabel}</span>
              <strong>{workflow.local.imported_from_package_version}</strong>
            </div>
          ) : null}
          {localWorkflowTags ? (
            <div className="sidebar-list__row">
              <span>{labels.localWorkflowTagsLabel}</span>
              <strong>{localWorkflowTags}</strong>
            </div>
          ) : null}
          {contractHealth ? (
            <div className="sidebar-list__row">
              <span>contract health</span>
              <strong>{contractHealth}</strong>
            </div>
          ) : null}
          {bridgeRuntimeSummary ? (
            <div className="sidebar-list__row">
              <span>bridge runtime</span>
              <strong>{bridgeRuntimeTone ? <WorkbenchWorkflowBridgeStatusPill mode="summary" summary={bridgeRuntimeSummary} tone={bridgeRuntimeTone} /> : bridgeRuntimeSummary}</strong>
            </div>
          ) : null}
        </div>
      ) : null}
      {workflow.local?.notes ? <p className="card-copy">{workflow.local.notes}</p> : null}
      <div className="button-row button-row--adaptive">
        <button onClick={onSelectForBuilder} type="button">
          {labels.selectForBuilderLabel}
        </button>
        <button onClick={onRun} type="button">
          {labels.runLabel}
        </button>
        {onDelete ? (
          <button onClick={onDelete} type="button">
            {labels.localWorkflowDeleteLabel}
          </button>
        ) : null}
      </div>
    </section>
  );
}
