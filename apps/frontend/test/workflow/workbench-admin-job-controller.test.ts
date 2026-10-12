import test from "node:test";
import assert from "node:assert/strict";
import { deleteWorkbenchAdminJobRecord, saveWorkbenchAdminJobRecord } from "../../src/components/workbench/workbench-admin-job-controller.ts";
import { removeConfirmedWorkbenchJobRecord } from "../../src/components/workbench/workbench-job-history-controller.ts";
import { createAdminDataBackendService } from "../../src/lib/workbench/admin-data-backend-service-core.ts";
import type { WorkbenchJobRecordPatch } from "../../src/lib/workbench/admin-data-backend-service-core.ts";
import type { JobState } from "../../src/lib/api/fem-shared.ts";
import type { ResultRecord } from "../../src/lib/api/security-results-types.ts";
import { createWorkbenchFeedback } from "../../src/components/workbench/workbench-feedback.ts";

function fixture() {
  const job = { job_id: "selected", status: "completed", progress: 1 } as JobState;
  const other = { ...job, job_id: "other" };
  const state = {
    jobs: [job, other], results: [job, other].map(({ job_id }) => ({ job_id, result: {} })),
    selectedJob: "selected" as string | null, selectedResult: "selected" as string | null,
    messages: [] as string[], patches: [] as WorkbenchJobRecordPatch[], deletions: [] as string[], reads: [] as string[],
  };
  const jobSeq = { current: 3 };
  const resultSeq = { current: 7 };
  let save = async (jobId: string, patch: WorkbenchJobRecordPatch) => ({ job: { ...job, job_id: jobId, ...patch } });
  let remove = async (jobId: string) => ({ job: { ...job, job_id: jobId }, deleted: true });
  const source = { isCurrent: () => true };
  const deps = {
    recordSource: source, draftSource: source,
    feedback: createWorkbenchFeedback(),
    selectedAdminJobId: "selected" as string | null,
    adminJobMessage: "reviewed", adminJobProjectId: "project", adminJobModelVersionId: "", adminJobCaseId: "case",
    labels: { jobSaved: "saved", jobDeleted: "deleted", initialFailed: "failed", sourceChanged: "source changed" },
    adminDataBackendService: createAdminDataBackendService({
      updateJob: async (id, patch) => { state.patches.push(patch); return save(id, patch); },
      deleteJob: async (id) => { state.deletions.push(id); return remove(id); },
      fetchJob: async () => { throw new Error("unexpected fetch"); },
      fetchResults: async () => { throw new Error("unexpected fetch"); },
      updateResult: async () => { throw new Error("unexpected write"); },
      deleteResult: async () => { throw new Error("unexpected write"); },
    }),
    refreshJobHistory: async () => { state.reads.push("jobs"); },
    refreshResults: async () => { state.reads.push("results"); },
    setMessage: (value: string) => { state.messages.push(value); },
    removeConfirmedJobRecord: (jobId: string) => removeConfirmedWorkbenchJobRecord(jobId, {
      jobHistoryRefreshSeqRef: jobSeq,
      setJobHistory: (next) => { state.jobs = typeof next === "function" ? next(state.jobs) : next; },
      setSelectedAdminJobId: (next) => { state.selectedJob = typeof next === "function" ? next(state.selectedJob) : next; },
    }),
    resultRefreshSeqRef: resultSeq,
    setResultRecords: (next: ResultRecord[] | ((current: ResultRecord[]) => ResultRecord[])) => {
      state.results = typeof next === "function" ? next(state.results) : next;
    },
    setSelectedAdminResultJobId: (next: string | null | ((current: string | null) => string | null)) => {
      state.selectedResult = typeof next === "function" ? next(state.selectedResult) : next;
    },
  };
  return { deps, state, jobSeq, resultSeq, setSave: (next: typeof save) => { save = next; }, setDelete: (next: typeof remove) => { remove = next; } };
}

test("job save uses explicit null for optional version and does not omit invalid required editing", async () => {
  const { deps, state } = fixture();
  await saveWorkbenchAdminJobRecord({ ...deps, adminJobProjectId: "", adminJobCaseId: "" });
  assert.deepEqual(state.patches, [{ message: "reviewed", project_id: "", model_version_id: null, simulation_case_id: "" }]);
  assert.deepEqual(state.reads, ["jobs"]);
  assert.deepEqual(state.messages, ["saved"]);
});

