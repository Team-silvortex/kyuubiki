"use client";

import { validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import {
  inspectWorkflowBridgeRuntimePaths,
  validateWorkflowBridgeRuntimeContracts,
} from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";
import {
  formatWorkflowContractHealthSummary,
  formatWorkflowDynamicReviewState,
} from "@/components/workbench/workflow/workbench-workflow-contract-health";
import { collectWorkflowInputArtifactContractWarnings } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import { buildWorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { findStoredLocalWorkflow } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { isWorkflowNodeSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import { listStoredWorkflowSnapshots } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
import { listWorkflowDiagnosticsReports } from "@/components/workbench/workflow/workbench-workflow-diagnostics-report-contract";
import {
  buildWorkflowDiagnosticsFocusCardSummary,
  formatWorkflowDiagnosticsMetricValue,
  orderWorkflowDiagnosticsFocusMetrics,
  orderWorkflowDiagnosticsHighlights,
  resolveWorkflowDiagnosticsFocusContext,
  resolveWorkflowDiagnosticsReportMode,
  resolveWorkflowDiagnosticsReportTitle,
  summarizeWorkflowDiagnosticsReport,
} from "@/components/workbench/workflow/workbench-workflow-diagnostics-presentation";
import { listWorkflowSummaryArtifacts } from "@/components/workbench/workflow/workbench-workflow-summary-contract";
import {
  isWorkflowBridgeContractOperator,
  resolveBridgeContractForOperator,
  resolveBridgeSeedModelForOperator,
} from "@/components/workbench/workflow/workbench-workflow-bridge-contract";
import {
  resolveWorkflowTraceBranchPredicateTone,
  resolveWorkflowTraceContractHealthTone,
  resolveWorkflowTraceHeaderHealthLabel,
  resolveWorkflowTraceLineageSourceLabel,
  resolveWorkflowTraceLineageSourceTone,
  resolveWorkflowTraceNodeRunTone,
  resolveWorkflowTraceProgressStageTone,
} from "@/components/workbench/workflow/workbench-workflow-trace-status";
import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";
import { readSecurityAuditLog } from "@/lib/workbench/security-audit";
import type { WorkflowCatalogEntry, WorkflowOperatorDescriptor } from "@/lib/api";

type WorkflowRunAuditReportOptions = {
  run: WorkflowRunRecord;
  workflow?: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
};

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function renderList(values: string[] | undefined) {
  if (!values || values.length === 0) return "<li>--</li>";
  return values.map((value) => `<li>${escapeHtml(value)}</li>`).join("");
}

function renderRows(rows: Array<[string, string]>) {
  return rows
    .map(
      ([label, value]) =>
        `<tr><th>${escapeHtml(label)}</th><td>${escapeHtml(value)}</td></tr>`,
    )
    .join("");
}

function renderToneBadge(label: string, tone: "good" | "watch" | "risk") {
  return `<span class="pill pill--${tone}">${escapeHtml(label)}</span>`;
}

function renderWarningRows(warnings?: Record<string, string[]>) {
  return Object.entries(warnings ?? {})
    .flatMap(([nodeId, lines]) =>
      lines.map(
        (line, index) =>
          `<tr><td>${escapeHtml(nodeId)}</td><td>${escapeHtml(String(index + 1))}</td><td>${escapeHtml(line)}</td></tr>`,
      ),
    )
    .join("");
}

function renderIssueRows(workflow?: WorkflowCatalogEntry | null, operatorDescriptors?: WorkflowOperatorDescriptor[]) {
  const graph = workflow?.graph;
  if (!graph) return "";
  const issues = validateWorkflowGraphDefinition(
    graph,
    workflow?.entry_inputs ?? [],
    workflow?.output_artifacts ?? [],
    operatorDescriptors ?? [],
  );
  return issues
    .map(
      (issue) =>
        `<tr><td>${escapeHtml(issue.level)}</td><td>${escapeHtml(issue.message)}</td><td>${escapeHtml(issue.locate?.kind ?? "--")}</td><td>${escapeHtml(issue.fix?.kind ?? "--")}</td></tr>`,
    )
    .join("");
}

function renderRuntimeRows(workflow?: WorkflowCatalogEntry | null) {
  const graph = workflow?.graph;
  if (!graph) return "";
  return graph.nodes
    .map((node) => {
      const supported = isWorkflowNodeSupportedInRuntime(node);
      return `<tr><td>${escapeHtml(node.id)}</td><td>${escapeHtml(node.kind)}</td><td>${escapeHtml(node.operator_id ?? "--")}</td><td class="${supported ? "good" : "risk"}">${supported ? "supported" : "unsupported"}</td></tr>`;
    })
    .join("");
}

function renderDatasetValueRows(workflow?: WorkflowCatalogEntry | null) {
  const values = workflow?.graph?.dataset_contract?.values ?? [];
  return values
    .map(
      (value) =>
        `<tr><td>${escapeHtml(value.id)}</td><td>${escapeHtml(value.data_class)}</td><td>${escapeHtml(value.element_type)}</td><td>${escapeHtml(value.semantic_type ?? "--")}</td><td>${escapeHtml(value.encoding ?? "--")}</td><td>${escapeHtml(String(value.shape?.axes?.length ?? 0))}</td></tr>`,
    )
    .join("");
}

function renderSummaryArtifactRows(run: WorkflowRunRecord) {
  return listWorkflowSummaryArtifacts(run.result ?? null)
    .map(
      (artifact) =>
        `<tr><td>${escapeHtml(artifact.artifactKey)}</td><td>${escapeHtml(artifact.artifactType)}</td><td>${escapeHtml(artifact.payload.summary_kind ?? "--")}</td><td>${escapeHtml(artifact.payload.source_operator_id ?? "--")}</td><td>${escapeHtml(String(Object.keys(artifact.payload.fields).length))}</td><td>${escapeHtml(artifact.nodeId ?? "--")}</td></tr>`,
    )
    .join("");
}

function renderDiagnosticsHighlightRows(run: WorkflowRunRecord) {
  const report = listWorkflowDiagnosticsReports(run.result ?? null)[0];
  if (!report) return "";
  const mode = resolveWorkflowDiagnosticsReportMode(report);
  return orderWorkflowDiagnosticsHighlights(report)
    .map(
      (highlight) =>
        `<tr><td>${renderToneBadge(mode === "peak" ? (highlight.attention ? "peak" : "review") : (highlight.attention ? "attention" : "info"), highlight.attention ? "risk" : "watch")}</td><td>${escapeHtml(highlight.label)}</td><td>${escapeHtml(formatWorkflowDiagnosticsMetricValue(highlight.value))}</td><td>${escapeHtml(highlight.id)}</td><td>${renderDiagnosticsContextSections(highlight.id, report)}</td></tr>`,
    )
    .join("");
}

function renderDiagnosticsFocusRows(run: WorkflowRunRecord) {
  const report = listWorkflowDiagnosticsReports(run.result ?? null)[0];
  if (!report) return "";
  return orderWorkflowDiagnosticsFocusMetrics(report)
    .map(
      ([key, value]) =>
        `<tr><td>${escapeHtml(key)}</td><td>${escapeHtml(formatWorkflowDiagnosticsMetricValue(value))}</td><td>${renderDiagnosticsContextSections(key, report)}</td></tr>`,
    )
    .join("");
}

function renderDiagnosticsContextSections(
  key: string,
  report: NonNullable<ReturnType<typeof listWorkflowDiagnosticsReports>[number]>,
) {
  const summary = buildWorkflowDiagnosticsFocusCardSummary(
    key,
    resolveWorkflowDiagnosticsFocusContext(report, key),
  );
  if (summary.sections.length === 0) return "<ul><li>--</li></ul>";
  return summary.sections
    .map(
      (section) =>
        `<div><strong>${escapeHtml(section.label)}</strong><ul>${renderList(section.lines)}</ul></div>`,
    )
    .join("");
}

function renderSnapshotRows(snapshots: ReturnType<typeof listStoredWorkflowSnapshots>) {
  return snapshots
    .slice(0, 6)
    .map(
      (snapshot) =>
        `<tr><td>${escapeHtml(snapshot.createdAt)}</td><td>${escapeHtml(snapshot.reason)}</td><td>${escapeHtml(snapshot.payloadState)}</td><td><ul>${renderList(snapshot.summary)}</ul></td></tr>`,
    )
    .join("");
}

function renderSecurityAuditRows(entries: ReturnType<typeof readSecurityAuditLog>) {
  return entries
    .slice(-8)
    .reverse()
    .map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.at)}</td><td>${escapeHtml(entry.source)}</td><td>${escapeHtml(entry.risk)}</td><td>${escapeHtml(entry.status)}</td><td>${escapeHtml(entry.action)}</td></tr>`,
    )
    .join("");
}

function renderIntegrityRows(
  workflow?: WorkflowCatalogEntry | null,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  return buildWorkflowIntegrityReport(workflow, operatorDescriptors ?? []).issues
    .map(
      (issue) =>
        `<tr><td>${escapeHtml(issue.scope)}</td><td>${escapeHtml(issue.severity)}</td><td>${escapeHtml(issue.message)}</td><td>${escapeHtml(issue.detail ?? "--")}</td></tr>`,
    )
    .join("");
}

function renderBridgeRuntimeRows(
  run: WorkflowRunRecord,
  workflow?: WorkflowCatalogEntry | null,
) {
  return validateWorkflowBridgeRuntimeContracts(workflow?.graph ?? null, run.result ?? null)
    .map(
      (issue) =>
        `<tr><td>${escapeHtml(issue.nodeId)}</td><td>${escapeHtml(issue.level)}</td><td>${escapeHtml(issue.artifactKey ?? "--")}</td><td>${escapeHtml(issue.message)}</td></tr>`,
    )
    .join("");
}

function renderBridgeRuntimeInspectionRows(
  run: WorkflowRunRecord,
  workflow?: WorkflowCatalogEntry | null,
) {
  return inspectWorkflowBridgeRuntimePaths(workflow?.graph ?? null, run.result ?? null)
    .map(
      (record) =>
        `<tr><td>${escapeHtml(record.nodeId)}</td><td>${escapeHtml(record.upstreamNodeId ?? "--")}</td><td>${escapeHtml(record.downstreamNodeIds.join(", ") || "--")}</td><td>${escapeHtml(`${record.sourceField} -> ${record.targetField}`)}</td><td>${escapeHtml(`${record.reduction} x ${String(record.scale)}`)}</td><td>${renderToneBadge(record.sourceFieldExposed ? "source ok" : "source missing", record.sourceFieldExposed ? "good" : "watch")}</td><td>${renderToneBadge(record.targetFieldExposed ? "target ok" : "target missing", record.targetFieldExposed ? "good" : "watch")}</td><td>${escapeHtml(`${record.inputArtifactKey ?? "--"} => ${record.outputArtifactKey ?? "--"}`)}</td></tr>`,
    )
    .join("");
}

function summarizeBridgeSeedModel(value: unknown) {
  const seedModel = typeof value === "object" && value !== null ? (value as Record<string, unknown>) : null;
  const nodes = Array.isArray(seedModel?.nodes) ? seedModel.nodes.length : 0;
  const elements = Array.isArray(seedModel?.elements) ? seedModel.elements.length : 0;
  return `${nodes} nodes / ${elements} elements`;
}

function renderBridgeContractCompareRows(
  run: WorkflowRunRecord,
  workflow?: WorkflowCatalogEntry | null,
) {
  const graph = workflow?.graph;
  if (!graph) return "";
  const runtimeByNodeId = new Map(
    inspectWorkflowBridgeRuntimePaths(graph, run.result ?? null).map((record) => [record.nodeId, record] as const),
  );
  return graph.nodes
    .filter((node) => isWorkflowBridgeContractOperator(node.operator_id))
    .map((node) => {
      const config = node.config as Record<string, unknown> | null | undefined;
      const contract = resolveBridgeContractForOperator(node.operator_id, config);
      const seedModel = resolveBridgeSeedModelForOperator(node.operator_id, config);
      const runtime = runtimeByNodeId.get(node.id);
      const aligned = runtime ? runtime.sourceFieldExposed && runtime.targetFieldExposed : false;
      return `<tr><td>${escapeHtml(node.id)}</td><td>${escapeHtml(contract ? `${contract.source.field} -> ${contract.target.field}` : "--")}</td><td>${escapeHtml(contract ? `${contract.transform.reduction} x ${String(contract.transform.scale)}` : "--")}</td><td>${escapeHtml(summarizeBridgeSeedModel(seedModel))}</td><td>${escapeHtml(runtime?.upstreamNodeId ?? "--")}</td><td>${escapeHtml(runtime?.downstreamNodeIds.join(", ") || "--")}</td><td>${renderToneBadge(runtime ? (runtime.sourceFieldExposed ? "source ok" : "source missing") : "no runtime", runtime ? (runtime.sourceFieldExposed ? "good" : "watch") : "risk")}</td><td>${renderToneBadge(runtime ? (runtime.targetFieldExposed ? "target ok" : "target missing") : "no runtime", runtime ? (runtime.targetFieldExposed ? "good" : "watch") : "risk")}</td><td>${renderToneBadge(aligned ? "aligned" : "check", aligned ? "good" : "watch")}</td></tr>`;
    })
    .join("");
}

function renderProgressTimelineRows(run: WorkflowRunRecord) {
  return (run.traceSummary?.recentProgressEvents ?? [])
    .map(
      (event) =>
        `<tr><td>${renderToneBadge(event.stage, resolveWorkflowTraceProgressStageTone(event.stage))}</td><td>${escapeHtml(`${Math.round(event.progress * 100)}%`)}</td><td>${escapeHtml(event.kind ?? "--")}</td><td>${escapeHtml(event.nodeId ?? "--")}</td><td>${escapeHtml(event.label ?? "--")}</td><td>${escapeHtml(event.emittedAt ?? "--")}</td></tr>`,
    )
    .join("");
}

export function buildWorkflowRunAuditReportHtml({
  run,
  workflow,
  operatorDescriptors,
}: WorkflowRunAuditReportOptions) {
  const graph = workflow?.graph;
  const integrity = buildWorkflowIntegrityReport(workflow, operatorDescriptors ?? []);
  const localWorkflow =
    workflow?.local?.storage_id ? findStoredLocalWorkflow(workflow.local.storage_id) : null;
  const contractWarnings = workflow
    ? collectWorkflowInputArtifactContractWarnings({
        entryInputs: workflow.entry_inputs,
        inputArtifactTexts: workflow.local?.input_artifact_texts,
      })
    : undefined;
  const snapshots = listStoredWorkflowSnapshots(run.workflowId);
  const securityAuditEntries = readSecurityAuditLog();
  const snapshotRows = renderSnapshotRows(snapshots);
  const securityAuditRows = renderSecurityAuditRows(securityAuditEntries);
  const integrityRows = renderIntegrityRows(workflow, operatorDescriptors);
  const bridgeRuntimeRows = renderBridgeRuntimeRows(run, workflow);
  const bridgeRuntimeInspectionRows = renderBridgeRuntimeInspectionRows(run, workflow);
  const bridgeContractCompareRows = renderBridgeContractCompareRows(run, workflow);
  const validationRows = renderIssueRows(workflow, operatorDescriptors);
  const runtimeRows = renderRuntimeRows(workflow);
  const datasetRows = renderDatasetValueRows(workflow);
  const summaryArtifactRows = renderSummaryArtifactRows(run);
  const diagnosticsHighlightRows = renderDiagnosticsHighlightRows(run);
  const diagnosticsFocusRows = renderDiagnosticsFocusRows(run);
  const diagnosticsReport = listWorkflowDiagnosticsReports(run.result ?? null)[0] ?? null;
  const diagnosticsPreview = summarizeWorkflowDiagnosticsReport(diagnosticsReport);
  const diagnosticsTitle = resolveWorkflowDiagnosticsReportTitle(diagnosticsReport);
  const contractWarningRows = renderWarningRows(contractWarnings);
  const summaryArtifactCount = listWorkflowSummaryArtifacts(run.result ?? null).length;
  const supportedNodeCount = graph?.nodes.filter((node) => isWorkflowNodeSupportedInRuntime(node)).length ?? 0;
  const fullSnapshotCount = snapshots.filter((entry) => entry.payloadState === "full").length;
  const summaryOnlySnapshotCount = snapshots.filter((entry) => entry.payloadState === "summary_only").length;
  const traceSummary = run.traceSummary;
  const progressTimelineRows = renderProgressTimelineRows(run);
  const staticContractHealth = formatWorkflowContractHealthSummary(contractWarnings);
  const dynamicReviewState = formatWorkflowDynamicReviewState({
    warnings: contractWarnings,
    recentRunStatus: run.status,
  });
  const headerHealthLabel = resolveWorkflowTraceHeaderHealthLabel(
    staticContractHealth,
    dynamicReviewState,
  );
  const branchRows =
    run.branchDecisions?.map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.node_id)}</td><td>${escapeHtml(entry.chosen_output)}</td><td>${renderToneBadge(entry.predicate_result ? "true" : "false", resolveWorkflowTraceBranchPredicateTone(entry.predicate_result))}</td></tr>`,
    ).join("") ?? "";
  const nodeRows =
    run.nodeRuns?.map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.node_id)}</td><td>${renderToneBadge(entry.status, resolveWorkflowTraceNodeRunTone(entry.status))}</td><td>${escapeHtml(entry.kind)}</td><td>${escapeHtml(entry.operator_id ?? "--")}</td><td>${escapeHtml(String(entry.consumed_artifacts?.length ?? 0))}</td><td>${escapeHtml(String(entry.produced_artifacts?.length ?? 0))}</td></tr>`,
    ).join("") ?? "";
  const lineageRows =
    run.artifactLineage?.map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.artifact_key)}</td><td>${escapeHtml(`${entry.node_id}.${entry.port_id}`)}</td><td>${renderToneBadge(resolveWorkflowTraceLineageSourceLabel(entry.source_artifacts), resolveWorkflowTraceLineageSourceTone(entry.source_artifacts))}<ul>${renderList(entry.source_artifacts)}</ul></td></tr>`,
    ).join("") ?? "";
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${escapeHtml(run.workflowId)} trace report</title>
  <style>
    :root { color-scheme: dark; }
    body { margin: 0; font: 14px/1.5 "IBM Plex Sans", "Segoe UI", sans-serif; background: #17191d; color: #e7edf5; }
    main { max-width: 1100px; margin: 0 auto; padding: 24px; display: grid; gap: 20px; }
    section { background: linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.22)); border: 1px solid #323844; border-radius: 14px; padding: 16px 18px; }
    h1, h2 { margin: 0 0 10px; }
    table { width: 100%; border-collapse: collapse; }
    th, td { text-align: left; padding: 8px 10px; border-top: 1px solid #323844; vertical-align: top; }
    th { width: 220px; color: #9fb1c7; font-weight: 600; }
    .meta th { width: 180px; }
    .pill { display: inline-block; padding: 4px 8px; border-radius: 999px; background: #243247; color: #9fd1ff; font-size: 12px; }
    .pill--good { background: rgba(46, 125, 50, 0.35); color: #9cf0a6; }
    .pill--watch { background: rgba(170, 120, 30, 0.35); color: #ffd38a; }
    .pill--risk { background: rgba(163, 45, 45, 0.35); color: #ffb0b0; }
    .good { color: #7ee08a; }
    .risk { color: #f49b9b; }
    ul { margin: 0; padding-left: 18px; }
  </style>
</head>
<body>
  <main>
    <section>
      <h1>${escapeHtml(run.workflowId)} audit report</h1>
      <p>${renderToneBadge(headerHealthLabel, resolveWorkflowTraceContractHealthTone(dynamicReviewState))}</p>
      <table class="meta"><tbody>${renderRows([
        ["job id", run.jobId],
        ["status", run.status],
        ["progress", `${Math.round(run.progress * 100)}%`],
        ["current node", run.currentNode ?? "--"],
        ["summary", run.summary ?? "--"],
        ["updated at", run.updatedAt ?? "--"],
        ["workflow version", workflow?.version ?? "--"],
        ["summary artifacts", String(summaryArtifactCount)],
        ["graph nodes", String(graph?.nodes.length ?? 0)],
        ["graph edges", String(graph?.edges?.length ?? 0)],
        ["runtime supported nodes", `${supportedNodeCount}/${graph?.nodes.length ?? 0}`],
        ["branch decisions", String(traceSummary?.branchDecisionCount ?? run.branchDecisions?.length ?? 0)],
        [
          "node runs (completed/skipped)",
          traceSummary
            ? `${traceSummary.completedNodeRunCount}/${traceSummary.skippedNodeRunCount}`
            : String(run.nodeRuns?.length ?? 0),
        ],
        ["progress events", String(traceSummary?.progressEventCount ?? 0)],
        ["latest phase", traceSummary?.latestProgressLabel ?? "--"],
        [
          "lineage (root/derived)",
          traceSummary
            ? `${traceSummary.rootArtifactCount}/${traceSummary.derivedArtifactCount}`
            : `${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length ?? 0}/${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0}`,
        ],
      ])}</tbody></table>
    </section>
    <section>
      <h2>Workflow contract</h2>
      <table class="meta"><tbody>${renderRows([
        ["workflow name", workflow?.name ?? run.workflowId],
        ["dataset contract id", graph?.dataset_contract?.id ?? "--"],
        ["dataset contract version", graph?.dataset_contract?.version ?? "--"],
        ["dataset values", String(graph?.dataset_contract?.values.length ?? 0)],
        ["entry inputs", String(workflow?.entry_inputs.length ?? 0)],
        ["output artifacts", String(workflow?.output_artifacts.length ?? 0)],
        ["static contract health", staticContractHealth],
        ["dynamic review state", dynamicReviewState],
      ])}</tbody></table>
      <table><thead><tr><th>node</th><th>#</th><th>export contract warning</th></tr></thead><tbody>${contractWarningRows || '<tr><td colspan="3">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Local lifecycle</h2>
      <table class="meta"><tbody>${renderRows([
        ["local workflow id", localWorkflow?.id ?? workflow?.local?.storage_id ?? "--"],
        ["source workflow", localWorkflow?.sourceWorkflowName ?? localWorkflow?.sourceWorkflowId ?? "--"],
        ["promoted at", localWorkflow?.promotedAt ?? workflow?.local?.promoted_at ?? "--"],
        ["variant of", localWorkflow?.variantOfWorkflowName ?? localWorkflow?.variantOfWorkflowId ?? "--"],
        ["local version", localWorkflow?.graph.version ?? workflow?.version ?? "--"],
        ["notes", localWorkflow?.notes ?? workflow?.local?.notes ?? "--"],
      ])}</tbody></table>
    </section>
    <section>
      <h2>Snapshot history</h2>
      <table class="meta"><tbody>${renderRows([
        ["snapshot count", String(snapshots.length)],
        ["full payload snapshots", String(fullSnapshotCount)],
        ["summary only snapshots", String(summaryOnlySnapshotCount)],
      ])}</tbody></table>
      <table><thead><tr><th>created</th><th>reason</th><th>payload</th><th>summary</th></tr></thead><tbody>${snapshotRows || '<tr><td colspan="4">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Session audit</h2>
      <table class="meta"><tbody>${renderRows([
        ["audit entries", String(securityAuditEntries.length)],
        ["latest audit timestamp", securityAuditEntries[securityAuditEntries.length - 1]?.at ?? "--"],
      ])}</tbody></table>
      <table><thead><tr><th>time</th><th>source</th><th>risk</th><th>status</th><th>action</th></tr></thead><tbody>${securityAuditRows || '<tr><td colspan="5">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Component integrity</h2>
      <table class="meta"><tbody>${renderRows([
        ["integrity issues", String(integrity.issues.length)],
        ["local storage linked", integrity.localWorkflowFound ? "yes" : "no"],
        ["snapshots indexed", String(integrity.snapshotCount)],
        ["summary-only snapshots", String(integrity.summaryOnlySnapshotCount)],
      ])}</tbody></table>
      <table><thead><tr><th>scope</th><th>severity</th><th>message</th><th>detail</th></tr></thead><tbody>${integrityRows || '<tr><td colspan="4">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Bridge runtime contracts</h2>
      <table><thead><tr><th>node</th><th>severity</th><th>artifact</th><th>message</th></tr></thead><tbody>${bridgeRuntimeRows || '<tr><td colspan="4">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Bridge runtime path inspection</h2>
      <table><thead><tr><th>bridge</th><th>upstream</th><th>downstream</th><th>field map</th><th>transform</th><th>source</th><th>target</th><th>artifacts</th></tr></thead><tbody>${bridgeRuntimeInspectionRows || '<tr><td colspan="8">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Bridge contract design vs runtime</h2>
      <table><thead><tr><th>bridge</th><th>design field map</th><th>design transform</th><th>seed model</th><th>runtime upstream</th><th>runtime downstream</th><th>source</th><th>target</th><th>status</th></tr></thead><tbody>${bridgeContractCompareRows || '<tr><td colspan="9">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>${escapeHtml(diagnosticsTitle)}</h2>
      <table class="meta"><tbody>${renderRows([
        ["report artifact", diagnosticsReport?.artifactKey ?? "--"],
        ["guard status", diagnosticsReport?.payload.report_guard_status ?? "--"],
        ["recommendation", diagnosticsReport?.payload.report_guard_recommendation ?? "--"],
        ["summary preview", diagnosticsPreview ?? "--"],
        ["highlight count", String(orderWorkflowDiagnosticsHighlights(diagnosticsReport).length)],
        ["focus metric count", String(orderWorkflowDiagnosticsFocusMetrics(diagnosticsReport).length)],
      ])}</tbody></table>
      <table><thead><tr><th>tone</th><th>highlight</th><th>value</th><th>id</th><th>context</th></tr></thead><tbody>${diagnosticsHighlightRows || '<tr><td colspan="5">--</td></tr>'}</tbody></table>
      <table><thead><tr><th>metric</th><th>value</th><th>context</th></tr></thead><tbody>${diagnosticsFocusRows || '<tr><td colspan="3">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Summary artifact contracts</h2>
      <table><thead><tr><th>artifact</th><th>type</th><th>summary kind</th><th>source operator</th><th>fields</th><th>node</th></tr></thead><tbody>${summaryArtifactRows || '<tr><td colspan="6">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Dataset values</h2>
      <table><thead><tr><th>value</th><th>class</th><th>element</th><th>semantic</th><th>encoding</th><th>axes</th></tr></thead><tbody>${datasetRows || '<tr><td colspan="6">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Runtime support</h2>
      <table><thead><tr><th>node</th><th>kind</th><th>operator</th><th>status</th></tr></thead><tbody>${runtimeRows || '<tr><td colspan="4">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Validation issues</h2>
      <table><thead><tr><th>level</th><th>message</th><th>locate</th><th>fix</th></tr></thead><tbody>${validationRows || '<tr><td colspan="4">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Branch trace</h2>
      <table><thead><tr><th>node</th><th>chosen output</th><th>predicate</th></tr></thead><tbody>${branchRows || '<tr><td colspan="3">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Progress timeline</h2>
      <table><thead><tr><th>stage</th><th>progress</th><th>kind</th><th>node</th><th>label</th><th>emitted</th></tr></thead><tbody>${progressTimelineRows || '<tr><td colspan="6">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Skipped nodes</h2>
      <ul>${renderList(run.skippedNodes)}</ul>
    </section>
    <section>
      <h2>Node runs</h2>
      <table><thead><tr><th>node</th><th>status</th><th>kind</th><th>operator</th><th>inputs</th><th>outputs</th></tr></thead><tbody>${nodeRows || '<tr><td colspan="6">--</td></tr>'}</tbody></table>
    </section>
    <section>
      <h2>Artifact lineage</h2>
      <table><thead><tr><th>artifact</th><th>producer</th><th>sources</th></tr></thead><tbody>${lineageRows || '<tr><td colspan="3">--</td></tr>'}</tbody></table>
    </section>
  </main>
</body>
</html>`;
}

export function buildWorkflowRunTraceReportHtml(run: WorkflowRunRecord) {
  return buildWorkflowRunAuditReportHtml({ run });
}
