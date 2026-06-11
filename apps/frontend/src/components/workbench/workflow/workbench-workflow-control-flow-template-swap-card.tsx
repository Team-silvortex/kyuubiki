"use client";

import { useMemo } from "react";
import type { WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";
import { listWorkflowNodeTemplatePresets, type WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { isWorkflowDescriptorSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowControlFlowTemplateSwapCardProps = {
  labels: WorkflowSidebarLabels;
  node: WorkflowGraphNode;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  onSyncNodeTemplate: (nodeId: string, template?: WorkflowNodeTemplateSelection) => void;
};

const PRIORITY_KINDS = new Set(["condition", "transform", "extract", "export", "input"]);

function buildTemplateValue(template: WorkflowNodeTemplateSelection) {
  return template.operatorId ? `operator:${template.operatorId}` : `kind:${template.kind ?? ""}`;
}

function parseTemplateValue(value: string): WorkflowNodeTemplateSelection | undefined {
  if (!value) return undefined;
  if (value.startsWith("operator:")) return { operatorId: value.slice(9), kind: "transform" };
  if (value.startsWith("kind:")) return { kind: value.slice(5) };
  return undefined;
}

export function WorkbenchWorkflowControlFlowTemplateSwapCard({
  labels,
  node,
  operatorDescriptors,
  onSyncNodeTemplate,
}: WorkbenchWorkflowControlFlowTemplateSwapCardProps) {
  const options = useMemo(() => {
    const current = { kind: node.kind, operatorId: node.operator_id ?? undefined };
    const templates = listWorkflowNodeTemplatePresets(undefined, operatorDescriptors)
      .filter((preset) => {
        if (!PRIORITY_KINDS.has(preset.kind)) return false;
        if (!preset.operatorId) return true;
        const descriptor = (operatorDescriptors ?? []).find((entry) => entry.id === preset.operatorId);
        return Boolean(descriptor && isWorkflowDescriptorSupportedInRuntime(descriptor));
      })
      .map((preset) => ({ label: preset.label, value: buildTemplateValue({ kind: preset.kind, operatorId: preset.operatorId }) }));
    const currentValue = buildTemplateValue(current);
    if (!templates.some((entry) => entry.value === currentValue)) {
      templates.unshift({ label: node.operator_id ?? node.kind, value: currentValue });
    }
    return templates.slice(0, 10);
  }, [node.kind, node.operator_id, operatorDescriptors]);

  return (
    <label style={{ display: "grid", gap: "0.35rem", marginBottom: "0.75rem" }}>
      <span className="card-copy">{labels.operatorLabel}</span>
      <select
        onChange={(event) => {
          const next = parseTemplateValue(event.target.value);
          if (next) onSyncNodeTemplate(node.id, next);
        }}
        value={buildTemplateValue({ kind: node.kind, operatorId: node.operator_id ?? undefined })}
      >
        {options.map((option) => (
          <option key={`${node.id}:${option.value}`} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
