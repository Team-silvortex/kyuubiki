import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, openSavedModels, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function seed(page) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Baseline" }));
  await invoke(page, "model/save");
}

for (const saveAs of [false, true]) {
  test(`reloaded WebView reconciles a committed checkpoint using GET only; saveAs=${saveAs}`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      library[saveAs ? "loseCreateResponseOnce" : "loseVersionResponseOnce"] = true;
      await assert.rejects(page.evaluate((saveAs) => window.__kyuubikiPwdt.saveModel({ name: "Reopen recovery", saveAs }), saveAs));
      const persisted = structuredClone(library.versions.at(-1));
      const before = (await invoke(page, "model/listPendingSaves")).pending;
      assert.equal(before.length, 1);
      assert.equal(before[0].requestId, library.requests.at(-1).requestId);
      assert.deepEqual(Object.keys(before[0]).sort(), ["createdAt", "key", "operation", "parentId", "requestId", "scope"]);
      const requests = library.requests.length;
      const writes = library.writes.length;

      await openWorkbench(page);
      const after = (await invoke(page, "model/listPendingSaves")).pending;
      assert.deepEqual(after, before, "IndexedDB survives a full navigation, not just React remounting");
      const draft = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const checked = await invoke(page, "model/checkPendingSave", { key: after[0].key });
      assert.equal(checked.checkpoint.status, "committed");
      assert.equal(checked.checkpoint.version_id, persisted.version_id);
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedVersionId, draft.selectedVersionId,
        "read-only checking cannot switch the draft");

      await openSavedModels(page);
      const panel = page.locator("[data-checkpoint-recovery-panel]");
      await panel.locator("summary").click();
      await panel.getByRole("button", { name: "Check receipt", exact: true }).click();
      await panel.locator('[data-checkpoint-recovery-status="committed"]').waitFor();
      await panel.getByRole("button", { name: "Open saved version", exact: true }).click();
      await page.evaluate((id) => window.__kyuubikiPwdt.waitForState({ selectedVersionId: id }), persisted.version_id);
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).loadedModelName, "Reopen recovery");
      assert.equal(library.requests.length, requests, "no POST at all, including idempotent replay");
      assert.equal(library.writes.length, writes);
      assert.equal(library.versions.length, 2);
      assert.equal(library.models.length, saveAs ? 2 : 1);
      await invoke(page, "model/acknowledgeSave", { key: after[0].key });
      assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, []);
      await openWorkbench(page);
      assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, []);
    });
  });
}

test("unknown save survives reload and cannot be opened, cleared, or silently resent", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    library.failVersions = true;
    await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Unknown save" })));
    const count = library.requests.length;
    await openWorkbench(page);
    const { pending: [entry] } = await invoke(page, "model/listPendingSaves");
    assert.ok(entry);
    assert.equal((await invoke(page, "model/checkPendingSave", { key: entry.key })).checkpoint.status, "unknown");
    await assert.rejects(invoke(page, "model/acknowledgeSave", { key: entry.key }), /commit_unconfirmed/);
    await assert.rejects(invoke(page, "model/openRecoveredSave", { key: entry.key }), /commit_unconfirmed/);
    assert.equal((await invoke(page, "model/listPendingSaves")).pending.length, 1);
    assert.equal(library.requests.length, count);
    assert.equal(library.versions.length, 1);
    await openSavedModels(page);
    const panel = page.locator("[data-checkpoint-recovery-panel]");
    await panel.locator("summary").click();
    await panel.getByRole("button", { name: "Check receipt", exact: true }).click();
    await panel.locator('[data-checkpoint-recovery-status="unknown"]').waitFor();
    assert.equal(await panel.getByRole("button", { name: "Open saved version", exact: true }).isDisabled(), true);
    assert.equal(await panel.getByRole("button", { name: "Clear confirmed receipt", exact: true }).isDisabled(), true);
  });
});

test("unavailable journal blocks before save transport without clearing the current draft", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    const draft = await page.evaluate(() => window.__kyuubikiPwdt.state());
    const requests = library.requests.length;
    await page.evaluate(() => Object.defineProperty(window, "indexedDB", { configurable: true, value: undefined }));
    await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Must not send" })), /journal_unavailable/);
    assert.equal(library.requests.length, requests);
    assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedVersionId, draft.selectedVersionId);
    assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).loadedModelName, draft.loadedModelName);
  });
});

test("deleted result is reported after reload and never recreated by recovery", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await seed(page);
    library.loseVersionResponseOnce = true;
    await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "Deleted receipt" })));
    const removed = library.versions.pop();
    assert.ok(removed);
    const writes = library.writes.length;
    await openWorkbench(page);
    const { pending: [entry] } = await invoke(page, "model/listPendingSaves");
    assert.equal((await invoke(page, "model/checkPendingSave", { key: entry.key })).checkpoint.status, "deleted");
    await assert.rejects(invoke(page, "model/openRecoveredSave", { key: entry.key }), /result_deleted/);
    await invoke(page, "model/acknowledgeSave", { key: entry.key });
    assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, []);
    assert.equal(library.writes.length, writes);
  });
});
