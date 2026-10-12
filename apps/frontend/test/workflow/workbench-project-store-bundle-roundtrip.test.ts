import test from "node:test";
import assert from "node:assert/strict";

import JSZip from "jszip";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import { WORKBENCH_API_BASE_URL_STORAGE_KEY } from "../../src/lib/api/backend-target.ts";
import { writeInMemoryWorkbenchSecrets } from "../../src/lib/workbench/workbench-secrets.ts";
import { getWorkbenchLibraryMutations } from "../../src/components/workbench/workbench-library-mutations.ts";

import {
  importWorkbenchProjectBundle,
  openPersistedWorkbenchModel,
  openPersistedWorkbenchVersionById,
} from "../../src/components/workbench/workbench-persisted-model-controller.ts";
import { exportProjectBundle } from "../../src/lib/models/modeler-export.ts";
import {
  defaultProjectFileManifest,
  exportProjectBundleZip,
  parseProjectBundleFile,
} from "../../src/lib/projects/project-format.ts";
import {
  readWorkspaceStoreManifest,
  STORE_MANIFEST_SCHEMA_VERSION,
  STORE_MANIFEST_STORAGE_KEY,
  type WorkspaceStoreManifest,
} from "../../src/lib/workbench/store-manifest.ts";

class MemoryStorage implements Storage {
  private readonly records = new Map<string, string>();
  failWrites = false;

  get length() {
    return this.records.size;
  }

  clear() {
    this.records.clear();
  }

  getItem(key: string) {
    return this.records.get(key) ?? null;
  }

  key(index: number) {
    return [...this.records.keys()][index] ?? null;
  }

  removeItem(key: string) {
    this.records.delete(key);
  }

  setItem(key: string, value: string) {
    if (this.failWrites) throw new Error("storage write failed");
    this.records.set(key, String(value));
  }
}

const storage = new MemoryStorage();
const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");

test.before(() => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { localStorage: storage, dispatchEvent: () => true } as unknown as Window,
  });
});

test.beforeEach(() => {
  storage.clear();
  storage.failWrites = false;
  writeInMemoryWorkbenchSecrets({});
});

test.after(() => {
  writeInMemoryWorkbenchSecrets({});
  if (originalWindowDescriptor) Object.defineProperty(globalThis, "window", originalWindowDescriptor);
  else Reflect.deleteProperty(globalThis, "window");
});

function storeManifest(projectId = "source-project"): WorkspaceStoreManifest {
  return {
    schema_version: STORE_MANIFEST_SCHEMA_VERSION,
    project_id: projectId,
    updated_at: "2026-08-28T00:00:00.000Z",
    entries: [{
      id: "solve-qualified",
      kind: "operator",
      title: "Qualified solver",
      version: "2.17.0",
      source_id: "qualification",
      package_ref: "store://qualification/solve-qualified@2.17.0",
      target: "operators/solve-qualified",
      installed_at: "2026-08-28T00:00:00.000Z",
    }],
  };
}

function projectBundleJson() {
  return exportProjectBundle({
    project: {
      project_id: "source-project",
      name: "Store roundtrip",
      description: "Store bundle qualification",
      inserted_at: "2026-08-28T00:00:00.000Z",
      updated_at: "2026-08-28T00:00:00.000Z",
      models: [],
    },
    models: [],
    modelVersions: [],
    storeManifest: storeManifest(),
  });
}

test("project archives preserve Store manifest assets and references", async () => {
  const bundleBlob = await exportProjectBundleZip(projectBundleJson());
  const archive = await JSZip.loadAsync(await bundleBlob.arrayBuffer());
  const manifestPath = defaultProjectFileManifest().store_manifest_path;
  const archivedManifest = archive.file(manifestPath);

  assert.ok(archivedManifest);
  assert.ok(archive.file(`${manifestPath}.meta`));
  assert.deepEqual(JSON.parse(await archivedManifest.async("string")), storeManifest());

  const parsed = await parseProjectBundleFile(new File([bundleBlob], "store-roundtrip.kyuubiki"));
  assert.deepEqual(parsed.store_manifest, storeManifest());
  const projectAsset = parsed.asset_catalog?.find((entry) => entry.kind === "project");
  const storeAsset = parsed.asset_catalog?.find((entry) => entry.kind === "store_manifest");
  assert.ok(projectAsset);
  assert.ok(storeAsset);
  assert.ok(parsed.asset_references?.some((reference) =>
    reference.from_guid === projectAsset.guid &&
    reference.relation === "store_manifest_for" &&
    reference.to_guid === storeAsset.guid));
});

