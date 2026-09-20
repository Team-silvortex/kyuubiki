import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";
import { ROOT } from "./workbench-ui-isolated.shared.mjs";

installProjectWorkbenchTestHooks();
const materialPage = (page, name) => page.locator(`[data-workbench-material-page="${name}"]`);
const materialControl = (page, name) => page.locator(`[data-workbench-material-control="${name}"]`);
const auditPage = (page, name) => page.locator(`[data-workbench-audit-page="${name}"]`);
const auditAction = (page, name) => page.locator(`[data-workbench-audit-action="${name}"]`);
const auditFilter = (page, name) => page.locator(`[data-workbench-audit-filter="${name}"]`);

function materialModel(count = 1) {
  return {
    nodes: [
      { id: "node-0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
      { id: "node-1", x: 1, y: 0, z: 0, fix_x: false, fix_y: true, fix_z: true, load_x: 10, load_y: 0, load_z: 0 },
    ],
    elements: [{ id: "member-0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9, material_id: "sample-0" }],
    materials: Array.from({ length: count }, (_, i) => ({ id: `sample-${i}`, name: `Research material ${i}`, youngs_modulus: 70e9 })),
  };
}

async function openMaterials(page, count = 1) {
  await openWorkbench(page);
  await invoke(page, "state/replaceTruss3dModel", materialModel(count));
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
  await page.evaluate(() => window.__kyuubikiPwdt.openTabs({ modelTab: "tools", modelToolsPage: "materials" }));
  await page.locator('[data-workbench-materials="panel"]').waitFor();
  // The replacement action resolves solver input, so populate the authoring library through its import UI.
  await materialPage(page, "add").click();
  await materialControl(page, "import").setInputFiles({ name: "materials.json", mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(materialModel(count).materials)) });
  await page.locator('[data-workbench-material-id="sample-0"]').click();
  await materialControl(page, "apply-all").click();
  await materialPage(page, "browse").click();
}

async function savePayload(page, library) {
  await page.evaluate(() => window.__kyuubikiPwdt.saveModel({ name: "compact-panel-model", saveAs: true }));
  return library.versions.at(-1).payload;
}

async function mockAudit(page) {
  const events = Array.from({ length: 23 }, (_, i) => ({
    event_id: `event-${i}`, event_type: "automation", source: i % 2 ? "assistant" : "script",
    action: `research/step-${i}`, risk: "low", status: "completed", note: `Research event ${i}`,
    context: { study_kind: "truss_3d", project_id: "qualification-project", model_version_id: "audit-model" },
    occurred_at: new Date(Date.now() - i * 1000).toISOString(),
  }));
  const requests = [];
  await page.route("**/api/v1/**security-events*", async (route) => {
    const url = new URL(route.request().url());
    if (route.request().method() !== "GET") return route.fallback();
    requests.push(url);
    const matched = events.filter((entry) => ["source", "risk", "status", "action"].every((key) =>
      !url.searchParams.has(key) || entry[key].includes(url.searchParams.get(key))));
    if (url.pathname.endsWith(".csv")) return route.fulfill({ contentType: "text/csv", body: `event_id\n${matched.map((entry) => entry.event_id).join("\n")}\n` });
    if (url.pathname.includes("/export/")) return route.fulfill({ json: { exported_at: new Date().toISOString(), events: matched,
      filters: Object.fromEntries(url.searchParams), summary: { total: matched.length } } });
    return route.fulfill({ json: { events: matched } });
  });
  return { events, requests };
}

async function openRuntime(page, tab) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.locator('[data-workbench-system-surface-tab="runtime"]').click();
  await page.locator(`[data-workbench-runtime-tab="${tab}"]`).click();
}

test("material library bounds 1000 entries and edits only the explicitly selected material", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openMaterials(page, 1000);
    assert.equal(await page.locator("[data-workbench-material-id]").count(), 8);
    assert.equal(await page.locator("[data-workbench-material-editor]").count(), 0);
    await materialControl(page, "search").fill("sample-701");
    assert.equal(await page.locator("[data-workbench-material-id]").count(), 1);
    await page.locator('[data-workbench-material-id="sample-701"]').click();
    assert.equal(await page.locator("[data-workbench-material-editor]").count(), 1);
    await page.waitForFunction(() => document.activeElement?.getAttribute("data-workbench-material-control") === "name");
    await materialControl(page, "name").fill("Revised material");
    await materialControl(page, "modulus").fill("125");
    assert.equal(await materialControl(page, "apply-selected").isDisabled(), true);
    await materialPage(page, "browse").click();
    assert.equal(await materialControl(page, "search").inputValue(), "sample-701");
    await materialControl(page, "search").fill("revised material");
    await page.locator('[data-workbench-material-id="sample-701"]').click();
    assert.equal(await materialControl(page, "modulus").inputValue(), "125");
    const saved = await savePayload(page, library);
    assert.equal(saved.materials.length, 1000);
    assert.equal(saved.materials[701].name, "Revised material");
    assert.equal(saved.materials[701].youngs_modulus, 125e9);
    assert.equal(saved.materials[700].youngs_modulus, 70e9);
    assert.equal(saved.elements[0].material_id, "sample-0", "browsing or editing cannot assign another material");
    await materialPage(page, "browse").click();
    await materialControl(page, "search").fill("no such material");
    assert.equal(await page.locator("[data-workbench-material-id]").count(), 0);
    await materialControl(page, "search").fill("");
    await page.locator('[data-workbench-list-page="next"]').click();
    assert.equal(await page.locator("[data-workbench-material-id]").first().getAttribute("data-workbench-material-id"), "sample-8");
  });
});

