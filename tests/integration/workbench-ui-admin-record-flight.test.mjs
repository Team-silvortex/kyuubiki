import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, usingWorkbench, openWorkbench, invoke, holdRequest, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "flight-job";
const OTHER = "other-flight-job";
const path = (kind) => `/api/v1/${kind === "job" ? "jobs" : "results"}`;
const button = (page, kind, action) => page.locator(`[data-workbench-data-action="${action}-${kind}"]`);
const row = (page, kind, id = ID) => page.locator(`[data-workbench-data-record-kind="${kind}"][data-workbench-data-record-id="${id}"]`);
const field = (page, kind) => page.locator(`[data-workbench-data-field="${kind === "job" ? "job-message" : "result-payload"}"]`);
const working = (kind) => kind === "job" ? "working message" : '{"metric":42}';
const success = (kind, action) => `${kind === "job" ? "Job" : "Result"} record ${action === "save" ? "updated" : "deleted"}.`;

function seed() {
  for (const job_id of [ID, OTHER]) {
    runtime.state.adminJobs.push({ job_id, status: "completed", progress: 1, message: "initial message",
      project_id: "qualification-project", model_version_id: "initial-version", simulation_case_id: "initial-case", has_result: true });
    runtime.state.adminResults.push({ job_id, result: { metric: 1 } });
  }
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
}

async function openData(page, kind, id = ID) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate((kind) => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: `${kind}s` }), kind);
  await page.locator('[data-workbench-data-page="browse"]').click();
  await row(page, kind, id).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}

async function assertLocked(page, kind, locked = true) {
  await page.waitForFunction(({ kind, locked }) => ["save", "delete"].every((action) =>
    document.querySelector(`[data-workbench-data-action="${action}-${kind}"]`)?.disabled === locked),
  { kind, locked }, { timeout: 5_000 });
}

function assertWrites(library, jobs, results) {
  assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
  assert.equal(runtime.state.jobRecordMutations.length, jobs);
  assert.equal(runtime.state.resultRecordMutations.length, results);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

for (const kind of ["job", "result"]) for (const rejected of [false, true]) {
  test(`Workbench ${kind} save owns one flight through ${rejected ? "rejection" : "blocked follow-up read"}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      await field(page, kind).fill(working(kind));
      let attempts = 0;
      const write = await holdRequest(page, `${path(kind)}/${ID}`, "PATCH", (route) => rejected
        ? route.fulfill({ status: 503, json: { error: "record write unavailable" } }) : route.fallback());
      await page.route(`**${path(kind)}/${ID}`, (route) => {
        if (route.request().method() === "PATCH") attempts += 1;
        return route.fallback();
      });
      let read;
      try {
        // Two synchronous activations also exercise the guard before React disables the button.
        await button(page, kind, "save").evaluate((element) => { element.click(); element.click(); });
        await write.received;
        await assertLocked(page, kind);
        await waitForGuiTransition(page, true);
        if (!rejected) {
          read = await holdRequest(page, path(kind), "GET", (route) => route.fulfill({ status: 503, json: { error: "post-save read unavailable" } }));
          write.release();
          await read.received;
          await assertLocked(page, kind);
          await field(page, kind).fill(kind === "job" ? "newer typing" : '{"metric":99}');
          await button(page, kind, "delete").evaluate((element) => element.click());
          read.release();
        } else write.release();
        await page.waitForFunction((message) => window.__kyuubikiPwdt.state().message === message,
          rejected ? "record write unavailable" : success(kind, "save"));
        await assertLocked(page, kind, false);
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        assert.equal(attempts, 1, "synchronous duplicate activation must not replay a record write");
        assert.equal(await field(page, kind).inputValue(), rejected ? working(kind) : kind === "job" ? "newer typing" : '{"metric":99}');
        assertWrites(library, kind === "job" && !rejected ? 1 : 0, kind === "result" && !rejected ? 1 : 0);
        await page.unroute(`**${path(kind)}`);
        await invoke(page, "runtime/refreshAll");
        assert.equal(attempts, 1, "read recovery never replays a finished record write");
      } finally { write.release(); read?.release(); }
    });
  });
}

for (const kind of ["job", "result"]) {
  test(`Workbench pending ${kind} save blocks its paired record but another identity remains writable after remount`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      const paired = kind === "job" ? "result" : "job";
      const held = await holdRequest(page, `${path(kind)}/${ID}`, "PATCH");
      try {
        await button(page, kind, "save").click();
        await held.received;
        await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
        await openData(page, paired);
        await assertLocked(page, paired);
        await button(page, paired, "delete").evaluate((element) => element.click());
        await openData(page, paired, OTHER);
        await assertLocked(page, paired, false);
        await field(page, paired).fill(working(paired));
        await button(page, paired, "save").click();
        await page.waitForFunction((message) => window.__kyuubikiPwdt.state().message === message, success(paired, "save"));
        await assertLocked(page, paired, false);
        await openData(page, kind);
        await assertLocked(page, kind);
        const newerMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
        held.release();
        await assertLocked(page, kind, false);
        await waitForGuiTransition(page);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), newerMessage);
        assertWrites(library, 1, 1);
        assert.ok(runtime.state.adminJobs.some((entry) => entry.job_id === ID));
        assert.ok(runtime.state.adminResults.some((entry) => entry.job_id === ID));
      } finally { held.release(); }
    });
  });
}

for (const kind of ["job", "result"]) for (const readFails of [false, true]) for (const otherSelection of [false, true]) {
  test(`Workbench ${kind} deletion removes visible rows while refresh waits; readFails=${readFails}; otherSelection=${otherSelection}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, "result");
      await openData(page, kind);
      const write = await holdRequest(page, `${path(kind)}/${ID}`, "DELETE");
      const read = await holdRequest(page, path(kind), "GET", (route) => readFails
        ? route.fulfill({ status: 503, json: { error: "post-delete read unavailable" } }) : route.fallback());
      if (kind === "job") await page.route("**/api/v1/results", (route) => route.fulfill({ status: 503, json: { error: "paired read unavailable" } }));
      try {
        await button(page, kind, "delete").click();
        await write.received;
        await assertLocked(page, kind);
        if (otherSelection) { await openData(page, kind, OTHER); await field(page, kind).fill(working(kind)); }
        write.release();
        await read.received;
        await page.locator('[data-workbench-data-page="browse"]').click();
        await row(page, kind).waitFor({ state: "detached", timeout: 5_000 });
        assert.equal(await row(page, kind, OTHER).count(), 1);
        if (kind === "job") {
          await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemDataTab: "results" }));
          await row(page, "result").waitFor({ state: "detached", timeout: 5_000 });
          assert.equal(await row(page, "result", OTHER).count(), 1);
          await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemDataTab: "jobs" }));
        }
        const newerMessage = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
        read.release();
        await waitForGuiTransition(page);
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message),
          kind === "job" || otherSelection ? newerMessage : success(kind, "delete"));
        if (otherSelection) {
          await page.locator('[data-workbench-data-page="edit"]').click();
          assert.equal(await field(page, kind).inputValue(), working(kind));
          await assertLocked(page, kind, false);
        }
        await page.unroute("**/api/v1/jobs");
        await page.unroute("**/api/v1/results");
        await invoke(page, "runtime/refreshAll");
        assertWrites(library, kind === "job" ? 1 : 0, kind === "result" ? 1 : 0);
      } finally { write.release(); read.release(); }
    });
  });
}
