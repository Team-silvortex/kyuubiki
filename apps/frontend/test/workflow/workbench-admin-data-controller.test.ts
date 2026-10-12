import test from "node:test";
import assert from "node:assert/strict";
import { openWorkbenchLinkedContext, resolveScriptLinkedJob, type AdminDataControllerDeps } from "../../src/components/workbench/workbench-admin-data-controller.ts";
import { workbenchAdminRecordSources } from "../../src/components/workbench/workbench-admin-record-source.ts";
import { workbenchLibraryRecordSources } from "../../src/components/workbench/workbench-library-record-source.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import { createWorkbenchFeedback, withWorkbenchFeedbackInvalidation } from "../../src/components/workbench/workbench-feedback.ts";
import type { ModelVersionRecord } from "../../src/lib/api/project-types.ts";

function fixture({ jobs = true, results = true, projects = true } = {}) {
  const calls: unknown[] = [], messages: string[] = [];
  const job = { job_id: "job-a", project_id: "project-a", model_version_id: "version-a", simulation_case_id: "case-a", status: "completed" as const, progress: 1, worker_id: null };
  const feedback = createWorkbenchFeedback();
  const deps: AdminDataControllerDeps = {
    selectedAdminJob: job, selectedAdminJobId: job.job_id, selectedAdminResultJobId: job.job_id,
    jobHistory: [job], resultRecords: [{ job_id: job.job_id, result: {} }],
    projects: [{ project_id: "project-a", name: "Project A", models: [], description: "", inserted_at: "2026-10-12", updated_at: "2026-10-12" }],
    projectContext: createWorkbenchProjectContext({ projectId: "old-project", modelId: "old-model", versionId: "old-version" }), feedback,
    labels: { linkedProjectMissing: "project missing", linkedProjectOpened: "project opened", missingResultJob: "result missing",
      noJobProject: "job project missing", noJobVersion: "job version missing", noRecordContext: "context missing", noResultProject: "result project missing",
      noResultVersion: "result version missing", recordContextApplied: "context applied", selectJobFirst: "select job", sourceChanged: "source changed" },
    refreshVersions: async () => assert.fail("project-only navigation must not load or bind a saved model"),
    openModelVersionById: async (id, validate, scope) => {
      calls.push(["version", id]);
      try { validate?.({ version_id: id, model_id: "model-a", project_id: "project-a" } as ModelVersionRecord); }
      catch (error) { return { ok: false, error: error as Error }; }
      scope?.onApplied();
      return { ok: true };
    },
    setAdminFilterProjectId: (id) => calls.push(["project-filter", id]),
    setAdminFilterModelVersionId: (id) => calls.push(["version-filter", id]),
    setAdminJobCaseId: (id) => calls.push(["case", id]), setLibraryTab: (tab) => { calls.push(["library", tab]); feedback.invalidate(); },
    setSelectedProjectId: (id) => calls.push(["project", id]), setSelectedModelId: (id) => calls.push(["model", id]),
    setSelectedVersionId: (id) => calls.push(["selected-version", id]), setModelVersions: (list) => calls.push(["versions", list]),
    setSidebarSection: (tab) => { calls.push(["sidebar", tab]); feedback.invalidate(); },
    setMessage: withWorkbenchFeedbackInvalidation(feedback, (message: string) => messages.push(message)),
  };
  if (jobs) workbenchAdminRecordSources.mark(deps.jobHistory);
  if (results) workbenchAdminRecordSources.mark(deps.resultRecords);
  if (projects) workbenchLibraryRecordSources.mark(deps.projects);
  return { deps, job, calls, messages };
}

for (const target of ["job", "result"]) for (const mode of ["project", "version", "apply"]) {
  test(`linked ${target}/${mode} rejects unknown job source before every effect`, async () => {
    const { deps, calls, messages } = fixture({ jobs: false });
    const result = await openWorkbenchLinkedContext({ target, mode }, deps);
    assert.equal(result.ok, false);
    assert.deepEqual(calls, []);
    assert.match(messages[0], /WORKBENCH_RECORD_SOURCE_CHANGED/u);
  });
  test(`linked ${target}/${mode} uses the shared guarded route`, async () => {
    const { deps, calls } = fixture();
    assert.deepEqual(await openWorkbenchLinkedContext({ target, mode }, deps), { ok: true, jobId: "job-a", projectId: "project-a", modelVersionId: "version-a" });
    if (mode === "project") {
      assert.deepEqual(deps.projectContext.current(), { projectId: "project-a", modelId: null, versionId: null });
      assert.ok(calls.some((call) => JSON.stringify(call) === '["model",null]'));
      assert.ok(calls.some((call) => JSON.stringify(call) === '["selected-version",null]'));
      assert.equal(calls.some((call) => Array.isArray(call) && call[0] === "version"), false);
    } else assert.deepEqual(calls[0], ["version", "version-a"]);
  });
}

for (const mode of ["project", "version", "apply"]) for (const missing of ["result", "result-source", "job"]) {
  test(`result ${mode} cannot borrow a same-ID task when ${missing} is missing`, async () => {
    const { deps, calls } = fixture({ results: missing !== "result-source" });
    if (missing === "result") deps.resultRecords = [];
    if (missing === "job") deps.jobHistory = [];
    const result = await openWorkbenchLinkedContext({ target: "result", mode }, deps);
    assert.equal(result.ok, false);
    assert.deepEqual(calls, []);
  });
}

