import test from "node:test";
import assert from "node:assert/strict";

import { applyHistoryJobPayload } from "../../src/components/workbench/workbench-history-result.ts";
import type { WorkflowRunRecord } from "../../src/components/workbench/workflow/workbench-workflow-types.ts";
import type { WorkflowGraphJobResult } from "../../src/lib/api/workflow-types.ts";

function buildWorkflowResult(): WorkflowGraphJobResult {
  return {
    workflow_id: "workflow.electrostatic-heat-thermo-diagnostics-markdown",
    current_node: "report",
    completed_nodes: ["bundle", "guard", "report"],
    skipped_nodes: [],
    branch_decisions: [],
    node_runs: [],
    artifact_lineage: [],
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

test("applyHistoryJobPayload opens workflow history entries with diagnostics highlight summaries", () => {
  let sidebarSection: unknown = null;
  let workflowPanelTab: unknown = null;
  let selectedWorkflowId: string | null = null;
  let message = "";
  let resultValue: unknown = "unchanged";
  let jobValue: unknown = null;
  let workflowRuns: WorkflowRunRecord[] = [];

  applyHistoryJobPayload(
    {
      job: {
        job_id: "job-history-001",
        project_id: "project-demo",
        simulation_case_id: "case-demo",
        status: "completed",
        progress: 1,
        updated_at: "2026-06-19T00:00:00Z",
      },
      result: buildWorkflowResult(),
    } as never,
    {
      activeMaterial: "steel",
      copy: {
        historyAction: "history action",
        historyLoaded: "history loaded",
        workflowCatalogCompleted: "workflow completed",
      },
      setJob: (value) => {
        jobValue = value;
      },
      setResult: (value) => {
        resultValue = value;
      },
      setSidebarSection: (value) => {
        sidebarSection = value;
      },
      setWorkflowPanelTab: (value) => {
        workflowPanelTab = value;
      },
      setSelectedWorkflowId: (value) => {
        selectedWorkflowId = value;
      },
      setWorkflowRuns: (updater) => {
        workflowRuns = typeof updater === "function" ? updater(workflowRuns) : updater;
      },
      setMessage: (value) => {
        message = value;
      },
      recordHistory: () => {},
      openWorkspaceStudy: () => {},
      setStudyKind: () => {},
      setAxialForm: () => {},
      setThermalBarModel: () => {},
      setHeatBarModel: () => {},
      setHeatPlaneModel: () => {},
      setPlaneResultField: () => {},
      setThermalBeamModel: () => {},
      setThermalTrussModel: () => {},
      setThermalTruss3dModel: () => {},
      setSpringModel: () => {},
      setSpring2dModel: () => {},
      setSpring3dModel: () => {},
      setBeamModel: () => {},
      setTorsionModel: () => {},
      setTrussModel: () => {},
      setTruss3dModel: () => {},
      setFrameModel: () => {},
      setThermalFrameModel: () => {},
      setPlaneModel: () => {},
    },
  );

  assert.equal(sidebarSection, "workflow");
  assert.equal(workflowPanelTab, "runs");
  assert.equal(
    selectedWorkflowId,
    "workflow.electrostatic-heat-thermo-diagnostics-markdown",
  );
  assert.equal(resultValue, null);
  assert.ok(jobValue);
  assert.equal(
    workflowRuns[0]?.summary,
    "peak review: Thermo stress peak=2.200e+2, Thermal temperature peak=1.250e+2",
  );
  assert.equal(
    message,
    "workflow completed: workflow.electrostatic-heat-thermo-diagnostics-markdown (peak review: Thermo stress peak=2.200e+2, Thermal temperature peak=1.250e+2)",
  );
});
