import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, runtime, holdRequest, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "known-job";
const PATH = "/api/v1/jobs";
const keys = { message: "message", project: "project_id", version: "model_version_id", case: "simulation_case_id" };
const record = (job_id = ID, prefix = "initial") => ({ job_id, status: "completed", progress: 1,
  message: `${prefix} message`, project_id: `${prefix}-project`, model_version_id: `${prefix}-version`,
  simulation_case_id: `${prefix}-case`, worker_id: "qualification-agent", has_result: false });
const row = (page, id = ID) => page.locator(`[data-workbench-data-record-kind="job"][data-workbench-data-record-id="${id}"]`);
const field = (page, name) => page.locator(`[data-workbench-data-field="job-${name}"]`);

function seed() {
  runtime.state.adminJobs.push(record(), record("other-job", "other"));
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
}

async function openJobs(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: "jobs" }));
  await page.locator('[data-workbench-data-page="browse"]').click();
  await row(page).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
  await page.waitForFunction(() => document.querySelector('[data-workbench-data-field="job-message"]')?.value === "initial message");
}

async function fillDraft(page, prefix = "unsaved") {
  for (const name of Object.keys(keys)) await field(page, name).fill(`${prefix}-${name}`);
}

async function assertDraft(page, values, label = "draft") {
  for (const [name, value] of Object.entries(values)) assert.equal(await field(page, name).inputValue(), value, `${label}: ${name}`);
}

const working = (prefix = "unsaved") => Object.fromEntries(Object.keys(keys).map((name) => [name, `${prefix}-${name}`]));
const values = (prefix) => ({ message: `${prefix} message`, project: `${prefix}-project`, version: `${prefix}-version`, case: `${prefix}-case` });

async function observePublishedHistory(page, marker) {
  await page.locator('[data-workbench-data-page="browse"]').click();
  await row(page, marker).waitFor({ state: "visible" });
  await page.locator('[data-workbench-data-page="edit"]').click();
}

function assertWrites(library, jobs = 0) {
  assert.equal(library.writes.length, 0);
  assert.equal(runtime.state.projectMutations.length, 0);
  assert.equal(runtime.state.jobRecordMutations.length, jobs);
  assert.equal(runtime.state.resultRecordMutations.length, 0);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

for (const dirty of Object.keys(keys)) {
  test(`Workbench job refresh retains dirty ${dirty} while clean fields follow verified data`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openJobs(page);
      await field(page, dirty).fill(`unsaved-${dirty}`);
      runtime.state.adminJobs[0] = record(ID, "refreshed");
      runtime.state.adminJobs.push(record("refresh-marker"));
      await invoke(page, "runtime/refreshAll");
      await observePublishedHistory(page, "refresh-marker");
      for (const [name, key] of Object.entries(keys)) {
        assert.equal(await field(page, name).inputValue(), name === dirty ? `unsaved-${dirty}` : runtime.state.adminJobs[0][key], name);
      }
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminJobId, ID);
      assertWrites(library);
    });
  });
}

