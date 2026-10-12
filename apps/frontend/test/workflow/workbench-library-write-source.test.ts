import test from "node:test";
import assert from "node:assert/strict";
import { captureWorkbenchLibraryWriteSource, type WorkbenchLibraryWriteKind } from "../../src/components/workbench/workbench-library-write-source.ts";
import { workbenchLibraryRecordSources, workbenchLibraryVersionSources } from "../../src/components/workbench/workbench-library-record-source.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import type { ModelVersionRecord, ProjectRecord } from "../../src/lib/api/project-types.ts";

function fixture(kind: WorkbenchLibraryWriteKind) {
  const projects = [{ project_id: "project-a", models: [{ project_id: "project-a", model_id: "model-a" }] }] as ProjectRecord[];
  const versions = [{ project_id: "project-a", model_id: "model-a", version_id: "version-a" }] as ModelVersionRecord[];
  workbenchLibraryRecordSources.mark(projects); workbenchLibraryVersionSources.mark(versions);
  return { kind, projectId: "project-a", modelId: "model-a", versionId: "version-a", projects, versions,
    projectContext: createWorkbenchProjectContext({ projectId: "project-a", modelId: "model-a", versionId: "version-a" }),
    sourceChangedLabel: "Refresh the catalog" };
}

for (const kind of ["project", "model", "version"] as const) for (const defect of ["unmarked", "stale", "unreadable", "missing", "wrong-project", "wrong-selection", "disposed"]) {
  test(`library ${kind} write admission fails closed for ${defect}`, () => {
    const input = fixture(kind);
    const registry = kind === "version" ? workbenchLibraryVersionSources : workbenchLibraryRecordSources;
    const records = kind === "version" ? input.versions : input.projects;
    const source = registry.sourceOf(records)!, original = source.isCurrent;
    if (defect === "unmarked") { input.projects = [...input.projects]; input.versions = [...input.versions]; }
    if (defect === "stale") source.isCurrent = () => false;
    if (defect === "unreadable") source.isCurrent = () => { throw new Error("unreadable source"); };
    if (defect === "missing") { input.projects.length = 0; input.versions.length = 0; }
    if (defect === "wrong-project") {
      input.projects[0].project_id = "other";
      input.versions[0].project_id = "other";
    }
    if (defect === "wrong-selection") input.projectContext.update({ projectId: "other", modelId: "other", versionId: "other" });
    if (defect === "disposed") input.projectContext.dispose();
    try {
      assert.throws(() => captureWorkbenchLibraryWriteSource(input),
        /WORKBENCH_RECORD_SOURCE_CHANGED|WORKBENCH_CONTEXT_CHANGED/);
    } finally { source.isCurrent = original; }
  });
}

for (const kind of ["project", "model", "version"] as const) {
  test(`library ${kind} confirmed cleanup owns its source independently of a newer workspace selection`, () => {
    const input = fixture(kind), current = captureWorkbenchLibraryWriteSource(input);
    assert.equal(current(), true);
    input.projectContext.update({ projectId: "other", modelId: null, versionId: null });
    assert.equal(current(), true, "same-source cleanup must still reconcile the deleted record");
    input.projectContext.dispose();
    assert.equal(current(), false, "disposed owners cannot refresh or apply receipts");
  });
}

for (const kind of ["project", "model", "version", "createProject"] as const) {
  test(`library ${kind} write authority rejects a new effective backend after dispatch`, () => {
    const previousWindow = globalThis.window;
    const location = { search: "?kyuubikiApiBaseUrl=http%3A%2F%2F127.0.0.1%3A4000" };
    globalThis.window = { location, localStorage: { getItem: () => null } } as unknown as Window & typeof globalThis;
    try {
      const input = fixture(kind), current = captureWorkbenchLibraryWriteSource(input);
      assert.equal(current(), true);
      location.search = "?kyuubikiApiBaseUrl=http%3A%2F%2F127.0.0.1%3A5000";
      assert.equal(current(), false);
    } finally { globalThis.window = previousWindow; }
  });
}

test("creating the first project requires mounted authority but not an existing catalog", () => {
  const input = fixture("createProject");
  input.projects = []; input.versions = [];
  input.projectContext.update({ projectId: null, modelId: null, versionId: null });
  assert.equal(captureWorkbenchLibraryWriteSource(input)(), true);
});

test("version writes can use verified history independently of an unavailable project catalog", () => {
  const input = fixture("version"); input.projects = [];
  assert.equal(captureWorkbenchLibraryWriteSource(input)(), true);
});

for (const kind of ["model", "version"] as const) {
  test(`library ${kind} writes reject a member with a foreign parent`, () => {
    const input = fixture(kind);
    if (kind === "model") input.projects[0].models![0].project_id = "other";
    else input.versions[0].model_id = "other";
    assert.throws(() => captureWorkbenchLibraryWriteSource(input), /WORKBENCH_RECORD_SOURCE_CHANGED/);
  });
}