type ImportEffects = Parameters<typeof importWorkbenchProjectBundle>[1];

function importEffects(state: { selectedProjectId: string | null; messages: string[]; alerts: Array<{ id: string; message: string }> }): ImportEffects {
  return {
    projectContext: createWorkbenchProjectContext({ projectId: state.selectedProjectId, modelId: null, versionId: null }),
    activeMaterial: "steel",
    createModel: async () => { throw new Error("unexpected model import"); },
    createModelVersion: async () => { throw new Error("unexpected version import"); },
    createProject: async () => ({ project: { project_id: "imported-project" } }),
    fetchModel: async () => { throw new Error("unused"); },
    fetchModelVersion: async () => { throw new Error("unused"); },
    formatImportNotice: () => ({ id: "import-notice", message: "notice" }),
    historyActionLabel: "history",
    importActionLabel: "import",
    importFailedLabel: "import failed",
    importedModelLabel: "model imported",
    importedProjectLabel: "project imported",
    importedVersionLabel: "version imported",
    recordHistory: () => {},
    refreshProjects: async () => {},
    refreshVersions: async () => {},
    resetActiveResult: () => {},
    setActiveMaterial: () => {},
    setAxialForm: () => {},
    setBeamModel: () => {},
    setFrameModel: () => {},
    setHeatBarModel: () => {},
    setHeatPlaneModel: () => {},
    setImportNotice: () => {},
    setLoadedModelName: () => {},
    setMessage: (message) => state.messages.push(message),
    setModelVersions: () => {},
    setParametric: () => {},
    setPlaneModel: () => {},
    setPlaneResultField: () => {},
    setSelectedModelId: () => {},
    setSelectedProjectId: (projectId) => { state.selectedProjectId = projectId; },
    setSelectedVersionId: () => {},
    setSpring2dModel: () => {},
    setSpring3dModel: () => {},
    setSpringModel: () => {},
    setStudyKind: () => {},
    setSystemAlerts: (update) => {
      state.alerts = typeof update === "function" ? update(state.alerts) : update;
    },
    setThermalBarModel: () => {},
    setThermalBeamModel: () => {},
    setThermalFrameModel: () => {},
    setThermalTruss3dModel: () => {},
    setThermalTrussModel: () => {},
    setTorsionModel: () => {},
    setTruss3dModel: () => {},
    setTrussModel: () => {},
    storeManifestPersistenceFailedLabel: "store manifest persistence failed",
    updateModelVersion: async () => ({ version: {} }),
  };
}

