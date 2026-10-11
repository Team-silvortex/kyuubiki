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

const invalidLists: unknown[] = [
  null, [], {}, { results: null }, { results: {} }, { results: [null] },
  { results: [[]] }, { results: [{ result: {} }] }, { results: [{ job_id: " ", result: {} }] },
  { results: [{ job_id: 42, result: {} }] }, { results: [{ job_id: "job", result: null }] },
  { results: [{ job_id: "job", result: [] }] }, { results: [{ job_id: "job", result: "invalid" }] },
  { results: [{ job_id: "job", result: {}, updated_at: "not-a-date" }] },
  { results: [{ job_id: "job", result: {}, inserted_at: 42 }] },
  { results: [{ job_id: "duplicate", result: {} }, { job_id: "duplicate", result: {} }] },
];

for (const [index, payload] of invalidLists.entries()) {
  test(`admin data backend rejects malformed result history ${index + 1} without publishing a partial list`, async () => {
    const transport = createTransport([]);
    let reads = 0;
    transport.fetchResults = async () => { reads += 1; return payload as ResultListPayload; };
    const service = createAdminDataBackendService(transport);
    await assert.rejects(service.fetchResults(), /RESULT_HISTORY_INVALID/u);
    await assert.rejects(service.listResults(), /RESULT_HISTORY_INVALID/u);
    assert.equal(reads, 2, "only explicit reads are permitted");
  });
}

test("admin data backend preserves valid payload identity and accepts empty result objects and lists", async () => {
  const payload: ResultListPayload = { results: [{ job_id: "custom-result", result: {}, updated_at: "2026-10-11T00:00:00Z" }] };
  const transport = createTransport([]);
  transport.fetchResults = async () => payload;
  const service = createAdminDataBackendService(transport);
  assert.equal(await service.fetchResults(), payload);
  assert.equal(await service.listResults(), payload.results);
  payload.results = [];
  assert.equal(await service.listResults(), payload.results);
});

test("admin data backend preserves transport failure identity without automatic retries", async () => {
  const failure = new Error("store unavailable");
  let reads = 0;
  const transport = createTransport([]);
  transport.fetchResults = async () => { reads += 1; throw failure; };
  const service = createAdminDataBackendService(transport);
  await assert.rejects(service.fetchResults(), (error) => error === failure);
  assert.equal(reads, 1);
});

const invalidJobReceipts: Array<[string, unknown]> = [
  ["null", null], ["array", []], ["missing-job", {}], ["null-job", { job: null }],
  ["array-job", { job: [] }], ["wrong-job", { job: jobState("wrong-job") }],
  ["missing-progress", { job: { ...jobState("job"), progress: undefined } }],
  ["invalid-progress", { job: { ...jobState("job"), progress: 2 } }],
  ["invalid-status", { job: { ...jobState("job"), status: "unknown" } }],
  ["contradictory-detail", { job: { ...jobState("job"), status_detail: { active: false } } }],
];

for (const action of ["save", "delete"] as const) {
  for (const [label, payload] of invalidJobReceipts) {
    test(`admin job ${action} rejects ${label} receipt without retry or follow-up reads`, async () => {
      let attempts = 0;
      const calls: unknown[] = [];
      const transport = createTransport(calls);
      if (action === "save") transport.updateJob = async () => { attempts += 1; return payload as JobEnvelope; };
      else transport.deleteJob = async () => {
        attempts += 1;
        return (payload && !Array.isArray(payload) ? { ...payload, deleted: true } : payload) as Awaited<ReturnType<typeof transport.deleteJob>>;
      };
      const service = createAdminDataBackendService(transport);
      await assert.rejects(action === "save" ? service.updateJob("job", { message: "review" }) : service.deleteJob("job"),
        /JOB_RECORD_MUTATION_UNCONFIRMED/u);
      assert.equal(attempts, 1);
      assert.deepEqual(calls, []);
    });
  }
}

for (const deleted of [undefined, false, "true", 1]) {
  test(`admin job deletion requires explicit boolean confirmation: ${String(deleted)}`, async () => {
    const transport = createTransport([]);
    transport.deleteJob = async () => ({ job: jobState("job"), deleted }) as Awaited<ReturnType<typeof transport.deleteJob>>;
    await assert.rejects(createAdminDataBackendService(transport).deleteJob("job"), /JOB_RECORD_MUTATION_UNCONFIRMED/u);
  });
}

test("admin job receipts preserve valid payload identity and transport errors", async () => {
  const transport = createTransport([]);
  const saved = { job: jobState("job") };
  const deleted = { ...saved, deleted: true };
  transport.updateJob = async () => saved;
  transport.deleteJob = async () => deleted;
  const service = createAdminDataBackendService(transport);
  assert.equal(await service.updateJob("job", {}), saved);
  assert.equal(await service.deleteJob("job"), deleted);
  const failure = new Error("write response lost");
  let attempts = 0;
  transport.updateJob = async () => { attempts += 1; throw failure; };
  transport.deleteJob = async () => { attempts += 1; throw failure; };
  const unavailable = createAdminDataBackendService(transport);
  await assert.rejects(unavailable.updateJob("job", {}), (error) => error === failure);
  await assert.rejects(unavailable.deleteJob("job"), (error) => error === failure);
  assert.equal(attempts, 2);
});
