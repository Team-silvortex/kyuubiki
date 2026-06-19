import test from "node:test";
import assert from "node:assert/strict";

import {
  listWorkflowDiagnosticsReports,
} from "../../src/components/workbench/workflow/workbench-workflow-diagnostics-report-contract.ts";
import {
  summarizeWorkflowResultArtifacts,
} from "../../src/components/workbench/workflow/workbench-workflow-summary-contract.ts";
import {
  buildWorkflowDiagnosticsFocusCardSummary,
  orderWorkflowDiagnosticsFocusMetrics,
  orderWorkflowDiagnosticsHighlights,
  resolveWorkflowDiagnosticsReportTitle,
  summarizeWorkflowDiagnosticsFocusContext,
  summarizeWorkflowDiagnosticsReport,
} from "../../src/components/workbench/workflow/workbench-workflow-diagnostics-presentation.ts";
import type { WorkflowGraphJobResult } from "../../src/lib/api/workflow-types.ts";

function buildDiagnosticsReportResult(): WorkflowGraphJobResult {
  return {
    workflow_id: "workflow.electrostatic-heat-thermo-diagnostics-markdown",
    completed_nodes: ["bundle", "guard", "report"],
    artifacts: {
      "report.result": {
        artifact_type: "artifact/json",
        node_id: "report",
        port_id: "result",
        payload: {
          report_contract: "kyuubiki.workflow_report_payload/v1",
          report_kind: "diagnostics_bundle_report_payload",
          report_guard_status: "block",
          report_guard_recommendation: "hold_and_review",
          report_focus_metrics: {
            "thermal.temperature_max": 125.0,
            "thermo.stress_peak": 220.0,
          },
          report_focus_context: {
            "thermo.stress_peak": {
              source: "thermo",
              value_field: "thermo_stress_peak",
              peak_element_id: "te1",
              peak_stress_x: 14.0,
            },
          },
          report_highlights: [
            {
              id: "thermal.temperature_max",
              label: "Thermal temperature peak",
              value: 125.0,
              attention: true,
            },
            {
              id: "thermo.stress_peak",
              label: "Thermo stress peak",
              value: 220.0,
              attention: true,
            },
          ],
        },
      },
    },
  };
}

test("listWorkflowDiagnosticsReports resolves standardized diagnostics report artifacts", () => {
  const reports = listWorkflowDiagnosticsReports(buildDiagnosticsReportResult());

  assert.equal(reports.length, 1);
  assert.equal(reports[0]?.artifactKey, "report.result");
  assert.equal(reports[0]?.payload.report_guard_status, "block");
  assert.equal(reports[0]?.payload.report_highlights[0]?.label, "Thermal temperature peak");
  assert.equal(reports[0]?.payload.report_focus_context?.["thermo.stress_peak"]?.source, "thermo");
});

test("summarizeWorkflowResultArtifacts prefers diagnostics report highlights", () => {
  const summary = summarizeWorkflowResultArtifacts(buildDiagnosticsReportResult());

  assert.equal(
    summary,
    "peak review: Thermo stress peak=2.200e+2, Thermal temperature peak=1.250e+2",
  );
});

test("peak-like diagnostics reports get peak-focused titles and previews", () => {
  const report = listWorkflowDiagnosticsReports(buildDiagnosticsReportResult())[0];

  assert.equal(resolveWorkflowDiagnosticsReportTitle(report), "Peak diagnostics focus");
  assert.equal(
    summarizeWorkflowDiagnosticsReport(report),
    "peak review: Thermo stress peak=2.200e+2, Thermal temperature peak=1.250e+2",
  );
});

test("peak-like diagnostics reports sort highlights and focus metrics by peak priority", () => {
  const report = listWorkflowDiagnosticsReports({
    ...buildDiagnosticsReportResult(),
    artifacts: {
      "report.result": {
        artifact_type: "artifact/json",
        node_id: "report",
        port_id: "result",
        payload: {
          report_contract: "kyuubiki.workflow_report_payload/v1",
          report_kind: "diagnostics_bundle_report_payload",
          report_focus_metrics: {
            "thermo.stress_peak": 220.0,
            "electrostatic.field_peak": 8.4,
            "thermal.flux_peak": 88.2,
          },
          report_highlights: [
            { id: "thermo.stress_peak", label: "Thermo stress peak", value: 220.0, attention: true },
            { id: "thermal.flux_peak", label: "Thermal flux peak", value: 88.2, attention: false },
            { id: "electrostatic.field_peak", label: "Electrostatic field peak", value: 8.4, attention: true },
          ],
        },
      },
    },
  })[0];

  assert.deepEqual(orderWorkflowDiagnosticsHighlights(report).map((entry) => entry.id), [
    "electrostatic.field_peak",
    "thermal.flux_peak",
    "thermo.stress_peak",
  ]);
  assert.deepEqual(orderWorkflowDiagnosticsFocusMetrics(report).map(([key]) => key), [
    "electrostatic.field_peak",
    "thermal.flux_peak",
    "thermo.stress_peak",
  ]);
});

test("focus context summaries group anchor, vectors, and companion values", () => {
  const report = listWorkflowDiagnosticsReports(buildDiagnosticsReportResult())[0];
  const lines = summarizeWorkflowDiagnosticsFocusContext(
    report?.payload.report_focus_context?.["thermo.stress_peak"],
  );

  assert.deepEqual(lines, [
    "source thermo · field thermo_stress_peak · anchor te1",
    "stress: x=1.400e+1",
  ]);
});

test("focus card summaries resolve thermo-mechanical domain framing", () => {
  const report = listWorkflowDiagnosticsReports(buildDiagnosticsReportResult())[0];
  const summary = buildWorkflowDiagnosticsFocusCardSummary(
    "thermo.stress_peak",
    report?.payload.report_focus_context?.["thermo.stress_peak"],
  );

  assert.equal(summary.domain, "thermo");
  assert.equal(summary.domainLabel, "thermo-mechanical");
  assert.equal(summary.anchorLine, "source thermo · field thermo_stress_peak · anchor te1");
  assert.deepEqual(summary.vectorLines, ["stress: x=1.400e+1"]);
  assert.deepEqual(summary.sections, [
    { label: "sample", lines: ["source thermo · field thermo_stress_peak · anchor te1"] },
    { label: "response", lines: ["stress: x=1.400e+1"] },
  ]);
});
