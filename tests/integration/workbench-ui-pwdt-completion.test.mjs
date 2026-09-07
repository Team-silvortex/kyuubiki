import assert from "node:assert/strict";
import { test } from "node:test";
import {
  PROJECT_ID, runtime, initialProject, usingWorkbench, invoke, openWorkbench,
  holdRequest, installProjectWorkbenchTestHooks,
} from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

for (const action of ["model/save", "model/saveAs"]) {
for (const navigation of ["stay", "switch", "away-and-back"]) {
test(`Workbench PWDT ${action} reports final-refresh context for ${navigation}`, { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Original research" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    const pending = await holdRequest(page, "/api/v1/models/*/versions");
    const saving = invoke(page, action).catch((error) => ({ error: error.message }));
    try {
      await pending.received;
      const persistedModel = library.models.at(-1);
      const persistedVersion = library.versions.at(-1);
      await page.evaluate((expected) => window.__kyuubikiPwdt.waitForState(expected), {
        selectedModelId: persistedModel.model_id, selectedVersionId: persistedVersion.version_id,
      });
      if (navigation !== "stay") {
        await invoke(page, "project/select", { projectId: "second-project" });
        if (navigation === "away-and-back") await invoke(page, "project/select", { projectId: PROJECT_ID });
        await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      }
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      pending.release();
      const result = await saving;
      assert.equal(result.ok, true);
      assert.equal(result.contextChanged ?? false, navigation !== "stay");
      assert.equal(result[action === "model/saveAs" ? "modelId" : "versionId"],
        action === "model/saveAs" ? persistedModel.model_id : persistedVersion.version_id);
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "message"]) {
        assert.equal(after[key], before[key]);
      }
      assert.equal(persistedModel.project_id, PROJECT_ID);
    } finally { pending.release(); await saving; }
  });
});
}
}

for (const navigation of ["stay", "switch", "away-and-back"]) {
test(`Workbench PWDT linked version load reports final-refresh context for ${navigation}`, { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Linked version" }));
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
    const versionId = library.versions[0].version_id;
    runtime.state.adminJobs.push({ job_id: "linked-completed-job", project_id: PROJECT_ID, model_version_id: versionId,
      status: "completed", progress: 1, has_result: false, created_at: initialProject.inserted_at, updated_at: initialProject.updated_at });
    await page.evaluate(() => window.__kyuubikiPwdt.refreshAll());
    await invoke(page, "project/select", { projectId: "second-project" });
    const pending = await holdRequest(page, "/api/v1/models/*/versions");
    const loading = invoke(page, "data/openLinkedContext", { mode: "version", jobId: "linked-completed-job" })
      .catch((error) => ({ error: error.message }));
    try {
      await pending.received;
      await page.evaluate((selectedVersionId) => window.__kyuubikiPwdt.waitForState({ selectedVersionId }), versionId);
      if (navigation !== "stay") {
        await invoke(page, "project/select", { projectId: "second-project" });
        if (navigation === "away-and-back") await invoke(page, "project/select", { projectId: PROJECT_ID });
        await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      }
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      pending.release();
      const result = await loading;
      if (navigation === "stay") assert.equal(result.ok, true);
      else assert.match(result.error ?? "", /WORKBENCH_CONTEXT_CHANGED/u);
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      for (const key of ["selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "message"]) {
        assert.equal(after[key], before[key]);
      }
    } finally { pending.release(); await loading; }
  });
});
}

for (const sequence of ["steps", "recipe"]) {
for (const waitingOn of ["write", "version-list"]) {
test(`Workbench PWDT ${sequence} cannot submit after a workspace switch during ${waitingOn}`, { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", name: "Second project" });
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Recipe research" }));
    let submissions = 0;
    await page.route("**/api/v1/fem/**/jobs", async (route) => {
      submissions += 1;
      await route.fulfill({ status: 503, json: { error: "unexpected submission from stale sequence" } });
    });
    const pending = await holdRequest(page, waitingOn === "write" ? `/api/v1/projects/${PROJECT_ID}/models` : "/api/v1/models/*/versions",
      waitingOn === "write" ? "POST" : "GET");
    const operation = page.evaluate((mode) => mode === "steps"
      ? window.__kyuubikiPwdt.runSteps([{ action: "model/saveAs" }, { action: "job/run" }])
      : window.__kyuubikiPwdt.runRecipe("recipe/truss2d/closed-loop", { modelName: "Recipe research" }), sequence)
      .catch((error) => ({ error: error.message }));
    try {
      await pending.received;
      await invoke(page, "project/select", { projectId: "second-project" });
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      pending.release();
      const result = await operation;
      assert.match(result.error ?? "", /WORKBENCH_CONTEXT_CHANGED/u);
      assert.equal(submissions, 0, "a stopped sequence must not dispatch a solver job");
      assert.equal(library.models.length, 1, "the completed write is retained without duplicate retries");
      assert.equal(library.models[0].project_id, PROJECT_ID);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().loadedModelName), "Current research");
    } finally { pending.release(); await operation; }
  });
});
}
}
