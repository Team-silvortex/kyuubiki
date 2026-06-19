import test from "node:test";
import assert from "node:assert/strict";

import { buildWorkflowRunAuditReportHtml } from "../../src/components/workbench/workflow/workbench-workflow-run-trace-report.ts";
import type { WorkflowRunRecord } from "../../src/components/workbench/workflow/workbench-workflow-types.ts";

function buildRun(): WorkflowRunRecord {
  return {
    jobId: "job-diagnostics-001",
    workflowId: "workflow.electrostatic-heat-thermo-diagnostics-markdown",
    status: "completed",
    progress: 1,
    currentNode: "report",
    summary: "Thermal temperature peak=1.250e+2, Thermo stress peak=2.200e+2",
    result: {
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
    },
  };
}

test("buildWorkflowRunAuditReportHtml includes diagnostics focus sections", () => {
  const html = buildWorkflowRunAuditReportHtml({ run: buildRun() });

  assert.match(html, /<h2>Peak diagnostics focus<\/h2>/);
  assert.match(html, /hold_and_review/);
  assert.match(html, /summary preview/);
  assert.match(html, /peak review: Thermo stress peak=2\.200e\+2, Thermal temperature peak=1\.250e\+2/);
  assert.match(html, /<span class="pill pill--risk">peak<\/span>/);
  assert.match(html, /<span class="pill pill--risk">peak<\/span><\/td><td>Thermal temperature peak<\/td>/);
  assert.match(html, /Thermal temperature peak/);
  assert.match(html, /Thermo stress peak/);
  assert.match(html, /thermal\.temperature_max/);
  assert.match(html, /thermo\.stress_peak/);
  assert.match(html, /<strong>sample<\/strong>/);
  assert.match(html, /source thermo · field thermo_stress_peak · anchor te1/);
  assert.match(html, /<strong>response<\/strong>/);
  assert.match(html, /stress: x=1\.400e\+1/);
});
