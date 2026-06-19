"use client";

import type { ProtocolAgentDescriptor } from "@/lib/api";

export type WorkbenchMeshClusterSummary = {
  clusterId: string;
  agentCount: number;
  relayCandidateCount: number;
  peerCount: number;
  averageHealthScore: number | null;
  entryAgentId: string;
};

export function summarizeWorkbenchMeshClusters(
  agents: readonly ProtocolAgentDescriptor[],
) {
  const meshAgents = agents.filter(
    (agent) =>
      agent.control_mode === "offline_mesh" ||
      Boolean(agent.mesh) ||
      Boolean(resolveMeshClusterId(agent)),
  );
  const grouped = new Map<string, ProtocolAgentDescriptor[]>();
  let unclusteredCount = 0;

  for (const agent of meshAgents) {
    const clusterId = resolveMeshClusterId(agent);
    if (!clusterId) {
      unclusteredCount += 1;
      continue;
    }
    grouped.set(clusterId, [...(grouped.get(clusterId) ?? []), agent]);
  }

  const clusters = [...grouped.entries()]
    .map(([clusterId, members]) => {
      const healthScores = members
        .map((agent) => agent.descriptor?.runtime.health_score)
        .filter((value): value is number => typeof value === "number");
      const peerIds = new Set<string>();
      for (const agent of members) {
        for (const peer of agent.mesh?.peers ?? []) {
          if (peer.id) peerIds.add(peer.id);
        }
      }
      const healthiest = [...members].sort(
        (left, right) =>
          (right.descriptor?.runtime.health_score ?? -1) -
          (left.descriptor?.runtime.health_score ?? -1),
      )[0];

      return {
        clusterId,
        agentCount: members.length,
        relayCandidateCount: members.filter(isMeshRelayCandidate).length,
        peerCount: peerIds.size,
        averageHealthScore:
          healthScores.length > 0
            ? healthScores.reduce((sum, value) => sum + value, 0) / healthScores.length
            : null,
        entryAgentId: healthiest?.id ?? members[0]?.id ?? "unknown",
      };
    })
    .sort((left, right) => left.clusterId.localeCompare(right.clusterId));

  return {
    clusters,
    clusterCount: clusters.length,
    relayCandidateCount: clusters.reduce(
      (sum, cluster) => sum + cluster.relayCandidateCount,
      0,
    ),
    unclusteredCount,
  };
}

function resolveMeshClusterId(agent: ProtocolAgentDescriptor) {
  return (
    agent.mesh?.cluster_id ??
    agent.cluster_id ??
    agent.descriptor?.runtime.cluster_id ??
    null
  );
}

function isMeshRelayCandidate(agent: ProtocolAgentDescriptor) {
  return (
    agent.mesh?.relay_candidate === true ||
    agent.mesh?.topology_role === "relay_candidate"
  );
}
