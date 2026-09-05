import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptProjectModelAction } from "../../src/components/workbench/workbench-script-project-model-controller.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import type { WorkbenchProjectLibraryBackendService } from "../../src/lib/workbench/project-library-backend-service-core.ts";
import type {
  ModelEnvelope,
  ModelVersionEnvelope,
  ModelVersionListPayload,
  ProjectEnvelope,
  ProjectListPayload,
} from "../../src/lib/api/project-types.ts";

function projectEnvelope(projectId: string, name = "Project"): ProjectEnvelope {
  return {
    project: {
      description: "",
      inserted_at: "2026-06-29T00:00:00.000Z",
      models: [],
      name,
      project_id: projectId,
      updated_at: "2026-06-29T00:00:00.000Z",
    },
  };
}

function modelEnvelope(modelId: string): ModelEnvelope {
  return {
    model: {
      inserted_at: "2026-06-29T00:00:00.000Z",
      kind: "truss_2d",
      latest_version_id: "version-a",
      model_id: modelId,
      model_schema_version: "kyuubiki.model/v1",
      name: "model",
      payload: {},
      project_id: "project-a",
      updated_at: "2026-06-29T00:00:00.000Z",
    },
  };
}

function versionEnvelope(versionId: string): ModelVersionEnvelope {
  return {
    version: {
      inserted_at: "2026-06-29T00:00:00.000Z",
      kind: "truss_2d",
      model_id: "model-a",
      model_schema_version: "kyuubiki.model/v1",
      name: "v1",
      payload: {},
      project_id: "project-a",
      updated_at: "2026-06-29T00:00:00.000Z",
      version_id: versionId,
      version_number: 1,
    },
  };
}

function projectLibraryService(calls: string[]): WorkbenchProjectLibraryBackendService {
  return {
    createModel: async (projectId, input) => {
      calls.push(`create-model:${projectId}:${input.name}`);
      return modelEnvelope("model-created");
    },
    createModelVersion: async (modelId, input) => {
      calls.push(`create-version:${modelId}:${input.name}`);
      return versionEnvelope("version-created");
    },
    createProject: async (input) => {
      calls.push(`create-project:${input.name}`);
      return projectEnvelope("project-created", input.name);
    },
    deleteModel: async (modelId) => {
      calls.push(`delete-model:${modelId}`);
      return modelEnvelope(modelId);
    },
    deleteModelVersion: async (versionId) => {
      calls.push(`delete-version:${versionId}`);
      return versionEnvelope(versionId);
    },
    deleteProject: async (projectId) => {
      calls.push(`delete-project:${projectId}`);
      return projectEnvelope(projectId);
    },
    fetchModel: async (modelId) => modelEnvelope(modelId),
    fetchModelVersion: async (versionId) => versionEnvelope(versionId),
    fetchModelVersions: async () => ({ versions: [] }) satisfies ModelVersionListPayload,
    fetchProjects: async () => ({ projects: [] }) satisfies ProjectListPayload,
    updateModel: async (modelId, input) => {
      calls.push(`update-model:${modelId}:${input.name}`);
      return modelEnvelope(modelId);
    },
    updateModelVersion: async (versionId, input) => {
      calls.push(`update-version:${versionId}:${input.name}`);
      return versionEnvelope(versionId);
    },
    updateProject: async (projectId, input) => {
      calls.push(`update-project:${projectId}:${input.name}`);
      return projectEnvelope(projectId, input.name);
    },
  };
}

