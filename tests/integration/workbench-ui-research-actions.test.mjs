import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, holdRequest, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";
import { installProjectionSolver } from "./workbench-ui-pwdt-projection-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const action = (page, name) => page.locator(`[data-workbench-research-action="${name}"]`);

test("research toolbar completes setup, inline checkpoint, run and result without leaving the model", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    const solver = await installProjectionSolver(page);
    await openWorkbench(page);
    await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    assert.equal(await action(page, "result").isDisabled(), true);
    await action(page, "study").click();
    await page.locator('[data-workbench-model-study="panel"]').waitFor();
    await action(page, "model").click();
    assert.equal(await action(page, "model").getAttribute("aria-pressed"), "true");
    await action(page, "save").click();
    const save = page.locator('[data-workbench-research="save"]');
    await save.locator('[data-workbench-immersive-save="name"]').fill("Nearby research checkpoint");
    await save.locator('[data-workbench-immersive-save="save"]').click();
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt.state().selectedVersionId));
    assert.equal(library.models.length, 1);
    assert.equal(library.versions.length, 1);
    const version = library.versions[0].version_id;
    await page.waitForFunction(() => !document.querySelector('[data-workbench-research="save"] input')?.disabled);
    await save.locator("input").press("Escape");
    await save.waitFor({ state: "detached" });
    assert.equal(await save.count(), 0);
    assert.equal(await action(page, "save").evaluate((button) => button === document.activeElement), true);
    await action(page, "run").click();
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ hasResult: true, jobStatus: "completed" }));
    assert.equal(solver.submissions.length, 1);
    assert.equal(solver.submissions[0].input.model_version_id, version);
    await action(page, "result").click();
    await page.locator('[data-workbench-panel="inspector"][data-workbench-inspector-tab="result"]').waitFor();
    assert.equal(await page.evaluate(() => window.__kyuubikiPwdt.state().sidebarSection), "model");
    await page.locator('[data-workbench-inspector-tab-target="status"]').click();
    await action(page, "result").click();
    await page.locator('[data-workbench-inspector-tab="result"]').waitFor();
  });
});

test("research toolbar rejects duplicate submissions and recovers after a failed call", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const solver = await installProjectionSolver(page);
    await openWorkbench(page);
    await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    let attempts = 0;
    const held = await holdRequest(page, "/api/v1/fem/heat-bar-1d/jobs", "POST", (route) => {
      attempts += 1;
      return route.fulfill({ status: 503, json: { error: "research service temporarily unavailable" } });
    });
    try {
      await action(page, "run").evaluate((button) => { button.click(); button.click(); });
      await held.received;
      assert.equal(await action(page, "run").isDisabled(), true);
      held.release();
      await page.locator('[data-workbench-research="toolbar"] [role="alert"]').waitFor();
      assert.equal(attempts, 1);
      assert.equal(await action(page, "run").isEnabled(), true);
      assert.equal(solver.submissions.length, 0);
    } finally { held.release(); }
    await page.unroute("**/api/v1/fem/heat-bar-1d/jobs");
    await action(page, "run").click();
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ hasResult: true }));
    assert.equal(solver.submissions.length, 1);
    assert.equal(await page.locator('[data-workbench-research="toolbar"] [role="alert"]').count(), 0);
  });
});

test("research submission stays locked after leaving and reopening the model panel", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const solver = await installProjectionSolver(page);
    await openWorkbench(page);
    await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    const held = await holdRequest(page, "/api/v1/fem/heat-bar-1d/jobs", "POST");
    try {
      await action(page, "run").click();
      await held.received;
      await invoke(page, "nav/setSidebarSection", { section: "system" });
      await invoke(page, "nav/setSidebarSection", { section: "model" });
      assert.equal(await action(page, "run").isDisabled(), true);
      held.release();
      await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ hasResult: true }));
      assert.equal(solver.submissions.length, 1);
    } finally { held.release(); }
  });
});

test("research toolbar can cancel a running observation without duplicate requests or stale errors", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const solver = await installProjectionSolver(page);
    let cancellations = 0;
    let releaseCancel;
    const cancelGate = new Promise((resolve) => { releaseCancel = resolve; });
    solver.beforeJobResponse = async (job) => {
      await page.route(`**/api/v1/jobs/${job.job_id}`, (route) => route.fulfill({ json: { job } }));
      await page.route(`**/api/v1/jobs/${job.job_id}/cancel`, async (route) => {
        cancellations += 1;
        await cancelGate;
        job.status = "cancelled";
        await route.fulfill({ json: { job } });
      });
    };
    await openWorkbench(page);
    await invoke(page, "nav/setStudyKind", { studyKind: "heat_bar_1d" });
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    try {
      await action(page, "run").click();
      await action(page, "cancel").waitFor();
      assert.equal(await action(page, "run").isDisabled(), true);
      assert.equal(await action(page, "cancel").isEnabled(), true);
      page.once("dialog", (dialog) => dialog.accept());
      await action(page, "cancel").evaluate((button) => { button.click(); button.click(); });
      await page.waitForFunction(() => document.querySelector('[data-workbench-research-action="cancel"]')?.disabled);
      releaseCancel();
      await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ jobStatus: "cancelled" }));
      await page.waitForFunction(() => !document.querySelector('[data-workbench-research-action="run"]')?.disabled);
      assert.equal(cancellations, 1);
      assert.equal(solver.submissions.length, 1);
      assert.equal(await action(page, "result").isDisabled(), true);
      assert.equal(await page.locator('[data-workbench-research="toolbar"] [role="alert"]').count(), 0);
    } finally { releaseCancel(); }
  });
});

test("research actions fit compact, RTL and desktop layouts without displacing the viewport", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openWorkbench(page);
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
    for (const [language, width, height] of [["zh", 1440, 900], ["de", 1024, 768], ["ar", 768, 720], ["es", 390, 844]]) {
      await page.setViewportSize({ width, height });
      await invoke(page, "settings/patch", { language });
      const toolbar = page.locator('[data-workbench-research="toolbar"]');
      await toolbar.waitFor();
      const fits = await toolbar.evaluate((root) => {
        const bounds = root.getBoundingClientRect();
        return [...root.querySelectorAll("button")].every((button) => {
          const rect = button.getBoundingClientRect();
          return rect.width > 0 && rect.left >= bounds.left - 1 && rect.right <= bounds.right + 1;
        }) && root.scrollWidth <= root.clientWidth + 1;
      });
      assert.equal(fits, true, `${language} controls fit at ${width}`);
      const stage = await page.locator('[data-workbench-viewport="stage"]').boundingBox();
      assert.ok(stage.height >= 200, `${language} main visualization retains height: ${stage.height}`);
      await action(page, "save").click();
      await page.locator('[data-workbench-research="save"]').waitFor();
      assert.equal(await toolbar.evaluate((root) => root.scrollWidth <= root.clientWidth + 1), true);
      await action(page, "save").click();
    }
    assert.deepEqual(library.writes, []);
  });
});
