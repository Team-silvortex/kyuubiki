import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptProjectModelAction } from "../../src/components/workbench/workbench-script-project-model-controller.ts";
import { createWorkbenchScriptInvoker } from "../../src/components/workbench/workbench-script-invoker.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";
import { workbenchLibraryRecordSources, workbenchLibraryVersionSources } from "../../src/components/workbench/workbench-library-record-source.ts";
import type { WorkbenchProjectLibraryBackendService } from "../../src/lib/workbench/project-library-backend-service-core.ts";
import type {
  ModelEnvelope,
  ModelVersionEnvelope,
  ModelVersionListPayload,
  ProjectEnvelope,
  ProjectListPayload,
} from "../../src/lib/api/project-types.ts";

function projectEnvelope(projectId: string, name = "Project"): ProjectEnvelope {
  return {
    project: {
      description: "",
      inserted_at: "2026-06-29T00:00:00.000Z",
      models: [],
      name,
      project_id: projectId,
      updated_at: "2026-06-29T00:00:00.000Z",
    },
  };
}

function modelEnvelope(modelId: string): ModelEnvelope {
  return {
    model: {
      inserted_at: "2026-06-29T00:00:00.000Z",
      kind: "truss_2d",
      latest_version_id: "version-a",
      model_id: modelId,
      model_schema_version: "kyuubiki.model/v1",
      name: "model",
      payload: {},
      project_id: "project-a",
      updated_at: "2026-06-29T00:00:00.000Z",
    },
  };
}

function versionEnvelope(versionId: string): ModelVersionEnvelope {
  return {
    version: {
      inserted_at: "2026-06-29T00:00:00.000Z",
      kind: "truss_2d",
      model_id: "model-a",
      model_schema_version: "kyuubiki.model/v1",
      name: "v1",
      payload: {},
      project_id: "project-a",
      updated_at: "2026-06-29T00:00:00.000Z",
      version_id: versionId,
      version_number: 1,
    },
  };
}

function projectLibraryService(calls: string[]): WorkbenchProjectLibraryBackendService {
  return {
    createModel: async (projectId, input) => {
      calls.push(`create-model:${projectId}:${input.name}`);
      return modelEnvelope("model-created");
    },
    createModelVersion: async (modelId, input) => {
      calls.push(`create-version:${modelId}:${input.name}`);
      return versionEnvelope("version-created");
    },
    createProject: async (input) => {
      calls.push(`create-project:${input.name}`);
      return projectEnvelope("project-created", input.name);
    },
    deleteModel: async (modelId) => {
      calls.push(`delete-model:${modelId}`);
      return modelEnvelope(modelId);
    },
    deleteModelVersion: async (versionId) => {
      calls.push(`delete-version:${versionId}`);
      return versionEnvelope(versionId);
    },
    deleteProject: async (projectId) => {
      calls.push(`delete-project:${projectId}`);
      return projectEnvelope(projectId);
    },
    fetchModel: async (modelId) => modelEnvelope(modelId),
    fetchModelVersion: async (versionId) => versionEnvelope(versionId),
    fetchModelVersions: async () => ({ versions: [] }) satisfies ModelVersionListPayload,
    fetchProjects: async () => ({ projects: [] }) satisfies ProjectListPayload,
    updateModel: async (modelId, input) => {
      calls.push(`update-model:${modelId}:${input.name}`);
      return modelEnvelope(modelId);
    },
    updateModelVersion: async (versionId, input) => {
      calls.push(`update-version:${versionId}:${input.name}`);
      return versionEnvelope(versionId);
    },
    updateProject: async (projectId, input) => {
      calls.push(`update-project:${projectId}:${input.name}`);
      return projectEnvelope(projectId, input.name);
    },
  };
}

