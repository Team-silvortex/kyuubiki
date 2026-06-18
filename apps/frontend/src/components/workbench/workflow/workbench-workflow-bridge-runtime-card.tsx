"use client";

import type { WorkflowGraphDefinition, WorkflowGraphJobResult } from "@/lib/api";
import {
  inspectWorkflowBridgeRuntimePaths,
  type WorkflowBridgeRuntimeNodeStatus,
  validateWorkflowBridgeRuntimeContracts,
  type WorkflowBridgeRuntimeInspectionRecord,
  type WorkflowBridgeRuntimeValidationIssue,
} from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";

type WorkbenchWorkflowBridgeRuntimeCardProps = {
  activeStatusFilter?: WorkflowBridgeRuntimeNodeStatus["label"] | null;
  graph?: WorkflowGraphDefinition | null;
  result?: WorkflowGraphJobResult | null;
  onLocateIssue?: (issue: WorkflowBridgeRuntimeValidationIssue) => void;
};

export function WorkbenchWorkflowBridgeRuntimeCard({
  activeStatusFilter = null,
  graph,
  result,
  onLocateIssue,
}: WorkbenchWorkflowBridgeRuntimeCardProps) {
  const issues = validateWorkflowBridgeRuntimeContracts(graph ?? null, result ?? null);
  const inspections = inspectWorkflowBridgeRuntimePaths(graph ?? null, result ?? null);
  const filteredInspections = activeStatusFilter
    ? inspections.filter((record) => resolveBridgeRuntimeInspectionStatus(record) === activeStatusFilter)
    : inspections;
  const filteredIssues = activeStatusFilter
    ? issues.filter((issue) => {
        const inspection = inspections.find((record) => record.nodeId === issue.nodeId);
        return inspection
          ? resolveBridgeRuntimeInspectionStatus(inspection) === activeStatusFilter
          : activeStatusFilter !== "aligned";
      })
    : issues;
  const previewIssues = filteredIssues.slice(0, 4);
  const previewInspections = filteredInspections.slice(0, 4);
  const statusSummary = activeStatusFilter ? ` (${activeStatusFilter})` : "";

  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>{`Bridge runtime contracts${statusSummary}`}</h2>
        <span className={`status-pill status-pill--${filteredIssues.length === 0 ? "good" : "watch"}`}>
          {filteredIssues.length}
        </span>
      </div>
      {previewIssues.length > 0 ? (
        <div style={{ display: "grid", gap: "0.35rem", marginTop: "0.75rem" }}>
          {previewIssues.map((issue) => (
            <BridgeRuntimeIssuePreview
              issue={issue}
              key={issue.id}
              onLocateIssue={onLocateIssue}
            />
          ))}
        </div>
      ) : (
        <p className="card-copy" style={{ marginTop: "0.75rem" }}>
          {activeStatusFilter === "aligned"
            ? "Filtered bridge nodes are exposing both expected source and target runtime fields."
            : activeStatusFilter === "missing-runtime"
              ? "Filtered bridge nodes are missing runtime payloads on the inspected path."
              : activeStatusFilter === "drift"
                ? "Filtered bridge nodes are exposing runtime payloads, but source or target fields drifted from contract expectations."
                : "Runtime bridge payloads expose the expected upstream source fields and downstream target fields."}
        </p>
      )}
      {filteredInspections.length > 0 ? (
        <details style={{ marginTop: "0.75rem" }}>
          <summary className="card-copy" style={{ cursor: "pointer" }}>
            {`runtime path inspection (${filteredInspections.length})`}
          </summary>
          <div style={{ display: "grid", gap: "0.45rem", marginTop: "0.55rem" }}>
            {previewInspections.map((record) => (
              <BridgeRuntimeInspectionPreview key={`${record.nodeId}:${record.operatorId}`} onLocateIssue={onLocateIssue} record={record} />
            ))}
          </div>
        </details>
      ) : null}
    </section>
  );
}

function resolveBridgeRuntimeInspectionStatus(
  record: WorkflowBridgeRuntimeInspectionRecord,
): WorkflowBridgeRuntimeNodeStatus["label"] {
  if (!record.inputArtifactKey && !record.outputArtifactKey) return "missing-runtime";
  if (record.sourceFieldExposed && record.targetFieldExposed) return "aligned";
  return "drift";
}

function BridgeRuntimeIssuePreview({
  issue,
  onLocateIssue,
}: {
  issue: WorkflowBridgeRuntimeValidationIssue;
  onLocateIssue?: (issue: WorkflowBridgeRuntimeValidationIssue) => void;
}) {
  return (
    <div style={{ display: "grid", gap: "0.15rem" }}>
      <strong style={{ fontSize: "0.9rem" }}>{issue.nodeId}</strong>
      <span className="card-copy">{issue.message}</span>
      {issue.artifactKey ? <span className="card-copy">{issue.artifactKey}</span> : null}
      {onLocateIssue ? (
        <div className="button-row">
          <button onClick={() => onLocateIssue(issue)} type="button">
            locate
          </button>
        </div>
      ) : null}
    </div>
  );
}

function BridgeRuntimeInspectionPreview({
  record,
  onLocateIssue,
}: {
  record: WorkflowBridgeRuntimeInspectionRecord;
  onLocateIssue?: (issue: WorkflowBridgeRuntimeValidationIssue) => void;
}) {
  const locateIssue = {
    id: `bridge:runtime:inspect:${record.nodeId}`,
    level: "warning" as const,
    message: `Inspect bridge runtime path for "${record.nodeId}".`,
    nodeId: record.nodeId,
    upstreamNodeId: record.upstreamNodeId,
    downstreamNodeIds: record.downstreamNodeIds,
  };
  return (
    <div style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
      <strong style={{ fontSize: "0.9rem" }}>{record.nodeId}</strong>
      <span className="card-copy">{`${record.upstreamNodeId ?? "--"} -> ${record.nodeId} -> ${record.downstreamNodeIds.join(", ") || "--"}`}</span>
      <span className="card-copy">{`${record.sourceField} -> ${record.targetField} | ${record.reduction} x ${record.scale}`}</span>
      <span className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
        <span className={`status-pill status-pill--${record.sourceFieldExposed ? "good" : "watch"}`}>{`source ${record.sourceFieldExposed ? "ok" : "missing"}`}</span>
        <span className={`status-pill status-pill--${record.targetFieldExposed ? "good" : "watch"}`}>{`target ${record.targetFieldExposed ? "ok" : "missing"}`}</span>
      </span>
      <span className="card-copy">{`${record.inputArtifactKey ?? "--"} => ${record.outputArtifactKey ?? "--"}`}</span>
      {onLocateIssue ? (
        <div className="button-row">
          <button onClick={() => onLocateIssue(locateIssue)} type="button">
            inspect chain
          </button>
        </div>
      ) : null}
    </div>
  );
}
