import test from "node:test";
import assert from "node:assert/strict";

import {
  listWorkflowDiagnosticsReports,
} from "../../src/components/workbench/workflow/workbench-workflow-diagnostics-report-contract.ts";
import {
  summarizeWorkflowResultArtifacts,
} from "../../src/components/workbench/workflow/workbench-workflow-summary-contract.ts";
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
});

test("summarizeWorkflowResultArtifacts prefers diagnostics report highlights", () => {
  const summary = summarizeWorkflowResultArtifacts(buildDiagnosticsReportResult());

  assert.equal(
    summary,
    "Thermal temperature peak=1.250e+2, Thermo stress peak=2.200e+2",
  );
});