test("confirmed job save remains successful when follow-up history is unavailable", async () => {
  const { deps, state } = fixture();
  deps.refreshJobHistory = async () => { state.reads.push("jobs"); throw new Error("read unavailable"); };
  await saveWorkbenchAdminJobRecord(deps);
  assert.equal(state.patches.length, 1);
  assert.deepEqual(state.messages, ["saved"]);
});

for (const action of ["save", "delete"] as const) {
  for (const unconfirmed of [false, true]) {
    test(`${action} ${unconfirmed ? "unconfirmed receipt" : "transport failure"} cannot remove records or run follow-up reads`, async () => {
      const { deps, state, setSave, setDelete, jobSeq, resultSeq } = fixture();
      const failure = new Error("write unavailable");
      if (action === "save") setSave(async () => {
        if (!unconfirmed) throw failure;
        return { job: { job_id: "wrong" } as JobState };
      });
      else setDelete(async () => {
        if (!unconfirmed) throw failure;
        return { job: { job_id: "wrong" } as JobState, deleted: true };
      });
      await (action === "save" ? saveWorkbenchAdminJobRecord(deps) : deleteWorkbenchAdminJobRecord(deps));
      assert.equal(state.jobs.length, 2);
      assert.equal(state.results.length, 2);
      assert.equal(state.selectedJob, "selected");
      assert.equal(state.selectedResult, "selected");
      assert.deepEqual(state.reads, []);
      assert.equal(state.patches.length + state.deletions.length, 1);
      assert.equal(jobSeq.current, 3);
      assert.equal(resultSeq.current, 7);
      assert.match(state.messages[0], unconfirmed ? /JOB_RECORD_MUTATION_UNCONFIRMED/u : /write unavailable/u);
    });
  }
}

for (const changedSelection of [false, true]) {
  test(`confirmed job deletion fences both read collections and preserves other records; selection changed=${changedSelection}`, async () => {
    const { deps, state, jobSeq, resultSeq } = fixture();
    if (changedSelection) state.selectedJob = state.selectedResult = "other";
    for (const name of ["refreshJobHistory", "refreshResults"] as const) {
      deps[name] = async () => { state.reads.push(name); throw new Error("read unavailable"); };
    }
    await deleteWorkbenchAdminJobRecord(deps);
    assert.deepEqual(state.jobs.map((job) => job.job_id), ["other"]);
    assert.deepEqual(state.results.map((result) => result.job_id), ["other"]);
    assert.equal(state.selectedJob, changedSelection ? "other" : null);
    assert.equal(state.selectedResult, changedSelection ? "other" : null);
    assert.equal(jobSeq.current, 4);
    assert.equal(resultSeq.current, 8);
    assert.deepEqual(state.messages, ["deleted"]);
    assert.deepEqual(state.deletions, ["selected"]);
    assert.deepEqual(state.reads, ["refreshJobHistory", "refreshResults"]);
  });
}

test("deletion confirmation starts result recovery without waiting for a blocked job read", async () => {
  const { deps, state } = fixture();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  let resultsStarted!: () => void;
  const observed = new Promise<void>((resolve) => { resultsStarted = resolve; });
  deps.refreshJobHistory = async () => { await gate; };
  deps.refreshResults = async () => { resultsStarted(); };
  const operation = deleteWorkbenchAdminJobRecord(deps);
  await observed;
  assert.deepEqual(state.results.map((record) => record.job_id), ["other"]);
  assert.deepEqual(state.messages, []);
  release();
  await operation;
  assert.deepEqual(state.messages, ["deleted"]);
});

test("a slow job save captures the submitted job and fields before further editing", async () => {
  const { deps, state, setSave } = fixture();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  setSave(async (id) => { await gate; return { job: { job_id: id, status: "completed", progress: 1 } as JobState }; });
  const operation = saveWorkbenchAdminJobRecord(deps);
  deps.selectedAdminJobId = "other";
  deps.adminJobMessage = "later typing";
  release();
  await operation;
  assert.equal(state.patches[0].message, "reviewed");
  assert.deepEqual(state.messages, ["saved"]);
});

test("no selected job is a no-op rather than a write or collection reset", async () => {
  const { deps, state } = fixture();
  deps.selectedAdminJobId = null;
  await saveWorkbenchAdminJobRecord(deps);
  await deleteWorkbenchAdminJobRecord(deps);
  assert.deepEqual(state.patches, []);
  assert.deepEqual(state.deletions, []);
  assert.deepEqual(state.reads, []);
  assert.deepEqual(state.messages, []);
  assert.equal(state.jobs.length, 2);
});
