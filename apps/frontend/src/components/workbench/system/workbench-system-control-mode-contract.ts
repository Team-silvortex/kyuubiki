"use client";

import type { DirectMeshSelectionMode, FrontendRuntimeMode, ProtocolAgentDescriptor } from "@/lib/api";
import type { WorkbenchControlGroupSummary } from "@/components/workbench/system/workbench-system-control-groups-helpers";
import { controlGroupsFromMeshClusters, summarizeWorkbenchDirectGroups, summarizeWorkbenchOrchestratedGroups } from "@/components/workbench/system/workbench-system-control-groups-helpers";
import type { WorkbenchMeshClusterSummary } from "@/components/workbench/system/workbench-system-mesh-topology-helpers";
import { summarizeWorkbenchMeshClusters } from "@/components/workbench/system/workbench-system-mesh-topology-helpers";
import { controlWindowModeForAuthority, countWorkbenchControlEndpoints, countWorkbenchControlPeers, formatWorkbenchControlAgeLabel, pickWorkbenchControlEntryAgentId, summarizeWorkbenchPeerHealth, summarizeWorkbenchPeerObservability } from "@/components/workbench/system/workbench-system-control-runtime-helpers";
import {
  buildWorkbenchGovernanceEnforcementPlan,
  buildWorkbenchGovernanceRuntimeDiagnostics,
  resolveWorkbenchAuthorityMode,
} from "@/lib/workbench/governance";

type ControlWindowMode = "orchestrated" | "direct" | "mesh";
type ControlLanguage = string;

export type WorkbenchSystemControlModeCopy = {
  pageLabel: string;
  title: string;
  modeLabel: string;
  exportSnapshotLabel: string;
  snapshotVersionLabel: string;
  snapshotObservedAtLabel: string;
  activeRuntimeModeLabel: string;
  topologyWindowLabel: string;
  topologyWindowHint: string;
  tabs: Record<ControlWindowMode, string>;
  windows: Record<ControlWindowMode, { title: string; hint: string }>;
  rows: {
    currentRuntimeLabel: string;
    directStrategyLabel: string;
    endpointCountLabel: string;
    agentCountLabel: string;
    auditCountLabel: string;
    protocolStatusLabel: string;
    securityStatusLabel: string;
    meshEntryLabel: string;
    meshEntryHealthLabel: string;
    meshPeersLabel: string;
    meshGraphLabel: string;
    meshClusterCountLabel: string;
    meshRelayCandidateCountLabel: string;
    meshUnclusteredCountLabel: string;
    groupCountLabel: string;
    groupSummaryLabel: string;
    groupEntryLabel: string;
    groupSessionsLabel: string;
    meshRouteTraceLabel: string;
    meshLastSeenLabel: string;
    meshHopLabel: string;
    meshRoutingLabel: string;
    meshFallbackLabel: string;
    meshFailoverReasonLabel: string;
    safeModeLabel: string;
    downgradeReasonLabel: string;
  };
  meshPlannedHint: string;
  statuses: { online: string; offline: string; ready: string; open: string };
  runtimeLabels: Record<FrontendRuntimeMode, string>;
  directStrategyLabels: Record<DirectMeshSelectionMode, string>;
};

export type WorkbenchSystemControlTopologySummary = {
  mode: ControlWindowMode;
  authorityMode: "single_orchestrator" | "offline_mesh";
  entryAgentId: string;
  entryHealthLabel: string;
  peerCount: number;
  meshClusterCount: number;
  meshRelayCandidateCount: number;
  meshUnclusteredCount: number;
  graphSummaryLabel: string;
  routeTraceLabel: string;
  lastSeenLabel: string;
  estimatedHopCount: number;
  endpointCount: number;
  visibleAgentCount: number;
  auditCount: number;
  protocolOnline: boolean;
  securityConfigured: boolean;
  routingPolicy: string;
  fallbackPolicy: string;
  failoverReason: string;
  safeModeActive: boolean;
  downgradeReason: string;
  runtimeLabel: string;
  directStrategyLabel: string;
  meshClusters: WorkbenchMeshClusterSummary[];
  controlGroups: WorkbenchControlGroupSummary[];
};

