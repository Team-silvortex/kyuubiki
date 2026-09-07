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
