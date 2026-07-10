import test from "node:test";
import assert from "node:assert/strict";

import type { HeadlessHandoffApiClient } from "@/lib/api/headless-handoff-client";
import type { RuntimeApiClient } from "@/lib/api/runtime-client";
import { createWorkbenchHeadlessWorkflowBackendService } from "@/lib/workbench/headless-workflow-backend-service";

test("headless workflow backend service composes runtime and handoff clients", async () => {
  const calls: string[] = [];
  const handoffClient = {
    fetchHeadlessOrchestraHandoffHistory: async () => {
      calls.push("handoff-history");
      return { handoffs: [] };
    },
    fetchHeadlessOrchestraHandoffSnapshot: async (handoffId: string) => {
      calls.push(`handoff-snapshot:${handoffId}`);
      return { handoff_id: handoffId };
    },
    fetchHeadlessOrchestraHandoffStatus: async (handoffId: string) => {
      calls.push(`handoff-status:${handoffId}`);
      return { handoff_id: handoffId };
    },
    submitHeadlessOrchestraHandoff: async () => {
      calls.push("handoff-submit");
      return { handoff_id: "handoff-a" };
    },
  } as unknown as HeadlessHandoffApiClient;
  const runtimeClient = {
    fetchProtocolAgents: async () => {
      calls.push("protocol-agents");
      return { agents: [] };
    },
  } as unknown as RuntimeApiClient;
  const service = createWorkbenchHeadlessWorkflowBackendService({ handoffClient, runtimeClient });

  await service.fetchProtocolAgents();
  await service.submitHandoff({} as never);
  await service.fetchHandoffStatus("handoff-a");
  await service.fetchHandoffHistory();
  await service.fetchHandoffSnapshot("handoff-a");

  assert.deepEqual(calls, [
    "protocol-agents",
    "handoff-submit",
    "handoff-status:handoff-a",
    "handoff-history",
    "handoff-snapshot:handoff-a",
  ]);
});
