import type { MutableRefObject } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import {
  isWorkflowRunTerminalStatus,
  type DirectMeshSelectionMode,
  type FrontendRuntimeMode,
  type JobEnvelope,
  type Torsion1dJobInput,
  type Truss2dJobInput,
  type Truss3dJobInput,
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
import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import {
  workbenchStudyRunBackendService,
} from "@/lib/workbench/study-run-backend-service";
import type {
  WorkbenchStudyResult,
  WorkbenchStudyRunBackendService,
} from "@/lib/workbench/study-run-backend-service-core";

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
  runBackendService?: WorkbenchStudyRunBackendService;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  setDirectMeshExecution: (value: DirectMeshExecutionState | null) => void;
  setJob: (value: JobEnvelope["job"] | null) => void;
  setMessage: (value: string) => void;
  setSystemAlerts: (value: WorkbenchAlertItem[] | ((current: WorkbenchAlertItem[]) => WorkbenchAlertItem[])) => void;
  setResult: (value: WorkbenchStudyResult | null) => void;
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
  runBackendService,
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
  runBackendService: WorkbenchStudyRunBackendService;
  setJob: (value: JobEnvelope["job"] | null) => void;
  setMessage: (value: string) => void;
  setResult: (value: WorkbenchStudyResult | null) => void;
  copy: WorkbenchCopy;
  labels: Pick<WorkbenchCopy, "pollingDetached">;
}) {
  const pollToken = ++jobPollTokenRef.current;
  let consecutiveErrors = 0;

  for (let attempt = 0; attempt < 40; attempt += 1) {
    if (pollToken !== jobPollTokenRef.current) return;

    try {
      const payload = await runBackendService.fetchJob(jobId);

      if (pollToken !== jobPollTokenRef.current) return;

      consecutiveErrors = 0;
      setJob(payload.job);

      if (payload.result) {
        setResult(payload.result);
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
  runBackendService = workbenchStudyRunBackendService,
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

  let created = await runBackendService.submitRun({
    axialForm,
    beamModel,
    directMeshEndpointsText,
    directMeshSelectionMode,
    frontendRuntimeMode,
    frameModel,
    heatBarModel,
    heatPlaneModel,
    planeModel,
    selectedProjectId,
    selectedVersionId,
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
    trussModel,
  }).catch((error: unknown) => {
    if (error instanceof Error && error.message === "direct-mesh-endpoints-required") {
      throw new Error(labels.directMeshEndpointsHelp);
    }
    throw error;
  });

  setJob(created.envelope.job);

  if (created.envelope.result) {
    setResult(created.envelope.result);
  }

  if (created.backend === "direct_mesh") {
    setDirectMeshExecution({
      endpoint: created.envelope.direct_mesh.endpoint,
      strategy: created.envelope.direct_mesh.strategy,
      at: new Date().toISOString(),
    });
    setMessage(`${labels.directMeshCompleted}: ${created.envelope.job.worker_id ?? "direct-mesh"}`);
    return;
  }

  await refreshJobHistory();
  await pollWorkbenchJob({
    jobId: created.envelope.job.job_id,
    kind: studyKind,
    jobPollTokenRef,
    refreshJobHistory,
    runBackendService,
    setJob,
    setMessage,
    setResult,
    copy,
    labels,
  });
}
