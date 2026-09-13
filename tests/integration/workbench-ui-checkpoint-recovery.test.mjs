import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, openSavedModels, holdRequest, waitForGuiTransition, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const rows = (page) => page.locator("button.history-item strong");

async function openVersions(page) {
  await openSavedModels(page);
  await page.locator('[data-workbench-library-model-page="versions"]').click();
}

async function seed(page) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Baseline" }));
  await invoke(page, "model/save");
  await openVersions(page);
  await rows(page).filter({ hasText: "Baseline" }).waitFor();
}

test("committed checkpoint survives a failed history refresh; read-only retry recovers the version list", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    const baseline = structuredClone(library.versions[0]);
    library.failVersionReads = true;
    const receipt = await invoke(page, "model/save", { name: "Revised" });
    assert.equal(receipt.ok, true);
    assert.equal(receipt.versionId, library.versions[1].version_id);
    assert.deepEqual(library.versions[0], baseline);
    assert.equal(library.models[0].name, "Revised");
    assert.deepEqual(await rows(page).allTextContents(), ["Baseline"], "a failed read must not erase known history");
    await page.getByText(/Versions:.*qualification version read unavailable/).first().waitFor();
    const writes = library.writes.length;
    library.failVersionReads = false;
    await invoke(page, "runtime/refreshAll");
    await rows(page).filter({ hasText: "Revised" }).waitFor();
    assert.deepEqual(await rows(page).allTextContents(), ["Revised", "Baseline"]);
    assert.equal(library.writes.length, writes, "recovery only reads; it cannot save a duplicate checkpoint");
    assert.equal(await page.getByText(/Versions:.*qualification version read unavailable/).count(), 0);
  });
});

test("failed history reads cannot carry another model's versions into Save As or reloaded models", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    library.failVersionReads = true;
    const receipt = await invoke(page, "model/saveAs", { name: "Independent variant" });
    assert.equal(receipt.ok, true);
    assert.equal(receipt.modelId, library.models[1].model_id);
    assert.equal(await rows(page).count(), 0, "old-model history cannot masquerade as variant history");
    await page.getByText(/Versions:.*qualification version read unavailable/).first().waitFor();
    const writes = library.writes.length;
    library.failVersionReads = false;
    await invoke(page, "runtime/refreshAll");
    await rows(page).filter({ hasText: "Independent variant" }).waitFor();
    assert.deepEqual(await rows(page).allTextContents(), ["Independent variant"]);
    assert.equal(library.writes.length, writes);
    library.failVersionReads = true;
    await openSavedModels(page);
    await page.locator("button.history-item").filter({ hasText: "Baseline" }).click();
    await page.evaluate((modelId) => window.__kyuubikiPwdt.waitForState({ selectedModelId: modelId }), library.models[0].model_id);
    await openVersions(page);
    assert.equal(await rows(page).count(), 0, "reopening a different model clears the previous model's history");
    library.failVersionReads = false;
    await invoke(page, "runtime/refreshAll");
    await rows(page).filter({ hasText: "Baseline" }).waitFor();
    assert.deepEqual(await rows(page).allTextContents(), ["Baseline"]);
  });
});

test("a late failed history response cannot clear or warn over a newer saved model", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    const pending = await holdRequest(page, `/api/v1/models/${library.models[0].model_id}/versions`, "GET",
      (route) => route.fulfill({ status: 503, json: { error: "obsolete history read" } }));
    const refresh = invoke(page, "runtime/refreshAll");
    try {
      await pending.received;
      await invoke(page, "model/saveAs", { name: "New context" });
      await rows(page).filter({ hasText: "New context" }).waitFor();
      pending.release();
      await refresh;
      assert.deepEqual(await rows(page).allTextContents(), ["New context"]);
      assert.equal(await page.getByText(/obsolete history read/).count(), 0);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedModelId), library.models[1].model_id);
    } finally { pending.release(); await refresh; }
  });
});

test("PWDT save helper preserves local and persisted metadata on rejection before a clean retry", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
    const saved = structuredClone(library.models[0]);
    library.failVersions = true;
    await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Retry name", material: "70" })),
      /qualification version service unavailable/);
    const failed = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(failed.loadedModelName, before.loadedModelName);
    assert.equal(failed.activeMaterial, before.activeMaterial);
    assert.deepEqual(library.models[0], saved);
    library.failVersions = false;
    await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Retry name", material: "70" }));
    assert.equal(library.versions.length, 2);
    assert.equal(library.models[0].name, "Retry name");
    assert.equal(library.models[0].material, "70");
    assert.equal(library.versions[1].payload.name, "Retry name");
    assert.equal(library.versions[1].payload.material, "70");
    const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(after.loadedModelName, "Retry name");
    assert.equal(after.activeMaterial, "70");
  });
});