for (const failure of ["http", "offline", "malformed", "invalid-entry", "duplicate-entry"]) {
  test(`Workbench job ${failure} read and recovery preserve all unsaved metadata fields`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      let mode = "ready";
      let reads = 0;
      await page.route(`**${PATH}`, (route) => {
        reads += 1;
        if (mode === "offline") return route.abort("failed");
        if (mode === "http") return route.fulfill({ status: 503, json: { error: "job history unavailable" } });
        return route.fulfill({ json: mode === "malformed" ? { jobs: null }
          : mode === "invalid-entry" ? { jobs: [record(), { job_id: "invalid", status: "completed", progress: 2 }] }
          : mode === "duplicate-entry" ? { jobs: [record(), record()] }
          : { jobs: runtime.state.adminJobs } });
      });
      await openWorkbench(page);
      await openJobs(page);
      await fillDraft(page);
      mode = failure;
      const before = reads;
      await invoke(page, "runtime/refreshAll");
      await page.waitForLoadState("networkidle");
      assert.equal(reads, before + 1, "failed reads must not automatically retry");
      await assertDraft(page, working());
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminJobId, ID);
      await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
      await page.locator('[data-workbench-runtime-tab="watchdog"]').click();
      await page.locator('[data-workbench-recovery-action="retry-all"]').waitFor({ state: "visible" });
      mode = "ready";
      runtime.state.adminJobs[0] = record(ID, "recovered");
      runtime.state.adminJobs.push(record("recovery-marker"));
      const retryReads = reads;
      await page.locator('[data-workbench-recovery-action="retry-all"]').click();
      await page.waitForLoadState("networkidle");
      await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: "jobs" }));
      await observePublishedHistory(page, "recovery-marker");
      assert.equal(reads, retryReads + 1);
      await assertDraft(page, working());
      assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminJobId, ID);
      assertWrites(library);
    });
  });
}

test("Workbench explicit job selection loads the current record and a verified empty list clears selection", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openJobs(page);
    await fillDraft(page);
    runtime.state.adminJobs[0] = record(ID, "latest");
    runtime.state.adminJobs.push(record("selection-marker"));
    await invoke(page, "runtime/refreshAll");
    await observePublishedHistory(page, "selection-marker");
    await assertDraft(page, working());
    for (const [id, prefix] of [["other-job", "other"], [ID, "latest"]]) {
      await page.locator('[data-workbench-data-page="browse"]').click();
      await row(page, id).click();
      await page.locator('[data-workbench-data-page="edit"]').click();
      await page.waitForFunction((expected) => document.querySelector('[data-workbench-data-field="job-message"]')?.value === expected, `${prefix} message`);
      await assertDraft(page, values(prefix));
    }
    runtime.state.adminJobs.length = 0;
    await invoke(page, "runtime/refreshAll");
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedAdminJobId: null }));
    for (const name of Object.keys(keys)) assert.equal(await field(page, name).count(), 0);
    assertWrites(library);
  });
});

test("Workbench confirmed job save stays successful and retains draft through failed follow-up history", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openJobs(page);
    await fillDraft(page);
    await page.route(`**${PATH}`, (route) => route.fulfill({ status: 503, json: { error: "post-save history unavailable" } }));
    await page.locator('[data-workbench-data-action="save-job"]').click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "Job record updated.");
    await assertDraft(page, working());
    assert.equal(runtime.state.jobRecordMutations.length, 1);
    assert.deepEqual(runtime.state.jobRecordMutations[0].body, {
      message: "unsaved-message", project_id: "unsaved-project", model_version_id: "unsaved-version", simulation_case_id: "unsaved-case",
    });
    await page.unroute(`**${PATH}`);
    runtime.state.adminJobs.push(record("save-recovery-marker"));
    await invoke(page, "runtime/refreshAll");
    await observePublishedHistory(page, "save-recovery-marker");
    await assertDraft(page, working());
    runtime.state.adminJobs[0] = record(ID, "subsequent");
    runtime.state.adminJobs.push(record("clean-after-save-marker"));
    await invoke(page, "runtime/refreshAll");
    await observePublishedHistory(page, "clean-after-save-marker");
    await assertDraft(page, values("subsequent"), "a confirmed saved draft must become clean after read reconciliation");
    assertWrites(library, 1);
  });
});

