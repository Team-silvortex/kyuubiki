import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const all = { kind: "all" };
const field = (page, name) => page.locator(`[data-model-batch="${name}"]`);
const inspect = (page, query = all) => invoke(page, "state/inspectModelBatch", { query });
const apply = (page, operation, query = all) => invoke(page, "state/applyModelBatch", { query, operation });

function modelFixture(spatial = true, frame = false) {
  return {
    ...(frame ? { materials: [
      { id: "aluminium", name: "Aluminium", youngs_modulus: 70e9 },
      { id: "steel", name: "Steel", youngs_modulus: 210e9 },
    ] } : {}),
    nodes: Array.from({ length: 6 }, (_, index) => ({
      id: `node-${index}`, x: index, y: 0, fix_x: index === 0, fix_y: index === 0, load_x: 0, load_y: 0,
      ...(spatial ? { z: 0, fix_z: index === 0, load_z: 0 } : {}),
      ...(frame ? { fix_rz: index === 0, moment_z: 0 } : {}),
    })),
    elements: Array.from({ length: 5 }, (_, index) => ({
      id: `member-${index}`, node_i: index, node_j: index + 1, area: 0.01, youngs_modulus: 70e9,
      ...(frame ? { moment_of_inertia: 0.001, section_modulus: 0.01 } : {}),
    })),
  };
}

async function openBatch(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ modelTab: "tools", modelToolsPage: "studio" }));
  await field(page, "toggle").click();
  await field(page, "editor").waitFor({ state: "visible" });
}

async function savedPayload(page, library, name) {
  await page.evaluate((name) => window.__kyuubikiPwdt.saveModel({ name, saveAs: true }), name);
  return library.versions.at(-1).payload;
}

test("Workbench batch UI and PWDT share atomic array, loads, deletion and undo/redo", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", modelFixture());
    await invoke(page, "selection/set3d", { nodeIndices: [1, 2], anchorNodeIndex: 1 });
    await openBatch(page);
    assert.equal((await inspect(page, { kind: "current" })).nodes, 2);
    await field(page, "query").selectOption("indices");
    await field(page, "indices").fill("1-2");
    await field(page, "operation").selectOption("array");
    await field(page, "vector-y").fill("2");
    await field(page, "copies").fill("3");
    assert.match(await field(page, "preview").innerText(), /\+Nodes: 6/);
    await field(page, "apply").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices.length === 6);
    assert.equal((await inspect(page)).nodes, 12);
    assert.equal((await inspect(page)).internalMembers, 8);
    await invoke(page, "history/undo");
    assert.equal((await inspect(page)).nodes, 6);
    assert.deepEqual(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices), [1, 2]);
    await invoke(page, "history/redo");
    assert.equal((await inspect(page)).nodes, 12);
    assert.deepEqual(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices), [6, 7, 8, 9, 10, 11]);

    await field(page, "query").selectOption("all");
    await field(page, "operation").selectOption("align");
    await field(page, "align-axis").selectOption("z");
    await field(page, "coordinate").fill("2");
    await field(page, "apply").click();
    assert.equal((await inspect(page, { kind: "range", axis: "z", min: 2, max: 2 })).nodes, 12);
    await field(page, "operation").selectOption("loads");
    await field(page, "distribution").selectOption("total");
    await field(page, "vector-y").fill("-120");
    await field(page, "apply").click();
    await apply(page, { kind: "members", scope: "internal", area: 0.05 });
    await field(page, "query").selectOption("indices");
    await field(page, "indices").fill("5");
    await field(page, "operation").selectOption("supports");
    await field(page, "support-y").uncheck();
    await field(page, "support-z").check();
    await field(page, "apply").click();
    const saved = await savedPayload(page, library, "batch-3d");
    assert.equal(saved.nodes.length, 12);
    assert.equal(saved.nodes.reduce((sum, node) => sum + node.load_y, 0), -120);
    assert.ok(saved.elements.every((member) => member.area === 0.05));
    assert.deepEqual(saved.elements.slice(5).map((member) => [member.node_i, member.node_j]), [[6, 7], [8, 9], [10, 11]]);
    assert.equal(saved.nodes[6].fix_x, false, "new copies must not inherit supports by default");
    assert.equal(saved.nodes[5].fix_z, true);
    assert.equal(saved.nodes[5].fix_y, false);

    await field(page, "operation").selectOption("delete");
    await field(page, "indices").fill("6-11");
    assert.equal(await field(page, "apply").isDisabled(), true);
    await field(page, "confirm-delete").check();
    await apply(page, { kind: "translate", offset: { x: 0, y: 1, z: 0 } });
    assert.equal(await field(page, "apply").isDisabled(), true, "changing the model invalidates delete consent");
    await field(page, "confirm-delete").check();
    await field(page, "apply").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices.length === 0);
    assert.equal((await inspect(page)).nodes, 6);
    assert.equal((await inspect(page)).internalMembers, 5);
    await invoke(page, "history/undo");
    assert.equal((await inspect(page)).nodes, 12);
    await invoke(page, "history/redo");
    assert.equal((await inspect(page)).nodes, 6);

    await assert.rejects(() => apply(page, { kind: "delete", confirmDelete: false }), /confirmation_required/);
    await assert.rejects(() => apply(page, { kind: "array", copies: 1001, offset: { x: 1, y: 0 } }), /invalid_copies/);
    assert.equal((await inspect(page)).nodes, 6);
    await invoke(page, "history/undo");
    assert.equal((await inspect(page)).nodes, 12, "invalid requests cannot consume undo checkpoints");
    if (process.env.KYUUBIKI_BATCH_SCREENSHOT) {
      await field(page, "operation").selectOption("array");
      await page.screenshot({ path: process.env.KYUUBIKI_BATCH_SCREENSHOT });
    }
  });
});

