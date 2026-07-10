import test from "node:test";
import assert from "node:assert/strict";

import { createHeadlessResultsApiClient } from "@/lib/api/headless-results-client";

test("headless results API client routes result records through injected request", async () => {
  const seen: Array<{ method: string; url: string }> = [];
  const client = createHeadlessResultsApiClient(async <T>(url: string, init?: RequestInit) => {
    seen.push({ method: init?.method ?? "GET", url });
    return { job_id: "job-a", result: {} } as T;
  });

  await client.fetchResultRecord("job-a");
  await client.fetchDirectMeshResultRecord("job-b");

  assert.deepEqual(seen, [
    { method: "GET", url: "/api/v1/results/job-a" },
    { method: "GET", url: "/api/direct-mesh/results/job-b" },
  ]);
});
