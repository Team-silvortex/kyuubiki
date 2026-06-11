"use client";

import { useMemo, useState } from "react";
import type { WorkflowOperatorDescriptor } from "@/lib/api";
import { listWorkflowNodeTemplatePresets, type WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { isWorkflowDescriptorSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowControlFlowQuickAddCardProps = {
  labels: WorkflowSidebarLabels;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  onAddNode: (template?: WorkflowNodeTemplateSelection) => void;
};

const PRIORITY_KINDS = new Set(["condition", "transform", "extract", "export", "input"]);

export function WorkbenchWorkflowControlFlowQuickAddCard({
  labels,
  operatorDescriptors,
  onAddNode,
}: WorkbenchWorkflowControlFlowQuickAddCardProps) {
  const [query, setQuery] = useState("");
  const presets = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return listWorkflowNodeTemplatePresets(undefined, operatorDescriptors)
      .filter((preset) => {
        if (preset.operatorId) {
          const descriptor = (operatorDescriptors ?? []).find((entry) => entry.id === preset.operatorId);
          if (!descriptor || !isWorkflowDescriptorSupportedInRuntime(descriptor)) return false;
        }
        if (!normalized) return PRIORITY_KINDS.has(preset.kind);
        return [preset.label, preset.operatorId, preset.kind, preset.id]
          .filter(Boolean)
          .some((value) => value!.toLowerCase().includes(normalized));
      })
      .slice(0, normalized ? 8 : 6);
  }, [operatorDescriptors, query]);

  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>{labels.addNodeLabel}</h2>
        <span className="status-pill status-pill--watch">{presets.length}</span>
      </div>
      <label style={{ display: "grid", gap: "0.35rem", marginBottom: "0.75rem" }}>
        <span className="card-copy">{labels.operatorSearchLabel}</span>
        <input
          onChange={(event) => setQuery(event.target.value)}
          placeholder={labels.operatorSearchPlaceholder}
          type="search"
          value={query}
        />
      </label>
      <div style={{ display: "flex", gap: "0.45rem", flexWrap: "wrap" }}>
        {presets.map((preset) => (
          <button
            key={preset.id}
            onClick={() => onAddNode({ kind: preset.kind, operatorId: preset.operatorId })}
            style={{
              display: "grid",
              gap: "0.2rem",
              justifyItems: "start",
              padding: "0.55rem 0.7rem",
              borderRadius: "10px",
              border: "1px solid var(--line)",
              background: "linear-gradient(180deg, rgba(255,255,255,0.05), rgba(0,0,0,0.2))",
            }}
            type="button"
          >
            <strong style={{ fontSize: "0.9rem" }}>{preset.label}</strong>
            <span className="card-copy">{preset.operatorId ?? preset.kind}</span>
          </button>
        ))}
      </div>
      {presets.length === 0 ? <p className="card-copy" style={{ marginTop: "0.75rem" }}>{labels.operatorSearchEmptyLabel}</p> : null}
    </section>
  );
}
