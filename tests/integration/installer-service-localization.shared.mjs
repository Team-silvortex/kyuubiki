import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { assertActionInvokes, assertLanguageChange, assertNoPageErrors } from "./desktop-shell-regression.shared.mjs";
import { loadInstallerShellTranslation } from "../../apps/installer-gui/ui/installer-shell-translations.js";
import { installerShellCopyFor } from "../../apps/installer-gui/ui/installer-shell-copy.js";

const languages = JSON.parse(readFileSync(new URL(
  "../../config/localization/mainstream-language-pack-locales.json", import.meta.url,
), "utf8")).locales.map(({ language }) => language);
const serviceActions = [
  ["service-start-local", "service_start", "local", "localStarted"],
  ["service-restart-local", "service_restart", "local", "localRestarted"],
  ["service-start-cloud", "service_start", "cloud", "cloudStarted"],
  ["service-restart-cloud", "service_restart", "cloud", "cloudRestarted"],
  ["service-start-distributed", "service_start", "distributed", "distributedStarted"],
  ["service-stop", "service_stop", undefined, "allStopped"],
];
const format = (copy, key) => copy[key].replaceAll("{service}", "frontend");

async function selectLanguage(page, language) {
  const copy = (await loadInstallerShellTranslation(language) || installerShellCopyFor(language)).setup;
  await assertLanguageChange(page, language);
  await page.waitForFunction((text) =>
    document.querySelector('[data-action="service-start-local"]')?.textContent === text, copy.startLocal);
  return copy;
}

async function assertServiceAction(page, action, nativeAction, mode, expected) {
  await assertActionInvokes(page, action, "guarded_mutation_action", nativeAction);
  assert.equal(await page.locator("#completion-message").textContent(), expected, action);
  const actual = await page.evaluate(() => [...window.__mockInvocations].reverse()
    .find(({ command }) => command === "guarded_mutation_action").payload.payload);
  assert.deepEqual(actual, { action: nativeAction, ...(mode ? { mode } : {}) }, action);
}

export async function assertInstallerServiceLanguageMatrix(page) {
  await page.waitForFunction(() =>
    /Unified regression gate is warn/.test(document.querySelector("#completion-message")?.textContent || ""));
  await page.evaluate(() => {
    const invoke = window.__TAURI__.core.invoke;
    window.__serviceLocaleFixture = { raw: "", failStream: false };
    window.__TAURI__.core.invoke = async (command, payload) => {
      const result = await invoke(command, payload);
      if (command === "start_log_stream" && window.__serviceLocaleFixture.failStream) {
        throw new Error("mock stream unavailable");
      }
      return command === "read_runtime_log"
        ? { service: payload.service, rendered: window.__serviceLocaleFixture.raw } : result;
    };
  });
  await page.locator('.sidebar-tab[data-tab="services"]').click();
  await page.locator("#log-service").selectOption("frontend");
  for (const language of [...languages, "es", "ja", "en", "zh"]) {
    const copy = await selectLanguage(page, language);
    for (const [action, nativeAction, mode, resultKey] of serviceActions) {
      await assertServiceAction(page, action, nativeAction, mode, copy[resultKey]);
    }
    await assertActionInvokes(page, "load-log", "read_runtime_log");
    assert.equal(await page.locator("#runtime-log").textContent(), format(copy, "logEmpty"), language);
    assert.ok((await page.locator("#output").textContent()).endsWith(format(copy, "logLoaded")), language);
    assert.equal(await page.locator("#log-service").inputValue(), "frontend", language);
    assert.match(await page.locator("#output").textContent(), /guarded mutation mock/u);
  }

  for (const language of ["zh", "de", "ar"]) {
    const copy = await selectLanguage(page, language);
    for (const mode of ["local", "cloud", "distributed"]) {
      await page.locator('.sidebar-tab[data-tab="setup"]').click();
      await page.locator(`button[data-action="use-${mode}-mode"]:visible`).first().click();
      await page.waitForFunction((expected) =>
        document.querySelector("#completion-message")?.textContent === expected, copy[`${mode}Selected`]);
      assert.equal(await page.locator("#deployment-mode").inputValue(), mode);
      await page.locator('.sidebar-tab[data-tab="wizard"]').click();
      await assertServiceAction(page, "wizard-start-active", "service_start", mode, copy[`${mode}Started`]);
    }
  }

  const copy = await selectLanguage(page, "zh");
  await page.locator('.sidebar-tab[data-tab="services"]').click();
  await page.evaluate(() => { window.__serviceLocaleFixture.failStream = true; });
  await page.locator("#log-autorefresh").check();
  await page.waitForFunction((expected) =>
    document.querySelector("#completion-message")?.textContent === expected, format(copy, "logPolling"));
  await assertActionInvokes(page, "load-log", "start_log_stream");
  assert.ok((await page.locator("#output").textContent()).endsWith(format(copy, "logPolling")));
  await page.locator("#log-autorefresh").uncheck();

  await page.evaluate(() => {
    window.__serviceLocaleFixture.failStream = false;
    window.__serviceLocaleFixture.raw = "[solver] <mesh> iteration=42 status=ready";
  });
  await page.locator("#log-autorefresh").check();
  await page.waitForFunction((expected) =>
    document.querySelector("#completion-message")?.textContent === expected, format(copy, "logAttached"));
  await assertActionInvokes(page, "load-log", "start_log_stream");
  assert.ok((await page.locator("#output").textContent()).endsWith(format(copy, "logAttached")));
  assert.equal(await page.locator("#runtime-log").textContent(), "[solver] <mesh> iteration=42 status=ready");
  assert.equal(await page.locator("#runtime-log mesh").count(), 0, "backend output is text, not markup");
  await page.locator("#log-autorefresh").uncheck();
  await assertNoPageErrors(page);
}
