import test from "node:test";
import assert from "node:assert/strict";

import { createProjectLibraryBackendService } from "../../src/lib/workbench/project-library-backend-service-core.ts";
import type {
  ModelEnvelope,
  ModelVersionEnvelope,
  ModelVersionListPayload,
  ProjectEnvelope,
  ProjectListPayload,
} from "../../src/lib/api/project-types.ts";
import type {
  WorkbenchProjectLibraryBackendTransport,
} from "../../src/lib/workbench/project-library-backend-service-core.ts";

function projectEnvelope(projectId: string, name = "Demo Project"): ProjectEnvelope {
  return {
    project: {
      description: "Local workspace",
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
      model_id: modelId,
      model_schema_version: "kyuubiki.model/v1",
      name: "model",
      payload: {},
      project_id: "project-a",
      updated_at: "2026-06-29T00:00:00.000Z",
    },
  };
}

function versionEnvelope(versionId: string, modelId = "model-a"): ModelVersionEnvelope {
  return {
    version: {
      inserted_at: "2026-06-29T00:00:00.000Z",
      kind: "truss_2d",
      model_id: modelId,
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

function projectTransport(
  overrides: Partial<WorkbenchProjectLibraryBackendTransport> = {},
): WorkbenchProjectLibraryBackendTransport {
  return {
    createModel: async () => modelEnvelope("model-created"),
    createModelVersion: async () => versionEnvelope("version-created"),
    createProject: async (input) => projectEnvelope("project-created", input.name),
    deleteModel: async (modelId) => modelEnvelope(modelId),
    deleteModelVersion: async (versionId) => versionEnvelope(versionId),
    deleteProject: async (projectId) => projectEnvelope(projectId),
    fetchModel: async (modelId) => modelEnvelope(modelId),
    fetchModelVersion: async (versionId) => versionEnvelope(versionId),
    fetchModelVersions: async () => ({ versions: [] }),
    fetchProjects: async () => ({ projects: [] }),
    updateModel: async (modelId) => modelEnvelope(modelId),
    updateModelVersion: async (versionId) => versionEnvelope(versionId),
    updateProject: async (projectId) => projectEnvelope(projectId),
    ...overrides,
  };
}

test("project library backend fetches projects through transport", async () => {
  const calls: string[] = [];
  const service = createProjectLibraryBackendService(projectTransport({
    createProject: async (input) => {
      calls.push(`create:${input.name}`);
      return projectEnvelope("project-created", input.name);
    },
    fetchModelVersions: async (modelId) => {
      calls.push(`versions:${modelId}`);
      return { versions: [] } satisfies ModelVersionListPayload;
    },
    fetchProjects: async () => {
      calls.push("projects");
      return { projects: [projectEnvelope("project-a").project] } satisfies ProjectListPayload;
    },
  }));

  const payload = await service.fetchProjects();

  assert.deepEqual(calls, ["projects"]);
  assert.equal(payload.projects[0]?.project_id, "project-a");
});

test("project library backend creates bootstrap projects through transport", async () => {
  const calls: unknown[] = [];
  const service = createProjectLibraryBackendService(projectTransport({
    createProject: async (input) => {
      calls.push(input);
      return projectEnvelope("project-created", input.name);
    },
    fetchModelVersions: async () => ({ versions: [] }) satisfies ModelVersionListPayload,
    fetchProjects: async () => ({ projects: [] }) satisfies ProjectListPayload,
  }));

  const payload = await service.createProject({
    description: "Local workspace",
    name: "manual-study",
  });

  assert.equal(payload.project.project_id, "project-created");
  assert.deepEqual(calls, [{ description: "Local workspace", name: "manual-study" }]);
});

test("project library backend fetches model versions through transport", async () => {
  const calls: string[] = [];
  const service = createProjectLibraryBackendService(projectTransport({
    createProject: async (input) => projectEnvelope("project-created", input.name),
    fetchModelVersions: async (modelId) => {
      calls.push(modelId);
      return {
        versions: [
          {
            inserted_at: "2026-06-29T00:00:00.000Z",
            kind: "truss_2d",
            model_id: modelId,
            model_schema_version: "kyuubiki.model/v1",
            name: "v1",
            payload: {},
            project_id: "project-a",
            updated_at: "2026-06-29T00:00:00.000Z",
            version_id: "version-a",
            version_number: 1,
          },
        ],
      } satisfies ModelVersionListPayload;
    },
    fetchProjects: async () => ({ projects: [] }) satisfies ProjectListPayload,
  }));

  const payload = await service.fetchModelVersions("model-a");

  assert.deepEqual(calls, ["model-a"]);
  assert.equal(payload.versions[0]?.version_id, "version-a");
});

test("project library backend forwards full model and version CRUD through transport", async () => {
  const calls: string[] = [];
  const service = createProjectLibraryBackendService(projectTransport({
    createModel: async (projectId, input) => {
      calls.push(`create-model:${projectId}:${input.name}`);
      return modelEnvelope("model-created");
    },
    updateModel: async (modelId, input) => {
      calls.push(`update-model:${modelId}:${input.name}`);
      return modelEnvelope(modelId);
    },
    createModelVersion: async (modelId, input) => {
      calls.push(`create-version:${modelId}:${input.name}`);
      return versionEnvelope("version-created", modelId);
    },
    updateModelVersion: async (versionId, input) => {
      calls.push(`update-version:${versionId}:${input.name}`);
      return versionEnvelope(versionId);
    },
    deleteModel: async (modelId) => {
      calls.push(`delete-model:${modelId}`);
      return modelEnvelope(modelId);
    },
    deleteModelVersion: async (versionId) => {
      calls.push(`delete-version:${versionId}`);
      return versionEnvelope(versionId);
    },
  }));

  await service.createModel("project-a", {
    kind: "truss_2d",
    name: "model-a",
    payload: {},
  });
  await service.updateModel("model-a", { name: "model-b" });
  await service.createModelVersion("model-a", { name: "v2", payload: {} });
  await service.updateModelVersion("version-a", { name: "v1-renamed" });
  await service.deleteModel("model-a");
  await service.deleteModelVersion("version-a");

  assert.deepEqual(calls, [
    "create-model:project-a:model-a",
    "update-model:model-a:model-b",
    "create-version:model-a:v2",
    "update-version:version-a:v1-renamed",
    "delete-model:model-a",
    "delete-version:version-a",
  ]);
});
