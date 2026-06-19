"use client";

import type {
  WorkflowGraphArtifactEnvelope,
  WorkflowGraphArtifactValue,
  WorkflowGraphJobResult,
  WorkflowSummaryArtifactFieldValue,
  WorkflowSummaryArtifactPayload,
} from "@/lib/api";
import { WORKFLOW_SUMMARY_ARTIFACT_CONTRACT } from "@/lib/workbench/workflow-summary-contract";
import { listWorkflowDiagnosticsReports } from "@/components/workbench/workflow/workbench-workflow-diagnostics-report-contract";
import { summarizeWorkflowDiagnosticsReport } from "@/components/workbench/workflow/workbench-workflow-diagnostics-presentation";

export type WorkflowResolvedSummaryArtifact = {
  artifactKey: string;
  artifactType: string;
  nodeId?: string;
  portId?: string;
  payload: WorkflowSummaryArtifactPayload;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isFieldValue(value: unknown): value is WorkflowSummaryArtifactFieldValue {
  return (
    value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  );
}

function normalizeFieldMap(value: unknown) {
  if (!isRecord(value)) return null;
  const entries = Object.entries(value).filter((entry) => isFieldValue(entry[1]));
  if (entries.length === 0) return null;
  return Object.fromEntries(entries) as Record<string, WorkflowSummaryArtifactFieldValue>;
}

function normalizeMetadata(value: unknown) {
  if (!isRecord(value)) return undefined;
  const entries = Object.entries(value).filter((entry) => isFieldValue(entry[1]));
  return entries.length > 0
    ? (Object.fromEntries(entries) as Record<string, string | number | boolean | null>)
    : undefined;
}

function normalizeSummaryPayload(value: unknown): WorkflowSummaryArtifactPayload | null {
  if (!isRecord(value)) return null;
  const fields =
    normalizeFieldMap(value.fields) ??
    normalizeFieldMap(value.summary) ??
    normalizeFieldMap(value.metrics);
  if (!fields) return null;
  return {
    contract_version:
      typeof value.contract_version === "string" && value.contract_version.trim().length > 0
        ? value.contract_version
        : `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
    summary_kind:
      typeof value.summary_kind === "string" ? value.summary_kind : undefined,
    source_operator_id:
      typeof value.source_operator_id === "string" ? value.source_operator_id : undefined,
    source_artifact_type:
      typeof value.source_artifact_type === "string" ? value.source_artifact_type : undefined,
    field_namespace:
      typeof value.field_namespace === "string" ? value.field_namespace : undefined,
    fields,
    metadata: normalizeMetadata(value.metadata),
  };
}

function parseJsonString(value: string) {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return null;
  }
}

function normalizeEnvelope(value: WorkflowGraphArtifactValue) {
  return isRecord(value) ? (value as WorkflowGraphArtifactEnvelope) : null;
}

export function resolveWorkflowSummaryArtifact(
  artifactKey: string,
  artifact: WorkflowGraphArtifactValue,
): WorkflowResolvedSummaryArtifact | null {
  const directPayload = normalizeSummaryPayload(artifact);
  if (directPayload) {
    return {
      artifactKey,
      artifactType: "artifact/result_summary",
      payload: directPayload,
    };
  }
  const envelope = normalizeEnvelope(artifact);
  if (!envelope) return null;
  const payload =
    normalizeSummaryPayload(envelope.payload) ??
    normalizeSummaryPayload(envelope.content) ??
    (typeof envelope.content === "string"
      ? normalizeSummaryPayload(parseJsonString(envelope.content))
      : null);
  if (!payload) return null;
  return {
    artifactKey,
    artifactType: envelope.artifact_type ?? "artifact/result_summary",
    nodeId: envelope.node_id,
    portId: envelope.port_id,
    payload,
  };
}

export function listWorkflowSummaryArtifacts(result?: WorkflowGraphJobResult | null) {
  if (!result) return [] as WorkflowResolvedSummaryArtifact[];
  return Object.entries(result.artifacts ?? {})
    .map(([artifactKey, artifact]) => resolveWorkflowSummaryArtifact(artifactKey, artifact))
    .filter((entry): entry is WorkflowResolvedSummaryArtifact => Boolean(entry));
}

export function summarizeWorkflowResultArtifacts(result: WorkflowGraphJobResult): string | null {
  const diagnosticsReport = listWorkflowDiagnosticsReports(result)[0];
  const diagnosticsPreview = summarizeWorkflowDiagnosticsReport(diagnosticsReport);
  if (diagnosticsPreview) return diagnosticsPreview;
  const summaries = listWorkflowSummaryArtifacts(result);
  const firstSummary = summaries[0];
  if (firstSummary) {
    const preview = Object.entries(firstSummary.payload.fields)
      .slice(0, 3)
      .map(([key, value]) =>
        `${key}=${typeof value === "number" ? value.toExponential(3) : String(value)}`,
      )
      .join(", ");
    return preview || null;
  }
  const legacyArtifact = result.artifacts["json_output.json"];
  const envelope = normalizeEnvelope(legacyArtifact);
  const rawContent = envelope?.content ?? legacyArtifact;
  const parsed =
    typeof rawContent === "string"
      ? parseJsonString(rawContent)
      : rawContent;
  if (!isRecord(parsed)) return null;
  const preview = Object.entries(parsed)
    .slice(0, 3)
    .map(([key, value]) =>
      `${key}=${typeof value === "number" ? value.toExponential(3) : String(value)}`,
    )
    .join(", ");
  return preview || null;
}
