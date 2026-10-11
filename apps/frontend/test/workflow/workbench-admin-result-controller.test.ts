import test from "node:test";
import assert from "node:assert/strict";
import {
  refreshWorkbenchResults,
  saveWorkbenchAdminResultRecord,
  deleteWorkbenchAdminResultRecord,
} from "../../src/components/workbench/workbench-admin-result-controller.ts";
import { createAdminDataBackendService } from "../../src/lib/workbench/admin-data-backend-service-core.ts";
import { EMPTY_WORKBENCH_RUNTIME_RECOVERY_STATE } from "../../src/components/workbench/workbench-runtime-recovery.ts";
import type { ResultRecord, ResultListPayload } from "../../src/lib/api/security-results-types.ts";
import type { WorkbenchRuntimeRecoveryState } from "../../src/components/workbench/workbench-runtime-recovery.ts";

function fixture() {
  const known = { job_id: "known-result", result: { metric: 1 } };
  const state = {
    records: [known] as ResultRecord[], selected: known.job_id as string | null,
    recovery: EMPTY_WORKBENCH_RUNTIME_RECOVERY_STATE, pending: false,
    reads: 0, writes: 0, messages: [] as string[],
  };
  let read: () => Promise<ResultListPayload> = async () => ({ results: [known] });
  const deps = {
    adminDataBackendService: createAdminDataBackendService({
      fetchResults: async () => { state.reads += 1; return read(); },
      updateResult: async (job_id, result) => { state.writes += 1; return { job_id, result }; },
      deleteResult: async (job_id) => { state.writes += 1; return { job_id, result: known.result, deleted: true }; },
      fetchJob: async () => { throw new Error("unexpected job read"); },
      updateJob: async () => { throw new Error("unexpected job write"); },
      deleteJob: async () => { throw new Error("unexpected job deletion"); },
    }),
    resultRefreshSeqRef: { current: 0 },
    setResultRecords: (value: ResultRecord[] | ((current: ResultRecord[]) => ResultRecord[])) => {
      state.records = typeof value === "function" ? value(state.records) : value;
    },
    setSelectedAdminResultJobId: (value: string | null | ((current: string | null) => string | null)) => {
      state.selected = typeof value === "function" ? value(state.selected) : value;
    },
    setRuntimeRecovery: (value: WorkbenchRuntimeRecoveryState | ((current: WorkbenchRuntimeRecoveryState) => WorkbenchRuntimeRecoveryState)) => {
      state.recovery = typeof value === "function" ? value(state.recovery) : value;
    },
    setResultReadPending: (value: boolean) => { state.pending = value; },
    selectedAdminResultJobId: known.job_id,
    adminResultDraft: '{"metric":42}',
    downloadTextFile: () => {},
    setMessage: (value: string) => { state.messages.push(value); },
    labels: { resultSaved: "saved", resultDeleted: "deleted", resultJsonDownloaded: "exported", invalidJson: "invalid", initialFailed: "failed" },
    resultScopeLabel: "Results",
  };
  return { deps, state, known, setRead: (next: typeof read) => { read = next; } };
}

test("failed result refresh retains verified records and selection, and reports a recoverable issue", async () => {
  const { deps, state, known, setRead } = fixture();
  const records = state.records;
  setRead(async () => { throw new TypeError("Failed to fetch"); });
  await refreshWorkbenchResults(deps);
  assert.equal(state.records, records);
  assert.equal(state.selected, known.job_id);
  assert.equal(state.pending, false);
  assert.equal(state.recovery.availability, "offline");
  assert.equal(state.recovery.issues[0]?.channel, "results");
  assert.equal(state.reads, 1);
  assert.equal(state.writes, 0);
  setRead(async () => ({ results: [] }));
  await refreshWorkbenchResults(deps);
  assert.deepEqual(state.records, []);
  assert.equal(state.selected, null);
  assert.deepEqual(state.recovery, EMPTY_WORKBENCH_RUNTIME_RECOVERY_STATE);
});