for (const failed of [false, true]) {
  test(`project import source change after creation suppresses ${failed ? "old errors" : "local persistence and recovery"}`, async () => {
    const state = { selectedProjectId: "existing-project" as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
    const effects = importEffects(state);
    storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://source-a.invalid");
    effects.createProject = async () => {
      storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://source-b.invalid");
      if (failed) throw new Error("old source create failed");
      return { project: { project_id: "imported-project" } };
    };
    let reads = 0;
    effects.refreshProjects = async () => { reads += 1; };
    await importWorkbenchProjectBundle(new File([projectBundleJson()], "source-change.kyuubiki.json"), effects);
    assert.equal(reads, 0, "old-source import cannot refresh the new backend");
    assert.equal(storage.getItem(STORE_MANIFEST_STORAGE_KEY), null, "old-source manifest cannot become new-source local state");
    assert.equal(state.selectedProjectId, "existing-project");
    assert.deepEqual(state.messages, []);
    assert.deepEqual(state.alerts, []);
  });
}

function modelImportFixture(boundary: (phase: string) => Promise<void>) {
  const state = { selectedProjectId: "existing-project" as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
  const effects = importEffects(state);
  const bundle = JSON.parse(projectBundleJson());
  const model = { project_id: "source-project", model_id: "source-model", name: "Imported research", kind: "truss_2d",
    latest_version_id: "source-version-2", model_schema_version: "kyuubiki.model/v1", inserted_at: "", updated_at: "",
    payload: { kind: "truss_2d", name: "Imported research", youngs_modulus_gpa: 210,
      nodes: [{ id: "n1", x: 0, y: 0 }, { id: "n2", x: 1, y: 0 }],
      elements: [{ id: "e1", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210e9 }] } };
  bundle.models = [model];
  bundle.model_versions = [1, 2].map((number) => ({ ...model, version_id: `source-version-${number}`, version_number: number }));
  const file = new File([], "scoped-import.kyuubiki.json");
  file.text = async () => { await boundary("parse"); return JSON.stringify(bundle); };
  effects.createProject = async () => { await boundary("project"); return { project: { project_id: "imported-project" } }; };
  effects.createModel = async () => { await boundary("model"); return { model: {
    model_id: "imported-model", latest_version_id: "imported-version-1", versions: [{ version_id: "imported-version-1" }],
  } }; };
  effects.updateModelVersion = async () => { await boundary("rename"); return { version: {} }; };
  effects.createModelVersion = async () => { await boundary("version"); return { version: { version_id: "imported-version-2" } }; };
  effects.refreshProjects = async () => { await boundary("catalog"); };
  effects.refreshVersions = async () => { await boundary("history"); };
  return { state, effects, file };
}

for (const phase of ["parse", "project", "model", "rename", "version", "catalog", "history"]) {
  for (const sameFile of [true, false]) for (const failed of [false, true]) {
    test(`pending import ${phase} excludes ${sameFile ? "same" : "other"} file before parsing; failure=${failed}`, async () => {
      let release!: () => void, reached!: () => void, shouldFail = failed, reads = 0;
      const gate = new Promise<void>((resolve) => { release = resolve; });
      const waiting = new Promise<void>((resolve) => { reached = resolve; });
      const calls: string[] = [];
      const fixture = modelImportFixture(async (step) => {
        calls.push(step);
        if (step !== phase) return;
        reached(); await gate;
        if (shouldFail) throw new Error("original import failed");
      });
      const first = importWorkbenchProjectBundle(fixture.file, fixture.effects);
      await waiting;
      const originalText = fixture.file.text, duplicate = sameFile ? fixture.file : new File([], "other.kyuubiki.json");
      duplicate.text = async () => { reads += 1; throw new Error("duplicate file must not be parsed"); };
      const ownsIntent = fixture.effects.projectContext.capture(), before = [...calls];
      try {
        await importWorkbenchProjectBundle(duplicate, fixture.effects);
        assert.equal(reads, 0, "overlapping import is rejected before parsing or changing intent");
        assert.equal(ownsIntent(), true);
        assert.deepEqual(calls, before, "no request is queued or borrowed from the original import");
        assert.match(fixture.state.messages.at(-1) ?? "", /WORKBENCH_LIBRARY_WRITE_PENDING/);
      } finally { fixture.file.text = originalText; release(); await first; }
      assert.equal(fixture.state.messages.at(-1), failed ? "original import failed" : "project imported");
      shouldFail = false;
      const creations = calls.filter((step) => step === "project").length;
      await importWorkbenchProjectBundle(fixture.file, fixture.effects);
      assert.equal(calls.filter((step) => step === "project").length, creations + 1, "only an explicit later import creates another project");
      assert.equal(fixture.state.messages.at(-1), "project imported");
    });
  }
}

for (const phase of ["model", "rename", "version", "catalog", "history"]) {
  test(`import protects its created project through ${phase} but leaves unrelated work available`, async () => {
    let release!: () => void, reached!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const waiting = new Promise<void>((resolve) => { reached = resolve; });
    const fixture = modelImportFixture(async (step) => { if (step === phase) { reached(); await gate; } });
    const first = importWorkbenchProjectBundle(fixture.file, fixture.effects);
    await waiting;
    const registry = getWorkbenchLibraryMutations(fixture.effects.projectContext);
    try {
      for (const target of [{ kind: "project", projectId: "imported-project" },
        { kind: "model", projectId: "imported-project", modelId: "imported-model" }] as const) {
        await assert.rejects(registry.run(target, async () => {}, "wait"), /WORKBENCH_LIBRARY_WRITE_PENDING/);
      }
      await registry.run({ kind: "project", projectId: "other-project" }, async () => {}, "wait");
      await registry.run({ kind: "createProject" }, async () => {}, "wait");
    } finally { release(); await first; }
    await registry.run({ kind: "project", projectId: "imported-project" }, async () => {}, "wait");
    assert.equal(fixture.state.messages.at(-1), "project imported");
  });
}

for (const change of ["stay", "intent", "backend"]) {
  test(`import admission localizes busy feedback without replacing newer ${change} feedback`, async () => {
    storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://source-a.invalid");
    let release!: () => void, reached!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    const waiting = new Promise<void>((resolve) => { reached = resolve; });
    const fixture = modelImportFixture(async (step) => { if (step === "project") { reached(); await gate; } });
    fixture.effects.language = "zh";
    const first = importWorkbenchProjectBundle(fixture.file, fixture.effects);
    await waiting;
    try {
      const second = importWorkbenchProjectBundle(fixture.file, fixture.effects);
      if (change === "intent") fixture.effects.projectContext.begin();
      if (change === "backend") storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://source-b.invalid");
      if (change !== "stay") fixture.state.messages.push("Newer feedback");
      await second;
      if (change === "stay") assert.match(fixture.state.messages.at(-1) ?? "", /WORKBENCH_LIBRARY_WRITE_PENDING: 此项目/);
      else assert.deepEqual(fixture.state.messages, ["Newer feedback"]);
    } finally { release(); await first; }
    if (change !== "stay") assert.deepEqual(fixture.state.messages, ["Newer feedback"]);
  });
}

for (const phase of ["parse", "project", "model", "rename", "version", "catalog", "history"]) {
  for (const change of ["backend", "credentials", "disposed"]) for (const failed of [false, true]) {
    test(`project import ${phase} ${failed ? "failure" : "receipt"} stops after ${change} changes`, async () => {
      storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://source-a.invalid");
      const calls: string[] = [];
      let stoppedAt: string[] | undefined, before: typeof fixture.state | undefined, manifest: string | null = null;
      const fixture = modelImportFixture(async (step) => {
        calls.push(step);
        if (step !== phase) return;
        if (change === "backend") storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://source-b.invalid");
        else if (change === "credentials") writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "fixture-next-token" });
        else fixture.effects.projectContext.dispose();
        stoppedAt = [...calls]; before = structuredClone(fixture.state); manifest = storage.getItem(STORE_MANIFEST_STORAGE_KEY);
        if (failed) throw new Error(`obsolete ${step} failure`);
      });
      await importWorkbenchProjectBundle(fixture.file, fixture.effects);
      assert.ok(stoppedAt, "the selected asynchronous boundary must actually run");
      assert.deepEqual(calls, stoppedAt, "no later write or read may use the changed authority");
      assert.deepEqual(fixture.state, before, "an obsolete import cannot publish bindings, warnings or failure messages");
      assert.equal(storage.getItem(STORE_MANIFEST_STORAGE_KEY), manifest, "already persisted local data is preserved, no new local import writes occur");
    });
  }
}

