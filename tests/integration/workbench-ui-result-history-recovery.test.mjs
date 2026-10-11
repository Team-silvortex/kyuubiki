import assert from "node:assert/strict";
import { test } from "node:test";
import { workbenchUrl } from "./workbench-ui-isolated.shared.mjs";
import { usingWorkbench, invoke, openWorkbench, runtime,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "known-result";
const PATH = "/api/v1/results";
const record = (job_id = ID) => ({ job_id, result: { metric: 1 } });
const row = (page, id = ID) => page.locator(`[data-workbench-data-record-kind="result"][data-workbench-data-record-id="${id}"]`);
const warning = (page) => page.locator('[data-workbench-results="read-error"]');

function seed() {
  runtime.state.adminResults.push(record());
  runtime.state.resultRecordMutations.length = 0;
  runtime.state.jobRecordMutations.length = 0;
}

async function openResults(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: "results" }));
  await page.locator('[data-workbench-data-page="browse"]').click();
}

async function editResult(page) {
  await row(page).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
  const draft = page.locator('[data-workbench-data-field="result-payload"]');
  await draft.fill('{"metric":42,"note":"unsaved working copy"}');
  return draft;
}

async function refresh(page) {
  const request = page.waitForRequest((entry) => new URL(entry.url()).pathname === PATH && entry.method() === "GET");
  await page.locator('[data-workbench-data-action="refresh-results"]').click();
  await request;
  await page.waitForLoadState("networkidle");
}

function assertNoOtherWrites(library, resultWrites = 0) {
  assert.equal(library.writes.length, 0);
  assert.equal(runtime.state.projectMutations.length, 0);
  assert.equal(runtime.state.jobRecordMutations.length, 0);
  assert.equal(runtime.state.resultRecordMutations.length, resultWrites);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

for (const failure of ["http", "offline", "malformed", "invalid-entry", "duplicate-entry"]) {
  test(`Workbench result ${failure} refresh retains selection and unsaved draft with explicit recovery`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      let mode = "ready";
      let reads = 0;
      await page.route(`**${PATH}`, (route) => {
        reads += 1;
        if (mode === "offline") return route.abort("failed");
        if (mode === "http") return route.fulfill({ status: 503, json: { error: "qualification results unavailable" } });
        return route.fulfill({ json: mode === "malformed" ? { results: null }
          : mode === "invalid-entry" ? { results: [record(), { job_id: "invalid", result: null }] }
          : mode === "duplicate-entry" ? { results: [record(), record()] }
          : { results: runtime.state.adminResults } });
      });
      await openWorkbench(page);
      await openResults(page);
      const draft = await editResult(page);
      const working = await draft.inputValue();
      mode = failure;
      const before = reads;
      await refresh(page);
      await warning(page).waitFor({ state: "visible" });
      assert.equal(reads, before + 1, "failed reads must not automatically retry");
      assert.equal(await draft.inputValue(), working);
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, ID);
      assert.match(await warning(page).textContent(), failure === "http" ? /qualification results unavailable/u
        : failure === "offline" ? /fetch|network|load failed/iu : /RESULT_HISTORY_INVALID/u);
      await page.locator('[data-workbench-data-page="browse"]').click();
      assert.equal(await row(page).count(), 1);
      assert.equal(await page.locator('[data-workbench-results="empty"]').count(), 0);
      mode = "ready";
      runtime.state.adminResults.push(record("recovered-result"));
      await refresh(page);
      await row(page, "recovered-result").waitFor({ state: "visible" });
      assert.equal(await warning(page).count(), 0);
      await page.locator('[data-workbench-data-page="edit"]').click();
      assert.equal(await draft.inputValue(), working, "read recovery must not discard unsaved edits");
      await page.locator('[data-workbench-data-page="browse"]').click();
      runtime.state.adminResults.length = 0;
      await refresh(page);
      await page.locator('[data-workbench-results="empty"]').waitFor({ state: "visible" });
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, null);
      assertNoOtherWrites(library);
    });
  });
}

