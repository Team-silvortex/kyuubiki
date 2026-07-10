"use client";

import { defaultRuntimeApiClient, type RuntimeApiClient } from "@/lib/api/runtime-client";
import { defaultSecurityResultsApiClient } from "@/lib/api/security-results-client";
import { createAdminDataBackendService } from "@/lib/workbench/admin-data-backend-service-core";
import { createJobHistoryBackendService } from "@/lib/workbench/job-history-backend-service-core";
import { createRuntimeStatusBackendService } from "@/lib/runtime-gateway/runtime-status-gateway";
import { createStudyRunBackendServiceFromRuntimeClient } from "@/lib/workbench/study-run-backend-service";
import { createWorkflowBackendService } from "@/lib/workbench/workflow-backend-service-core";

export function createWorkbenchRuntimeBackedBackendServices(runtimeClient: RuntimeApiClient) {
  return {
    adminData: createAdminDataBackendService({
      deleteJob: runtimeClient.deleteJobRecord,
      deleteResult: defaultSecurityResultsApiClient.deleteResultRecord,
      fetchJob: runtimeClient.fetchJobStatus,
      fetchResults: defaultSecurityResultsApiClient.fetchResults,
      updateJob: runtimeClient.updateJobRecord,
      updateResult: defaultSecurityResultsApiClient.updateResultRecord,
    }),
    jobHistory: createJobHistoryBackendService({
      cancelJob: runtimeClient.cancelJob,
      fetchJobHistory: runtimeClient.fetchJobHistory,
    }),
    runtimeStatus: createRuntimeStatusBackendService({
      fetchDirectMeshAgents: runtimeClient.fetchDirectMeshAgents,
      fetchHealth: runtimeClient.fetchHealth,
      fetchProtocolAgents: runtimeClient.fetchProtocolAgents,
      fetchRegisteredAgents: runtimeClient.fetchRegisteredAgents,
    }),
    studyRun: createStudyRunBackendServiceFromRuntimeClient(runtimeClient),
    workflow: createWorkflowBackendService({
      fetchCatalog: runtimeClient.fetchWorkflowCatalog,
      fetchOperators: runtimeClient.fetchWorkflowOperators,
      fetchJob: runtimeClient.fetchJobStatus,
      submitCatalogJob: runtimeClient.submitWorkflowCatalogJob,
      submitGraphJob: runtimeClient.submitWorkflowGraphJob,
    }),
  };
}

export type WorkbenchRuntimeBackedBackendServices = ReturnType<
  typeof createWorkbenchRuntimeBackedBackendServices
>;

export const defaultWorkbenchRuntimeBackedBackendServices =
  createWorkbenchRuntimeBackedBackendServices(defaultRuntimeApiClient);
