"use client";

import { useMemo, useState } from "react";
import { WorkbenchWorkflowSectionMount } from "@/components/workbench/workbench-workflow-section-mount";
import { buildWorkbenchWorkflowLabels } from "@/components/workbench/workbench-shell-copy-composition";
import { copyZhCore } from "@/components/workbench/workbench-copy-zh-core";
import type {
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { WorkflowRunRecord, WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";
import { summarizeWorkflowRunTrace } from "@/components/workbench/workflow/workbench-workflow-run-trace-summary";
import { WORKFLOW_SUMMARY_ARTIFACT_CONTRACT } from "@/lib/workbench/workflow-summary-contract";

const primaryWorkflowId = "workflow.synthetic.benchmark";

function buildSyntheticOperatorDescriptors(): WorkflowOperatorDescriptor[] {
  return [
    {
      id: "transform.normalize_summary_fields",
      version: "1.8.0-bench",
      domain: "thermal",
      family: "summary_pipeline",
      kind: "transform",
      summary: "Normalize thermal result summaries into a stable comparison contract.",
      capability_tags: ["benchmark", "summary", "thermal", "normalize"],
      origin: "built_in",
      input_schema: { schema: "artifact/result_summary", version: "1.0.0" },
      output_schema: { schema: "artifact/result_summary", version: "1.0.0" },
      config_schema: { schema: "config/normalize_summary", version: "1.0.0" },
      config_example: { field_namespace: "normalized", copy_unmapped: false },
      inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "result summary payload" }],
      outputs: [{ id: "normalized", artifact_type: "artifact/result_summary", description: "normalized summary payload" }],
      validation: { baseline_status: "verified", baseline_cases: ["summary.normalize.smoke"], smoke_paths: ["/workflow-benchmark"] },
    },
    {
      id: "transform.aggregate_hotspots",
      version: "1.8.0-bench",
      domain: "thermal",
      family: "summary_pipeline",
      kind: "transform",
      summary: "Aggregate hotspot extrema into a compact thermal hotspot report.",
      capability_tags: ["benchmark", "hotspot", "thermal", "aggregate"],
      origin: "built_in",
      input_schema: { schema: "artifact/result_summary", version: "1.0.0" },
      output_schema: { schema: "artifact/result_summary", version: "1.0.0" },
      config_schema: { schema: "config/aggregate_hotspots", version: "1.0.0" },
      config_example: { reduction: "max", include_locations: true },
      inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "normalized result summary" }],
      outputs: [{ id: "hotspots", artifact_type: "artifact/result_summary", description: "hotspot summary payload" }],
      validation: { baseline_status: "partial", baseline_cases: ["summary.hotspot.smoke"], smoke_paths: ["/workflow-benchmark"] },
    },
    {
      id: "transform.bridge_metric_table",
      version: "1.8.0-bench",
      domain: "mechanical",
      family: "bridge_runtime",
      kind: "transform",
      summary: "Map bridge metrics into a contract-ready table for downstream export.",
      capability_tags: ["benchmark", "bridge", "mechanical", "table"],
      origin: "built_in",
      input_schema: { schema: "artifact/result_summary", version: "1.0.0" },
      output_schema: { schema: "artifact/result_summary", version: "1.0.0" },
      config_schema: { schema: "config/bridge_metric_table", version: "1.0.0" },
      config_example: { columns: ["temperature_peak", "stress_peak", "displacement_peak"] },
      inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "summary payload" }],
      outputs: [{ id: "table", artifact_type: "artifact/result_summary", description: "bridge metric table" }],
      validation: { baseline_status: "unverified", baseline_cases: ["bridge.metric.table.sample"], smoke_paths: ["/workflow-benchmark"] },
    },
  ];
}