export type WorkbenchSystemTopologySnapshot = {
  schema: {
    name: "kyuubiki.mesh-topology-snapshot";
    version: 1;
  };
  observed_at: string;
  runtime_mode: FrontendRuntimeMode;
  control_mode: ControlWindowMode;
  entry_agent_id: string;
  entry_health_score: number | null;
  direct_mesh_strategy: DirectMeshSelectionMode;
  endpoint_count: number;
  visible_agent_count: number;
  peer_count: number;
  estimated_hop_count: number;
  route_trace: string;
  graph_summary: string;
  last_seen_age_label: string;
  routing_policy: string;
  fallback_policy: string;
  failover_reason: string;
  safe_mode_active: boolean;
  downgrade_reason: string;
  mesh_cluster_count: number;
  mesh_relay_candidate_count: number;
  mesh_unclustered_count: number;
  mesh_clusters: Array<{
    cluster_id: string;
    agent_count: number;
    relay_candidate_count: number;
    peer_count: number;
    average_health_score: number | null;
    entry_agent_id: string;
  }>;
  control_groups: Array<{
    id: string;
    kind: "orchestrated" | "direct" | "mesh";
    agent_count: number;
    relay_candidate_count: number;
    peer_count: number;
    average_health_score: number | null;
    entry_agent_id: string;
    session_count: number;
  }>;
  agents: Array<{
    id: string;
    endpoint: string;
    health_score: number | null;
    peer_count: number;
    peer_failures: number;
    latest_peer_seen_unix_s: number | null;
  }>;
};

export type WorkbenchSystemTopologySnapshotSource =
  | { kind: "derived_frontend"; label: string }
  | { kind: "imported_snapshot"; label: string; observedAt: string };

function localizedRecord<T>(language: ControlLanguage, values: { zh: T; ja: T; en: T; es?: T }) {
  if (language === "zh") return values.zh;
  if (language === "ja") return values.ja;
  if (language === "es" && values.es !== undefined) return values.es;
  return values.en;
}

