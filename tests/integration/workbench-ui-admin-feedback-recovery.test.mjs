import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, usingWorkbench, openWorkbench, invoke, holdRequest, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "feedback-job";
const OTHER = "other-feedback-job";
const endpoint = (kind) => `/api/v1/${kind}s`;
const button = (page, kind, action) => page.locator(`[data-workbench-data-action="${action}-${kind}"]`);
const row = (page, kind, id) => page.locator(`[data-workbench-data-record-kind="${kind}"][data-workbench-data-record-id="${id}"]`);

function seed() {
  for (const job_id of [ID, OTHER]) {
    runtime.state.adminJobs.push({ job_id, status: "completed", progress: 1, message: "initial message",
      project_id: "qualification-project", model_version_id: "initial-version", simulation_case_id: "initial-case", has_result: true });
    runtime.state.adminResults.push({ job_id, result: { metric: 1 } });
  }
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
}

async function exportResult(page) {
  await Promise.all([page.waitForEvent("download"), button(page, "result", "export").click()]);
  return page.evaluate(() => window.__kyuubikiPwdt.state().message);
}

async function changeAuthority(page) {
  await page.evaluate((base) => {
    const url = new URL(location.href);
    url.searchParams.set("kyuubikiApiBaseUrl", `${base}/alternate-authority`);
    history.replaceState(null, "", url);
  }, runtime.backendUrl);
}

async function openData(page, kind, id = ID) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate((kind) => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: `${kind}s` }), kind);
  await page.locator('[data-workbench-data-page="browse"]').click();
  await row(page, kind, id).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}

for (const kind of ["job", "result"]) for (const action of ["save", "delete"]) for (const rejected of [false, true]) {
  test(`Workbench late ${kind} ${action} ${rejected ? "error" : "success"} cannot replace newer export feedback`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      const write = await holdRequest(page, `${endpoint(kind)}/${ID}`, action === "save" ? "PATCH" : "DELETE", (route) => rejected
        ? route.fulfill({ status: 503, json: { error: "old administrative write failed" } }) : route.fallback());
      try {
        await button(page, kind, action).click();
        await write.received;
        if (kind === "job") await openData(page, "result");
        const message = await exportResult(page);
        assert.match(message, /download/i);
        write.release();
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), message);
        assert.equal(runtime.state.jobRecordMutations.length, kind === "job" && !rejected ? 1 : 0);
        assert.equal(runtime.state.resultRecordMutations.length, kind === "result" && !rejected ? 1 : 0);
        assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
        assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
      } finally { write.release(); }
    });
  });
}

for (const kind of ["job", "result"]) for (const rejected of [false, true]) {
  test(`Workbench manual ${kind} selection ABA invalidates late ${rejected ? "failure" : "success"} feedback`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      const write = await holdRequest(page, `${endpoint(kind)}/${ID}`, "PATCH", (route) => rejected
        ? route.fulfill({ status: 503, json: { error: "old administrative write failed" } }) : route.fallback());
      try {
        await button(page, kind, "save").click();
        await write.received;
        await page.locator('[data-workbench-data-page="browse"]').click();
        await row(page, kind, OTHER).click();
        await row(page, kind, ID).click();
        await page.locator('[data-workbench-data-page="edit"]').click();
        const message = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
        write.release();
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.message, message);
        assert.equal(kind === "job" ? state.selectedAdminJobId : state.selectedAdminResultJobId, ID);
      } finally { write.release(); }
    });
  });
}

for (const kind of ["job", "result"]) for (const action of ["save", "delete"]) {
  test(`Workbench ${kind} ${action} follow-up read cannot overwrite newer feedback`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      const read = await holdRequest(page, endpoint(kind), "GET", (route) =>
        route.fulfill({ status: 503, json: { error: "post-write read unavailable" } }));
      try {
        await button(page, kind, action).click();
        await read.received;
        await openData(page, "result", OTHER);
        const message = await exportResult(page);
        read.release();
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), message);
        assert.equal(runtime.state.jobRecordMutations.length, kind === "job" ? 1 : 0);
        assert.equal(runtime.state.resultRecordMutations.length, kind === "result" ? 1 : 0);
      } finally { read.release(); }
    });
  });
}

