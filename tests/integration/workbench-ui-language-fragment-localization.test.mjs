import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const packsKey = "kyuubiki-workbench-language-packs";
const localeLabels = [
  ["cs", "Nástroje", "Procházet"], ["da", "Værktøjer", "Gennemse"],
  ["fi", "Työkalut", "Selaa"], ["id", "Alat", "Jelajahi"],
  ["ms", "Alat", "Semak imbas"], ["no", "Verktøy", "Bla gjennom"],
  ["ro", "Instrumente", "Răsfoire"], ["sw", "Zana", "Vinjari"],
];
const swedishPhrases = /Lägg till|Använd objektträdet|Exportera data|Orkestrerad GUI|Verktyg|Objektträd/u;

async function assertModelLabels(page, tools, tree) {
  await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("model"));
  await page.waitForFunction(({ tools, tree }) =>
    document.querySelector('[data-workbench-model-tab="tools"]')?.textContent === tools &&
    document.querySelector('.panel-tab[data-workbench-model-tab="tree"]')?.textContent === tree,
  { tools, tree });
  assert.doesNotMatch(await page.locator("body").innerText(), swedishPhrases);
}

test("corrected extension locales replace stale official copy across startup, switching, and reload", { timeout: 180_000 }, async () => {
  await usingWorkbench(async (page) => {
    const oldPacks = localeLabels.map(([language]) => ({
      schema_version: "kyuubiki.language-pack/v1", id: `workbench-${language}-core-2.0`, language,
      targetSurface: "workbench", name: `${language} Workbench Core`, version: "3.2.0",
      versionLine: "daji 3.x", targetAppVersion: "3.2.0", source: "downloaded",
      updatedAt: "2026-09-19T01:00:00.000Z",
      overrides: { properties: "Egenskaper", tabs: { tools: "Verktyg", tree: "Träd" } },
    }));
    await page.addInitScript(({ oldPacks, packsKey }) => {
      if (window.localStorage.getItem(packsKey) !== null) return;
      window.localStorage.setItem(packsKey, JSON.stringify(oldPacks));
      window.localStorage.setItem("kyuubiki-workbench-settings", JSON.stringify({ language: "cs", theme: "graphite" }));
    }, { oldPacks, packsKey });
    await openWorkbench(page);
    await assertModelLabels(page, "Nástroje", "Procházet");
    for (const [language, tools, tree] of localeLabels) {
      await invoke(page, "settings/patch", { language: "sv" });
      await page.waitForFunction(() => document.querySelector('[data-workbench-model-tab="tools"]')?.textContent === "Verktyg");
      await invoke(page, "settings/patch", { language });
      await assertModelLabels(page, tools, tree);
      await page.reload({ waitUntil: "networkidle" });
      await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
      await assertModelLabels(page, tools, tree);
    }
    assert.deepEqual(await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key)), packsKey), oldPacks);
    await invoke(page, "settings/patch", { language: "en" });
    await assertModelLabels(page, "Tools", "Browse");
  });
});
