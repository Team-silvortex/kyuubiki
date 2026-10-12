import assert from "node:assert/strict";
import { test } from "node:test";
import { runtime, usingWorkbench, openWorkbench, invoke, holdRequest, waitForGuiTransition,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";
import { installProjectionSolver } from "./workbench-ui-pwdt-projection-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const ID = "selected-cancellation";
const cancel = (page) => page.locator('[data-workbench-data-action="cancel-job"]');
const message = (page) => page.evaluate(() => window.__kyuubikiPwdt.state().message);
function seed() {
  const job = { job_id: ID, status: "solving", progress: 0.5, project_id: "qualification-project",
    message: "backend A", has_result: false };
  runtime.state.adminJobs.push(job);
  runtime.state.adminResults.push({ job_id: ID, result: { metric: 42 } });
  runtime.state.jobRecordMutations.length = 0;
  runtime.state.resultRecordMutations.length = 0;
  return job;
}
async function openData(page, id = ID) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "data", systemDataTab: "jobs" }));
  await page.locator('[data-workbench-data-page="browse"]').click();
  await page.locator(`[data-workbench-data-record-kind="job"][data-workbench-data-record-id="${id}"]`).click();
  await page.locator('[data-workbench-data-page="edit"]').click();
}

test("Workbench selected cancellation does not treat a solving receipt as cancellation", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const job = seed();
    let writes = 0;
    await page.route(`**/api/v1/jobs/${ID}/cancel`, (route) => {
      writes += 1;
      return route.fulfill({ json: { job } });
    });
    await openWorkbench(page);
    await openData(page);
    await page.waitForLoadState("networkidle");
    const reads = runtime.state.historyFetches;
    await cancel(page).click();
    await waitForGuiTransition(page);
    await page.waitForLoadState("networkidle");
    assert.equal(writes, 1);
    assert.match(await message(page), /JOB_CANCELLATION_NOT_CONFIRMED/u);
    assert.equal(runtime.state.historyFetches, reads, "an unconfirmed cancellation cannot start read recovery");
  });
});

test("Workbench selected cancellation cannot send an old record to backend B", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const job = seed();
    const writes = [];
    await page.route("**/cancel-authority/api/**", (route) => {
      if (route.request().method() !== "GET") writes.push(route.request().url());
      return route.fulfill({ json: { job: { ...job, status: "cancelled" } } });
    });
    await openWorkbench(page);
    await openData(page);
    await page.evaluate((base) => {
      const url = new URL(location.href);
      url.searchParams.set("kyuubikiApiBaseUrl", `${base}/cancel-authority`);
      history.replaceState(null, "", url);
    }, runtime.backendUrl);
    await cancel(page).evaluate((element) => { element.disabled = false; element.click(); });
    await waitForGuiTransition(page);
    await page.waitForLoadState("networkidle");
    assert.deepEqual(writes, [], "retained history cannot authorize cancellation on another backend");
    await openData(page);
    assert.equal(await cancel(page).isDisabled(), true);
  });
});

for (const failure of ["null", "wrong-id", "completed", "invalid-progress", "contradictory-status", "unavailable"]) {
  test(`Workbench selected cancellation rejects ${failure} with no recovery read or write replay`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const job = seed();
      let writes = 0;
      const receipt = failure === "null" ? null : { job: { ...job, status: "cancelled",
        ...(failure === "wrong-id" ? { job_id: "other" } : {}),
        ...(failure === "completed" ? { status: "completed" } : {}),
        ...(failure === "invalid-progress" ? { progress: -1 } : {}),
        ...(failure === "contradictory-status" ? { status_detail: { lifecycle: "active", active: true, terminal: false } } : {}),
      } };
      await page.route(`**/api/v1/jobs/${ID}/cancel`, (route) => {
        writes += 1;
        return route.fulfill({ status: failure === "unavailable" ? 503 : 200,
          json: failure === "unavailable" ? { error: "cancellation unavailable" } : receipt });
      });
      await openWorkbench(page);
      await openData(page);
      await page.waitForLoadState("networkidle");
      const reads = runtime.state.historyFetches;
      await cancel(page).click();
      await waitForGuiTransition(page);
      await page.waitForLoadState("networkidle");
      assert.equal(writes, 1);
      assert.equal(runtime.state.historyFetches, reads);
      assert.match(await message(page), failure === "unavailable" ? /cancellation unavailable/u
        : failure === "null" || failure === "wrong-id" ? /different job/u
          : failure === "completed" ? /JOB_CANCELLATION_NOT_CONFIRMED/u : /JOB_CANCELLATION_INVALID/u);
      assert.equal(await cancel(page).isDisabled(), false, "failure releases the write reservation for explicit retry");
      assert.equal(library.writes.length + runtime.state.projectMutations.length + runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
    });
  });
}

