import test from "node:test";
import assert from "node:assert/strict";

import { cancelWorkbenchJob, readWorkbenchJobHistory } from "../../src/components/workbench/workbench-job-history-controller.ts";

function cancelledJob() {
  return {
    job_id: "job-a",
    status: "cancelled" as const,
    worker_id: "worker-a",
    progress: 1,
  };
}

test("accepted cancellation invalidates polling only after the backend responds", async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const token = { current: 4 };
  const messages: string[] = [];
  const operation = cancelWorkbenchJob({
    jobId: "job-a",
    jobHistoryBackendService: {
      cancelJob: async () => {
        await gate;
        return { job: cancelledJob() };
      },
      fetchHistory: async () => ({ jobs: [] }),
    },
    jobPollTokenRef: token,
    labels: {
      initialFailed: "failed",
      jobCancelled: "cancelled",
      requestTimedOut: "timed out",
    },
    refreshJobHistory: async () => {},
    setResult: () => {},
    setJob: () => {},
    setMessage: (message) => {
      assert.equal(typeof message, "string");
      if (typeof message === "string") messages.push(message);
    },
  });

  await Promise.resolve();
  assert.equal(token.current, 4);
  release();
  assert.deepEqual(await operation, { ok: true, jobId: "job-a" });
  assert.equal(token.current, 5);
  assert.deepEqual(messages, ["cancelled"]);
});

test("rejected cancellation preserves active polling and exposes failure", async () => {
  const token = { current: 7 };
  const messages: string[] = [];
  const result = await cancelWorkbenchJob({
    jobId: "job-a",
    jobHistoryBackendService: {
      cancelJob: async () => { throw new Error("cancel rejected"); },
      fetchHistory: async () => ({ jobs: [] }),
    },
    jobPollTokenRef: token,
    labels: {
      initialFailed: "failed",
      jobCancelled: "cancelled",
      requestTimedOut: "timed out",
    },
    refreshJobHistory: async () => {},
    setResult: () => {},
    setJob: () => {},
    setMessage: (message) => {
      assert.equal(typeof message, "string");
      if (typeof message === "string") messages.push(message);
    },
  });

  assert.equal(result.ok, false);
  if (result.ok) assert.fail("rejected cancellation must not report success");
  assert.equal(result.error.message, "cancel rejected");
  assert.equal(token.current, 7);
  assert.deepEqual(messages, ["cancel rejected"]);
});

for (const failed of [false, true]) {
  test(`late cancellation ${failed ? "failure" : "success"} cannot replace a newer job observation`, async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const token = { current: 1 };
    const writes: unknown[] = [];
    const pending = cancelWorkbenchJob({ jobId: "job-a", jobPollTokenRef: token,
      labels: { initialFailed: "failed", jobCancelled: "cancelled", requestTimedOut: "timeout" },
      jobHistoryBackendService: { fetchHistory: async () => ({ jobs: [] }), cancelJob: async () => {
        await gate;
        if (failed) throw new Error("old cancellation failed");
        return { job: cancelledJob() };
      } },
      refreshJobHistory: async () => {}, setJob: (value) => writes.push(value), setResult: (value) => writes.push(value), setMessage: (value) => writes.push(value) });
    token.current += 1;
    release();
    const outcome = await pending;
    assert.equal(token.current, 2, "the late cancellation must not stop the new job's polling");
    assert.deepEqual(writes, []);
    assert.equal(outcome.ok, !failed);
    if (outcome.ok) assert.deepEqual(outcome, { ok: true, jobId: "job-a", contextChanged: true });
  });
}

test("cancellation reports changed context after its final history refresh", async () => {
  const token = { current: 1 };
  const outcome = await cancelWorkbenchJob({ jobId: "job-a", jobPollTokenRef: token,
    labels: { initialFailed: "failed", jobCancelled: "cancelled", requestTimedOut: "timeout" },
    jobHistoryBackendService: { fetchHistory: async () => ({ jobs: [] }), cancelJob: async () => ({ job: cancelledJob() }) },
    refreshJobHistory: async () => { token.current += 1; }, setJob: () => {}, setResult: () => {}, setMessage: () => {} });
  assert.deepEqual(outcome, { ok: true, jobId: "job-a", contextChanged: true });
});

test("cancellation rejects a mismatched job response without stopping active polling", async () => {
  const token = { current: 1 };
  const jobs: unknown[] = [];
  const outcome = await cancelWorkbenchJob({ jobId: "job-a", jobPollTokenRef: token,
    labels: { initialFailed: "failed", jobCancelled: "cancelled", requestTimedOut: "timeout" },
    jobHistoryBackendService: { fetchHistory: async () => ({ jobs: [] }),
      cancelJob: async () => ({ job: { ...cancelledJob(), job_id: "other" } }) },
    refreshJobHistory: async () => {}, setJob: (value) => jobs.push(value), setResult: (value) => jobs.push(value), setMessage: () => {} });
  assert.equal(outcome.ok, false);
  assert.equal(token.current, 1);
  assert.deepEqual(jobs, []);
});

