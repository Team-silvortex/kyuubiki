import test from "node:test";
import assert from "node:assert/strict";

import { cancelWorkbenchJob } from "../../src/components/workbench/workbench-job-history-controller.ts";

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
      refreshJobHistory: async () => {}, setJob: (value) => writes.push(value), setMessage: (value) => writes.push(value) });
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
    refreshJobHistory: async () => { token.current += 1; }, setJob: () => {}, setMessage: () => {} });
  assert.deepEqual(outcome, { ok: true, jobId: "job-a", contextChanged: true });
});

test("cancellation rejects a mismatched job response without stopping active polling", async () => {
  const token = { current: 1 };
  const jobs: unknown[] = [];
  const outcome = await cancelWorkbenchJob({ jobId: "job-a", jobPollTokenRef: token,
    labels: { initialFailed: "failed", jobCancelled: "cancelled", requestTimedOut: "timeout" },
    jobHistoryBackendService: { fetchHistory: async () => ({ jobs: [] }),
      cancelJob: async () => ({ job: { ...cancelledJob(), job_id: "other" } }) },
    refreshJobHistory: async () => {}, setJob: (value) => jobs.push(value), setMessage: () => {} });
  assert.equal(outcome.ok, false);
  assert.equal(token.current, 1);
  assert.deepEqual(jobs, []);
});