function baseArgs(calls: string[]) {
  const projects = [projectEnvelope("project-a").project, projectEnvelope("project-b", "Second project").project];
  projects[0].models = [modelEnvelope("model-a").model];
  const modelVersions = [versionEnvelope("version-a").version];
  workbenchLibraryRecordSources.mark(projects);
  workbenchLibraryVersionSources.mark(modelVersions);
  return {
    projectContext: createWorkbenchProjectContext({ projectId: "project-a", modelId: "model-a", versionId: "version-a" }),
    removeConfirmedLibraryRecord: () => {},
    action: "",
    activeMaterial: "steel",
    defaultProjectLabel: "Default Project",
    deleteModelVersion: async () => {},
    downloadProjectBundleJson: async () => ({ ok: true as const }),
    downloadProjectBundleZip: async () => ({ ok: true as const }),
    generateModel: () => {},
    generatePanelModel: () => {},
    loadedModelName: "model-a",
    modelCreatedLabel: "model created",
    modelDeletedLabel: "model deleted",
    modelSavedLabel: "model saved",
    noSavedModelsLabel: "no models",
    noVersionsLabel: "no versions",
    payload: {},
    projects,
    modelVersions,
    projectCreatedLabel: "project created",
    projectDeletedLabel: "project deleted",
    projectDescriptionDraft: "",
    projectLibraryBackendService: projectLibraryService(calls),
    projectNameDraft: "draft",
    projectRequiredLabel: "project required",
    projectUpdatedLabel: "project updated",
    refreshProjects: async () => {
      calls.push("refresh-projects");
    },
    refreshVersions: async (modelId: string) => {
      calls.push(`refresh-versions:${modelId}`);
    },
    selectedModelId: null,
    selectedProjectId: null,
    selectedVersionId: null,
    serializeCurrentModel: () => ({ model_schema_version: "kyuubiki.model/v1" }),
    setActiveMaterial: () => {},
    setLoadedModelName: () => {},
    setMessage: (message: string) => {
      calls.push(`message:${message}`);
    },
    setModelVersions: () => {},
    setProjectDescriptionDraft: () => {},
    setProjectNameDraft: () => {},
    setSelectedModelId: (value: string | null) => {
      calls.push(`set-model:${value ?? ""}`);
    },
    setSelectedProjectId: (value: string | null) => {
      calls.push(`set-project:${value ?? ""}`);
    },
    setSelectedVersionId: (value: string | null) => {
      calls.push(`set-version:${value ?? ""}`);
    },
    studyKind: "truss_2d",
    updateModelVersion: async () => {},
    versionDeletedLabel: "version deleted",
    versionRenamedLabel: "version renamed",
  };
}

test("PWDT invoker preserves the canonical version history for real version write admission", async () => {
  const calls: string[] = [], args = baseArgs(calls);
  const invoke = createWorkbenchScriptInvoker({ ...args, selectedProjectId: "project-a", selectedModelId: "model-a",
    selectedVersionId: "version-a", recordSecurityAuditEvent: () => {}, appendScriptActionLog: () => {} });
  assert.equal((await invoke("model/renameSelectedVersion")).ok, true);
  assert.deepEqual(calls, ["update-version:version-a:model-a", "refresh-versions:model-a", "message:version renamed"]);
});

