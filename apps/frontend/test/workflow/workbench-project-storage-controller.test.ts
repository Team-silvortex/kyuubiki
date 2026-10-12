import test from "node:test";
import assert from "node:assert/strict";

import { createWorkbenchProjectStorageController } from "../../src/components/workbench/workbench-project-storage-controller.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import { createWorkbenchFeedback } from "../../src/components/workbench/workbench-feedback.ts";
import { workbenchLibraryRecordSources, workbenchLibraryVersionSources } from "../../src/components/workbench/workbench-library-record-source.ts";
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
    project_id: "project-a",
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

function baseController(
  calls: string[],
  getSelectedProject: () => ProjectRecord | null = projectRecord,
  overrides: Partial<Parameters<typeof createWorkbenchProjectStorageController>[0]> = {},
) {
  const projects = [projectRecord()];
  projects[0].models = [{ model_id: "model-a", project_id: "project-a" } as any];
  const versions = [{ model_id: "model-a", project_id: "project-a", version_id: "version-a" } as any];
  workbenchLibraryRecordSources.mark(projects); workbenchLibraryVersionSources.mark(versions);
  return createWorkbenchProjectStorageController({
    projectContext: createWorkbenchProjectContext({ projectId: "project-a", modelId: "model-a", versionId: null }),
    feedback: createWorkbenchFeedback(), sourceChangedLabel: "Refresh the record source",
    getProjects: () => projects, getModelVersions: () => versions,
    removeConfirmedLibraryRecord: () => {},
    activeMaterial: "steel",
    adminDataBackendService: adminDataService(calls),
    axialForm: {},
    beamModel: {},
    frameModel: {},
    getPersistedModelEffects: () => ({}),
    getSelectedProject,
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
    ...overrides,
  });
}

for (const kind of ["model", "version"] as const) for (const admission of ["unmarked", "copied", "foreign", "stale", "disposed"]) {
  test(`saved ${kind} navigation rejects ${admission} catalog admission without a payload read`, async () => {
    const calls: string[] = [];
    const record = { model_id: "model-a", project_id: "project-a", version_id: "version-a" } as any;
    const projects = [{ ...projectRecord(), models: [record] }], versions = [record];
    const context = createWorkbenchProjectContext({ projectId: "project-a", modelId: "model-a", versionId: "version-a" });
    const feedback = createWorkbenchFeedback();
    const registry = kind === "model" ? workbenchLibraryRecordSources : workbenchLibraryVersionSources;
    const catalog = kind === "model" ? projects : versions;
    if (admission !== "unmarked") registry.mark(catalog);
    const source = registry.sourceOf(catalog), original = source?.isCurrent;
    if (admission === "stale" && source) source.isCurrent = () => false;
    if (admission === "disposed") { context.dispose(); feedback.dispose(); }
    try {
      const controller = baseController(calls, projectRecord, { projectContext: context, feedback,
        getProjects: () => projects, getModelVersions: () => versions,
        getPersistedModelEffects: () => assert.fail("rejected navigation cannot access payload effects"),
        setMessage: (message) => { calls.push(message); } });
      const input = admission === "copied" ? { ...record } : admission === "foreign" ? { ...record, project_id: "other" } : record;
      assert.equal((await (kind === "model" ? controller.openSavedModel(input) : controller.openSavedVersion(input))).ok, false);
      assert.equal(calls.length, admission === "disposed" ? 0 : 1);
      if (calls.length) assert.match(calls[0], /WORKBENCH_RECORD_SOURCE_CHANGED/);
    } finally { if (source && original) source.isCurrent = original; }
  });
}

for (const action of ["updateProjectRecord", "deleteProjectRecord", "saveModelVersion", "deleteSavedModelRecord",
  "renameSelectedVersion", "deleteSelectedVersion"] as const) for (const failed of [false, true]) {
  test(`GUI ${action} suppresses late ${failed ? "failure" : "confirmation"} from an old source`, async () => {
    const calls: string[] = [], projects = [projectRecord()];
    const version = { project_id: "project-a", model_id: "model-a", version_id: "version-a" } as any;
    projects[0].models = [{ ...version }];
    const versions = [version];
    workbenchLibraryRecordSources.mark(projects); workbenchLibraryVersionSources.mark(versions);
    const context = createWorkbenchProjectContext({ projectId: "project-a", modelId: "model-a", versionId: "version-a" });
    const source = action.includes("Version") && action !== "saveModelVersion"
      ? workbenchLibraryVersionSources.sourceOf(versions)! : workbenchLibraryRecordSources.sourceOf(projects)!;
    const original = source.isCurrent, service = projectLibraryService(calls);
    const write = async () => { calls.push("write"); source.isCurrent = () => false;
      if (failed) throw new Error("obsolete failure"); return { project: projects[0], model: projects[0].models![0], version }; };
    for (const method of ["updateProject", "deleteProject", "createModelVersion", "deleteModel", "updateModelVersion", "deleteModelVersion"] as const) service[method] = write as never;
    let completed = Promise.resolve();
    try {
      const controller = baseController(calls, projectRecord, { projectContext: context, selectedVersionId: "version-a",
        getProjects: () => projects, getModelVersions: () => versions, projectLibraryBackendService: service,
        removeConfirmedLibraryRecord: () => { calls.push("remove"); },
        refreshProjects: async () => { calls.push("refresh"); }, refreshVersions: async () => { calls.push("refresh"); },
        setSelectedProjectId: () => { calls.push("apply"); }, setSelectedModelId: () => { calls.push("apply"); },
        setSelectedVersionId: () => { calls.push("apply"); }, setModelVersions: () => { calls.push("apply"); },
        setMessage: () => { calls.push("message"); }, startTransition: (callback) => { completed = Promise.resolve(callback()); } });
      if (action === "saveModelVersion") controller.saveModelVersion(false); else controller[action]();
      await completed;
      assert.deepEqual(calls, ["write"]);
      assert.deepEqual(context.current(), { projectId: "project-a", modelId: "model-a", versionId: "version-a" });
    } finally { source.isCurrent = original; }
  });
}