export function buildWorkbenchSystemControlModeCopy(
  language: ControlLanguage,
  frontendRuntimeMode: FrontendRuntimeMode,
): WorkbenchSystemControlModeCopy {
  const runtimeLabels = {
    orchestrated_gui: localizedRecord(language, { zh: "中心调度 GUI", ja: "オーケストレーション GUI", en: "Orchestrated GUI" }),
    direct_mesh_gui: localizedRecord(language, { zh: "直连 Mesh GUI", ja: "ダイレクト mesh GUI", en: "Direct mesh GUI" }),
  } satisfies Record<FrontendRuntimeMode, string>;

  return {
    pageLabel: localizedRecord(language, { zh: "控制窗", ja: "制御ウィンドウ", en: "Control windows" }),
    title: localizedRecord(language, { zh: "控制模式窗口", ja: "制御モードウィンドウ", en: "Control mode windows" }),
    modeLabel: localizedRecord(language, { zh: "控制模式", ja: "制御モード", en: "Control mode" }),
    exportSnapshotLabel: localizedRecord(language, { zh: "导出拓扑快照", ja: "トポロジースナップショットを書き出す", en: "Export topology snapshot" }),
    snapshotVersionLabel: localizedRecord(language, { zh: "快照版本", ja: "スナップショット版", en: "Snapshot version" }),
    snapshotObservedAtLabel: localizedRecord(language, { zh: "观测时间", ja: "観測時刻", en: "Observed at" }),
    activeRuntimeModeLabel:
      frontendRuntimeMode === "direct_mesh_gui"
        ? localizedRecord(language, { zh: "当前直连", ja: "現在は direct", en: "Direct active" })
        : localizedRecord(language, { zh: "当前中心调度", ja: "現在は orchestration", en: "Orchestrated active" }),
    topologyWindowLabel: localizedRecord(language, { zh: "拓扑窗口", ja: "トポロジーウィンドウ", en: "Topology window" }),
    topologyWindowHint: localizedRecord(language, {
      zh: "不同控制拓扑可以挂不同的嵌入式窗口壳，不必共用一套面板语义。",
      ja: "制御トポロジごとに別々の埋め込みウィンドウを持たせ、同じパネル意味論に縛られないようにします。",
      en: "Each control topology can own its own embedded window shell instead of sharing a single panel contract.",
    }),
    tabs: {
      orchestrated: localizedRecord(language, { zh: "中心调度", ja: "オーケストレーション", en: "Orchestrated" }),
      direct: localizedRecord(language, { zh: "直连 Agent", ja: "ダイレクト agent", en: "Direct agent" }),
      mesh: localizedRecord(language, { zh: "去中心 Mesh", ja: "分散 mesh", en: "Decentralized mesh" }),
    },
    windows: {
      orchestrated: {
        title: localizedRecord(language, { zh: "中心调度窗", ja: "オーケストレーション窓", en: "Orchestrated window" }),
        hint: localizedRecord(language, { zh: "适合统一调度、审计、权限和集中路由。", ja: "集中スケジューリング、監査、権限、集約ルーティング向けです。", en: "Best for centralized scheduling, audit, policy, and routed dispatch." }),
      },
      direct: {
        title: localizedRecord(language, { zh: "直连 Agent 窗", ja: "ダイレクト agent 窓", en: "Direct agent window" }),
        hint: localizedRecord(language, { zh: "控制面可以直接落到 agent 或一组 agent，绕过 orchestra。", ja: "制御面は orchestra を経由せずに agent または agent 群へ直接降ろせます。", en: "Lets the control plane land directly on one or more agents without going through the orchestrator." }),
      },
      mesh: {
        title: localizedRecord(language, { zh: "去中心 Mesh 窗", ja: "分散 mesh 窓", en: "Decentralized mesh window" }),
        hint: localizedRecord(language, { zh: "保留给任一入口 agent 接管传播、转发和去中心协同的控制模式。", ja: "任意の入口 agent が伝播・転送・分散協調を担う制御モードのための予約領域です。", en: "Reserved for entry-agent propagation, forwarding, and decentralized coordination." }),
      },
    },
    rows: {
      currentRuntimeLabel: localizedRecord(language, { zh: "当前运行入口", ja: "現在の実行入口", en: "Current runtime entry" }),
      directStrategyLabel: localizedRecord(language, { zh: "直连选择策略", ja: "ダイレクト選択戦略", en: "Direct selection strategy" }),
      endpointCountLabel: localizedRecord(language, { zh: "端点数", ja: "エンドポイント数", en: "Endpoint count" }),
      agentCountLabel: localizedRecord(language, { zh: "可见 agent", ja: "可視 agent", en: "Visible agents" }),
      auditCountLabel: localizedRecord(language, { zh: "审计事件", ja: "監査イベント", en: "Audit events" }),
      protocolStatusLabel: localizedRecord(language, { zh: "协议状态", ja: "プロトコル状態", en: "Protocol status" }),
      securityStatusLabel: localizedRecord(language, { zh: "安全入口", ja: "セキュリティ入口", en: "Security gate" }),
      meshEntryLabel: localizedRecord(language, { zh: "入口 agent", ja: "入口 agent", en: "Entry agent" }),
      meshEntryHealthLabel: localizedRecord(language, { zh: "入口健康度", ja: "入口ヘルス", en: "Entry health" }),
      meshPeersLabel: localizedRecord(language, { zh: "可传播 peers", ja: "伝播可能 peers", en: "Propagating peers" }),
      meshGraphLabel: localizedRecord(language, { zh: "Mesh 图摘要", ja: "mesh グラフ要約", en: "Mesh graph" }),
      meshClusterCountLabel: localizedRecord(language, { zh: "Mesh 集群", ja: "mesh クラスタ", en: "Mesh clusters" }),
      meshRelayCandidateCountLabel: localizedRecord(language, { zh: "Relay 候选", ja: "relay 候補", en: "Relay candidates" }),
      meshUnclusteredCountLabel: localizedRecord(language, { zh: "未分组节点", ja: "未分類ノード", en: "Unclustered nodes" }),
      groupCountLabel: localizedRecord(language, { zh: "控制分组", ja: "制御グループ", en: "Control groups" }),
      groupSummaryLabel: localizedRecord(language, { zh: "分组概览", ja: "グループ概要", en: "Group overview" }),
      groupEntryLabel: localizedRecord(language, { zh: "入口", ja: "入口", en: "Entry" }),
      groupSessionsLabel: localizedRecord(language, { zh: "会话数", ja: "セッション数", en: "Sessions" }),
      meshRouteTraceLabel: localizedRecord(language, { zh: "路由轨迹", ja: "ルート軌跡", en: "Route trace" }),
      meshLastSeenLabel: localizedRecord(language, { zh: "最近观测", ja: "最終観測", en: "Last seen" }),
      meshHopLabel: localizedRecord(language, { zh: "估计 hop", ja: "推定 hop", en: "Estimated hops" }),
      meshRoutingLabel: localizedRecord(language, { zh: "路由策略", ja: "ルーティング方針", en: "Routing policy" }),
      meshFallbackLabel: localizedRecord(language, { zh: "回退路径", ja: "フォールバック経路", en: "Fallback path" }),
      meshFailoverReasonLabel: localizedRecord(language, { zh: "回退原因", ja: "フォールバック理由", en: "Failover reason" }),
      safeModeLabel: localizedRecord(language, { zh: "安全模式", ja: "セーフモード", en: "Safe mode" }),
      downgradeReasonLabel: localizedRecord(language, { zh: "降级原因", ja: "降格理由", en: "Downgrade reason" }),
    },
    meshPlannedHint: localizedRecord(language, {
      zh: "这扇窗先作为去中心控制面的最小契约摘要，后面再接真实的 entry-agent、传播路由和失败转移状态。",
      ja: "この窓はまず分散制御面の最小契約サマリとして置き、後で実際の entry-agent・伝播経路・障害時フェイルオーバー状態を接続します。",
      en: "This window starts as a minimal contract summary for decentralized control; real entry-agent, propagation routing, and failover state can be attached later.",
    }),
    statuses: {
      online: localizedRecord(language, { zh: "在线", ja: "オンライン", en: "online" }),
      offline: localizedRecord(language, { zh: "离线", ja: "オフライン", en: "offline" }),
      ready: localizedRecord(language, { zh: "就绪", ja: "準備完了", en: "ready" }),
      open: localizedRecord(language, { zh: "开放", ja: "オープン", en: "open" }),
    },
    runtimeLabels,
    directStrategyLabels: {
      healthiest: localizedRecord(language, { zh: "优先健康节点", ja: "健全ノード優先", en: "Healthiest agent" }),
      first_reachable: localizedRecord(language, { zh: "首个可达节点", ja: "最初の到達可能ノード", en: "First reachable" }),
    },
  };
}

