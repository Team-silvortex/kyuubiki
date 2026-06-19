import test from "node:test";
import assert from "node:assert/strict";

import { summarizeWorkflowResultArtifacts } from "../../src/components/workbench/workflow/workbench-workflow-summary-contract.ts";
import type { WorkflowGraphJobResult } from "../../src/lib/api/workflow-types.ts";

test("benchmark-style workflow previews prefer diagnostics report highlights", () => {
  const result: WorkflowGraphJobResult = {
    workflow_id: "workflow.synthetic.benchmark",
    completed_nodes: ["input.source", "extract.summary", "transform.normalize", "export.summary"],
    artifacts: {
      "artifact.normalized": {
        artifact_key: "artifact.normalized",
        artifact_type: "artifact/result_summary",
        node_id: "transform.normalize",
        port_id: "normalized",
        payload: {
          contract_version: "kyuubiki.workflow.summary_artifact@1",
          summary_kind: "normalized_summary",
          fields: {
            temperature_peak: 412.5,
            heat_flux_peak: 88.2,
          },
        },
      },
      "report.result": {
        artifact_key: "report.result",
        artifact_type: "artifact/json",
        node_id: "transform.normalize",
        port_id: "report",
        payload: {
          report_contract: "kyuubiki.workflow_report_payload/v1",
          report_kind: "diagnostics_bundle_report_payload",
          report_focus_metrics: {
            "thermal.temperature_max": 412.5,
            "thermal.flux_peak": 88.2,
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
          ],
        },
      },
    },
  };

  assert.equal(
    summarizeWorkflowResultArtifacts(result),
    "peak review: Thermal flux peak=8.820e+1, Thermal temperature peak=4.125e+2",
  );
});
