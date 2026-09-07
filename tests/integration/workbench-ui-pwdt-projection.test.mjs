import assert from "node:assert/strict";
import { test } from "node:test";
import {
  usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks,
  holdRequest, openSavedModels, exportProject, runtime, initialProject, PROJECT_ID,
} from "./workbench-ui-project-fixture.shared.mjs";
import { installProjectionSolver, openProjectionButton } from "./workbench-ui-pwdt-projection-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function historyAction(page, action, surface = "pwdt") {
  if (surface === "pwdt") return invoke(page, `history/${action}`);
  await page.locator('[data-workbench-inspector-tab-target="actions"]').click();
  await page.locator('[data-workbench-inspector-actions-target="history"]').click();
  await page.locator(`[data-workbench-history-action="${action}"]`).click();
}

async function editSavedTruss(page, model) {
  const payload = structuredClone(model.payload);
  payload.nodes[1].load_y = (payload.nodes[1].load_y ?? 0) - 123;
  await invoke(page, "state/replaceTruss2dModel", payload);
}

for (const kind of ["bar_1d", "plane_triangle_2d", "plane_quad_2d"]) {
  for (const failure of ["conductivity", "heat_load", "boundary"]) {
    test(`Workbench stale heat ${kind} ${failure} stops projection and recovers after a fresh solve`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        const solver = await installProjectionSolver(page, { heatFailure: failure });
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: `heat_${kind}` });
        await invoke(page, "model/saveAs");
        const sourceRecord = structuredClone(library.models[0]);
        await invoke(page, "job/run");
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        await assert.rejects(invoke(page, "state/projectHeatToThermo"), /HEAT_THERMO_PROJECTION_INVALID/u);
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        for (const key of ["studyKind", "hasResult", "selectedModelId", "selectedVersionId"]) assert.equal(after[key], before[key]);
        assert.equal(solver.submissions.length, 1);
        assert.equal(library.models.length, 1);
        solver.heatFailure = null;
        await invoke(page, "job/run");
        await invoke(page, "state/projectHeatToThermo");
        await invoke(page, "model/save");
        await invoke(page, "job/run");
        assert.equal(solver.submissions.at(-1).kind, `thermal_${kind}`);
        assert.equal(library.models.length, 2);
        assert.deepEqual(library.models[0], sourceRecord);
      });
    });
  }

  for (const surface of ["pwdt", "gui"]) {
    test(`Workbench ${surface} projection history ${kind} cannot save heat content over its thermo model`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        await installProjectionSolver(page);
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: `heat_${kind}` });
        await invoke(page, "model/saveAs");
        await invoke(page, "job/run");
        await invoke(page, "state/projectHeatToThermo");
        await invoke(page, "model/save");
        const saved = structuredClone(library.models);
        await historyAction(page, "undo", surface);
        await page.evaluate((studyKind) => window.__kyuubikiPwdt.waitForState({ studyKind }), `heat_${kind}`);
        const undone = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(undone.studyKind, `heat_${kind}`);
        assert.equal(undone.hasResult, false);
        assert.equal(undone.selectedModelId, null, "undo must detach the thermo save destination");
        assert.equal(undone.selectedVersionId, null);
        await invoke(page, "model/save");
        assert.equal(library.models.length, 3);
        assert.deepEqual(library.models.slice(0, 2), saved);
        await historyAction(page, "redo", surface);
        await page.evaluate((studyKind) => window.__kyuubikiPwdt.waitForState({ studyKind }), `thermal_${kind}`);
        const redone = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(redone.studyKind, `thermal_${kind}`);
        assert.equal(redone.selectedModelId, null, "redo must not keep the newly saved heat destination");
        await invoke(page, "model/save");
        assert.equal(library.models.length, 4);
        assert.deepEqual(library.models.slice(0, 2), saved);
      });
    });
  }
}

