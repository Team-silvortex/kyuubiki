"use client";

import {
  fetchJobStatus,
  fetchWorkflowCatalog,
  fetchWorkflowOperators,
  submitWorkflowCatalogJob,
  submitWorkflowGraphJob,
} from "@/lib/api/runtime-client";
import {
  createWorkflowBackendService,
  type WorkbenchWorkflowBackendService,
  type WorkbenchWorkflowBackendTransport,
  type WorkbenchWorkflowSubmitInput,
} from "@/lib/workbench/workflow-backend-service-core";

export {
  createWorkflowBackendService,
  type WorkbenchWorkflowBackendService,
  type WorkbenchWorkflowBackendTransport,
  type WorkbenchWorkflowSubmitInput,
};

export const orchestratedWorkflowBackendService = createWorkflowBackendService({
  fetchCatalog: fetchWorkflowCatalog,
  fetchOperators: fetchWorkflowOperators,
  fetchJob: fetchJobStatus,
  submitCatalogJob: submitWorkflowCatalogJob,
  submitGraphJob: submitWorkflowGraphJob,
});
