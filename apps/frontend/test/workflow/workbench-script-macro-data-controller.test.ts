import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptMacroDataAction } from "../../src/components/workbench/workbench-script-macro-data-controller.ts";
import type { JobState } from "../../src/lib/api/fem-shared.ts";

type MacroDataArgs = Parameters<typeof handleWorkbenchScriptMacroDataAction>[0];

function baseArgs(): MacroDataArgs {
  return {
    action: "data/exportDatabase",
    applyJobContextToWorkbench: async () => ({ ok: true }),
    downloadDatabaseSnapshot: async () => ({ ok: true }),
    getScriptSnapshot: () => ({} as ReturnType<MacroDataArgs["getScriptSnapshot"]>),
    invokeScriptAction: async () => ({}),
    language: "en",
    openModelVersionById: async () => ({ ok: true }),
    openProjectContextById: async () => ({ ok: true }),
    payload: {},
    resolveScriptLinkedJob: () => null,
    setAdminFilterModelVersionId: () => {},
    setAdminFilterProjectId: () => {},
    setSelectedAdminJobId: () => {},
    setSelectedAdminResultJobId: () => {},
    setSidebarSection: () => {},
    setSystemDataTab: () => {},
    setSystemPanelTab: () => {},
    source: "script",
  };
}

test("script database export reports a completed download", async () => {
  assert.deepEqual(await handleWorkbenchScriptMacroDataAction(baseArgs()), {
    ok: true,
    action: "data/exportDatabase",
  });
});

test("script database export propagates download failures", async () => {
  const failure = new Error("database download failed");
  await assert.rejects(
    handleWorkbenchScriptMacroDataAction({
      ...baseArgs(),
      downloadDatabaseSnapshot: async () => ({ ok: false, error: failure }),
    }),
    failure,
  );
});

function linkedJob(): JobState {
  return {
    has_result: true,
    job_id: "job-linked",
    model_version_id: "version-linked",
    progress: 1,
    project_id: "project-linked",
    status: "completed",
    worker_id: "worker-linked",
  };
}

test("script linked-context navigation waits for version loading", async () => {
  let release!: () => void;
  let settled = false;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const operation = handleWorkbenchScriptMacroDataAction({
    ...baseArgs(),
    action: "data/openLinkedContext",
    openModelVersionById: async () => {
      await gate;
      return { ok: true };
    },
    payload: { mode: "version" },
    resolveScriptLinkedJob: linkedJob,
  });
  void operation.then(() => { settled = true; });

  await Promise.resolve();
  assert.equal(settled, false);
  release();
  const result = await operation;
  assert.equal(result?.ok, true);
  assert.equal(result?.mode, "version");
});

test("script linked-context navigation propagates version loading failures", async () => {
  const failure = new Error("version loading failed");
  await assert.rejects(
    handleWorkbenchScriptMacroDataAction({
      ...baseArgs(),
      action: "data/openLinkedContext",
      openModelVersionById: async () => ({ ok: false, error: failure }),
      payload: { mode: "version" },
      resolveScriptLinkedJob: linkedJob,
    }),
    failure,
  );
});

test("script macros stop remaining steps after an operation reports a changed workspace", async () => {
  const calls: string[] = [];
  await assert.rejects(handleWorkbenchScriptMacroDataAction({
    ...baseArgs(), action: "macro/run", payload: { macroId: "macro/openProjectLibrary" },
    invokeScriptAction: async (action) => {
      calls.push(action);
      return { ok: true, contextChanged: true };
    },
  }), /WORKBENCH_CONTEXT_CHANGED/u);
  assert.deepEqual(calls, ["nav/setSidebarSection"]);
});
