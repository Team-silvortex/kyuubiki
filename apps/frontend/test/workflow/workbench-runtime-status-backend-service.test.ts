import test from "node:test";
import assert from "node:assert/strict";

import { createRuntimeStatusGateway as createRuntimeStatusBackendService } from "../../src/lib/runtime-gateway/runtime-status-gateway.ts";
import type {
  DirectMeshAgentListPayload,
  HealthPayload,
  ProtocolAgentDescriptor,
  ProtocolAgentListPayload,
  RegisteredAgentRegistryPayload,
} from "../../src/lib/api/runtime-types.ts";

function agent(id: string, methods: string[] = []): ProtocolAgentDescriptor {
  return {
    host: "127.0.0.1",
    id,
    port: 5001,
    descriptor: {
      program: "kyuubiki-agent",
      role: "agent",
      authority: {
        accepts_multi_orchestrator_binding: false,
        agent_library_replication: "forbidden",
        authority_mode: "single_orchestrator",
        control_mode: "orchestrated",
      },
      capabilities: [],
      deployment_modes: [],
      protocol: {
        methods,
        name: "kyuubiki.solver-rpc/v1",
        rpc_version: 1,
        transport: { encoding: "json", kind: "tcp" },
      },
      runtime: {
        headless: true,
        peers: [],
        runtime_mode: "agent",
      },
    },
  } as ProtocolAgentDescriptor;
}

test("runtime status backend synthesizes direct-mesh GUI health from endpoints", async () => {
  const service = createRuntimeStatusBackendService({
    fetchDirectMeshAgents: async (endpoints) =>
      ({
        agents: [agent("mesh-a", ["solve_heat"])],
        discovery: "request",
        endpoint_count: endpoints.length,
        mode: "direct_mesh_gui",
      }) satisfies DirectMeshAgentListPayload,
    fetchHealth: async () => ({ service: "unused", status: "ok" }) satisfies HealthPayload,
    fetchProtocolAgents: async () => ({ agents: [] }) satisfies ProtocolAgentListPayload,
    fetchRegisteredAgents: async () =>
      ({
        agents: [],
        summary: { active_execution_lease_count: 0, stale_execution_lease_count: 0 },
      }) satisfies RegisteredAgentRegistryPayload,
  });

  const snapshot = await service.fetchStatus({
    directMeshEndpointsText: "10.0.0.1:5001",
    directMeshSelectionMode: "healthiest",
    frontendRuntimeMode: "direct_mesh_gui",
  });

  assert.equal(snapshot.health?.service, "kyuubiki-frontend-direct-mesh");
  assert.equal(snapshot.health?.deployment?.mode, "direct_mesh");
  assert.equal(snapshot.health?.deployment?.endpoint_count, 1);
  assert.deepEqual(snapshot.health?.protocol?.compatible_solver_rpc?.methods, ["solve_heat"]);
  assert.equal(snapshot.protocolAgents[0]?.id, "mesh-a");
});

test("runtime status backend returns empty direct-mesh snapshot when endpoints are invalid", async () => {
  const service = createRuntimeStatusBackendService({
    fetchDirectMeshAgents: async () => {
      throw new Error("should not fetch");
    },
    fetchHealth: async () => ({ service: "unused", status: "ok" }) satisfies HealthPayload,
    fetchProtocolAgents: async () => ({ agents: [] }) satisfies ProtocolAgentListPayload,
    fetchRegisteredAgents: async () =>
      ({
        agents: [],
        summary: { active_execution_lease_count: 0, stale_execution_lease_count: 0 },
      }) satisfies RegisteredAgentRegistryPayload,
  });

  const snapshot = await service.fetchStatus({
    directMeshEndpointsText: "",
    directMeshSelectionMode: "healthiest",
    frontendRuntimeMode: "direct_mesh_gui",
  });

  assert.equal(snapshot.health, null);
  assert.deepEqual(snapshot.protocolAgents, []);
});

test("runtime status backend merges orchestrated registry state into protocol agents", async () => {
  const service = createRuntimeStatusBackendService({
    fetchDirectMeshAgents: async () =>
      ({ agents: [], discovery: "unused", endpoint_count: 0, mode: "direct_mesh_gui" }) satisfies DirectMeshAgentListPayload,
    fetchHealth: async () => ({ service: "orchestra", status: "ok" }) satisfies HealthPayload,
    fetchProtocolAgents: async () => ({ agents: [agent("agent-a")] }) satisfies ProtocolAgentListPayload,
    fetchRegisteredAgents: async () =>
      ({
        agents: [
          {
            active_lease: {
              age_ms: 10,
              agent_id: "agent-a",
              is_stale: false,
              job_id: "job-1",
              lease_id: "lease-1",
              method: "solve",
            },
            control_mode: "orchestrated",
            execution_state: "leased",
            id: "agent-a",
            orch_id: "orch-main",
          },
        ],
        summary: { active_execution_lease_count: 1, stale_execution_lease_count: 0 },
      }) satisfies RegisteredAgentRegistryPayload,
  });

  const snapshot = await service.fetchStatus({
    directMeshEndpointsText: "",
    directMeshSelectionMode: "healthiest",
    frontendRuntimeMode: "orchestrated_gui",
  });

  assert.equal(snapshot.health?.service, "orchestra");
  assert.equal(snapshot.protocolAgents[0]?.execution_state, "leased");
  assert.equal(snapshot.protocolAgents[0]?.active_lease?.job_id, "job-1");
  assert.equal(snapshot.protocolAgents[0]?.orch_id, "orch-main");
});
