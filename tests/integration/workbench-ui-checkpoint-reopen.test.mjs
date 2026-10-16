import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, openSavedModels, holdRequest, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function seed(page) {
  await openWorkbench(page);
  await page.evaluate(() => window.__kyuubikiPwdt.buildParametricTruss2d({ bays: 3, modelName: "Baseline" }));
  await invoke(page, "model/save");
}

async function pendingRecords(page, replacement = null) {
  return page.evaluate(replacement => new Promise((resolve, reject) => {
    const request = indexedDB.open("kyuubiki-checkpoint-recovery", 1);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result, tx = db.transaction("pending", replacement ? "readwrite" : "readonly");
      const store = tx.objectStore("pending");
      if (replacement) store.put(replacement);
      const read = store.getAll();
      tx.oncomplete = () => { db.close(); resolve(read.result); };
      tx.onabort = () => { db.close(); reject(tx.error); };
    };
  }), replacement);
}

for (const field of ["scope", "operation", "parentId"]) {
  test(`checkpoint save rejects an IndexedDB reservation identity conflict without transport: ${field}`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page); library.failVersions = true;
      const save = () => invoke(page, "model/save", { name: "Stable retry snapshot" });
      await assert.rejects(save());
      const [original] = await pendingRecords(page);
      assert.ok(original);
      const corrupt = { ...original, [field]: field === "scope" ? "f".repeat(64)
        : field === "operation" ? "model" : "another-parent" };
      await pendingRecords(page, corrupt);
      library.failVersions = false;
      const requests = library.requests.length;
      await assert.rejects(save(), /checkpoint:journal_corrupt/);
      assert.equal(library.requests.length, requests, "conflicting metadata must block before POST");
      assert.deepEqual(await pendingRecords(page), [corrupt], "no overwrite or cleanup of the conflicting record");
      assert.equal(library.versions.length, 1);
      // Restore only the injected test corruption, never a production auto-repair.
      await pendingRecords(page, original);
      await save();
      assert.equal(library.requests.at(-1).requestId, original.requestId);
      assert.equal(library.versions.length, 2);
      assert.deepEqual(await pendingRecords(page), []);
    });
  });
}

for (const action of ["model/checkPendingSave", "model/acknowledgeSave", "model/openRecoveredSave"]) {
  test(`checkpoint page lifecycle retires ${action} without clearing or opening the saved receipt`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      library.loseVersionResponseOnce = true;
      await assert.rejects(invoke(page, "model/save", { name: "Recover exact owner" }));
      const { pending: [entry] } = await invoke(page, "model/listPendingSaves");
      const saved = library.versions.at(-1), before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const posts = library.requests.length;
      const path = action === "model/openRecoveredSave" ? `/api/v1/model-versions/${saved.version_id}`
        : `/api/v1/checkpoints/${entry.operation}/${entry.parentId}/${entry.requestId}`;
      const gate = await holdRequest(page, path, "GET");
      const outcome = invoke(page, action, { key: entry.key }).catch(error => ({ error: error.message }));
      try {
        await gate.received;
        // Controlled lifecycle injection, not actual browser BFCache qualification.
        await page.evaluate(() => {
          dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true }));
          dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
        });
        gate.release();
        assert.match((await outcome).error, /context_changed|source.*changed/i);
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(after.selectedVersionId, before.selectedVersionId);
        assert.equal(after.loadedModelName, before.loadedModelName);
        assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, [entry]);
        await invoke(page, "model/acknowledgeSave", { key: entry.key });
        assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, []);
        assert.equal(library.requests.length, posts, "recovery sends no checkpoint POST");
        assert.equal(library.versions.length, 2);
      } finally { gate.release(); await outcome; }
    });
  });
}

for (const saveAs of [false, true]) {
  test(`retired in-flight checkpoint save retains its recovery identity; saveAs=${saveAs}`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page);
      const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
      const path = saveAs ? `/api/v1/projects/${before.selectedProjectId}/models` : `/api/v1/models/${before.selectedModelId}/versions`;
      const gate = await holdRequest(page, path, "POST");
      const outcome = invoke(page, saveAs ? "model/saveAs" : "model/save", { name: "Late committed save" })
        .catch(error => ({ error: error.message }));
      try {
        await gate.received;
        await page.evaluate(() => {
          dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true }));
          dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
        });
        gate.release();
        assert.match((await outcome).error, /context_changed/);
        assert.equal(library.versions.length, 2, "already sent work can commit after authority retirement");
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(after.selectedVersionId, before.selectedVersionId);
        assert.equal(after.loadedModelName, before.loadedModelName);
        const { pending: [entry] } = await invoke(page, "model/listPendingSaves");
        assert.equal(entry.requestId, library.requests.at(-1).requestId);
        const posts = library.requests.length;
        assert.equal((await invoke(page, "model/checkPendingSave", { key: entry.key })).checkpoint.version_id, library.versions.at(-1).version_id);
        await invoke(page, "model/acknowledgeSave", { key: entry.key });
        assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, []);
        assert.equal(library.requests.length, posts);
      } finally { gate.release(); await outcome; }
    });
  });
}

for (const phase of ["get", "delete"]) {
  test(`checkpoint cleanup aborts its active IndexedDB transaction after ${phase} owner loss`, { timeout: 120_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      await seed(page); library.loseVersionResponseOnce = true;
      await assert.rejects(invoke(page, "model/save", { name: "Retain this receipt" }));
      const { pending: [entry] } = await invoke(page, "model/listPendingSaves");
      const posts = library.requests.length;
      await page.evaluate(phase => {
        const original = IDBObjectStore.prototype[phase];
        window.__checkpointRetired = false;
        IDBObjectStore.prototype[phase] = function(...args) {
          const request = original.apply(this, args);
          if (this.transaction.db.name === "kyuubiki-checkpoint-recovery" && this.name === "pending") {
            IDBObjectStore.prototype[phase] = original;
            Object.defineProperty(request, "onsuccess", { configurable: true, set: handler => {
              request.addEventListener("success", event => {
                window.__checkpointRetired = true;
                dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true }));
                dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
                handler.call(request, event);
              }, { once: true });
            } });
          }
          return request;
        };
      }, phase);
      await assert.rejects(invoke(page, "model/acknowledgeSave", { key: entry.key }), /context_changed/);
      assert.equal(await page.evaluate(() => window.__checkpointRetired), true);
      assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, [entry]);
      await invoke(page, "model/acknowledgeSave", { key: entry.key });
      assert.deepEqual((await invoke(page, "model/listPendingSaves")).pending, []);
      assert.equal(library.requests.length, posts);
    });
  });
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
