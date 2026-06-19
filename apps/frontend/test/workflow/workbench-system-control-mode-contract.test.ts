import test from "node:test";
import assert from "node:assert/strict";

import {
  buildControlTopologySummaryFromSnapshot,
  buildWorkbenchSystemControlModeCopy,
  buildWorkbenchSystemControlTopologySummary,
  buildWorkbenchSystemTopologySnapshot,
  parseWorkbenchSystemTopologySnapshot,
} from "../../src/components/workbench/system/workbench-system-control-mode-contract.ts";

const COPY = buildWorkbenchSystemControlModeCopy("en", "direct_mesh_gui");

const AGENTS = [
  {
    id: "mesh-a",
    host: "10.0.0.1",
    port: 5001,
    control_mode: "offline_mesh",
    cluster_id: "mesh-alpha",
    mesh: {
      cluster_id: "mesh-alpha",
      relay_candidate: true,
      topology_role: "relay_candidate",
      peers: [{ id: "mesh-b", address: "10.0.0.2:5001", status: "healthy" }],
    },
    descriptor: {
      runtime: {
        runtime_mode: "direct_mesh_gui",
        headless: true,
        cluster_id: "mesh-alpha",
        health_score: 0.92,
        peers: [{ address: "10.0.0.2:5001", status: "healthy", failure_count: 0, last_seen_unix_s: 995 }],
      },
    },
  },
  {
    id: "mesh-b",
    host: "10.0.0.2",
    port: 5001,
    control_mode: "offline_mesh",
    cluster_id: "mesh-alpha",
    mesh: {
      cluster_id: "mesh-alpha",
      relay_candidate: false,
      topology_role: "peer",
      peers: [{ id: "mesh-a", address: "10.0.0.1:5001", status: "healthy" }],
    },
    descriptor: {
      runtime: {
        runtime_mode: "direct_mesh_gui",
        headless: true,
        cluster_id: "mesh-alpha",
        health_score: 0.8,
        peers: [{ address: "10.0.0.1:5001", status: "healthy", failure_count: 0, last_seen_unix_s: 990 }],
      },
    },
  },
  {
    id: "mesh-c",
    host: "10.0.1.1",
    port: 5001,
    control_mode: "offline_mesh",
    cluster_id: "mesh-beta",
    mesh: {
      cluster_id: "mesh-beta",
      relay_candidate: true,
      topology_role: "relay_candidate",
      peers: [],
    },
    descriptor: {
      runtime: {
        runtime_mode: "direct_mesh_gui",
        headless: true,
        cluster_id: "mesh-beta",
        health_score: 0.61,
        peers: [],
      },
    },
  },
  {
    id: "mesh-d",
    host: "10.0.2.1",
    port: 5001,
    control_mode: "offline_mesh",
    mesh: {
      relay_candidate: false,
      topology_role: "peer",
      peers: [],
    },
    descriptor: {
      runtime: {
        runtime_mode: "direct_mesh_gui",
        headless: true,
        health_score: 0.49,
        peers: [],
      },
    },
  },
] as any;

test("buildWorkbenchSystemControlTopologySummary reports mesh cluster overview", () => {
  const summary = buildWorkbenchSystemControlTopologySummary({
    frontendRuntimeMode: "direct_mesh_gui",
    directMeshSelectionMode: "healthiest",
    directMeshEndpointsText: "10.0.0.1:5001\n10.0.0.2:5001",
    protocolAgents: AGENTS,
    controlPlaneApiToken: "",
    clusterApiToken: "cluster-token",
    directMeshApiToken: "mesh-token",
    protocolOnline: true,
    securityConfigured: true,
    auditCount: 3,
    copy: COPY,
    nowUnixS: 1_000,
  });

  assert.equal(summary.mode, "mesh");
  assert.equal(summary.meshClusterCount, 2);
  assert.equal(summary.meshRelayCandidateCount, 2);
  assert.equal(summary.meshUnclusteredCount, 1);
  assert.equal(summary.meshClusters[0]?.clusterId, "mesh-alpha");
  assert.equal(summary.meshClusters[0]?.agentCount, 2);
  assert.equal(summary.meshClusters[0]?.peerCount, 2);
  assert.equal(summary.meshClusters[0]?.entryAgentId, "mesh-a");
  assert.equal(summary.controlGroups.length, 2);
  assert.equal(summary.controlGroups[0]?.kind, "mesh");
});

