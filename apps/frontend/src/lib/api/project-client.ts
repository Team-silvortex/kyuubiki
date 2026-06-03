import type {
  ModelEnvelope,
  ModelListPayload,
  ModelVersionEnvelope,
  ModelVersionListPayload,
  ProjectEnvelope,
  ProjectListPayload,
} from "./index";
import { requestJson } from "./core";

export function fetchProjects(): Promise<ProjectListPayload> {
  return requestJson<ProjectListPayload>("/api/v1/projects", { method: "GET", cache: "no-store" });
}

export function createProject(input: { name: string; description?: string }): Promise<ProjectEnvelope> {
  return requestJson<ProjectEnvelope>("/api/v1/projects", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function updateProject(projectId: string, input: { name?: string; description?: string }): Promise<ProjectEnvelope> {
  return requestJson<ProjectEnvelope>(`/api/v1/projects/${projectId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteProject(projectId: string): Promise<ProjectEnvelope> {
  return requestJson<ProjectEnvelope>(`/api/v1/projects/${projectId}`, { method: "DELETE" });
}

export function fetchModel(modelId: string): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/models/${modelId}`, { method: "GET", cache: "no-store" });
}

export function createModel(
  projectId: string,
  input: {
    name: string;
    kind: string;
    material?: string;
    model_schema_version?: string;
    payload: Record<string, unknown>;
  },
): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/projects/${projectId}/models`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function updateModel(
  modelId: string,
  input: Partial<{
    name: string;
    kind: string;
    material: string;
    model_schema_version: string;
    payload: Record<string, unknown>;
  }>,
): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/models/${modelId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteModel(modelId: string): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/models/${modelId}`, { method: "DELETE" });
}

export function fetchModelVersions(modelId: string): Promise<ModelVersionListPayload> {
  return requestJson<ModelVersionListPayload>(`/api/v1/models/${modelId}/versions`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchModelVersion(versionId: string): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function createModelVersion(
  modelId: string,
  input: Partial<{
    name: string;
    kind: string;
    material: string;
    model_schema_version: string;
  }> & { payload: Record<string, unknown> },
): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/models/${modelId}/versions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function updateModelVersion(
  versionId: string,
  input: Partial<{
    name: string;
    kind: string;
    material: string;
    model_schema_version: string;
    payload: Record<string, unknown>;
  }>,
): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteModelVersion(versionId: string): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, { method: "DELETE" });
}