test("same-source project selection still lets an accepted import finish without replacing newer work", async () => {
  const calls: string[] = [];
  const fixture = modelImportFixture(async (step) => {
    calls.push(step);
    if (step === "project") {
      fixture.effects.projectContext.update({ projectId: "newer-project", modelId: null, versionId: null });
      fixture.state.selectedProjectId = "newer-project";
      fixture.state.messages.push("Newer workspace");
    }
  });
  await importWorkbenchProjectBundle(fixture.file, fixture.effects);
  assert.deepEqual(calls, ["parse", "project", "model", "rename", "version", "catalog"]);
  assert.equal(fixture.state.selectedProjectId, "newer-project");
  assert.deepEqual(fixture.state.messages, ["Newer workspace"]);
  assert.equal(readWorkspaceStoreManifest("imported-project").entries.length, 1);
});

test("disposed project import cannot parse a file or start a transition", async () => {
  const fixture = modelImportFixture(async () => assert.fail("disposed owner cannot start import"));
  fixture.effects.startTransition = () => assert.fail("disposed owner cannot start transition");
  fixture.effects.projectContext.dispose();
  await importWorkbenchProjectBundle(fixture.file, fixture.effects);
  assert.deepEqual(fixture.state.messages, []);
});

test("import busy callback and returned promise remain pending through final history recovery", async () => {
  let release!: () => void, reached!: () => void, finished = false, callbackResult: unknown;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const reading = new Promise<void>((resolve) => { reached = resolve; });
  const fixture = modelImportFixture(async (phase) => { if (phase === "history") { reached(); await gate; } });
  fixture.effects.startTransition = (callback) => { callbackResult = callback(); };
  const operation = importWorkbenchProjectBundle(fixture.file, fixture.effects).then(() => { finished = true; });
  try {
    await reading;
    assert.deepEqual(fixture.effects.projectContext.current(), { projectId: "imported-project",
      modelId: "imported-model", versionId: "imported-version-2" }, "history reads require the remapped model context before React commits selection");
    assert.ok(callbackResult instanceof Promise, "React needs the async callback to retain its busy state");
    assert.equal(finished, false);
    assert.deepEqual(fixture.state.messages, [], "success is not published before final history recovery");
  } finally { release(); await operation; }
  assert.equal(fixture.state.selectedProjectId, "imported-project");
  assert.equal(fixture.state.messages.at(-1), "project imported");
});

