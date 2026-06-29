"use client";

import {
  createAxialBarJob,
  createBeam1dJob,
  createHeatBar1dJob,
  createSpring1dJob,
  createThermalBar1dJob,
  createThermalBeam1dJob,
  createTorsion1dJob,
  resolveBeam1dJobInput,
  resolveHeatBar1dJobInput,
  resolveSpring1dJobInput,
  resolveThermalBar1dJobInput,
  resolveThermalBeam1dJobInput,
  resolveTorsion1dJobInput,
} from "@/lib/api/fem-1d";
import {
  createFrame2dJob,
  createSpring2dJob,
  createThermalFrame2dJob,
  createThermalTruss2dJob,
  createTruss2dJob,
  resolveFrame2dJobInput,
  resolveSpring2dJobInput,
  resolveThermalFrame2dJobInput,
  resolveThermalTruss2dJobInput,
  resolveTruss2dJobInput,
} from "@/lib/api/fem-2d-line";
import {
  createElectrostaticPlaneQuad2dJob,
  createElectrostaticPlaneTriangle2dJob,
  createHeatPlaneQuad2dJob,
  createHeatPlaneTriangle2dJob,
  createPlaneQuad2dJob,
  createPlaneTriangle2dJob,
  createThermalPlaneQuad2dJob,
  createThermalPlaneTriangle2dJob,
  resolveElectrostaticPlaneQuad2dJobInput,
  resolveElectrostaticPlaneTriangle2dJobInput,
  resolveHeatPlaneQuad2dJobInput,
  resolveHeatPlaneTriangle2dJobInput,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveThermalPlaneQuad2dJobInput,
  resolveThermalPlaneTriangle2dJobInput,
  type ElectrostaticPlaneQuad2dJobInput,
  type ElectrostaticPlaneTriangle2dJobInput,
  type HeatPlaneQuad2dJobInput,
  type HeatPlaneTriangle2dJobInput,
  type PlaneQuad2dJobInput,
  type PlaneTriangle2dJobInput,
  type ThermalPlaneQuad2dJobInput,
  type ThermalPlaneTriangle2dJobInput,
} from "@/lib/api/fem-2d-surface";
import {
  createSpring3dJob,
  createThermalTruss3dJob,
  createTruss3dJob,
  resolveSpring3dJobInput,
  resolveThermalTruss3dJobInput,
  resolveTruss3dJobInput,
} from "@/lib/api/fem-3d";
import { createDirectMeshSolve, fetchJobStatus } from "@/lib/api/runtime-client";
import {
  type JobEnvelope,
} from "@/lib/api/fem-shared";
import type { StudyKind } from "@/components/workbench/workbench-types";
import {
  createStudyRunBackendService,
  type WorkbenchStudyResult,
  type WorkbenchStudyRunInput,
  type WorkbenchStudyRunTransport,
} from "@/lib/workbench/study-run-backend-service-core";
import { toAxialInput } from "@/lib/workbench/helpers";

const orchestratedSubmitters: Record<
  StudyKind,
  (input: Record<string, unknown>) => Promise<JobEnvelope<WorkbenchStudyResult>>
> = {
  axial_bar_1d: adaptSubmitter(createAxialBarJob),
  heat_bar_1d: adaptSubmitter(createHeatBar1dJob),
  electrostatic_plane_triangle_2d: adaptSubmitter(createElectrostaticPlaneTriangle2dJob),
  electrostatic_plane_quad_2d: adaptSubmitter(createElectrostaticPlaneQuad2dJob),
  heat_plane_triangle_2d: adaptSubmitter(createHeatPlaneTriangle2dJob),
  heat_plane_quad_2d: adaptSubmitter(createHeatPlaneQuad2dJob),
  thermal_bar_1d: adaptSubmitter(createThermalBar1dJob),
  thermal_beam_1d: adaptSubmitter(createThermalBeam1dJob),
  thermal_truss_2d: adaptSubmitter(createThermalTruss2dJob),
  thermal_truss_3d: adaptSubmitter(createThermalTruss3dJob),
  thermal_plane_triangle_2d: adaptSubmitter(createThermalPlaneTriangle2dJob),
  thermal_plane_quad_2d: adaptSubmitter(createThermalPlaneQuad2dJob),
  spring_1d: adaptSubmitter(createSpring1dJob),
  spring_2d: adaptSubmitter(createSpring2dJob),
  spring_3d: adaptSubmitter(createSpring3dJob),
  beam_1d: adaptSubmitter(createBeam1dJob),
  torsion_1d: adaptSubmitter(createTorsion1dJob),
  truss_2d: adaptSubmitter(createTruss2dJob),
  truss_3d: adaptSubmitter(createTruss3dJob),
  frame_2d: adaptSubmitter(createFrame2dJob),
  thermal_frame_2d: adaptSubmitter(createThermalFrame2dJob),
  plane_triangle_2d: adaptSubmitter(createPlaneTriangle2dJob),
  plane_quad_2d: adaptSubmitter(createPlaneQuad2dJob),
};

