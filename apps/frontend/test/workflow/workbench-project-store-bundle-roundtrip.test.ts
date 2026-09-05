import test from "node:test";
import assert from "node:assert/strict";

import JSZip from "jszip";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";

import {
  importWorkbenchProjectBundle,
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
});

test.after(() => {
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
