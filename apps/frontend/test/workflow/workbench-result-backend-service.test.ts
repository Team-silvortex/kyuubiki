import test from "node:test";
import assert from "node:assert/strict";

import { createResultBackendService } from "../../src/lib/workbench/result-backend-service-core.ts";
import { chunkCacheKey } from "../../src/lib/workbench/result-window.ts";
import type { ResultChunkKind, ResultChunkPayload } from "../../src/lib/api/security-results-types.ts";

test("result backend service forwards chunk requests through transport", async () => {
  const calls: string[] = [];
  const service = createResultBackendService({
    backendId: "mock-backend",
    fetchChunk: async <TItem>(
      jobId: string,
      kind: ResultChunkKind,
      options: { limit?: number; offset?: number },
    ) => {
      calls.push(`${jobId}:${kind}:${options.offset}:${options.limit}`);
      return {
        job_id: jobId,
        items: [{ id: "n1" }],
        kind,
        limit: options.limit ?? 0,
        offset: options.offset ?? 0,
        returned: 1,
        total: 1,
      } as ResultChunkPayload<TItem>;
    },
  });

  const chunk = await service.fetchChunk({
    jobId: "job-1",
    kind: "nodes",
    limit: 25,
    offset: 50,
  });

  assert.equal(service.backendId, "mock-backend");
  assert.deepEqual(calls, ["job-1:nodes:50:25"]);
  assert.deepEqual(chunk.items, [{ id: "n1" }]);
});

test("result chunk cache keys are backend-id based, not hard-wired to runtime modes", () => {
  assert.equal(
    chunkCacheKey("mobile-remote-orch", "job-1", "nodes", 0, 100),
    "mobile-remote-orch:job-1:nodes:0:100",
  );
  assert.notEqual(
    chunkCacheKey("mobile-remote-orch", "job-1", "nodes", 0, 100),
    chunkCacheKey("direct_mesh_gui", "job-1", "nodes", 0, 100),
  );
});
