import assert from "node:assert/strict";
import { before, after, test } from "node:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { chromium, startIsolatedWorkbenchUiRuntime, workbenchUrl } from "./workbench-ui-isolated.shared.mjs";

let browser;
let runtime;
before(async () => {
  runtime = await startIsolatedWorkbenchUiRuntime();
  browser = await launchIntegrationBrowser(chromium);
}, { timeout: 180_000 });
after(async () => { await browser?.close(); await runtime?.stop(); }, { timeout: 90_000 });

const entries = Array.from({ length: 18 }, (_, index) => ({
  id: `store-operator-${index}`, kind: "operator", title: `Mechanical research operator ${index}`,
  summary: "A catalog reference for a reproducible mechanical research workflow.",
  version: "1.0.0", source_id: index < 9 ? "builtin.local" : "lab",
  source_kind: index < 9 ? "builtin" : "catalog_file", domain: index % 2 ? "thermal" : "mechanical",
  tags: ["research", "headless"], install: { mode: "source_reference", requires_download: index >= 9 },
}));
const sources = ["builtin.local", "lab"].map((id) => ({
  id, label: id, type: id === "lab" ? "catalog_file" : "builtin", enabled: true, editable: false,
  status: "ready", supports: ["operator"],
}));

test("Store PWDT selects an exact source/version and agrees with the GUI reference", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  let legacyCalls = 0;
  const candidates = [
    { ...entries[0], id: "shared.operator", title: "Shared operator" },
    { ...entries[0], id: "shared.operator", title: "Shared operator", source_id: "lab" },
    { ...entries[0], id: "shared.operator", title: "Shared operator", source_id: "lab", version: "2.0.0",
      package_ref: "operator://shared/2", install: { mode: "source_reference", requires_download: true, target: "macos-aarch64" } },
  ];
  page.on("pageerror", (error) => errors.push(error.message));
  const stage = (selection = {}) => page.evaluate((selection) =>
    window.__kyuubikiPwdt.stageStoreEntry("operator", "shared.operator", selection), selection);
  const manifestEntries = () => page.evaluate(() => JSON.parse(
    localStorage.getItem("kyuubiki-workbench-store-manifests") || "{}",
  )["qualification-project"]?.entries || []);
  try {
    await page.route("**/api/v1/store**", async (route) => {
      if (new URL(route.request().url()).pathname !== "/api/v1/store") {
        legacyCalls++;
        await route.fulfill({ json: { entry: candidates[0] } });
        return;
      }
      // Deliberately ignore server-side filters: the client must still check identity.
      await route.fulfill({ json: { entries: candidates, sources,
        summary: { entry_count: candidates.length, kinds: { operator: candidates.length }, sources: {} } } });
    });
    await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ selectedProjectId: "qualification-project" }));
    await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("store"));
    await page.locator('[data-workbench-store-entry-id="shared.operator"]').first().waitFor();
    await assert.rejects(stage(), /store_entry_ambiguous/);
    await assert.rejects(stage({ sourceId: "lab" }), /store_entry_ambiguous/);
    await assert.rejects(stage({ sourceId: "lab", version: "missing" }), /store_entry_not_found/);
    assert.deepEqual(await manifestEntries(), []);
    const selection = { sourceId: "lab", version: "2.0.0", packageRef: "operator://shared/2", target: "macos-aarch64" };
    const result = await stage(selection);
    for (const [key, value] of Object.entries(selection)) assert.equal(result[key], value);
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 1 }));
    const staged = await manifestEntries();
    assert.equal(staged[0].source_id, "lab");
    assert.equal(staged[0].version, "2.0.0");
    await assert.rejects(stage({ sourceId: "builtin.local" }), /Another source or version/);
    assert.deepEqual(await manifestEntries(), staged, "a rejected replacement cannot rewrite the existing reference");
    let confirmed = false;
    page.once("dialog", async (dialog) => {
      assert.equal(dialog.type(), "confirm");
      confirmed = true;
      await dialog.accept();
    });
    await page.evaluate(() => window.__kyuubikiPwdt.removeStoreEntry("operator", "shared.operator"));
    assert.equal(confirmed, true, "explicit removal retains the product confirmation boundary");
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 0 }));
    const chosenCard = page.locator('[data-workbench-store-entry-id="shared.operator"]').nth(2);
    await chosenCard.locator('[data-workbench-store-entry-action="details"]').click();
    await page.locator('[data-workbench-store-detail-action="stage"]').click();
    await page.evaluate(() => window.__kyuubikiPwdt.waitForState({ storeManifestEntryCount: 1 }));
    const fromGui = await manifestEntries();
    assert.deepEqual({ ...fromGui[0], installed_at: null }, { ...staged[0], installed_at: null });
    assert.equal(legacyCalls, 0, "no unqualified kind/id detail fallback");
    assert.deepEqual(errors, []);
  } finally { await context.close(); }
}, { timeout: 120_000 });