test("Workbench batch UI supports planar studies and guards unsupported study kinds", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    for (const frame of [false, true]) {
      await invoke(page, frame ? "state/replaceFrameModel" : "state/replaceTruss2dModel", modelFixture(false, frame));
      await openBatch(page);
      await field(page, "query").selectOption("range");
      await field(page, "min").fill("1");
      await field(page, "max").fill("3");
      await field(page, "vector-y").fill("2");
      await field(page, "apply").click();
      await field(page, "status").waitFor({ state: "visible" });
      assert.equal((await inspect(page, { kind: "range", axis: "y", min: 2, max: 2 })).nodes, 3);
      assert.equal(await field(page, "vector-z").count(), 0);
      await assert.rejects(() => apply(page, { kind: "translate", offset: { x: 0, y: 0, z: 1 } }), /unsupported_axis/);
      if (frame) {
        await apply(page, { kind: "supports", axes: ["rz"], fixed: true });
        await field(page, "query").selectOption("all");
        await field(page, "operation").selectOption("members");
        await field(page, "area").fill("0.02");
        await field(page, "material").selectOption("steel");
        await field(page, "apply").click();
      }
      const saved = await savedPayload(page, library, frame ? "batch-frame" : "batch-truss");
      assert.equal(saved.nodes[2].y, 2);
      assert.equal("z" in saved.nodes[2], false);
      if (frame) {
        assert.ok(saved.nodes.every((node) => node.fix_rz));
        assert.ok(saved.elements.every((member) => member.area === 0.02 && member.youngs_modulus === 210e9));
      }
    }
    await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
    assert.equal(await field(page, "panel").count(), 0);
    await assert.rejects(() => apply(page, { kind: "translate", offset: { x: 1, y: 0 } }), /unsupported_study/);
  });
});

