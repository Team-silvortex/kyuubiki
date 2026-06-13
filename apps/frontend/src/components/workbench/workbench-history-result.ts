"use client";

import type {
  AxialBarResult,
  Beam1dResult,
  Frame2dResult,
  HeatBar1dResult,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dResult,
  JobEnvelope,
  PlaneQuad2dResult,
  PlaneTriangle2dResult,
  ThermalBar1dResult,
  ThermalBeam1dResult,
  ThermalFrame2dResult,
  ThermalPlaneQuad2dResult,
  ThermalPlaneTriangle2dResult,
  ThermalTruss2dResult,
  ThermalTruss3dResult,
  Truss2dResult,
  Truss3dResult,
  WorkflowGraphJobResult,
  Spring1dResult,
  Spring2dResult,
  Spring3dResult,
  Torsion1dResult,
} from "@/lib/api";
import { createMaterialDefinition } from "@/lib/materials";
import {
  ensureFrameModelMaterials,
  ensurePlaneModelMaterials,
  ensureTruss3dModelMaterials,
  ensureTrussModelMaterials,
} from "@/lib/workbench/material-commands";
import {
  isWorkflowGraphResult,
  summarizeWorkflowArtifacts,
  upsertWorkflowRunRecord,
} from "@/components/workbench/workflow/workbench-workflow-controller";
import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";

type HistoryResultCopy = {
  historyAction: string;
  historyLoaded: string;
  workflowCatalogCompleted: string;
};

type HistoryOpenEffects = {
  activeMaterial: string;
  copy: HistoryResultCopy;
  setJob: (value: JobEnvelope["job"] | null) => void;
  setResult: (value: any) => void;
  setSidebarSection: (section: any) => void;
  setWorkflowPanelTab: (tab: any) => void;
  setSelectedWorkflowId: (value: string | null) => void;
  setWorkflowRuns: (value: any) => void;
  setMessage: (value: string) => void;
  recordHistory: (label: string) => void;
  openWorkspaceStudy: (tab: any) => void;
  setStudyKind: (value: any) => void;
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

type HistoryJobResult =
  | AxialBarResult
  | HeatBar1dResult
  | HeatPlaneTriangle2dResult
  | HeatPlaneQuad2dResult
  | ThermalBar1dResult
  | ThermalBeam1dResult
  | ThermalTruss2dResult
  | ThermalTruss3dResult
  | Spring1dResult
  | Spring2dResult
  | Spring3dResult
  | Beam1dResult
  | Torsion1dResult
  | Truss2dResult
  | Truss3dResult
  | PlaneTriangle2dResult
  | PlaneQuad2dResult
  | Frame2dResult
  | ThermalFrame2dResult
  | ThermalPlaneTriangle2dResult
  | ThermalPlaneQuad2dResult
  | WorkflowGraphJobResult;

function isAxialResult(value: unknown): value is AxialBarResult {
  return typeof value === "object" && value !== null && "displacements" in value && "strains" in value && "input" in value;
}

function isThermalBar1dResult(value: unknown): value is ThermalBar1dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "axial_force" in value && "input" in value;
}

function isHeatBar1dResult(value: unknown): value is HeatBar1dResult {
  return typeof value === "object" && value !== null && "max_temperature" in value && "heat_flux" in value && "input" in value;
}

function isHeatPlaneTriangle2dResult(value: unknown): value is HeatPlaneTriangle2dResult {
  return typeof value === "object" && value !== null && "max_heat_flux" in value && "elements" in value && "input" in value;
}

function isHeatPlaneQuad2dResult(value: unknown): value is HeatPlaneQuad2dResult {
  return typeof value === "object" && value !== null && "max_heat_flux" in value && "elements" in value && "input" in value;
}

function isThermalTruss2dResult(value: unknown): value is ThermalTruss2dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "elements" in value && "input" in value;
}

function isThermalTruss3dResult(value: unknown): value is ThermalTruss3dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "elements" in value && "input" in value;
}

function isTrussResult(value: unknown): value is Truss2dResult {
  return typeof value === "object" && value !== null && "max_stress" in value && "elements" in value && "nodes" in value && "input" in value;
}

function isTruss3dResult(value: unknown): value is Truss3dResult {
  return typeof value === "object" && value !== null && "max_stress" in value && "elements" in value && "nodes" in value && "input" in value;
}

function isBeam1dResult(value: unknown): value is Beam1dResult {
  return typeof value === "object" && value !== null && "max_bending_stress" in value && "elements" in value && "input" in value;
}

function isThermalBeam1dResult(value: unknown): value is ThermalBeam1dResult {
  return typeof value === "object" && value !== null && "max_temperature_gradient" in value && "max_bending_stress" in value && "input" in value;
}

function isTorsion1dResult(value: unknown): value is Torsion1dResult {
  return typeof value === "object" && value !== null && "max_torque" in value && "max_rotation" in value && "input" in value;
}

function isSpring1dResult(value: unknown): value is Spring1dResult {
  return typeof value === "object" && value !== null && "max_force" in value && "elements" in value && "input" in value;
}

function isSpring2dResult(value: unknown): value is Spring2dResult {
  return typeof value === "object" && value !== null && "max_force" in value && "elements" in value && "input" in value;
}

function isSpring3dResult(value: unknown): value is Spring3dResult {
  return typeof value === "object" && value !== null && "max_force" in value && "elements" in value && "input" in value;
}

function isFrame2dResult(value: unknown): value is Frame2dResult {
  return typeof value === "object" && value !== null && "max_moment" in value && "max_rotation" in value && "input" in value;
}

function isThermalFrame2dResult(value: unknown): value is ThermalFrame2dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "max_temperature_gradient" in value && "max_rotation" in value && "input" in value;
}

