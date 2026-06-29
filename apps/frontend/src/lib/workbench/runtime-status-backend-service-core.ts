"use client";

import type {
  DirectMeshAgentListPayload,
  DirectMeshSelectionMode,
  FrontendRuntimeMode,
  HealthPayload,
  ProtocolAgentDescriptor,
  ProtocolAgentListPayload,
  RegisteredAgentRegistryPayload,
} from "@/lib/api/runtime-types";
import { validateWorkbenchExecutionGovernance } from "@/lib/workbench/governance";
import { parseDirectMeshEndpoints } from "@/lib/workbench/helpers";

export type WorkbenchRuntimeStatusRequest = {
  directMeshEndpointsText: string;
  directMeshSelectionMode: DirectMeshSelectionMode;
  frontendRuntimeMode: FrontendRuntimeMode;
};

export type WorkbenchRuntimeStatusSnapshot = {
  health: HealthPayload | null;
  protocolAgents: ProtocolAgentDescriptor[];
};

export type WorkbenchRuntimeStatusTransport = {
  fetchDirectMeshAgents(endpoints: string[]): Promise<DirectMeshAgentListPayload>;
  fetchHealth(): Promise<HealthPayload>;
  fetchProtocolAgents(): Promise<ProtocolAgentListPayload>;
  fetchRegisteredAgents(): Promise<RegisteredAgentRegistryPayload>;
};

export type WorkbenchRuntimeStatusBackendService = {
  fetchStatus(input: WorkbenchRuntimeStatusRequest): Promise<WorkbenchRuntimeStatusSnapshot>;
};

export function createRuntimeStatusBackendService(
  transport: WorkbenchRuntimeStatusTransport,
): WorkbenchRuntimeStatusBackendService {
  return {
    async fetchStatus(input) {
      return input.frontendRuntimeMode === "direct_mesh_gui"
        ? fetchDirectMeshStatus(transport, input)
        : fetchOrchestratedStatus(transport);
    },
  };
}

async function fetchDirectMeshStatus(
  transport: WorkbenchRuntimeStatusTransport,
  input: WorkbenchRuntimeStatusRequest,
): Promise<WorkbenchRuntimeStatusSnapshot> {
  const governance = validateWorkbenchExecutionGovernance({
    directMeshEndpointsText: input.directMeshEndpointsText,
    frontendRuntimeMode: input.frontendRuntimeMode,
  });
  if (!governance.ok) return { health: null, protocolAgents: [] };

  const endpoints = parseDirectMeshEndpoints(governance.directMeshEndpointsText);
  const nextDirect = await transport.fetchDirectMeshAgents(endpoints);
  const directMethods = [
    ...new Set(nextDirect.agents.flatMap((agent) => agent.descriptor?.protocol?.methods ?? [])),
  ];

  return {
    protocolAgents: nextDirect.agents,
    health: {
      service: "kyuubiki-frontend-direct-mesh",
      status: nextDirect.agents.length > 0 ? "ok" : "degraded",
      protocol: {
        program: "kyuubiki-frontend",
        role: "gui",
        protocol: {
          name: "kyuubiki.direct-mesh/http-v1",
          version: 1,
          transport: { kind: "http", encoding: "json" },
        },
        compatible_solver_rpc: {
          name: "kyuubiki.solver-rpc/v1",
          rpc_version: 1,
          transport: {
            kind: "tcp",
            framing: "length_prefixed_u32",
            encoding: "json",
          },
          methods: directMethods,
        },
      },
      deployment: {
        mode: "direct_mesh",
        discovery: nextDirect.discovery,
        endpoint_count: nextDirect.endpoint_count,
      },
      remote_solver_registry: {
        active_agents: nextDirect.agents.length,
      },
    },
  };
}

async function fetchOrchestratedStatus(
  transport: WorkbenchRuntimeStatusTransport,
): Promise<WorkbenchRuntimeStatusSnapshot> {
  const [nextHealth, nextProtocolAgents, nextRegisteredAgents] = await Promise.all([
    transport.fetchHealth(),
    transport.fetchProtocolAgents().catch(() => ({ agents: [] })),
    transport.fetchRegisteredAgents().catch(() => ({
      agents: [],
      summary: { active_execution_lease_count: 0, stale_execution_lease_count: 0 },
    })),
  ]);

  const registryById = new Map(nextRegisteredAgents.agents.map((agent) => [agent.id, agent] as const));
  return {
    health: nextHealth,
    protocolAgents: nextProtocolAgents.agents.map((agent) => {
      const registered = registryById.get(agent.id);
      return registered
        ? {
            ...agent,
            active_lease: registered.active_lease,
            cluster_id: registered.cluster_id ?? agent.cluster_id,
            control_mode: registered.control_mode ?? agent.control_mode,
            execution_state: registered.execution_state,
            mesh: registered.mesh ?? agent.mesh,
            orch_id: registered.orch_id ?? agent.orch_id,
            orch_session_id: registered.orch_session_id ?? agent.orch_session_id,
          }
        : agent;
    }),
  };
}