test("Workbench result first-read failure is not empty and the runtime Retry All button really recovers results", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    let unavailable = true;
    let reads = 0;
    await page.route(`**${PATH}`, (route) => {
      reads += 1;
      return route.fulfill(unavailable ? { status: 503, json: { error: "first result read unavailable" } }
        : { json: { results: runtime.state.adminResults } });
    });
    await openWorkbench(page);
    await openResults(page);
    await warning(page).waitFor({ state: "visible" });
    assert.equal(await page.locator('[data-workbench-results="empty"]').count(), 0);
    unavailable = false;
    const before = reads;
    await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
    await page.locator('[data-workbench-runtime-tab="watchdog"]').click();
    await page.locator('[data-workbench-recovery-action="retry-all"]').click();
    await page.waitForLoadState("networkidle");
    await openResults(page);
    await row(page).waitFor({ state: "visible" });
    assert.equal(reads, before + 1);
    assert.equal(await warning(page).count(), 0);
    assertNoOtherWrites(library);
  });
});

test("Workbench successful refresh preserves unsaved result edits but selection changes load the selected record", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    runtime.state.adminResults.push({ job_id: "other-result", result: { metric: 17 } });
    await openWorkbench(page);
    await openResults(page);
    const draft = await editResult(page);
    const working = await draft.inputValue();
    runtime.state.adminResults[0].result.metric = 2;
    runtime.state.adminResults.push(record("fresh-list-marker"));
    await refresh(page);
    await page.locator('[data-workbench-data-page="browse"]').click();
    await row(page, "fresh-list-marker").waitFor({ state: "visible" });
    await page.locator('[data-workbench-data-page="edit"]').click();
    assert.equal(await draft.inputValue(), working, "valid background data must not replace an unsaved draft");
    await page.locator('[data-workbench-data-page="browse"]').click();
    await row(page, "other-result").click();
    await page.locator('[data-workbench-data-page="edit"]').click();
    assert.equal(JSON.parse(await draft.inputValue()).metric, 17);
    await page.locator('[data-workbench-data-page="browse"]').click();
    await row(page).click();
    await page.locator('[data-workbench-data-page="edit"]').click();
    assert.equal(JSON.parse(await draft.inputValue()).metric, 2, "explicit record selection loads its latest verified value");
    runtime.state.adminResults[0].result.metric = 3;
    await refresh(page);
    await page.waitForFunction(() => JSON.parse(document.querySelector('[data-workbench-data-field="result-payload"]').value).metric === 3);
    assertNoOtherWrites(library);
  });
});

test("Workbench result pending first read shows loading rather than empty history", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    await page.route(`**${PATH}`, async (route) => { await gate; await route.fulfill({ json: { results: [] } }); });
    try {
      await page.goto(workbenchUrl(runtime), { waitUntil: "domcontentloaded" });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await openResults(page);
      await page.locator('[data-workbench-results="read-pending"]').waitFor({ state: "visible" });
      assert.equal(await page.locator('[data-workbench-data-admin="panel"]').getAttribute("aria-busy"), "true");
      assert.equal(await page.locator('[data-workbench-results="empty"]').count(), 0);
      release();
      await page.locator('[data-workbench-results="empty"]').waitFor({ state: "visible" });
      assert.equal(await page.locator('[data-workbench-data-admin="panel"]').getAttribute("aria-busy"), "false");
      assertNoOtherWrites(library);
    } finally { release(); }
  });
});