function adaptSubmitter<TInput, TResult extends WorkbenchStudyResult>(
  submitter: (input: TInput) => Promise<JobEnvelope<TResult>>,
) {
  return (input: Record<string, unknown>) =>
    submitter(input as TInput) as Promise<JobEnvelope<WorkbenchStudyResult>>;
}

const studyRunTransport: WorkbenchStudyRunTransport = {
  resolveStudyPayload,
  fetchJob(jobId) {
    return fetchJobStatus<WorkbenchStudyResult>(jobId);
  },
  submitDirectMesh(studyKind, input, endpoints, selectionMode) {
    return createDirectMeshSolve<WorkbenchStudyResult>(studyKind, input, endpoints, selectionMode);
  },
  async submitOrchestrated(studyKind, input) {
    return orchestratedSubmitters[studyKind](input) as ReturnType<WorkbenchStudyRunTransport["submitOrchestrated"]>;
  },
};

export const workbenchStudyRunBackendService = createStudyRunBackendService(studyRunTransport);

function resolveStudyPayload(input: WorkbenchStudyRunInput): Record<string, unknown> {
  switch (input.studyKind) {
    case "axial_bar_1d":
      return toAxialInput(input.axialForm);
    case "heat_bar_1d":
      return resolveHeatBar1dJobInput(input.heatBarModel) as unknown as Record<string, unknown>;
    case "electrostatic_plane_quad_2d":
      return resolveElectrostaticPlaneQuad2dJobInput(
        input.planeModel as ElectrostaticPlaneQuad2dJobInput,
      ) as unknown as Record<string, unknown>;
    case "electrostatic_plane_triangle_2d":
      return resolveElectrostaticPlaneTriangle2dJobInput(
        input.planeModel as ElectrostaticPlaneTriangle2dJobInput,
      ) as unknown as Record<string, unknown>;
    case "heat_plane_quad_2d":
      return resolveHeatPlaneQuad2dJobInput(
        input.heatPlaneModel as HeatPlaneQuad2dJobInput,
      ) as unknown as Record<string, unknown>;
    case "heat_plane_triangle_2d":
      return resolveHeatPlaneTriangle2dJobInput(
        input.heatPlaneModel as HeatPlaneTriangle2dJobInput,
      ) as unknown as Record<string, unknown>;
    case "thermal_bar_1d":
      return resolveThermalBar1dJobInput(input.thermalBarModel) as unknown as Record<string, unknown>;
    case "thermal_beam_1d":
      return resolveThermalBeam1dJobInput(input.thermalBeamModel) as unknown as Record<string, unknown>;
    case "thermal_frame_2d":
      return resolveThermalFrame2dJobInput(input.thermalFrameModel) as unknown as Record<string, unknown>;
    case "thermal_truss_2d":
      return resolveThermalTruss2dJobInput(input.thermalTrussModel) as unknown as Record<string, unknown>;
    case "thermal_truss_3d":
      return resolveThermalTruss3dJobInput(input.thermalTruss3dModel) as unknown as Record<string, unknown>;
    case "spring_1d":
      return resolveSpring1dJobInput(input.springModel) as unknown as Record<string, unknown>;
    case "spring_2d":
      return resolveSpring2dJobInput(input.spring2dModel) as unknown as Record<string, unknown>;
    case "spring_3d":
      return resolveSpring3dJobInput(input.spring3dModel) as unknown as Record<string, unknown>;
    case "torsion_1d":
      return resolveTorsion1dJobInput(input.torsionModel) as unknown as Record<string, unknown>;
    case "beam_1d":
      return resolveBeam1dJobInput(input.beamModel) as unknown as Record<string, unknown>;
    case "truss_2d":
      return resolveTruss2dJobInput(input.trussModel) as unknown as Record<string, unknown>;
    case "truss_3d":
      return resolveTruss3dJobInput(input.truss3dModel) as unknown as Record<string, unknown>;
    case "frame_2d":
      return resolveFrame2dJobInput(input.frameModel) as unknown as Record<string, unknown>;
    case "thermal_plane_quad_2d":
      return resolveThermalPlaneQuad2dJobInput(
        input.planeModel as ThermalPlaneQuad2dJobInput,
      ) as unknown as Record<string, unknown>;
    case "plane_quad_2d":
      return resolvePlaneQuad2dJobInput(input.planeModel as PlaneQuad2dJobInput) as unknown as Record<string, unknown>;
    case "thermal_plane_triangle_2d":
      return resolveThermalPlaneTriangle2dJobInput(
        input.planeModel as ThermalPlaneTriangle2dJobInput,
      ) as unknown as Record<string, unknown>;
    case "plane_triangle_2d":
      return resolvePlaneTriangle2dJobInput(
        input.planeModel as PlaneTriangle2dJobInput,
      ) as unknown as Record<string, unknown>;
  }
}
