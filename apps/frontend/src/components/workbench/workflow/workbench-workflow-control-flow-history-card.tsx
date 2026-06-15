"use client";

import { useMemo, useState } from "react";
import type { WorkflowAuditNavigationTarget } from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import { resolveWorkflowAuditNavigationTarget } from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";

type WorkbenchWorkflowControlFlowHistoryCardProps = {
  entries: WorkbenchAuditTimelineEntry[];
  onLocateTarget: (target: WorkflowAuditNavigationTarget) => void;
  onReplayEntry: (entry: WorkbenchAuditTimelineEntry) => void;
};

function formatActivityTimestamp(value: string) {
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function buildControlFlowDiffLabel(entry: WorkbenchAuditTimelineEntry) {
  const context = entry.context;
  if (entry.kind === "control_flow_edge_updated") {
    const previousTarget = typeof context?.previousTarget === "string" ? context.previousTarget : null;
    const nextTarget = typeof context?.nextTarget === "string" ? context.nextTarget : null;
    if (previousTarget && nextTarget && previousTarget !== nextTarget) return `${previousTarget} -> ${nextTarget}`;
    if (!previousTarget && nextTarget) return `connected -> ${nextTarget}`;
    if (previousTarget && !nextTarget) return `${previousTarget} -> disconnected`;
  }
  if (entry.kind === "control_flow_plane_inserted") {
    return entry.detail ?? "Inserted branch/merge plane";
  }
  if (entry.kind === "control_flow_node_added") {
    return typeof context?.controlFlowNodeKind === "string" ? context.controlFlowNodeKind : entry.detail ?? "Inserted control node";
  }
  return null;
}

function buildControlFlowGroupKey(entry: WorkbenchAuditTimelineEntry) {
  const branchNodeId = typeof entry.context?.branchNodeId === "string" ? entry.context.branchNodeId : null;
  const branchOutputId = typeof entry.context?.branchOutputId === "string" ? entry.context.branchOutputId : null;
  if (branchNodeId && branchOutputId) return `${branchNodeId}.${branchOutputId}`;
  return "structure";
}

function buildControlFlowGroupLabel(groupKey: string) {
  return groupKey === "structure" ? "Structure changes" : groupKey;
}

function buildControlFlowGroupCurrentTarget(entries: WorkbenchAuditTimelineEntry[]) {
  for (const entry of entries) {
    const nextTarget = typeof entry.context?.nextTarget === "string" ? entry.context.nextTarget : null;
    const previousTarget = typeof entry.context?.previousTarget === "string" ? entry.context.previousTarget : null;
    if (nextTarget) return nextTarget;
    if (previousTarget && entry.kind === "control_flow_edge_updated") return "disconnected";
  }
  return null;
}

export function WorkbenchWorkflowControlFlowHistoryCard({
  entries,
  onLocateTarget,
  onReplayEntry,
}: WorkbenchWorkflowControlFlowHistoryCardProps) {
  const [activeReplayEntryId, setActiveReplayEntryId] = useState<string | null>(null);
  const groupedEntries = useMemo(() => {
    const groups = new Map<string, WorkbenchAuditTimelineEntry[]>();
    for (const entry of entries) {
      const key = buildControlFlowGroupKey(entry);
      groups.set(key, [...(groups.get(key) ?? []), entry]);
    }
    return [...groups.entries()];
  }, [entries]);
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>Control-flow history</h2>
        <span className={`status-pill status-pill--${entries.length > 0 ? "watch" : "good"}`}>{entries.length}</span>
      </div>
      {entries.length > 0 ? (
        <div className="sidebar-stack">
          {groupedEntries.map(([groupKey, groupEntries]) => {
            const currentTarget = buildControlFlowGroupCurrentTarget(groupEntries);
            const isActiveGroup = groupEntries.some((entry) => entry.id === activeReplayEntryId);
            return (
              <details className="sidebar-card sidebar-card--compact" key={groupKey} open>
                <summary className="sidebar-list__row" style={{ cursor: "pointer", listStyle: "none" }}>
                  <span>{buildControlFlowGroupLabel(groupKey)}</span>
                  <strong>{isActiveGroup ? `${groupEntries.length} active` : groupEntries.length}</strong>
                </summary>
                {currentTarget ? (
                  <div className="sidebar-list" style={{ marginTop: "0.45rem" }}>
                    <div className="sidebar-list__row">
                      <span>current target</span>
                      <strong>{currentTarget}</strong>
                    </div>
                  </div>
                ) : null}
                <div className="sidebar-stack" style={{ marginTop: "0.6rem" }}>
                  {groupEntries.map((entry) => {
                    const target = resolveWorkflowAuditNavigationTarget(entry);
                    const diffLabel = buildControlFlowDiffLabel(entry);
                    const isActiveEntry = entry.id === activeReplayEntryId;
                    return (
                      <div className="sidebar-card sidebar-card--compact" key={entry.id}>
                        <div className="sidebar-list">
                          <div className="sidebar-list__row">
                            <span>{entry.title}</span>
                            <strong>{formatActivityTimestamp(entry.at)}</strong>
                          </div>
                          <div className="sidebar-list__row">
                            <span>kind</span>
                            <strong>{entry.kind}</strong>
                          </div>
                          {diffLabel ? (
                            <div className="sidebar-list__row">
                              <span>diff</span>
                              <strong>{diffLabel}</strong>
                            </div>
                          ) : null}
                          {entry.detail ? (
                            <div className="sidebar-list__row">
                              <span>detail</span>
                              <strong>{entry.detail}</strong>
                            </div>
                          ) : null}
                          <div className="sidebar-list__row">
                            <span>replay</span>
                            <button onClick={() => { setActiveReplayEntryId(entry.id); onReplayEntry(entry); window.setTimeout(() => setActiveReplayEntryId((current) => current === entry.id ? null : current), 2200); }} type="button">{isActiveEntry ? "Active" : "Highlight"}</button>
                          </div>
                          {target ? (
                            <div className="sidebar-list__row">
                              <span>target</span>
                              <button onClick={() => onLocateTarget(target)} type="button">{target.label}</button>
                            </div>
                          ) : null}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </details>
            );
          })}
        </div>
      ) : (
        <p className="card-copy">No recent branch or merge design actions were recorded in this session.</p>
      )}
    </section>
  );
}
