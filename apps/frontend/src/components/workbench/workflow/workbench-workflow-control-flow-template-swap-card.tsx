"use client";

import { useMemo, useState } from "react";
import type { WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";
import { listWorkflowNodeTemplatePresets, type WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { scoreWorkflowNodeTemplatePresetSearch } from "@/components/workbench/workflow/workbench-workflow-operator-search-match";
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
  const [query, setQuery] = useState("");
  const operatorDescriptorMap = useMemo(
    () => new Map((operatorDescriptors ?? []).map((entry) => [entry.id, entry] as const)),
    [operatorDescriptors],
  );
  const options = useMemo(() => {
    const current = { kind: node.kind, operatorId: node.operator_id ?? undefined };
    const normalized = query.trim();
    const templates = listWorkflowNodeTemplatePresets(undefined, operatorDescriptors)
      .flatMap((preset) => {
        if (!PRIORITY_KINDS.has(preset.kind)) return [];
        const descriptor = preset.operatorId
          ? operatorDescriptorMap.get(preset.operatorId)
          : undefined;
        if (preset.operatorId && (!descriptor || !isWorkflowDescriptorSupportedInRuntime(descriptor))) {
          return [];
        }
        const value = buildTemplateValue({ kind: preset.kind, operatorId: preset.operatorId });
        if (!normalized) return [{ label: preset.label, value, score: 0 }];
        const score = scoreWorkflowNodeTemplatePresetSearch(preset, descriptor, normalized);
        return score == null ? [] : [{ label: preset.label, value, score }];
      })
      .sort((left, right) => right.score - left.score || left.label.localeCompare(right.label))
      .map(({ label, value }) => ({ label, value }));
    const currentValue = buildTemplateValue(current);
    if (!templates.some((entry) => entry.value === currentValue)) {
      templates.unshift({ label: node.operator_id ?? node.kind, value: currentValue });
    }
    return templates.slice(0, 10);
  }, [node.kind, node.operator_id, operatorDescriptorMap, operatorDescriptors, query]);

  return (
    <div style={{ display: "grid", gap: "0.5rem", marginBottom: "0.75rem" }}>
      <label style={{ display: "grid", gap: "0.35rem" }}>
        <span className="card-copy">{labels.operatorSearchLabel}</span>
        <input
          onChange={(event) => setQuery(event.target.value)}
          placeholder={labels.operatorSearchPlaceholder}
          type="search"
          value={query}
        />
      </label>
      <label style={{ display: "grid", gap: "0.35rem" }}>
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
    </div>
  );
}
