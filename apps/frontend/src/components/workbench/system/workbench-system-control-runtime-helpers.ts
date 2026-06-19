"use client";

import type { DirectMeshSelectionMode, FrontendRuntimeMode, ProtocolAgentDescriptor } from "@/lib/api";

export function countWorkbenchControlEndpoints(value: string) {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean).length;
}

export function countWorkbenchControlPeers(
  agents: readonly ProtocolAgentDescriptor[],
) {
  const uniquePeers = new Set<string>();
  for (const agent of agents) {
    for (const peer of agent.descriptor?.runtime.peers ?? []) {
      if (peer.address) uniquePeers.add(peer.address);
    }
  }
  return uniquePeers.size;
}

export function summarizeWorkbenchPeerHealth(
  agents: readonly ProtocolAgentDescriptor[],
) {
  let healthy = 0;
  let degraded = 0;
  let stale = 0;
  for (const agent of agents) {
    for (const peer of agent.descriptor?.runtime.peers ?? []) {
      if (peer.status === "healthy" || peer.status === "online") healthy += 1;
      else if (peer.status === "stale" || peer.status === "offline") stale += 1;
      else degraded += 1;
    }
  }
  return { healthy, degraded, stale };
}

export function summarizeWorkbenchPeerObservability(
  agents: readonly ProtocolAgentDescriptor[],
  nowUnixS: number,
) {
  let latestSeen: number | null = null;
  let totalFailureCount = 0;
  for (const agent of agents) {
    for (const peer of agent.descriptor?.runtime.peers ?? []) {
      if (typeof peer.last_seen_unix_s === "number") {
        latestSeen =
          latestSeen === null ? peer.last_seen_unix_s : Math.max(latestSeen, peer.last_seen_unix_s);
      }
      totalFailureCount += peer.failure_count ?? 0;
    }
  }

  const ageSeconds = latestSeen === null ? null : Math.max(0, nowUnixS - latestSeen);
  return { latestSeen, ageSeconds, totalFailureCount };
}

export function formatWorkbenchControlAgeLabel(ageSeconds: number | null) {
  if (ageSeconds === null) return "--";
  if (ageSeconds < 60) return `${ageSeconds}s ago`;
  if (ageSeconds < 3600) return `${Math.floor(ageSeconds / 60)}m ago`;
  if (ageSeconds < 86400) return `${Math.floor(ageSeconds / 3600)}h ago`;
  return `${Math.floor(ageSeconds / 86400)}d ago`;
}

export function pickWorkbenchControlEntryAgentId(
  agents: readonly ProtocolAgentDescriptor[],
  directMeshSelectionMode: DirectMeshSelectionMode,
  frontendRuntimeMode: FrontendRuntimeMode,
) {
  if (agents.length === 0) {
    return frontendRuntimeMode === "direct_mesh_gui" ? "direct-mesh seed" : "orchestra";
  }
  if (directMeshSelectionMode === "healthiest") {
    return [...agents].sort((left, right) => {
      const leftScore = left.descriptor?.runtime.health_score ?? -1;
      const rightScore = right.descriptor?.runtime.health_score ?? -1;
      return rightScore - leftScore;
    })[0]?.id ?? agents[0]?.id ?? "entry-agent";
  }
  return agents[0]?.id ?? "entry-agent";
}

export function controlWindowModeForAuthority(
  authorityMode: "single_orchestrator" | "offline_mesh",
  agentCount: number,
) {
  if (authorityMode === "single_orchestrator") return "orchestrated" as const;
  return agentCount > 1 ? ("mesh" as const) : ("direct" as const);
}