test("PWDT conflicting deletion cannot advance the intent or borrow the pending save's result", async () => {
  const calls: string[] = [], args = baseArgs(calls);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  args.projectLibraryBackendService.createModelVersion = async () => { await gate; return versionEnvelope("version-created"); };
  args.projectLibraryBackendService.deleteModel = async () => assert.fail("conflicting delete must not dispatch");
  const input = { ...args, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a" };
  const first = handleWorkbenchScriptProjectModelAction({ ...input, action: "model/save" });
  const ownsIntent = args.projectContext.capture();
  await assert.rejects(handleWorkbenchScriptProjectModelAction({ ...input, action: "model/deleteSelected" }), /WORKBENCH_LIBRARY_WRITE_PENDING/);
  assert.equal(ownsIntent(), true);
  release(); const outcome = await first;
  assert.equal(outcome?.ok, true); assert.equal(outcome?.contextChanged ?? false, false);
  assert.equal(calls.filter((call) => call.startsWith("set-version:")).length, 1);
});

test("script project action creates projects through project library service", async () => {
  const calls: string[] = [];
  const result = await handleWorkbenchScriptProjectModelAction({
    ...baseArgs(calls),
    action: "project/create",
    payload: { name: "Script Project" },
  });

  assert.deepEqual(result, { ok: true, action: "project/create", projectId: "project-created" });
  assert.deepEqual(calls, [
    "create-project:Script Project",
    "refresh-projects",
    "set-project:project-created",
    "set-model:",
    "set-version:",
    "message:project created",
  ]);
});

test("script model save creates models through project library service", async () => {
  const calls: string[] = [];
  const result = await handleWorkbenchScriptProjectModelAction({
    ...baseArgs(calls),
    action: "model/save",
    selectedProjectId: "project-a",
  });

  assert.deepEqual(result, { ok: true, action: "model/save", modelId: "model-created" });
  assert.deepEqual(calls, [
    "create-model:project-a:model-a",
    "refresh-projects",
    "set-model:model-created",
    "set-version:version-a",
    "message:model created",
    "refresh-versions:model-created",
  ]);
});

for (const action of ["model/save", "model/saveAs"]) {
  test(`script ${action} validates and forwards checkpoint request ids separately from geometry`, async () => {
    const calls: string[] = [], args = baseArgs(calls);
    for (const request_id of ["", "short", {}, 123, "a".repeat(129)]) {
      await assert.rejects(handleWorkbenchScriptProjectModelAction({
        ...args, action, payload: { request_id }, selectedProjectId: "project-a",
        serializeCurrentModel: () => assert.fail("invalid request ids must not read geometry"),
      }), /invalid_checkpoint_request_id/);
    }
    const requestId = "durable-checkpoint-key-0001";
    args.projectLibraryBackendService.createModel = async (_id, input) => {
      assert.equal(input.request_id, requestId);
      assert.equal(input.payload.request_id, undefined);
      return modelEnvelope("created");
    };
    args.projectLibraryBackendService.createModelVersion = async (_id, input) => {
      assert.equal(input.request_id, requestId);
      assert.equal(input.payload.request_id, undefined);
      return versionEnvelope("created");
    };
    assert.equal((await handleWorkbenchScriptProjectModelAction({
      ...args, action, payload: { request_id: requestId }, selectedProjectId: "project-a", selectedModelId: "model-a",
    }))?.ok, true);
  });

  test(`script ${action} accepts an atomic name payload and publishes the trimmed name only on success`, async () => {
    const calls: string[] = [], args = baseArgs(calls);
    await handleWorkbenchScriptProjectModelAction({
      ...args, action, payload: { name: "  Research model  " }, selectedProjectId: "project-a", selectedModelId: "model-a",
      setLoadedModelName: (name) => { calls.push(`local-name:${name}`); },
    });
    const writes = calls.filter((call) => /^(create|update)-/.test(call));
    assert.deepEqual(writes, action === "model/saveAs" ? ["create-model:project-a:Research model"]
      : ["create-version:model-a:Research model"]);
    assert.deepEqual(calls.filter((call) => call.startsWith("local-name:")), ["local-name:Research model"]);
  });

  test(`script ${action} rejects invalid names before serialization or writes`, async () => {
    for (const name of ["", "  ", 3, null, {}, "a".repeat(257)]) {
      const calls: string[] = [];
      await assert.rejects(handleWorkbenchScriptProjectModelAction({
        ...baseArgs(calls), action, payload: { name }, selectedProjectId: "project-a",
        serializeCurrentModel: () => assert.fail("must reject before reading the model"),
        setLoadedModelName: () => assert.fail("must not rename the live model"),
      }), /model:invalid_name/);
      assert.deepEqual(calls, []);
    }
  });

  test(`script ${action} preserves the local name on write failure and on changed project context`, async () => {
    for (const fail of [true, false]) {
      const calls: string[] = [], args = baseArgs(calls);
      if (fail) {
        args.projectLibraryBackendService.createModel = async () => { throw new Error("write unavailable"); };
        args.projectLibraryBackendService.createModelVersion = async () => { throw new Error("write unavailable"); };
      }
      const pending = handleWorkbenchScriptProjectModelAction({
        ...args, action, payload: { name: "Original project model" }, selectedProjectId: "project-a", selectedModelId: "model-a",
        setLoadedModelName: () => assert.fail("must not overwrite local name after failure or navigation"),
        refreshProjects: async () => { args.projectContext.update({ projectId: "project-b", modelId: null, versionId: null }); },
      });
      if (fail) await assert.rejects(pending, /write unavailable/);
      else assert.equal((await pending)?.contextChanged, true);
    }
  });
}

test("script checkpoint rejection cannot mutate the persisted model before its version exists", async () => {
  const calls: string[] = [], args = baseArgs(calls);
  args.projectLibraryBackendService.updateModel = async () => assert.fail("save must use the atomic checkpoint endpoint");
  args.projectLibraryBackendService.createModelVersion = async () => { throw new Error("checkpoint rejected"); };
  await assert.rejects(handleWorkbenchScriptProjectModelAction({
    ...args, action: "model/save", payload: { name: "Unsaved change" }, selectedProjectId: "project-a", selectedModelId: "model-a",
  }), /checkpoint rejected/);
  assert.deepEqual(calls, []);
});

for (const fail of [false, true]) {
  test(`script checkpoint keeps serialized and live metadata aligned without premature updates; failure=${fail}`, async () => {
    const calls: string[] = [], args = baseArgs(calls);
    const original = { name: "Old name", material: "Old material", nodes: [{ x: 1 }] };
    args.projectLibraryBackendService.createModelVersion = async (_id, input) => {
      assert.equal(input.name, "Saved name");
      assert.equal(input.material, "Saved material");
      assert.deepEqual(input.payload, { ...original, name: input.name, material: input.material });
      if (fail) throw new Error("checkpoint rejected");
      return versionEnvelope("version-created");
    };
    const pending = handleWorkbenchScriptProjectModelAction({
      ...args, action: "model/save", payload: { name: " Saved name ", material: " Saved material " },
      selectedProjectId: "project-a", selectedModelId: "model-a", serializeCurrentModel: () => original,
      setLoadedModelName: (name) => { calls.push(`local-name:${name}`); },
      setActiveMaterial: (material) => { calls.push(`local-material:${material}`); },
    });
    if (fail) await assert.rejects(pending, /checkpoint rejected/);
    else await pending;
    assert.deepEqual(calls.filter((call) => call.startsWith("local-")),
      fail ? [] : ["local-name:Saved name", "local-material:Saved material"]);
    assert.equal(original.name, "Old name");
    assert.equal(original.material, "Old material");
  });
}

test("script project exports propagate download failures instead of reporting false success", async () => {
  const calls: string[] = [];
  const failure = new Error("project bundle download failed");

  await assert.rejects(
    handleWorkbenchScriptProjectModelAction({
      ...baseArgs(calls),
      action: "project/exportJson",
      downloadProjectBundleJson: async () => ({ ok: false, error: failure }),
    }),
    failure,
  );
});

test("script project selection detaches saved associations without pretending to load geometry", async () => {
  const calls: string[] = [];
  const args = { ...baseArgs(calls), action: "project/select", selectedProjectId: "project-a", selectedModelId: "model-a" };
  await handleWorkbenchScriptProjectModelAction({ ...args, payload: { projectId: "project-a" } });
  assert.deepEqual(calls, ["set-model:", "set-version:", "set-project:project-a"]);
  calls.length = 0;
  await handleWorkbenchScriptProjectModelAction({ ...args, payload: { projectId: "project-b" } });
  assert.deepEqual(calls, ["set-model:", "set-version:", "set-project:project-b"]);
});

for (const admission of ["unmarked", "stale", "unreadable", "disposed"]) {
  test(`project selection refuses ${admission} catalog without changing selection or drafts`, async () => {
    const calls: string[] = [], args = baseArgs(calls);
    const source = workbenchLibraryRecordSources.sourceOf(args.projects)!, original = source.isCurrent;
    if (admission === "unmarked") args.projects = [...args.projects];
    if (admission === "stale") source.isCurrent = () => false;
    if (admission === "unreadable") source.isCurrent = () => { throw new Error("unreadable"); };
    if (admission === "disposed") args.projectContext.dispose();
    try {
      await assert.rejects(handleWorkbenchScriptProjectModelAction({ ...args, action: "project/select", payload: { projectId: "project-a" } }),
        admission === "disposed" ? /WORKBENCH_CONTEXT_CHANGED/ : /WORKBENCH_RECORD_SOURCE_CHANGED/);
      assert.equal(calls.length, admission === "disposed" ? 0 : 1);
      if (calls.length) assert.match(calls[0], /message:WORKBENCH_RECORD_SOURCE_CHANGED/);
    } finally { source.isCurrent = original; }
  });
}

for (const projectId of ["project-a", null]) {
  test(`project selection ${projectId ?? "none"} synchronously supersedes an older intent before React commits`, async () => {
    const calls: string[] = [], args = baseArgs(calls);
    args.projectContext.update({ projectId: "project-a", modelId: "model-a", versionId: "version-a" });
    const previous = args.projectContext.begin();
    const pending = handleWorkbenchScriptProjectModelAction({ ...args, action: "project/select", payload: { projectId } });
    assert.equal(previous(), false);
    assert.deepEqual(args.projectContext.current(), { projectId, modelId: null, versionId: null });
    assert.equal((await pending)?.ok, true);
  });
}

for (const payload of [{ loadedModelName: "Newer edit" }, { activeMaterial: "copper" }]) {
  test(`workspace metadata ${Object.keys(payload)[0]} supersedes a saved load synchronously`, async () => {
    const args = baseArgs([]), old = args.projectContext.begin();
    const result = handleWorkbenchScriptProjectModelAction({ ...args, action: "model/setWorkspaceMeta", payload });
    assert.equal(old(), false);
    assert.equal((await result)?.ok, true);
  });
}

for (const projectId of ["missing-project", "", 17]) {
  test(`script project selection rejects invalid project ${JSON.stringify(projectId)} without mutations`, async () => {
    const calls: string[] = [];
    await assert.rejects(handleWorkbenchScriptProjectModelAction({
      ...baseArgs(calls), action: "project/select", selectedProjectId: "project-a", payload: { projectId },
    }), /project required/u);
    assert.deepEqual(calls, []);
  });
}

test("script save-as restores the newly created model after refreshing an existing project", async () => {
  const calls: string[] = [];
  const args = baseArgs(calls);
  await handleWorkbenchScriptProjectModelAction({
    ...args, action: "model/saveAs", selectedProjectId: "project-a", selectedModelId: "old-model",
    refreshProjects: async () => { args.setSelectedModelId("old-model"); },
  });
  assert.deepEqual(calls.filter((entry) => entry.startsWith("set-model:")), ["set-model:old-model", "set-model:model-created"]);
});

for (const action of ["model/save", "model/saveAs"]) {
  test(`script ${action} reports a completed write without applying it to a changed workspace`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a",
      refreshProjects: async (_bootstrap, _preferred, options) => {
        assert.equal(options?.preserveSelection, true);
        args.projectContext.update({ projectId: "project-b", modelId: null, versionId: null });
      },
    });
    assert.equal(result?.ok, true);
    assert.equal(result?.contextChanged, true);
    assert.ok(calls.some((entry) => entry.startsWith("create-")), "the original backend write is retained");
    assert.deepEqual(calls.filter((entry) => /^(set-|message:|refresh-versions:)/u.test(entry)), []);
  });
}