function baseArgs(calls: string[]) {
  return {
    projectContext: createWorkbenchProjectContext({ projectId: null, modelId: null, versionId: null }),
    action: "",
    activeMaterial: "steel",
    defaultProjectLabel: "Default Project",
    deleteModelVersion: async () => {},
    downloadProjectBundleJson: async () => ({ ok: true as const }),
    downloadProjectBundleZip: async () => ({ ok: true as const }),
    generateModel: () => {},
    generatePanelModel: () => {},
    loadedModelName: "model-a",
    modelCreatedLabel: "model created",
    modelDeletedLabel: "model deleted",
    modelSavedLabel: "model saved",
    noSavedModelsLabel: "no models",
    noVersionsLabel: "no versions",
    payload: {},
    projects: [projectEnvelope("project-a").project, projectEnvelope("project-b", "Second project").project],
    projectCreatedLabel: "project created",
    projectDeletedLabel: "project deleted",
    projectDescriptionDraft: "",
    projectLibraryBackendService: projectLibraryService(calls),
    projectNameDraft: "draft",
    projectRequiredLabel: "project required",
    projectUpdatedLabel: "project updated",
    refreshProjects: async () => {
      calls.push("refresh-projects");
    },
    refreshVersions: async (modelId: string) => {
      calls.push(`refresh-versions:${modelId}`);
    },
    selectedModelId: null,
    selectedProjectId: null,
    selectedVersionId: null,
    serializeCurrentModel: () => ({ model_schema_version: "kyuubiki.model/v1" }),
    setActiveMaterial: () => {},
    setLoadedModelName: () => {},
    setMessage: (message: string) => {
      calls.push(`message:${message}`);
    },
    setModelVersions: () => {},
    setProjectDescriptionDraft: () => {},
    setProjectNameDraft: () => {},
    setSelectedModelId: (value: string | null) => {
      calls.push(`set-model:${value ?? ""}`);
    },
    setSelectedProjectId: (value: string | null) => {
      calls.push(`set-project:${value ?? ""}`);
    },
    setSelectedVersionId: (value: string | null) => {
      calls.push(`set-version:${value ?? ""}`);
    },
    studyKind: "truss_2d",
    updateModelVersion: async () => {},
    versionDeletedLabel: "version deleted",
    versionRenamedLabel: "version renamed",
  };
}

test("script project action creates projects through project library service", async () => {
  const calls: string[] = [];
  const result = await handleWorkbenchScriptProjectModelAction({
    ...baseArgs(calls),
    action: "project/create",
    payload: { name: "Script Project" },
  });

  assert.deepEqual(result, { ok: true, action: "project/create", projectId: "project-created" });
  assert.deepEqual(calls, [
    "create-project:Script Project",
    "refresh-projects",
    "set-project:project-created",
    "set-model:",
    "set-version:",
    "message:project created",
  ]);
});

test("script model save creates models through project library service", async () => {
  const calls: string[] = [];
  const result = await handleWorkbenchScriptProjectModelAction({
    ...baseArgs(calls),
    action: "model/save",
    selectedProjectId: "project-a",
  });

  assert.deepEqual(result, { ok: true, action: "model/save", modelId: "model-created" });
  assert.deepEqual(calls, [
    "create-model:project-a:model-a",
    "refresh-projects",
    "set-model:model-created",
    "set-version:version-a",
    "message:model created",
    "refresh-versions:model-created",
  ]);
});

test("script project exports propagate download failures instead of reporting false success", async () => {
  const calls: string[] = [];
  const failure = new Error("project bundle download failed");

  await assert.rejects(
    handleWorkbenchScriptProjectModelAction({
      ...baseArgs(calls),
      action: "project/exportJson",
      downloadProjectBundleJson: async () => ({ ok: false, error: failure }),
    }),
    failure,
  );
});

test("script project selection clears saved associations only when the project changes", async () => {
  const calls: string[] = [];
  const args = { ...baseArgs(calls), action: "project/select", selectedProjectId: "project-a", selectedModelId: "model-a" };
  await handleWorkbenchScriptProjectModelAction({ ...args, payload: { projectId: "project-a" } });
  assert.deepEqual(calls, ["set-project:project-a"]);
  calls.length = 0;
  await handleWorkbenchScriptProjectModelAction({ ...args, payload: { projectId: "project-b" } });
  assert.deepEqual(calls, ["set-model:", "set-version:", "set-project:project-b"]);
});

