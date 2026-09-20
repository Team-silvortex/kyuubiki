import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const packsKey = "kyuubiki-workbench-language-packs";
const scriptPage = (page, name) => page.locator(`[data-workbench-pwdt-content="${name}"]`);
const foreignCopy = /Egenskaper|Verktyg|Objektträd|Exportera data|manuell studie|Orkestrerad GUI/u;

async function openPwdt(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
  await page.locator('[data-workbench-system-surface-tab="settings"]').click();
  await page.locator('[data-workbench-system-settings-page="scripts"]').click();
  await page.locator('[data-workbench-pwdt="workspace"]').waitFor();
}

async function assertGreekModel(page) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
  await page.waitForFunction(() => document.querySelector('[data-workbench-model-tab="tools"]')?.textContent === "Εργαλεία");
  assert.equal(await page.locator('.panel-tab[data-workbench-model-tab="tree"]').innerText(), "Δέντρο");
  assert.doesNotMatch(await page.locator("body").innerText(), foreignCopy);
}

test("Greek Workbench replaces stale official copy on startup and reload while keeping stored packs intact", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page) => {
    const old = {
      schema_version: "kyuubiki.language-pack/v1", id: "workbench-el-core-2.0", language: "el",
      targetSurface: "workbench", name: "Greek Workbench Core", version: "3.2.0",
      versionLine: "daji 3.x", targetAppVersion: "3.2.0", source: "downloaded",
      updatedAt: "2026-09-19T00:00:00.000Z",
      overrides: { properties: "Egenskaper", tabs: { tools: "Verktyg", tree: "Träd" } },
    };
    await page.addInitScript(({ old, packsKey }) => {
      if (window.localStorage.getItem(packsKey) !== null) return;
      window.localStorage.setItem(packsKey, JSON.stringify([old]));
      window.localStorage.setItem("kyuubiki-workbench-settings", JSON.stringify({ language: "el", theme: "graphite" }));
    }, { old, packsKey });
    await openWorkbench(page);
    await assertGreekModel(page);
    await openPwdt(page);
    await scriptPage(page, "script").getByRole("button", { name: "Φόρτωση περιβάλλοντος εκτέλεσης", exact: true }).waitFor();
    await scriptPage(page, "script").getByRole("button", { name: "Εκτέλεση σκριπτ", exact: true }).waitFor();
    assert.equal(await scriptPage(page, "script").getByRole("button", { name: "Run script", exact: true }).count(), 0);
    await page.locator('[data-workbench-pwdt-page="dsl"]').click();
    await scriptPage(page, "dsl").getByRole("button", { name: "Μεταγλώττιση σε σκριπτ", exact: true }).waitFor();
    await page.locator('[data-workbench-pwdt-page="record"]').click();
    await scriptPage(page, "record").getByRole("button", { name: "Έναρξη καταγραφής", exact: true }).waitFor();
    await page.route("**/api/v1/**security-events*", (route) => route.fulfill({ json: { events: [] } }));
    await page.locator('[data-workbench-system-surface-tab="runtime"]').click();
    await page.locator('[data-workbench-runtime-tab="audit"]').click();
    await page.getByText("Δεν υπάρχουν συμβάντα ασφαλείας που να αντιστοιχούν στα τρέχοντα φίλτρα.", { exact: true }).waitFor();
    assert.deepEqual(await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key)), packsKey), [old]);
    await page.reload({ waitUntil: "networkidle" });
    await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
    await assertGreekModel(page);
    assert.deepEqual(await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key)), packsKey), [old]);
  });
});

test("switching between Swedish, English and Greek never retains another language's panel copy", { timeout: 120_000 }, async () => {
  await usingWorkbench(async (page) => {
    await openWorkbench(page);
    for (const [language, tools] of [["sv", "Verktyg"], ["el", "Εργαλεία"], ["en", "Tools"], ["el", "Εργαλεία"]]) {
      await invoke(page, "settings/patch", { language });
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
      await page.waitForFunction((expected) => document.querySelector('[data-workbench-model-tab="tools"]')?.textContent === expected, tools);
      if (language === "el") await assertGreekModel(page);
      if (language === "sv") continue;
      await openPwdt(page);
      const label = language === "el" ? "Εκτέλεση σκριπτ" : "Run script";
      await scriptPage(page, "script").getByRole("button", { name: label, exact: true }).waitFor();
      assert.doesNotMatch(await page.locator('[data-workbench-pwdt="workspace"]').innerText(), foreignCopy);
    }
  });
});