test("material add, assign, visibility and delete keep the original model and undo chain", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openMaterials(page);
    await materialPage(page, "add").click();
    await materialControl(page, "add-preset").click();
    await page.locator('[data-workbench-material-editor="mat-1"]').waitFor();
    await materialPage(page, "add").click();
    await materialControl(page, "add-custom").click();
    await page.locator('[data-workbench-material-editor="mat-2"]').waitFor();
    await materialControl(page, "name").fill("Custom research material");
    await materialControl(page, "modulus").fill("90");
    await materialControl(page, "visibility").click();
    await materialPage(page, "browse").click();
    assert.match(await page.locator('[data-workbench-material-id="mat-2"]').innerText(), /Hide/);
    await page.locator('[data-workbench-material-id="mat-2"]').click();
    await materialControl(page, "visibility").click();
    await materialControl(page, "apply-all").click();
    assert.equal((await savePayload(page, library)).elements[0].material_id, "mat-2");
    await invoke(page, "history/undo");
    assert.equal((await savePayload(page, library)).elements[0].material_id, "sample-0");
    await materialControl(page, "delete").click();
    assert.equal(await page.locator('[data-workbench-material-id="mat-2"]').count(), 0);
    await invoke(page, "history/undo");
    await page.locator('[data-workbench-material-id="mat-2"]').waitFor();
    await page.locator('[data-workbench-material-id="mat-2"]').click();
    assert.equal(await materialControl(page, "name").inputValue(), "Custom research material");
  });
});

test("material import can reuse the same filename without overwriting existing materials", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page, library) => {
    await openMaterials(page);
    for (const [id, name] of [["imported", "Imported first"], ["imported-2", "Imported revised"]]) {
      await materialPage(page, "add").click();
      await materialControl(page, "import").setInputFiles({ name: "research-materials.json", mimeType: "application/json",
        buffer: Buffer.from(JSON.stringify([{ id: "imported", name, youngs_modulus: 110e9 }])) });
      await page.locator(`[data-workbench-material-id="${id}"]`).waitFor();
      assert.match(await page.locator(`[data-workbench-material-id="${id}"]`).innerText(), new RegExp(name));
      assert.equal(await page.locator("[data-workbench-material-editor]").count(), 0);
    }
    const saved = await savePayload(page, library);
    assert.equal(saved.materials.filter((entry) => entry.id === "imported").length, 1);
    assert.equal(saved.materials.find((entry) => entry.id === "imported").name, "Imported first");
    assert.equal(saved.materials.find((entry) => entry.id === "imported-2").name, "Imported revised");
  });
});

