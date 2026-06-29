import test from "node:test";
import assert from "node:assert/strict";

import { createJobHistoryBackendService } from "../../src/lib/workbench/job-history-backend-service-core.ts";
import type { JobEnvelope, JobHistoryPayload } from "../../src/lib/api/fem-shared.ts";

function jobEnvelope(jobId: string): JobEnvelope {
  return {
    job: {
      created_at: "2026-06-29T00:00:00.000Z",
      job_id: jobId,
      progress: 0,
      status: "cancelled",
      updated_at: "2026-06-29T00:00:00.000Z",
      worker_id: null,
    },
  };
}

test("job history backend fetches history through transport", async () => {
  const calls: string[] = [];
  const service = createJobHistoryBackendService({
    cancelJob: async (jobId) => {
      calls.push(`cancel:${jobId}`);
      return jobEnvelope(jobId);
    },
    fetchJobHistory: async () => {
      calls.push("history");
      return { jobs: [jobEnvelope("job-a").job] } satisfies JobHistoryPayload;
    },
  });

  const payload = await service.fetchHistory();

  assert.deepEqual(calls, ["history"]);
  assert.equal(payload.jobs[0]?.job_id, "job-a");
});

test("job history backend cancels jobs through transport", async () => {
  const calls: string[] = [];
  const service = createJobHistoryBackendService({
    cancelJob: async (jobId) => {
      calls.push(`cancel:${jobId}`);
      return jobEnvelope(jobId);
    },
    fetchJobHistory: async () => {
      calls.push("history");
      return { jobs: [] } satisfies JobHistoryPayload;
    },
  });

  const payload = await service.cancelJob("job-42");

  assert.deepEqual(calls, ["cancel:job-42"]);
  assert.equal(payload.job.job_id, "job-42");
});