for (const staleFails of [false, true]) {
  for (const latestFails of [false, true]) {
    test(`Workbench result late ${staleFails ? "failure" : "success"} cannot overwrite newer ${latestFails ? "failure" : "success"}`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        seed();
        let mode = "initial";
        let release;
        let reached;
        const gate = new Promise((resolve) => { release = resolve; });
        const received = new Promise((resolve) => { reached = resolve; });
        await page.route(`**${PATH}`, async (route) => {
          if (mode === "stale") {
            reached();
            await gate;
            return route.fulfill(staleFails ? { status: 503, json: { error: "stale result unavailable" } }
              : { json: { results: [record("stale-result")] } });
          }
          return route.fulfill(mode === "latest" && latestFails ? { status: 503, json: { error: "latest result unavailable" } }
            : { json: { results: [record(mode === "initial" ? ID : "latest-result")] } });
        });
        await openWorkbench(page);
        await openResults(page);
        await row(page).waitFor({ state: "visible" });
        try {
          mode = "stale";
          await page.locator('[data-workbench-data-action="refresh-results"]').click();
          await received;
          mode = "latest";
          const response = page.waitForResponse((entry) => new URL(entry.url()).pathname === PATH);
          await page.locator('[data-workbench-data-action="refresh-results"]').click();
          await response;
          if (latestFails) await warning(page).waitFor({ state: "visible" });
          else await row(page, "latest-result").waitFor({ state: "visible" });
          release();
          await page.waitForLoadState("networkidle");
          assert.equal(await row(page, latestFails ? ID : "latest-result").count(), 1);
          assert.equal(await row(page, "stale-result").count(), 0);
          assert.equal(await warning(page).count(), Number(latestFails));
          if (latestFails) assert.match(await warning(page).textContent(), /latest result unavailable/u);
          assert.equal(await page.locator('[data-workbench-data-admin="panel"]').getAttribute("aria-busy"), "false");
          assertNoOtherWrites(library);
        } finally { release(); }
      });
    });
  }
}

for (const action of ["save", "delete"]) {
  test(`Workbench confirmed result ${action} remains successful when the follow-up list fails`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      runtime.state.adminResults.push(record("other-result"));
      await openWorkbench(page);
      await openResults(page);
      const draft = await editResult(page);
      const working = await draft.inputValue();
      await page.route(`**${PATH}`, (route) => route.fulfill({ status: 503, json: { error: "post-write list unavailable" } }));
      const response = page.waitForResponse((entry) => new URL(entry.url()).pathname === `${PATH}/${ID}` &&
        entry.request().method() === (action === "save" ? "PATCH" : "DELETE"));
      await page.locator(`[data-workbench-data-action="${action}-result"]`).click();
      assert.equal((await response).status(), 200);
      await warning(page).waitFor({ state: "visible" });
      await page.waitForFunction((needle) => window.__kyuubikiPwdt.state().message.toLowerCase().includes(needle), action === "save" ? "updated" : "deleted");
      if (action === "save") assert.equal(await draft.inputValue(), working);
      else assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, null);
      await page.locator('[data-workbench-data-page="browse"]').click();
      assert.equal(await row(page, "other-result").count(), 1);
      assert.equal(await row(page).count(), action === "save" ? 1 : 0);
      await page.unroute(`**${PATH}`);
      await invoke(page, "runtime/refreshAll");
      await row(page, "other-result").waitFor({ state: "visible" });
      assert.equal(await warning(page).count(), 0);
      assertNoOtherWrites(library, 1);
    });
  });
}

for (const action of ["save", "delete"]) {
  test(`Workbench failed result ${action} preserves its draft and never automatically repeats the write`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openResults(page);
      const draft = await editResult(page);
      const working = await draft.inputValue();
      let reads = 0;
      let attempts = 0;
      await page.route(`**${PATH}`, (route) => { reads += 1; return route.fallback(); });
      await page.route(`**${PATH}/${ID}`, (route) => {
        attempts += 1;
        return route.fulfill({ status: 503, json: { error: "result write unavailable" } });
      });
      await page.locator(`[data-workbench-data-action="${action}-result"]`).click();
      await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "result write unavailable");
      await page.waitForLoadState("networkidle");
      assert.equal(await draft.inputValue(), working);
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminResultJobId, ID);
      assert.equal(attempts, 1);
      assert.equal(reads, 0, "a rejected write must not pretend to refresh a committed change");
      await refresh(page);
      assert.equal(attempts, 1, "read recovery must not replay failed writes");
      assert.equal(reads, 1);
      assertNoOtherWrites(library);
    });
  });
}
