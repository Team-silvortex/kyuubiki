"use client";

import {
  createModel,
  createModelVersion,
  createProject,
  deleteModel,
  deleteModelVersion,
  deleteProject,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchProjects,
  updateModel,
  updateModelVersion,
  updateProject,
} from "@/lib/api/project-client";
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
  createModel,
  createModelVersion,
  createProject,
  deleteModel,
  deleteModelVersion,
  deleteProject,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchProjects,
  updateModel,
  updateModelVersion,
  updateProject,
});
