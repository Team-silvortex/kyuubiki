import type { ProtocolAgentDescriptor } from "@/lib/api";

export function buildProtocolAgentCards({
  agents,
  labels,
  clusterHealthTone,
  peerStatusLabel,
}: {
  agents: ProtocolAgentDescriptor[];
  labels: {
    authorityMode: string;
    controlMode: string;
    runtimeMode: string;
    cluster: string;
    clusterSize: string;
    clusterHealth: string;
    peers: string;
    headless: string;
    yes: string;
    no: string;
    capabilities: string;
    methods: string;
    peerState: string;
    execution: string;
    leaseAge: string;
    leaseIdle: string;
    leaseActive: string;
    leaseStale: string;
    leaseUnknown: string;
    leaseStateChip: string;
    leaseAgeChip: string;
    leaseJobChip: string;
    leaseMethodChip: string;
  };
  clusterHealthTone: (score: number | null | undefined) => string;
  peerStatusLabel: (status: string | undefined) => string;
}) {
  return agents.slice(0, 4).map((agent) => ({
    id: agent.id,
    endpoint: `${agent.host}:${agent.port}`,
    metrics: [
      { label: labels.authorityMode, value: agent.descriptor?.authority?.authority_mode ?? "--" },
      { label: labels.controlMode, value: agent.descriptor?.authority?.control_mode ?? "--" },
      { label: labels.runtimeMode, value: agent.descriptor?.runtime?.runtime_mode ?? "--" },
      { label: labels.cluster, value: agent.descriptor?.runtime?.cluster_id ?? "--" },
      { label: labels.clusterSize, value: agent.descriptor?.runtime?.cluster_size ?? 1 },
      {
        label: labels.clusterHealth,
        value: agent.descriptor?.runtime?.health_score ?? "--",
        tone: clusterHealthTone(agent.descriptor?.runtime?.health_score),
      },
      { label: labels.peers, value: agent.descriptor?.runtime?.peers?.length ?? 0 },
      { label: labels.headless, value: agent.descriptor?.runtime?.headless ? labels.yes : labels.no },
      { label: labels.capabilities, value: agent.descriptor?.capabilities?.length ?? 0 },
      { label: labels.methods, value: agent.descriptor?.protocol?.methods?.length ?? 0 },
      {
        label: labels.execution,
        value: formatExecutionState(agent, labels),
        tone: executionStateTone(agent.execution_state, agent.active_lease?.is_stale),
      },
      {
        label: labels.leaseAge,
        value: formatLeaseAge(agent.active_lease?.age_ms, labels.leaseIdle),
        tone: agent.active_lease?.is_stale ? "stale" : undefined,
      },
    ],
    chips: [
      ...buildLeaseChips(agent, labels),
      ...(agent.descriptor?.capabilities?.flatMap((capability) =>
        capability.tags.slice(0, 3).map((tag) => ({
          key: `${agent.id}-${capability.id}-${tag}`,
          label: tag,
        })),
      ) ?? []),
      ...(agent.descriptor?.runtime?.peers?.slice(0, 2).map((peer) => ({
        key: `${agent.id}-${peer.address}`,
        label: peer.address,
        tone: clusterHealthTone(
          peer.status === "healthy" ? 100 : peer.status === "degraded" ? 65 : peer.status === "seed" ? 85 : 25,
        ),
        title: `${labels.peerState}: ${peerStatusLabel(peer.status)}`,
      })) ?? []),
    ],
    error: agent.descriptor_error,
  }));
}

function buildLeaseChips(
  agent: ProtocolAgentDescriptor,
  labels: {
    leaseStateChip: string;
    leaseAgeChip: string;
    leaseJobChip: string;
    leaseMethodChip: string;
    leaseActive: string;
    leaseStale: string;
    leaseIdle: string;
  },
) {
  if (!agent.active_lease) return [];

  return [
    {
      key: `${agent.id}-lease-state`,
      label: `${labels.leaseStateChip}: ${agent.active_lease.is_stale ? labels.leaseStale : labels.leaseActive}`,
      tone: agent.active_lease.is_stale ? "stale" : "watch",
    },
    ...(typeof agent.active_lease.age_ms === "number"
      ? [
          {
            key: `${agent.id}-lease-age`,
            label: `${labels.leaseAgeChip}: ${formatLeaseAge(agent.active_lease.age_ms, labels.leaseIdle)}`,
            tone: agent.active_lease.is_stale ? "stale" : "quiet",
          },
        ]
      : []),
    ...(agent.active_lease.job_id
      ? [
          {
            key: `${agent.id}-lease-job`,
            label: `${labels.leaseJobChip}: ${agent.active_lease.job_id.slice(0, 8)}`,
            tone: "quiet",
            title: agent.active_lease.job_id,
          },
        ]
      : []),
    ...(agent.active_lease.method
      ? [
          {
            key: `${agent.id}-lease-method`,
            label: `${labels.leaseMethodChip}: ${agent.active_lease.method}`,
            tone: agent.active_lease.is_stale ? "stale" : "quiet",
          },
        ]
      : []),
  ];
}

function formatExecutionState(
  agent: ProtocolAgentDescriptor,
  labels: {
    leaseIdle: string;
    leaseActive: string;
    leaseStale: string;
    leaseUnknown: string;
  },
) {
  if (agent.active_lease?.is_stale || agent.execution_state === "lease_stale") {
    return labels.leaseStale;
  }
  if (agent.execution_state === "leased") return labels.leaseActive;
  if (agent.execution_state === "idle") return labels.leaseIdle;
  return agent.active_lease ? labels.leaseActive : labels.leaseUnknown;
}

function formatLeaseAge(ageMs: number | null | undefined, idleLabel: string) {
  if (typeof ageMs !== "number" || ageMs < 0) return idleLabel;
  if (ageMs < 1_000) return `${ageMs} ms`;

  const seconds = Math.round(ageMs / 1_000);
  if (seconds < 60) return `${seconds}s`;

  const minutes = Math.floor(seconds / 60);
  const remainSeconds = seconds % 60;
  return remainSeconds > 0 ? `${minutes}m ${remainSeconds}s` : `${minutes}m`;
}

function executionStateTone(
  executionState: ProtocolAgentDescriptor["execution_state"],
  isStale: boolean | null | undefined,
) {
  if (isStale || executionState === "lease_stale") return "stale";
  if (executionState === "leased") return "watch";
  return "quiet";
}
