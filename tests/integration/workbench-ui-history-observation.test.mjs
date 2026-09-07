import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, holdRequest, exportProject, runtime,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";
import { installProjectionSolver } from "./workbench-ui-pwdt-projection-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

async function openHistory(page, jobId) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "jobs" }));
  // The real Library button is the action entry, not a test-only controller call.
  await page.locator(`[data-workbench-history-job-id="${jobId}"]`).click();
}

for (const kind of ["heat_bar_1d", "heat_plane_triangle_2d", "heat_plane_quad_2d",
  "electrostatic_plane_triangle_2d", "electrostatic_plane_quad_2d", "thermal_plane_triangle_2d", "thermal_plane_quad_2d"]) {
  test(`Workbench history restores ${kind} as a detached working copy and can solve it again`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const solver = await installProjectionSolver(page);
      await openWorkbench(page);
      await invoke(page, "nav/setStudyKind", { studyKind: kind });
      await invoke(page, "model/saveAs");
      await invoke(page, "job/run");
      const archived = structuredClone(solver.results[0]);
      await invoke(page, "nav/setStudyKind", { studyKind: "truss_2d" });
      await invoke(page, "model/saveAs");
      const kept = structuredClone(library.models);
      await openHistory(page, "projection-job-1");
      await page.evaluate((studyKind) => window.__kyuubikiPwdt.waitForState({ studyKind, hasResult: true, selectedModelId: null }), kind);
      const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
      assert.equal(state.studyKind, kind);
      assert.equal(state.hasResult, true);
      assert.equal(state.selectedModelId, null);
      assert.equal(state.selectedVersionId, null);
      await invoke(page, "model/save");
      await invoke(page, "job/run");
      assert.equal(library.models.length, 3);
      assert.deepEqual(library.models.slice(0, 2), kept);
      assert.equal(solver.submissions[1].kind, kind);
      assert.deepEqual(solver.submissions[1].input.nodes, archived.input.nodes);
      assert.deepEqual(solver.submissions[1].input.elements, archived.input.elements);
      assert.deepEqual(solver.results[0], archived);
    });
  });
}

for (const failed of [false, true]) {
  test(`Workbench late history ${failed ? "failure" : "success"} cannot overwrite an undone workspace`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page) => {
      await installProjectionSolver(page);
      await openWorkbench(page);
      await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
      await invoke(page, "job/run");
      await invoke(page, "nav/setStudyKind", { studyKind: "spring_2d" });
      const path = "/api/v1/jobs/projection-job-1";
      const pending = await holdRequest(page, path, "GET", (route) => failed
        ? route.fulfill({ status: 503, json: { error: "old history unavailable" } }) : route.fallback());
      try {
        await openHistory(page, "projection-job-1");
        await pending.received;
        await invoke(page, "history/undo");
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        const response = page.waitForResponse((entry) => new URL(entry.url()).pathname === path);
        pending.release();
        await response;
        await page.waitForLoadState("networkidle");
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        for (const key of ["studyKind", "hasResult", "jobStatus", "selectedModelId", "message"]) assert.equal(after[key], before[key]);
      } finally { pending.release(); }
    });
  });
}

test("Workbench history with no result clears the old view but retains working geometry and save binding", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await installProjectionSolver(page);
    runtime.state.adminJobs.push({ job_id: "empty-history", status: "failed", progress: 0, worker_id: "test", has_result: false });
    await page.route("**/api/v1/jobs/empty-history", (route) => route.fulfill({ json: { job: runtime.state.adminJobs[0] } }));
    await openWorkbench(page);
    await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
    await invoke(page, "model/saveAs");
    await invoke(page, "job/run");
    const original = (await exportProject(page)).workspace_snapshot;
    await openHistory(page, "empty-history");
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ hasResult: false, jobStatus: "failed" }));
    const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
    assert.equal(state.hasResult, false);
    assert.equal(state.jobStatus, "failed");
    assert.equal(state.selectedModelId, library.models[0].model_id);
    assert.deepEqual((await exportProject(page)).workspace_snapshot, original);
  });
});