for (const oldFails of [false, true]) {
  for (const currentFails of [false, true]) {
    test(`result refresh ignores stale ${oldFails ? "failure" : "success"} after newer ${currentFails ? "failure" : "success"}`, async () => {
      const { deps, state, known, setRead } = fixture();
      let release!: () => void;
      const gate = new Promise<void>((resolve) => { release = resolve; });
      setRead(async () => {
        await gate;
        if (oldFails) throw new Error("stale read");
        return { results: [{ job_id: "stale", result: {} }] };
      });
      const old = refreshWorkbenchResults(deps);
      assert.equal(state.pending, true);
      setRead(async () => {
        if (currentFails) throw new Error("current read");
        return { results: [{ job_id: "current", result: {} }] };
      });
      await refreshWorkbenchResults(deps);
      const recovery = state.recovery;
      release();
      await old;
      assert.equal(state.records[0]?.job_id, currentFails ? known.job_id : "current");
      assert.equal(state.selected, currentFails ? known.job_id : "current");
      assert.equal(state.recovery, recovery);
      assert.equal(state.pending, false);
      assert.equal(state.reads, 2);
    });
  }
}

test("confirmed result save followed by unavailable history stays successful without repeating the write", async () => {
  const { deps, state, setRead } = fixture();
  setRead(async () => { throw new Error("history unavailable"); });
  await saveWorkbenchAdminResultRecord(deps);
  assert.equal(state.selected, deps.selectedAdminResultJobId);
  assert.deepEqual(state.messages, ["saved"]);
  assert.equal(state.recovery.issues[0]?.channel, "results");
  assert.equal(state.reads, 1);
  assert.equal(state.writes, 1);
});

test("confirmed result deletion removes its local record even if the follow-up list is unavailable", async () => {
  const { deps, state, setRead } = fixture();
  const other = { job_id: "other-result", result: {} };
  state.records.push(other);
  setRead(async () => { throw new Error("history unavailable"); });
  await deleteWorkbenchAdminResultRecord(deps);
  assert.deepEqual(state.records, [other]);
  assert.equal(state.selected, null);
  assert.deepEqual(state.messages, ["deleted"]);
  assert.equal(state.recovery.issues[0]?.channel, "results");
  assert.equal(state.reads, 1);
  assert.equal(state.writes, 1);
});

for (const oldFails of [false, true]) {
  test(`stale result ${oldFails ? "failure" : "success"} cannot finish a newer pending read`, async () => {
    const { deps, state, known, setRead } = fixture();
    let releaseOld!: () => void;
    let releaseNew!: () => void;
    const oldGate = new Promise<void>((resolve) => { releaseOld = resolve; });
    const newGate = new Promise<void>((resolve) => { releaseNew = resolve; });
    setRead(async () => {
      await oldGate;
      if (oldFails) throw new Error("old failure");
      return { results: [] };
    });
    const oldRead = refreshWorkbenchResults(deps);
    setRead(async () => { await newGate; return { results: [] }; });
    const newRead = refreshWorkbenchResults(deps);
    releaseOld();
    await oldRead;
    assert.equal(state.pending, true);
    assert.deepEqual(state.records, [known]);
    assert.equal(state.selected, known.job_id);
    assert.equal(state.recovery.issues.length, 0);
    releaseNew();
    await newRead;
    assert.equal(state.pending, false);
    assert.equal(state.selected, null);
  });
}

test("result recovery clears only its own channel and keeps unrelated backend warnings", async () => {
  const { deps, state, setRead } = fixture();
  state.recovery = { availability: "degraded", lastFailureAt: "2026-10-11T00:00:00Z", issues: [{
    channel: "projects", kind: "http", lastFailureAt: "2026-10-11T00:00:00Z", message: "project store lost",
    recoveryHint: "retry", retryable: true, scopeLabel: "Projects", statusCode: 503,
  }] };
  setRead(async () => { throw new Error("result store lost"); });
  await refreshWorkbenchResults(deps);
  assert.equal(state.recovery.issues.length, 2);
  setRead(async () => ({ results: [] }));
  await refreshWorkbenchResults(deps);
  assert.equal(state.recovery.issues.length, 1);
  assert.equal(state.recovery.issues[0]?.channel, "projects");
  assert.equal(state.recovery.availability, "degraded");
});