export function buildWorkbenchSystemControlTopologySummary(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshSelectionMode: DirectMeshSelectionMode;
  directMeshEndpointsText: string;
  protocolAgents: readonly ProtocolAgentDescriptor[];
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
  protocolOnline: boolean;
  securityConfigured: boolean;
  auditCount: number;
  copy: WorkbenchSystemControlModeCopy;
  nowUnixS?: number;
}): WorkbenchSystemControlTopologySummary {
  const governanceDiagnostics = buildWorkbenchGovernanceRuntimeDiagnostics({
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
    protocolAgents: input.protocolAgents,
    controlPlaneApiToken: input.controlPlaneApiToken,
    clusterApiToken: input.clusterApiToken,
    directMeshApiToken: input.directMeshApiToken,
  });
  const governanceEnforcement = buildWorkbenchGovernanceEnforcementPlan({
    frontendRuntimeMode: input.frontendRuntimeMode,
    diagnostics: governanceDiagnostics,
  });
  const authorityMode = resolveWorkbenchAuthorityMode({
    frontendRuntimeMode: input.frontendRuntimeMode,
    protocolAgents: input.protocolAgents,
  });
  const mode = controlWindowModeForAuthority(authorityMode, input.protocolAgents.length);
  const nowUnixS = input.nowUnixS ?? Math.floor(Date.now() / 1000);
  const entryAgentId = pickWorkbenchControlEntryAgentId(input.protocolAgents, input.directMeshSelectionMode, input.frontendRuntimeMode);
  const entryAgent = input.protocolAgents.find((agent) => agent.id === entryAgentId);
  const entryHealthScore = entryAgent?.descriptor?.runtime.health_score ?? null;
  const peerHealth = summarizeWorkbenchPeerHealth(input.protocolAgents);
  const peerObservability = summarizeWorkbenchPeerObservability(input.protocolAgents, nowUnixS);
  const peerCount = countWorkbenchControlPeers(input.protocolAgents);
  const meshSummary = summarizeWorkbenchMeshClusters(input.protocolAgents);
  const orchestratedGroups = summarizeWorkbenchOrchestratedGroups(input.protocolAgents);
  const directGroups = summarizeWorkbenchDirectGroups(input.protocolAgents, input.frontendRuntimeMode);
  const meshGroups = controlGroupsFromMeshClusters(meshSummary.clusters);
  const estimatedHopCount = peerCount > 0 ? Math.min(3, Math.max(1, Math.ceil(peerCount / Math.max(input.protocolAgents.length || 1, 1)))) : 0;
  const graphSummaryLabel = `${peerHealth.healthy}/${peerCount || 0} healthy · ${peerObservability.totalFailureCount} failures`;
  const routeTraceLabel =
    peerCount === 0
      ? `${entryAgentId} -> isolated`
      : estimatedHopCount <= 1
        ? `${entryAgentId} -> peers`
        : `${entryAgentId} -> relay -> peers`;
  const failoverReason =
    !input.protocolOnline
      ? input.copy.statuses.offline
      : input.protocolAgents.length === 0
        ? input.copy.tabs.direct
        : peerObservability.totalFailureCount > 0
          ? `${peerObservability.totalFailureCount} peer failures`
        : entryHealthScore !== null && entryHealthScore < 0.5
          ? input.copy.directStrategyLabels.healthiest
          : input.copy.tabs.orchestrated;
  return {
    mode,
    authorityMode,
    entryAgentId,
    entryHealthLabel:
      entryHealthScore === null
        ? "--"
        : `${Math.round(entryHealthScore * 100)}%`,
    peerCount,
    meshClusterCount: meshSummary.clusterCount,
    meshRelayCandidateCount: meshSummary.relayCandidateCount,
    meshUnclusteredCount: meshSummary.unclusteredCount,
    graphSummaryLabel,
    routeTraceLabel,
    lastSeenLabel: formatWorkbenchControlAgeLabel(peerObservability.ageSeconds),
    estimatedHopCount,
    endpointCount: countWorkbenchControlEndpoints(input.directMeshEndpointsText),
    visibleAgentCount: input.protocolAgents.length,
    auditCount: input.auditCount,
    protocolOnline: input.protocolOnline,
    securityConfigured: input.securityConfigured,
    routingPolicy:
      authorityMode === "single_orchestrator"
        ? "single_orchestrator"
        : mode === "mesh"
          ? "offline_mesh relay"
          : "offline_mesh direct",
    fallbackPolicy:
      authorityMode === "single_orchestrator"
        ? input.copy.tabs.direct
        : input.protocolOnline
          ? input.copy.tabs.orchestrated
          : input.copy.tabs.direct,
    failoverReason,
    safeModeActive: governanceEnforcement.shouldDowngrade,
    downgradeReason: governanceEnforcement.reason ?? governanceDiagnostics.driftLabel,
    runtimeLabel: input.copy.runtimeLabels[input.frontendRuntimeMode],
    directStrategyLabel: input.copy.directStrategyLabels[input.directMeshSelectionMode],
    meshClusters: meshSummary.clusters,
    controlGroups:
      mode === "orchestrated"
        ? orchestratedGroups
        : mode === "direct"
          ? directGroups
          : meshGroups,
  };
}