test("audit subpages retain filters and export all matching events rather than the visible page", { timeout: 90_000 }, async () => {
  await usingWorkbench(async (page) => {
    const mock = await mockAudit(page);
    await openWorkbench(page);
    await openRuntime(page, "audit");
    assert.equal(await page.locator("[data-workbench-audit-event]").count(), 10);
    assert.equal(await page.locator("[data-workbench-audit-filter]").count(), 0);
    await page.locator('[data-workbench-list-page="next"]').click();
    await page.locator('[data-workbench-list-page="next"]').click();
    assert.equal(await page.locator("[data-workbench-audit-event]").count(), 3);
    for (const name of ["summary", "facets"]) {
      await auditPage(page, name).click();
      assert.equal(await page.locator("[data-workbench-audit-content]").count(), 1);
      assert.equal(await page.locator("[data-workbench-audit-event]").count(), 0);
    }
    await auditPage(page, "events").click();
    assert.equal(await page.locator("[data-workbench-audit-event]").count(), 3);
    const event = page.locator("[data-workbench-audit-event]").first();
    assert.equal(await event.locator("p").isVisible(), false);
    await event.locator("summary").click();
    assert.match(await event.locator("p").innerText(), /Research event/);
    await auditPage(page, "filters").click();
    const filtered = page.waitForResponse((response) => response.url().includes("security-events?") && response.url().includes("source=script"));
    await auditFilter(page, "source").selectOption("script");
    await filtered;
    await auditPage(page, "events").click();
    assert.equal(await page.locator("[data-workbench-audit-event]").count(), 10);
    assert.match(await page.locator('[data-workbench-audit="active-filters"]').innerText(), /Script/);
    await page.locator('[data-workbench-list-page="next"]').click();
    assert.equal(await page.locator("[data-workbench-audit-event]").count(), 2);
    await auditAction(page, "exports-toggle").click();
    for (const format of ["json", "csv"]) {
      const downloadPromise = page.waitForEvent("download");
      await auditAction(page, `export-${format}`).click();
      const content = await readFile(await (await downloadPromise).path(), "utf8");
      if (format === "json") assert.equal(JSON.parse(content).events.length, 12);
      else assert.equal(content.trim().split("\n").length, 13);
    }
    await auditPage(page, "filters").click();
    assert.equal(await auditFilter(page, "source").inputValue(), "script");
    await auditAction(page, "clear-filters").click();
    await auditPage(page, "events").click();
    await auditAction(page, "refresh").click();
    await page.waitForFunction(() => document.querySelector('[data-workbench-audit="panel"] .card-head > span')?.textContent === "23");
    assert.equal(await page.locator("[data-workbench-audit-event]").count(), 10);
    assert.equal(mock.requests.at(-1).searchParams.has("source"), false);
  });
});

test("runtime stack separates backend, protocol and storage panels without losing navigation", { timeout: 75_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    await openRuntime(page, "stack");
    const stack = page.locator('[data-workbench-runtime-stack="panel"]');
    assert.equal(await stack.locator("section").count(), 1);
    await stack.locator('[data-workbench-runtime-stack-page="protocols"]').click();
    assert.equal(await stack.locator("section").count(), 1);
    await stack.locator('[data-workbench-runtime-stack-page="storage"]').click();
    await stack.locator(".runtime-overview-card").waitFor();
    await page.locator('[data-workbench-runtime-menu="toggle"]').click();
    await page.locator('[data-workbench-runtime-tab="security"]').click();
    assert.equal(await page.locator(".runtime-page-menu").getAttribute("open"), null);
    await page.locator('[data-workbench-runtime-menu="toggle"]').click();
    await page.locator('[data-workbench-runtime-tab="stack"]').click();
    assert.equal(await stack.locator('[data-workbench-runtime-stack-page="storage"]').getAttribute("aria-pressed"), "true");
    await stack.locator('[data-workbench-runtime-stack-page="storage"]').press("Home");
    assert.equal(await stack.locator('[data-workbench-runtime-stack-page="backend"]').getAttribute("aria-pressed"), "true");
  });
});

for (const viewport of [{ width: 1440, height: 1000 }, { width: 1024, height: 700 }, { width: 390, height: 844 }]) {
  test(`compact material and audit panels fit at ${viewport.width}px`, { timeout: 90_000 }, async () => {
    await usingWorkbench(async (page) => {
      await page.setViewportSize(viewport);
      await mockAudit(page);
      await openMaterials(page, 25);
      for (const target of ["materials", "material-edit", "material-add", "audit"]) {
        if (target === "material-edit") await page.locator('[data-workbench-material-id="sample-0"]').click();
        if (target === "material-add") await materialPage(page, "add").click();
        if (target === "audit") { await openRuntime(page, "audit"); await auditPage(page, "filters").click(); }
        const root = page.locator(target === "audit" ? '[data-workbench-audit="panel"]' : '[data-workbench-materials="panel"]');
        const bounds = await root.evaluate((element) => {
          const rect = element.getBoundingClientRect();
          return { width: rect.width, right: rect.right, client: element.clientWidth, scroll: element.scrollWidth };
        });
        assert.ok(bounds.width > 100 && bounds.right <= viewport.width + 2, JSON.stringify(bounds));
        assert.ok(bounds.scroll <= bounds.client + 2, JSON.stringify(bounds));
        const overflowing = await root.locator("input, select, button").evaluateAll((elements) => elements.filter((element) => {
          const parent = element.closest("section").getBoundingClientRect();
          const rect = element.getBoundingClientRect();
          return rect.width > 0 && (rect.left < parent.left - 2 || rect.right > parent.right + 2);
        }).map((element) => element.outerHTML));
        assert.deepEqual(overflowing, [], `${target} controls stay inside their panel`);
        await page.screenshot({ path: `${ROOT}/tmp/compact-panels-${target}-${viewport.width}.png` });
      }
    });
  });
}
