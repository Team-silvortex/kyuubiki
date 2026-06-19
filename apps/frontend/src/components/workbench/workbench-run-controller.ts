import type { MutableRefObject } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import {
  createAxialBarJob,
  createBeam1dJob,
  createElectrostaticPlaneQuad2dJob,
  createElectrostaticPlaneTriangle2dJob,
  createDirectMeshSolve,
  createFrame2dJob,
  createHeatBar1dJob,
  createHeatPlaneQuad2dJob,
  createHeatPlaneTriangle2dJob,
  createPlaneQuad2dJob,
  createPlaneTriangle2dJob,
  createSpring1dJob,
  createSpring2dJob,
  createSpring3dJob,
  createThermalBar1dJob,
  createThermalBeam1dJob,
  createThermalFrame2dJob,
  createThermalPlaneQuad2dJob,
  createThermalPlaneTriangle2dJob,
  createThermalTruss2dJob,
  createThermalTruss3dJob,
  createTorsion1dJob,
  createTruss2dJob,
  createTruss3dJob,
  fetchJobStatus,
  isWorkflowRunTerminalStatus,
  resolveBeam1dJobInput,
  resolveElectrostaticPlaneQuad2dJobInput,
  resolveElectrostaticPlaneTriangle2dJobInput,
  resolveFrame2dJobInput,
  resolveHeatBar1dJobInput,
  resolveHeatPlaneQuad2dJobInput,
  resolveHeatPlaneTriangle2dJobInput,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveSpring1dJobInput,
  resolveSpring2dJobInput,
  resolveSpring3dJobInput,
  resolveThermalBar1dJobInput,
  resolveThermalBeam1dJobInput,
  resolveThermalFrame2dJobInput,
  resolveThermalPlaneQuad2dJobInput,
  resolveThermalPlaneTriangle2dJobInput,
  resolveThermalTruss2dJobInput,
  resolveThermalTruss3dJobInput,
  resolveTorsion1dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  type AxialBarResult,
  type Beam1dResult,
  type ElectrostaticPlaneQuad2dJobInput,
  type ElectrostaticPlaneQuad2dResult,
  type ElectrostaticPlaneTriangle2dJobInput,
  type ElectrostaticPlaneTriangle2dResult,
  type DirectMeshSelectionMode,
  type Frame2dResult,
  type FrontendRuntimeMode,
  type HeatBar1dResult,
  type HeatPlaneQuad2dJobInput,
  type HeatPlaneQuad2dResult,
  type HeatPlaneTriangle2dJobInput,
  type HeatPlaneTriangle2dResult,
  type JobEnvelope,
  type PlaneQuad2dJobInput,
  type PlaneQuad2dResult,
  type PlaneTriangle2dJobInput,
  type PlaneTriangle2dResult,
  type Spring1dResult,
  type Spring2dResult,
  type Spring3dResult,
  type ThermalBar1dResult,
  type ThermalBeam1dResult,
  type ThermalFrame2dJobInput,
  type ThermalFrame2dResult,
  type ThermalPlaneQuad2dJobInput,
  type ThermalPlaneQuad2dResult,
  type ThermalPlaneTriangle2dJobInput,
  type ThermalPlaneTriangle2dResult,
  type ThermalTruss2dResult,
  type ThermalTruss3dResult,
  type Torsion1dJobInput,
  type Torsion1dResult,
  type Truss2dJobInput,
  type Truss2dResult,
  type Truss3dJobInput,
  type Truss3dResult,
} from "@/lib/api";
import type {
  AxialFormState,
  DirectMeshExecutionState,
  TrussDiagnostics,
} from "@/components/workbench/workbench-defaults";
import { formatJobMessage } from "@/components/workbench/workbench-result-helpers";
import type {
  BeamStudyJobInput,
  FrameStudyJobInput,
  HeatBarStudyJobInput,
  HeatPlaneStudyJobInput,
  PlaneStudyJobInput,
  StudyKind,
  ThermalBarStudyJobInput,
  ThermalBeamStudyJobInput,
  ThermalFrameStudyJobInput,
  ThermalTruss2dStudyJobInput,
  ThermalTruss3dStudyJobInput,
  SpringStudyJobInput,
  Spring2dStudyJobInput,
  Spring3dStudyJobInput,
} from "@/components/workbench/workbench-types";
import { parseDirectMeshEndpoints, toAxialInput } from "@/lib/workbench/helpers";
import { validateWorkbenchExecutionGovernance } from "@/lib/workbench/governance";
import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";

