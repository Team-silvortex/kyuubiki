"use client";

import type { ProtocolAgentDescriptor } from "@/lib/api";
import {
  matchesWorkflowAuditFocusHint,
  type WorkflowAuditFocusHint,
  resolveWorkflowAuditAgentMatches,
  resolveWorkflowAuditNavigationTarget,
  type WorkflowAuditNavigationTarget,
} from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import { useMemo, useState } from "react";
import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";

type WorkbenchWorkflowActivityLogCardProps = {
  auditFocusHint?: WorkflowAuditFocusHint | null;
  entries: WorkbenchAuditTimelineEntry[];
  onLocateTarget: (target: WorkflowAuditNavigationTarget) => void;
  protocolAgents: readonly ProtocolAgentDescriptor[];
  workflowId: string;
};

function formatActivityTimestamp(value: string) {
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function escapeCsvField(value: string | number | undefined) {
  const text = String(value ?? "");
  return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function stringifyContext(context: Record<string, unknown> | undefined) {
  if (!context) return "";
  try {
    return JSON.stringify(context);
  } catch {
    return "";
  }
}

function collectContextFocusTokens(entry: WorkbenchAuditTimelineEntry) {
  const context = entry.context;
  if (!context) return [] as string[];
  const values = [
    context.jobId,
    context.agentId,
    context.runtimeMode,
    context.authorityMode,
    context.frontendRuntimeMode,
    context.method,
  ];
  return [...new Set(values.filter((value): value is string => typeof value === "string" && value.length > 0))];
}

function matchesContextFocus(entry: WorkbenchAuditTimelineEntry, tokens: readonly string[]) {
  if (tokens.length === 0) return true;
  const serialized = stringifyContext(entry.context).toLowerCase();
  return tokens.some((token) => serialized.includes(token.toLowerCase()));
}

export function WorkbenchWorkflowActivityLogCard({
  auditFocusHint,
  entries,
  onLocateTarget,
  protocolAgents,
  workflowId,
}: WorkbenchWorkflowActivityLogCardProps) {
  const [sourceFilter, setSourceFilter] = useState<"all" | "workflow" | "security">("all");
  const [toneFilter, setToneFilter] = useState<"all" | "good" | "watch" | "risk">("all");
  const [classFilter, setClassFilter] = useState<"all" | "persistent" | "runtime_snapshot">("all");
  const [timeFilter, setTimeFilter] = useState<"all" | "15m" | "1h" | "24h">("all");
  const [kindQuery, setKindQuery] = useState("");
  const [contextQuery, setContextQuery] = useState("");
  const [focusEntryId, setFocusEntryId] = useState<string | null>(null);
  const [builderFocusOnly, setBuilderFocusOnly] = useState(false);
  const baseFilteredEntries = useMemo(
    () => {
      const now = Date.now();
      const timeWindowMs =
        timeFilter === "15m" ? 15 * 60 * 1000
          : timeFilter === "1h" ? 60 * 60 * 1000
          : timeFilter === "24h" ? 24 * 60 * 60 * 1000
          : null;
      const normalizedKindQuery = kindQuery.trim().toLowerCase();
      const normalizedContextQuery = contextQuery.trim().toLowerCase();
      return entries.filter((entry) => {
        const matchesSource = sourceFilter === "all" || entry.source === sourceFilter;
        const matchesTone = toneFilter === "all" || entry.tone === toneFilter;
        const matchesClass = classFilter === "all" || entry.class === classFilter;
        const matchesKind =
          normalizedKindQuery.length === 0 ||
          entry.kind.toLowerCase().includes(normalizedKindQuery) ||
          entry.title.toLowerCase().includes(normalizedKindQuery);
        const matchesContext =
          normalizedContextQuery.length === 0 ||
          stringifyContext(entry.context).toLowerCase().includes(normalizedContextQuery) ||
          (entry.detail ?? "").toLowerCase().includes(normalizedContextQuery);
        const matchesTime =
          timeWindowMs === null ||
          now - Date.parse(entry.at) <= timeWindowMs;
        return matchesSource && matchesTone && matchesClass && matchesKind && matchesContext && matchesTime;
      });
    },
    [classFilter, contextQuery, entries, kindQuery, sourceFilter, timeFilter, toneFilter],
  );
  const focusEntry = useMemo(
    () => baseFilteredEntries.find((entry) => entry.id === focusEntryId) ?? null,
    [baseFilteredEntries, focusEntryId],
  );
  const focusTokens = useMemo(
    () => (focusEntry ? collectContextFocusTokens(focusEntry) : []),
    [focusEntry],
  );
  const focusTarget = useMemo(
    () => (focusEntry ? resolveWorkflowAuditNavigationTarget(focusEntry) : null),
    [focusEntry],
  );
  const focusAgentMatches = useMemo(
    () => (focusEntry ? resolveWorkflowAuditAgentMatches(focusEntry, protocolAgents) : []),
    [focusEntry, protocolAgents],
  );
  const builderFocusEntries = useMemo(
    () => baseFilteredEntries.filter((entry) => matchesWorkflowAuditFocusHint(entry, auditFocusHint)),
    [auditFocusHint, baseFilteredEntries],
  );
  const filteredEntries = useMemo(
    () =>
      baseFilteredEntries.filter((entry) =>
        matchesContextFocus(entry, focusTokens) &&
        (!builderFocusOnly || matchesWorkflowAuditFocusHint(entry, auditFocusHint))),
    [auditFocusHint, baseFilteredEntries, builderFocusOnly, focusTokens],
  );
  const filterMeta = useMemo(
    () => ({
      source: sourceFilter,
      tone: toneFilter,
      class: classFilter,
      time: timeFilter,
      kindQuery: kindQuery.trim(),
      contextQuery: contextQuery.trim(),
      builderFocusOnly,
      focusEntryId,
      focusTokens,
    }),
    [builderFocusOnly, classFilter, contextQuery, focusEntryId, focusTokens, kindQuery, sourceFilter, timeFilter, toneFilter],
  );

  function exportTimelineJson() {
    const blob = new Blob([JSON.stringify({
      workflowId,
      exportedAt: new Date().toISOString(),
      filters: filterMeta,
      entries: filteredEntries,
    }, null, 2)], {
      type: "application/json",
    });
    const href = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.download = `${workflowId}.audit-timeline.json`;
    anchor.click();
    URL.revokeObjectURL(href);
  }

  function exportTimelineCsv() {
    const rows = [
      `# workflowId=${workflowId}`,
      `# exportedAt=${new Date().toISOString()}`,
      `# filters=${JSON.stringify(filterMeta)}`,
      ["at", "source", "class", "tone", "kind", "title", "detail", "count", "context"].join(","),
      ...filteredEntries.map((entry) =>
        [
          escapeCsvField(entry.at),
          escapeCsvField(entry.source),
          escapeCsvField(entry.class),
          escapeCsvField(entry.tone),
          escapeCsvField(entry.kind),
          escapeCsvField(entry.title),
          escapeCsvField(entry.detail),
          escapeCsvField(entry.count),
          escapeCsvField(stringifyContext(entry.context)),
        ].join(","),
      ),
    ];
    const blob = new Blob([rows.join("\n")], {
      type: "text/csv;charset=utf-8",
    });
    const href = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.download = `${workflowId}.audit-timeline.csv`;
    anchor.click();
    URL.revokeObjectURL(href);
  }

  async function copyCurrentFilters() {
    const payload = JSON.stringify({
      workflowId,
      filters: filterMeta,
    }, null, 2);
    try {
      await navigator.clipboard.writeText(payload);
    } catch {
      window.prompt("Copy audit filters", payload);
    }
  }

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>Audit timeline</h2>
        <span className={`status-pill status-pill--${filteredEntries.length > 0 ? "watch" : "good"}`}>{filteredEntries.length}</span>
      </div>
      {focusEntry ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>Context focus</span>
            <strong>{focusEntry.title}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>tokens</span>
            <strong>{focusTokens.join(", ") || "--"}</strong>
          </div>
          {focusTarget ? (
            <div className="sidebar-list__row">
              <span>target</span>
              <button onClick={() => onLocateTarget(focusTarget)} type="button">{focusTarget.label}</button>
            </div>
          ) : null}
          {focusAgentMatches[0] ? (
            <div className="sidebar-list__row">
              <span>agent</span>
              <strong>{`${focusAgentMatches[0].id} (${focusAgentMatches[0].descriptor?.runtime?.runtime_mode ?? "--"})`}</strong>
            </div>
          ) : null}
          <button onClick={() => setFocusEntryId(null)} type="button">Clear focus</button>
        </div>
      ) : null}
      {auditFocusHint?.nodeId || auditFocusHint?.branchNodeId || auditFocusHint?.edgeId || auditFocusHint?.artifactNodeId || auditFocusHint?.datasetValueId ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>Builder focus</span>
            <strong>{auditFocusHint.branchNodeId ? `${auditFocusHint.branchNodeId}.${auditFocusHint.branchOutputId ?? "--"}` : auditFocusHint.nodeId ?? auditFocusHint.edgeId ?? auditFocusHint.artifactNodeId ?? auditFocusHint.datasetValueId ?? "--"}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>linked events</span>
            <strong>{builderFocusEntries.length}</strong>
          </div>
        </div>
      ) : null}
      <div className="button-row">
        <button onClick={() => setSourceFilter("all")} type="button">{sourceFilter === "all" ? "Source: all" : "All sources"}</button>
        <button onClick={() => setSourceFilter(sourceFilter === "workflow" ? "all" : "workflow")} type="button">Workflow</button>
        <button onClick={() => setSourceFilter(sourceFilter === "security" ? "all" : "security")} type="button">Security</button>
        <button onClick={() => setToneFilter(toneFilter === "risk" ? "all" : "risk")} type="button">Risk</button>
        <button onClick={() => setClassFilter(classFilter === "runtime_snapshot" ? "all" : "runtime_snapshot")} type="button">Runtime</button>
        <button onClick={() => setClassFilter(classFilter === "persistent" ? "all" : "persistent")} type="button">Persistent</button>
        <button onClick={() => setTimeFilter(timeFilter === "15m" ? "all" : "15m")} type="button">15m</button>
        <button onClick={() => setTimeFilter(timeFilter === "1h" ? "all" : "1h")} type="button">1h</button>
        <button onClick={() => setTimeFilter(timeFilter === "24h" ? "all" : "24h")} type="button">24h</button>
        {auditFocusHint?.nodeId || auditFocusHint?.branchNodeId || auditFocusHint?.edgeId || auditFocusHint?.artifactNodeId || auditFocusHint?.datasetValueId ? (
          <button onClick={() => setBuilderFocusOnly((current) => !current)} type="button">{builderFocusOnly ? "All events" : "Builder focus"}</button>
        ) : null}
        <button onClick={copyCurrentFilters} type="button">Copy filters</button>
        <button onClick={exportTimelineJson} type="button">Export JSON</button>
        <button onClick={exportTimelineCsv} type="button">Export CSV</button>
      </div>
      <label style={{ display: "grid", gap: "0.35rem" }}>
        <span className="card-copy">Kind search</span>
        <input onChange={(event) => setKindQuery(event.target.value)} placeholder="lease, governance, import..." value={kindQuery} />
      </label>
      <label style={{ display: "grid", gap: "0.35rem" }}>
        <span className="card-copy">Context search</span>
        <input onChange={(event) => setContextQuery(event.target.value)} placeholder="job id, agent id, runtime mode..." value={contextQuery} />
      </label>
      {filteredEntries.length > 0 ? (
        <div className="sidebar-stack">
          {filteredEntries.map((entry) => (
            <div className="sidebar-card sidebar-card--compact" key={entry.id}>
              <div className="sidebar-list">
                <div className="sidebar-list__row">
                  <span>{entry.title}</span>
                  <strong>{formatActivityTimestamp(entry.at)}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>trace</span>
                  <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", justifyContent: "flex-end" }}>
                    <button
                      onClick={() => setFocusEntryId((current) => current === entry.id ? null : entry.id)}
                      type="button"
                    >
                      {focusEntryId === entry.id ? "Focused" : "Focus chain"}
                    </button>
                    {resolveWorkflowAuditNavigationTarget(entry) ? (
                      <button onClick={() => onLocateTarget(resolveWorkflowAuditNavigationTarget(entry)!)} type="button">Open target</button>
                    ) : null}
                  </div>
                </div>
                <div className="sidebar-list__row">
                  <span>source</span>
                  <strong>{matchesWorkflowAuditFocusHint(entry, auditFocusHint) ? `${entry.source} · linked` : entry.source}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>class</span>
                  <strong>{entry.class}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>kind</span>
                  <strong>{entry.kind}</strong>
                </div>
                {entry.count !== undefined ? (
                  <div className="sidebar-list__row">
                    <span>count</span>
                    <strong>{entry.count}</strong>
                  </div>
                ) : null}
                {entry.detail ? (
                  <div className="sidebar-list__row">
                    <span>detail</span>
                    <strong className={`status-pill status-pill--${entry.tone}`}>{entry.detail}</strong>
                  </div>
                ) : null}
                {entry.context ? (
                  <div className="sidebar-list__row">
                    <span>context</span>
                    <strong>{stringifyContext(entry.context)}</strong>
                  </div>
                ) : null}
                {resolveWorkflowAuditAgentMatches(entry, protocolAgents)[0] ? (
                  <div className="sidebar-list__row">
                    <span>agent match</span>
                    <strong>{resolveWorkflowAuditAgentMatches(entry, protocolAgents)[0]?.id}</strong>
                  </div>
                ) : null}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="card-copy">No audit events match the current filters in this session.</p>
      )}
    </section>
  );
}