function cancellationFixture(overrides: Record<string, unknown> = {}) {
  const token = { current: 1 };
  const jobs: unknown[] = [];
  const results: unknown[] = [];
  const messages: unknown[] = [];
  const args = { jobId: "job-a", jobPollTokenRef: token,
    labels: { initialFailed: "failed", jobCancelled: "cancelled", requestTimedOut: "timeout" },
    jobHistoryBackendService: { fetchHistory: async () => ({ jobs: [] }), cancelJob: async () => ({ job: cancelledJob() }) },
    refreshJobHistory: async () => {}, setJob: (value: unknown) => jobs.push(value),
    setResult: (value: unknown) => results.push(value), setMessage: (value: unknown) => messages.push(value),
    ...overrides,
  } as Parameters<typeof cancelWorkbenchJob>[0];
  return { args, token, jobs, results, messages };
}

for (const status of ["queued", "preprocessing", "partitioning", "solving", "postprocessing", "completed", "failed"]) {
  test(`HTTP success with ${status} is not confirmed cancellation and keeps observing`, async () => {
    const fixture = cancellationFixture({ jobHistoryBackendService: {
      fetchHistory: async () => ({ jobs: [] }), cancelJob: async () => ({ job: { ...cancelledJob(), status } }),
    } });
    const outcome = await cancelWorkbenchJob(fixture.args);
    assert.equal(outcome.ok, false);
    if (outcome.ok) assert.fail("only cancelled confirms cancellation");
    assert.match(outcome.error.message, /JOB_CANCELLATION_NOT_CONFIRMED/u);
    assert.match(outcome.error.message, new RegExp(status));
    assert.equal(fixture.token.current, 1);
    assert.deepEqual(fixture.jobs, []);
    assert.deepEqual(fixture.results, []);
    assert.equal(fixture.messages.includes("cancelled"), false);
  });
}

for (const invalid of ["negative-progress", "overflow-progress", "nan-progress", "missing-progress", "unknown-status", "contradictory-detail"]) {
  test(`cancellation rejects ${invalid} before changing the active observation`, async () => {
    const job: any = cancelledJob();
    if (invalid === "negative-progress") job.progress = -1;
    if (invalid === "overflow-progress") job.progress = 2;
    if (invalid === "nan-progress") job.progress = Number.NaN;
    if (invalid === "missing-progress") delete job.progress;
    if (invalid === "unknown-status") job.status = "cancel_requested";
    if (invalid === "contradictory-detail") job.status_detail = { lifecycle: "active", active: true, terminal: false };
    const fixture = cancellationFixture({ jobHistoryBackendService: {
      fetchHistory: async () => ({ jobs: [] }), cancelJob: async () => ({ job }),
    } });
    const outcome = await cancelWorkbenchJob(fixture.args);
    assert.equal(outcome.ok, false);
    if (outcome.ok) assert.fail("an invalid response is not confirmed cancellation");
    assert.match(outcome.error.message, /JOB_CANCELLATION_INVALID/u);
    assert.equal(fixture.token.current, 1);
    assert.deepEqual(fixture.jobs, []);
    assert.deepEqual(fixture.results, []);
  });
}

for (const refreshFails of [false, true]) {
  test(`confirmed cancellation clears the active preview and stays successful when refresh ${refreshFails ? "fails" : "succeeds"}`, async () => {
    const fixture = cancellationFixture({ refreshJobHistory: async () => {
      if (refreshFails) throw new Error("history unavailable after accepted cancellation");
    } });
    assert.deepEqual(await cancelWorkbenchJob(fixture.args), { ok: true, jobId: "job-a" });
    assert.deepEqual(fixture.results, [null]);
    assert.deepEqual(fixture.messages, ["cancelled"]);
    assert.equal(fixture.token.current, 2);
  });
}

test("a timed-out cancellation leaves observation intact and can be explicitly retried", async () => {
  let attempts = 0;
  const fixture = cancellationFixture({ jobHistoryBackendService: {
    fetchHistory: async () => ({ jobs: [] }), cancelJob: async () => {
      if (++attempts === 1) throw new Error("request timed out: /jobs/job-a/cancel");
      return { job: cancelledJob() };
    },
  } });
  assert.equal((await cancelWorkbenchJob(fixture.args)).ok, false);
  assert.equal(fixture.token.current, 1);
  assert.deepEqual(fixture.results, []);
  assert.equal(attempts, 1);
  assert.equal((await cancelWorkbenchJob(fixture.args)).ok, true);
  assert.equal(attempts, 2);
});

for (const invalid of ["null", "missing-list", "null-list", "non-array", "null-record", "missing-id", "duplicate-id", "invalid-status", "invalid-progress"]) {
  test(`history refresh rejects ${invalid} before publishing catalog records`, () => {
    let payload: any = { jobs: [cancelledJob()] };
    if (invalid === "null") payload = null;
    if (invalid === "missing-list") delete payload.jobs;
    if (invalid === "null-list") payload.jobs = null;
    if (invalid === "non-array") payload.jobs = {};
    if (invalid === "null-record") payload.jobs = [null];
    if (invalid === "missing-id") delete payload.jobs[0].job_id;
    if (invalid === "duplicate-id") payload.jobs.push(cancelledJob());
    if (invalid === "invalid-status") payload.jobs[0].status = "cancel_requested";
    if (invalid === "invalid-progress") payload.jobs[0].progress = -1;
    assert.throws(() => readWorkbenchJobHistory(payload), /JOB_HISTORY_INVALID/u);
  });
}

test("history refresh accepts an authoritative empty catalog and shares validated records", () => {
  const jobs = [cancelledJob()];
  assert.equal(readWorkbenchJobHistory({ jobs }), jobs);
  assert.deepEqual(readWorkbenchJobHistory({ jobs: [] }), []);
});
