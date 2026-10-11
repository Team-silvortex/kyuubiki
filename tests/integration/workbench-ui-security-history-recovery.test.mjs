import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, runtime,
  installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();

function event(id, overrides = {}) {
  return { event_id: id, event_type: "security_high_risk_action", action: "qualification/read-history",
    source: "script", risk: "sensitive", status: "completed", occurred_at: new Date().toISOString(),
    context: { study_kind: "heat_bar_1d", project_id: "qualification-project" }, ...overrides };
}

async function openAudit(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ systemPanelTab: "runtime" }));
  await page.locator('[data-workbench-runtime-tab="audit"]').click();
  await page.locator('[data-workbench-audit="panel"]').waitFor({ state: "visible" });
}

async function refreshAudit(page) {
  const request = page.waitForRequest((entry) => new URL(entry.url()).pathname === "/api/v1/security-events");
  await page.locator('[data-workbench-audit-action="refresh"]').click();
  await request;
  await page.waitForLoadState("networkidle");
}

async function selectSource(page, value) {
  await page.locator('[data-workbench-audit-page="filters"]').click();
  await page.locator('[data-workbench-audit-filter="source"]').selectOption(value);
}

function assertReadOnly(library) {
  assert.equal(library.writes.length, 0);
  assert.equal(runtime.state.projectMutations.length, 0);
  assert.equal(runtime.state.catalogSubmissions + runtime.state.graphSubmissions, 0);
}

for (const failure of ["http", "offline", "malformed", "invalid-entry", "duplicate-entry"]) {
  test(`Workbench audit ${failure} refresh retains known records and recovers through the real button`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      let mode = "ready";
      let writes = 0;
      let reads = 0;
      const records = [event("known-history")];
      await page.route("**/api/v1/security-events**", (route) => {
        if (route.request().method() !== "GET") { writes += 1; return route.fallback(); }
        reads += 1;
        if (mode === "offline") return route.abort("failed");
        const payload = mode === "malformed" ? { events: null }
          : mode === "invalid-entry" ? { events: [event("invalid", { context: null })] }
          : mode === "duplicate-entry" ? { events: [event("duplicate"), event("duplicate")] }
          : { events: records };
        return route.fulfill(mode === "http" ? { status: 503, json: { error: "qualification audit unavailable" } }
          : { json: payload });
      });
      await openWorkbench(page);
      await openAudit(page);
      const row = page.locator('[data-workbench-audit-event="known-history"]');
      await row.waitFor({ state: "visible" });
      mode = failure;
      const before = reads;
      await refreshAudit(page);
      assert.equal(reads, before + 1, "a failed read must not trigger automatic retries");
      assert.equal(await row.count(), 1, "read failure must not erase known audit history");
      const warning = page.locator('[data-workbench-audit="read-error"]');
      await warning.waitFor({ state: "visible" });
      assert.match(await warning.textContent(), failure === "http" ? /qualification audit unavailable/u
        : failure === "offline" ? /fetch|network|load failed/iu : /SECURITY_EVENT_HISTORY_INVALID/u);
      mode = "ready";
      records.push(event("recovered-history"));
      await refreshAudit(page);
      await page.locator('[data-workbench-audit-event="recovered-history"]').waitFor({ state: "visible" });
      assert.equal(await warning.count(), 0);
      records.length = 0;
      await refreshAudit(page);
      await page.locator('[data-workbench-audit="empty"]').waitFor({ state: "visible" });
      assert.equal(await page.locator('[data-workbench-audit-event]').count(), 0);
      assert.equal(await page.locator('[data-workbench-audit="empty"]').count(), 1);
      assert.equal(writes, 0);
      assertReadOnly(library);
    });
  });
}