function buildSyntheticGraph(workflowId: string, name: string): WorkflowGraphDefinition {
  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: workflowId,
    name,
    version: "1.8.0-bench",
    dataset_contract: {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: `${workflowId}.dataset`,
      version: "1.0.0",
      name: `${name} Dataset Contract`,
      values: [
        {
          id: "thermo_result",
          data_class: "field",
          element_type: "float64",
          shape: { axes: [{ id: "node", label: "Node", size: 24, semantic: "mesh_node" }] },
          semantic_type: "study.result",
          unit: "mixed",
          encoding: "json",
          schema_ref: { schema: "study.result", version: "1.0.0" },
        },
        {
          id: "result_summary",
          data_class: "summary",
          element_type: "float64",
          shape: { axes: [{ id: "metric", label: "Metric", size: 3, semantic: "summary_metric" }] },
          semantic_type: "artifact/result_summary",
          unit: "mixed",
          encoding: "json",
          schema_ref: { schema: "artifact/result_summary", version: "1.0.0" },
        },
        {
          id: "json_export",
          data_class: "file",
          element_type: "utf8",
          shape: { axes: [{ id: "document", label: "Document", size: 1, semantic: "export_file" }] },
          semantic_type: "artifact/json",
          unit: "n/a",
          encoding: "utf8",
          schema_ref: { schema: "artifact/json", version: "1.0.0" },
        },
      ],
      metadata: {
        source: "synthetic_benchmark",
        contract_scope: "workflow_layout_guard",
      },
    },
    entry_inputs: [{ node_id: "input.source", artifact_type: "study.result", description: "Synthetic result input" }],
    output_artifacts: [{ node_id: "export.summary", artifact_type: "report.html", description: "Synthetic export artifact" }],
    entry_nodes: ["input.source"],
    output_nodes: ["export.summary"],
    nodes: [
      { id: "input.source", kind: "input", outputs: [{ id: "out", artifact_type: "study.result", description: "result payload", dataset_value: "thermo_result" }] },
      { id: "extract.summary", kind: "extract", operator_id: "extract.result_summary", inputs: [{ id: "source", artifact_type: "study.result", description: "raw result", dataset_value: "thermo_result" }], outputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "summary payload", dataset_value: "result_summary" }] },
      { id: "transform.normalize", kind: "transform", operator_id: "transform.normalize_summary_fields", inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "summary", dataset_value: "result_summary" }], outputs: [{ id: "normalized", artifact_type: "artifact/result_summary", description: "normalized summary", dataset_value: "result_summary" }] },
      { id: "export.summary", kind: "export", operator_id: "export.summary_json", inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "normalized summary", dataset_value: "result_summary" }], outputs: [{ id: "file", artifact_type: "artifact/json", description: "export file", dataset_value: "json_export" }] },
    ],
    edges: [
      { id: "edge.input.extract", from: { node: "input.source", port: "out" }, to: { node: "extract.summary", port: "source" }, artifact_type: "study.result", dataset_value: "thermo_result" },
      { id: "edge.extract.transform", from: { node: "extract.summary", port: "summary" }, to: { node: "transform.normalize", port: "summary" }, artifact_type: "artifact/result_summary", dataset_value: "result_summary" },
      { id: "edge.transform.export", from: { node: "transform.normalize", port: "normalized" }, to: { node: "export.summary", port: "summary" }, artifact_type: "artifact/result_summary", dataset_value: "result_summary" },
    ],
  };
}

function buildSyntheticWorkflow(config: {
  id: string;
  name: string;
  summary: string;
  domains: string[];
  capabilityTags: string[];
  notes: string;
  tags: string[];
}) {
  return {
    id: config.id,
    name: config.name,
    version: "1.8.0-bench",
    summary: config.summary,
    domains: config.domains,
    capability_tags: config.capabilityTags,
    graph: buildSyntheticGraph(config.id, config.name),
    entry_inputs: [{ node_id: "input.source", artifact_type: "study.result", description: "Synthetic result input" }],
    output_artifacts: [{ node_id: "export.summary", artifact_type: "artifact/json", description: "Synthetic export artifact" }],
    local: {
      storage_id: `${config.id}-local`,
      source_workflow_id: config.id,
      source_workflow_name: config.name,
      input_artifact_texts: {
        "input.source:study.result": JSON.stringify({ sample: true, magnitude: 42 }, null, 2),
      },
      promoted_at: new Date("2026-06-13T00:00:00Z").toISOString(),
      notes: config.notes,
      tags: config.tags,
    },
  };
}

