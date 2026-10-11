import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, runtime, holdRequest,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "record-job";
const JOBS = "/api/v1/jobs";
const RESULTS = "/api/v1/results";
const record = (job_id = ID) => ({ job_id, status: "completed", progress: 1, message: "initial message",
  project_id: "qualification-project", model_version_id: "initial-version", simulation_case_id: "initial-case", has_result: true });
const row = (page, kind, id = ID) => page.locator(`[data-workbench-data-record-kind="${kind}"][data-workbench-data-record-id="${id}"]`);
const draft = (page) => page.locator('[data-workbench-data-field="job-message"]');

function seed() {
  runtime.state.adminJobs.push(record(), record("other-job"));
  runtime.state.adminResults.push(...[ID, "other-job"].map((job_id) => ({ job_id, result: { metric: 1 } })));
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
}

async function openData(page, kind, id = ID) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate((kind) => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: `${kind}s` }), kind);
  await page.locator('[data-workbench-data-page="browse"]').click();
  if (id) await row(page, kind, id).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}

async function submit(page, action) {
  const response = page.waitForResponse((entry) => new URL(entry.url()).pathname === `${JOBS}/${ID}` &&
    entry.request().method() === (action === "save" ? "PATCH" : "DELETE"));
  await page.locator(`[data-workbench-data-action="${action}-job"]`).click();
  return response;
}

function assertWrites(library, jobs = 0) {
  assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
  assert.equal(runtime.state.jobRecordMutations.length, jobs);
  assert.equal(runtime.state.resultRecordMutations.length, 0);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

test("Workbench clears optional job version with explicit null and read recovery never replays the save", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openData(page, "job");
    await page.locator('[data-workbench-data-field="job-version"]').fill("");
    assert.equal((await submit(page, "save")).status(), 200);
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Job record updated.");
    assert.equal(runtime.state.adminJobs[0].model_version_id, null);
    assert.equal(runtime.state.jobRecordMutations[0].body.model_version_id, null);
    await invoke(page, "runtime/refreshAll");
    assert.equal(await page.locator('[data-workbench-data-field="job-version"]').inputValue(), "");
    assertWrites(library, 1);
  });
});

for (const field of ["project", "case"]) for (const value of ["", "  "]) {
  test(`Workbench cannot silently omit a ${value ? "whitespace-only" : "cleared"} required job ${field}`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, "job");
      await page.locator(`[data-workbench-data-field="job-${field}"]`).fill(value);
      assert.equal((await submit(page, "save")).status(), 422);
      await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "required job context cannot be empty");
      assert.equal(await page.locator(`[data-workbench-data-field="job-${field}"]`).inputValue(), value);
      assert.equal(runtime.state.adminJobs[0].project_id, "qualification-project");
      assert.equal(runtime.state.adminJobs[0].simulation_case_id, "initial-case");
      await invoke(page, "runtime/refreshAll");
      assert.equal(await page.locator(`[data-workbench-data-field="job-${field}"]`).inputValue(), value);
      assertWrites(library);
    });
  });
}

test("Workbench confirmed job deletion removes only its job and result even when both follow-up reads fail", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openData(page, "result");
    await openData(page, "job");
    await page.route(`**${JOBS}`, (route) => route.fulfill({ status: 503, json: { error: "post-delete jobs unavailable" } }));
    await page.route(`**${RESULTS}`, (route) => route.fulfill({ status: 503, json: { error: "post-delete results unavailable" } }));
    assert.equal((await submit(page, "delete")).status(), 200);
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Job record deleted.");
    const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(state.selectedAdminJobId, null);
    assert.equal(state.selectedAdminResultJobId, null);
    await page.locator('[data-workbench-data-page="browse"]').click();
    assert.equal(await row(page, "job").count(), 0);
    assert.equal(await row(page, "job", "other-job").count(), 1);
    await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemDataTab: "results" }));
    assert.equal(await row(page, "result").count(), 0);
    assert.equal(await row(page, "result", "other-job").count(), 1);
    await page.locator('[data-workbench-results="read-error"]').waitFor({ state: "visible" });
    await page.unroute(`**${JOBS}`);
    await page.unroute(`**${RESULTS}`);
    await invoke(page, "runtime/refreshAll");
    await row(page, "result", "other-job").waitFor({ state: "visible" });
    assert.equal(await row(page, "result").count(), 0);
    assert.equal(await page.locator('[data-workbench-results="read-error"]').count(), 0);
    assertWrites(library, 1);
  });
});

test("Workbench delayed job deletion preserves another selected job and result's unsaved editing", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openData(page, "job");
    const held = await holdRequest(page, `${JOBS}/${ID}`, "DELETE");
    try {
      const response = submit(page, "delete");
      await held.received;
      await openData(page, "result", "other-job");
      await page.locator('[data-workbench-data-field="result-payload"]').fill('{"metric":42}');
      await openData(page, "job", "other-job");
      await draft(page).fill("other unsaved message");
      held.release();
      assert.equal((await response).status(), 200);
      await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Job record deleted.");
      assert.equal(await draft(page).inputValue(), "other unsaved message");
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.selectedAdminJobId, "other-job");
      assert.equal(state.selectedAdminResultJobId, "other-job");
      await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemDataTab: "results" }));
      assert.equal(await page.locator('[data-workbench-data-field="result-payload"]').inputValue(), '{"metric":42}');
      assertWrites(library, 1);
    } finally { held.release(); }
  });
});

