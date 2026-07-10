import type {
  ModelEnvelope,
  ModelListPayload,
  ModelVersionEnvelope,
  ModelVersionListPayload,
  ProjectEnvelope,
  ProjectListPayload,
} from "./project-types.ts";
import { requestJson } from "./core.ts";

type ProjectRequestJson = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;

export type ProjectCreateInput = { name: string; description?: string };
export type ProjectUpdateInput = { name?: string; description?: string };
export type ModelCreateInput = {
  name: string;
  kind: string;
  material?: string;
  model_schema_version?: string;
  payload: Record<string, unknown>;
};
export type ModelUpdateInput = Partial<{
  name: string;
  kind: string;
  material: string;
  model_schema_version: string;
  payload: Record<string, unknown>;
}>;
export type ModelVersionCreateInput = ModelUpdateInput & { payload: Record<string, unknown> };
export type ModelVersionUpdateInput = ModelUpdateInput;

export function fetchProjects(): Promise<ProjectListPayload> {
  return defaultProjectApiClient.fetchProjects();
}

export function createProject(input: ProjectCreateInput): Promise<ProjectEnvelope> {
  return defaultProjectApiClient.createProject(input);
}

export function updateProject(projectId: string, input: ProjectUpdateInput): Promise<ProjectEnvelope> {
  return defaultProjectApiClient.updateProject(projectId, input);
}

export function deleteProject(projectId: string): Promise<ProjectEnvelope> {
  return defaultProjectApiClient.deleteProject(projectId);
}

export function fetchModel(modelId: string): Promise<ModelEnvelope> {
  return defaultProjectApiClient.fetchModel(modelId);
}

export function createModel(projectId: string, input: ModelCreateInput): Promise<ModelEnvelope> {
  return defaultProjectApiClient.createModel(projectId, input);
}

export function updateModel(modelId: string, input: ModelUpdateInput): Promise<ModelEnvelope> {
  return defaultProjectApiClient.updateModel(modelId, input);
}

export function deleteModel(modelId: string): Promise<ModelEnvelope> {
  return defaultProjectApiClient.deleteModel(modelId);
}

export function fetchModelVersions(modelId: string): Promise<ModelVersionListPayload> {
  return defaultProjectApiClient.fetchModelVersions(modelId);
}

export function fetchModelVersion(versionId: string): Promise<ModelVersionEnvelope> {
  return defaultProjectApiClient.fetchModelVersion(versionId);
}

export function createModelVersion(
  modelId: string,
  input: ModelVersionCreateInput,
): Promise<ModelVersionEnvelope> {
  return defaultProjectApiClient.createModelVersion(modelId, input);
}

export function updateModelVersion(
  versionId: string,
  input: ModelVersionUpdateInput,
): Promise<ModelVersionEnvelope> {
  return defaultProjectApiClient.updateModelVersion(versionId, input);
}

export function deleteModelVersion(versionId: string): Promise<ModelVersionEnvelope> {
  return defaultProjectApiClient.deleteModelVersion(versionId);
}

export function createProjectApiClient(request: ProjectRequestJson) {
  return {
    fetchProjects() {
      return request<ProjectListPayload>("/api/v1/projects", { method: "GET", cache: "no-store" });
    },
    createProject(input: ProjectCreateInput) {
      return request<ProjectEnvelope>("/api/v1/projects", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    updateProject(projectId: string, input: ProjectUpdateInput) {
      return request<ProjectEnvelope>(`/api/v1/projects/${projectId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    deleteProject(projectId: string) {
      return request<ProjectEnvelope>(`/api/v1/projects/${projectId}`, { method: "DELETE" });
    },
    fetchModel(modelId: string) {
      return request<ModelEnvelope>(`/api/v1/models/${modelId}`, { method: "GET", cache: "no-store" });
    },
    createModel(projectId: string, input: ModelCreateInput) {
      return request<ModelEnvelope>(`/api/v1/projects/${projectId}/models`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    updateModel(modelId: string, input: ModelUpdateInput) {
      return request<ModelEnvelope>(`/api/v1/models/${modelId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    deleteModel(modelId: string) {
      return request<ModelEnvelope>(`/api/v1/models/${modelId}`, { method: "DELETE" });
    },
    fetchModelVersions(modelId: string) {
      return request<ModelVersionListPayload>(`/api/v1/models/${modelId}/versions`, {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchModelVersion(versionId: string) {
      return request<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, {
        method: "GET",
        cache: "no-store",
      });
    },
    createModelVersion(modelId: string, input: ModelVersionCreateInput) {
      return request<ModelVersionEnvelope>(`/api/v1/models/${modelId}/versions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    updateModelVersion(versionId: string, input: ModelVersionUpdateInput) {
      return request<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    deleteModelVersion(versionId: string) {
      return request<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, { method: "DELETE" });
    },
  };
}

export type ProjectApiClient = ReturnType<typeof createProjectApiClient>;

export const defaultProjectApiClient = createProjectApiClient(requestJson);
