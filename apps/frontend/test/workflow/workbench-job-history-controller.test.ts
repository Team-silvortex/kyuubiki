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
