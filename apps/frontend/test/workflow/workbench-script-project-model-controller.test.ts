import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptProjectModelAction } from "../../src/components/workbench/workbench-script-project-model-controller.ts";
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
    action: "",
    activeMaterial: "steel",
    defaultProjectLabel: "Default Project",
    deleteModelVersion: async () => {},
    downloadProjectBundleJson: async () => {},
    downloadProjectBundleZip: async () => {},
    generateModel: () => {},
    generatePanelModel: () => {},
    loadedModelName: "model-a",
    modelCreatedLabel: "model created",
    modelDeletedLabel: "model deleted",
    modelSavedLabel: "model saved",
    noSavedModelsLabel: "no models",
    noVersionsLabel: "no versions",
    payload: {},
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
    "set-project:project-created",
    "refresh-projects",
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
    "set-model:model-created",
    "set-version:version-a",
    "refresh-projects",
    "refresh-versions:model-created",
    "message:model created",
  ]);
});