type AnyStudyResult =
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

type WorkbenchRunLabels = Pick<
  WorkbenchCopy,
  | "precheckPrefix"
  | "dispatching"
  | "directMeshEndpointsHelp"
  | "directMeshCompleted"
  | "requestTimedOut"
  | "initialFailed"
  | "pollingDetached"
>;

type RunWorkbenchAnalysisArgs = {
  axialForm: AxialFormState;
  beamModel: BeamStudyJobInput;
  directMeshEndpointsText: string;
  directMeshSelectionMode: DirectMeshSelectionMode;
  frontendRuntimeMode: FrontendRuntimeMode;
  frameModel: FrameStudyJobInput;
  heatBarModel: HeatBarStudyJobInput;
  heatPlaneModel: HeatPlaneStudyJobInput;
  jobPollTokenRef: MutableRefObject<number>;
  copy: WorkbenchCopy;
  labels: WorkbenchRunLabels;
  planeModel: PlaneStudyJobInput;
  refreshJobHistory: () => Promise<void>;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  setDirectMeshExecution: (value: DirectMeshExecutionState | null) => void;
  setJob: (value: JobEnvelope["job"] | null) => void;
  setMessage: (value: string) => void;
  setSystemAlerts: (value: WorkbenchAlertItem[] | ((current: WorkbenchAlertItem[]) => WorkbenchAlertItem[])) => void;
  setResult: (value: AnyStudyResult | null) => void;
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
  trussDiagnostics: TrussDiagnostics | null;
  trussModel: Truss2dJobInput;
};

async function pollWorkbenchJob({
  jobId,
  kind,
  jobPollTokenRef,
  refreshJobHistory,
  setJob,
  setMessage,
  setResult,
  copy,
  labels,
}: {
  jobId: string;
  kind: StudyKind;
  jobPollTokenRef: MutableRefObject<number>;
  refreshJobHistory: () => Promise<void>;
  setJob: (value: JobEnvelope["job"] | null) => void;
  setMessage: (value: string) => void;
  setResult: (value: AnyStudyResult | null) => void;
  copy: WorkbenchCopy;
  labels: Pick<WorkbenchCopy, "pollingDetached">;
}) {
  const pollToken = ++jobPollTokenRef.current;
  let consecutiveErrors = 0;

  for (let attempt = 0; attempt < 40; attempt += 1) {
    if (pollToken !== jobPollTokenRef.current) return;

    try {
      const payload =
        kind === "axial_bar_1d"
          ? await fetchJobStatus<AxialBarResult>(jobId)
          : kind === "heat_bar_1d"
          ? await fetchJobStatus<HeatBar1dResult>(jobId)
          : kind === "electrostatic_plane_triangle_2d"
            ? await fetchJobStatus<ElectrostaticPlaneTriangle2dResult>(jobId)
            : kind === "electrostatic_plane_quad_2d"
              ? await fetchJobStatus<ElectrostaticPlaneQuad2dResult>(jobId)
            : kind === "thermal_bar_1d"
              ? await fetchJobStatus<ThermalBar1dResult>(jobId)
              : kind === "thermal_beam_1d"
                ? await fetchJobStatus<ThermalBeam1dResult>(jobId)
                : kind === "thermal_truss_2d"
                  ? await fetchJobStatus<ThermalTruss2dResult>(jobId)
                  : kind === "thermal_truss_3d"
                    ? await fetchJobStatus<ThermalTruss3dResult>(jobId)
                    : kind === "spring_1d"
                      ? await fetchJobStatus<Spring1dResult>(jobId)
                      : kind === "spring_2d"
                        ? await fetchJobStatus<Spring2dResult>(jobId)
                        : kind === "spring_3d"
                          ? await fetchJobStatus<Spring3dResult>(jobId)
                          : kind === "torsion_1d"
                            ? await fetchJobStatus<Torsion1dResult>(jobId)
                            : kind === "beam_1d"
                              ? await fetchJobStatus<Beam1dResult>(jobId)
                              : kind === "truss_2d"
                                ? await fetchJobStatus<Truss2dResult>(jobId)
                                : kind === "truss_3d"
                                  ? await fetchJobStatus<Truss3dResult>(jobId)
                                  : kind === "frame_2d"
                                    ? await fetchJobStatus<Frame2dResult>(jobId)
                                    : await fetchJobStatus<PlaneTriangle2dResult | PlaneQuad2dResult>(jobId);

      if (pollToken !== jobPollTokenRef.current) return;

      consecutiveErrors = 0;
      setJob(payload.job);

      if (payload.result) {
        setResult(payload.result as AnyStudyResult);
      }

      setMessage(formatJobMessage(payload.job, `${jobId} ${payload.job.status}`, copy));

      if (isWorkflowRunTerminalStatus(payload.job.status)) {
        await refreshJobHistory();
        return;
      }
    } catch (error) {
      if (pollToken !== jobPollTokenRef.current) return;

      consecutiveErrors += 1;
      if (consecutiveErrors >= 3) {
        throw error;
      }

      await new Promise((resolve) => window.setTimeout(resolve, 500));
      continue;
    }

    await new Promise((resolve) => window.setTimeout(resolve, 250));
  }

  if (pollToken === jobPollTokenRef.current) {
    setMessage(labels.pollingDetached);
    await refreshJobHistory();
  }
}

