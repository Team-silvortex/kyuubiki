import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { assertLanguageChange, assertNoPageErrors } from "./desktop-shell-regression.shared.mjs";
import { loadInstallerShellTranslation } from "../../apps/installer-gui/ui/installer-shell-translations.js";

const root = new URL("../../", import.meta.url);
const readJson = (path) => JSON.parse(readFileSync(new URL(path, root), "utf8"));
const languages = readJson("config/localization/mainstream-language-pack-locales.json").locales.map((entry) => entry.language);

export async function assertHubBundleLanguageMatrix(page) {
  await page.waitForSelector('html[data-hub-ready="true"]');
  await page.locator("#projects-tab-bundles").click();
  const draft = "/tmp/Research language check.kyuubiki";
  await page.locator("#project-bundle-path").fill(draft);
  for (const language of languages) {
    const copy = readJson(`language-packs/hub/${language.toLowerCase()}.json`).overrides.bundles;
    await assertLanguageChange(page, language);
    await page.waitForFunction((expected) =>
      document.querySelector("#bundles-action-validate")?.textContent === expected, copy.validate);
    assert.equal(await page.locator("#bundles-action-inspect").textContent(), copy.inspect, language);
    assert.equal(await page.locator("#bundles-action-normalize").textContent(), copy.normalize, language);
    assert.equal(await page.locator("#bundles-action-diff").textContent(), copy.diff, language);
    assert.equal(await page.locator("#projects-tab-bundles").textContent(), copy.introTitle, language);
    assert.equal(await page.locator("#project-bundle-path").inputValue(), draft, language);
  }
  await assertLanguageChange(page, "zh");
  await page.waitForFunction(() =>
    document.querySelector(".hub-mainline-track")?.getAttribute("aria-label") === "主线工作流");
  assert.equal(await page.locator("#bundles-action-normalize").textContent(), "规范化项目包");
  await assertNoPageErrors(page);
}

export async function assertInstallerLanguageMatrix(page) {
  const requested = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.pathname.includes("/installer-shell-locales/")) requested.push(url.pathname.split("/").at(-1));
  });
  await page.waitForFunction(() =>
    /Unified regression gate is warn/.test(document.querySelector("#completion-message")?.textContent || ""));
  const completion = await page.locator("#completion-message").textContent();
  const output = await page.locator("#output").textContent();
  for (const language of languages) {
    const copy = await loadInstallerShellTranslation(language);
    await assertLanguageChange(page, language);
    await page.waitForFunction((expected) =>
      document.querySelector('[data-action="service-status"]')?.textContent === expected, copy.actions.serviceStatus);
    assert.deepEqual(await page.locator(".sidebar-tab > span:last-child").allTextContents(), copy.tabs, language);
    assert.equal(await page.locator('[data-action="bootstrap"]').first().textContent(), copy.actions.bootstrap, language);
    assert.equal(await page.locator("#brand-installer-description").textContent(), copy.description, language);
    assert.equal(await page.locator("#completion-message").textContent(), completion, "language changes retain diagnostic results");
    assert.equal(await page.locator("#output").textContent(), output, "language changes retain command output");
    assert.equal(new Set(requested).size, languages.indexOf(language) + 1, "locale assets load only as selected");
  }
  await assertLanguageChange(page, "es");
  await page.waitForFunction(() =>
    document.querySelector('.sidebar-tab[data-tab="updates"] span:last-child')?.textContent === "Actualizaciones");
  assert.equal(new Set(requested).size, languages.length, "built-in Spanish requires no extra locale module");
  await assertNoPageErrors(page);
}