test("Workbench audit first-read failure is unavailable, not an authoritative empty history", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    let unavailable = true;
    await page.route("**/api/v1/security-events**", (route) => route.fulfill(unavailable
      ? { status: 503, json: { error: "qualification first audit unavailable" } } : { json: { events: [] } }));
    await openWorkbench(page);
    await openAudit(page);
    await page.locator('[data-workbench-audit="read-error"]').waitFor({ state: "visible" });
    assert.equal(await page.locator('[data-workbench-audit="empty"]').count(), 0);
    unavailable = false;
    await invoke(page, "runtime/refreshAll");
    await page.locator('[data-workbench-audit="empty"]').waitFor({ state: "visible" });
    assert.equal(await page.locator('[data-workbench-audit="read-error"]').count(), 0);
    assertReadOnly(library);
  });
});

test("Workbench audit pending filtered history never masquerades as empty or reuses old facets", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    let release;
    let reached;
    const gate = new Promise((resolve) => { release = resolve; });
    const received = new Promise((resolve) => { reached = resolve; });
    await page.route("**/api/v1/security-events**", async (route) => {
      if (new URL(route.request().url()).searchParams.get("source") === "assistant") {
        reached();
        await gate;
        return route.fulfill({ json: { events: [] } });
      }
      return route.fulfill({ json: { events: [event("previous-filter")] } });
    });
    await openWorkbench(page);
    await openAudit(page);
    await page.locator('[data-workbench-audit-event="previous-filter"]').waitFor({ state: "visible" });
    try {
      await selectSource(page, "assistant");
      await received;
      await page.locator('[data-workbench-audit="read-pending"]').waitFor({ state: "visible" });
      assert.equal(await page.locator('[data-workbench-audit="panel"]').getAttribute("aria-busy"), "true");
      await page.locator('[data-workbench-audit-page="events"]').click();
      assert.equal(await page.locator('[data-workbench-audit="empty"]').count(), 0);
      assert.equal(await page.locator('[data-workbench-audit-event]').count(), 0);
      for (const tab of ["summary", "facets"]) {
        await page.locator(`[data-workbench-audit-page="${tab}"]`).click();
        assert.equal(await page.locator(`[data-workbench-audit-content="${tab}"]`).count(), 0);
      }
      release();
      await page.waitForLoadState("networkidle");
      await page.locator('[data-workbench-audit-page="events"]').click();
      await page.locator('[data-workbench-audit="empty"]').waitFor({ state: "visible" });
      assert.equal(await page.locator('[data-workbench-audit="panel"]').getAttribute("aria-busy"), "false");
      assertReadOnly(library);
    } finally { release(); }
  });
});

test("Workbench audit filter failure never shows records from the previous scope", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    let unavailable = true;
    await page.route("**/api/v1/security-events**", (route) => {
      const source = new URL(route.request().url()).searchParams.get("source");
      return route.fulfill(source === "assistant" && unavailable
        ? { status: 503, json: { error: "qualification filtered audit unavailable" } }
        : { json: { events: [event(source === "assistant" ? "assistant-history" : "script-history", { source: source || "script" })] } });
    });
    await openWorkbench(page);
    await openAudit(page);
    await page.locator('[data-workbench-audit-event="script-history"]').waitFor({ state: "visible" });
    await selectSource(page, "assistant");
    await page.locator('[data-workbench-audit-page="events"]').click();
    await page.locator('[data-workbench-audit="read-error"]').waitFor({ state: "visible" });
    assert.equal(await page.locator('[data-workbench-audit-event="script-history"]').count(), 0);
    assert.equal(await page.locator('[data-workbench-audit="empty"]').count(), 0);
    unavailable = false;
    await refreshAudit(page);
    await page.locator('[data-workbench-audit-event="assistant-history"]').waitFor({ state: "visible" });
    assert.equal(await page.locator('[data-workbench-audit="read-error"]').count(), 0);
    assertReadOnly(library);
  });
});