test("buildWorkbenchSystemControlTopologySummary keeps orchestrated groups first-class", () => {
  const summary = buildWorkbenchSystemControlTopologySummary({
    frontendRuntimeMode: "orchestrated_gui",
    directMeshSelectionMode: "healthiest",
    directMeshEndpointsText: "",
    protocolAgents: [
      {
        id: "orch-a1",
        host: "127.0.0.1",
        port: 5001,
        control_mode: "orch_managed",
        orch_id: "orch-alpha",
        orch_session_id: "s1",
        descriptor: { authority: { control_mode: "orch_managed", orchestrator_id: "orch-alpha", orchestrator_session_id: "s1" }, runtime: { runtime_mode: "orchestrated_gui", headless: true, health_score: 0.75, peers: [{ address: "127.0.0.2:5001" }] } },
      },
      {
        id: "orch-a2",
        host: "127.0.0.2",
        port: 5001,
        control_mode: "orch_managed",
        orch_id: "orch-alpha",
        orch_session_id: "s2",
        descriptor: { authority: { control_mode: "orch_managed", orchestrator_id: "orch-alpha", orchestrator_session_id: "s2" }, runtime: { runtime_mode: "orchestrated_gui", headless: true, health_score: 0.55, peers: [] } },
      },
      {
        id: "orch-b1",
        host: "127.0.1.1",
        port: 5001,
        control_mode: "orch_managed",
        orch_id: "orch-beta",
        orch_session_id: "s3",
        descriptor: { authority: { control_mode: "orch_managed", orchestrator_id: "orch-beta", orchestrator_session_id: "s3" }, runtime: { runtime_mode: "orchestrated_gui", headless: true, health_score: 0.9, peers: [] } },
      },
    ] as any,
    controlPlaneApiToken: "cp",
    clusterApiToken: "",
    directMeshApiToken: "",
    protocolOnline: true,
    securityConfigured: true,
    auditCount: 5,
    copy: buildWorkbenchSystemControlModeCopy("en", "orchestrated_gui"),
    nowUnixS: 1_000,
  });

  assert.equal(summary.mode, "orchestrated");
  assert.equal(summary.controlGroups.length, 2);
  assert.equal(summary.controlGroups[0]?.id, "orch-alpha");
  assert.equal(summary.controlGroups[0]?.sessionCount, 2);
  assert.equal(summary.controlGroups[1]?.entryAgentId, "orch-b1");
});

test("topology snapshot preserves mesh cluster summaries", () => {
  const topology = buildWorkbenchSystemControlTopologySummary({
    frontendRuntimeMode: "direct_mesh_gui",
    directMeshSelectionMode: "healthiest",
    directMeshEndpointsText: "10.0.0.1:5001",
    protocolAgents: AGENTS,
    controlPlaneApiToken: "",
    clusterApiToken: "cluster-token",
    directMeshApiToken: "mesh-token",
    protocolOnline: true,
    securityConfigured: true,
    auditCount: 0,
    copy: COPY,
    nowUnixS: 1_000,
  });
  const snapshot = buildWorkbenchSystemTopologySnapshot({
    frontendRuntimeMode: "direct_mesh_gui",
    directMeshSelectionMode: "healthiest",
    directMeshEndpointsText: "10.0.0.1:5001",
    protocolAgents: AGENTS,
    topology,
    observedAt: new Date("2026-06-19T12:00:00.000Z"),
  });
  const parsed = parseWorkbenchSystemTopologySnapshot(snapshot);

  assert.ok(parsed);
  assert.equal(parsed?.mesh_cluster_count, 2);
  assert.equal(parsed?.mesh_relay_candidate_count, 2);
  assert.equal(parsed?.mesh_unclustered_count, 1);
  assert.equal(parsed?.mesh_clusters[1]?.cluster_id, "mesh-beta");

  const rebuilt = buildControlTopologySummaryFromSnapshot(parsed!, COPY);
  assert.equal(rebuilt.meshClusterCount, 2);
  assert.equal(rebuilt.meshClusters[1]?.entryAgentId, "mesh-c");
});
