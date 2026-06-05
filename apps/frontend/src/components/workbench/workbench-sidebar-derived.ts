"use client";

import {
  buildAdminJobRows,
  buildAdminResultRows,
  buildLibraryJobRows,
  buildLibraryModelRows,
  buildLibrarySampleRows,
  buildLibraryVersionRows,
  buildProtocolAgentCards,
} from "@/lib/workbench/view-models";
import {
  buildRuntimeAuditModelVersionFacets,
  buildRuntimeAuditProjectFacets,
  buildRuntimeAuditSourceStatusFacets,
  buildRuntimeAuditStudyFacets,
  buildRuntimeAuditSummaryRows,
  buildRuntimeAuditTrendBars,
} from "@/components/workbench/workbench-runtime-audit-helpers";
import { clusterHealthTone, formatPeerStatus, formatProtocolMethodLabel, materialColorByIndex } from "@/components/workbench/workbench-result-helpers";
import { SAMPLE_LIBRARY } from "@/lib/models";

export function buildWorkbenchSidebarDerived(props: Record<string, any>) {
  const {
    t,
    language,
    health,
    frontendRuntimeMode,
    directMeshSelectionMode,
    directMeshExecution,
    directMeshApiToken,
    protocolAgents,
    securityUi,
    currentMaterials,
    hiddenMaterials,
    studyKind,
    deferredJobHistory,
    deferredResultRecords,
    deferredProjectModels,
    deferredModelVersions,
    adminFilterProjectId,
    adminFilterModelVersionId,
    jobHistory,
    projects,
    securityEventRecords,
    securityEventWindowFilter,
    formatTime,
    formatMilliseconds,
  } = props;

  const hiddenMaterialIds = hiddenMaterials[studyKind] ?? [];
  const materialColorMap = new Map(currentMaterials.map((material: any, index: number) => [material.id, materialColorByIndex(index)]));
  const materialOptions = currentMaterials.map((material: any) => ({
    id: material.id,
    label: `${material.name} (${props.round(material.youngs_modulus / 1.0e9)} GPa)`,
  }));

  const adminJobRows = buildAdminJobRows({
    jobs: deferredJobHistory.filter((job: any) => {
      const matchesProject =
        !adminFilterProjectId ||
        (job.project_id ?? "").toLowerCase().includes(adminFilterProjectId.trim().toLowerCase());
      const matchesVersion =
        !adminFilterModelVersionId ||
        (job.model_version_id ?? "").toLowerCase().includes(adminFilterModelVersionId.trim().toLowerCase());
      return matchesProject && matchesVersion;
    }),
    heartbeatTone: (job: any) => props.heartbeatTone(job),
    heartbeatLabel: (job: any) => props.heartbeatStatus(job, t),
    detailLabel: (job: any) => props.humanizeSolverFailure(job.message, t) ?? job.message ?? job.worker_id ?? "--",
  });

  const adminResultRows = buildAdminResultRows({
    records: deferredResultRecords.filter((record: any) => {
      const linkedJob = jobHistory.find((job: any) => job.job_id === record.job_id);
      const matchesProject =
        !adminFilterProjectId ||
        (linkedJob?.project_id ?? "").toLowerCase().includes(adminFilterProjectId.trim().toLowerCase());
      const matchesVersion =
        !adminFilterModelVersionId ||
        (linkedJob?.model_version_id ?? "").toLowerCase().includes(adminFilterModelVersionId.trim().toLowerCase());
      return matchesProject && matchesVersion;
    }),
    jobs: jobHistory,
    updatedAtLabel: (record: any) => (record.updated_at ? formatTime(record.updated_at, language) : t.hasResult),
    summaryLabel: (record: any) => Object.keys(record.result).join(", ").slice(0, 64) || t.resultPayload,
  });

  const librarySampleRows = buildLibrarySampleRows({
    samples: SAMPLE_LIBRARY,
    kindLabel: (kind: string) => (kind in t.kinds ? t.kinds[kind] : kind),
    domainLabel: (domain: string) => t.studyDomains[domain],
    familyLabel: (family: string) => t.studyFamilies[family],
  });

  const libraryModelRows = buildLibraryModelRows({
    models: deferredProjectModels,
    kindLabel: (kind: string) => (kind in t.kinds ? t.kinds[kind] : kind),
    updatedAtLabel: (value?: string) => formatTime(value, language),
  });

  const libraryVersionRows = buildLibraryVersionRows({
    versions: deferredModelVersions,
    updatedAtLabel: (value?: string) => formatTime(value, language),
  });

  const libraryJobRows = buildLibraryJobRows({
    jobs: deferredJobHistory,
    updatedAtLabel: (value?: string) => formatTime(value, language),
    hasResultLabel: (hasResult: boolean) => (hasResult ? t.yes : t.no),
  });

  const protocolAgentCards = buildProtocolAgentCards({
    agents: protocolAgents,
    labels: {
      runtimeMode: t.runtimeMode,
      cluster: t.cluster,
      clusterSize: t.clusterSize,
      clusterHealth: t.clusterHealth,
      peers: t.peers,
      headless: t.headless,
      yes: t.yes,
      no: t.no,
      capabilities: t.capabilities,
      methods: t.methods,
      peerState: t.peerState,
    },
    clusterHealthTone,
    peerStatusLabel: (status?: string) => formatPeerStatus(status, t),
  });

  const runtimeBackendRows = [
    { label: t.ui, value: "3000" },
    { label: t.orchestrator, value: health ? "4000" : t.offline },
    { label: t.solverAgent, value: health?.transport?.solver_agent_tcp ?? 5001 },
  ];

  const runtimeProtocolRows = [
    { label: t.controlPlaneProtocol, value: health?.protocol?.protocol?.name ?? "--" },
    { label: t.solverRpcProtocol, value: health?.protocol?.compatible_solver_rpc?.name ?? "--" },
    { label: t.deploymentMode, value: health?.deployment?.mode ?? "--" },
    { label: t.discoveryMode, value: health?.deployment?.discovery ?? "--" },
    { label: t.registeredAgents, value: health?.remote_solver_registry?.active_agents ?? 0 },
    { label: t.reachableAgents, value: protocolAgents.length },
    ...(frontendRuntimeMode === "direct_mesh_gui"
      ? [
          { label: t.directMeshStrategy, value: t.directMeshStrategies[directMeshSelectionMode] },
          { label: t.directMeshLastAgent, value: directMeshExecution?.endpoint ?? "--" },
          {
            label: t.directMeshLastRoute,
            value: directMeshExecution
              ? `${t.directMeshStrategies[directMeshExecution.strategy]} · ${formatTime(directMeshExecution.at, language)}`
              : "--",
          },
        ]
      : []),
  ];

  const runtimeProtocolMethods = health?.protocol?.compatible_solver_rpc?.methods?.map((method: string) =>
    formatProtocolMethodLabel(method),
  );

  const runtimeSecurityRows = [
    {
      label: securityUi.controlPlaneToken,
      value: health?.security?.api_token_configured ? securityUi.configured : securityUi.notConfigured,
    },
    {
      label: securityUi.clusterToken,
      value: health?.security?.cluster_token_configured ? securityUi.configured : securityUi.notConfigured,
    },
    {
      label: securityUi.clusterWindow,
      value: `${health?.security?.cluster_timestamp_window_ms ?? 30000} ms`,
    },
    {
      label: language === "zh" ? "Agent 白名单" : language === "ja" ? "Agent 許可リスト" : "Agent allowlist",
      value: health?.security?.cluster_agent_allowlist_enabled
        ? `${securityUi.enabled} · ${health?.security?.cluster_agent_allowlist_count ?? 0}`
        : securityUi.disabled,
    },
    {
      label: language === "zh" ? "Cluster 白名单" : language === "ja" ? "Cluster 許可リスト" : "Cluster allowlist",
      value: health?.security?.cluster_cluster_allowlist_enabled
        ? `${securityUi.enabled} · ${health?.security?.cluster_cluster_allowlist_count ?? 0}`
        : securityUi.disabled,
    },
    {
      label: language === "zh" ? "Fingerprint 绑定" : language === "ja" ? "Fingerprint バインディング" : "Fingerprint binding",
      value: health?.security?.cluster_fingerprint_required ? securityUi.enabled : securityUi.disabled,
    },
    {
      label: securityUi.protectReads,
      value: health?.security?.protect_reads ? securityUi.enabled : securityUi.disabled,
    },
    {
      label: securityUi.mutatingRoutes,
      value: health?.security?.mutating_routes_protected ? securityUi.enabled : securityUi.disabled,
    },
    {
      label: securityUi.clusterRoutes,
      value: health?.security?.cluster_routes_protected ? securityUi.enabled : securityUi.disabled,
    },
    {
      label: securityUi.directMeshRoutes,
      value: directMeshApiToken ? securityUi.configured : securityUi.enabled,
    },
  ];

  const runtimeAuditEntries = securityEventRecords.map((entry: any) => ({
    id: entry.event_id,
    at: formatTime(entry.occurred_at, language),
    action: entry.action,
    source:
      entry.source === "assistant"
        ? language === "zh"
          ? "助手"
          : language === "ja"
            ? "アシスタント"
            : "Assistant"
        : language === "zh"
          ? "脚本"
          : language === "ja"
            ? "スクリプト"
            : "Script",
    risk:
      entry.risk === "destructive"
        ? language === "zh"
          ? "高风险"
          : language === "ja"
            ? "破壊的"
            : "Destructive"
        : language === "zh"
          ? "敏感"
          : language === "ja"
            ? "機微"
            : "Sensitive",
    status:
      entry.status === "prompted"
        ? language === "zh"
          ? "待确认"
          : language === "ja"
            ? "確認待ち"
            : "Prompted"
        : entry.status === "cancelled"
          ? language === "zh"
            ? "已取消"
            : language === "ja"
              ? "取消済み"
              : "Cancelled"
          : entry.status === "completed"
            ? language === "zh"
              ? "已执行"
              : language === "ja"
                ? "完了"
                : "Completed"
            : language === "zh"
              ? "失败"
              : language === "ja"
                ? "失敗"
                : "Failed",
    note: entry.note ?? "--",
  }));

  const runtimeAuditSummaryRows = buildRuntimeAuditSummaryRows(language, securityEventRecords);
  const runtimeAuditTrendBars = buildRuntimeAuditTrendBars(language, securityEventRecords, props.securityEventWindowFilter);
  const runtimeAuditSourceStatusFacets = buildRuntimeAuditSourceStatusFacets(language, securityEventRecords);
  const runtimeAuditStudyFacets = buildRuntimeAuditStudyFacets(securityEventRecords);
  const runtimeAuditProjectFacets = buildRuntimeAuditProjectFacets(securityEventRecords);
  const runtimeAuditModelVersionFacets = buildRuntimeAuditModelVersionFacets(securityEventRecords);
  const runtimeWatchdogRows = [
    { label: t.activeJobs, value: health?.watchdog?.active_jobs ?? 0 },
    { label: t.stalledJobs, value: health?.watchdog?.stalled_jobs ?? 0 },
    { label: t.timedOutJobs, value: health?.watchdog?.timed_out_jobs ?? 0 },
    { label: t.scanEvery, value: formatMilliseconds(health?.watchdog?.scan_interval_ms) },
    { label: t.staleAfter, value: formatMilliseconds(health?.watchdog?.stale_job_ms) },
    { label: t.timeoutAfter, value: formatMilliseconds(health?.watchdog?.job_timeout_ms) },
  ];

  return {
    hiddenMaterialIds,
    materialColorMap,
    materialOptions,
    adminJobRows,
    adminResultRows,
    librarySampleRows,
    libraryModelRows,
    libraryVersionRows,
    libraryJobRows,
    protocolAgentCards,
    runtimeBackendRows,
    runtimeProtocolRows,
    runtimeProtocolMethods,
    runtimeSecurityRows,
    runtimeAuditEntries,
    runtimeAuditSummaryRows,
    runtimeAuditTrendBars,
    runtimeAuditSourceStatusFacets,
    runtimeAuditStudyFacets,
    runtimeAuditProjectFacets,
    runtimeAuditModelVersionFacets,
    runtimeWatchdogRows,
  };
}
