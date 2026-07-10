"use client";

import { defaultWorkbenchRuntimeBackedBackendServices } from "@/lib/workbench/backend-service-composer";
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

export const orchestratedWorkflowBackendService =
  defaultWorkbenchRuntimeBackedBackendServices.workflow;
