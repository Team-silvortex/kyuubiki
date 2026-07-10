import test from "node:test";
import assert from "node:assert/strict";

import { createRuntimeApiClient } from "@/lib/api/runtime-client";

type SeenRequest = {
  url: string;
  method: string;
  body: unknown;
};

function createRecordingClient() {
  const seen: SeenRequest[] = [];
  const client = createRuntimeApiClient(async <T>(url: string, init?: RequestInit) => {
    seen.push({
      url,
      method: init?.method ?? "GET",
      body: init?.body ? JSON.parse(String(init.body)) : null,
    });
    return { ok: true } as T;
  });
  return { client, seen };
}

test("runtime API client factory routes health through injected request", async () => {
  const { client, seen } = createRecordingClient();

  assert.deepEqual(await client.fetchHealth(), { ok: true });
  assert.deepEqual(seen, [{ url: "/api/health", method: "GET", body: null }]);
});

test("runtime API client factory serializes workflow graph submissions", async () => {
  const { client, seen } = createRecordingClient();
  const graph = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "graph-a",
    nodes: [],
    edges: [],
  };

  await client.submitWorkflowGraphJob(graph, { input: 1 }, { include_artifacts: false });

  assert.equal(seen[0]?.url, "/api/v1/workflows/graph/jobs");
  assert.equal(seen[0]?.method, "POST");
  assert.deepEqual(seen[0]?.body, {
    graph,
    input_artifacts: { input: 1 },
    response_options: { include_artifacts: false },
  });
});

test("runtime API client factory serializes direct mesh solves", async () => {
  const { client, seen } = createRecordingClient();

  await client.createDirectMeshSolve("truss_3d", { nodes: [] }, ["127.0.0.1:5001"], "healthiest");

  assert.equal(seen[0]?.url, "/api/direct-mesh/solve");
  assert.equal(seen[0]?.method, "POST");
  assert.deepEqual(seen[0]?.body, {
    study_kind: "truss_3d",
    input: { nodes: [] },
    endpoints: ["127.0.0.1:5001"],
    selection_mode: "healthiest",
  });
});
