"use client";

import { useMemo, useState } from "react";
import { WorkbenchWorkflowSectionMount } from "@/components/workbench/workbench-workflow-section-mount";
import { buildWorkbenchWorkflowLabels } from "@/components/workbench/workbench-shell-copy-composition";
import { copyZhCore } from "@/components/workbench/workbench-copy-zh-core";
import type { WorkflowCatalogEntry, WorkflowGraphJobResult } from "@/lib/api";
import type { WorkflowRunRecord, WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";
import { summarizeWorkflowRunTrace } from "@/components/workbench/workflow/workbench-workflow-run-trace-summary";
import { WORKFLOW_SUMMARY_ARTIFACT_CONTRACT } from "@/lib/workbench/workflow-summary-contract";

const workflowId = "workflow.synthetic.benchmark";

function buildSyntheticWorkflow(): WorkflowCatalogEntry {
  return {
    id: workflowId,
    name: "Synthetic Benchmark Workflow",
    version: "1.6.0-bench",
    summary: "Synthetic workflow used by the standalone benchmark page.",
    domains: ["benchmark"],
    capability_tags: ["contract_health:clean", "benchmark"],
    graph: {
      schema_version: "kyuubiki.workflow-graph/v1",
      id: workflowId,
      name: "Synthetic Benchmark Workflow",
      version: "1.6.0-bench",
      entry_inputs: [{ node_id: "input.source", artifact_type: "study.result", description: "Synthetic result input" }],
      output_artifacts: [{ node_id: "export.summary", artifact_type: "report.html", description: "Synthetic export artifact" }],
      entry_nodes: ["input.source"],
      output_nodes: ["export.summary"],
      nodes: [
        { id: "input.source", kind: "input", outputs: [{ id: "out", artifact_type: "study.result", description: "result payload" }] },
        { id: "extract.summary", kind: "extract", operator_id: "extract.result_summary", inputs: [{ id: "source", artifact_type: "study.result", description: "raw result" }], outputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "summary payload" }] },
        { id: "transform.normalize", kind: "transform", operator_id: "transform.normalize_summary_fields", inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "summary" }], outputs: [{ id: "normalized", artifact_type: "artifact/result_summary", description: "normalized summary" }] },
        { id: "export.summary", kind: "export", operator_id: "export.summary_json", inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "normalized summary" }], outputs: [{ id: "file", artifact_type: "artifact/json", description: "export file" }] },
      ],
      edges: [
        { id: "edge.input.extract", from: { node: "input.source", port: "out" }, to: { node: "extract.summary", port: "source" }, artifact_type: "study.result" },
        { id: "edge.extract.transform", from: { node: "extract.summary", port: "summary" }, to: { node: "transform.normalize", port: "summary" }, artifact_type: "artifact/result_summary" },
        { id: "edge.transform.export", from: { node: "transform.normalize", port: "normalized" }, to: { node: "export.summary", port: "summary" }, artifact_type: "artifact/result_summary" },
      ],
    },
    entry_inputs: [{ node_id: "input.source", artifact_type: "study.result", description: "Synthetic result input" }],
    output_artifacts: [{ node_id: "export.summary", artifact_type: "artifact/json", description: "Synthetic export artifact" }],
    local: {
      storage_id: "synthetic-benchmark-local",
      source_workflow_id: workflowId,
      source_workflow_name: "Synthetic Benchmark Workflow",
      input_artifact_texts: {
        "input.source:study.result": JSON.stringify({ sample: true, magnitude: 42 }, null, 2),
      },
      promoted_at: new Date("2026-06-13T00:00:00Z").toISOString(),
      notes: "Local synthetic benchmark workflow.",
      tags: ["benchmark"],
    },
  };
}