for (const navigation of ["stay", "switch", "away-and-back", "new-intent"]) {
  test(`project import final history failure preserves feedback ownership: ${navigation}`, async () => {
    const fixture = modelImportFixture(async (phase) => {
      if (phase !== "history") return;
      const context = fixture.effects.projectContext, selection = context.current();
      if (navigation === "new-intent") context.begin();
      else if (navigation !== "stay") context.update({ projectId: "newer-project", modelId: null, versionId: null });
      if (navigation === "away-and-back") context.update(selection);
      if (navigation !== "stay") fixture.state.messages.push("Newer operation");
      throw new Error("history unavailable");
    });
    await importWorkbenchProjectBundle(fixture.file, fixture.effects);
    assert.deepEqual(fixture.state.messages, [navigation === "stay" ? "history unavailable" : "Newer operation"]);
    assert.equal(fixture.state.alerts.length, navigation === "stay" ? 1 : 0);
  });
}

test("project import rewrites and persists the Store manifest for the new project", async () => {
  const state = { selectedProjectId: null as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
  await importWorkbenchProjectBundle(
    new File([projectBundleJson()], "store-roundtrip.kyuubiki.json", { type: "application/json" }),
    importEffects(state),
  );

  assert.equal(state.selectedProjectId, "imported-project");
  assert.equal(state.messages.at(-1), "project imported");
  assert.deepEqual(state.alerts, []);
  const importedManifest = readWorkspaceStoreManifest("imported-project");
  assert.equal(importedManifest.project_id, "imported-project");
  assert.equal(importedManifest.entries[0]?.id, "solve-qualified");
  assert.ok(storage.getItem(STORE_MANIFEST_STORAGE_KEY));
});

test("project import reports Store persistence failure without discarding imported project data", async () => {
  const state = { selectedProjectId: null as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
  storage.failWrites = true;
  await importWorkbenchProjectBundle(
    new File([projectBundleJson()], "store-roundtrip.kyuubiki.json", { type: "application/json" }),
    importEffects(state),
  );

  assert.equal(state.selectedProjectId, "imported-project");
  assert.equal(state.messages.at(-1), "project imported");
  assert.ok(state.alerts.some((alert) =>
    alert.id === "project-import-store-manifest-warning" &&
    alert.message === "store manifest persistence failed"));
  assert.equal(storage.getItem(STORE_MANIFEST_STORAGE_KEY), null);
});

test("project import validates the workspace payload before creating persistent records", async () => {
  const state = { selectedProjectId: "existing-project" as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
  const effects = importEffects(state);
  let creations = 0;
  effects.createProject = async () => { creations += 1; return { project: { project_id: "unexpected" } }; };
  const bundle = { ...JSON.parse(projectBundleJson()), workspace_snapshot: { kind: "unknown-study" } };
  await importWorkbenchProjectBundle(new File([JSON.stringify(bundle)], "invalid.kyuubiki.json"), effects);
  assert.equal(creations, 0);
  assert.equal(state.selectedProjectId, "existing-project");
  assert.ok(state.alerts.some((alert) => alert.id === "project-import-error"));
  assert.notEqual(state.messages.at(-1), "project imported");
});

test("persisted version loading remains awaitable across a React transition", async () => {
  let release!: () => void;
  let settled = false;
  let selectedVersionId: string | null = null;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const state = { selectedProjectId: null as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
  const effects = importEffects(state);
  effects.startTransition = (callback) => callback();
  effects.fetchModelVersion = async () => {
    await gate;
    return {
      version: {
        inserted_at: "2026-08-28T00:00:00.000Z",
        kind: "truss_2d",
        model_id: "model-a",
        model_schema_version: "kyuubiki.model/v1",
        name: "Version A",
        payload: {
          kind: "truss_2d",
          name: "Version A",
          material: "steel",
          youngs_modulus_gpa: 210,
          nodes: [
            { id: "node-1", x: 0, y: 0, fix_x: true, fix_y: true },
            { id: "node-2", x: 1, y: 0 },
          ],
          elements: [{
            id: "element-1",
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210e9,
            material_id: "mat-1",
          }],
          materials: [{ id: "mat-1", name: "Steel", youngs_modulus: 210e9, poisson_ratio: 0.3 }],
        },
        project_id: "project-a",
        updated_at: "2026-08-28T00:00:00.000Z",
        version_id: "version-a",
        version_number: 1,
      },
    };
  };
  effects.setSelectedVersionId = (versionId) => { selectedVersionId = versionId; };
  const operation = openPersistedWorkbenchVersionById("version-a", effects);
  void operation.then(() => { settled = true; });

  await Promise.resolve();
  assert.equal(settled, false);
  release();
  assert.deepEqual(await operation, { ok: true });
  assert.equal(selectedVersionId, "version-a");
});

for (const source of ["model", "version"]) {
for (const navigation of ["stay", "switch", "away-and-back", "new-intent"]) {
  test(`persisted ${source} completion checks context after version refresh: ${navigation}`, async () => {
    const state = { selectedProjectId: "project-a" as string | null, messages: [] as string[],
      alerts: [] as Array<{ id: string; message: string }> };
    const effects = importEffects(state);
    const record = {
      project_id: "project-a", model_id: "model-a", version_id: "version-a", version_number: 1,
      name: "Loaded model", kind: "truss_2d", model_schema_version: "kyuubiki.model/v1",
      inserted_at: "2026-09-07T00:00:00.000Z", updated_at: "2026-09-07T00:00:00.000Z",
      payload: { kind: "truss_2d", name: "Loaded model", youngs_modulus_gpa: 210,
        nodes: [{ id: "n1", x: 0, y: 0 }, { id: "n2", x: 1, y: 0 }],
        elements: [{ id: "e1", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210e9 }] },
    };
    const selection = { projectId: "project-a", modelId: "model-a", versionId: "version-a" };
    effects.fetchModelVersion = async () => ({ version: record });
    effects.fetchModel = async () => ({ model: { ...record, latest_version_id: "version-a" } });
    effects.refreshVersions = async () => {
      effects.projectContext.update(selection);
      if (navigation === "stay") return;
      if (navigation === "new-intent") effects.projectContext.begin();
      else effects.projectContext.update({ projectId: "project-b", modelId: null, versionId: null });
      if (navigation === "away-and-back") effects.projectContext.update(selection);
      effects.setMessage("Newer operation");
    };
    const result = await (source === "model" ? openPersistedWorkbenchModel(record, effects)
      : openPersistedWorkbenchVersionById("version-a", effects));
    assert.equal(result.ok, navigation === "stay");
    if (!result.ok) {
      assert.match(result.error.message, /WORKBENCH_CONTEXT_CHANGED/u);
      assert.equal(state.messages.at(-1), "Newer operation");
    }
    assert.deepEqual(state.alerts, [], "a superseded completion must not create a current-workspace error");
  });
}
}

for (const scopeChange of ["source", "unreadable", "feedback"] as const) for (const failed of [false, true]) {
  test(`linked version scope ${scopeChange} suppresses late ${failed ? "failure" : "payload"} before any workspace application`, async () => {
    const state = { selectedProjectId: "project-a" as string | null, messages: [] as string[], alerts: [] as Array<{ id: string; message: string }> };
    const effects = importEffects(state);
    let sourceCurrent = true, feedbackCurrent = true, release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    effects.fetchModelVersion = async () => { await gate; if (failed) throw new Error("old read failed"); return { version: {} }; };
    effects.recordHistory = () => assert.fail("obsolete read cannot create an undo entry");
    effects.resetActiveResult = () => assert.fail("obsolete read cannot clear the live preview");
    effects.refreshVersions = async () => assert.fail("obsolete read cannot start a version-list read");
    const pending = openPersistedWorkbenchVersionById("version-a", effects, () => assert.fail("obsolete scope must be rejected before validation"), {
      isCurrent: () => { if (!sourceCurrent && scopeChange === "unreadable") throw new Error("scope unreadable"); return sourceCurrent; },
      ownsFeedback: () => feedbackCurrent,
      onApplied: () => assert.fail("obsolete scope cannot be applied"),
    });
    if (scopeChange === "feedback") feedbackCurrent = false;
    else sourceCurrent = false;
    release();
    assert.equal((await pending).ok, false);
    assert.deepEqual(state.messages, []);
    assert.deepEqual(state.alerts, []);
    assert.equal(state.selectedProjectId, "project-a");
  });
}