for (const failure of ["network", "malformed"]) {
  test(`Workbench confirmed selected cancellation survives ${failure} read recovery and never repeats on refresh`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      const job = seed();
      let writes = 0, unavailable = true;
      await page.route(`**/api/v1/jobs/${ID}/cancel`, (route) => {
        writes += 1; job.status = "cancelled";
        return route.fulfill({ json: { job } });
      });
      await openWorkbench(page);
      await openData(page);
      const field = page.locator('[data-workbench-data-field="job-message"]');
      await field.fill("new working draft");
      await page.route("**/api/v1/jobs", (route) => unavailable
        ? route.fulfill({ status: failure === "network" ? 503 : 200,
          json: failure === "network" ? { error: "post-cancel history unavailable" } : { jobs: null } })
        : route.fallback());
      await cancel(page).click();
      await waitForGuiTransition(page);
      assert.equal(await message(page), "Job cancelled.");
      assert.equal(await field.inputValue(), "new working draft");
      assert.equal(await cancel(page).isDisabled(), false, "an unread catalog is retained rather than fabricated as terminal");
      await page.locator('[data-workbench-alert-id="runtime-recovery-job_history"]').first().waitFor({ state: "visible" });
      unavailable = false;
      await invoke(page, "runtime/refreshAll");
      await page.waitForFunction(() => !document.querySelector('[data-workbench-alert-id="runtime-recovery-job_history"]'));
      assert.equal(await cancel(page).isDisabled(), true);
      assert.equal(await field.inputValue(), "new working draft");
      assert.equal(writes, 1);
    });
  });
}

for (const sameJob of [false, true]) {
  test(`Workbench selected cancellation preserves live ownership and is observed by polling; sameJob=${sameJob}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const historical = seed(), solver = await installProjectionSolver(page);
      let active, writes = 0;
      solver.beforeJobResponse = async (job) => {
        active = job;
        runtime.state.adminJobs.push(job);
        await page.route(`**/api/v1/jobs/${job.job_id}`, (route) => route.fulfill({ json: {
          job, ...(job.status === "solving" ? { result: solver.results.at(-1) } : {}),
        } }));
      };
      await openWorkbench(page);
      await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
      const running = invoke(page, "job/run").catch((error) => ({ error: error.message }));
      try {
        await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "solving", hasResult: true }));
        await invoke(page, "runtime/refreshAll");
        const selected = sameJob ? active : historical;
        await page.route(`**/api/v1/jobs/${selected.job_id}/cancel`, (route) => {
          writes += 1; selected.status = "cancelled";
          return route.fulfill({ json: { job: selected } });
        });
        await openData(page, selected.job_id);
        await cancel(page).click();
        await waitForGuiTransition(page);
        if (sameJob) {
          await running;
          await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "cancelled", hasResult: false }));
        } else {
          const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
          assert.equal(state.jobStatus, "solving");
          assert.equal(state.hasResult, true, "cancelling a different historical record cannot clear the active preview");
        }
        assert.equal(writes, 1);
        assert.equal(solver.submissions.length, 1);
        assert.equal(library.writes.length, 0);
      } finally { active.status = "failed"; await running; }
    });
  });
}

for (const blockedAction of ["cancel-job", "save-job", "delete-job", "delete-result"]) {
  test(`Workbench selected cancellation owns the record through refresh and blocks ${blockedAction}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const job = seed();
      let writes = 0;
      await openWorkbench(page);
      await openData(page);
      const write = await holdRequest(page, `/api/v1/jobs/${ID}/cancel`, "POST", (route) => {
        writes += 1; job.status = "cancelled";
        return route.fulfill({ json: { job } });
      });
      let read;
      try {
        await cancel(page).evaluate((element) => { element.click(); element.click(); });
        await write.received;
        await waitForGuiTransition(page, true);
        assert.equal(await cancel(page).isDisabled(), true);
        read = await holdRequest(page, "/api/v1/jobs", "GET", (route) => route.fulfill({ status: 503, json: { error: "read held after cancellation" } }));
        write.release();
        await read.received;
        if (blockedAction === "delete-result") {
          await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
          await openData(page);
          await page.locator('[data-workbench-data-tab="results"]').click();
          await page.locator('[data-workbench-data-page="browse"]').click();
          await page.locator(`[data-workbench-data-record-kind="result"][data-workbench-data-record-id="${ID}"]`).click();
          await page.locator('[data-workbench-data-page="edit"]').click();
        }
        const blocked = page.locator(`[data-workbench-data-action="${blockedAction}"]`);
        assert.equal(await blocked.isDisabled(), true);
        await blocked.evaluate((element) => element.click());
        read.release();
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        assert.equal(writes, 1);
        assert.equal(runtime.state.jobRecordMutations.length + runtime.state.resultRecordMutations.length, 0);
        assert.equal(library.writes.length + runtime.state.projectMutations.length, 0);
        if (blockedAction !== "delete-result") assert.equal(await message(page), "Job cancelled.");
      } finally { write.release(); read?.release(); }
    });
  });
}

