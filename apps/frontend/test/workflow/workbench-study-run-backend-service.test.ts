import test from "node:test";
import assert from "node:assert/strict";

import { createStudyRunBackendService } from "../../src/lib/workbench/study-run-backend-service-core.ts";
import type { JobEnvelope } from "../../src/lib/api/fem-shared.ts";
import type {
  DirectMeshSelectionMode,
  DirectMeshSolveEnvelope,
  FrontendRuntimeMode,
} from "../../src/lib/api/runtime-types.ts";
import type { WorkbenchStudyRunInput } from "../../src/lib/workbench/study-run-backend-service-core.ts";
import type { WorkbenchStudyResult } from "../../src/lib/workbench/study-run-backend-service-core.ts";

function jobEnvelope(jobId: string): JobEnvelope<WorkbenchStudyResult> {
  return {
    job: {
      created_at: "2026-06-29T00:00:00.000Z",
      job_id: jobId,
      progress: 0,
      status: "queued",
      updated_at: "2026-06-29T00:00:00.000Z",
      worker_id: null,
    },
  };
}

function directEnvelope(jobId: string): DirectMeshSolveEnvelope<WorkbenchStudyResult> {
  return {
    ...jobEnvelope(jobId),
    direct_mesh: {
      endpoint: "http://agent-a",
      progress_frames: [],
      strategy: "healthiest",
    },
  };
}

function runInput(params: {
  directMeshEndpointsText?: string;
  directMeshSelectionMode?: DirectMeshSelectionMode;
  frontendRuntimeMode?: FrontendRuntimeMode;
} = {}): WorkbenchStudyRunInput {
  const placeholder = {} as never;
  return {
    axialForm: {
      area: 0.01,
      elements: 4,
      length: 2,
      material: "steel",
      tipForce: 1000,
      youngsModulusGpa: 210,
    },
    beamModel: placeholder,
    directMeshEndpointsText: params.directMeshEndpointsText ?? "",
    directMeshSelectionMode: params.directMeshSelectionMode ?? "healthiest",
    frontendRuntimeMode: params.frontendRuntimeMode ?? "orchestrated_gui",
    frameModel: placeholder,
    heatBarModel: placeholder,
    heatPlaneModel: placeholder,
    planeModel: placeholder,
    selectedProjectId: "project-a",
    selectedVersionId: "version-a",
    spring2dModel: placeholder,
    spring3dModel: placeholder,
    springModel: placeholder,
    studyKind: "axial_bar_1d",
    thermalBarModel: placeholder,
    thermalBeamModel: placeholder,
    thermalFrameModel: placeholder,
    thermalTruss3dModel: placeholder,
    thermalTrussModel: placeholder,
    torsionModel: placeholder,
    truss3dModel: placeholder,
    trussModel: placeholder,
  };
}

test("study run backend routes orchestrated studies with project context", async () => {
  const calls: Array<{ input: Record<string, unknown>; kind: string }> = [];
  const service = createStudyRunBackendService({
    resolveStudyPayload: (input) => ({
      area: input.axialForm.area,
      elements: input.axialForm.elements,
      length: input.axialForm.length,
      material: input.axialForm.material,
      tip_force: input.axialForm.tipForce,
      youngs_modulus_gpa: input.axialForm.youngsModulusGpa,
    }),
    fetchJob: async () => jobEnvelope("unused"),
    submitDirectMesh: async () => directEnvelope("direct-job"),
    submitOrchestrated: async (kind, input) => {
      calls.push({ input, kind });
      return jobEnvelope("orch-job");
    },
  });

  const created = await service.submitRun(runInput());

  assert.equal(created.backend, "orchestrated");
  assert.equal(created.envelope.job.job_id, "orch-job");
  assert.deepEqual(calls, [
    {
      kind: "axial_bar_1d",
      input: {
        area: 0.01,
        elements: 4,
        length: 2,
        material: "steel",
        model_version_id: "version-a",
        project_id: "project-a",
        tip_force: 1000,
        youngs_modulus_gpa: 210,
      },
    },
  ]);
});

test("study run backend routes direct mesh studies without project context", async () => {
  const calls: Array<{
    endpoints: string[];
    input: Record<string, unknown>;
    kind: string;
    selectionMode: DirectMeshSelectionMode;
  }> = [];
  const service = createStudyRunBackendService({
    resolveStudyPayload: (input) => ({
      area: input.axialForm.area,
      elements: input.axialForm.elements,
      length: input.axialForm.length,
      material: input.axialForm.material,
      tip_force: input.axialForm.tipForce,
      youngs_modulus_gpa: input.axialForm.youngsModulusGpa,
    }),
    fetchJob: async () => jobEnvelope("unused"),
    submitDirectMesh: async (kind, input, endpoints, selectionMode) => {
      calls.push({ endpoints, input, kind, selectionMode });
      return directEnvelope("direct-job");
    },
    submitOrchestrated: async () => jobEnvelope("orch-job"),
  });

  const created = await service.submitRun(
    runInput({
      directMeshEndpointsText: "http://agent-a, http://agent-b",
      frontendRuntimeMode: "direct_mesh_gui",
    }),
  );

  assert.equal(created.backend, "direct_mesh");
  assert.equal(created.envelope.job.job_id, "direct-job");
  assert.deepEqual(calls, [
    {
      endpoints: ["http://agent-a", "http://agent-b"],
      kind: "axial_bar_1d",
      selectionMode: "healthiest",
      input: {
        area: 0.01,
        elements: 4,
        length: 2,
        material: "steel",
        tip_force: 1000,
        youngs_modulus_gpa: 210,
      },
    },
  ]);
});

test("study run backend keeps job polling behind the service seam", async () => {
  const service = createStudyRunBackendService({
    resolveStudyPayload: () => ({}),
    fetchJob: async (jobId) => jobEnvelope(jobId),
    submitDirectMesh: async () => directEnvelope("direct-job"),
    submitOrchestrated: async () => jobEnvelope("orch-job"),
  });

  const fetched = await service.fetchJob("job-42");

  assert.equal(fetched.job.job_id, "job-42");
});
