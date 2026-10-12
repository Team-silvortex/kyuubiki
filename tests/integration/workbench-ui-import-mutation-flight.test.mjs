import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, usingWorkbench, openWorkbench, invoke, exportProject, holdRequest,
  installProjectWorkbenchTestHooks, waitForGuiTransition, openProjectManager, openSavedModels,
} from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
async function seed(page) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Import admission research" }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ saveAs: true }));
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Second version" }));
  return exportProject(page);
}
async function startImport(page, bundle) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "projects" }));
  await page.locator('[data-workbench-library-project-page="exchange"]').click();
  const input = page.locator('[data-workbench-library-project-action="import"]');
  await input.setInputFiles({ name: "same-file.kyuubiki.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(bundle)) });
  assert.equal(await input.inputValue(), "", "explicit retry must be able to select the same file again");
}
function trackWrites(page) {
  const writes = [];
  page.on("request", (request) => {
    const path = new URL(request.url()).pathname;
    if (["POST", "PATCH", "DELETE"].includes(request.method()) && /^\/api\/v1\/(projects|models|model-versions)(\/|$)/.test(path)) {
      writes.push({ path, method: request.method() });
    }
  });
  return writes;
}
const phases = [
  { name: "project", path: "/api/v1/projects", method: "POST" },
  { name: "model", path: "/api/v1/projects/qualification-created-project-1/models", method: "POST" },
  { name: "version", path: "/api/v1/models/boundary-model-2/versions", method: "POST" },
  { name: "catalog", path: "/api/v1/projects", method: "GET" },
  { name: "history", path: "/api/v1/models/boundary-model-2/versions", method: "GET" },
];
for (const phase of phases) for (const failed of [false, true]) {
  test(`Workbench overlapping import is rejected through ${phase.name}; failure=${failed}, explicit later retry remains available`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const bundle = await seed(page), writes = trackWrites(page);
      const held = await holdRequest(page, phase.path, phase.method, failed
        ? (route) => route.fulfill({ status: 503, json: { error: "held import phase failed" } })
        : (route) => route.fallback());
      try {
        await startImport(page, bundle); await held.received; await waitForGuiTransition(page, true);
        const before = structuredClone(writes);
        await startImport(page, bundle);
        await page.waitForFunction(() => /WORKBENCH_LIBRARY_WRITE_PENDING/.test(window.__kyuubikiPwdt.state().message));
        assert.deepEqual(writes, before, "the second import cannot dispatch any write");
        held.release(); await waitForGuiTransition(page);
        assert.equal(writes.filter((entry) => entry.path === "/api/v1/projects" && entry.method === "POST").length, 1,
          "blocked import is not queued behind the first");
        if (!failed || phase.method === "GET") {
          assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).message, "Project bundle imported.",
            "busy rejection cannot cancel the original import's intent");
        }
        await page.unroute(`**${phase.path}`);
        const counts = { projects: runtime.state.projectMutations.length, models: library.models.length, versions: library.versions.length };
        const retry = await holdRequest(page, "/api/v1/projects", "POST");
        try {
          await startImport(page, bundle); await retry.received; await waitForGuiTransition(page, true);
          retry.release(); await waitForGuiTransition(page);
          assert.equal(runtime.state.projectMutations.length, counts.projects + 1);
          assert.equal(library.models.length, counts.models + 1); assert.equal(library.versions.length, counts.versions + 2);
          assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedModelId, library.models.at(-1).model_id);
        } finally { retry.release(); }
      } finally { held.release(); }
    });
  });
}

for (const surface of ["GUI", "PWDT"]) for (const action of ["project/deleteSelected", "model/deleteSelected", "model/renameSelectedVersion"]) {
  test(`Workbench pending import history excludes ${surface} ${action} on the imported project`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const bundle = await seed(page), writes = trackWrites(page);
      page.on("dialog", (dialog) => dialog.accept());
      const held = await holdRequest(page, "/api/v1/models/boundary-model-2/versions", "GET");
      try {
        await startImport(page, bundle); await held.received; await waitForGuiTransition(page, true);
        const before = structuredClone(writes);
        if (surface === "PWDT") await assert.rejects(invoke(page, action), /WORKBENCH_LIBRARY_WRITE_PENDING/);
        else {
          if (action.startsWith("project/")) {
            await openProjectManager(page);
            await page.locator('[data-workbench-library-project-action="delete"]').click();
          } else {
            await openSavedModels(page);
            if (action === "model/renameSelectedVersion") await page.locator('[data-workbench-library-model-page="versions"]').click();
            await page.getByRole("button", { name: action === "model/deleteSelected" ? "Delete model" : "Rename version", exact: true }).click();
          }
          await page.waitForFunction(() => /WORKBENCH_LIBRARY_WRITE_PENDING/.test(window.__kyuubikiPwdt.state().message));
        }
        assert.deepEqual(writes, before, "import owns its project until final recovery settles");
        held.release(); await waitForGuiTransition(page);
        assert.deepEqual(writes, before, "blocked mutation is never replayed");
        assert.equal(library.models.length, 2); assert.equal(library.versions.length, 4);
        assert.equal((await invoke(page, "model/renameSelectedVersion", { name: "Explicit later edit" })).ok, true);
        assert.equal(writes.length, before.length + 1);
      } finally { held.release(); }
    });
  });
}

test("Workbench import reservation leaves unrelated project editing available during writes", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const bundle = await seed(page), held = await holdRequest(page, phases[1].path, "POST");
    try {
      await startImport(page, bundle); await held.received; await waitForGuiTransition(page, true);
      assert.equal((await invoke(page, "project/updateSelected", { name: "Independent original project" })).ok, true);
      held.release(); await waitForGuiTransition(page);
      assert.equal(runtime.state.projects[0].name, "Independent original project");
      assert.equal(runtime.state.projectMutations.filter((entry) => entry.method === "POST").length, 1);
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedProjectId, runtime.state.projects[0].project_id);
    } finally { held.release(); }
  });
});

for (const surface of ["GUI", "PWDT"]) {
  test(`Workbench ${surface} cannot delete a partially imported project after explicit catalog discovery`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const bundle = await seed(page), held = await holdRequest(page, phases[1].path, "POST");
      page.on("dialog", (dialog) => dialog.accept());
      try {
        await startImport(page, bundle); await held.received; await waitForGuiTransition(page, true);
        await invoke(page, "runtime/refreshAll");
        await invoke(page, "project/select", { projectId: "qualification-created-project-1" });
        if (surface === "PWDT") await assert.rejects(invoke(page, "project/deleteSelected"), /WORKBENCH_LIBRARY_WRITE_PENDING/);
        else {
          await openProjectManager(page);
          await page.locator('[data-workbench-library-project-action="delete"]').click();
          await page.waitForFunction(() => /WORKBENCH_LIBRARY_WRITE_PENDING/.test(window.__kyuubikiPwdt.state().message));
        }
        assert.equal(runtime.state.projectMutations.filter((entry) => entry.method === "DELETE").length, 0);
        held.release(); await waitForGuiTransition(page);
        assert.equal(library.models.length, 2); assert.equal(library.versions.length, 4);
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.selectedProjectId, "qualification-created-project-1");
        assert.equal(state.selectedModelId, null, "accepted import cannot undo a newer project-only navigation");
        assert.equal((await invoke(page, "project/deleteSelected")).ok, true);
        assert.equal(runtime.state.projectMutations.filter((entry) => entry.method === "DELETE").length, 1);
      } finally { held.release(); }
    });
  });
}
