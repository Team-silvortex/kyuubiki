"use client";

import type { FrontendRuntimeMode, ProtocolAgentDescriptor } from "@/lib/api";
import type { HeadlessWorkflowExecutionBatch } from "@/components/workbench/workbench-headless-workflow-export";
import {
  buildWorkbenchGovernanceConfig,
  buildWorkbenchGovernanceRuntimeDiagnostics,
  resolveWorkbenchAuthorityMode,
} from "@/lib/workbench/governance";
import {
  buildHeadlessAgentDispatchPlan,
  type HeadlessAgentDispatchPlan,
} from "@/lib/scripting/workbench-headless-agent-dispatch";
import type {
  HeadlessDispatchOverrideDocument,
  HeadlessDispatchOverrideReference,
} from "@/lib/scripting/workbench-headless-dispatch-override";

export type HeadlessOrchestraHandoffEnvelope = {
  schema_version: "kyuubiki.headless-orchestra-handoff/v1";
  generated_at: string;
  workflow_id: string;
  execution_batch: HeadlessWorkflowExecutionBatch;
  dispatch_plan: HeadlessAgentDispatchPlan;
  governance: {
    config: ReturnType<typeof buildWorkbenchGovernanceConfig>;
    diagnostics: ReturnType<typeof buildWorkbenchGovernanceRuntimeDiagnostics>;
  };
  runtime_manifest: {
    authority_mode: "single_orchestrator" | "offline_mesh";
    source_of_truth: "central_orchestrator_library";
    agent_library_replication: "forbidden";
    target_clusters: string[];
    target_runtime_modes: string[];
  };
  dispatch_override_ref?: HeadlessDispatchOverrideReference;
  dispatch_overrides?: HeadlessDispatchOverrideDocument["overrides"];
};

export function buildHeadlessOrchestraHandoffEnvelope(input: {
  batch: HeadlessWorkflowExecutionBatch;
  protocolAgents: ProtocolAgentDescriptor[];
  frontendRuntimeMode: FrontendRuntimeMode;
  directMeshEndpointsText: string;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
}) {
  const dispatchPlan = buildHeadlessAgentDispatchPlan({
    batch: input.batch,
    protocolAgents: input.protocolAgents,
  });
  const governanceConfig = buildWorkbenchGovernanceConfig({
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
    controlPlaneApiToken: input.controlPlaneApiToken,
    clusterApiToken: input.clusterApiToken,
    directMeshApiToken: input.directMeshApiToken,
    protocolAgents: input.protocolAgents,
  });
  const governanceDiagnostics = buildWorkbenchGovernanceRuntimeDiagnostics({
    frontendRuntimeMode: input.frontendRuntimeMode,
    directMeshEndpointsText: input.directMeshEndpointsText,
    protocolAgents: input.protocolAgents,
    controlPlaneApiToken: input.controlPlaneApiToken,
    clusterApiToken: input.clusterApiToken,
    directMeshApiToken: input.directMeshApiToken,
  });
  const authorityMode = resolveWorkbenchAuthorityMode({
    frontendRuntimeMode: governanceConfig.controlMode,
    protocolAgents: input.protocolAgents,
  });

  return {
    schema_version: "kyuubiki.headless-orchestra-handoff/v1",
    generated_at: new Date().toISOString(),
    workflow_id: input.batch.workflow_id,
    execution_batch: input.batch,
    dispatch_plan: dispatchPlan,
    governance: {
      config: governanceConfig,
      diagnostics: governanceDiagnostics,
    },
    runtime_manifest: {
      authority_mode: authorityMode,
      source_of_truth: "central_orchestrator_library",
      agent_library_replication: "forbidden",
      target_clusters: governanceDiagnostics.visibleClusterIds,
      target_runtime_modes: governanceDiagnostics.visibleRuntimeModes,
    },
  } satisfies HeadlessOrchestraHandoffEnvelope;
}

export function asHeadlessOrchestraHandoffEnvelope(value: unknown): HeadlessOrchestraHandoffEnvelope | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Record<string, unknown>;
  if (candidate.schema_version !== "kyuubiki.headless-orchestra-handoff/v1") return null;
  if (typeof candidate.generated_at !== "string" || typeof candidate.workflow_id !== "string") return null;
  if (!candidate.execution_batch || typeof candidate.execution_batch !== "object") return null;
  if (!candidate.dispatch_plan || typeof candidate.dispatch_plan !== "object") return null;
  if (!candidate.runtime_manifest || typeof candidate.runtime_manifest !== "object") return null;
  if (!candidate.governance || typeof candidate.governance !== "object") return null;
  return candidate as HeadlessOrchestraHandoffEnvelope;
}

export function attachDispatchOverrideDraft(input: {
  handoff: HeadlessOrchestraHandoffEnvelope;
  document: HeadlessDispatchOverrideDocument;
}): HeadlessOrchestraHandoffEnvelope {
  return {
    ...input.handoff,
    dispatch_override_ref: {
      schema_version: "kyuubiki.headless-dispatch-override-ref/v1",
      handoff_id: input.document.handoff_id,
      workflow_id: input.document.workflow_id,
      override_count: input.document.overrides.length,
    },
    dispatch_overrides: input.document.overrides,
  };
}