for (const differentProject of [false, true]) {
  test(`Workbench undo of another ${differentProject ? "project" : "model"} cannot adopt its current save binding`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      if (differentProject) runtime.state.projects.push({ ...structuredClone(initialProject), project_id: "second-project", models: [] });
      await openWorkbench(page);
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Original research" }));
      await invoke(page, "model/saveAs");
      const original = structuredClone(library.models[0]);
      if (differentProject) await invoke(page, "project/select", { projectId: "second-project" });
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Current research" }));
      await invoke(page, "model/saveAs");
      const current = structuredClone(library.models[1]);
      await invoke(page, "history/undo");
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.studyKind, "truss_2d");
      assert.equal(state.selectedProjectId, differentProject ? "second-project" : PROJECT_ID);
      assert.equal(state.selectedModelId, null);
      await invoke(page, "model/save");
      assert.equal(library.models.length, 3);
      assert.deepEqual(library.models.slice(0, 2), [original, current]);
      assert.deepEqual(library.models[2].payload.nodes, original.payload.nodes);
    });
  });
}

test("Workbench undo within the same saved model appends a version rather than creating a duplicate", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
    await invoke(page, "model/saveAs");
    const firstVersion = structuredClone(library.versions[0]);
    await editSavedTruss(page, library.models[0]);
    await invoke(page, "model/save");
    await invoke(page, "history/undo");
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), library.models[0].model_id);
    await invoke(page, "model/save");
    assert.equal(library.models.length, 1);
    assert.equal(library.versions.length, 3);
    assert.deepEqual(library.versions[0], firstVersion);
    assert.deepEqual(library.versions[2].payload.nodes, firstVersion.payload.nodes);
  });
});

for (const surface of ["pwdt", "gui"]) {
  test(`Workbench ${surface} undo invalidates a pending save even when the saved binding stays the same`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await openWorkbench(page);
      await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3 }));
      await invoke(page, "model/saveAs");
      const original = structuredClone(library.versions[0]);
      await editSavedTruss(page, library.models[0]);
      const pending = await holdRequest(page, `/api/v1/models/${library.models[0].model_id}/versions`, "POST");
      const saving = invoke(page, "model/save").catch((error) => ({ error: error.message }));
      try {
        await pending.received;
        await historyAction(page, "undo", surface);
        const restored = (await exportProject(page)).workspace_snapshot;
        assert.deepEqual(restored.nodes, original.payload.nodes);
        pending.release();
        const outcome = await saving;
        assert.equal(outcome.ok, true);
        assert.equal(outcome.contextChanged, true);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedVersionId), original.version_id);
        assert.equal(library.versions.length, 2, "an already-started save is retained, not rolled back");
        assert.deepEqual((await exportProject(page)).workspace_snapshot.nodes, original.payload.nodes);
      } finally { pending.release(); await saving; }
    });
  });
}

test("Workbench undo invalidates an older model load without changing the retained save binding", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Earlier model" }));
    await invoke(page, "model/saveAs");
    await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 5, modelName: "Later model" }));
    await invoke(page, "model/saveAs");
    const kept = structuredClone(library.models[1]);
    await editSavedTruss(page, library.models[1]);
    await openSavedModels(page);
    const path = `/api/v1/models/${library.models[0].model_id}`;
    const pending = await holdRequest(page, path);
    try {
      await page.locator("button.history-item").filter({ hasText: "Earlier model" }).click();
      await pending.received;
      await invoke(page, "history/undo");
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const finished = page.waitForResponse((response) => new URL(response.url()).pathname === path);
      pending.release();
      await finished;
      await page.waitForLoadState("networkidle");
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(after.selectedModelId, kept.model_id);
      assert.equal(after.message, before.message);
      assert.deepEqual((await exportProject(page)).workspace_snapshot.nodes, kept.payload.nodes);
    } finally { pending.release(); }
  });
});

for (const phase of ["submission", "poll"]) {
  test(`Workbench undo ignores a late heat ${phase} response instead of restoring the old active result`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const solver = await installProjectionSolver(page);
      await openWorkbench(page);
      await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
      await invoke(page, "model/saveAs");
      const path = phase === "submission" ? "/api/v1/fem/heat-plane-quad-2d/jobs" : "/api/v1/jobs/projection-job-1";
      let pending;
      let pollPrepared;
      const prepared = new Promise((resolve) => { pollPrepared = resolve; });
      if (phase === "submission") pending = await holdRequest(page, path, "POST");
      else {
        solver.beforeJobResponse = async () => {
          pending = await holdRequest(page, path);
          pollPrepared();
        };
      }
      const running = invoke(page, "job/run").catch((error) => ({ error: error.message }));
      try {
        if (phase === "poll") await prepared;
        await pending.received;
        await invoke(page, "history/undo");
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(before.studyKind, "axial_bar_1d");
        pending.release();
        const outcome = await running;
        assert.match(outcome.error, /superseded|WORKBENCH_CONTEXT_CHANGED/u);
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        for (const key of ["studyKind", "hasResult", "jobStatus", "selectedModelId", "selectedVersionId", "message"]) assert.equal(after[key], before[key]);
        assert.equal(after.hasResult, false);
        assert.equal(library.models.length, 1);
        assert.equal(solver.submissions.length, 1, "undo stops observation, not an already-started backend submission");
      } finally { pending?.release(); await running; }
    });
  });
}

