"use client";

import { resolveJobStatusDetailLabel } from "@/lib/api";
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
import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import { getWorkbenchRuntimeAuditCopy } from "@/components/workbench/workbench-extended-language-copy";
import { SAMPLE_LIBRARY } from "@/lib/models";
import { buildWorkbenchGovernanceRuntimeDiagnostics } from "@/lib/workbench/governance";

function localizeGovernanceRuntimeValue(
  value: string,
  copy: WorkbenchCopy,
  securityUi: { configured: string; notConfigured: string; clusterToken: string; controlPlaneToken: string },
) {
  const knownValues: Record<string, string> = {
    "direct-mesh authority": `${copy.frontendModes.direct_mesh_gui} · ${securityUi.configured}`,
    "direct-mesh authority missing token": `${copy.frontendModes.direct_mesh_gui} · ${securityUi.notConfigured}`,
    "single orchestrator authority via cluster token": `${copy.frontendModes.orchestrated_gui} · ${securityUi.clusterToken}`,
    "single orchestrator authority via control-plane token": `${copy.frontendModes.orchestrated_gui} · ${securityUi.controlPlaneToken}`,
    "orchestrator authority missing token": `${copy.frontendModes.orchestrated_gui} · ${securityUi.notConfigured}`,
    "single cluster scope": `${copy.cluster} · 1`,
    "direct mesh missing endpoints": `${copy.directMeshEndpoints} · ${securityUi.notConfigured}`,
    "multi-cluster exposure": `${copy.cluster} · ${copy.stabilityWatch}`,
    "mixed runtime modes": `${copy.runtimeMode} · ${copy.stabilityWatch}`,
    aligned: copy.ready,
  };
  const visibleClusters = value.match(/^(\d+) clusters visible$/);
  return visibleClusters ? `${copy.cluster} · ${visibleClusters[1]}` : knownValues[value] ?? value;
}

