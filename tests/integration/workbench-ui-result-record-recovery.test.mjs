import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, runtime, holdRequest, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "record-result";
const PATH = "/api/v1/results";
const record = (job_id = ID) => ({ job_id, result: { metric: 1 } });
const row = (page, id = ID) => page.locator(`[data-workbench-data-record-kind="result"][data-workbench-data-record-id="${id}"]`);
const draft = (page) => page.locator('[data-workbench-data-field="result-payload"]');
const warning = (page) => page.locator('[data-workbench-results="read-error"]');

function seed() {
  runtime.state.adminResults.push(record(), record("other-result"));
  runtime.state.resultRecordMutations.length = 0;
  runtime.state.jobRecordMutations.length = 0;
}

async function openEditor(page, id = ID) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: "results" }));
  await page.locator('[data-workbench-data-page="browse"]').click();
  await row(page, id).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}

async function submit(page, action) {
  const response = page.waitForResponse((entry) => new URL(entry.url()).pathname === `${PATH}/${ID}` &&
    entry.request().method() === (action === "save" ? "PATCH" : "DELETE"));
  await page.locator(`[data-workbench-data-action="${action}-result"]`).click();
  return response;
}

function assertWrites(library, writes = 0) {
  assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
  assert.equal(runtime.state.jobRecordMutations.length, 0);
  assert.equal(runtime.state.resultRecordMutations.length, writes);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

for (const action of ["save", "delete"]) {
  const failures = [["null", null], ["wrong-id", record("wrong")],
    ["null-result", { job_id: ID, result: null }], ["array-result", { job_id: ID, result: [] }]];
  if (action === "delete") failures.push(["missing-deleted", record()], ["false-deleted", { ...record(), deleted: false }]);
  for (const [label, input] of failures) {
    test(`Workbench result ${action} ${label} receipt stays unconfirmed and read recovery never replays it`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        seed();
        await openWorkbench(page);
        await openEditor(page);
        await draft(page).fill('{"metric":42}');
        let reads = 0;
        let attempts = 0;
        await page.route(`**${PATH}`, (route) => { reads += 1; return route.fallback(); });
        const payload = action === "delete" && input && !label.includes("deleted") ? { ...input, deleted: true } : input;
        await page.route(`**${PATH}/${ID}`, (route) => { attempts += 1; return route.fulfill({ json: payload }); });
        assert.equal((await submit(page, action)).status(), 200);
        await page.waitForFunction(() => window.__kyuubikiPwdt.state().message.includes("RESULT_RECORD_MUTATION_UNCONFIRMED"), null, { timeout: 5_000 });
        assert.equal(await draft(page).inputValue(), '{"metric":42}');
        assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, ID);
        assert.equal(reads, 0);
        assert.equal(attempts, 1);
        await invoke(page, "runtime/refreshAll");
        await page.waitForLoadState("networkidle");
        assert.equal(attempts, 1, "read recovery must not replay an unconfirmed result write");
        assert.equal(reads, 1);
        assert.equal(await draft(page).inputValue(), '{"metric":42}');
        assertWrites(library);
      });
    });
  }
}

for (const action of ["save", "export"]) for (const [label, value] of [
  ["null", "null"], ["array", "[]"], ["overflow", '{"nested":[1e400]}'],
]) {
  test(`Workbench result invalid ${label} ${action} stays local without writes or downloads`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openEditor(page);
      await draft(page).fill(value);
      let downloads = 0;
      let requests = 0;
      page.on("download", () => { downloads += 1; });
      await page.route(`**${PATH}/${ID}`, (route) => { requests += 1; return route.fallback(); });
      await page.locator(`[data-workbench-data-action="${action}-result"]`).click();
      await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Invalid JSON payload.", null, { timeout: 5_000 });
      await page.waitForLoadState("networkidle");
      assert.equal(await draft(page).inputValue(), value);
      assert.equal(downloads + requests, 0);
      await invoke(page, "runtime/refreshAll");
      assert.equal(await draft(page).inputValue(), value);
      assertWrites(library);
    });
  });
}

test("Workbench result empty object saves and exports the working draft without another write", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openEditor(page);
    await draft(page).fill("{}");
    assert.equal((await submit(page, "save")).status(), 200);
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Result record updated.");
    assert.deepEqual(runtime.state.adminResults[0].result, {});
    const working = { nested: [null, [], {}, true, "x", 1e-300] };
    await draft(page).fill(JSON.stringify(working));
    const downloaded = page.waitForEvent("download", { timeout: 10_000 });
    await page.locator('[data-workbench-data-action="export-result"]').click();
    const download = await downloaded;
    assert.equal(download.suggestedFilename(), `${ID}-result.json`);
    assert.deepEqual(JSON.parse(await readFile(await download.path(), "utf8")), working);
    assert.deepEqual(runtime.state.adminResults[0].result, {});
    assertWrites(library, 1);
  });
});