for (const replacement of ["job", "undo"]) {
  for (const failed of [false, true]) {
    test(`Workbench late cancellation ${failed ? "failure" : "success"} preserves a newer ${replacement} state`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page) => {
        const oldJob = { job_id: "cancel-old", status: "solving", progress: 0.2, worker_id: "test", has_result: false };
        const nextJob = { ...oldJob, job_id: "cancel-next", status: "queued" };
        runtime.state.adminJobs.push(oldJob, nextJob);
        for (const job of [oldJob, nextJob]) await page.route(`**/api/v1/jobs/${job.job_id}`, (route) => route.fulfill({ json: { job } }));
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
        await openHistory(page, oldJob.job_id);
        await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "solving" }));
        let cancellations = 0;
        const pending = await holdRequest(page, `/api/v1/jobs/${oldJob.job_id}/cancel`, "POST", async (route) => {
          cancellations += 1;
          if (failed) await route.fulfill({ status: 503, json: { error: "old cancellation unavailable" } });
          else { oldJob.status = "cancelled"; await route.fulfill({ json: { job: oldJob } }); }
        });
        page.once("dialog", (dialog) => dialog.accept());
        const cancelling = invoke(page, "job/cancel").catch((error) => ({ error: error.message }));
        try {
          await pending.received;
          if (replacement === "job") {
            await openHistory(page, nextJob.job_id);
            await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "queued" }));
          } else await invoke(page, "history/undo");
          const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
          pending.release();
          const outcome = await cancelling;
          if (failed) assert.match(outcome.error, /unavailable|503/u);
          else { assert.equal(outcome.ok, true); assert.equal(outcome.contextChanged, true); }
          const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
          for (const key of ["studyKind", "jobStatus", "hasResult", "selectedModelId", "message"]) assert.equal(after[key], before[key]);
          assert.equal(nextJob.status, "queued");
          assert.equal(oldJob.status, failed ? "solving" : "cancelled");
          assert.equal(cancellations, 1);
        } finally { pending.release(); await cancelling; }
      });
    });
  }
}

for (const phase of ["submission", "polling"]) {
  for (const failure of ["network", "identity", "result"]) {
    test(`Workbench failed history ${failure} preserves live ${phase} and retries without resubmitting`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        const solver = await installProjectionSolver(page);
        const archive = { job_id: "retry-archive", status: "completed", progress: 1, worker_id: "test", has_result: true };
        runtime.state.adminJobs.push(archive);
        let recover = false;
        await page.route(`**/api/v1/jobs/${archive.job_id}`, (route) => {
          if (!recover && failure === "network") return route.fulfill({ status: 503, json: { error: "archive unavailable" } });
          return route.fulfill({ json: {
            job: !recover && failure === "identity" ? { ...archive, job_id: "wrong-archive" } : archive,
            result: !recover && failure === "result" ? { input: null } : solver.results[0],
          } });
        });
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
        await invoke(page, "model/saveAs");
        const pendingGate = phase === "submission"
          ? Promise.resolve(await holdRequest(page, "/api/v1/fem/heat-bar-1d/jobs", "POST"))
          : new Promise((resolve) => {
            solver.beforeJobResponse = async (job) => resolve(await holdRequest(page, `/api/v1/jobs/${job.job_id}`));
          });
        const running = invoke(page, "job/run").catch((error) => ({ error: error.message }));
        const pending = await pendingGate;
        try {
          await pending.received;
          await openHistory(page, archive.job_id);
          await page.waitForFunction(() => /archive unavailable|HISTORY_RESULT_INVALID/u.test(window.__kyuubikiPwdt.state().message));
          const preserved = await page.evaluate(() => window.__kyuubikiPwdt.state());
          assert.equal(preserved.studyKind, "heat_bar_1d");
          assert.equal(preserved.selectedModelId, library.models[0].model_id);
          pending.release();
          const completed = await running;
          assert.equal(completed.ok, true, JSON.stringify(completed));
          await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "completed", hasResult: true }));
          recover = true;
          await openHistory(page, archive.job_id);
          await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedModelId: null, hasResult: true }));
          assert.equal(solver.submissions.length, 1, "retrying a history read must not submit a second computation");
        } finally { pending.release(); await running; }
      });
    });
  }
}

