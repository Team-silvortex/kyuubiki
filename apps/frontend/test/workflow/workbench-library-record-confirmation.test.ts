import test from "node:test";
import assert from "node:assert/strict";
import { createProjectLibraryBackendService, type WorkbenchProjectLibraryBackendTransport } from "../../src/lib/workbench/project-library-backend-service-core.ts";

const records = {
  project: { project_id: "project", name: "Project", models: [] },
  model: { model_id: "model", project_id: "project", name: "Model", payload: {} },
  version: { version_id: "version", model_id: "model", project_id: "project", name: "Version", payload: {}, version_number: 1 },
};

function fixture(receipt: unknown) {
  const state = { writes: 0, reads: 0 };
  const write = async () => { state.writes += 1; return receipt; };
  const read = async () => { state.reads += 1; throw new Error("unexpected read"); };
  const transport = {
    createModel: write, createModelVersion: write, createProject: write,
    deleteModel: write, deleteModelVersion: write, deleteProject: write,
    fetchModel: read, fetchModelVersion: read, fetchModelVersions: read, fetchProjects: read,
    updateModel: write, updateModelVersion: write, updateProject: write,
  } as WorkbenchProjectLibraryBackendTransport;
  const service = createProjectLibraryBackendService(transport);
  const invoke = (kind: keyof typeof records, action: "update" | "delete") => {
    if (kind === "project") return action === "update" ? service.updateProject(kind, { name: "Edited" }) : service.deleteProject(kind);
    if (kind === "model") return action === "update" ? service.updateModel(kind, { name: "Edited" }) : service.deleteModel(kind);
    return action === "update" ? service.updateModelVersion(kind, { name: "Edited" }) : service.deleteModelVersion(kind);
  };
  return { state, service, invoke, transport };
}

for (const kind of ["project", "model", "version"] as const) for (const action of ["update", "delete"] as const) {
  const failures: Array<[string, unknown]> = [
    ["null", null], ["array-envelope", []], ["missing-record", {}],
    ["null-record", { [kind]: null }], ["array-record", { [kind]: [] }],
    ["wrong-id", { [kind]: { ...records[kind], [`${kind}_id`]: "other" } }],
    ["numeric-id", { [kind]: { ...records[kind], [`${kind}_id`]: 42 } }],
    ["blank-name", { [kind]: { ...records[kind], name: " " } }],
  ];
  if (kind !== "project") failures.push(
    ["missing-parent", { [kind]: { ...records[kind], project_id: null } }],
    ["nonobject-payload", { [kind]: { ...records[kind], payload: [] } }],
  );
  if (kind === "version") failures.push(
    ["missing-model", { version: { ...records.version, model_id: null } }],
    ["invalid-sequence", { version: { ...records.version, version_number: 0 } }],
  );
  for (const [label, receipt] of failures) test(`library ${kind} ${action} rejects ${label} receipt without reads or retry`, async () => {
    const { state, invoke } = fixture(receipt);
    await assert.rejects(invoke(kind, action), /LIBRARY_RECORD_MUTATION_UNCONFIRMED/u);
    assert.deepEqual(state, { writes: 1, reads: 0 });
  });

  test(`library ${kind} ${action} keeps valid receipt identity without adding a deletion flag contract`, async () => {
    const receipt = { [kind]: records[kind] };
    const { state, invoke } = fixture(receipt);
    assert.equal(await invoke(kind, action), receipt);
    assert.deepEqual(state, { writes: 1, reads: 0 });
  });

  test(`library ${kind} ${action} retains the original transport error without retry`, async () => {
    const failure = new Error("write response lost");
    const { state, transport } = fixture(null);
    const write = async () => { state.writes += 1; throw failure; };
    Object.assign(transport, { deleteProject: write, deleteModel: write, deleteModelVersion: write,
      updateProject: write, updateModel: write, updateModelVersion: write });
    const service = createProjectLibraryBackendService(transport);
    const methods = { project: [service.updateProject, service.deleteProject], model: [service.updateModel, service.deleteModel],
      version: [service.updateModelVersion, service.deleteModelVersion] };
    await assert.rejects(methods[kind][action === "update" ? 0 : 1](kind, { name: "Edited" }), (error) => error === failure);
    assert.deepEqual(state, { writes: 1, reads: 0 });
  });
}

for (const receipt of [null, {}, { project: { project_id: " ", name: "Project" } }, { project: { project_id: "new", name: null } }]) {
  test(`library project creation rejects malformed receipt ${JSON.stringify(receipt)} without reads or retry`, async () => {
    const { service, state } = fixture(receipt);
    await assert.rejects(service.createProject({ name: "New" }), /LIBRARY_RECORD_MUTATION_UNCONFIRMED/u);
    assert.deepEqual(state, { writes: 1, reads: 0 });
  });
}