export function buildWorkbenchSidebarDerived(props: Record<string, any>) {
  const {
    t,
    language,
    health,
    frontendRuntimeMode,
    directMeshSelectionMode,
    directMeshExecution,
    controlPlaneApiToken,
    clusterApiToken,
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
    detailLabel: (job: any) =>
      props.humanizeSolverFailure(job.message, t) ??
      resolveJobStatusDetailLabel(job.status_detail) ??
      job.message ??
      job.worker_id ??
      "--",
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
      authorityMode: `${t.controls} · ${t.runtimeMode}`,
      controlMode: `${t.controls} · ${t.status}`,
      runtimeMode: t.runtimeMode,
      cluster: t.cluster,
      meshGroup: `${t.mesh} · ${t.cluster}`,
      clusterSize: t.clusterSize,
      clusterHealth: t.clusterHealth,
      peers: t.peers,
      relay: `${t.mesh} · relay`,
      headless: t.headless,
      yes: t.yes,
      no: t.no,
      capabilities: t.capabilities,
      methods: t.methods,
      engine: t.solverAgent,
      taskSource: `${t.tabs.jobs} · ${t.sourceModel}`,
      operatorSource: `${t.sections.workflow} · ${t.sourceModel}`,
      peerState: t.peerState,
      meshRoleChip: t.runtimeMode,
      relayChip: "relay",
      meshGroupChip: t.cluster,
      execution: t.status,
      leaseAge: `${t.activeJobs} · ${t.lastHeartbeat}`,
      leaseIdle: t.ready,
      leaseActive: t.busy,
      leaseStale: t.stalledJobs,
      leaseUnknown: "--",
      leaseStateChip: t.status,
      leaseAgeChip: t.lastHeartbeat,
      leaseJobChip: t.tabs.jobs,
      leaseMethodChip: t.methods,
      showMore: t.details,
      showLess: t.close,
    },
    clusterHealthTone,
    peerStatusLabel: (status?: string) => formatPeerStatus(status, t),
  });
  const governanceRuntime = buildWorkbenchGovernanceRuntimeDiagnostics({
    frontendRuntimeMode,
    directMeshEndpointsText: props.directMeshEndpointsText,
    protocolAgents,
    controlPlaneApiToken,
    clusterApiToken,
    directMeshApiToken,
  });

  const runtimeBackendRows = [
    { label: t.ui, value: "3000" },
    { label: t.orchestrator, value: health ? "4000" : t.offline },
    { label: t.solverAgent, value: health?.transport?.solver_agent_tcp ?? 5001 },
  ];

  const runtimeProtocolRows = [
    { label: t.controlPlaneProtocol, value: health?.protocol?.protocol?.name ?? "--" },
    { label: t.solverRpcProtocol, value: health?.protocol?.compatible_solver_rpc?.name ?? "--" },
    {
      label: `${t.controlPlaneProtocol} · ${t.runtimeMode}`,
      value: health?.protocol?.authority
        ? `${health.protocol.authority.authority_mode}${health.protocol.authority.orchestrator_id ? ` · ${health.protocol.authority.orchestrator_id}` : ""}`
        : "--",
    },
    { label: t.deploymentMode, value: health?.deployment?.mode ?? "--" },
    { label: t.discoveryMode, value: health?.deployment?.discovery ?? "--" },
    { label: t.registeredAgents, value: health?.remote_solver_registry?.active_agents ?? 0 },
    {
      label: `${t.mesh} · ${t.cluster}`,
      value: health?.remote_solver_registry?.mesh_topology?.offline_mesh?.clustered_meshes?.length ?? 0,
    },
    {
      label: `${t.mesh} · relay`,
      value:
        health?.remote_solver_registry?.mesh_topology?.offline_mesh?.clustered_meshes?.reduce(
          (sum: number, cluster: any) => sum + (cluster.relay_candidate_ids?.length ?? 0),
          0,
        ) ?? 0,
    },
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
      label: `${t.controls} · ${t.runtimeMode}`,
      value: localizeGovernanceRuntimeValue(governanceRuntime.authorityLabel, t, securityUi),
    },
    {
      label: `${t.cluster} · ${t.access}`,
      value: localizeGovernanceRuntimeValue(governanceRuntime.exposureLabel, t, securityUi),
    },
    {
      label: `${t.security} · ${t.status}`,
      value: localizeGovernanceRuntimeValue(governanceRuntime.driftLabel, t, securityUi),
    },
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
      label: `${t.access} · agent`,
      value: health?.security?.cluster_agent_allowlist_enabled
        ? `${securityUi.enabled} · ${health?.security?.cluster_agent_allowlist_count ?? 0}`
        : securityUi.disabled,
    },
    {
      label: `${t.access} · ${t.cluster}`,
      value: health?.security?.cluster_cluster_allowlist_enabled
        ? `${securityUi.enabled} · ${health?.security?.cluster_cluster_allowlist_count ?? 0}`
        : securityUi.disabled,
    },
    {
      label: `${t.security} · fingerprint`,
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

  const runtimeAuditCopy = getWorkbenchRuntimeAuditCopy(language);
  const runtimeAuditEntries = securityEventRecords.map((entry: any) => ({
    id: entry.event_id,
    at: formatTime(entry.occurred_at, language),
    action: entry.action,
    source: entry.source === "assistant" ? runtimeAuditCopy.assistant : entry.source === "governance" ? t.controls : runtimeAuditCopy.script,
    risk: entry.risk === "destructive" ? runtimeAuditCopy.destructive : runtimeAuditCopy.sensitive,
    status:
      entry.status === "prompted"
        ? runtimeAuditCopy.prompted
        : entry.status === "cancelled"
          ? runtimeAuditCopy.cancelled
          : entry.status === "completed"
            ? runtimeAuditCopy.completed
            : runtimeAuditCopy.failed,
    note: entry.note ?? "--",
  }));

  const runtimeAuditSummaryRows = buildRuntimeAuditSummaryRows(language, securityEventRecords);
  const runtimeAuditTrendBars = buildRuntimeAuditTrendBars(language, securityEventRecords, props.securityEventWindowFilter);
  const runtimeAuditSourceStatusFacets = buildRuntimeAuditSourceStatusFacets(language, securityEventRecords);
  const runtimeAuditStudyFacets = buildRuntimeAuditStudyFacets(securityEventRecords);
  const runtimeAuditProjectFacets = buildRuntimeAuditProjectFacets(securityEventRecords);
  const runtimeAuditModelVersionFacets = buildRuntimeAuditModelVersionFacets(securityEventRecords);
  const recentWatchdogIssueJobs = deferredJobHistory.filter((job: any) => {
    const failureClass = job.status_detail?.failure_class;
    return (
      failureClass === "watchdog_stalled" ||
      failureClass === "watchdog_timeout" ||
      failureClass === "execution_timeout"
    );
  });
  const latestWatchdogIssue = recentWatchdogIssueJobs[0];
  const latestWatchdogIssueLabel = latestWatchdogIssue
    ? `${latestWatchdogIssue.job_id.slice(0, 8)} ${resolveJobStatusDetailLabel(latestWatchdogIssue.status_detail) ?? latestWatchdogIssue.status}`
    : t.none;
  const runtimeWatchdogRows = [
    { label: t.activeJobs, value: health?.watchdog?.active_jobs ?? 0 },
    { label: t.stalledJobs, value: health?.watchdog?.stalled_jobs ?? 0 },
    { label: t.timedOutJobs, value: health?.watchdog?.timed_out_jobs ?? 0 },
    {
      label: `${t.watchdog} · ${t.tabs.jobs}`,
      value: recentWatchdogIssueJobs.length,
    },
    {
      label: t.failureReason,
      value: latestWatchdogIssue
        ? `${latestWatchdogIssueLabel} · ${formatTime(latestWatchdogIssue.updated_at, language)}`
        : latestWatchdogIssueLabel,
    },
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