export function buildWorkbenchSystemTopologySnapshot(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshSelectionMode: DirectMeshSelectionMode;
  directMeshEndpointsText: string;
  protocolAgents: readonly ProtocolAgentDescriptor[];
  topology: WorkbenchSystemControlTopologySummary;
  observedAt?: Date;
}): WorkbenchSystemTopologySnapshot {
  const observedAt = input.observedAt ?? new Date();
  return {
    schema: {
      name: "kyuubiki.mesh-topology-snapshot",
      version: 1,
    },
    observed_at: observedAt.toISOString(),
    runtime_mode: input.frontendRuntimeMode,
    control_mode: input.topology.mode,
    entry_agent_id: input.topology.entryAgentId,
    entry_health_score:
      input.protocolAgents.find((agent) => agent.id === input.topology.entryAgentId)?.descriptor?.runtime.health_score ?? null,
    direct_mesh_strategy: input.directMeshSelectionMode,
    endpoint_count: countWorkbenchControlEndpoints(input.directMeshEndpointsText),
    visible_agent_count: input.topology.visibleAgentCount,
    peer_count: input.topology.peerCount,
    estimated_hop_count: input.topology.estimatedHopCount,
    route_trace: input.topology.routeTraceLabel,
    graph_summary: input.topology.graphSummaryLabel,
    last_seen_age_label: input.topology.lastSeenLabel,
    routing_policy: input.topology.routingPolicy,
    fallback_policy: input.topology.fallbackPolicy,
    failover_reason: input.topology.failoverReason,
    safe_mode_active: input.topology.safeModeActive,
    downgrade_reason: input.topology.downgradeReason,
    mesh_cluster_count: input.topology.meshClusterCount,
    mesh_relay_candidate_count: input.topology.meshRelayCandidateCount,
    mesh_unclustered_count: input.topology.meshUnclusteredCount,
    mesh_clusters: input.topology.meshClusters.map((cluster) => ({
      cluster_id: cluster.clusterId,
      agent_count: cluster.agentCount,
      relay_candidate_count: cluster.relayCandidateCount,
      peer_count: cluster.peerCount,
      average_health_score: cluster.averageHealthScore,
      entry_agent_id: cluster.entryAgentId,
    })),
    control_groups: input.topology.controlGroups.map((group) => ({
      id: group.id,
      kind: group.kind,
      agent_count: group.agentCount,
      relay_candidate_count: group.relayCandidateCount,
      peer_count: group.peerCount,
      average_health_score: group.averageHealthScore,
      entry_agent_id: group.entryAgentId,
      session_count: group.sessionCount,
    })),
    agents: input.protocolAgents.map((agent) => {
      const peers = agent.descriptor?.runtime.peers ?? [];
      const latestPeerSeen = peers.reduce<number | null>((latest, peer) => {
        if (typeof peer.last_seen_unix_s !== "number") return latest;
        return latest === null ? peer.last_seen_unix_s : Math.max(latest, peer.last_seen_unix_s);
      }, null);
      return {
        id: agent.id,
        endpoint: `${agent.host}:${agent.port}`,
        health_score: agent.descriptor?.runtime.health_score ?? null,
        peer_count: peers.length,
        peer_failures: peers.reduce((sum, peer) => sum + (peer.failure_count ?? 0), 0),
        latest_peer_seen_unix_s: latestPeerSeen,
      };
    }),
  };
}

