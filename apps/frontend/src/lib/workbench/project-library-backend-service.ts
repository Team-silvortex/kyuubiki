"use client";

import { defaultProjectApiClient } from "@/lib/api/project-client";
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
});