export async function runWorkbenchAnalysis({
  axialForm,
  beamModel,
  directMeshEndpointsText,
  directMeshSelectionMode,
  frontendRuntimeMode,
  frameModel,
  heatBarModel,
  heatPlaneModel,
  jobPollTokenRef,
  copy,
  labels,
  planeModel,
  refreshJobHistory,
  selectedProjectId,
  selectedVersionId,
  setDirectMeshExecution,
  setJob,
  setMessage,
  setSystemAlerts,
  setResult,
  spring2dModel,
  spring3dModel,
  springModel,
  studyKind,
  thermalBarModel,
  thermalBeamModel,
  thermalFrameModel,
  thermalTruss3dModel,
  thermalTrussModel,
  torsionModel,
  truss3dModel,
  trussDiagnostics,
  trussModel,
}: RunWorkbenchAnalysisArgs) {
  const precheckErrors = trussDiagnostics?.blockingMessages ?? [];
  if (precheckErrors.length > 0) {
    const precheckMessage = `${labels.precheckPrefix}: ${precheckErrors[0]}`;
    upsertWorkbenchAlert(setSystemAlerts, {
      id: "precheck-blocking",
      message: precheckMessage,
      tone: "error",
    });
    setMessage(precheckMessage);
    setResult(null);
    setJob(null);
    return;
  }

  dismissWorkbenchAlert(setSystemAlerts, "precheck-blocking");
  setMessage(labels.dispatching);
  setResult(null);
  jobPollTokenRef.current += 1;

  if (frontendRuntimeMode === "direct_mesh_gui") {
    const governance = validateWorkbenchExecutionGovernance({ frontendRuntimeMode, directMeshEndpointsText });
    if (!governance.ok) {
      throw new Error(labels.directMeshEndpointsHelp);
    }
    const endpoints = parseDirectMeshEndpoints(governance.directMeshEndpointsText);

    const created =
      studyKind === "axial_bar_1d"
        ? await createDirectMeshSolve<AxialBarResult>(
            "axial_bar_1d",
            toAxialInput(axialForm),
            endpoints,
            directMeshSelectionMode,
          )
        : studyKind === "heat_bar_1d"
          ? await createDirectMeshSolve<HeatBar1dResult>(
              "heat_bar_1d",
              resolveHeatBar1dJobInput(heatBarModel) as unknown as Record<string, unknown>,
              endpoints,
              directMeshSelectionMode,
            )
          : studyKind === "heat_plane_quad_2d"
            ? await createDirectMeshSolve<HeatPlaneQuad2dResult>(
                "heat_plane_quad_2d",
                resolveHeatPlaneQuad2dJobInput(heatPlaneModel as HeatPlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                endpoints,
                directMeshSelectionMode,
              )
            : studyKind === "electrostatic_plane_quad_2d"
              ? await createDirectMeshSolve<ElectrostaticPlaneQuad2dResult>(
                  "electrostatic_plane_quad_2d",
                  resolveElectrostaticPlaneQuad2dJobInput(planeModel as ElectrostaticPlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                  endpoints,
                  directMeshSelectionMode,
                )
              : studyKind === "electrostatic_plane_triangle_2d"
                ? await createDirectMeshSolve<ElectrostaticPlaneTriangle2dResult>(
                    "electrostatic_plane_triangle_2d",
                    resolveElectrostaticPlaneTriangle2dJobInput(planeModel as ElectrostaticPlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
                
            : studyKind === "heat_plane_triangle_2d"
              ? await createDirectMeshSolve<HeatPlaneTriangle2dResult>(
                  "heat_plane_triangle_2d",
                  resolveHeatPlaneTriangle2dJobInput(heatPlaneModel as HeatPlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
                  endpoints,
                  directMeshSelectionMode,
                )
              : studyKind === "thermal_bar_1d"
                ? await createDirectMeshSolve<ThermalBar1dResult>(
                    "thermal_bar_1d",
                    resolveThermalBar1dJobInput(thermalBarModel) as unknown as Record<string, unknown>,
                    endpoints,
                    directMeshSelectionMode,
                  )
                : studyKind === "thermal_beam_1d"
                  ? await createDirectMeshSolve<ThermalBeam1dResult>(
                      "thermal_beam_1d",
                      resolveThermalBeam1dJobInput(thermalBeamModel) as unknown as Record<string, unknown>,
                      endpoints,
                      directMeshSelectionMode,
                    )
                  : studyKind === "thermal_frame_2d"
                    ? await createDirectMeshSolve<ThermalFrame2dResult>(
                        "thermal_frame_2d",
                        resolveThermalFrame2dJobInput(thermalFrameModel) as unknown as Record<string, unknown>,
                        endpoints,
                        directMeshSelectionMode,
                      )
                    : studyKind === "thermal_truss_2d"
                      ? await createDirectMeshSolve<ThermalTruss2dResult>(
                          "thermal_truss_2d",
                          resolveThermalTruss2dJobInput(thermalTrussModel) as unknown as Record<string, unknown>,
                          endpoints,
                          directMeshSelectionMode,
                        )
                      : studyKind === "thermal_truss_3d"
                        ? await createDirectMeshSolve<ThermalTruss3dResult>(
                            "thermal_truss_3d",
                            resolveThermalTruss3dJobInput(thermalTruss3dModel) as unknown as Record<string, unknown>,
                            endpoints,
                            directMeshSelectionMode,
                          )
                        : studyKind === "spring_1d"
                          ? await createDirectMeshSolve<Spring1dResult>(
                              "spring_1d",
                              resolveSpring1dJobInput(springModel) as unknown as Record<string, unknown>,
                              endpoints,
                              directMeshSelectionMode,
                            )
                          : studyKind === "spring_2d"
                            ? await createDirectMeshSolve<Spring2dResult>(
                                "spring_2d",
                                resolveSpring2dJobInput(spring2dModel) as unknown as Record<string, unknown>,
                                endpoints,
                                directMeshSelectionMode,
                              )
                            : studyKind === "spring_3d"
                              ? await createDirectMeshSolve<Spring3dResult>(
                                  "spring_3d",
                                  resolveSpring3dJobInput(spring3dModel) as unknown as Record<string, unknown>,
                                  endpoints,
                                  directMeshSelectionMode,
                                )
                              : studyKind === "torsion_1d"
                                ? await createDirectMeshSolve<Torsion1dResult>(
                                    "torsion_1d",
                                    resolveTorsion1dJobInput(torsionModel) as unknown as Record<string, unknown>,
                                    endpoints,
                                    directMeshSelectionMode,
                                  )
                                : studyKind === "beam_1d"
                                  ? await createDirectMeshSolve<Beam1dResult>(
                                      "beam_1d",
                                      resolveBeam1dJobInput(beamModel) as unknown as Record<string, unknown>,
                                      endpoints,
                                      directMeshSelectionMode,
                                    )
                                  : studyKind === "truss_2d"
                                    ? await createDirectMeshSolve<Truss2dResult>(
                                        "truss_2d",
                                        resolveTruss2dJobInput(trussModel) as unknown as Record<string, unknown>,
                                        endpoints,
                                        directMeshSelectionMode,
                                      )
                                    : studyKind === "truss_3d"
                                      ? await createDirectMeshSolve<Truss3dResult>(
                                          "truss_3d",
                                          resolveTruss3dJobInput(truss3dModel) as unknown as Record<string, unknown>,
                                          endpoints,
                                          directMeshSelectionMode,
                                        )
                                      : studyKind === "frame_2d"
                                        ? await createDirectMeshSolve<Frame2dResult>(
                                            "frame_2d",
                                            resolveFrame2dJobInput(frameModel) as unknown as Record<string, unknown>,
                                            endpoints,
                                            directMeshSelectionMode,
                                          )
                                        : studyKind === "thermal_plane_quad_2d"
                                          ? await createDirectMeshSolve<ThermalPlaneQuad2dResult>(
                                              "thermal_plane_quad_2d",
                                              resolveThermalPlaneQuad2dJobInput(planeModel as ThermalPlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                                              endpoints,
                                              directMeshSelectionMode,
                                            )
                                          : studyKind === "plane_quad_2d"
                                            ? await createDirectMeshSolve<PlaneQuad2dResult>(
                                                "plane_quad_2d",
                                                resolvePlaneQuad2dJobInput(planeModel as PlaneQuad2dJobInput) as unknown as Record<string, unknown>,
                                                endpoints,
                                                directMeshSelectionMode,
                                              )
                                            : studyKind === "thermal_plane_triangle_2d"
                                              ? await createDirectMeshSolve<ThermalPlaneTriangle2dResult>(
                                                  "thermal_plane_triangle_2d",
                                                  resolveThermalPlaneTriangle2dJobInput(planeModel as ThermalPlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
                                                  endpoints,
                                                  directMeshSelectionMode,
                                                )
                                              : await createDirectMeshSolve<PlaneTriangle2dResult>(
                                                  "plane_triangle_2d",
                                                  resolvePlaneTriangle2dJobInput(planeModel as PlaneTriangle2dJobInput) as unknown as Record<string, unknown>,
                                                  endpoints,
                                                  directMeshSelectionMode,
                                                );

    setJob(created.job);
    if (created.result) {
      setResult(created.result as AnyStudyResult);
    }
    setDirectMeshExecution({
      endpoint: created.direct_mesh.endpoint,
      strategy: created.direct_mesh.strategy,
      at: new Date().toISOString(),
    });
    setMessage(`${labels.directMeshCompleted}: ${created.job.worker_id ?? "direct-mesh"}`);
    return;
  }

  const jobContext = {
    ...(selectedProjectId ? { project_id: selectedProjectId } : {}),
    ...(selectedVersionId ? { model_version_id: selectedVersionId } : {}),
  };

  const created =
    studyKind === "axial_bar_1d"
      ? await createAxialBarJob({ ...toAxialInput(axialForm), ...jobContext })
      : studyKind === "heat_bar_1d"
        ? await createHeatBar1dJob(resolveHeatBar1dJobInput({ ...heatBarModel, ...jobContext }))
        : studyKind === "electrostatic_plane_quad_2d"
          ? await createElectrostaticPlaneQuad2dJob(resolveElectrostaticPlaneQuad2dJobInput({ ...(planeModel as ElectrostaticPlaneQuad2dJobInput), ...jobContext }))
          : studyKind === "electrostatic_plane_triangle_2d"
            ? await createElectrostaticPlaneTriangle2dJob(resolveElectrostaticPlaneTriangle2dJobInput({ ...(planeModel as ElectrostaticPlaneTriangle2dJobInput), ...jobContext }))
        : studyKind === "heat_plane_quad_2d"
          ? await createHeatPlaneQuad2dJob(resolveHeatPlaneQuad2dJobInput({ ...(heatPlaneModel as HeatPlaneQuad2dJobInput), ...jobContext }))
          : studyKind === "heat_plane_triangle_2d"
            ? await createHeatPlaneTriangle2dJob(resolveHeatPlaneTriangle2dJobInput({ ...(heatPlaneModel as HeatPlaneTriangle2dJobInput), ...jobContext }))
            : studyKind === "thermal_bar_1d"
              ? await createThermalBar1dJob(resolveThermalBar1dJobInput({ ...thermalBarModel, ...jobContext }))
              : studyKind === "thermal_beam_1d"
                ? await createThermalBeam1dJob(resolveThermalBeam1dJobInput({ ...thermalBeamModel, ...jobContext }))
                : studyKind === "thermal_frame_2d"
                  ? await createThermalFrame2dJob(resolveThermalFrame2dJobInput({ ...thermalFrameModel, ...jobContext }))
                  : studyKind === "thermal_truss_2d"
                    ? await createThermalTruss2dJob(resolveThermalTruss2dJobInput({ ...thermalTrussModel, ...jobContext }))
                    : studyKind === "thermal_truss_3d"
                      ? await createThermalTruss3dJob(resolveThermalTruss3dJobInput({ ...thermalTruss3dModel, ...jobContext }))
                      : studyKind === "spring_1d"
                        ? await createSpring1dJob(resolveSpring1dJobInput({ ...springModel, ...jobContext }))
                        : studyKind === "spring_2d"
                          ? await createSpring2dJob(resolveSpring2dJobInput({ ...spring2dModel, ...jobContext }))
                          : studyKind === "spring_3d"
                            ? await createSpring3dJob(resolveSpring3dJobInput({ ...spring3dModel, ...jobContext }))
                            : studyKind === "torsion_1d"
                              ? await createTorsion1dJob(resolveTorsion1dJobInput({ ...torsionModel, ...jobContext }))
                              : studyKind === "beam_1d"
                                ? await createBeam1dJob(resolveBeam1dJobInput({ ...beamModel, ...jobContext }))
                                : studyKind === "truss_2d"
                                  ? await createTruss2dJob(resolveTruss2dJobInput({ ...trussModel, ...jobContext }))
                                  : studyKind === "truss_3d"
                                    ? await createTruss3dJob(resolveTruss3dJobInput({ ...truss3dModel, ...jobContext }))
                                    : studyKind === "frame_2d"
                                      ? await createFrame2dJob(resolveFrame2dJobInput({ ...frameModel, ...jobContext }))
                                      : studyKind === "thermal_plane_quad_2d"
                                        ? await createThermalPlaneQuad2dJob(
                                            resolveThermalPlaneQuad2dJobInput({ ...(planeModel as ThermalPlaneQuad2dJobInput), ...jobContext }),
                                          )
                                        : studyKind === "plane_quad_2d"
                                          ? await createPlaneQuad2dJob(
                                              resolvePlaneQuad2dJobInput({ ...(planeModel as PlaneQuad2dJobInput), ...jobContext }),
                                            )
                                          : studyKind === "thermal_plane_triangle_2d"
                                            ? await createThermalPlaneTriangle2dJob(
                                                resolveThermalPlaneTriangle2dJobInput({ ...(planeModel as ThermalPlaneTriangle2dJobInput), ...jobContext }),
                                              )
                                            : await createPlaneTriangle2dJob(
                                                resolvePlaneTriangle2dJobInput({ ...(planeModel as PlaneTriangle2dJobInput), ...jobContext }),
                                              );

  setJob(created.job);
  await refreshJobHistory();
  await pollWorkbenchJob({
    jobId: created.job.job_id,
    kind: studyKind,
    jobPollTokenRef,
    refreshJobHistory,
    setJob,
    setMessage,
    setResult,
    copy,
    labels,
  });
}