for (const failed of [false, true]) for (const change of ["feedback", "backend"]) {
  test(`Workbench late selected cancellation failed=${failed} respects newer ${change}`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      const job = seed();
      let writes = 0;
      await openWorkbench(page);
      await openData(page);
      const held = await holdRequest(page, `/api/v1/jobs/${ID}/cancel`, "POST", (route) => {
        writes += 1;
        if (failed) return route.fulfill({ status: 503, json: { error: "old cancellation failed" } });
        job.status = "cancelled";
        return route.fulfill({ json: { job } });
      });
      const reads = [];
      try {
        await cancel(page).click();
        await held.received;
        if (change === "feedback") {
          await page.locator('[data-workbench-data-tab="results"]').click();
          await page.locator('[data-workbench-data-page="browse"]').click();
          await page.locator(`[data-workbench-data-record-kind="result"][data-workbench-data-record-id="${ID}"]`).click();
          await page.locator('[data-workbench-data-page="edit"]').click();
          await Promise.all([page.waitForEvent("download"), page.locator('[data-workbench-data-action="export-result"]').click()]);
        } else {
          await page.route("**/cancel-authority/api/**", (route) => {
            assert.equal(route.request().method(), "GET");
            const path = new URL(route.request().url()).pathname;
            reads.push(path);
            if (path.endsWith("/jobs")) return route.fulfill({ json: { jobs: [{ ...job, message: "backend B", status: "solving" }] } });
            if (path.endsWith("/results")) return route.fulfill({ json: { results: runtime.state.adminResults } });
            if (path.endsWith("/projects")) return route.fulfill({ json: { projects: runtime.state.projects } });
            return route.fulfill({ status: 503, json: { error: "unrelated B read unavailable" } });
          });
          await page.evaluate((base) => {
            const url = new URL(location.href); url.searchParams.set("kyuubikiApiBaseUrl", `${base}/cancel-authority`);
            history.replaceState(null, "", url);
          }, runtime.backendUrl);
          await invoke(page, "runtime/refreshAll");
        }
        const newerMessage = await message(page), bReads = reads.length;
        held.release();
        await waitForGuiTransition(page);
        await page.waitForLoadState("networkidle");
        assert.equal(writes, 1);
        assert.equal(await message(page), newerMessage);
        if (change === "backend") {
          assert.equal(reads.length, bReads, "old confirmation cannot start a read on backend B");
          assert.equal(await page.locator('[data-workbench-data-field="job-message"]').inputValue(), "backend B");
          assert.equal(await cancel(page).isDisabled(), false);
        }
      } finally { held.release(); }
    });
  });
}
