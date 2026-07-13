"use client";

import type { JobEnvelope } from "@/lib/api/fem-shared";
import type {
  WorkflowCatalogEntry,
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

export type WorkbenchWorkflowPreflightInput = {
  workflow: WorkflowCatalogEntry;
  operators?: WorkflowOperatorCatalogPayload["operators"];
};

export type WorkbenchWorkflowPreflightResult = {
  schema_version: string;
  ok: boolean;
  status: "ready" | "blocked";
  workflow_id: string;
  issues: Array<{
    id: string;
    level: "info" | "warning" | "error";
    message: string;
    source: string;
  }>;
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
  preflightWorkflow?(
    input: WorkbenchWorkflowPreflightInput,
  ): Promise<WorkbenchWorkflowPreflightResult>;
};

export type WorkbenchWorkflowBackendService = {
  fetchCatalog(query?: WorkflowCatalogQuery): Promise<WorkflowCatalogPayload>;
  fetchOperators(query?: WorkflowOperatorCatalogQuery): Promise<WorkflowOperatorCatalogPayload>;
  preflightWorkflow(
    input: WorkbenchWorkflowPreflightInput,
  ): Promise<WorkbenchWorkflowPreflightResult>;
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
    preflightWorkflow(input) {
      return transport.preflightWorkflow
        ? transport.preflightWorkflow(input)
        : Promise.resolve(localUnsupportedPreflight(input.workflow.id));
    },
    submitWorkflow(input) {
      return input.graph
        ? transport.submitGraphJob(input.graph, input.inputArtifacts)
        : transport.submitCatalogJob(input.workflowId, input.inputArtifacts);
    },
  };
}

function localUnsupportedPreflight(workflowId: string): WorkbenchWorkflowPreflightResult {
  return {
    schema_version: "kyuubiki.workbench-workflow-preflight/v1",
    ok: false,
    status: "blocked",
    workflow_id: workflowId,
    issues: [
      {
        id: "workflow:preflight:not-configured",
        level: "warning",
        message: "Workflow preflight is not configured on this backend service.",
        source: "runtime_api",
      },
    ],
  };
}
