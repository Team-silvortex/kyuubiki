import test from "node:test";
import assert from "node:assert/strict";

import { handleWorkbenchScriptMacroDataAction } from "../../src/components/workbench/workbench-script-macro-data-controller.ts";

type MacroDataArgs = Parameters<typeof handleWorkbenchScriptMacroDataAction>[0];

function baseArgs(): MacroDataArgs {
  return {
    action: "data/exportDatabase",
    downloadDatabaseSnapshot: async () => ({ ok: true }),
    getScriptSnapshot: () => ({} as ReturnType<MacroDataArgs["getScriptSnapshot"]>),
    invokeScriptAction: async () => ({}),
    language: "en",
    openLinkedContext: async () => ({ ok: true, jobId: "job-linked", projectId: "project-linked", modelVersionId: "version-linked" }),
    payload: {},
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

test("script linked-context navigation waits for version loading", async () => {
  let release!: () => void;
  let settled = false;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const operation = handleWorkbenchScriptMacroDataAction({
    ...baseArgs(),
    action: "data/openLinkedContext",
    openLinkedContext: async () => {
      await gate;
      return { ok: true, jobId: "job-linked", projectId: "project-linked", modelVersionId: "version-linked" };
    },
    payload: { mode: "version" },
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
      openLinkedContext: async () => ({ ok: false, error: failure }),
      payload: { mode: "version" },
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

for (const mode of ["project", "version", "apply"]) {
  test(`PWDT linked ${mode} forwards original record identity to the single guarded service`, async () => {
    const payload = { target: "result", resultJobId: "authoritative-result", mode };
    const calls: Record<string, unknown>[] = [];
    const result = await handleWorkbenchScriptMacroDataAction({ ...baseArgs(), action: "data/openLinkedContext", payload,
      openLinkedContext: async (input) => { calls.push(input); return { ok: true, jobId: "authoritative-result", projectId: "project-a", modelVersionId: "version-a" }; },
    });
    assert.deepEqual(calls, [payload]);
    assert.equal(result?.jobId, "authoritative-result");
    assert.equal(result?.projectId, "project-a");
  });
}