const savedSelection = { projectId: "project-a", modelId: "model-a", versionId: "version-a" };

for (const action of ["model/save", "model/saveAs"]) {
for (const moved of [false, true]) {
  test(`script ${action} completion tracks the final version refresh with navigation=${moved}`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    args.projectContext.update(savedSelection);
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a",
      refreshVersions: async () => {
        const ownSelection = { ...savedSelection,
          modelId: action === "model/saveAs" ? "model-created" : "model-a",
          versionId: action === "model/saveAs" ? "version-a" : "version-created" };
        args.projectContext.update(ownSelection);
        if (moved) {
          args.projectContext.update({ projectId: "project-b", modelId: null, versionId: null });
          args.projectContext.update(ownSelection);
        }
      },
    });
    assert.equal(result?.ok, true);
    assert.equal(result?.contextChanged ?? false, moved);
    assert.equal(result?.[action === "model/saveAs" ? "modelId" : "versionId"],
      action === "model/saveAs" ? "model-created" : "version-created");
  });
}
}

for (const action of ["project/create", "project/updateSelected", "project/deleteSelected",
  "model/deleteSelected", "model/renameSelectedVersion", "model/deleteSelectedVersion"]) {
  test(`script ${action} completes its original write without mutating a newer saved workspace`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    args.projectContext.update(savedSelection);
    const changeWorkspace = () => args.projectContext.update({ projectId: "project-b", modelId: "model-b", versionId: "version-b" });
    const updateVersion = args.projectLibraryBackendService.updateModelVersion;
    args.projectLibraryBackendService.updateModelVersion = async (...inputs) => {
      const response = await updateVersion(...inputs);
      changeWorkspace();
      return response;
    };
    const deleteProject = args.projectLibraryBackendService.deleteProject;
    args.projectLibraryBackendService.deleteProject = async (id) => {
      const response = await deleteProject(id); changeWorkspace(); return response;
    };
    const deleteModel = args.projectLibraryBackendService.deleteModel;
    args.projectLibraryBackendService.deleteModel = async (id) => {
      const response = await deleteModel(id); changeWorkspace(); return response;
    };
    const deleteVersion = args.projectLibraryBackendService.deleteModelVersion;
    args.projectLibraryBackendService.deleteModelVersion = async (id) => {
      const response = await deleteVersion(id); changeWorkspace(); return response;
    };
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a",
      setProjectNameDraft: () => { calls.push("set-name"); },
      setProjectDescriptionDraft: () => { calls.push("set-description"); },
      setModelVersions: () => { calls.push("set-versions"); },
      refreshProjects: async (_bootstrap, _preferred, options) => {
        assert.equal(options?.preserveSelection, true);
        changeWorkspace();
      },
    });
    assert.equal(result?.ok, true);
    assert.equal(result?.contextChanged, true);
    assert.equal(calls.filter((entry) => /^(create-|update-|delete-)/u.test(entry)).length, 1);
    assert.deepEqual(calls.filter((entry) => /^(set-|message:|refresh-versions:)/u.test(entry)), []);
  });
}