function buildSyntheticRuns(): WorkflowRunRecord[] {
  const emittedAt = new Date("2026-06-13T00:00:00Z").toISOString();
  const result: WorkflowGraphJobResult = {
    workflow_id: primaryWorkflowId,
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
      { artifact_key: "report.result", node_id: "transform.normalize", port_id: "report", source_artifacts: ["artifact.normalized"] },
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
      "report.result": {
        artifact_key: "report.result",
        artifact_type: "artifact/json",
        node_id: "transform.normalize",
        port_id: "report",
        payload: {
          report_contract: "kyuubiki.workflow_report_payload/v1",
          report_kind: "diagnostics_bundle_report_payload",
          report_guard_status: "warn",
          report_guard_recommendation: "review_before_continue",
          report_focus_metrics: {
            "thermal.temperature_max": 412.5,
            "thermal.flux_peak": 88.2,
            "thermo.displacement_peak": 0.0042,
          },
          report_highlights: [
            {
              id: "thermal.temperature_max",
              label: "Thermal temperature peak",
              value: 412.5,
              attention: true,
            },
            {
              id: "thermal.flux_peak",
              label: "Thermal flux peak",
              value: 88.2,
              attention: false,
            },
            {
              id: "thermo.displacement_peak",
              label: "Thermo displacement peak",
              value: 0.0042,
              attention: false,
            },
          ],
        },
      },
    },
  };
  return [{
    jobId: "bench-job-1",
    workflowId: primaryWorkflowId,
    status: "completed",
    progress: 1,
    currentNode: "export.summary",
    summary: "Thermal temperature peak=4.125e+2, Thermal flux peak=8.820e+1",
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
  const workflowCatalogEntries = useMemo(
    () => [
      buildSyntheticWorkflow({
        id: primaryWorkflowId,
        name: "Synthetic Benchmark Workflow",
        summary: "Synthetic workflow used by the standalone benchmark page.",
        domains: ["benchmark"],
        capabilityTags: ["contract_health:clean", "benchmark", "thermal", "summary"],
        notes: "Local synthetic benchmark workflow.",
        tags: ["benchmark", "thermal"],
      }),
      buildSyntheticWorkflow({
        id: "workflow.synthetic.bridge-thermal-export",
        name: "Bridge Thermal Export Chain",
        summary: "Bridge-focused thermal summary chain with contract-safe export steps.",
        domains: ["thermal", "mechanical"],
        capabilityTags: ["contract_health:manageable", "bridge", "thermal", "export"],
        notes: "Synthetic bridge thermal export variant.",
        tags: ["bridge", "thermal", "export"],
      }),
      buildSyntheticWorkflow({
        id: "workflow.synthetic.mesh-audit-pack",
        name: "Mesh Audit Pack",
        summary: "Mesh and result auditing chain for benchmarked report generation.",
        domains: ["mechanical"],
        capabilityTags: ["contract_health:review", "mesh", "audit", "report"],
        notes: "Synthetic mesh audit variant.",
        tags: ["mesh", "audit", "report"],
      }),
    ],
    [],
  );
  const workflowOperatorDescriptors = useMemo(() => buildSyntheticOperatorDescriptors(), []);
  const [surfaceTab, setSurfaceTab] = useState<WorkflowSurfaceTab>("overview");
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(primaryWorkflowId);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunRecord[]>(buildSyntheticRuns());
  const selectedWorkflow = workflowCatalogEntries.find((entry) => entry.id === selectedWorkflowId) ?? workflowCatalogEntries[0] ?? null;

  return (
    <main style={{ minHeight: "100vh", padding: "16px", background: "#17191d" }}>
      <WorkbenchWorkflowSectionMount
        surfaceTab={surfaceTab}
        onSurfaceTabChange={setSurfaceTab}
        labels={labels}
        workflowCatalogEntries={workflowCatalogEntries}
        workflowOperatorDescriptors={workflowOperatorDescriptors}
        workflowCatalogBusy={false}
        selectedWorkflowId={selectedWorkflowId}
        selectedWorkflow={selectedWorkflow}
        currentStudyKind={"plane_triangle_2d" as never}
        currentHeatPlaneModel={{} as never}
        currentPlaneModel={{} as never}
        latestJob={null}
        latestWorkflowSummary={workflowRuns[0]?.summary ?? null}
        workflowRuns={workflowRuns}
        protocolAgents={[]}
        frontendRuntimeMode="orchestrated_gui"
        refreshWorkflowCatalog={async () => {}}
        setSelectedWorkflowId={setSelectedWorkflowId}
        setWorkflowRuns={setWorkflowRuns}
        runWorkflowCatalogEntry={() => {}}
        runWorkflowDraft={() => {}}
        openHistoryJob={() => {}}
        setSystemAlerts={() => {}}
      />
    </main>
  );
}
