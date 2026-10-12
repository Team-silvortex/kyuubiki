import test from "node:test";
import assert from "node:assert/strict";
import { openPersistedWorkbenchModel, openPersistedWorkbenchVersion, openPersistedWorkbenchVersionById,
  type WorkbenchVersionLoadScope } from "../../src/components/workbench/workbench-persisted-model-controller.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import type { ModelRecord, ModelVersionRecord } from "../../src/lib/api/project-types.ts";

const record: ModelVersionRecord & ModelRecord = {
  project_id: "project-a", model_id: "model-a", version_id: "version-a", latest_version_id: "version-a",
  version_number: 1, name: "Saved research", kind: "truss_2d", model_schema_version: "kyuubiki.model/v1",
  inserted_at: "2026-10-12T00:00:00Z", updated_at: "2026-10-12T00:00:00Z",
  payload: { kind: "truss_2d", name: "Saved research", youngs_modulus_gpa: 210,
    nodes: [{ id: "n1", x: 0, y: 0 }, { id: "n2", x: 1, y: 0 }],
    elements: [{ id: "e1", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210e9 }] },
};

function fixture() {
  const calls: string[] = [], messages: string[] = [], alerts: any[] = [];
  const apply = () => { calls.push("apply"); };
  const effects: Parameters<typeof openPersistedWorkbenchModel>[1] = {
    projectContext: createWorkbenchProjectContext({ projectId: "project-a", modelId: "model-a", versionId: "version-a" }),
    activeMaterial: "steel", startTransition: (callback) => callback(),
    createProject: async () => ({}), createModel: async () => ({}), createModelVersion: async () => ({}),
    updateModelVersion: async () => ({}),
    fetchModel: async () => ({ model: structuredClone(record) }),
    fetchModelVersion: async () => ({ version: structuredClone(record) }),
    refreshProjects: async () => {}, refreshVersions: async () => { calls.push("refresh"); },
    recordHistory: () => { calls.push("history"); }, resetActiveResult: () => { calls.push("reset"); },
    importActionLabel: "import", historyActionLabel: "history", importedModelLabel: "model loaded",
    importedProjectLabel: "project loaded", importedVersionLabel: "version loaded", importFailedLabel: "load failed",
    storeManifestPersistenceFailedLabel: "store failed", formatImportNotice: () => ({ id: "notice", tone: "warning", message: "notice" }),
    setMessage: (message) => { messages.push(message); }, setImportNotice: () => {},
    setSystemAlerts: (update) => { alerts.splice(0, alerts.length, ...(typeof update === "function" ? update(alerts) : update)); },
    setSelectedProjectId: apply, setSelectedModelId: apply, setSelectedVersionId: apply,
    setLoadedModelName: apply, setModelVersions: apply, setStudyKind: apply,
    setAxialForm: apply, setHeatBarModel: apply, setHeatPlaneModel: apply,
    setThermalBarModel: apply, setThermalBeamModel: apply, setThermalFrameModel: apply,
    setThermalTrussModel: apply, setThermalTruss3dModel: apply, setSpringModel: apply,
    setSpring2dModel: apply, setSpring3dModel: apply, setTrussModel: apply,
    setTruss3dModel: apply, setPlaneModel: apply, setFrameModel: apply,
    setBeamModel: apply, setTorsionModel: apply, setPlaneResultField: apply,
    setParametric: apply, setActiveMaterial: apply,
  };
  return { effects, calls, messages, alerts };
}
const open = (kind: string, effects: Parameters<typeof openPersistedWorkbenchModel>[1], scope?: WorkbenchVersionLoadScope) =>
  kind === "model" ? openPersistedWorkbenchModel(record, effects, scope) : openPersistedWorkbenchVersion(record, effects, scope);

for (const kind of ["model", "version"]) for (const mismatch of ["id", "project", "model", "missing"]) {
  if (kind === "model" && mismatch === "model") continue;
  test(`saved ${kind} rejects ${mismatch} identity before history, geometry, result or version refresh`, async () => {
    const { effects, calls, messages, alerts } = fixture();
    const changed = mismatch === "missing" ? null : { ...record,
      ...(mismatch === "id" ? { model_id: "different", version_id: "different" } : {}),
      ...(mismatch === "project" ? { project_id: "different" } : {}),
      ...(mismatch === "model" ? { model_id: "different" } : {}) };
    effects.fetchModel = async () => ({ model: changed });
    effects.fetchModelVersion = async () => ({ version: changed });
    const outcome = await open(kind, effects);
    assert.equal(outcome.ok, false);
    assert.deepEqual(calls, []);
    assert.match(messages.at(-1)!, /WORKBENCH_SAVED_(MODEL|VERSION)_INVALID/);
    assert.equal(alerts.length, 1);
    assert.deepEqual(effects.projectContext.current(), { projectId: "project-a", modelId: "model-a", versionId: "version-a" });
  });
}

for (const kind of ["model", "version"]) for (const change of ["context", "source", "unreadable", "feedback"]) for (const failed of [false, true]) {
  test(`saved ${kind} suppresses late ${failed ? "failure" : "payload"} after ${change} supersedes it`, async () => {
    const { effects, calls, messages, alerts } = fixture();
    let release!: () => void, sourceCurrent = true, feedbackCurrent = true;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    effects.fetchModel = async () => { await gate; if (failed) throw new Error("old failure"); return { model: record }; };
    effects.fetchModelVersion = async () => { await gate; if (failed) throw new Error("old failure"); return { version: record }; };
    const scope = { isCurrent: () => { if (!sourceCurrent && change === "unreadable") throw new Error("unreadable source"); return sourceCurrent; },
      ownsFeedback: () => feedbackCurrent, onApplied: () => assert.fail("obsolete payload must not apply") };
    const pending = open(kind, effects, scope);
    if (change === "context") effects.projectContext.begin();
    else if (change === "feedback") feedbackCurrent = false;
    else sourceCurrent = false;
    release();
    assert.equal((await pending).ok, false);
    assert.deepEqual(calls, []);
    assert.deepEqual(messages, []);
    assert.deepEqual(alerts, []);
  });
}

for (const kind of ["model", "version"]) {
  test(`saved ${kind} remains recoverable after one rejected read without replaying automatically`, async () => {
    const { effects, calls, messages, alerts } = fixture();
    let reads = 0;
    const fetch = async () => { if (++reads === 1) throw new Error("temporarily unavailable"); return { model: record, version: record }; };
    effects.fetchModel = fetch; effects.fetchModelVersion = fetch;
    assert.equal((await open(kind, effects)).ok, false);
    assert.equal(reads, 1); assert.deepEqual(calls, []);
    assert.equal(messages.at(-1), "temporarily unavailable");
    assert.equal((await open(kind, effects)).ok, true);
    assert.equal(reads, 2); assert.equal(alerts.length, 0);
    assert.equal(calls.filter((call) => call === "reset").length, 1);
    assert.equal(calls.filter((call) => call === "refresh").length, 1);
  });
}

test("by-ID checkpoint loads cannot attach a different returned version without a caller validator", async () => {
  const { effects, calls } = fixture();
  effects.fetchModelVersion = async () => ({ version: { ...record, version_id: "different" } });
  const result = await openPersistedWorkbenchVersionById("version-a", effects);
  assert.equal(result.ok, false);
  assert.deepEqual(calls, []);
});