for (const mode of ["steps", "recipe"]) {
  for (const failure of ["null-result", "identity"]) {
    test(`Workbench PWDT ${mode} stops on ${failure} without submitting dependent work and allows explicit recovery`, { timeout: 75_000 }, async () => {
      await usingWorkbench(async (page, library) => {
        const solver = await installProjectionSolver(page);
        let inject = true;
        let polls = 0;
        solver.beforeJobResponse = async (job) => {
          await page.route(`**/api/v1/jobs/${job.job_id}`, async (route) => {
            if (!inject) return route.fallback();
            polls += 1;
            const result = solver.results.at(-1);
            return route.fulfill({ json: failure === "identity"
              ? { job: { ...job, job_id: "foreign-job", status: "completed", progress: 1 }, result }
              : polls === 1 ? { job, result } : { job: { ...job, status: "completed", progress: 1 }, result: null },
            });
          });
        };
        await openWorkbench(page);
        await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
        await assert.rejects(page.evaluate((sequence) => sequence === "steps"
          ? window.__kyuubikiPwdt.runSteps([{ action: "job/run" }, { action: "model/saveAs" }])
          : window.__kyuubikiPwdt.runRecipe("recipe/heat-thermo/quad-closed-loop"), mode),
        failure === "identity" ? /different job/u : /did not include a result/u);
        const state = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(state.hasResult, false, "a progress preview must not survive a result-free terminal response");
        assert.equal(solver.submissions.length, 1);
        assert.equal(polls, failure === "identity" ? 3 : 2);
        assert.equal(library.models.length, mode === "steps" ? 0 : 1, "dependent saves must not execute after an invalid completion");
        inject = false;
        await invoke(page, "job/run");
        await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "completed", hasResult: true }));
        assert.equal(solver.submissions.length, 2, "a new computation requires an explicit invocation");
      });
    });
  }
}

async function startCancellationPreview(page) {
  const solver = await installProjectionSolver(page);
  const control = { status: "cancelled", unavailable: false, cancellations: 0, job: null };
  solver.beforeJobResponse = async (job) => {
    control.job = job;
    runtime.state.adminJobs.push(job);
    await page.route(`**/api/v1/jobs/${job.job_id}`, (route) => route.fulfill({ json: {
      job, ...(["solving", "completed"].includes(job.status) ? { result: solver.results.at(-1) } : {}),
    } }));
    await page.route(`**/api/v1/jobs/${job.job_id}/cancel`, (route) => {
      control.cancellations += 1;
      if (control.unavailable) return route.fulfill({ status: 503, json: { error: "qualification cancellation unavailable" } });
      job.status = control.status;
      return route.fulfill({ json: { job } });
    });
  };
  await openWorkbench(page);
  await invoke(page, "nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
  const running = invoke(page, "job/run").catch((error) => ({ error: error.message }));
  await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "solving", hasResult: true }));
  return { control, solver, running };
}

for (const status of ["solving", "completed", "failed"]) {
  test(`Workbench cancellation returning ${status} stops dependent steps but preserves truthful job observation`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      const { control, solver, running } = await startCancellationPreview(page);
      try {
        control.status = status;
        page.once("dialog", (dialog) => dialog.accept());
        await assert.rejects(page.evaluate(() => window.__kyuubikiPwdt.runSteps([
          { action: "job/cancel" }, { action: "model/saveAs" },
        ])), /JOB_CANCELLATION_NOT_CONFIRMED/u);
        assert.equal(library.models.length, 0, "dependent writes require actual cancellation confirmation");
        assert.equal(solver.submissions.length, 1);
        if (status === "solving") {
          await page.waitForResponse((response) => new URL(response.url()).pathname === `/api/v1/jobs/${control.job.job_id}`);
          control.status = "cancelled";
          page.once("dialog", (dialog) => dialog.accept());
          await invoke(page, "job/cancel");
          await running;
          assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
        } else {
          const completed = await running;
          if (status === "completed") assert.equal(completed.ok, true);
          else assert.match(completed.error, /failed/u);
          await page.evaluate((jobStatus) => window.__kyuubikiPwdt.waitForState({ jobStatus }), status);
        }
      } finally { control.job.status = "failed"; await running; }
    });
  });
}

