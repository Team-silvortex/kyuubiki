"use client";

import type { JobEnvelope } from "@/lib/api/fem-shared";
import type { WorkflowGraphJobResult } from "@/lib/api/workflow-types";
import { resolveJobStatusDetailLabel } from "@/lib/api/job-status";
import type { WorkbenchStudyResult } from "@/lib/workbench/study-run-backend-service-core";
import type { WorkbenchStudyKind } from "@/lib/workbench/history";
import { summarizeWorkflowResultArtifacts } from "@/components/workbench/workflow/workbench-workflow-summary-contract";
import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";
import { resolveHistoryResultStudyKind } from "./workbench-history-result-kind";

type HistoryOpenEffects = {
  activeMaterial: string;
  copy: { historyAction: string; historyLoaded: string; workflowCatalogCompleted: string };
  setJob: (value: JobEnvelope["job"] | null) => void;
  setResult: (value: any) => void;
  setSidebarSection: (section: any) => void;
  setWorkflowPanelTab: (tab: any) => void;
  setSelectedWorkflowId: (value: string | null) => void;
  setWorkflowRuns: (value: any) => void;
  setMessage: (value: string) => void;
  recordHistory: (label: string) => void;
  commitObservation: () => void;
  openWorkspaceStudy: (tab: any) => void;
  detachSavedModel: () => void;
  setStudyKind: (value: WorkbenchStudyKind) => void;
  setAxialForm: (value: any) => void;
  setThermalBarModel: (value: any) => void;
  setHeatBarModel: (value: any) => void;
  setHeatPlaneModel: (value: any) => void;
  setPlaneResultField: (value: any) => void;
  setThermalBeamModel: (value: any) => void;
  setThermalTrussModel: (value: any) => void;
  setThermalTruss3dModel: (value: any) => void;
  setSpringModel: (value: any) => void;
  setSpring2dModel: (value: any) => void;
  setSpring3dModel: (value: any) => void;
  setBeamModel: (value: any) => void;
  setTorsionModel: (value: any) => void;
  setTrussModel: (value: any) => void;
  setTruss3dModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setThermalFrameModel: (value: any) => void;
  setPlaneModel: (value: any) => void;
};

export type HistoryJobResult = WorkbenchStudyResult | WorkflowGraphJobResult;

function isWorkflowGraphResult(value: unknown): value is WorkflowGraphJobResult {
  return typeof value === "object" && value !== null && "workflow_id" in value &&
    "completed_nodes" in value && "artifacts" in value;
}

function upsertWorkflowRunRecord(current: WorkflowRunRecord[], next: WorkflowRunRecord): WorkflowRunRecord[] {
  return [next, ...current.filter((entry) => entry.jobId !== next.jobId)].slice(0, 12);
}

export function applyHistoryJobPayload(
  payload: { job: JobEnvelope["job"]; result?: HistoryJobResult | null },
  effects: HistoryOpenEffects,
) {
  const { activeMaterial, copy, recordHistory, openWorkspaceStudy, setMessage } = effects;
  const workflowResult = isWorkflowGraphResult(payload.result) ? payload.result : null;
  const kind = payload.result && !workflowResult ? resolveHistoryResultStudyKind(payload.result) : null;
  // Finish validation and summary construction before any state mutation.
  const summary = workflowResult ? summarizeWorkflowResultArtifacts(workflowResult) : null;
  effects.commitObservation();

  if (workflowResult) {
    effects.setJob(payload.job);
    effects.setResult(null);
    effects.setSidebarSection("workflow");
    effects.setWorkflowPanelTab("runs");
    effects.setSelectedWorkflowId(workflowResult.workflow_id);
    effects.setWorkflowRuns((current: WorkflowRunRecord[]) => upsertWorkflowRunRecord(current, {
      jobId: payload.job.job_id, workflowId: workflowResult.workflow_id, status: payload.job.status,
      statusDetail: payload.job.status_detail ?? null, progress: payload.job.progress ?? 0,
      currentNode: workflowResult.current_node ?? payload.job.message ?? null, summary,
      updatedAt: payload.job.updated_at ?? null, skippedNodes: workflowResult.skipped_nodes ?? [],
      branchDecisions: workflowResult.branch_decisions ?? [], nodeRuns: workflowResult.node_runs ?? [],
      artifactLineage: workflowResult.artifact_lineage ?? [], result: workflowResult,
    }));
    setMessage(`${copy.workflowCatalogCompleted}: ${workflowResult.workflow_id}${summary ? ` (${summary})` : ""}`);
    return;
  }

  if (kind && payload.result) {
    const result = payload.result as WorkbenchStudyResult;
    const { project_id: _project, model_version_id: _version, ...input } = result.input as
      WorkbenchStudyResult["input"] & { project_id?: string; model_version_id?: string };
    recordHistory(copy.historyAction);
    effects.detachSavedModel();
    effects.setStudyKind(kind);
    if (kind === "axial_bar_1d" && "length" in input) {
      effects.setAxialForm({ length: input.length, area: input.area, elements: input.elements,
        tipForce: input.tip_force, material: activeMaterial, youngsModulusGpa: input.youngs_modulus / 1e9 });
    } else {
      const setters: Record<Exclude<WorkbenchStudyKind, "axial_bar_1d">, (value: any) => void> = {
        heat_bar_1d: effects.setHeatBarModel,
        heat_plane_triangle_2d: effects.setHeatPlaneModel, heat_plane_quad_2d: effects.setHeatPlaneModel,
        electrostatic_plane_triangle_2d: effects.setPlaneModel, electrostatic_plane_quad_2d: effects.setPlaneModel,
        thermal_plane_triangle_2d: effects.setPlaneModel, thermal_plane_quad_2d: effects.setPlaneModel,
        plane_triangle_2d: effects.setPlaneModel, plane_quad_2d: effects.setPlaneModel,
        thermal_bar_1d: effects.setThermalBarModel, thermal_beam_1d: effects.setThermalBeamModel,
        thermal_frame_2d: effects.setThermalFrameModel, thermal_truss_2d: effects.setThermalTrussModel,
        thermal_truss_3d: effects.setThermalTruss3dModel,
        spring_1d: effects.setSpringModel, spring_2d: effects.setSpring2dModel, spring_3d: effects.setSpring3dModel,
        beam_1d: effects.setBeamModel, torsion_1d: effects.setTorsionModel,
        truss_2d: effects.setTrussModel, truss_3d: effects.setTruss3dModel, frame_2d: effects.setFrameModel,
      };
      if (kind !== "axial_bar_1d") setters[kind](input);
    }
    if (kind.startsWith("heat_plane_")) effects.setPlaneResultField("average_temperature");
    else if (kind.startsWith("electrostatic_plane_")) effects.setPlaneResultField("electric_field_magnitude");
    else if (kind.includes("plane_")) effects.setPlaneResultField("von_mises");
    openWorkspaceStudy("controls");
  }
  effects.setJob(payload.job);
  effects.setResult(payload.result ?? null);
  setMessage(payload.job.status === "failed"
    ? payload.job.message ?? resolveJobStatusDetailLabel(payload.job.status_detail) ?? copy.historyLoaded
    : copy.historyLoaded);
}
