import type { FrontendRuntimeMode, ProtocolAgentDescriptor } from "@/lib/api";

export type WorkbenchGovernanceConfig = {
  contractVersion: "kyuubiki.system-governance/v1";
  controlMode: "orchestrated_gui" | "direct_mesh_gui";
  orchestration: {
    topology: "single_orchestrator" | "offline_mesh";
    singleOrchestratorPerAgent: true;
    multiOrchestratorAuthentication: "forbidden";
    offlineMeshWithoutOrchestratorAllowed: true;
  };
  uiSurface: {
    extensible: false;
    authority: "built_in_only";
    wasmPythonAutomationTarget: "stable_builtin_ui";
  };
  ownership: {
    hub: "system_entrypoint";
    workbench: "workflow_state_and_execution_intent";
    installer: "runtime_and_agent_deployment";
    agent: "execution_only";
  };
  library: {
    sourceOfTruth: "central_orchestrator_library";
    replication: "pull_on_demand";
    fullAgentMirrorAllowed: false;
  };
  connectivity: {
    declaredDirectMeshEndpoints: number;
    controlPlaneTokenConfigured: boolean;
    clusterTokenConfigured: boolean;
    directMeshTokenConfigured: boolean;
  };
};

export type WorkbenchGovernanceViolation = "direct_mesh_requires_declared_endpoints";

type WorkbenchGovernedSecrets = {
  controlPlaneApiToken?: string;
  clusterApiToken?: string;
  directMeshApiToken?: string;
};

export type WorkbenchGovernanceRuntimeDiagnostics = {
  authorityLabel: string;
  exposureLabel: string;
  driftLabel: string;
  hasViolation: boolean;
  visibleClusterIds: string[];
  visibleRuntimeModes: string[];
};

export type WorkbenchGovernanceEnforcementPlan = {
  shouldDowngrade: boolean;
  nextFrontendRuntimeMode: FrontendRuntimeMode;
  reason: string | null;
};

type RuntimeNormalizationResult = {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
  declaredDirectMeshEndpoints: number;
  violation: WorkbenchGovernanceViolation | null;
};

function countDeclaredDirectMeshEndpoints(value: string) {
  return value
    .split(/[\n,]+/)
    .map((entry) => entry.trim())
    .filter(Boolean).length;
}

export function normalizeWorkbenchGovernanceRuntime(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
}): RuntimeNormalizationResult {
  const directMeshEndpointsText = input.directMeshEndpointsText;
  const declaredDirectMeshEndpoints = countDeclaredDirectMeshEndpoints(directMeshEndpointsText);
  const violation =
    input.frontendRuntimeMode === "direct_mesh_gui" && declaredDirectMeshEndpoints === 0
      ? "direct_mesh_requires_declared_endpoints"
      : null;

  return {
    frontendRuntimeMode: violation ? "orchestrated_gui" : input.frontendRuntimeMode,
    directMeshEndpointsText,
    declaredDirectMeshEndpoints,
    violation,
  };
}

export function applyWorkbenchGovernancePatch(input: {
  currentFrontendRuntimeMode: FrontendRuntimeMode;
  currentDirectMeshEndpointsText: string;
  nextFrontendRuntimeMode?: FrontendRuntimeMode;
  nextDirectMeshEndpointsText?: string;
}) {
  const mergedRuntimeMode = input.nextFrontendRuntimeMode ?? input.currentFrontendRuntimeMode;
  const mergedDirectMeshEndpointsText =
    input.nextDirectMeshEndpointsText ?? input.currentDirectMeshEndpointsText;

  return normalizeWorkbenchGovernanceRuntime({
    frontendRuntimeMode: mergedRuntimeMode,
    directMeshEndpointsText: mergedDirectMeshEndpointsText,
  });
}

export function validateWorkbenchExecutionGovernance(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
}) {
  const normalized = normalizeWorkbenchGovernanceRuntime(input);

  return {
    ok: normalized.violation === null,
    ...normalized,
  };
}

export function buildWorkbenchGovernedAuthHeaders(input: {
  url: string;
  frontendRuntimeMode: FrontendRuntimeMode;
  secrets: WorkbenchGovernedSecrets;
}) {
  const controlPlaneApiToken = input.secrets.controlPlaneApiToken?.trim() ?? "";
  const clusterApiToken = input.secrets.clusterApiToken?.trim() ?? "";
  const directMeshApiToken = input.secrets.directMeshApiToken?.trim() ?? "";

  if (input.url.startsWith("/api/direct-mesh")) {
    return input.frontendRuntimeMode === "direct_mesh_gui" && directMeshApiToken
      ? { "x-kyuubiki-token": directMeshApiToken }
      : {};
  }

  if (
    input.url === "/api/v1/agents/register" ||
    /^\/api\/v1\/agents\/[^/]+\/heartbeat$/.test(input.url) ||
    /^\/api\/v1\/agents\/[^/]+$/.test(input.url)
  ) {
    if (input.frontendRuntimeMode !== "orchestrated_gui") return {};
    return clusterApiToken
      ? { "x-kyuubiki-token": clusterApiToken }
      : controlPlaneApiToken
        ? { "x-kyuubiki-token": controlPlaneApiToken }
        : {};
  }

  if (input.url.startsWith("/api/v1") || input.url.startsWith("/api/playground") || input.url === "/api/health") {
    return input.frontendRuntimeMode === "orchestrated_gui" && controlPlaneApiToken
      ? { "x-kyuubiki-token": controlPlaneApiToken }
      : {};
  }

  return {};
}

