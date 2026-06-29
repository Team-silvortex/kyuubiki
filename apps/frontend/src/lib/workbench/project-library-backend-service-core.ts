"use client";

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
  kind: string;
  name: string;
  payload: Record<string, unknown>;
};

export type WorkbenchModelVersionCreateInput = WorkbenchModelMutationInput & {
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
): WorkbenchProjectLibraryBackendService {
  return {
    createModel: transport.createModel,
    createModelVersion: transport.createModelVersion,
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
