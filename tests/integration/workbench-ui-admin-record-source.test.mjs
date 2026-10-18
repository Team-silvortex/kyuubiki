import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import { runtime, usingWorkbench, openWorkbench, invoke, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "source-record";
const row = (page, kind) => page.locator(`[data-workbench-data-record-kind="${kind}"][data-workbench-data-record-id="${ID}"]`);
const field = (page, kind) => page.locator(`[data-workbench-data-field="${kind === "job" ? "job-message" : "result-payload"}"]`);
const button = (page, kind, action) => page.locator(`[data-workbench-data-action="${action}-${kind}"]`);
const working = (kind) => kind === "job" ? "unsaved from backend A" : '{"metric":42,"source":"A"}';

function seed() {
  runtime.state.adminJobs.push({ job_id: ID, status: "completed", progress: 1, message: "backend A",
    project_id: "qualification-project", model_version_id: "initial-version", simulation_case_id: "initial-case", has_result: true });
  runtime.state.adminResults.push({ job_id: ID, result: { metric: 1 } });
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
}

async function openData(page, kind) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate((kind) => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: `${kind}s` }), kind);
  await page.locator('[data-workbench-data-page="browse"]').click();
  await row(page, kind).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}

async function backendB(page) {
  const writes = [];
  const job = { ...runtime.state.adminJobs[0], message: "backend B", project_id: "backend-b-project",
    model_version_id: "backend-b-version", simulation_case_id: "backend-b-case" };
  const result = { job_id: ID, result: { metric: 84, source: "B" } };
  let unavailable = false;
  await page.route("**/record-authority/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    if (method !== "GET") {
      writes.push({ path, method, body: route.request().postDataJSON() });
      if (path.includes("/results/")) return route.fulfill({ json: { ...result, deleted: true } });
      return route.fulfill({ json: { job, deleted: true } });
    }
    if (unavailable) return route.fulfill({ status: 503, json: { error: "backend B read unavailable" } });
    if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [job] } });
    if (path.endsWith("/results")) return route.fulfill({ json: { results: [result] } });
    if (path.endsWith("/projects")) return route.fulfill({ json: { projects: runtime.state.projects } });
    return route.fulfill({ status: 503, json: { error: "unrelated backend B read unavailable" } });
  });
  await page.evaluate((base) => {
    const url = new URL(location.href);
    url.searchParams.set("kyuubikiApiBaseUrl", `${base}/record-authority`);
    history.replaceState(null, "", url);
  }, runtime.backendUrl);
  return { writes, job, result, unavailable: (value) => { unavailable = value; } };
}

for (const kind of ["job", "result"]) for (const action of ["save", "delete"]) for (const failedRead of [false, true]) {
  test(`Workbench stale ${kind} ${action} cannot transfer to backend B; failedRead=${failedRead}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      await field(page, kind).fill(working(kind));
      if (kind === "job") for (const name of ["project", "version", "case"]) {
        await page.locator(`[data-workbench-data-field="job-${name}"]`).fill(`unsaved-${name}-A`);
      }
      const target = await backendB(page);
      if (failedRead) {
        target.unavailable(true);
        await invoke(page, "runtime/refreshAll");
      }
      // Exercise the imperative guard even if the retained GUI button has not rerendered yet.
      await button(page, kind, action).evaluate((element) => { element.disabled = false; element.click(); });
      await waitForGuiTransition(page);
      await page.waitForLoadState("networkidle");
      assert.deepEqual(target.writes, [], "an old record or draft must not authorize a write on the new backend");
      assert.equal(await field(page, kind).inputValue(), working(kind));
      await openData(page, kind);
      assert.equal(await button(page, kind, "save").isDisabled(), true);
      assert.equal(await button(page, kind, "delete").isDisabled(), true);
      await page.locator('[data-workbench-admin-source="stale"]').waitFor({ state: "visible" });
      target.unavailable(false);
      if (failedRead) await invoke(page, "runtime/refreshAll");
      else await page.locator('[data-workbench-data-action="refresh-record-source"]').click();
      await page.waitForFunction(({ kind }) => {
        const value = document.querySelector(`[data-workbench-data-field="${kind === "job" ? "job-message" : "result-payload"}"]`)?.value;
        return kind === "job" ? value === "backend B" : value?.includes('"metric": 84');
      }, { kind });
      assert.equal(await button(page, kind, "save").isDisabled(), false);
      assert.equal(await button(page, kind, "delete").isDisabled(), false);
      assert.equal(await page.locator('[data-workbench-admin-source="stale"]').count(), 0);
      if (kind === "job") for (const [name, value] of [["project", target.job.project_id], ["version", target.job.model_version_id], ["case", target.job.simulation_case_id]]) {
        assert.equal(await page.locator(`[data-workbench-data-field="job-${name}"]`).inputValue(), value);
      }
      await button(page, kind, action).click();
      await waitForGuiTransition(page);
      await page.waitForLoadState("networkidle");
      assert.equal(target.writes.length, 1, "a new explicit action uses the newly verified record");
      if (action === "save") assert.equal(kind === "job" ? target.writes[0].body.message : target.writes[0].body.result.source,
        kind === "job" ? "backend B" : "B");
      assert.equal(runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
      assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
      assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
    });
  });
}

for (const kind of ["job", "result"]) for (const failedRead of [false, true]) {
  test(`Workbench same-backend ${kind} refresh retains dirty editing and permits an explicit save; failedRead=${failedRead}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      await field(page, kind).fill(working(kind));
      if (failedRead) await page.route(`**/api/v1/${kind}s`, (route) => route.fulfill({ status: 503, json: { error: "same-backend read unavailable" } }));
      await invoke(page, "runtime/refreshAll");
      assert.equal(await field(page, kind).inputValue(), working(kind));
      assert.equal(await button(page, kind, "save").isDisabled(), false);
      assert.equal(await page.locator('[data-workbench-admin-source="stale"]').count(), 0);
      await button(page, kind, "save").click();
      await waitForGuiTransition(page);
      await page.waitForLoadState("networkidle");
      assert.equal(runtime.state.jobRecordMutations.length, kind === "job" ? 1 : 0);
      assert.equal(runtime.state.resultRecordMutations.length, kind === "result" ? 1 : 0);
    });
  });
}