for (const kind of ["jobs", "projects"] as const) for (const unreadable of [false, true]) {
  test(`linked context blocks ${kind} source stale or unreadable=${unreadable}`, async () => {
    const { deps, calls } = fixture();
    const source = kind === "jobs" ? workbenchAdminRecordSources.sourceOf(deps.jobHistory)! : workbenchLibraryRecordSources.sourceOf(deps.projects)!;
    const previous = source.isCurrent;
    source.isCurrent = () => { if (unreadable) throw new Error("unreadable"); return false; };
    try {
      assert.equal((await openWorkbenchLinkedContext({ mode: "project" }, deps)).ok, false);
      assert.deepEqual(calls, []);
    } finally { source.isCurrent = previous; }
  });
}

for (const problem of ["wrong-version", "wrong-project", "missing-model", "missing-project", "invalid-model"]) {
  test(`linked version validates ${problem} before context filters are changed`, async () => {
    const { deps, calls, messages } = fixture();
    deps.openModelVersionById = async (id, validate) => {
      const version: any = { version_id: id, model_id: "model-a", project_id: "project-a" };
      if (problem === "wrong-version") version.version_id = "other-version";
      if (problem === "wrong-project") version.project_id = "other-project";
      if (problem === "missing-model") delete version.model_id;
      if (problem === "missing-project") delete version.project_id;
      if (problem === "invalid-model") version.model_id = 12;
      try { validate?.(version); } catch (error) { return { ok: false, error: error as Error }; }
      assert.fail("invalid version must not be applied");
    };
    const result = await openWorkbenchLinkedContext({ mode: "apply" }, deps);
    assert.equal(result.ok, false);
    if (!result.ok) assert.match(result.error.message, /WORKBENCH_LINKED_VERSION_INVALID/u);
    assert.deepEqual(calls, []);
    assert.deepEqual(messages, []);
  });
}

test("linked apply waits for successful loading before filters or success feedback", async () => {
  const { deps, calls, messages } = fixture();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  deps.openModelVersionById = async (_id, _validate, scope) => { await gate; scope?.onApplied(); return { ok: true }; };
  const pending = openWorkbenchLinkedContext({ mode: "apply" }, deps);
  assert.deepEqual(calls, []);
  release();
  assert.equal((await pending).ok, true);
  assert.deepEqual(calls.slice(0, 3), [["project-filter", "project-a"], ["version-filter", "version-a"], ["case", "case-a"]]);
  assert.deepEqual(messages, ["context applied"]);
});

test("linked version loading failure preserves filters and does not report context applied", async () => {
  const { deps, calls, messages } = fixture();
  const failure = new Error("version unavailable");
  deps.openModelVersionById = async () => ({ ok: false, error: failure });
  const result = await openWorkbenchLinkedContext({}, deps);
  assert.equal(result.ok, false);
  if (!result.ok) assert.equal(result.error, failure);
  assert.deepEqual(calls, []);
  assert.deepEqual(messages, []);
});

test("linked project navigation cannot use an unverified catalog or bind a saved model", async () => {
  const { deps, calls } = fixture({ projects: false });
  assert.equal((await openWorkbenchLinkedContext({ mode: "project" }, deps)).ok, false);
  assert.deepEqual(calls, []);
});

test("explicit PWDT result identity requires a real result record", () => {
  const { deps } = fixture();
  assert.equal(resolveScriptLinkedJob({ target: "result", resultJobId: "job-a" }, deps), deps.jobHistory[0]);
  deps.resultRecords = [];
  assert.equal(resolveScriptLinkedJob({ target: "result", resultJobId: "job-a" }, deps), null);
});

for (const owner of ["feedback", "project"] as const) {
  test(`disposed ${owner} owner cannot start linked project navigation`, async () => {
    const { deps, calls, messages } = fixture();
    if (owner === "feedback") deps.feedback.dispose();
    else deps.projectContext.dispose();
    const result = await openWorkbenchLinkedContext({ mode: "project" }, deps);
    assert.equal(result.ok, false);
    if (!result.ok) assert.match(result.error.message, /WORKBENCH_CONTEXT_CHANGED/u);
    assert.deepEqual(calls, []);
    assert.deepEqual(messages, []);
  });
}

test("unexpected linked opener rejection is a structured failure without filter mutation", async () => {
  const { deps, calls, messages } = fixture();
  const error = new Error("version opener rejected");
  deps.openModelVersionById = async () => { throw error; };
  const result = await openWorkbenchLinkedContext({}, deps);
  assert.equal(result.ok, false);
  if (!result.ok) assert.equal(result.error, error);
  assert.deepEqual(calls, []);
  assert.deepEqual(messages, [error.message]);
});

test("Apply Context commits filters with the payload and never reapplies them after a delayed list read", async () => {
  const { deps, calls, messages } = fixture();
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  deps.openModelVersionById = async (_id, _validate, scope) => { scope?.onApplied(); await gate; return { ok: true }; };
  const pending = openWorkbenchLinkedContext({}, deps);
  assert.deepEqual(calls.slice(0, 3), [["project-filter", "project-a"], ["version-filter", "version-a"], ["case", "case-a"]]);
  assert.deepEqual(messages, []);
  const applied = [...calls];
  deps.setMessage("newer feedback while the list read waits");
  release();
  assert.equal((await pending).ok, true);
  assert.deepEqual(calls, applied, "late completion cannot repeat context setters over newer editing");
  assert.deepEqual(messages, ["newer feedback while the list read waits"]);
});