function buildSyntheticRuns(): WorkflowRunRecord[] {
  const emittedAt = new Date("2026-06-13T00:00:00Z").toISOString();
  const result: WorkflowGraphJobResult = {
    workflow_id: workflowId,
    current_node: "export.summary",
    completed_nodes: ["input.source", "extract.summary", "transform.normalize", "export.summary"],
    skipped_nodes: ["condition.route"],
    progress_events: [
      { stage: "queued", progress: 0.12, message: "input accepted", node_id: "input.source", kind: "input", emitted_at: emittedAt },
      { stage: "preprocessing", progress: 0.44, message: "extract result summary", node_id: "extract.summary", kind: "extract", emitted_at: emittedAt },
      { stage: "solving", progress: 0.7, message: "normalize summary", node_id: "transform.normalize", kind: "transform", emitted_at: emittedAt },
      { stage: "postprocessing", progress: 0.92, message: "export summary", node_id: "export.summary", kind: "export", emitted_at: emittedAt },
      { stage: "completed", progress: 1, message: "workflow completed", kind: "workflow", emitted_at: emittedAt },
    ],
    branch_decisions: [{ node_id: "condition.route", chosen_output: "if_true", predicate_result: true }],
    node_runs: [
      { node_id: "input.source", kind: "input", status: "completed" as const, produced_artifacts: ["artifact.source"] },
      { node_id: "extract.summary", kind: "extract", operator_id: "extract.result_summary", status: "completed" as const, consumed_artifacts: ["artifact.source"], produced_artifacts: ["artifact.summary"] },
      { node_id: "transform.normalize", kind: "transform", operator_id: "transform.normalize_summary_fields", status: "completed" as const, consumed_artifacts: ["artifact.summary"], produced_artifacts: ["artifact.normalized"] },
      { node_id: "condition.route", kind: "condition", status: "skipped" as const, consumed_artifacts: ["artifact.normalized"] },
      { node_id: "export.summary", kind: "export", operator_id: "export.summary_json", status: "completed" as const, consumed_artifacts: ["artifact.normalized"], produced_artifacts: ["artifact.export"] },
    ],
    artifact_lineage: [
      { artifact_key: "artifact.source", node_id: "input.source", port_id: "out" },
      { artifact_key: "artifact.summary", node_id: "extract.summary", port_id: "summary", source_artifacts: ["artifact.source"] },
      { artifact_key: "artifact.normalized", node_id: "transform.normalize", port_id: "normalized", source_artifacts: ["artifact.summary"] },
      { artifact_key: "artifact.export", node_id: "export.summary", port_id: "file", source_artifacts: ["artifact.normalized"] },
    ],
    artifacts: {
      "artifact.summary": {
        artifact_key: "artifact.summary",
        artifact_type: "artifact/result_summary",
        node_id: "extract.summary",
        port_id: "summary",
        contract_version: `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
        payload: {
          contract_version: `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
          summary_kind: "result_summary",
          source_operator_id: "extract.result_summary",
          source_artifact_type: "study.result",
          field_namespace: "raw",
          fields: { max_temperature: 412.5, max_heat_flux: 88.2, max_displacement: 0.0042 },
          metadata: { source: "synthetic_benchmark", stage: "extract" },
        },
      },
      "artifact.normalized": {
        artifact_key: "artifact.normalized",
        artifact_type: "artifact/result_summary",
        node_id: "transform.normalize",
        port_id: "normalized",
        contract_version: `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
        payload: {
          contract_version: `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
          summary_kind: "normalized_summary",
          source_operator_id: "transform.normalize_summary_fields",
          source_artifact_type: "artifact/result_summary",
          field_namespace: "normalized",
          fields: { temperature_peak: 412.5, heat_flux_peak: 88.2, displacement_peak: 0.0042 },
          metadata: { source: "synthetic_benchmark", stage: "normalize", copy_unmapped: false },
        },
      },
      "artifact.export": {
        artifact_key: "artifact.export",
        artifact_type: "artifact/json",
        node_id: "export.summary",
        port_id: "file",
        content: JSON.stringify({
          contract_version: `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
          summary_kind: "normalized_summary",
          fields: { temperature_peak: 412.5, heat_flux_peak: 88.2, displacement_peak: 0.0042 },
        }),
      },
    },
  };
  return [{
    jobId: "bench-job-1",
    workflowId,
    status: "completed",
    progress: 1,
    currentNode: "export.summary",
    summary: "Synthetic workflow benchmark run",
    updatedAt: emittedAt,
    skippedNodes: result.skipped_nodes,
    branchDecisions: result.branch_decisions,
    nodeRuns: result.node_runs,
    artifactLineage: result.artifact_lineage,
    result,
    traceSummary: summarizeWorkflowRunTrace(result),
  }];
}

export default function WorkflowBenchmarkPage() {
  const labels = useMemo(() => buildWorkbenchWorkflowLabels(copyZhCore), []);
  const workflowCatalogEntries = useMemo(() => [buildSyntheticWorkflow()], []);
  const [surfaceTab, setSurfaceTab] = useState<WorkflowSurfaceTab>("overview");
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(workflowId);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunRecord[]>(buildSyntheticRuns());
  const selectedWorkflow = workflowCatalogEntries.find((entry) => entry.id === selectedWorkflowId) ?? workflowCatalogEntries[0] ?? null;

  return (
    <main style={{ minHeight: "100vh", padding: "16px", background: "#17191d" }}>
      <WorkbenchWorkflowSectionMount
        surfaceTab={surfaceTab}
        onSurfaceTabChange={setSurfaceTab}
        labels={labels}
        workflowCatalogEntries={workflowCatalogEntries}
        workflowOperatorDescriptors={[]}
        workflowCatalogBusy={false}
        selectedWorkflowId={selectedWorkflowId}
        selectedWorkflow={selectedWorkflow}
        currentStudyKind={"plane_triangle_2d" as never}
        currentHeatPlaneModel={{} as never}
        currentPlaneModel={{} as never}
        latestJob={null}
        latestWorkflowSummary={workflowRuns[0]?.summary ?? null}
        workflowRuns={workflowRuns}
        refreshWorkflowCatalog={async () => {}}
        setSelectedWorkflowId={setSelectedWorkflowId}
        setWorkflowRuns={setWorkflowRuns}
        runWorkflowCatalogEntry={() => {}}
        runWorkflowDraft={() => {}}
        openHistoryJob={() => {}}
      />
    </main>
  );
}
