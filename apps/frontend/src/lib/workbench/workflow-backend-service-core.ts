"use client";

import type { JobEnvelope } from "@/lib/api/fem-shared";
import type {
  WorkflowCatalogPayload,
  WorkflowCatalogQuery,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
  WorkflowOperatorCatalogPayload,
  WorkflowOperatorCatalogQuery,
} from "@/lib/api/workflow-types";

export type WorkbenchWorkflowSubmitInput = {
  graph?: WorkflowGraphDefinition;
  inputArtifacts: Record<string, unknown>;
  workflowId: string;
};

export type WorkbenchWorkflowBackendTransport = {
  fetchCatalog(query?: WorkflowCatalogQuery): Promise<WorkflowCatalogPayload>;
  fetchOperators(query?: WorkflowOperatorCatalogQuery): Promise<WorkflowOperatorCatalogPayload>;
  submitCatalogJob(
    workflowId: string,
    inputArtifacts: Record<string, unknown>,
  ): Promise<JobEnvelope<WorkflowGraphJobResult>>;
  submitGraphJob(
    graph: WorkflowGraphDefinition,
    inputArtifacts: Record<string, unknown>,
  ): Promise<JobEnvelope<WorkflowGraphJobResult>>;
  fetchJob<TResult>(jobId: string): Promise<JobEnvelope<TResult>>;
};

export type WorkbenchWorkflowBackendService = {
  fetchCatalog(query?: WorkflowCatalogQuery): Promise<WorkflowCatalogPayload>;
  fetchOperators(query?: WorkflowOperatorCatalogQuery): Promise<WorkflowOperatorCatalogPayload>;
  submitWorkflow(input: WorkbenchWorkflowSubmitInput): Promise<JobEnvelope<WorkflowGraphJobResult>>;
  fetchJob<TResult>(jobId: string): Promise<JobEnvelope<TResult>>;
};

export function createWorkflowBackendService(
  transport: WorkbenchWorkflowBackendTransport,
): WorkbenchWorkflowBackendService {
  return {
    fetchCatalog: transport.fetchCatalog,
    fetchOperators: transport.fetchOperators,
    fetchJob: transport.fetchJob,
    submitWorkflow(input) {
      return input.graph
        ? transport.submitGraphJob(input.graph, input.inputArtifacts)
        : transport.submitCatalogJob(input.workflowId, input.inputArtifacts);
    },
  };
}
