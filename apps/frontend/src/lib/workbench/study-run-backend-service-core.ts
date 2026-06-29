"use client";

import type {
  AxialBarResult,
  Beam1dResult,
  HeatBar1dResult,
  Spring1dResult,
  ThermalBar1dResult,
  ThermalBeam1dResult,
  Torsion1dJobInput,
  Torsion1dResult,
} from "@/lib/api/fem-1d";
import type {
  Frame2dResult,
  Spring2dResult,
  ThermalFrame2dResult,
  ThermalTruss2dResult,
  Truss2dJobInput,
  Truss2dResult,
} from "@/lib/api/fem-2d-line";
import type {
  ElectrostaticPlaneQuad2dResult,
  ElectrostaticPlaneTriangle2dResult,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dResult,
  PlaneQuad2dResult,
  PlaneTriangle2dResult,
  ThermalPlaneQuad2dResult,
  ThermalPlaneTriangle2dResult,
} from "@/lib/api/fem-2d-surface";
import type {
  Spring3dResult,
  Truss3dJobInput,
  Truss3dResult,
  ThermalTruss3dResult,
} from "@/lib/api/fem-3d";
import type { JobEnvelope } from "@/lib/api/fem-shared";
import type {
  DirectMeshSelectionMode,
  DirectMeshSolveEnvelope,
  FrontendRuntimeMode,
} from "@/lib/api/runtime-types";
import type { AxialFormState } from "@/components/workbench/workbench-defaults";
import type {
  BeamStudyJobInput,
  FrameStudyJobInput,
  HeatBarStudyJobInput,
  HeatPlaneStudyJobInput,
  PlaneStudyJobInput,
  Spring2dStudyJobInput,
  Spring3dStudyJobInput,
  SpringStudyJobInput,
  StudyKind,
  ThermalBarStudyJobInput,
  ThermalBeamStudyJobInput,
  ThermalFrameStudyJobInput,
  ThermalTruss2dStudyJobInput,
  ThermalTruss3dStudyJobInput,
} from "@/components/workbench/workbench-types";
import { validateWorkbenchExecutionGovernance } from "@/lib/workbench/governance";
import { parseDirectMeshEndpoints } from "@/lib/workbench/helpers";

export type WorkbenchStudyResult =
  | AxialBarResult
  | HeatBar1dResult
  | ElectrostaticPlaneTriangle2dResult
  | ElectrostaticPlaneQuad2dResult
  | HeatPlaneTriangle2dResult
  | HeatPlaneQuad2dResult
  | ThermalBar1dResult
  | ThermalBeam1dResult
  | ThermalTruss2dResult
  | ThermalTruss3dResult
  | ThermalPlaneTriangle2dResult
  | ThermalPlaneQuad2dResult
  | Spring1dResult
  | Spring2dResult
  | Spring3dResult
  | Beam1dResult
  | Torsion1dResult
  | Truss2dResult
  | Truss3dResult
  | Frame2dResult
  | ThermalFrame2dResult
  | PlaneTriangle2dResult
  | PlaneQuad2dResult;

export type WorkbenchStudyRunInput = {
  axialForm: AxialFormState;
  beamModel: BeamStudyJobInput;
  directMeshEndpointsText: string;
  directMeshSelectionMode: DirectMeshSelectionMode;
  frontendRuntimeMode: FrontendRuntimeMode;
  frameModel: FrameStudyJobInput;
  heatBarModel: HeatBarStudyJobInput;
  heatPlaneModel: HeatPlaneStudyJobInput;
  planeModel: PlaneStudyJobInput;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  spring2dModel: Spring2dStudyJobInput;
  spring3dModel: Spring3dStudyJobInput;
  springModel: SpringStudyJobInput;
  studyKind: StudyKind;
  thermalBarModel: ThermalBarStudyJobInput;
  thermalBeamModel: ThermalBeamStudyJobInput;
  thermalFrameModel: ThermalFrameStudyJobInput;
  thermalTruss3dModel: ThermalTruss3dStudyJobInput;
  thermalTrussModel: ThermalTruss2dStudyJobInput;
  torsionModel: Torsion1dJobInput;
  truss3dModel: Truss3dJobInput;
  trussModel: Truss2dJobInput;
};

export type WorkbenchStudyRunCreated =
  | { backend: "orchestrated"; envelope: JobEnvelope<WorkbenchStudyResult> }
  | { backend: "direct_mesh"; envelope: DirectMeshSolveEnvelope<WorkbenchStudyResult> };

export type WorkbenchStudyRunTransport = {
  resolveStudyPayload(input: WorkbenchStudyRunInput): Record<string, unknown>;
  submitDirectMesh(
    studyKind: StudyKind,
    input: Record<string, unknown>,
    endpoints: string[],
    selectionMode: DirectMeshSelectionMode,
  ): Promise<DirectMeshSolveEnvelope<WorkbenchStudyResult>>;
  submitOrchestrated(
    studyKind: StudyKind,
    input: Record<string, unknown>,
  ): Promise<JobEnvelope<WorkbenchStudyResult>>;
  fetchJob(jobId: string): Promise<JobEnvelope<WorkbenchStudyResult>>;
};

export type WorkbenchStudyRunBackendService = {
  fetchJob(jobId: string): Promise<JobEnvelope<WorkbenchStudyResult>>;
  submitRun(input: WorkbenchStudyRunInput): Promise<WorkbenchStudyRunCreated>;
};

export function createStudyRunBackendService(
  transport: WorkbenchStudyRunTransport,
): WorkbenchStudyRunBackendService {
  return {
    fetchJob: transport.fetchJob,
    async submitRun(input) {
      if (input.frontendRuntimeMode === "direct_mesh_gui") {
        const governance = validateWorkbenchExecutionGovernance({
          directMeshEndpointsText: input.directMeshEndpointsText,
          frontendRuntimeMode: input.frontendRuntimeMode,
        });
        if (!governance.ok) throw new Error("direct-mesh-endpoints-required");

        return {
          backend: "direct_mesh",
          envelope: await transport.submitDirectMesh(
            input.studyKind,
            transport.resolveStudyPayload(input),
            parseDirectMeshEndpoints(governance.directMeshEndpointsText),
            input.directMeshSelectionMode,
          ),
        };
      }

      return {
        backend: "orchestrated",
        envelope: await transport.submitOrchestrated(input.studyKind, {
          ...transport.resolveStudyPayload(input),
          ...resolveJobContext(input),
        }),
      };
    },
  };
}

function resolveJobContext(input: WorkbenchStudyRunInput): Record<string, string> {
  return {
    ...(input.selectedProjectId ? { project_id: input.selectedProjectId } : {}),
    ...(input.selectedVersionId ? { model_version_id: input.selectedVersionId } : {}),
  };
}
