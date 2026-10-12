import test from "node:test";
import assert from "node:assert/strict";
import type { JobEnvelope } from "@/lib/api/fem-shared";
import { cancelWorkbenchAdminJobRecord } from "../../src/components/workbench/workbench-admin-job-controller.ts";
import { readWorkbenchCancellationReceipt } from "../../src/components/workbench/workbench-job-cancellation-receipt.ts";
import { createWorkbenchFeedback, withWorkbenchFeedbackInvalidation } from "../../src/components/workbench/workbench-feedback.ts";

const job = { job_id: "selected", status: "cancelled" as const, progress: 0.5, worker_id: null };
function fixture() {
  const calls: string[] = [], messages: string[] = [];
  const feedback = createWorkbenchFeedback();
  const source = { isCurrent: () => true };
  const deps = {
    selectedAdminJobId: "selected" as string | null, recordSource: source as typeof source | null, feedback,
    cancelJob: async (id: string): Promise<JobEnvelope> => { calls.push(id); return { job }; },
    refreshJobHistory: async () => { calls.push("history"); },
    setMessage: withWorkbenchFeedbackInvalidation(feedback, (value: string) => { messages.push(value); }),
    labels: { jobCancelled: "cancelled", sourceChanged: "source changed", initialFailed: "failed", requestTimedOut: "timeout" },
  };
  return { deps, source, calls, messages };
}

for (const failure of ["missing", "stale", "unreadable"] as const) {
  test(`selected cancellation blocks ${failure} source before a write or refresh`, async () => {
    const { deps, source, calls, messages } = fixture();
    if (failure === "missing") deps.recordSource = null;
    else source.isCurrent = () => { if (failure === "unreadable") throw new Error("unreadable"); return false; };
    await cancelWorkbenchAdminJobRecord(deps);
    assert.deepEqual(calls, []);
    assert.deepEqual(messages, ["source changed"]);
  });
}

test("no selected record does not begin cancellation or replace feedback", async () => {
  const { deps, calls, messages } = fixture();
  const owns = deps.feedback.begin();
  deps.selectedAdminJobId = null;
  await cancelWorkbenchAdminJobRecord(deps);
  assert.equal(owns(), true);
  assert.deepEqual(calls, []);
  assert.deepEqual(messages, []);
});

const malformed: Array<[string, unknown, RegExp]> = [
  ["null envelope", null, /different job/u],
  ["missing job", {}, /different job/u],
  ["null job", { job: null }, /different job/u],
  ["array job", { job: [] }, /different job/u],
  ["wrong identity", { job: { ...job, job_id: "other" } }, /different job/u],
  ...["queued", "preprocessing", "partitioning", "solving", "postprocessing", "completed", "failed"].map((status) =>
    [status, { job: { ...job, status } }, /JOB_CANCELLATION_NOT_CONFIRMED/u] as [string, unknown, RegExp]),
  ...[-1, 2, NaN, undefined].map((progress) =>
    [`progress ${progress}`, { job: { ...job, progress } }, /JOB_CANCELLATION_INVALID/u] as [string, unknown, RegExp]),
  ["unknown status", { job: { ...job, status: "cancel_requested" } }, /JOB_CANCELLATION_INVALID/u],
  ["contradictory status", { job: { ...job, status_detail: { lifecycle: "active", active: true, terminal: false } } }, /JOB_CANCELLATION_INVALID/u],
];
for (const [label, receipt, reason] of malformed) {
  test(`selected cancellation rejects ${label} without confirmation, refresh or replay`, async () => {
    const { deps, calls, messages } = fixture();
    deps.cancelJob = async (id) => { calls.push(id); return receipt as JobEnvelope; };
    assert.throws(() => readWorkbenchCancellationReceipt(receipt, "selected"), reason);
    await cancelWorkbenchAdminJobRecord(deps);
    assert.deepEqual(calls, ["selected"]);
    assert.equal(messages.length, 1);
    assert.match(messages[0], reason);
  });
}

for (const failedRead of [false, true]) {
  test(`confirmed selected cancellation survives follow-up read failure=${failedRead}`, async () => {
    const { deps, calls, messages } = fixture();
    deps.refreshJobHistory = async () => { calls.push("history"); if (failedRead) throw new Error("read unavailable"); };
    await cancelWorkbenchAdminJobRecord(deps);
    assert.deepEqual(calls, ["selected", "history"]);
    assert.deepEqual(messages, ["cancelled"]);
    assert.equal(readWorkbenchCancellationReceipt({ job }, "selected"), job);
  });
}

for (const failure of ["network", "timeout"] as const) {
  test(`selected cancellation ${failure} keeps its read catalog and permits only an explicit retry`, async () => {
    const { deps, calls, messages } = fixture();
    deps.cancelJob = async (id) => { calls.push(id); throw new Error(failure === "timeout" ? "request timed out: cancel" : "write unavailable"); };
    await cancelWorkbenchAdminJobRecord(deps);
    assert.deepEqual(calls, ["selected"]);
    assert.deepEqual(messages, [failure === "timeout" ? "timeout" : "write unavailable"]);
    deps.cancelJob = async (id) => { calls.push(id); return { job }; };
    await cancelWorkbenchAdminJobRecord(deps);
    assert.deepEqual(calls, ["selected", "selected", "history"]);
  });
}

for (const failed of [false, true]) for (const invalidation of ["source", "unreadable", "feedback"] as const) {
  test(`late selected cancellation failed=${failed} cannot publish into ${invalidation} context`, async () => {
    const { deps, source, calls, messages } = fixture();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    deps.cancelJob = async (id) => { calls.push(id); await gate; if (failed) throw new Error("old failure"); return { job }; };
    const pending = cancelWorkbenchAdminJobRecord(deps);
    if (invalidation === "feedback") deps.setMessage("newer message");
    else source.isCurrent = () => { if (invalidation === "unreadable") throw new Error("unreadable"); return false; };
    release();
    await pending;
    assert.deepEqual(calls, invalidation === "feedback" && !failed ? ["selected", "history"] : ["selected"]);
    assert.deepEqual(messages, invalidation === "feedback" ? ["newer message"] : []);
  });
}

for (const invalidation of ["source", "feedback"] as const) {
  test(`selected cancellation checks ${invalidation} ownership after the confirmed read`, async () => {
    const { deps, source, calls, messages } = fixture();
    deps.refreshJobHistory = async () => {
      calls.push("history");
      if (invalidation === "source") source.isCurrent = () => false;
      else deps.setMessage("newer read feedback");
    };
    await cancelWorkbenchAdminJobRecord(deps);
    assert.deepEqual(calls, ["selected", "history"]);
    assert.deepEqual(messages, invalidation === "feedback" ? ["newer read feedback"] : []);
  });
}