function isPlaneResult(
  value: unknown,
): value is PlaneTriangle2dResult | PlaneQuad2dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "elements" in value &&
    "nodes" in value &&
    "input" in value &&
    Array.isArray(
      (value as PlaneTriangle2dResult | PlaneQuad2dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult).elements,
    ) &&
    (value as PlaneTriangle2dResult | PlaneQuad2dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult).elements.some(
      (element) => "node_k" in element,
    )
  );
}

export function applyHistoryJobPayload(
  payload: { job: JobEnvelope["job"]; result?: HistoryJobResult | null },
  effects: HistoryOpenEffects,
) {
  const { activeMaterial, copy, recordHistory, openWorkspaceStudy, setMessage } = effects;
  effects.setJob(payload.job);

  if (!payload.result) {
    setMessage(payload.job.status === "failed" ? payload.job.message ?? copy.historyLoaded : copy.historyLoaded);
    return;
  }

  const workflowResult = isWorkflowGraphResult(payload.result) ? payload.result : null;
  if (workflowResult) {
    effects.setResult(null);
    const summary = summarizeWorkflowArtifacts(workflowResult);
    effects.setSidebarSection("workflow");
    effects.setWorkflowPanelTab("runs");
    effects.setSelectedWorkflowId(workflowResult.workflow_id);
    effects.setWorkflowRuns((current: WorkflowRunRecord[]) =>
      upsertWorkflowRunRecord(current, {
        jobId: payload.job.job_id,
        workflowId: workflowResult.workflow_id,
        status: payload.job.status,
        progress: payload.job.progress ?? 0,
        currentNode: workflowResult.current_node ?? payload.job.message ?? null,
        summary,
        updatedAt: payload.job.updated_at ?? null,
        skippedNodes: workflowResult.skipped_nodes ?? [],
        branchDecisions: workflowResult.branch_decisions ?? [],
        nodeRuns: workflowResult.node_runs ?? [],
        artifactLineage: workflowResult.artifact_lineage ?? [],
        result: workflowResult,
      }),
    );
    setMessage(
      summary
        ? `${copy.workflowCatalogCompleted}: ${workflowResult.workflow_id} (${summary})`
        : `${copy.workflowCatalogCompleted}: ${workflowResult.workflow_id}`,
    );
    return;
  }

  const nonWorkflowResult = payload.result as Exclude<HistoryJobResult, WorkflowGraphJobResult>;
  effects.setResult(nonWorkflowResult);

  if (isAxialResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("axial_bar_1d");
    effects.setAxialForm({
      length: nonWorkflowResult.input.length,
      area: nonWorkflowResult.input.area,
      elements: nonWorkflowResult.input.elements,
      tipForce: nonWorkflowResult.input.tip_force,
      material: activeMaterial,
      youngsModulusGpa: nonWorkflowResult.input.youngs_modulus / 1.0e9,
    });
  }

  if (isThermalBar1dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("thermal_bar_1d");
    effects.setThermalBarModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isHeatBar1dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("heat_bar_1d");
    effects.setHeatBarModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isHeatPlaneTriangle2dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("heat_plane_triangle_2d");
    effects.setHeatPlaneModel(nonWorkflowResult.input);
    effects.setPlaneResultField("average_temperature");
    openWorkspaceStudy("controls");
  }

  if (isHeatPlaneQuad2dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("heat_plane_quad_2d");
    effects.setHeatPlaneModel(nonWorkflowResult.input);
    effects.setPlaneResultField("average_temperature");
    openWorkspaceStudy("controls");
  }

  if (isThermalBeam1dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("thermal_beam_1d");
    effects.setThermalBeamModel(ensureBeamModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  if (isThermalTruss2dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("thermal_truss_2d");
    effects.setThermalTrussModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isThermalTruss3dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("thermal_truss_3d");
    effects.setThermalTruss3dModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isSpring1dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("spring_1d");
    effects.setSpringModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isSpring2dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("spring_2d");
    effects.setSpring2dModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isSpring3dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("spring_3d");
    effects.setSpring3dModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isBeam1dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("beam_1d");
    effects.setBeamModel(ensureBeamModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  if (isTorsion1dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("torsion_1d");
    effects.setTorsionModel(nonWorkflowResult.input);
    openWorkspaceStudy("controls");
  }

  if (isTrussResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("truss_2d");
    effects.setTrussModel(ensureTrussModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  if (isTruss3dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("truss_3d");
    effects.setTruss3dModel(ensureTruss3dModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  if (isFrame2dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("frame_2d");
    effects.setFrameModel(ensureFrameModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  if (isThermalFrame2dResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind("thermal_frame_2d");
    effects.setThermalFrameModel(ensureFrameModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  if (isPlaneResult(nonWorkflowResult)) {
    recordHistory(copy.historyAction);
    effects.setStudyKind(
      nonWorkflowResult.input.elements.some((element) => "node_l" in element) ? "plane_quad_2d" : "plane_triangle_2d",
    );
    effects.setPlaneModel(ensurePlaneModelMaterials(nonWorkflowResult.input, activeMaterial));
    openWorkspaceStudy("controls");
  }

  setMessage(payload.job.status === "failed" ? payload.job.message ?? copy.historyLoaded : copy.historyLoaded);
}
function ensureBeamModelMaterials<T extends { materials?: Array<{ id: string }>; elements: Array<{ material_id?: string | undefined }> }>(
  model: T,
  materialValue: string,
): T {
  const existingMaterials = model.materials?.length
    ? model.materials
    : [createMaterialDefinition(materialValue, 1, { id: "mat-1" })];
  const defaultMaterialId = existingMaterials[0]?.id ?? "mat-1";
  return {
    ...model,
    materials: existingMaterials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}
