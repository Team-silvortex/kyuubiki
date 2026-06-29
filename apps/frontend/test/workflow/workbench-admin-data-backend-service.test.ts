import test from "node:test";
import assert from "node:assert/strict";

import {
  createAdminDataBackendService,
  type WorkbenchAdminDataBackendTransport,
} from "../../src/lib/workbench/admin-data-backend-service-core.ts";
import type { JobEnvelope, JobState } from "../../src/lib/api/fem-shared.ts";
import type { ResultListPayload } from "../../src/lib/api/security-results-types.ts";

function jobState(jobId: string): JobState {
  return {
    created_at: "2026-06-29T00:00:00.000Z",
    job_id: jobId,
    message: "queued",
    progress: 0,
    status: "queued",
    updated_at: "2026-06-29T00:00:00.000Z",
    worker_id: null,
  };
}

function createTransport(calls: unknown[]): WorkbenchAdminDataBackendTransport {
  return {
    deleteJob: async (jobId) => {
      calls.push(["deleteJob", jobId]);
      return { deleted: true, job: jobState(jobId) };
    },
    deleteResult: async (jobId) => {
      calls.push(["deleteResult", jobId]);
      return { deleted: true, job_id: jobId, result: { deleted: true } };
    },
    fetchJob: async <TResult = unknown>(jobId: string) => {
      calls.push(["fetchJob", jobId]);
      return {
        job: jobState(jobId),
        result: { ok: true } as TResult,
      } satisfies JobEnvelope<TResult>;
    },
    fetchResults: async () => {
      calls.push(["fetchResults"]);
      return {
        results: [
          {
            job_id: "job-a",
            result: { displacement: 1.25 },
          },
        ],
      } satisfies ResultListPayload;
    },
    updateJob: async (jobId, input) => {
      calls.push(["updateJob", jobId, input]);
      return { job: { ...jobState(jobId), ...input } };
    },
    updateResult: async (jobId, result) => {
      calls.push(["updateResult", jobId, result]);
      return { job_id: jobId, result };
    },
  };
}

test("admin data backend lists results through transport", async () => {
  const calls: unknown[] = [];
  const service = createAdminDataBackendService(createTransport(calls));

  const payload = await service.fetchResults();
  const results = await service.listResults();

  assert.equal(payload.results[0]?.job_id, "job-a");
  assert.deepEqual(results, payload.results);
  assert.deepEqual(calls, [["fetchResults"], ["fetchResults"]]);
});

test("admin data backend manages job records through transport", async () => {
  const calls: unknown[] = [];
  const service = createAdminDataBackendService(createTransport(calls));

  const fetched = await service.fetchJob("job-42");
  const updated = await service.updateJob("job-42", {
    message: "rerouted",
    project_id: "project-a",
  });
  const deleted = await service.deleteJob("job-42");

  assert.equal(fetched.job.job_id, "job-42");
  assert.equal(updated.job.message, "rerouted");
  assert.equal(deleted.deleted, true);
  assert.deepEqual(calls, [
    ["fetchJob", "job-42"],
    ["updateJob", "job-42", { message: "rerouted", project_id: "project-a" }],
    ["deleteJob", "job-42"],
  ]);
});

test("admin data backend manages result records through transport", async () => {
  const calls: unknown[] = [];
  const service = createAdminDataBackendService(createTransport(calls));

  const updated = await service.updateResult("job-9", { stress: 12.5 });
  const deleted = await service.deleteResult("job-9");

  assert.deepEqual(updated, { job_id: "job-9", result: { stress: 12.5 } });
  assert.equal(deleted.deleted, true);
  assert.deepEqual(calls, [
    ["updateResult", "job-9", { stress: 12.5 }],
    ["deleteResult", "job-9"],
  ]);
});
