import assert from "node:assert/strict";
import { test } from "node:test";
import { usingWorkbench, invoke, openWorkbench, installProjectWorkbenchTestHooks } from "./workbench-ui-project-fixture.shared.mjs";

installProjectWorkbenchTestHooks();
const packsKey = "kyuubiki-workbench-language-packs";
const localeLabels = [
  ["ar", "الأدوات", "تصفح"], ["bn", "সরঞ্জাম", "ব্রাউজ করুন"],
  ["cs", "Nástroje", "Procházet"], ["da", "Værktøjer", "Gennemse"],
  ["fa", "ابزارها", "مرور"], ["he", "כלים", "עיון"],
  ["hi", "उपकरण", "ब्राउज़ करें"],
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
      updatedAt: "2026-09-19T02:00:00.000Z",
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

test("Arabic and Persian base repairs survive selector changes, official-cache refresh, assistant toggles, and reload", { timeout: 90_000 }, async () => {
  const copies = [
    { language: "ar", open: "فتح المساعد", close: "إغلاق المساعد", title: "اسأل مساعد Workbench" },
    { language: "fa", open: "باز کردن دستیار", close: "بستن دستیار", title: "از دستیار Workbench بپرسید" },
  ];
  await usingWorkbench(async (page, library) => {
    const oldPacks = copies.map(({ language }) => ({
      schema_version: "kyuubiki.language-pack/v1", id: `workbench-${language}-core-2.0`, language,
      targetSurface: "workbench", name: `${language} Workbench Core`, version: "3.2.0",
      versionLine: "daji 3.x", targetAppVersion: "3.2.0", source: "downloaded",
      updatedAt: "2026-09-20T01:00:00.000Z",
      overrides: { assistantOpen: "Open assistant\u200B", assistantClose: "\uFEDE\uFED4\uFED7" },
    }));
    await page.addInitScript(({ oldPacks, packsKey }) => {
      if (window.localStorage.getItem(packsKey) !== null) return;
      window.localStorage.setItem(packsKey, JSON.stringify(oldPacks));
      window.localStorage.setItem("kyuubiki-workbench-settings", JSON.stringify({ language: "ar", theme: "graphite" }));
    }, { oldPacks, packsKey });
    await openWorkbench(page);
    for (const copy of copies) {
      await invoke(page, "settings/patch", { language: "en" });
      await page.evaluate(() => window.__kyuubikiPwdt.openSidebar("system"));
      await invoke(page, "nav/setTabs", { systemPanelTab: "config" });
      await page.locator("select").filter({ has: page.locator('option[value="ar"]') }).selectOption(copy.language);
      for (const reload of [false, true]) {
        if (reload) {
          await page.reload({ waitUntil: "networkidle" });
          await page.waitForFunction(() => Boolean(window.__kyuubikiPwdt));
        }
        await page.waitForFunction((label) => document.querySelector(".assistant-float-launcher")?.getAttribute("aria-label") === label, copy.open);
        await page.locator(".assistant-float-launcher").click();
        const dialog = page.locator(".assistant-float-panel");
        await dialog.waitFor({ state: "visible" });
        assert.equal(await dialog.locator(".assistant-float-panel__headline strong").innerText(), copy.title);
        assert.doesNotMatch(await dialog.locator(".assistant-float-panel__header").innerText(), /[\uFB50-\uFDFF\uFE70-\uFEFF\u200B\u2060]/u);
        assert.equal(await page.locator(".assistant-float-launcher").getAttribute("aria-label"), copy.close);
        await dialog.getByRole("button", { name: copy.close, exact: true }).click();
        await dialog.waitFor({ state: "hidden" });
      }
    }
    assert.deepEqual(await page.evaluate((key) => JSON.parse(window.localStorage.getItem(key)), packsKey), oldPacks);
    assert.deepEqual(library.writes, [], "language and assistant controls must not save or alter the model");
    await invoke(page, "settings/patch", { language: "en" });
    await page.waitForFunction(() => document.querySelector(".assistant-float-launcher")?.getAttribute("aria-label") === "Open assistant");
  });
});