test("Workbench rejected job save preserves draft without a follow-up read or repeated write", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openJobs(page);
    await fillDraft(page);
    let reads = 0;
    let attempts = 0;
    await page.route(`**${PATH}`, (route) => { reads += 1; return route.fallback(); });
    await page.route(`**${PATH}/${ID}`, (route) => {
      attempts += 1;
      return route.fulfill({ status: 503, json: { error: "job write unavailable" } });
    });
    await page.locator('[data-workbench-data-action="save-job"]').click();
    await page.waitForFunction(() => window.__kyuubikiPwdt.state().message === "job write unavailable");
    await assertDraft(page, working());
    assert.equal(reads, 0);
    assert.equal(attempts, 1);
    runtime.state.adminJobs[0] = record(ID, "refreshed");
    runtime.state.adminJobs.push(record("rejected-marker"));
    await invoke(page, "runtime/refreshAll");
    await observePublishedHistory(page, "rejected-marker");
    await assertDraft(page, working());
    assert.equal(reads, 1);
    assert.equal(attempts, 1, "read recovery must not replay the failed write");
    assertWrites(library);
  });
});

for (const switchRecord of [false, true]) {
  test(`Workbench slow job save cannot replace newer typing${switchRecord ? " on another selected record" : " on the same record"}`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openJobs(page);
      await fillDraft(page, "submitted");
      const held = await holdRequest(page, `${PATH}/${ID}`, "PATCH");
      try {
        await page.locator('[data-workbench-data-action="save-job"]').click();
        await held.received;
        if (switchRecord) {
          await page.locator('[data-workbench-data-page="browse"]').click();
          await row(page, "other-job").click();
          await page.locator('[data-workbench-data-page="edit"]').click();
          await page.waitForFunction(() => document.querySelector('[data-workbench-data-field="job-message"]')?.value === "other message");
        }
        await fillDraft(page, "newer");
        runtime.state.adminJobs.push(record("post-save-marker"));
        const newerMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
        held.release();
        await observePublishedHistory(page, "post-save-marker");
        await waitForGuiTransition(page);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), switchRecord ? newerMessage : "Job record updated.");
        await assertDraft(page, working("newer"));
        assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminJobId, switchRecord ? "other-job" : ID);
        assert.equal(runtime.state.adminJobs[0].message, "submitted-message", "the write uses the submitted snapshot, not later input");
        assert.equal(runtime.state.adminJobs[1].message, "other message");
        assertWrites(library, 1);
      } finally { held.release(); }
    });
  });
}

for (const staleFails of [false, true]) {
  for (const latestFails of [false, true]) {
    test(`Workbench dirty job late ${staleFails ? "failure" : "success"} cannot replace newer ${latestFails ? "failure" : "success"}`, { timeout: 75_000 }, async () => {
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
            return route.fulfill(staleFails ? { status: 503, json: { error: "stale job unavailable" } }
              : { json: { jobs: [record("stale-job")] } });
          }
          return route.fulfill(mode === "latest" && latestFails ? { status: 503, json: { error: "latest job unavailable" } }
            : { json: { jobs: mode === "initial" ? runtime.state.adminJobs : [record(ID, "latest"), record("latest-marker")] } });
        });
        await openWorkbench(page);
        await openJobs(page);
        await fillDraft(page);
        let old;
        try {
          mode = "stale";
          old = invoke(page, "runtime/refreshAll");
          void old.catch(() => {});
          await received;
          mode = "latest";
          await invoke(page, "runtime/refreshAll");
          if (!latestFails) await observePublishedHistory(page, "latest-marker");
          release();
          await old;
          await page.waitForLoadState("networkidle");
          await assertDraft(page, working());
          assert.equal((await page.evaluate(() => window.__kyuubikiPwdt.state())).selectedAdminJobId, ID);
          await page.locator('[data-workbench-data-page="browse"]').click();
          assert.equal(await row(page).count(), 1);
          assert.equal(await row(page, "stale-job").count(), 0);
          assert.equal(await row(page, "latest-marker").count(), Number(!latestFails));
          await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
          await page.locator('[data-workbench-runtime-tab="watchdog"]').click();
          assert.equal(await page.getByText("latest job unavailable", { exact: true }).count(), Number(latestFails));
          assert.equal(await page.getByText("stale job unavailable", { exact: true }).count(), 0);
          assertWrites(library);
        } finally { release(); await old; }
      });
    });
  }
}
