"use client";

import { createCheckpointRetry } from "./checkpoint-retry.ts";
import type { CheckpointJournal } from "./checkpoint-journal.ts";

import type {
  ModelEnvelope,
  ModelVersionEnvelope,
  ModelVersionListPayload,
  ProjectEnvelope,
  ProjectListPayload,
} from "@/lib/api/project-types";

export type WorkbenchProjectCreateInput = {
  description?: string;
  name: string;
};

export type WorkbenchProjectUpdateInput = Partial<WorkbenchProjectCreateInput>;

export type WorkbenchModelMutationInput = Partial<{
  kind: string;
  material: string;
  model_schema_version: string;
  name: string;
}> & {
  payload?: Record<string, unknown>;
};

export type WorkbenchModelCreateInput = WorkbenchModelMutationInput & {
  request_id?: string;
  kind: string;
  name: string;
  payload: Record<string, unknown>;
};

export type WorkbenchModelVersionCreateInput = WorkbenchModelMutationInput & {
  request_id?: string;
  payload: Record<string, unknown>;
};

export type WorkbenchProjectLibraryBackendTransport = {
  createModel(projectId: string, input: WorkbenchModelCreateInput): Promise<ModelEnvelope>;
  createModelVersion(modelId: string, input: WorkbenchModelVersionCreateInput): Promise<ModelVersionEnvelope>;
  createProject(input: WorkbenchProjectCreateInput): Promise<ProjectEnvelope>;
  deleteModel(modelId: string): Promise<ModelEnvelope>;
  deleteModelVersion(versionId: string): Promise<ModelVersionEnvelope>;
  deleteProject(projectId: string): Promise<ProjectEnvelope>;
  fetchModel(modelId: string): Promise<ModelEnvelope>;
  fetchModelVersion(versionId: string): Promise<ModelVersionEnvelope>;
  fetchModelVersions(modelId: string): Promise<ModelVersionListPayload>;
  fetchProjects(): Promise<ProjectListPayload>;
  updateModel(modelId: string, input: WorkbenchModelMutationInput): Promise<ModelEnvelope>;
  updateModelVersion(versionId: string, input: WorkbenchModelMutationInput): Promise<ModelVersionEnvelope>;
  updateProject(projectId: string, input: WorkbenchProjectUpdateInput): Promise<ProjectEnvelope>;
};

export type WorkbenchProjectLibraryBackendService = {
  createModel(projectId: string, input: WorkbenchModelCreateInput): Promise<ModelEnvelope>;
  createModelVersion(modelId: string, input: WorkbenchModelVersionCreateInput): Promise<ModelVersionEnvelope>;
  createProject(input: WorkbenchProjectCreateInput): Promise<ProjectEnvelope>;
  deleteModel(modelId: string): Promise<ModelEnvelope>;
  deleteModelVersion(versionId: string): Promise<ModelVersionEnvelope>;
  deleteProject(projectId: string): Promise<ProjectEnvelope>;
  fetchModel(modelId: string): Promise<ModelEnvelope>;
  fetchModelVersion(versionId: string): Promise<ModelVersionEnvelope>;
  fetchModelVersions(modelId: string): Promise<ModelVersionListPayload>;
  fetchProjects(): Promise<ProjectListPayload>;
  updateModel(modelId: string, input: WorkbenchModelMutationInput): Promise<ModelEnvelope>;
  updateModelVersion(versionId: string, input: WorkbenchModelMutationInput): Promise<ModelVersionEnvelope>;
  updateProject(projectId: string, input: WorkbenchProjectUpdateInput): Promise<ProjectEnvelope>;
};

export function createProjectLibraryBackendService(
  transport: WorkbenchProjectLibraryBackendTransport,
  retryOptions: { scope?: () => string; journal?: CheckpointJournal } = {},
): WorkbenchProjectLibraryBackendService {
  const checkpoint = createCheckpointRetry(retryOptions);
  return {
    createModel: (id, input) => checkpoint("model", id, input, async (snapshot) => {
      const response = await transport.createModel(id, snapshot);
      const model = response?.model;
      if (!model || model.project_id !== id || !validId(model.model_id) || !validId(model.latest_version_id)
          || !Number.isInteger(model.latest_version_number) || (model.latest_version_number ?? 0) < 1) {
        throw new Error("checkpoint:invalid_response");
      }
      return response;
    }),
    createModelVersion: (id, input) => checkpoint("version", id, input, async (snapshot) => {
      const response = await transport.createModelVersion(id, snapshot);
      const version = response?.version;
      if (!version || version.model_id !== id || !validId(version.version_id)
          || !Number.isInteger(version.version_number) || version.version_number < 1) {
        throw new Error("checkpoint:invalid_response");
      }
      return response;
    }),
    createProject: transport.createProject,
    deleteModel: transport.deleteModel,
    deleteModelVersion: transport.deleteModelVersion,
    deleteProject: transport.deleteProject,
    fetchModel: transport.fetchModel,
    fetchModelVersion: transport.fetchModelVersion,
    fetchModelVersions: transport.fetchModelVersions,
    fetchProjects: transport.fetchProjects,
    updateModel: transport.updateModel,
    updateModelVersion: transport.updateModelVersion,
    updateProject: transport.updateProject,
  };
}

function validId(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}