for (const failure of ["network", "malformed"]) {
  test(`Workbench confirmed cancellation survives ${failure} history refresh and retains a visibly stale catalog`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page) => {
      const { control, solver, running } = await startCancellationPreview(page);
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("library"));
      await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ libraryTab: "jobs" }));
      let recover = false;
      await page.route("**/api/v1/jobs", (route) => recover ? route.fallback()
        : failure === "network"
          ? route.fulfill({ status: 503, json: { error: "qualification history unavailable" } })
          : route.fulfill({ json: { jobs: null } }));
      try {
        const before = await page.evaluate(() => window.__kyuubikiPwdt.state());
        page.once("dialog", (dialog) => dialog.accept());
        assert.equal((await invoke(page, "job/cancel")).ok, true);
        await running;
        const after = await page.evaluate(() => window.__kyuubikiPwdt.state());
        assert.equal(after.jobStatus, "cancelled");
        assert.equal(after.hasResult, false);
        assert.equal(after.jobHistoryCount, before.jobHistoryCount);
        assert.equal(after.selectedAdminJobId, before.selectedAdminJobId);
        assert.equal(await page.locator(`[data-workbench-history-job-id="${control.job.job_id}"]`).count(), 1);
        const issue = page.locator('[data-workbench-alert-id="runtime-recovery-job_history"]');
        await issue.first().waitFor({ state: "visible" });
        assert.match(await issue.first().textContent(), failure === "network" ? /qualification history unavailable/u : /JOB_HISTORY_INVALID/u);
        recover = true;
        if (failure === "malformed") {
          await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
          await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
          await page.locator('[data-workbench-runtime-tab="watchdog"]').click();
          const refreshed = page.waitForResponse((response) => new URL(response.url()).pathname === "/api/v1/jobs", { timeout: 5000 });
          const retry = page.locator('[data-workbench-recovery-action="retry-all"]').click();
          await Promise.all([refreshed, retry]);
        } else await invoke(page, "runtime/refreshAll");
        await page.waitForFunction(() => !document.querySelector('[data-workbench-alert-id="runtime-recovery-job_history"]'));
        runtime.state.adminJobs.length = 0;
        await invoke(page, "runtime/refreshAll");
        await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobHistoryCount: 0, selectedAdminJobId: null }));
        assert.equal(solver.submissions.length, 1);
        assert.equal(control.cancellations, 1);
      } finally { control.job.status = "failed"; await running; }
    });
  });
}

for (const failFirst of [false, true]) {
  test(`Workbench overlapping cancellation shares one request and ${failFirst ? "allows retry after failure" : "one confirmed receipt"}`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page) => {
      const { control, running } = await startCancellationPreview(page);
      control.unavailable = failFirst;
      const pending = await holdRequest(page, `/api/v1/jobs/${control.job.job_id}/cancel`, "POST");
      let confirmed;
      const confirmations = new Promise((resolve) => { confirmed = resolve; });
      let dialogs = 0;
      page.on("dialog", async (dialog) => { await dialog.accept(); if (++dialogs === 2) confirmed(); });
      const cancelling = page.evaluate(() => Promise.allSettled([
        window.__kyuubikiPwdt.invoke("job/cancel"), window.__kyuubikiPwdt.invoke("job/cancel"),
      ]));
      try {
        await pending.received;
        await confirmations;
        await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        pending.release();
        const outcomes = await cancelling;
        assert.equal(control.cancellations, 1, "concurrent UI/PWDT invocations must not duplicate cancellation writes");
        for (const outcome of outcomes) assert.equal(outcome.status, failFirst ? "rejected" : "fulfilled");
        if (failFirst) {
          control.unavailable = false;
          await invoke(page, "job/cancel");
          assert.equal(control.cancellations, 2);
        } else {
          for (const outcome of outcomes) assert.equal(outcome.value.contextChanged, undefined);
        }
        await running;
        assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().hasResult), false);
      } finally { pending.release(); control.job.status = "failed"; await cancelling; await running; }
    });
  });
}

test("Workbench cancellation shares its receipt while the confirmed history refresh is pending", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    const { control, running } = await startCancellationPreview(page);
    const pending = await holdRequest(page, "/api/v1/jobs", "GET");
    let confirmed;
    const confirmations = new Promise((resolve) => { confirmed = resolve; });
    let dialogs = 0;
    page.on("dialog", async (dialog) => { await dialog.accept(); if (++dialogs === 2) confirmed(); });
    const cancelling = invoke(page, "job/cancel").catch((error) => ({ error: error.message }));
    let repeated = Promise.resolve(null);
    try {
      await pending.received;
      await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "cancelled", hasResult: false }));
      repeated = invoke(page, "job/cancel").catch((error) => ({ error: error.message }));
      await confirmations;
      await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      pending.release();
      const receipt = await cancelling;
      assert.equal(receipt.ok, true);
      assert.deepEqual(await repeated, receipt, "a pending receipt remains shared after cancellation changes the job status");
      assert.equal(control.cancellations, 1);
    } finally { pending.release(); control.job.status = "failed"; await cancelling; await repeated; await running; }
  });
});