for (const action of ["project/deleteSelected", "model/deleteSelected", "model/deleteSelectedVersion"]) {
  test(`script ${action} detaches on confirmation but cannot replace feedback after navigation during refresh`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    args.projectContext.update(savedSelection);
    let afterConfirmation: string[] = [];
    const newer = { projectId: "project-b", modelId: "model-b", versionId: "version-b" };
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a",
      refreshProjects: async () => {
        afterConfirmation = [...calls];
        assert.equal(args.projectContext.current().versionId, null);
        args.projectContext.update(newer);
      },
    });
    assert.equal(result?.ok, true);
    assert.equal(result?.contextChanged, true);
    assert.deepEqual(args.projectContext.current(), newer);
    assert.deepEqual(calls, afterConfirmation, "confirmation cleanup precedes the read; nothing overwrites the newer workspace afterward");
    assert.equal(calls.some((entry) => entry.startsWith("message:")), false);
  });
}

for (const action of ["project/updateSelected", "project/deleteSelected", "model/save", "model/saveAs",
  "model/deleteSelected", "model/renameSelectedVersion", "model/deleteSelectedVersion"]) {
  test(`PWDT ${action} stops after confirmation if the catalog source changed, without cleanup or recovery reads`, async () => {
    const calls: string[] = [], args = baseArgs(calls);
    const source = action.includes("Version") ? workbenchLibraryVersionSources.sourceOf(args.modelVersions)!
      : workbenchLibraryRecordSources.sourceOf(args.projects)!;
    const original = source.isCurrent;
    const service = args.projectLibraryBackendService;
    for (const method of ["updateProject", "deleteProject", "createModel", "createModelVersion", "deleteModel", "updateModelVersion", "deleteModelVersion"] as const) {
      const write = service[method] as (...args: any[]) => Promise<any>;
      service[method] = (async (...inputs: any[]) => { const response = await write(...inputs); source.isCurrent = () => false; return response; }) as never;
    }
    try {
      const result = await handleWorkbenchScriptProjectModelAction({ ...args, action, selectedProjectId: "project-a",
        selectedModelId: "model-a", selectedVersionId: "version-a", removeConfirmedLibraryRecord: () => assert.fail("new-source records must not be removed") });
      assert.equal(result?.ok, true); assert.equal(result?.contextChanged, true);
      assert.equal(calls.length, 1); assert.match(calls[0], /^(create|delete|update)-/);
      assert.deepEqual(args.projectContext.current(), savedSelection);
    } finally { source.isCurrent = original; }
  });
}

for (const action of ["project/deleteSelected", "model/deleteSelected", "model/deleteSelectedVersion"]) {
  test(`script ${action} reconciles current deleted bindings without selecting another saved record`, async () => {
    const calls: string[] = [];
    const args = baseArgs(calls);
    args.projectContext.update(savedSelection);
    const result = await handleWorkbenchScriptProjectModelAction({
      ...args, action, selectedProjectId: "project-a", selectedModelId: "model-a", selectedVersionId: "version-a",
      refreshProjects: async (_bootstrap, _preferred, options) => { assert.equal(options?.preserveSelection, true); },
    });
    assert.deepEqual(result, { ok: true, action });
    const expected = action === "project/deleteSelected" ? ["set-project:", "set-model:", "set-version:"]
      : action === "model/deleteSelected" ? ["set-model:", "set-version:"] : ["set-version:"];
    assert.deepEqual(calls.filter((entry) => entry.startsWith("set-")), expected);
    assert.equal(calls.filter((entry) => entry.startsWith("message:")).length, 1);
  });
}