export function parseWorkbenchSystemTopologySnapshot(value: unknown): WorkbenchSystemTopologySnapshot | null {
  if (!value || typeof value !== "object") return null;
  const record = value as Record<string, unknown>;
  const schema = record.schema as Record<string, unknown> | undefined;
  if (schema?.name !== "kyuubiki.mesh-topology-snapshot" || schema?.version !== 1) return null;
  if (typeof record.observed_at !== "string") return null;
  if (record.runtime_mode !== "orchestrated_gui" && record.runtime_mode !== "direct_mesh_gui") return null;
  if (record.control_mode !== "orchestrated" && record.control_mode !== "direct" && record.control_mode !== "mesh") return null;
  if (!Array.isArray(record.agents)) return null;

  return {
    schema: { name: "kyuubiki.mesh-topology-snapshot", version: 1 },
    observed_at: record.observed_at,
    runtime_mode: record.runtime_mode,
    control_mode: record.control_mode,
    entry_agent_id: typeof record.entry_agent_id === "string" ? record.entry_agent_id : "unknown",
    entry_health_score: typeof record.entry_health_score === "number" ? record.entry_health_score : null,
    direct_mesh_strategy: record.direct_mesh_strategy === "first_reachable" ? "first_reachable" : "healthiest",
    endpoint_count: typeof record.endpoint_count === "number" ? record.endpoint_count : 0,
    visible_agent_count: typeof record.visible_agent_count === "number" ? record.visible_agent_count : 0,
    peer_count: typeof record.peer_count === "number" ? record.peer_count : 0,
    estimated_hop_count: typeof record.estimated_hop_count === "number" ? record.estimated_hop_count : 0,
    route_trace: typeof record.route_trace === "string" ? record.route_trace : "--",
    graph_summary: typeof record.graph_summary === "string" ? record.graph_summary : "--",
    last_seen_age_label: typeof record.last_seen_age_label === "string" ? record.last_seen_age_label : "--",
    routing_policy: typeof record.routing_policy === "string" ? record.routing_policy : "--",
    fallback_policy: typeof record.fallback_policy === "string" ? record.fallback_policy : "--",
    failover_reason: typeof record.failover_reason === "string" ? record.failover_reason : "--",
    safe_mode_active: record.safe_mode_active === true,
    downgrade_reason: typeof record.downgrade_reason === "string" ? record.downgrade_reason : "--",
    mesh_cluster_count: typeof record.mesh_cluster_count === "number" ? record.mesh_cluster_count : 0,
    mesh_relay_candidate_count: typeof record.mesh_relay_candidate_count === "number" ? record.mesh_relay_candidate_count : 0,
    mesh_unclustered_count: typeof record.mesh_unclustered_count === "number" ? record.mesh_unclustered_count : 0,
    mesh_clusters: Array.isArray(record.mesh_clusters)
      ? record.mesh_clusters.flatMap((entry) => {
          if (!entry || typeof entry !== "object") return [];
          const cluster = entry as Record<string, unknown>;
          return [{
            cluster_id: typeof cluster.cluster_id === "string" ? cluster.cluster_id : "unknown",
            agent_count: typeof cluster.agent_count === "number" ? cluster.agent_count : 0,
            relay_candidate_count: typeof cluster.relay_candidate_count === "number" ? cluster.relay_candidate_count : 0,
            peer_count: typeof cluster.peer_count === "number" ? cluster.peer_count : 0,
            average_health_score: typeof cluster.average_health_score === "number" ? cluster.average_health_score : null,
            entry_agent_id: typeof cluster.entry_agent_id === "string" ? cluster.entry_agent_id : "unknown",
          }];
        })
      : [],
    control_groups: Array.isArray(record.control_groups)
      ? record.control_groups.flatMap((entry) => {
          if (!entry || typeof entry !== "object") return [];
          const group = entry as Record<string, unknown>;
          if (group.kind !== "orchestrated" && group.kind !== "direct" && group.kind !== "mesh") return [];
          return [{
            id: typeof group.id === "string" ? group.id : "unknown",
            kind: group.kind,
            agent_count: typeof group.agent_count === "number" ? group.agent_count : 0,
            relay_candidate_count: typeof group.relay_candidate_count === "number" ? group.relay_candidate_count : 0,
            peer_count: typeof group.peer_count === "number" ? group.peer_count : 0,
            average_health_score: typeof group.average_health_score === "number" ? group.average_health_score : null,
            entry_agent_id: typeof group.entry_agent_id === "string" ? group.entry_agent_id : "unknown",
            session_count: typeof group.session_count === "number" ? group.session_count : 0,
          }];
        })
      : [],
    agents: record.agents.flatMap((entry) => {
      if (!entry || typeof entry !== "object") return [];
      const agent = entry as Record<string, unknown>;
      return [{
        id: typeof agent.id === "string" ? agent.id : "unknown",
        endpoint: typeof agent.endpoint === "string" ? agent.endpoint : "unknown",
        health_score: typeof agent.health_score === "number" ? agent.health_score : null,
        peer_count: typeof agent.peer_count === "number" ? agent.peer_count : 0,
        peer_failures: typeof agent.peer_failures === "number" ? agent.peer_failures : 0,
        latest_peer_seen_unix_s: typeof agent.latest_peer_seen_unix_s === "number" ? agent.latest_peer_seen_unix_s : null,
      }];
    }),
  };
}