for (const projectId of ["missing-project", "", null, 17]) {
  test(`script project selection rejects invalid project ${JSON.stringify(projectId)} without mutations`, async () => {
    const calls: string[] = [];
    await assert.rejects(handleWorkbenchScriptProjectModelAction({
      ...baseArgs(calls), action: "project/select", selectedProjectId: "project-a", payload: { projectId },
    }), /project required/u);
    assert.deepEqual(calls, []);
  });
}

test("script save-as restores the newly created model after refreshing an existing project", async () => {
  const calls: string[] = [];
  const args = baseArgs(calls);
  await handleWorkbenchScriptProjectModelAction({
    ...args, action: "model/saveAs", selectedProjectId: "project-a", selectedModelId: "old-model",
    refreshProjects: async () => { args.setSelectedModelId("old-model"); },
  });
  assert.deepEqual(calls.filter((entry) => entry.startsWith("set-model:")), ["set-model:old-model", "set-model:model-created"]);
});

for (const action of ["model/save", "model/saveAs"]) {
  test(`script ${action} reports a completed write without applying it to a changed workspace`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a",
      refreshProjects: async (_bootstrap, _preferred, options) => {
        assert.equal(options?.preserveSelection, true);
        args.projectContext.update({ projectId: "project-b", modelId: null, versionId: null });
      },
    });
    assert.equal(result?.ok, true);
    assert.equal(result?.contextChanged, true);
    assert.ok(calls.some((entry) => entry.startsWith("create-")), "the original backend write is retained");
    assert.deepEqual(calls.filter((entry) => /^(set-|message:|refresh-versions:)/u.test(entry)), []);
  });
}

const savedSelection = { projectId: "project-a", modelId: "model-a", versionId: "version-a" };
for (const action of ["project/create", "project/updateSelected", "project/deleteSelected",
  "model/deleteSelected", "model/renameSelectedVersion", "model/deleteSelectedVersion"]) {
  test(`script ${action} completes its original write without mutating a newer saved workspace`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    args.projectContext.update(savedSelection);
    const changeWorkspace = () => args.projectContext.update({ projectId: "project-b", modelId: "model-b", versionId: "version-b" });
    const updateVersion = args.projectLibraryBackendService.updateModelVersion;
    args.projectLibraryBackendService.updateModelVersion = async (...inputs) => {
      const response = await updateVersion(...inputs);
      changeWorkspace();
      return response;
    };
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a",
      setProjectNameDraft: () => { calls.push("set-name"); },
      setProjectDescriptionDraft: () => { calls.push("set-description"); },
      setModelVersions: () => { calls.push("set-versions"); },
      refreshProjects: async (_bootstrap, _preferred, options) => {
        assert.equal(options?.preserveSelection, true);
        changeWorkspace();
      },
    });
    assert.equal(result?.ok, true);
    assert.equal(result?.contextChanged, true);
    assert.equal(calls.filter((entry) => /^(create-|update-|delete-)/u.test(entry)).length, 1);
    assert.deepEqual(calls.filter((entry) => /^(set-|message:|refresh-versions:)/u.test(entry)), []);
  });
}

for (const action of ["project/deleteSelected", "model/deleteSelected", "model/deleteSelectedVersion"]) {
  test(`script ${action} reconciles current deleted bindings without selecting another saved record`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    args.projectContext.update(savedSelection);
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a",
      refreshProjects: async (_bootstrap, _preferred, options) => { assert.equal(options?.preserveSelection, true); },
    });
    assert.deepEqual(result, { ok: true, action });
    const expected = action === "project/deleteSelected" ? ["set-project:", "set-model:", "set-version:"]
      : action === "model/deleteSelected" ? ["set-model:", "set-version:"] : ["set-version:"];
    assert.deepEqual(calls.filter((entry) => entry.startsWith("set-")), expected);
    assert.equal(calls.filter((entry) => entry.startsWith("message:")).length, 1);
  });
}
