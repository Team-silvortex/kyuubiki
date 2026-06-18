"use client";

import { useMemo, useState } from "react";
import type { WorkflowOperatorDescriptor } from "@/lib/api";
import { listWorkflowNodeTemplatePresets, type WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { isWorkflowDescriptorSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import { scoreWorkflowNodeTemplatePresetSearch } from "@/components/workbench/workflow/workbench-workflow-operator-search-match";
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
  const operatorDescriptorMap = useMemo(
    () => new Map((operatorDescriptors ?? []).map((entry) => [entry.id, entry] as const)),
    [operatorDescriptors],
  );
  const presets = useMemo(() => {
    const normalized = query.trim();
    return listWorkflowNodeTemplatePresets(undefined, operatorDescriptors)
      .flatMap((preset) => {
        const descriptor = preset.operatorId
          ? operatorDescriptorMap.get(preset.operatorId)
          : undefined;
        if (preset.operatorId && (!descriptor || !isWorkflowDescriptorSupportedInRuntime(descriptor))) {
          return [];
        }
        if (!normalized) {
          return PRIORITY_KINDS.has(preset.kind) ? [{ preset, score: 0 }] : [];
        }
        const score = scoreWorkflowNodeTemplatePresetSearch(preset, descriptor, normalized);
        return score == null ? [] : [{ preset, score }];
      })
      .sort((left, right) => {
        const scoreDiff = right.score - left.score;
        if (scoreDiff !== 0) return scoreDiff;
        return left.preset.label.localeCompare(right.preset.label);
      })
      .map((entry) => entry.preset)
      .slice(0, normalized ? 8 : 6);
  }, [operatorDescriptorMap, operatorDescriptors, query]);

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