for (const kind of ["job", "result"]) for (const action of ["save", "delete"]) {
  test(`Workbench confirmed ${kind} ${action} from the old backend cannot modify the new backend view`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      const jobs = structuredClone(runtime.state.adminJobs).map((job) => ({ ...job, message: "new authority message" }));
      const results = structuredClone(runtime.state.adminResults).map((result) => ({ ...result, result: { metric: 84 } }));
      const reads = [];
      await page.route("**/alternate-authority/api/**", (route) => {
        const pathname = new URL(route.request().url()).pathname;
        assert.equal(route.request().method(), "GET", "no administrative write may transfer to the new authority");
        reads.push(pathname);
        if (pathname.endsWith("/jobs")) return route.fulfill({ json: { jobs } });
        if (pathname.endsWith("/results")) return route.fulfill({ json: { results } });
        if (pathname.endsWith("/projects")) return route.fulfill({ json: { projects: runtime.state.projects } });
        return route.fulfill({ status: 503, json: { error: "unrelated new authority read unavailable" } });
      });
      const write = await holdRequest(page, `${endpoint(kind)}/${ID}`, action === "save" ? "PATCH" : "DELETE");
      try {
        await button(page, kind, action).click();
        await write.received;
        await changeAuthority(page);
        await invoke(page, "runtime/refreshAll");
        const before = reads.length;
        const message = await page.evaluate(() => window.__kyuubikiPwdt.state().message);
        write.release();
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        assert.equal(reads.length, before, "old write confirmation must not start follow-up reads on another authority");
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().message), message);
        await page.locator('[data-workbench-data-page="browse"]').click();
        assert.equal(await row(page, kind, ID).count(), 1, "the same record ID on the new authority must remain visible");
        await row(page, kind, ID).click();
        await page.locator('[data-workbench-data-page="edit"]').click();
        assert.equal(await page.locator(`[data-workbench-data-field="${kind === "job" ? "job-message" : "result-payload"}"]`).inputValue(),
          kind === "job" ? "new authority message" : JSON.stringify({ metric: 84 }, null, 2));
        assert.equal(runtime.state.jobRecordMutations.length, kind === "job" ? 1 : 0);
        assert.equal(runtime.state.resultRecordMutations.length, kind === "result" ? 1 : 0);
        assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
      } finally { write.release(); }
    });
  });
}

for (const kind of ["job", "result"]) for (const rejected of [false, true]) {
  test(`Workbench old ${kind} read ${rejected ? "failure" : "success"} is discarded after backend change without a newer read`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      const read = await holdRequest(page, endpoint(kind), "GET", (route) => route.fulfill(rejected
        ? { status: 503, json: { error: "stale authority read failed" } }
        : { json: { [kind === "job" ? "jobs" : "results"]: [] } }));
      let refresh;
      try {
        refresh = invoke(page, "runtime/refreshAll");
        await read.received;
        await changeAuthority(page);
        read.release();
        await refresh;
        await waitForGuiTransition(page);
        await page.locator('[data-workbench-data-page="browse"]').click();
        assert.equal(await row(page, kind, ID).count(), 1);
        assert.equal(await row(page, kind, OTHER).count(), 1);
        assert.equal(await page.getByText("stale authority read failed", { exact: false }).count(), 0);
        assert.equal(await page.locator('[data-workbench-results="read-pending"]').count(), 0);
        if (kind === "result") assert.equal(await page.locator('[data-workbench-results="read-error"]').count(), 0);
        await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
        await page.locator('[data-workbench-runtime-tab="watchdog"]').click();
        const recovery = page.locator('.runtime-overview-card').filter({ has: page.locator('[data-workbench-recovery-action="retry-all"]') });
        await recovery.waitFor({ state: "visible" });
        assert.doesNotMatch(await recovery.innerText(), /stale authority read failed/u);
        assert.equal(runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
      } finally { read.release(); await refresh; }
    });
  });
}