for (const kind of ["bar_1d", "plane_triangle_2d", "plane_quad_2d"]) {
  for (const surface of ["pwdt", "gui"]) {
    test(`Workbench ${surface} projects heat ${kind}, saves a separate model, and runs thermo`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        const solver = await installProjectionSolver(page);
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: `heat_${kind}` });
        await invoke(page, "model/setWorkspaceMeta", { loadedModelName: "Source heat" });
        const saved = await invoke(page, "model/saveAs");
        const sourceRecord = structuredClone(library.models[0]);
        const polygons = page.locator('.plane-triangle:not(.plane-triangle--deformed)');
        const sourceGeometry = kind === "bar_1d" ? null
          : await polygons.evaluateAll((elements) => elements.map((element) => element.getAttribute("points")));
        if (sourceGeometry) assert.ok(sourceGeometry.length > 0);
        await invoke(page, "job/run");
        if (sourceGeometry) {
          assert.deepEqual(await polygons.evaluateAll((elements) => elements.map((element) => element.getAttribute("points"))),
            sourceGeometry, "out-of-order result nodes must not change the rendered mesh");
        }
        if (surface === "pwdt") await invoke(page, "state/projectHeatToThermo");
        else await (await openProjectionButton(page)).click();
        await page.waitForFunction((expected) => window.__kyuubikiPwdt.state().studyKind === expected, `thermal_${kind}`);
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.hasResult, false);
        assert.equal(state.selectedModelId, null, "projection must not overwrite its source model on save");
        await invoke(page, "model/save");
        assert.equal(library.models.length, 2);
        assert.deepEqual(library.models.find((entry) => entry.model_id === saved.modelId), sourceRecord);
        await invoke(page, "job/run");
        assert.equal(solver.submissions.length, 2);
        const source = solver.submissions[0].input;
        const target = solver.submissions[1].input;
        assert.equal(solver.submissions[1].kind, `thermal_${kind}`);
        assert.deepEqual(target.nodes.map((node) => node.temperature_delta), source.nodes.map((_, index) => 80 - index * 10));
        assert.deepEqual(target.nodes.map(({ id, x, y }) => ({ id, x, y })), source.nodes.map(({ id, x, y }) => ({ id, x, y })));
        assert.notEqual(target.model_version_id, source.model_version_id);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), true);
      });
    });
  }
}

for (const kind of ["triangle", "quad"]) {
  for (const electrostatic of [false, true]) {
    test(`Workbench PWDT ${electrostatic ? "electrostatic-" : ""}heat-thermo ${kind} recipe closes every stage`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        const solver = await installProjectionSolver(page);
        await openWorkbench(page);
        const recipe = electrostatic ? `recipe/electrostatic-heat-thermo/${kind}-closed-loop` : `recipe/heat-thermo/${kind}-closed-loop`;
        const result = await page.evaluate((id) => window.__kyuubikiPwdt.runRecipe(id), recipe);
        assert.equal(result.ok, true);
        const expected = [...(electrostatic ? [`electrostatic_plane_${kind}_2d`] : []), `heat_plane_${kind}_2d`, `thermal_plane_${kind}_2d`];
        assert.deepEqual(solver.submissions.map((entry) => entry.kind), expected);
        assert.equal(library.models.length, expected.length);
        assert.equal(new Set(solver.submissions.map((entry) => entry.input.model_version_id)).size, expected.length);
        if (electrostatic) {
          const source = solver.submissions[0].input;
          const heat = solver.submissions[1].input;
          assert.deepEqual(heat.nodes.map(({ id, x, y }) => ({ id, x, y })), source.nodes.map(({ id, x, y }) => ({ id, x, y })));
          assert.deepEqual(heat.elements.map(({ id, node_i, node_j, node_k, node_l, thickness }) => ({ id, node_i, node_j, node_k, node_l, thickness })),
            source.elements.map(({ id, node_i, node_j, node_k, node_l, thickness }) => ({ id, node_i, node_j, node_k, node_l, thickness })));
        }
        const target = solver.submissions.at(-1).input;
        assert.deepEqual(target.nodes.map((node) => node.temperature_delta), target.nodes.map((_, index) => 80 - index * 10));
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.sidebarSection, "system");
        assert.equal(state.systemPanelTab, "data");
        assert.equal(state.systemDataTab, "results");
        assert.equal(state.adminFilterProjectId, "qualification-project");
        assert.equal(state.resultCount, expected.length, "completed stages must refresh the result archive");
        await page.locator('[data-workbench-data-admin="panel"]').waitFor({ state: "visible" });
        await page.locator('[data-workbench-data-page="browse"]').click();
        for (let index = 1; index <= expected.length; index += 1) {
          await page.locator(`[data-workbench-data-record-kind="result"][data-workbench-data-record-id="projection-job-${index}"]`).waitFor({ state: "visible" });
        }
      });
    });
  }
}

