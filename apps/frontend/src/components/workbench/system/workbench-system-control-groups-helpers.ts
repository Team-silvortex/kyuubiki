"use client";

import type { ProtocolAgentDescriptor } from "@/lib/api";
import type { WorkbenchMeshClusterSummary } from "@/components/workbench/system/workbench-system-mesh-topology-helpers";

export type WorkbenchControlGroupSummary = {
  id: string;
  kind: "orchestrated" | "direct" | "mesh";
  agentCount: number;
  relayCandidateCount: number;
  peerCount: number;
  averageHealthScore: number | null;
  entryAgentId: string;
  sessionCount: number;
};

export function summarizeWorkbenchOrchestratedGroups(
  agents: readonly ProtocolAgentDescriptor[],
) {
  const managedAgents = agents.filter(
    (agent) => (agent.control_mode ?? agent.descriptor?.authority?.control_mode) === "orch_managed",
  );
  const groups = new Map<string, ProtocolAgentDescriptor[]>();

  for (const agent of managedAgents) {
    const orchId =
      agent.orch_id ?? agent.descriptor?.authority?.orchestrator_id ?? "orchestra/default";
    groups.set(orchId, [...(groups.get(orchId) ?? []), agent]);
  }

  return [...groups.entries()]
    .map(([orchId, members]) =>
      buildGroupSummary(
        orchId,
        "orchestrated",
        members,
        members.flatMap((agent) => agent.descriptor?.runtime.peers ?? []),
        0,
        new Set(
          members
            .map(
              (agent) =>
                agent.orch_session_id ??
                agent.descriptor?.authority?.orchestrator_session_id ??
                null,
            )
            .filter(Boolean),
        ).size,
      ),
    )
    .sort((left, right) => left.id.localeCompare(right.id));
}

export function summarizeWorkbenchDirectGroups(
  agents: readonly ProtocolAgentDescriptor[],
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui",
) {
  const directAgents =
    frontendRuntimeMode === "direct_mesh_gui"
      ? agents.filter(
          (agent) => (agent.control_mode ?? agent.descriptor?.authority?.control_mode) !== "orch_managed",
        )
      : [];
  if (directAgents.length === 0) return [];
  return [
    buildGroupSummary(
      "direct-entry",
      "direct",
      directAgents,
      directAgents.flatMap((agent) => agent.descriptor?.runtime.peers ?? []),
      directAgents.filter((agent) => agent.mesh?.relay_candidate).length,
      1,
    ),
  ];
}

export function controlGroupsFromMeshClusters(
  clusters: readonly WorkbenchMeshClusterSummary[],
) {
  return clusters.map((cluster) => ({
    id: cluster.clusterId,
    kind: "mesh" as const,
    agentCount: cluster.agentCount,
    relayCandidateCount: cluster.relayCandidateCount,
    peerCount: cluster.peerCount,
    averageHealthScore: cluster.averageHealthScore,
    entryAgentId: cluster.entryAgentId,
    sessionCount: 1,
  }));
}

function buildGroupSummary(
  id: string,
  kind: WorkbenchControlGroupSummary["kind"],
  members: readonly ProtocolAgentDescriptor[],
  peers: readonly { address: string }[],
  relayCandidateCount: number,
  sessionCount: number,
): WorkbenchControlGroupSummary {
  const healthScores = members
    .map((agent) => agent.descriptor?.runtime.health_score)
    .filter((value): value is number => typeof value === "number");
  const healthiest = [...members].sort(
    (left, right) =>
      (right.descriptor?.runtime.health_score ?? -1) -
      (left.descriptor?.runtime.health_score ?? -1),
  )[0];

  return {
    id,
    kind,
    agentCount: members.length,
    relayCandidateCount,
    peerCount: new Set(peers.map((peer) => peer.address).filter(Boolean)).size,
    averageHealthScore:
      healthScores.length > 0
        ? healthScores.reduce((sum, value) => sum + value, 0) / healthScores.length
        : null,
    entryAgentId: healthiest?.id ?? members[0]?.id ?? "unknown",
    sessionCount,
  };
}