for (const viewport of [{ width: 1440, height: 900 }, { width: 390, height: 844 }]) {
  test(`Store workspace browse, filter, detail and manifest loop at ${viewport.width}px`, async () => {
    const context = await browser.newContext({ viewport });
    const page = await context.newPage();
    const errors = [];
    let failNext = false;
    page.on("pageerror", (error) => errors.push(error.message));
    try {
      await page.route("**/api/v1/store**", async (route) => {
        if (failNext) {
          failNext = false;
          await route.fulfill({ status: 503, json: { error: "Store temporarily unavailable" } });
          return;
        }
        const query = new URL(route.request().url()).searchParams.get("q") || "";
        const filtered = entries.filter((entry) => entry.title.includes(query));
        await route.fulfill({ json: { entries: filtered, sources, summary: { entry_count: filtered.length, kinds: { operator: filtered.length }, sources: {} } } });
      });
      await page.goto(workbenchUrl(runtime), { waitUntil: "networkidle", timeout: 60_000 });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await page.evaluate(() => window.__kyuubikiPwdt.waitUntil(
        (state) => state.selectedProjectId === "qualification-project", { timeoutMs: 15_000 },
      ));
      await page.locator('[aria-label="workbench-rail:store"]').click();
      await page.locator('[data-workbench-store-entry-id]').first().waitFor();
      assert.equal(await page.locator('[data-workbench-store-entry-id]').count(), 4);
      await page.locator('[data-workbench-store-workspace-action="expand"]').click();
      const dialog = page.locator('[data-workbench-store-workspace="expanded"]');
      await dialog.waitFor();
      assert.equal(await dialog.evaluate((element) => element.matches(":modal")), true);
      assert.equal(await dialog.locator('[data-workbench-store-entry-id]').count(), viewport.width <= 600 ? 4 : 12);
      assert.equal(await dialog.locator('[data-workbench-panel-content]').evaluate((element) => element.scrollWidth <= element.clientWidth + 1), true);
      const box = await dialog.boundingBox();
      assert.ok(box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width + 1 && box.y + box.height <= viewport.height + 1);
      if (process.env.KYUUBIKI_STORE_SCREENSHOT_DIR) {
        await mkdir(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, { recursive: true });
        await page.screenshot({ path: join(process.env.KYUUBIKI_STORE_SCREENSHOT_DIR, `store-${viewport.width}.png`) });
      }
      await dialog.locator("details > summary").click();
      await dialog.locator('[data-workbench-store-filter="source"]').selectOption("lab");
      await dialog.locator('[data-workbench-store-filter="domain"]').selectOption("mechanical");
      assert.equal(await dialog.locator('[data-workbench-store-entry-id]').count(), 4);
      await dialog.locator('[data-workbench-store-entry-action="details"]').first().click();
      const detail = dialog.locator('[data-workbench-store-detail]');
      await detail.waitFor();
      assert.match(await detail.textContent(), /not connected yet|尚未接通|未接続/);
      await detail.locator('[data-workbench-store-detail-action="stage"]').click();
      await page.waitForFunction(() => document.querySelector('[data-workbench-store-panel]')?.getAttribute("data-workbench-store-manifest-count") === "1");
      assert.equal(await detail.locator('[data-workbench-store-detail-action="stage"]').isDisabled(), true);
      await dialog.locator('[data-workbench-store-view-tab="project"]').click();
      await dialog.locator('[data-workbench-store-manifest-action="remove"]').click();
      assert.equal(await dialog.locator('[data-workbench-store-manifest-entry-id]').count(), 0);
      await dialog.locator('[data-workbench-store-view-tab="catalog"]').click();
      assert.equal(await dialog.locator('[data-workbench-store-filter="source"]').inputValue(), "lab");
      const search = dialog.locator('[data-workbench-store-search="query"]');
      await search.fill("not-found");
      await search.press("Enter");
      await page.waitForFunction(() => document.querySelector('[data-workbench-store-panel]')?.getAttribute("data-workbench-store-visible-count") === "0");
      await search.fill("");
      await search.press("Enter");
      await dialog.locator('[data-workbench-store-entry-id]').first().waitFor();
      failNext = true;
      await dialog.locator('[data-workbench-store-action="refresh"]').click();
      await dialog.locator('[data-workbench-store-status="error"]').waitFor();
      assert.equal(await dialog.locator('[data-workbench-store-entry-action="stage"]:enabled').count(), 0);
      await dialog.locator('[data-workbench-store-action="refresh"]').click();
      await dialog.locator('[data-workbench-store-status="ready"]').waitFor();
      assert.equal(await dialog.locator('[data-workbench-store-entry-action="stage"]:enabled').count(), 4);
      await page.keyboard.press("Escape");
      await dialog.waitFor({ state: "detached" });
      assert.equal(await page.locator('[data-workbench-store-workspace-action="expand"]').evaluate((element) => element === document.activeElement), true);
      assert.deepEqual(errors, []);
    } finally { await context.close(); }
  }, { timeout: 120_000 });
}