test("Workbench stale result draft remains locally exportable without writing either backend", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    seed();
    await openWorkbench(page);
    await openData(page, "result");
    await field(page, "result").fill(working("result"));
    const target = await backendB(page);
    await openData(page, "result");
    assert.equal(await button(page, "result", "save").isDisabled(), true);
    const [download] = await Promise.all([page.waitForEvent("download"), button(page, "result", "export").click()]);
    assert.deepEqual(JSON.parse(await readFile(await download.path(), "utf8")), JSON.parse(working("result")));
    assert.deepEqual(target.writes, []);
    assert.equal(runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
    assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
  });
});

for (const kind of ["job", "result"]) {
  test(`Workbench unreadable ${kind} source blocks writing without a GUI exception and recovers after explicit refresh`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      seed();
      await openWorkbench(page);
      await openData(page, kind);
      await field(page, kind).fill(working(kind));
      await page.evaluate(() => {
        window.__sourceTestUrl = location.href;
        window.__sourceTestGetItem = Storage.prototype.getItem;
        const url = new URL(location.href);
        url.searchParams.delete("kyuubikiApiBaseUrl");
        url.searchParams.delete("apiBaseUrl");
        history.replaceState(null, "", url);
        Storage.prototype.getItem = function (key) {
          if (key === "kyuubiki-workbench-api-base-url") throw new Error("source settings unavailable");
          return window.__sourceTestGetItem.call(this, key);
        };
      });
      try {
        await button(page, kind, "save").evaluate((element) => element.click());
        await waitForGuiTransition(page);
        assert.equal(runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
        await page.locator('[data-workbench-admin-source="stale"]').waitFor({ state: "visible" });
        assert.equal(await field(page, kind).inputValue(), working(kind));
      } finally {
        await page.evaluate(() => {
          Storage.prototype.getItem = window.__sourceTestGetItem;
          history.replaceState(null, "", window.__sourceTestUrl);
        });
      }
      await page.locator('[data-workbench-data-action="refresh-record-source"]').click();
      await page.waitForFunction((kind) => document.querySelector(`[data-workbench-data-action="save-${kind}"]`)?.disabled === false, kind);
      // An observed authority failure retires the old draft, even after settings are restored.
      assert.equal(await field(page, kind).inputValue(), kind === "job" ? "backend A" : JSON.stringify({ metric: 1 }, null, 2));
      await field(page, kind).fill(working(kind));
      await button(page, kind, "save").click();
      await waitForGuiTransition(page);
      await page.waitForLoadState("networkidle");
      assert.equal(runtime.state.jobRecordMutations.length, kind === "job" ? 1 : 0);
      assert.equal(runtime.state.resultRecordMutations.length, kind === "result" ? 1 : 0);
    });
  });
}