for (const action of ["save", "delete"]) {
  test(`Workbench result ${action} malformed response preserves editing without blaming valid draft JSON`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openEditor(page);
      await draft(page).fill('{"metric":42}');
      let attempts = 0;
      let reads = 0;
      await page.route(`**${PATH}`, (route) => { reads += 1; return route.fallback(); });
      await page.route(`**${PATH}/${ID}`, (route) => {
        attempts += 1;
        return route.fulfill({ status: 200, contentType: "application/json", body: "{" });
      });
      await submit(page, action);
      // The HTTP client tolerates JSON decoding failure as an empty payload; receipt validation owns confirmation.
      await page.waitForFunction(() => window.__kyuubikiPwdt.state().message.includes("RESULT_RECORD_MUTATION_UNCONFIRMED"), null, { timeout: 5_000 });
      assert.equal(await draft(page).inputValue(), '{"metric":42}');
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, ID);
      assert.equal(reads, 0);
      await invoke(page, "runtime/refreshAll");
      await page.waitForLoadState("networkidle");
      assert.equal(attempts, 1);
      assert.equal(reads, 1);
      assert.equal(await draft(page).inputValue(), '{"metric":42}');
      assertWrites(library);
    });
  });
}

for (const other of [false, true]) {
  test(`Workbench slow result save retains newer typing on ${other ? "another" : "the same"} selected record`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openEditor(page);
      await draft(page).fill('{"metric":42}');
      const held = await holdRequest(page, `${PATH}/${ID}`, "PATCH");
      try {
        const response = submit(page, "save");
        await held.received;
        if (other) await openEditor(page, "other-result");
        await draft(page).fill('{"metric":99}');
        const newerMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
        held.release();
        assert.equal((await response).status(), 200);
        await waitForGuiTransition(page);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), other ? newerMessage : "Result record updated.");
        assert.equal(await draft(page).inputValue(), '{"metric":99}');
        assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, other ? "other-result" : ID);
        assert.deepEqual(runtime.state.resultRecordMutations[0].body, { result: { metric: 42 } });
        assertWrites(library, 1);
      } finally { held.release(); }
    });
  });
}

test("Workbench delayed result deletion preserves another selection and its unsaved draft", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openEditor(page);
    const held = await holdRequest(page, `${PATH}/${ID}`, "DELETE");
    try {
      const response = submit(page, "delete");
      await held.received;
      await openEditor(page, "other-result");
      await draft(page).fill('{"metric":99}');
      const newerMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
      held.release();
      assert.equal((await response).status(), 200);
      await waitForGuiTransition(page);
      assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), newerMessage);
      assert.equal(await draft(page).inputValue(), '{"metric":99}');
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, "other-result");
      assertWrites(library, 1);
    } finally { held.release(); }
  });
});

for (const oldFails of [false, true]) {
  test(`Workbench confirmed result deletion fences an older ${oldFails ? "failed" : "successful"} read`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openEditor(page);
      const oldRecords = structuredClone(runtime.state.adminResults);
      const oldRead = await holdRequest(page, PATH, "GET", (route) => route.fulfill(oldFails
        ? { status: 503, json: { error: "old result failure" } } : { json: { results: oldRecords } }));
      let followupRead;
      const oldRefresh = invoke(page, "runtime/refreshAll");
      void oldRefresh.catch(() => {});
      try {
        await oldRead.received;
        await page.unroute(`**${PATH}`);
        followupRead = await holdRequest(page, PATH, "GET", (route) => route.fulfill({ status: 503, json: { error: "post-delete read failed" } }));
        assert.equal((await submit(page, "delete")).status(), 200);
        await followupRead.received;
        oldRead.release();
        await oldRefresh;
        await page.locator('[data-workbench-data-page="browse"]').click();
        assert.equal(await row(page).count(), 0);
        assert.equal(await row(page, "other-result").count(), 1);
        assert.equal(await warning(page).count(), 0);
        assert.equal(await page.locator('[data-workbench-results="read-pending"]').count(), 1);
        followupRead.release();
        await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Result record deleted.");
        assert.match(await warning(page).innerText(), /post-delete read failed/u);
        await page.unroute(`**${PATH}`);
        await invoke(page, "runtime/refreshAll");
        await page.waitForLoadState("networkidle");
        assert.equal(await row(page).count(), 0);
        assertWrites(library, 1);
      } finally {
        oldRead.release();
        followupRead?.release();
        await oldRefresh.catch(() => {});
      }
    });
  });
}

test("Workbench result export download failure is reported accurately and releases its object URL", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openEditor(page);
    await page.evaluate(() => {
      window.__resultDownloadUrls = new Set();
      const create = URL.createObjectURL.bind(URL);
      const revoke = URL.revokeObjectURL.bind(URL);
      URL.createObjectURL = (blob) => { const url = create(blob); window.__resultDownloadUrls.add(url); return url; };
      URL.revokeObjectURL = (url) => { window.__resultDownloadUrls.delete(url); revoke(url); };
      HTMLAnchorElement.prototype.click = () => { throw new Error("result download blocked"); };
    });
    await page.locator('[data-workbench-data-action="export-result"]').click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "result download blocked", null, { timeout: 5_000 });
    assert.equal(await page.evaluate(() => window.__resultDownloadUrls.size), 0);
    assertWrites(library);
  });
});