for (const kind of ["triangle", "quad"]) {
  for (const failure of ["missing-node", "duplicate-node", "invalid-field", "stale-input"]) {
    test(`Workbench PWDT electrostatic ${kind} projection preserves source on ${failure} and recovers after a fresh solve`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        const solver = await installProjectionSolver(page, { electrostaticFailure: failure });
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: `electrostatic_plane_${kind}_2d` });
        await invoke(page, "model/saveAs");
        const sourceRecord = structuredClone(library.models[0]);
        await invoke(page, "job/run");
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        await assert.rejects(invoke(page, "state/projectElectrostaticToHeat"), /ELECTROSTATIC_HEAT_PROJECTION_INVALID/u);
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        for (const key of ["studyKind", "hasResult", "jobStatus", "selectedModelId", "selectedVersionId"]) assert.equal(after[key], before[key]);
        assert.equal(solver.submissions.length, 1);
        assert.equal(library.models.length, 1);
        solver.electrostaticFailure = null;
        await invoke(page, "job/run");
        await invoke(page, "state/projectElectrostaticToHeat");
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), null);
        await invoke(page, "model/save");
        await invoke(page, "job/run");
        assert.equal(solver.submissions.at(-1).kind, `heat_plane_${kind}_2d`);
        assert.equal(library.models.length, 2);
        assert.deepEqual(library.models[0], sourceRecord);
      });
    });
  }
  test(`Workbench PWDT electrostatic ${kind} recipe stops before heat save or submission on invalid projection`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const solver = await installProjectionSolver(page, { electrostaticFailure: "missing-node" });
      await openWorkbench(page);
      await assert.rejects(page.evaluate((id) => window.__kyuubikiPwdt.runRecipe(id),
        `recipe/electrostatic-heat-thermo/${kind}-closed-loop`), /ELECTROSTATIC_HEAT_PROJECTION_INVALID/u);
      assert.equal(solver.submissions.length, 1);
      assert.equal(library.models.length, 1);
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.studyKind, `electrostatic_plane_${kind}_2d`);
      assert.equal(state.hasResult, true);
      assert.equal(state.selectedModelId, library.models[0].model_id);
    });
  });
}

for (const surface of ["pwdt", "gui"]) {
  test(`Workbench ${surface} rejects incomplete heat projection without discarding the source`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const solver = await installProjectionSolver(page, { incompleteHeat: true });
      await openWorkbench(page);
      await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
      await invoke(page, "model/saveAs");
      await invoke(page, "job/run");
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      if (surface === "pwdt") {
        await assert.rejects(invoke(page, "state/projectHeatToThermo"), /HEAT_THERMO_PROJECTION_INVALID/u);
      } else {
        await (await openProjectionButton(page)).click();
        await page.waitForFunction(() => window.__kyuubikiPwdt.state().message.includes("HEAT_THERMO_PROJECTION_INVALID"));
      }
      const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
      for (const key of ["studyKind", "hasResult", "jobStatus", "selectedModelId", "selectedVersionId"]) {
        assert.equal(after[key], before[key]);
      }
      assert.equal(solver.submissions.length, 1);
      assert.equal(library.models.length, 1);
    });
  });
}