for (const staleFails of [false, true]) {
  test(`Workbench audit ignores late ${staleFails ? "failure" : "success"} after a filter change`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      let hold = false;
      let release;
      let reached;
      const gate = new Promise((resolve) => { release = resolve; });
      const received = new Promise((resolve) => { reached = resolve; });
      await page.route("**/api/v1/security-events**", async (route) => {
        const source = new URL(route.request().url()).searchParams.get("source");
        if (hold && !source) {
          reached();
          await gate;
          return route.fulfill(staleFails ? { status: 503, json: { error: "stale audit unavailable" } }
            : { json: { events: [event("stale-history")] } });
        }
        return route.fulfill({ json: { events: [event(source ? "current-history" : "initial-history", { source: source || "script" })] } });
      });
      await openWorkbench(page);
      await openAudit(page);
      hold = true;
      try {
        await page.locator('[data-workbench-audit-action="refresh"]').click();
        await received;
        await selectSource(page, "assistant");
        await page.locator('[data-workbench-audit-page="events"]').click();
        await page.locator('[data-workbench-audit-event="current-history"]').waitFor({ state: "visible" });
        release();
        await page.waitForLoadState("networkidle");
        assert.equal(await page.locator('[data-workbench-audit-event="current-history"]').count(), 1);
        assert.equal(await page.locator('[data-workbench-audit-event="stale-history"]').count(), 0);
        assert.equal(await page.locator('[data-workbench-audit="read-error"]').count(), 0);
        assertReadOnly(library);
      } finally { release(); }
    });
  });
}

const concurrentCases = [false, true].flatMap((latestFails) =>
  [false, true].map((staleFails) => ({ latestFails, staleFails })));
for (const { latestFails, staleFails } of concurrentCases) {
  test(`Workbench audit concurrent refresh discards late ${staleFails ? "failure" : "success"} after the latest ${latestFails ? "failure" : "success"}`, { timeout: 75_000 }, async () => {
    await usingWorkbench(async (page, library) => {
      let reads = 0;
      let hold = false;
      let held = false;
      let release;
      let reached;
      const gate = new Promise((resolve) => { release = resolve; });
      const received = new Promise((resolve) => { reached = resolve; });
      await page.route("**/api/v1/security-events**", async (route) => {
        reads += 1;
        if (hold && !held) {
          held = true;
          reached();
          await gate;
          return route.fulfill(staleFails ? { status: 503, json: { error: "obsolete refresh unavailable" } }
            : { json: { events: [event("obsolete-refresh")] } });
        }
        return route.fulfill(hold && latestFails ? { status: 503, json: { error: "latest refresh unavailable" } }
          : { json: { events: [event(hold ? "latest-refresh" : "initial-refresh")] } });
      });
      await openWorkbench(page);
      await openAudit(page);
      const before = reads;
      hold = true;
      try {
        await page.locator('[data-workbench-audit-action="refresh"]').click();
        await received;
        assert.equal(await page.locator('[data-workbench-audit-event="initial-refresh"]').count(), 1);
        await page.locator('[data-workbench-audit-action="refresh"]').click();
        const expected = latestFails ? '[data-workbench-audit="read-error"]' : '[data-workbench-audit-event="latest-refresh"]';
        await page.locator(expected).waitFor({ state: "visible" });
        assert.equal(await page.locator('[data-workbench-audit="read-pending"]').count(), 0);
        release();
        await page.waitForLoadState("networkidle");
        assert.equal(await page.locator(`[data-workbench-audit-event="${latestFails ? "initial-refresh" : "latest-refresh"}"]`).count(), 1);
        assert.equal(await page.locator('[data-workbench-audit-event="obsolete-refresh"]').count(), 0);
        assert.equal(await page.locator('[data-workbench-audit="read-error"]').count(), latestFails ? 1 : 0);
        if (latestFails) assert.match(await page.locator('[data-workbench-audit="read-error"]').textContent(), /latest refresh unavailable/u);
        assert.equal(reads, before + 2);
        assertReadOnly(library);
      } finally { release(); }
    });
  });
}