export function buildWorkbenchGovernanceRuntimeDiagnostics(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
  protocolAgents: readonly ProtocolAgentDescriptor[];
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
}) {
  const normalized = normalizeWorkbenchGovernanceRuntime({
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
  });
  const visibleClusterIds = [...new Set(
    input.protocolAgents
      .map((agent) => agent.descriptor?.runtime.cluster_id?.trim() ?? "")
      .filter(Boolean),
  )];
  const visibleRuntimeModes = [...new Set(
    input.protocolAgents
      .map((agent) => agent.descriptor?.runtime.runtime_mode?.trim() ?? "")
      .filter(Boolean),
  )];
  const orchestratedAuthorityActive =
    normalized.frontendRuntimeMode === "orchestrated_gui" &&
    input.controlPlaneApiToken.trim().length > 0;
  const directMeshAuthorityActive =
    normalized.frontendRuntimeMode === "direct_mesh_gui" &&
    input.directMeshApiToken.trim().length > 0;
  const authorityLabel =
    normalized.frontendRuntimeMode === "direct_mesh_gui"
      ? directMeshAuthorityActive
        ? "direct-mesh authority"
        : "direct-mesh authority missing token"
      : orchestratedAuthorityActive
        ? input.clusterApiToken.trim().length > 0
          ? "single orchestrator authority via cluster token"
          : "single orchestrator authority via control-plane token"
        : "orchestrator authority missing token";
  const exposureLabel =
    visibleClusterIds.length <= 1
      ? visibleClusterIds[0] ?? "single cluster scope"
      : `${visibleClusterIds.length} clusters visible`;
  const driftSources = [
    normalized.violation === "direct_mesh_requires_declared_endpoints" ? "direct mesh missing endpoints" : null,
    visibleClusterIds.length > 1 ? "multi-cluster exposure" : null,
    visibleRuntimeModes.length > 1 ? "mixed runtime modes" : null,
  ].filter(Boolean);

  return {
    authorityLabel,
    exposureLabel,
    driftLabel: driftSources[0] ?? "aligned",
    hasViolation:
      normalized.violation !== null ||
      visibleClusterIds.length > 1 ||
      visibleRuntimeModes.length > 1,
    visibleClusterIds,
    visibleRuntimeModes,
  } satisfies WorkbenchGovernanceRuntimeDiagnostics;
}

export function buildWorkbenchGovernanceEnforcementPlan(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  diagnostics: WorkbenchGovernanceRuntimeDiagnostics;
}) {
  const shouldDowngrade =
    input.frontendRuntimeMode === "direct_mesh_gui" &&
    (input.diagnostics.hasViolation ||
      input.diagnostics.visibleClusterIds.length > 1 ||
      input.diagnostics.visibleRuntimeModes.length > 1);

  return {
    shouldDowngrade,
    nextFrontendRuntimeMode: shouldDowngrade ? "orchestrated_gui" : input.frontendRuntimeMode,
    reason: shouldDowngrade ? input.diagnostics.driftLabel : null,
  } satisfies WorkbenchGovernanceEnforcementPlan;
}

export function buildWorkbenchGovernanceConfig(input: {
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
}): WorkbenchGovernanceConfig {
  const normalized = normalizeWorkbenchGovernanceRuntime({
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
  });

  return {
    contractVersion: "kyuubiki.system-governance/v1",
    controlMode: normalized.frontendRuntimeMode,
    orchestration: {
      topology: normalized.frontendRuntimeMode === "direct_mesh_gui" ? "offline_mesh" : "single_orchestrator",
      singleOrchestratorPerAgent: true,
      multiOrchestratorAuthentication: "forbidden",
      offlineMeshWithoutOrchestratorAllowed: true,
    },
    uiSurface: {
      extensible: false,
      authority: "built_in_only",
      wasmPythonAutomationTarget: "stable_builtin_ui",
    },
    ownership: {
      hub: "system_entrypoint",
      workbench: "workflow_state_and_execution_intent",
      installer: "runtime_and_agent_deployment",
      agent: "execution_only",
    },
    library: {
      sourceOfTruth: "central_orchestrator_library",
      replication: "pull_on_demand",
      fullAgentMirrorAllowed: false,
    },
    connectivity: {
      declaredDirectMeshEndpoints: normalized.declaredDirectMeshEndpoints,
      controlPlaneTokenConfigured: input.controlPlaneApiToken.trim().length > 0,
      clusterTokenConfigured: input.clusterApiToken.trim().length > 0,
      directMeshTokenConfigured: input.directMeshApiToken.trim().length > 0,
    },
  };
}

export function buildWorkbenchGovernanceRows(config: WorkbenchGovernanceConfig) {
  return [
    { label: "Control mode", value: config.controlMode },
    { label: "Topology", value: config.orchestration.topology },
    {
      label: "Agent orchestration",
      value: config.orchestration.singleOrchestratorPerAgent ? "single orch per agent" : "mutable",
    },
    {
      label: "Multi-orch auth",
      value: config.orchestration.multiOrchestratorAuthentication,
    },
    { label: "UI surface", value: config.uiSurface.authority },
    { label: "Agent library", value: config.library.replication },
  ];
}