export function buildControlTopologySummaryFromSnapshot(
  snapshot: WorkbenchSystemTopologySnapshot,
  copy: WorkbenchSystemControlModeCopy,
): WorkbenchSystemControlTopologySummary {
  return {
    mode: snapshot.control_mode,
    authorityMode: snapshot.control_mode === "orchestrated" ? "single_orchestrator" : "offline_mesh",
    entryAgentId: snapshot.entry_agent_id,
    entryHealthLabel: snapshot.entry_health_score === null ? "--" : `${Math.round(snapshot.entry_health_score * 100)}%`,
    peerCount: snapshot.peer_count,
    graphSummaryLabel: snapshot.graph_summary,
    routeTraceLabel: snapshot.route_trace,
    lastSeenLabel: snapshot.last_seen_age_label,
    estimatedHopCount: snapshot.estimated_hop_count,
    endpointCount: snapshot.endpoint_count,
    visibleAgentCount: snapshot.visible_agent_count,
    auditCount: 0,
    protocolOnline: snapshot.runtime_mode === "orchestrated_gui",
    securityConfigured: true,
    routingPolicy: snapshot.routing_policy,
    fallbackPolicy: snapshot.fallback_policy,
    failoverReason: snapshot.failover_reason,
    safeModeActive: snapshot.safe_mode_active,
    downgradeReason: snapshot.downgrade_reason,
    runtimeLabel: copy.runtimeLabels[snapshot.runtime_mode],
    directStrategyLabel: copy.directStrategyLabels[snapshot.direct_mesh_strategy],
    meshClusterCount: snapshot.mesh_cluster_count,
    meshRelayCandidateCount: snapshot.mesh_relay_candidate_count,
    meshUnclusteredCount: snapshot.mesh_unclustered_count,
    meshClusters: snapshot.mesh_clusters.map((cluster) => ({
      clusterId: cluster.cluster_id,
      agentCount: cluster.agent_count,
      relayCandidateCount: cluster.relay_candidate_count,
      peerCount: cluster.peer_count,
      averageHealthScore: cluster.average_health_score,
      entryAgentId: cluster.entry_agent_id,
    })),
    controlGroups: snapshot.control_groups.map((group) => ({
      id: group.id,
      kind: group.kind,
      agentCount: group.agent_count,
      relayCandidateCount: group.relay_candidate_count,
      peerCount: group.peer_count,
      averageHealthScore: group.average_health_score,
      entryAgentId: group.entry_agent_id,
      sessionCount: group.session_count,
    })),
  };
}
