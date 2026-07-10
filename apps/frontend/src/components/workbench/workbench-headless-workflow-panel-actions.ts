import type { HeadlessWorkflowExecutionBatch } from "@/components/workbench/workbench-headless-workflow-export";
import { buildHeadlessAgentDispatchPlan } from "@/lib/scripting/workbench-headless-agent-dispatch";
import {
  buildHeadlessOrchestraHandoffEnvelope,
  type HeadlessOrchestraHandoffEnvelope,
} from "@/lib/scripting/workbench-headless-orchestra-handoff";
import type { WorkbenchHeadlessWorkflowBackendService } from "@/lib/workbench/headless-workflow-backend-service";
import type { FrontendRuntimeMode } from "@/lib/api/runtime-types";

export type WorkbenchHeadlessWorkflowAuthSnapshot = {
  clusterApiToken: string;
  controlPlaneApiToken: string;
  directMeshApiToken: string;
  directMeshEndpointsText: string;
  frontendRuntimeMode: FrontendRuntimeMode;
};

export async function buildHeadlessAgentDispatchPlanFromBackend(input: {
  batch: HeadlessWorkflowExecutionBatch;
  backendService: Pick<WorkbenchHeadlessWorkflowBackendService, "fetchProtocolAgents">;
}) {
  const payload = await input.backendService.fetchProtocolAgents();
  return buildHeadlessAgentDispatchPlan({ batch: input.batch, protocolAgents: payload.agents });
}

export async function buildHeadlessOrchestraHandoffFromBackend(input: {
  auth: WorkbenchHeadlessWorkflowAuthSnapshot;
  backendService: Pick<WorkbenchHeadlessWorkflowBackendService, "fetchProtocolAgents">;
  batch: HeadlessWorkflowExecutionBatch;
}) {
  const payload = await input.backendService.fetchProtocolAgents();
  return buildHeadlessOrchestraHandoffEnvelope({
    batch: input.batch,
    protocolAgents: payload.agents,
    frontendRuntimeMode: input.auth.frontendRuntimeMode,
    directMeshEndpointsText: input.auth.directMeshEndpointsText,
    controlPlaneApiToken: input.auth.controlPlaneApiToken,
    clusterApiToken: input.auth.clusterApiToken,
    directMeshApiToken: input.auth.directMeshApiToken,
  });
}

export async function submitHeadlessOrchestraHandoffFromBackend(input: {
  backendService: Pick<WorkbenchHeadlessWorkflowBackendService, "fetchProtocolAgents" | "submitHandoff">;
  buildAuthSnapshot: () => WorkbenchHeadlessWorkflowAuthSnapshot;
  batch: HeadlessWorkflowExecutionBatch;
}) {
  const handoff = await buildHeadlessOrchestraHandoffFromBackend({
    auth: input.buildAuthSnapshot(),
    backendService: input.backendService,
    batch: input.batch,
  });
  const receipt = await input.backendService.submitHandoff(handoff);
  return { handoff, receipt };
}

export function describeHeadlessHandoffReceiptForLog(receipt: unknown) {
  return `[handoff] ${JSON.stringify(receipt)}`;
}

export type BuiltHeadlessOrchestraHandoff = HeadlessOrchestraHandoffEnvelope;
