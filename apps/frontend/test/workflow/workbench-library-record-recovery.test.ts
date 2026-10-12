import test from "node:test";
import assert from "node:assert/strict";
import { removeConfirmedWorkbenchLibraryRecord } from "../../src/components/workbench/workbench-library-record-recovery.ts";
import { retainCurrentWorkbenchRecords } from "../../src/components/workbench/workbench-admin-context.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import type { ProjectRecord, ModelVersionRecord, ModelRecord } from "../../src/lib/api/project-types.ts";

function fixture(otherScope = false) {
  const version = (id: string, model = "model", project = "project"): ModelVersionRecord => ({
    version_id: id, model_id: model, project_id: project, name: id, payload: {}, version_number: 1,
    model_schema_version: "kyuubiki.model/v1", kind: "truss_2d", inserted_at: "2026-10-11T00:00:00Z", updated_at: "2026-10-11T00:00:00Z",
  });
  const model = (id = "model", project = "project"): ModelRecord => ({
    model_id: id, project_id: project, name: id, payload: { working: true }, model_schema_version: "kyuubiki.model/v1",
    kind: "truss_2d", inserted_at: "2026-10-11T00:00:00Z", updated_at: "2026-10-11T00:00:00Z",
    latest_version_id: id === "model" ? "latest" : "other", latest_version_number: 2,
    versions: id === "model" ? [version("earlier"), version("latest")] : [version("other", id, project)],
  });
  const projects: ProjectRecord[] = [
    { project_id: "project", name: "Project", models: [model()], inserted_at: "2026-10-11T00:00:00Z", updated_at: "2026-10-11T00:00:00Z" },
    { project_id: "other-project", name: "Other", models: [model("other-model", "other-project")], inserted_at: "2026-10-11T00:00:00Z", updated_at: "2026-10-11T00:00:00Z" },
  ];
  const versions = otherScope ? [version("other", "other-model", "other-project")] : [version("earlier"), version("latest")];
  const state = { projects, versions };
  const deps = {
    projectContext: createWorkbenchProjectContext({ projectId: otherScope ? "other-project" : "project",
      modelId: otherScope ? "other-model" : "model", versionId: otherScope ? "other" : "latest" }),
    projectRefreshSeqRef: { current: 3 }, versionsRefreshSeqRef: { current: 7 },
    versionListModelIdRef: { current: otherScope ? "other-model" : "model" },
    setProjects: (next: ProjectRecord[] | ((current: ProjectRecord[]) => ProjectRecord[])) => {
      state.projects = typeof next === "function" ? next(state.projects) : next;
    },
    setModelVersions: (next: ModelVersionRecord[] | ((current: ModelVersionRecord[]) => ModelVersionRecord[])) => {
      state.versions = typeof next === "function" ? next(state.versions) : next;
    },
  };
  return { state, deps, projects, versions };
}

for (const kind of ["projectId", "modelId", "versionId"] as const) for (const otherScope of [false, true]) {
  test(`confirmed ${kind} removal fences only the affected version scope; otherScope=${otherScope}`, () => {
    const { state, deps, projects, versions } = fixture(otherScope);
    const before = structuredClone(projects);
    const selection = deps.projectContext.current();
    const id = kind === "projectId" ? "project" : kind === "modelId" ? "model" : "latest";
    removeConfirmedWorkbenchLibraryRecord({ ...deps, kind, id, modelId: "model" });
    assert.equal(deps.projectRefreshSeqRef.current, 4);
    assert.equal(deps.versionsRefreshSeqRef.current, otherScope ? 7 : 8);
    assert.deepEqual(deps.projectContext.current(), selection, "record cleanup does not change selection or the live modeling copy");
    assert.deepEqual(projects, before, "record removal is immutable");
    assert.equal(state.projects.at(-1), projects[1]);
    assert.deepEqual(state.versions, otherScope ? versions : kind === "versionId" ? [versions[0]] : []);
    if (kind === "projectId") assert.equal(state.projects.length, 1);
    else if (kind === "modelId") assert.deepEqual(state.projects[0].models, []);
    else {
      const retained = state.projects[0].models![0];
      assert.equal(retained.latest_version_id, null);
      assert.equal(retained.latest_version_number, null);
      assert.deepEqual(retained.versions, [projects[0].models![0].versions![0]]);
      assert.equal(retained.payload, projects[0].models![0].payload);
    }
  });
}

test("confirmed historical version deletion preserves the known latest pointer", () => {
  const { state, deps } = fixture();
  removeConfirmedWorkbenchLibraryRecord({ ...deps, kind: "versionId", id: "earlier", modelId: "model" });
  assert.equal(state.projects[0].models![0].latest_version_id, "latest");
  assert.deepEqual(state.versions.map((version) => version.version_id), ["latest"]);
});

test("library record removal does not publish state after disposal", () => {
  const { state, deps, projects, versions } = fixture();
  deps.projectContext.dispose();
  removeConfirmedWorkbenchLibraryRecord({ ...deps, kind: "projectId", id: "project" });
  assert.equal(state.projects, projects);
  assert.equal(state.versions, versions);
  assert.equal(deps.projectRefreshSeqRef.current, 3);
  assert.equal(deps.versionsRefreshSeqRef.current, 7);
});

for (const kind of ["model_id", "version_id", "job_id"] as const) {
  test(`deferred ${kind} rendering removes obsolete rows while new rows may wait`, () => {
    const removed = { [kind]: "removed", name: "Removed" };
    const retained = { [kind]: "retained", name: "Earlier" };
    const added = { [kind]: "added", name: "New" };
    const deferred = [removed, retained];
    const current = [{ ...retained, name: "Updated" }, added];
    let identityCalls = 0;
    const identity = (record: Record<string, string>) => { identityCalls += 1; return record[kind]; };
    assert.deepEqual(retainCurrentWorkbenchRecords(deferred, current, identity), [retained]);
    assert.equal(identityCalls, deferred.length + current.length, "membership filtering is linear, not a nested scan");
    assert.deepEqual(deferred, [removed, retained]);
    assert.deepEqual(current, [{ ...retained, name: "Updated" }, added]);
    assert.deepEqual(retainCurrentWorkbenchRecords(deferred, [added], identity), [], "another workspace cannot keep old rows");
    assert.deepEqual(retainCurrentWorkbenchRecords(deferred, [], identity), []);
  });
}
