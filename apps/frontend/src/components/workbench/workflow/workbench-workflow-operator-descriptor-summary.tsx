"use client";

import type { WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkflowOperatorOptionPreset = {
  id: string;
  label: string;
  operatorId?: string;
};

function formatOperatorSchemaRef(schemaRef?: { schema: string; version: string } | null) {
  if (!schemaRef?.schema) return "--";
  return `${schemaRef.schema}@${schemaRef.version}`;
}

export function formatOperatorValidationStatus(
  labels: WorkflowSidebarLabels,
  status?: WorkflowOperatorDescriptor["validation"]["baseline_status"],
) {
  if (status === "verified") return labels.operatorValidationVerifiedLabel;
  if (status === "partial") return labels.operatorValidationPartialLabel;
  if (status === "unverified") return labels.operatorValidationUnverifiedLabel;
  return "--";
}

function getOperatorValidationSortRank(
  status?: WorkflowOperatorDescriptor["validation"]["baseline_status"],
) {
  if (status === "verified") return 0;
  if (status === "partial") return 1;
  if (status === "unverified") return 2;
  return 3;
}

export function buildOperatorOptionLabel(
  labels: WorkflowSidebarLabels,
  presetLabel: string,
  descriptor?: WorkflowOperatorDescriptor,
) {
  if (!descriptor) return presetLabel;
  const statusLabel = formatOperatorValidationStatus(labels, descriptor.validation?.baseline_status);
  return statusLabel === "--" ? presetLabel : `${presetLabel} [${statusLabel}]`;
}

export function sortWorkflowOperatorOptionPresets(
  presets: WorkflowOperatorOptionPreset[],
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>,
) {
  return [...presets].sort((left, right) => {
    const leftDescriptor = left.operatorId ? operatorDescriptorMap.get(left.operatorId) : undefined;
    const rightDescriptor = right.operatorId
      ? operatorDescriptorMap.get(right.operatorId)
      : undefined;
    const rankDiff =
      getOperatorValidationSortRank(leftDescriptor?.validation?.baseline_status) -
      getOperatorValidationSortRank(rightDescriptor?.validation?.baseline_status);
    if (rankDiff !== 0) return rankDiff;
    return left.label.localeCompare(right.label);
  });
}

export function WorkbenchWorkflowOperatorDescriptorSummary(props: {
  labels: WorkflowSidebarLabels;
  descriptor?: WorkflowOperatorDescriptor;
}) {
  const { labels, descriptor } = props;
  if (!descriptor) return null;

  return (
    <div className="sidebar-list">
      <div className="sidebar-list__row">
        <span>{labels.operatorValidationLabel}</span>
        <strong>
          {formatOperatorValidationStatus(labels, descriptor.validation?.baseline_status)}
        </strong>
      </div>
      <div className="sidebar-list__row">
        <span>{labels.operatorInputSchemaLabel}</span>
        <strong>{formatOperatorSchemaRef(descriptor.input_schema)}</strong>
      </div>
      <div className="sidebar-list__row">
        <span>{labels.operatorOutputSchemaLabel}</span>
        <strong>{formatOperatorSchemaRef(descriptor.output_schema)}</strong>
      </div>
      {descriptor.capability_tags.length > 0 ? (
        <div className="sidebar-list__row">
          <span>{labels.operatorCapabilitiesLabel}</span>
          <strong>{descriptor.capability_tags.join(", ")}</strong>
        </div>
      ) : null}
    </div>
  );
}
