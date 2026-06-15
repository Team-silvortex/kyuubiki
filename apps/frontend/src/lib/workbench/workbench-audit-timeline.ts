"use client";

import type { ProtocolAgentDescriptor } from "@/lib/api";
import {
  readSecurityAuditLog,
  type WorkbenchSecurityAuditEntry,
} from "@/lib/workbench/security-audit";
import {
  readWorkflowActivityLog,
  type WorkflowActivityLogEntry,
} from "@/lib/workbench/workflow-activity-log";

export type WorkbenchAuditTimelineEntry = {
  id: string;
  at: string;
  source: "workflow" | "security";
  class: "persistent" | "runtime_snapshot";
  tone: "good" | "watch" | "risk";
  title: string;
  detail?: string;
  count?: number;
  kind: string;
  context?: Record<string, unknown>;
};

function resolveSecurityTone(entry: WorkbenchSecurityAuditEntry): WorkbenchAuditTimelineEntry["tone"] {
  if (entry.status === "failed" || entry.status === "cancelled") return "risk";
  if (entry.risk === "destructive" || entry.status === "prompted") return "watch";
  return "good";
}

function mapWorkflowEntry(entry: WorkflowActivityLogEntry): WorkbenchAuditTimelineEntry {
  return {
    id: `workflow:${entry.id}`,
    at: entry.at,
    source: "workflow",
    class: "persistent",
    tone: entry.kind === "package_residual_repaired" ? "good" : "watch",
    title: entry.message,
    detail: entry.detail,
    count: entry.count,
    kind: entry.kind,
    context: entry.context,
  };
}

function mapSecurityEntry(entry: WorkbenchSecurityAuditEntry): WorkbenchAuditTimelineEntry {
  return {
    id: `security:${entry.id}`,
    at: entry.at,
    source: "security",
    class: "persistent",
    tone: resolveSecurityTone(entry),
    title: entry.action,
    detail: entry.note,
    kind: `${entry.source}:${entry.status}:${entry.risk}`,
    context: entry.context,
  };
}

function buildRuntimeSnapshotEntries(input: {
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  protocolAgents: readonly ProtocolAgentDescriptor[];
}) {
  const entries: WorkbenchAuditTimelineEntry[] = [];
  const activeLeaseCount = input.protocolAgents.filter((agent) => Boolean(agent.active_lease)).length;
  const staleLeaseCount = input.protocolAgents.filter((agent) => agent.active_lease?.is_stale).length;
  const authorityModes = [...new Set(input.protocolAgents.map((agent) => agent.descriptor?.authority?.authority_mode).filter(Boolean))];
  const runtimeModes = [...new Set(input.protocolAgents.map((agent) => agent.descriptor?.runtime?.runtime_mode).filter(Boolean))];

  entries.push({
    id: `runtime-mode:${input.frontendRuntimeMode}:${input.protocolAgents.length}`,
    at: new Date().toISOString(),
    source: "workflow",
    class: "runtime_snapshot",
    tone: staleLeaseCount > 0 ? "risk" : activeLeaseCount > 0 ? "watch" : "good",
    title: `Runtime mode snapshot: ${input.frontendRuntimeMode}`,
    detail: `${input.protocolAgents.length} agents, ${activeLeaseCount} active leases, ${staleLeaseCount} stale leases`,
    count: input.protocolAgents.length,
    kind: "runtime_mode_snapshot",
    context: {
      frontendRuntimeMode: input.frontendRuntimeMode,
      protocolAgentCount: input.protocolAgents.length,
      activeLeaseCount,
      staleLeaseCount,
    },
  });

  if (authorityModes.length > 0 || runtimeModes.length > 0) {
    entries.push({
      id: `runtime-authority:${authorityModes.join(",")}:${runtimeModes.join(",")}`,
      at: new Date().toISOString(),
      source: "workflow",
      class: "runtime_snapshot",
      tone: authorityModes.length > 1 || runtimeModes.length > 1 ? "risk" : "watch",
      title: "Agent authority snapshot",
      detail: `${authorityModes.join(", ") || "--"} / ${runtimeModes.join(", ") || "--"}`,
      count: authorityModes.length,
      kind: "agent_authority_snapshot",
      context: {
        authorityModes,
        runtimeModes,
      },
    });
  }

  entries.push(
    ...input.protocolAgents
      .filter((agent) => agent.active_lease)
      .slice(0, 4)
      .map((agent) => {
        const tone: WorkbenchAuditTimelineEntry["tone"] = agent.active_lease?.is_stale ? "risk" : "watch";
        return {
        id: `agent-lease:${agent.id}:${agent.active_lease?.job_id ?? "lease"}`,
        at: new Date().toISOString(),
        source: "workflow" as const,
        class: "runtime_snapshot" as const,
        tone,
        title: `Agent lease snapshot: ${agent.id}`,
        detail: `${agent.active_lease?.method ?? "--"} / ${agent.active_lease?.job_id ?? "--"}`,
        kind: agent.active_lease?.is_stale ? "agent_lease_stale" : "agent_lease_active",
        context: {
          agentId: agent.id,
          method: agent.active_lease?.method ?? null,
          jobId: agent.active_lease?.job_id ?? null,
          isStale: agent.active_lease?.is_stale ?? false,
          authorityMode: agent.descriptor?.authority?.authority_mode ?? null,
          runtimeMode: agent.descriptor?.runtime?.runtime_mode ?? null,
        },
      }}),
  );

  return entries;
}

export function readWorkbenchAuditTimeline(
  workflowId?: string | null,
  limit = 16,
  runtimeSnapshot?: {
    frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
    protocolAgents: readonly ProtocolAgentDescriptor[];
  },
) {
  return [
    ...readWorkflowActivityLog(workflowId).map(mapWorkflowEntry),
    ...readSecurityAuditLog().map(mapSecurityEntry),
    ...(runtimeSnapshot ? buildRuntimeSnapshotEntries(runtimeSnapshot) : []),
  ]
    .sort((left, right) => Date.parse(right.at) - Date.parse(left.at))
    .slice(0, limit);
}
