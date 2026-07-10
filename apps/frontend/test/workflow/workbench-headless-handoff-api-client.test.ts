import test from "node:test";
import assert from "node:assert/strict";

import { createHeadlessHandoffApiClient } from "@/lib/api/headless-handoff-client";

type SeenRequest = {
  body: unknown;
  method: string;
  url: string;
};

function createRecordingClient() {
  const seen: SeenRequest[] = [];
  const client = createHeadlessHandoffApiClient(async <T>(url: string, init?: RequestInit) => {
    seen.push({
      body: init?.body ? JSON.parse(String(init.body)) : null,
      method: init?.method ?? "GET",
      url,
    });
    return { handoff_id: "handoff-a" } as T;
  });
  return { client, seen };
}

test("headless handoff API client routes submit, status, history, and snapshot", async () => {
  const { client, seen } = createRecordingClient();
  const envelope = {
    schema_version: "kyuubiki.headless-orchestra-handoff/v1",
    workflow_id: "workflow-a",
  };

  await client.submitHeadlessOrchestraHandoff(envelope as never);
  await client.fetchHeadlessOrchestraHandoffStatus("handoff/a");
  await client.fetchHeadlessOrchestraHandoffHistory();
  await client.fetchHeadlessOrchestraHandoffSnapshot("handoff/a");

  assert.deepEqual(seen, [
    {
      body: envelope,
      method: "POST",
      url: "/api/v1/headless/handoff",
    },
    {
      body: null,
      method: "GET",
      url: "/api/v1/headless/handoff/handoff%2Fa",
    },
    {
      body: null,
      method: "GET",
      url: "/api/v1/headless/handoff",
    },
    {
      body: null,
      method: "GET",
      url: "/api/v1/headless/handoff/handoff%2Fa/snapshot",
    },
  ]);
});
