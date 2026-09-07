import assert from "node:assert/strict";
import { test } from "node:test";
import {
  usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks,
} from "./workbench-ui-project-fixture.shared.mjs";
import { installProjectionSolver, openProjectionButton } from "./workbench-ui-pwdt-projection-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

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
