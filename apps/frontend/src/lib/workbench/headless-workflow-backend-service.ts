"use client";

import { defaultHeadlessHandoffApiClient, type HeadlessHandoffApiClient } from "@/lib/api/headless-handoff-client";
import { defaultRuntimeApiClient, type RuntimeApiClient } from "@/lib/api/runtime-client";

export type WorkbenchHeadlessWorkflowBackendService = {
  fetchHandoffHistory: HeadlessHandoffApiClient["fetchHeadlessOrchestraHandoffHistory"];
  fetchHandoffSnapshot: HeadlessHandoffApiClient["fetchHeadlessOrchestraHandoffSnapshot"];
  fetchHandoffStatus: HeadlessHandoffApiClient["fetchHeadlessOrchestraHandoffStatus"];
  fetchProtocolAgents: RuntimeApiClient["fetchProtocolAgents"];
  submitHandoff: HeadlessHandoffApiClient["submitHeadlessOrchestraHandoff"];
};

export function createWorkbenchHeadlessWorkflowBackendService(input: {
  handoffClient: HeadlessHandoffApiClient;
  runtimeClient: RuntimeApiClient;
}): WorkbenchHeadlessWorkflowBackendService {
  return {
    fetchHandoffHistory: input.handoffClient.fetchHeadlessOrchestraHandoffHistory,
    fetchHandoffSnapshot: input.handoffClient.fetchHeadlessOrchestraHandoffSnapshot,
    fetchHandoffStatus: input.handoffClient.fetchHeadlessOrchestraHandoffStatus,
    fetchProtocolAgents: input.runtimeClient.fetchProtocolAgents,
    submitHandoff: input.handoffClient.submitHeadlessOrchestraHandoff,
  };
}

export const defaultWorkbenchHeadlessWorkflowBackendService =
  createWorkbenchHeadlessWorkflowBackendService({
    handoffClient: defaultHeadlessHandoffApiClient,
    runtimeClient: defaultRuntimeApiClient,
  });
