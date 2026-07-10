import test from "node:test";
import assert from "node:assert/strict";

import type { HeadlessWorkflowExecutionBatch } from "@/components/workbench/workbench-headless-workflow-export";
import {
  buildHeadlessAgentDispatchPlanFromBackend,
  buildHeadlessOrchestraHandoffFromBackend,
  describeHeadlessHandoffReceiptForLog,
  submitHeadlessOrchestraHandoffFromBackend,
} from "@/components/workbench/workbench-headless-workflow-panel-actions";
import type { HeadlessHandoffReceipt } from "@/lib/api/headless-handoff-client";
import type { ProtocolAgentDescriptor } from "@/lib/api/runtime-types";

function batch(): HeadlessWorkflowExecutionBatch {
  return {
    exported_at: "2026-07-10T00:00:00.000Z",
    language: "ts",
    schema_version: "kyuubiki.headless-execution-batch/v1",
    steps: [
      {
        action: "service_health",
        guidanceNotes: [],
        index: 0,
        payload: { kind: "literal", value: {} },
        risk: "normal",
      },
    ],
    warnings: [],
    workflow_id: "workflow-a",
  };
}

function authSnapshot() {
  return {
    clusterApiToken: "cluster",
    controlPlaneApiToken: "control",
    directMeshApiToken: "mesh",
    directMeshEndpointsText: "127.0.0.1:5001",
    frontendRuntimeMode: "orchestrated_gui" as const,
  };
}

function protocolAgent(): ProtocolAgentDescriptor {
  return {
    id: "agent-a",
    host: "127.0.0.1",
    port: 5001,
    descriptor: {
      capabilities: [
        {
          id: "service-api",
          methods: ["health.check"],
          role: "service",
          tags: ["service_api"],
        },
      ],
      deployment_modes: ["local"],
      program: "kyuubiki-agent",
      protocol: {
        name: "kyuubiki.agent",
        rpc_version: 1,
        transport: { encoding: "json", kind: "http" },
        methods: ["health.check"],
      },
      role: "agent",
      runtime: {
        cluster_id: "cluster-a",
        headless: true,
        health_score: 90,
        peers: [],
        runtime_mode: "orchestrated_gui",
      },
    },
  };
}

function receipt(): HeadlessHandoffReceipt {
  return {
    accepted: true,
    authority_mode: "single_orchestrator",
    chosen_agent_count: 1,
    dispatch_override_count: 0,
    handoff_id: "handoff-a",
    has_dispatch_override: false,
    override_acknowledged: false,
    override_note: null,
    override_step_keys: [],
    override_summary: null,
    received_at: "2026-07-10T00:00:00.000Z",
    stage: "received",
    status_message: "received",
    step_count: 1,
    target_clusters: [],
    warning_count: 0,
    workflow_id: "workflow-a",
  };
}

test("headless workflow panel actions build dispatch plans from backend agents", async () => {
  const plan = await buildHeadlessAgentDispatchPlanFromBackend({
    backendService: {
      fetchProtocolAgents: async () => ({
        agents: [protocolAgent()],
      }),
    },
    batch: batch(),
  });

  assert.equal(plan.workflow_id, "workflow-a");
  assert.equal(plan.steps[0]?.chosen_agent_id, "agent-a");
});

test("headless workflow panel actions build and submit orchestra handoffs", async () => {
  const seen: string[] = [];
  const backendService = {
    fetchProtocolAgents: async () => {
      seen.push("agents");
      return { agents: [] };
    },
    submitHandoff: async (handoff: unknown) => {
      seen.push(`submit:${(handoff as { workflow_id: string }).workflow_id}`);
      return receipt();
    },
  };

  const handoff = await buildHeadlessOrchestraHandoffFromBackend({
    auth: authSnapshot(),
    backendService,
    batch: batch(),
  });
  const submitted = await submitHeadlessOrchestraHandoffFromBackend({
    backendService,
    buildAuthSnapshot: authSnapshot,
    batch: batch(),
  });

  assert.equal(handoff.workflow_id, "workflow-a");
  assert.equal(handoff.runtime_manifest.source_of_truth, "central_orchestrator_library");
  assert.deepEqual(seen, ["agents", "agents", "submit:workflow-a"]);
  assert.deepEqual(submitted.receipt, receipt());
  assert.equal(describeHeadlessHandoffReceiptForLog({ handoff_id: "handoff-a", workflow_id: "workflow-a" }), '[handoff] {"handoff_id":"handoff-a","workflow_id":"workflow-a"}');
});