for (const fails of [false, true]) {
  test(`GUI checkpoint uses one atomic write and preserves local selection on failure=${fails}`, async () => {
    const calls: string[] = [], service = projectLibraryService(calls);
    let completed: Promise<void> = Promise.resolve();
    service.updateModel = async () => assert.fail("save must not update the model before checkpointing");
    service.createModelVersion = async (modelId, input) => {
      calls.push(`checkpoint:${modelId}:${input.name}`);
      if (fails) throw new Error("checkpoint rejected");
      return { version: { ...input, version_id: "version-new", model_id: modelId, project_id: "project-a",
        name: input.name!, kind: input.kind!, model_schema_version: "kyuubiki.model/v1", version_number: 2,
        inserted_at: "2026-09-13T00:00:00Z", updated_at: "2026-09-13T00:00:00Z" } };
    };
    const controller = baseController(calls, projectRecord, {
      projectLibraryBackendService: service,
      startTransition: (callback) => { completed = Promise.resolve(callback()); },
      setSelectedVersionId: (id) => { calls.push(`selected:${id}`); },
      setMessage: (message) => { calls.push(`message:${message}`); },
      t: { modelSaved: "saved", initialFailed: "failed" },
    });
    controller.saveModelVersion(false);
    await completed;
    assert.deepEqual(calls, fails ? ["checkpoint:model-a:Model A", "message:checkpoint rejected"]
      : ["checkpoint:model-a:Model A", "selected:version-new", "message:saved"]);
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

test("project storage download wrappers preserve explicit failure outcomes", async () => {
  const controller = baseController([], () => null);

  for (const download of [controller.downloadProjectBundleJson, controller.downloadProjectBundleZip]) {
    const result = await download();
    assert.equal(result.ok, false);
    if (result.ok) assert.fail("missing project must not report a successful download");
    assert.match(result.error.message, /project required/u);
  }
});

test("project export excludes foreign and unassigned jobs before fetching results", async () => {
  const calls: string[] = [];
  const controller = baseController(calls, projectRecord, {
    jobHistory: [jobState("owned"), { ...jobState("foreign"), project_id: "project-b" },
      { ...jobState("unassigned"), project_id: undefined }],
  });
  const { bundle, partial } = await controller.buildProjectBundleJson();
  assert.equal(partial, false);
  assert.deepEqual(JSON.parse(bundle).jobs.map((entry: JobState) => entry.job_id), ["owned"]);
  assert.deepEqual(JSON.parse(bundle).results.map((entry: JobState) => entry.job_id), ["owned"]);
  assert.deepEqual(calls.filter((entry) => entry.startsWith("fetch-job:")), ["fetch-job:owned"]);
});

for (const mode of ["rejected", "missing", "null", "wrong-job", "wrong-project"]) {
  test(`project export reports a partial bundle for a ${mode} result`, async () => {
    const calls: string[] = [];
    const service = adminDataService(calls);
    service.fetchJob = async <TResult>() => {
      if (mode === "rejected") throw new Error("result unavailable");
      return {
        job: { ...jobState(mode === "wrong-job" ? "other-job" : "job-a"),
          ...(mode === "wrong-project" ? { project_id: "project-b" } : {}) },
        result: (mode === "missing" ? undefined : mode === "null" ? null : { displacement: 1 }) as TResult,
      };
    };
    const controller = baseController(calls, projectRecord, { adminDataBackendService: service });
    const { bundle, partial } = await controller.buildProjectBundleJson();
    assert.equal(partial, true);
    assert.equal(JSON.parse(bundle).jobs.length, 1);
    assert.deepEqual(JSON.parse(bundle).results, []);
  });
}

for (const result of [0, false, ""]) {
  test(`project export preserves the valid false-like result ${JSON.stringify(result)}`, async () => {
    const service = adminDataService([]);
    service.fetchJob = async <TResult>() => ({ job: jobState("job-a"), result: result as TResult });
    const { bundle, partial } = await baseController([], projectRecord, {
      adminDataBackendService: service,
    }).buildProjectBundleJson();
    assert.equal(partial, false);
    assert.equal(JSON.parse(bundle).results[0].result, result);
  });
}

test("project export retains available results when another result fails", async () => {
  const service = adminDataService([]);
  const fetchJob = service.fetchJob;
  service.fetchJob = async <TResult>(id: string) => {
    if (id === "unavailable") throw new Error("unavailable");
    return fetchJob<TResult>(id);
  };
  const { bundle, partial } = await baseController([], projectRecord, {
    jobHistory: [jobState("available"), jobState("unavailable")],
    adminDataBackendService: service,
  }).buildProjectBundleJson();
  assert.equal(partial, true);
  assert.equal(JSON.parse(bundle).jobs.length, 2);
  assert.deepEqual(JSON.parse(bundle).results.map((entry: JobState) => entry.job_id), ["available"]);
});
