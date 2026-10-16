"use client";

import { defaultProjectApiClient } from "@/lib/api/project-client";
import { buildWorkbenchApiAuthHeaders, captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { resolveWorkbenchApiUrl } from "@/lib/api/backend-target";
import { createWorkbenchCheckpointJournal } from "./checkpoint-journal";
import { createCheckpointRecovery } from "./checkpoint-recovery";
import {
  createProjectLibraryBackendService,
  type WorkbenchProjectCreateInput,
  type WorkbenchProjectLibraryBackendService,
  type WorkbenchProjectLibraryBackendTransport,
} from "@/lib/workbench/project-library-backend-service-core";

export {
  createProjectLibraryBackendService,
  type WorkbenchProjectCreateInput,
  type WorkbenchProjectLibraryBackendService,
  type WorkbenchProjectLibraryBackendTransport,
};

const journal = createWorkbenchCheckpointJournal();
const scope = () => JSON.stringify([resolveWorkbenchApiUrl("/api/v1/projects"), buildWorkbenchApiAuthHeaders("/api/v1/projects")]);
const captureAuthority = () => captureWorkbenchApiAuthority("/api/v1/projects", "/api/v1/models", "/api/v1/model-versions", "/api/v1/checkpoints");

export const workbenchCheckpointRecovery = createCheckpointRecovery({ journal, scope, captureAuthority, lookup: defaultProjectApiClient.fetchCheckpoint });

export const workbenchProjectLibraryBackendService = createProjectLibraryBackendService({
  createModel: defaultProjectApiClient.createModel,
  createModelVersion: defaultProjectApiClient.createModelVersion,
  createProject: defaultProjectApiClient.createProject,
  deleteModel: defaultProjectApiClient.deleteModel,
  deleteModelVersion: defaultProjectApiClient.deleteModelVersion,
  deleteProject: defaultProjectApiClient.deleteProject,
  fetchModel: defaultProjectApiClient.fetchModel,
  fetchModelVersion: defaultProjectApiClient.fetchModelVersion,
  fetchModelVersions: defaultProjectApiClient.fetchModelVersions,
  fetchProjects: defaultProjectApiClient.fetchProjects,
  updateModel: defaultProjectApiClient.updateModel,
  updateModelVersion: defaultProjectApiClient.updateModelVersion,
  updateProject: defaultProjectApiClient.updateProject,
}, {
  // Only a digest is retained by the retry manager, never credentials or mesh payloads.
  scope,
  captureAuthority,
  journal,
});
