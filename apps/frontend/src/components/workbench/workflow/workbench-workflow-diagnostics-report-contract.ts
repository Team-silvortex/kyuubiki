import type {
  WorkflowGraphArtifactEnvelope,
  WorkflowGraphArtifactValue,
  WorkflowGraphJobResult,
  WorkflowSummaryArtifactFieldValue,
} from "@/lib/api";

export type WorkflowDiagnosticsReportHighlight = {
  id: string;
  label: string;
  value: WorkflowSummaryArtifactFieldValue;
  attention: boolean;
};

export type WorkflowDiagnosticsReportPayload = {
  report_contract: string;
  report_kind: string;
  report_focus_metrics: Record<string, WorkflowSummaryArtifactFieldValue>;
  report_highlights: WorkflowDiagnosticsReportHighlight[];
  report_guard_status?: string;
  report_guard_recommendation?: string;
};

export type WorkflowResolvedDiagnosticsReport = {
  artifactKey: string;
  artifactType: string;
  nodeId?: string;
  portId?: string;
  payload: WorkflowDiagnosticsReportPayload;
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

function normalizeFocusMetrics(value: unknown) {
  if (!isRecord(value)) return null;
  const entries = Object.entries(value).filter((entry) => isFieldValue(entry[1]));
  if (entries.length === 0) return null;
  return Object.fromEntries(entries) as Record<string, WorkflowSummaryArtifactFieldValue>;
}

function normalizeHighlights(value: unknown) {
  if (!Array.isArray(value)) return null;
  const items = value.flatMap((entry) => {
    if (!isRecord(entry)) return [];
    if (
      typeof entry.id !== "string" ||
      typeof entry.label !== "string" ||
      !isFieldValue(entry.value)
    ) {
      return [];
    }
    return [
      {
        id: entry.id,
        label: entry.label,
        value: entry.value,
        attention: entry.attention === true,
      } satisfies WorkflowDiagnosticsReportHighlight,
    ];
  });
  return items.length > 0 ? items : null;
}

function normalizeDiagnosticsReportPayload(value: unknown): WorkflowDiagnosticsReportPayload | null {
  if (!isRecord(value)) return null;
  if (value.report_contract !== "kyuubiki.workflow_report_payload/v1") return null;
  if (value.report_kind !== "diagnostics_bundle_report_payload") return null;
  const report_focus_metrics = normalizeFocusMetrics(value.report_focus_metrics) ?? {};
  const report_highlights = normalizeHighlights(value.report_highlights) ?? [];
  if (Object.keys(report_focus_metrics).length === 0 && report_highlights.length === 0) return null;
  return {
    report_contract: value.report_contract,
    report_kind: value.report_kind,
    report_focus_metrics,
    report_highlights,
    report_guard_status:
      typeof value.report_guard_status === "string" ? value.report_guard_status : undefined,
    report_guard_recommendation:
      typeof value.report_guard_recommendation === "string"
        ? value.report_guard_recommendation
        : undefined,
  };
}

export function resolveWorkflowDiagnosticsReportArtifact(
  artifactKey: string,
  artifact: WorkflowGraphArtifactValue,
): WorkflowResolvedDiagnosticsReport | null {
  const directPayload = normalizeDiagnosticsReportPayload(artifact);
  if (directPayload) {
    return {
      artifactKey,
      artifactType: "artifact/workflow_report_payload",
      payload: directPayload,
    };
  }
  const envelope = normalizeEnvelope(artifact);
  if (!envelope) return null;
  const payload =
    normalizeDiagnosticsReportPayload(envelope.payload) ??
    normalizeDiagnosticsReportPayload(envelope.content) ??
    (typeof envelope.content === "string"
      ? normalizeDiagnosticsReportPayload(parseJsonString(envelope.content))
      : null);
  if (!payload) return null;
  return {
    artifactKey,
    artifactType: envelope.artifact_type ?? "artifact/workflow_report_payload",
    nodeId: envelope.node_id,
    portId: envelope.port_id,
    payload,
  };
}

export function listWorkflowDiagnosticsReports(result?: WorkflowGraphJobResult | null) {
  if (!result) return [] as WorkflowResolvedDiagnosticsReport[];
  return Object.entries(result.artifacts ?? {})
    .map(([artifactKey, artifact]) =>
      resolveWorkflowDiagnosticsReportArtifact(artifactKey, artifact),
    )
    .filter((entry): entry is WorkflowResolvedDiagnosticsReport => Boolean(entry));
}