for (const saveAs of [false, true]) {
  test(`a committed save survives catalog and history outages; recovery never rewrites; saveAs=${saveAs}`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      const previousVersion = structuredClone(library.versions[0]);
      let failCatalog = true;
      await page.route("**/api/v1/projects", (route) => failCatalog
        ? route.fulfill({ status: 503, json: { error: "qualification catalog unavailable" } })
        : route.fallback());
      library.failVersionReads = true;
      const receipt = await page.evaluate((saveAs) => window.__kyuubikiPwdt.saveModel({ name: "Recovered checkpoint", saveAs }), saveAs);
      assert.equal(receipt.ok, true, "read outages cannot turn a confirmed commit into a failed save");
      const checkpoint = structuredClone(library.versions.at(-1));
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.selectedModelId, checkpoint.model_id);
      assert.equal(state.selectedVersionId, checkpoint.version_id);
      assert.equal(state.loadedModelName, "Recovered checkpoint");
      assert.deepEqual(library.versions[0], previousVersion);
      await page.getByText(/Project library:.*qualification catalog unavailable/).first().waitFor();
      await page.getByText(/Versions:.*qualification version read unavailable/).first().waitFor();
      const writes = library.writes.length;
      const requests = library.requests.length;
      failCatalog = false;
      library.failVersionReads = false;
      await invoke(page, "runtime/refreshAll");
      await openVersions(page);
      await rows(page).filter({ hasText: "Recovered checkpoint" }).waitFor();
      assert.equal(library.writes.length, writes);
      assert.equal(library.requests.length, requests, "reconciliation sends no checkpoint POST, even an idempotent one");
      assert.equal(library.versions.length, 2);
      assert.equal(library.models.length, saveAs ? 2 : 1);
      assert.equal(await page.getByText(/qualification (catalog|version read) unavailable/).count(), 0);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().selectedVersionId), checkpoint.version_id);
    });
  });

  test(`GUI save buttons recover a lost committed response without another write; saveAs=${saveAs}`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      await openSavedModels(page);
      await page.getByRole("textbox", { name: "Model", exact: true }).fill("GUI retry");
      const button = page.getByRole("button", { name: saveAs ? "Save As" : "Save", exact: true });
      library[saveAs ? "loseCreateResponseOnce" : "loseVersionResponseOnce"] = true;
      const failed = page.waitForEvent("requestfailed", (request) => request.method() === "POST" && request.url().includes("/models"));
      await button.click();
      await failed;
      await waitForGuiTransition(page);
      const persisted = structuredClone(library.versions.at(-1));
      const key = library.requests.at(-1).requestId;
      const writes = library.writes.length;
      const versions = library.versions.length;
      const response = page.waitForResponse((response) => response.request().method() === "POST" && response.url().includes("/models"));
      await button.click();
      assert.equal((await response).status(), 201);
      await page.evaluate((versionId) => window.__kyuubikiPwdt.waitForState({ selectedVersionId: versionId }), persisted.version_id);
      await waitForGuiTransition(page);
      assert.equal(library.requests.at(-1).requestId, key);
      assert.equal(library.writes.length, writes);
      assert.equal(library.versions.length, versions);
    });
  });

  test(`PWDT recovers a committed checkpoint after losing its response without duplicate writes; saveAs=${saveAs}`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const params = { name: "Confirmed retry", material: "70", saveAs };
      library[saveAs ? "loseCreateResponseOnce" : "loseVersionResponseOnce"] = true;
      await assert.rejects(page.evaluate((params) => window.__kyuubikiPwdt.saveModel(params), params));
      const first = library.requests.at(-1);
      assert.match(first.requestId, /^[a-f0-9-]{36}$/);
      const persisted = structuredClone(library.versions.at(-1));
      const modelCount = library.models.length;
      const versionCount = library.versions.length;
      const writeCount = library.writes.length;
      const failed = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(failed.loadedModelName, before.loadedModelName);
      assert.equal(failed.selectedModelId, before.selectedModelId);
      assert.equal(failed.selectedVersionId, before.selectedVersionId);
      const receipt = await page.evaluate((params) => window.__kyuubikiPwdt.saveModel(params), params);
      assert.equal(receipt.ok, true);
      assert.equal(library.requests.at(-1).requestId, first.requestId);
      assert.equal(library.models.length, modelCount);
      assert.equal(library.versions.length, versionCount);
      assert.equal(library.writes.length, writeCount);
      const recovered = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(recovered.selectedVersionId, persisted.version_id);
      assert.equal(recovered.loadedModelName, params.name);
      await page.evaluate((params) => window.__kyuubikiPwdt.saveModel(params), params);
      assert.notEqual(library.requests.at(-1).requestId, first.requestId);
      assert.equal(library.versions.length, versionCount + 1, "a later intentional save is a new checkpoint");
    });
  });
}
