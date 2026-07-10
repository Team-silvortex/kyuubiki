import test from "node:test";
import assert from "node:assert/strict";

import { createSecurityResultsApiClient } from "@/lib/api/security-results-client";

type SeenRequest = {
  body: unknown;
  method: string;
  transport: "json" | "text";
  url: string;
};

function createRecordingClient() {
  const seen: SeenRequest[] = [];
  const client = createSecurityResultsApiClient({
    requestJson: async <T>(url: string, init?: RequestInit) => {
      seen.push({
        body: init?.body ? JSON.parse(String(init.body)) : null,
        method: init?.method ?? "GET",
        transport: "json",
        url,
      });
      return { ok: true } as T;
    },
    requestText: async (url: string, init?: RequestInit) => {
      seen.push({
        body: init?.body ? String(init.body) : null,
        method: init?.method ?? "GET",
        transport: "text",
        url,
      });
      return "csv";
    },
  });
  return { client, seen };
}

test("security results API client routes filtered event reads through injected JSON request", async () => {
  const { client, seen } = createRecordingClient();

  assert.deepEqual(await client.fetchSecurityEvents({ risk: "high", status: "open", limit: 3 }), {
    ok: true,
  });
  assert.deepEqual(seen, [
    {
      body: null,
      method: "GET",
      transport: "json",
      url: "/api/v1/security-events?risk=high&status=open&limit=3",
    },
  ]);
});

test("security results API client keeps CSV export on injected text request", async () => {
  const { client, seen } = createRecordingClient();

  assert.equal(await client.exportSecurityEventsCsv({ source: "engine" }), "csv");
  assert.deepEqual(seen, [
    {
      body: null,
      method: "GET",
      transport: "text",
      url: "/api/v1/export/security-events.csv?source=engine",
    },
  ]);
});

test("security results API client serializes result mutation and chunk window", async () => {
  const { client, seen } = createRecordingClient();

  await client.fetchResultChunk("job-a", "nodes", { offset: 10, limit: 25 });
  await client.updateResultRecord("job-a", { converged: true });
  await client.deleteResultRecord("job-a");

  assert.deepEqual(seen, [
    {
      body: null,
      method: "GET",
      transport: "json",
      url: "/api/v1/results/job-a/chunks/nodes?offset=10&limit=25",
    },
    {
      body: { result: { converged: true } },
      method: "PATCH",
      transport: "json",
      url: "/api/v1/results/job-a",
    },
    {
      body: null,
      method: "DELETE",
      transport: "json",
      url: "/api/v1/results/job-a",
    },
  ]);
});