for (const oldJobFails of [false, true]) for (const oldResultFails of [false, true]) {
  test(`Workbench confirmed deletion fences old job ${oldJobFails ? "failure" : "success"} and result ${oldResultFails ? "failure" : "success"}`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, "result");
      await openData(page, "job");
      const oldJobs = structuredClone(runtime.state.adminJobs);
      const oldResults = structuredClone(runtime.state.adminResults);
      const oldJobRead = await holdRequest(page, JOBS, "GET", (route) => route.fulfill(oldJobFails
        ? { status: 503, json: { error: "stale jobs failure" } } : { json: { jobs: oldJobs } }));
      const oldResultRead = await holdRequest(page, RESULTS, "GET", (route) => route.fulfill(oldResultFails
        ? { status: 503, json: { error: "stale results failure" } } : { json: { results: oldResults } }));
      let followupJobRead;
      const oldRefresh = invoke(page, "runtime/refreshAll");
      void oldRefresh.catch(() => {});
      try {
        await Promise.all([oldJobRead.received, oldResultRead.received]);
        await page.unroute(`**${JOBS}`);
        await page.unroute(`**${RESULTS}`);
        followupJobRead = await holdRequest(page, JOBS, "GET", (route) => route.fulfill({
          status: 503, json: { error: "post-delete jobs unavailable" },
        }));
        await page.route(`**${RESULTS}`, (route) => route.fulfill({
          status: 503, json: { error: "post-delete results unavailable" },
        }));
        assert.equal((await submit(page, "delete")).status(), 200);
        await followupJobRead.received;
        await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemDataTab: "results" }));
        await page.locator('[data-workbench-data-page="browse"]').click();
        await page.locator('[data-workbench-results="read-error"]').waitFor({ state: "visible" });
        oldJobRead.release();
        oldResultRead.release();
        await oldRefresh;
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.jobHistoryCount, 1);
        assert.equal(state.resultCount, 1);
        assert.equal(state.selectedAdminJobId, null);
        assert.equal(state.selectedAdminResultJobId, null);
        assert.equal(await row(page, "result").count(), 0);
        assert.equal(await row(page, "result", "other-job").count(), 1);
        assert.match(await page.locator('[data-workbench-results="read-error"]').innerText(), /post-delete results unavailable/u);
        followupJobRead.release();
        await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Job record deleted.");
        await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemDataTab: "jobs" }));
        assert.equal(await row(page, "job").count(), 0);
        assert.equal(await row(page, "job", "other-job").count(), 1);
        await page.unroute(`**${JOBS}`);
        await page.unroute(`**${RESULTS}`);
        await invoke(page, "runtime/refreshAll");
        await page.waitForLoadState("networkidle");
        assert.equal(await row(page, "job").count(), 0);
        assertWrites(library, 1);
      } finally {
        oldJobRead.release();
        oldResultRead.release();
        followupJobRead?.release();
        await oldRefresh.catch(() => {});
      }
    });
  });
}

const invalidResponses = [
  ["null", null], ["wrong-job", { job: record("wrong-job"), deleted: true }],
  ["invalid-status", { job: { ...record(), status: "unknown" }, deleted: true }],
  ["invalid-progress", { job: { ...record(), progress: 2 }, deleted: true }],
];
for (const action of ["save", "delete"]) {
  const failures = action === "delete" ? [...invalidResponses,
    ["missing-deleted", { job: record() }], ["false-deleted", { job: record(), deleted: false }]] : invalidResponses;
  for (const [label, payload] of failures) {
    test(`Workbench ${action} ${label} acknowledgement cannot confirm a job mutation or trigger read recovery`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        seed();
        await openWorkbench(page);
        await openData(page, "job");
        await draft(page).fill("unsaved review");
        let reads = 0;
        let attempts = 0;
        await page.route(`**${JOBS}`, (route) => { reads += 1; return route.fallback(); });
        await page.route(`**${RESULTS}`, (route) => { reads += 1; return route.fallback(); });
        await page.route(`**${JOBS}/${ID}`, (route) => { attempts += 1; return route.fulfill({ json: payload }); });
        assert.equal((await submit(page, action)).status(), 200);
        await page.waitForFunction(() => window.__kyuubikiPwdt.state().message.includes("JOB_RECORD_MUTATION_UNCONFIRMED"));
        await page.waitForLoadState("networkidle");
        assert.match((await page.evaluate(() => window.__kyuubikiPwdt.state())).message, /JOB_RECORD_MUTATION_UNCONFIRMED/u);
        assert.equal(await draft(page).inputValue(), "unsaved review");
        assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminJobId, ID);
        assert.equal(reads, 0, "an unconfirmed write must not publish a successful follow-up read");
        assert.equal(attempts, 1);
        await invoke(page, "runtime/refreshAll");
        assert.equal(attempts, 1, "read recovery must not replay an ambiguous mutation");
        assert.equal(reads, 2);
        assertWrites(library);
      });
    });
  }
}

test("Workbench rejected job deletion retains records and does not automatically retry", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openData(page, "job");
    await draft(page).fill("unsaved review");
    let reads = 0;
    let attempts = 0;
    await page.route(`**${JOBS}`, (route) => { reads += 1; return route.fallback(); });
    await page.route(`**${JOBS}/${ID}`, (route) => {
      attempts += 1;
      return route.fulfill({ status: 503, json: { error: "job deletion unavailable" } });
    });
    assert.equal((await submit(page, "delete")).status(), 503);
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "job deletion unavailable");
    assert.equal(await draft(page).inputValue(), "unsaved review");
    assert.equal(reads, 0);
    await invoke(page, "runtime/refreshAll");
    assert.equal(attempts, 1);
    assert.equal(reads, 1);
    assertWrites(library);
  });
});
