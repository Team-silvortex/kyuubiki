"use client";

import { validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { buildWorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { findStoredLocalWorkflow } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { isWorkflowNodeSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import { listStoredWorkflowSnapshots } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
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

function renderSnapshotRows(workflowId: string) {
  return listStoredWorkflowSnapshots(workflowId)
    .slice(0, 6)
    .map(
      (snapshot) =>
        `<tr><td>${escapeHtml(snapshot.createdAt)}</td><td>${escapeHtml(snapshot.reason)}</td><td>${escapeHtml(snapshot.payloadState)}</td><td><ul>${renderList(snapshot.summary)}</ul></td></tr>`,
    )
    .join("");
}

function renderSecurityAuditRows() {
  return readSecurityAuditLog()
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

export function buildWorkflowRunAuditReportHtml({
  run,
  workflow,
  operatorDescriptors,
}: WorkflowRunAuditReportOptions) {
  const graph = workflow?.graph;
  const integrity = buildWorkflowIntegrityReport(workflow, operatorDescriptors ?? []);
  const localWorkflow =
    workflow?.local?.storage_id ? findStoredLocalWorkflow(workflow.local.storage_id) : null;
  const snapshotRows = renderSnapshotRows(run.workflowId);
  const securityAuditRows = renderSecurityAuditRows();
  const integrityRows = renderIntegrityRows(workflow, operatorDescriptors);
  const validationRows = renderIssueRows(workflow, operatorDescriptors);
  const runtimeRows = renderRuntimeRows(workflow);
  const datasetRows = renderDatasetValueRows(workflow);
  const supportedNodeCount = graph?.nodes.filter((node) => isWorkflowNodeSupportedInRuntime(node)).length ?? 0;
  const snapshots = listStoredWorkflowSnapshots(run.workflowId);
  const fullSnapshotCount = snapshots.filter((entry) => entry.payloadState === "full").length;
  const summaryOnlySnapshotCount = snapshots.filter((entry) => entry.payloadState === "summary_only").length;
  const securityAuditEntries = readSecurityAuditLog();
  const branchRows =
    run.branchDecisions?.map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.node_id)}</td><td>${escapeHtml(entry.chosen_output)}</td><td>${entry.predicate_result ? "true" : "false"}</td></tr>`,
    ).join("") ?? "";
  const nodeRows =
    run.nodeRuns?.map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.node_id)}</td><td>${escapeHtml(entry.status)}</td><td>${escapeHtml(entry.kind)}</td><td>${escapeHtml(entry.operator_id ?? "--")}</td><td>${escapeHtml(String(entry.consumed_artifacts?.length ?? 0))}</td><td>${escapeHtml(String(entry.produced_artifacts?.length ?? 0))}</td></tr>`,
    ).join("") ?? "";
  const lineageRows =
    run.artifactLineage?.map(
      (entry) =>
        `<tr><td>${escapeHtml(entry.artifact_key)}</td><td>${escapeHtml(`${entry.node_id}.${entry.port_id}`)}</td><td><ul>${renderList(entry.source_artifacts)}</ul></td></tr>`,
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
    .good { color: #7ee08a; }
    .risk { color: #f49b9b; }
    ul { margin: 0; padding-left: 18px; }
  </style>
</head>
<body>
  <main>
    <section>
      <h1>${escapeHtml(run.workflowId)} audit report</h1>
      <table class="meta"><tbody>${renderRows([
        ["job id", run.jobId],
        ["status", run.status],
        ["progress", `${Math.round(run.progress * 100)}%`],
        ["current node", run.currentNode ?? "--"],
        ["summary", run.summary ?? "--"],
        ["updated at", run.updatedAt ?? "--"],
        ["workflow version", workflow?.version ?? "--"],
        ["graph nodes", String(graph?.nodes.length ?? 0)],
        ["graph edges", String(graph?.edges?.length ?? 0)],
        ["runtime supported nodes", `${supportedNodeCount}/${graph?.nodes.length ?? 0}`],
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
      ])}</tbody></table>
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
