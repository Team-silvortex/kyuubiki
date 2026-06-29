import test from "node:test";
import assert from "node:assert/strict";

import { createWorkbenchProjectStorageController } from "../../src/components/workbench/workbench-project-storage-controller.ts";
import type { JobEnvelope, JobState } from "../../src/lib/api/fem-shared.ts";
import type { ProjectRecord } from "../../src/lib/api/project-types.ts";
import type { WorkbenchAdminDataBackendService } from "../../src/lib/workbench/admin-data-backend-service-core.ts";
import type { WorkbenchProjectLibraryBackendService } from "../../src/lib/workbench/project-library-backend-service-core.ts";

function projectRecord(): ProjectRecord {
  return {
    description: "bundle export",
    inserted_at: "2026-06-29T00:00:00.000Z",
    models: [],
    name: "Project A",
    project_id: "project-a",
    updated_at: "2026-06-29T00:00:00.000Z",
  };
}

function jobState(jobId: string): JobState {
  return {
    has_result: true,
    job_id: jobId,
    progress: 1,
    status: "completed",
    worker_id: "worker-a",
  };
}

function adminDataService(calls: string[]): WorkbenchAdminDataBackendService {
  return {
    deleteJob: async (jobId) => ({ deleted: true, job: jobState(jobId) }),
    deleteResult: async (jobId) => ({ deleted: true, job_id: jobId, result: {} }),
    fetchJob: async <TResult = unknown>(jobId: string): Promise<JobEnvelope<TResult>> => {
      calls.push(`fetch-job:${jobId}`);
      return {
        job: jobState(jobId),
        result: { displacement: 1.5 } as TResult,
      };
    },
    fetchResults: async () => ({ results: [] }),
    listResults: async () => [],
    updateJob: async (jobId) => ({ job: jobState(jobId) }),
    updateResult: async (jobId, result) => ({ job_id: jobId, result }),
  };
}

function projectLibraryService(calls: string[]): WorkbenchProjectLibraryBackendService {
  return {
    createModel: async () => {
      throw new Error("unused");
    },
    createModelVersion: async () => {
      throw new Error("unused");
    },
    createProject: async () => {
      throw new Error("unused");
    },
    deleteModel: async () => {
      throw new Error("unused");
    },
    deleteModelVersion: async () => {
      throw new Error("unused");
    },
    deleteProject: async () => {
      throw new Error("unused");
    },
    fetchModel: async (modelId) => {
      calls.push(`fetch-model:${modelId}`);
      return {
        model: {
          inserted_at: "2026-06-29T00:00:00.000Z",
          kind: "truss_2d",
          model_id: modelId,
          model_schema_version: "kyuubiki.model/v1",
          name: "Model A",
          payload: {},
          project_id: "project-a",
          updated_at: "2026-06-29T00:00:00.000Z",
        },
      };
    },
    fetchModelVersion: async () => {
      throw new Error("unused");
    },
    fetchModelVersions: async (modelId) => {
      calls.push(`fetch-versions:${modelId}`);
      return { versions: [] };
    },
    fetchProjects: async () => ({ projects: [] }),
    updateModel: async () => {
      throw new Error("unused");
    },
    updateModelVersion: async () => {
      throw new Error("unused");
    },
    updateProject: async () => {
      throw new Error("unused");
    },
  };
}

function baseController(calls: string[]) {
  return createWorkbenchProjectStorageController({
    activeMaterial: "steel",
    adminDataBackendService: adminDataService(calls),
    axialForm: {},
    beamModel: {},
    frameModel: {},
    getPersistedModelEffects: () => ({}),
    getSelectedProject: projectRecord,
    getSelectedProjectModels: () => [{ model_id: "model-a" }],
    heatBarModel: {},
    heatPlaneModel: {},
    jobHistory: [jobState("job-a")],
    loadedModelName: "Model A",
    parametric: {},
    planeModel: {},
    projectDescriptionDraft: "",
    projectLibraryBackendService: projectLibraryService(calls),
    projectNameDraft: "Project A",
    refreshProjects: async () => {},
    refreshVersions: async () => {},
    round: (value) => value,
    selectedModelId: "model-a",
    selectedProjectId: "project-a",
    selectedVersionId: null,
    serializeCurrentModel: () => ({ model_schema_version: "kyuubiki.model/v1" }),
    setMessage: () => {},
    setModelVersions: () => {},
    setSelectedModelId: () => {},
    setSelectedProjectId: () => {},
    setSelectedVersionId: () => {},
    spring2dModel: {},
    spring3dModel: {},
    springModel: {},
    startTransition: (callback) => callback(),
    studyKind: "truss_2d",
    t: {
      initialFailed: "failed",
      projectExported: "exported",
      projectExportedPartial: "partial",
      projectRequired: "project required",
    },
    thermalBarModel: {},
    thermalBeamModel: {},
    thermalFrameModel: {},
    thermalTruss3dModel: {},
    thermalTrussModel: {},
    torsionModel: {},
    truss3dModel: {},
    trussModel: {},
  });
}

test("project bundle export reads job results through admin data backend service", async () => {
  const calls: string[] = [];
  const controller = baseController(calls);

  const payload = await controller.buildProjectBundleJson();
  const bundle = JSON.parse(payload.bundle) as {
    results: Array<{ job_id: string; result: Record<string, unknown> }>;
  };

  assert.equal(payload.partial, false);
  assert.deepEqual(bundle.results, [
    {
      job_id: "job-a",
      result: { displacement: 1.5 },
      status: "completed",
      worker_id: "worker-a",
    },
  ]);
  assert.deepEqual(calls, ["fetch-model:model-a", "fetch-versions:model-a", "fetch-job:job-a"]);
});