test("Workbench geometry transform UI and PWDT agree on pivots, copying, snapping and recovery", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await invoke(page, "state/replaceTruss3dModel", modelFixture());
    await invoke(page, "selection/set3d", { nodeIndices: [1, 2, 3], anchorNodeIndex: 1 });
    await openBatch(page);
    await field(page, "operation").selectOption("rotate");
    await field(page, "pivot").selectOption("origin");
    await field(page, "angle").fill("90");
    await field(page, "apply").click();
    const rotated = await savedPayload(page, library, "geometry-rotated");
    assert.deepEqual(rotated.nodes.slice(1, 4).map((node) => [node.x, node.y, node.z]), [[0, 1, 0], [0, 2, 0], [0, 3, 0]]);
    assert.equal(rotated.nodes[4].x, 4);
    await invoke(page, "history/undo");

    await field(page, "operation").selectOption("scale");
    await field(page, "pivot").selectOption("point");
    await field(page, "pivot-x").fill("1");
    await field(page, "factor-x").fill("2");
    await field(page, "apply").click();
    const scaled = await savedPayload(page, library, "geometry-scaled");
    assert.deepEqual(scaled.nodes.slice(1, 4).map((node) => node.x), [1, 3, 5]);
    assert.ok(scaled.elements.every((member) => member.area === 0.01));
    await invoke(page, "history/undo");

    await field(page, "operation").selectOption("mirror");
    await field(page, "pivot-x").fill("-1.1");
    await field(page, "create-copy").check();
    assert.match(await field(page, "preview").innerText(), /\+Nodes: 3/);
    await field(page, "apply").click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices[0] === 6);
    const mirrored = await savedPayload(page, library, "geometry-mirrored");
    assert.equal(mirrored.nodes.length, 9);
    assert.deepEqual(mirrored.elements.slice(5).map((member) => [member.node_i, member.node_j]), [[6, 7], [7, 8]]);
    assert.equal(mirrored.nodes[6].x, -3.2);
    assert.equal(mirrored.nodes[6].fix_x, false);
    assert.deepEqual(mirrored.elements.slice(0, 5).map((member) => member.id), modelFixture().elements.map((member) => member.id));
    await invoke(page, "history/undo");
    assert.equal((await inspect(page)).nodes, 6);
    assert.deepEqual(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedTruss3dNodeIndices), [1, 2, 3]);
    await invoke(page, "history/redo");

    await field(page, "operation").selectOption("snap");
    await field(page, "spacing").fill("2");
    await field(page, "apply").click();
    assert.match(await field(page, "status").innerText(), /degenerate_member/);
    const failed = await savedPayload(page, library, "geometry-rejected");
    assert.deepEqual(failed.nodes, mirrored.nodes);
    await invoke(page, "history/undo");
    assert.equal((await inspect(page)).nodes, 6, "failed snap must not consume an undo checkpoint");
    await invoke(page, "history/redo");
    await field(page, "spacing").fill("1");
    await field(page, "apply").click();
    const snapped = await savedPayload(page, library, "geometry-snapped");
    assert.deepEqual(snapped.nodes.slice(6).map((node) => node.x), [-3, -4, -5]);
    await field(page, "apply").click();
    assert.match(await field(page, "status").innerText(), /No changes/);
    await invoke(page, "history/undo");
    assert.deepEqual((await savedPayload(page, library, "geometry-unsnapped")).nodes, mirrored.nodes);

    await invoke(page, "state/replaceTruss3dModel", modelFixture());
    await apply(page, { kind: "mirror", axis: "x", pivot: { kind: "point", point: { x: -1.1, y: 0, z: 0 } }, copy: true },
      { kind: "indices", indices: [1, 2, 3] });
    await apply(page, { kind: "snap", axes: ["x", "y", "z"], spacing: 1 }, { kind: "current" });
    const scripted = await savedPayload(page, library, "geometry-pwdt");
    assert.deepEqual(scripted.nodes, snapped.nodes);
    assert.deepEqual(scripted.elements, snapped.elements);
    await field(page, "operation").selectOption("rotate");
    await field(page, "create-copy").check();
    await field(page, "panel").evaluate((panel) => {
      const scroller = panel.closest(".panel-scroll-window");
      if (scroller) scroller.scrollTop = 0;
    });
    const applyBounds = await field(page, "apply").boundingBox();
    assert.ok(applyBounds && applyBounds.y + applyBounds.height <= page.viewportSize().height,
      "custom-pivot rotation should keep Apply visible in the standard 1440x1000 workspace");
    if (process.env.KYUUBIKI_TRANSFORM_SCREENSHOT) {
      await page.screenshot({ path: process.env.KYUUBIKI_TRANSFORM_SCREENSHOT });
    }
  });
});
