import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { assertLanguageChange, assertNoPageErrors } from "./desktop-shell-regression.shared.mjs";
import { loadInstallerShellTranslation } from "../../apps/installer-gui/ui/installer-shell-translations.js";
import { installerShellCopyFor } from "../../apps/installer-gui/ui/installer-shell-copy.js";

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
  await page.evaluate(() => {
    const invoke = window.__TAURI__.core.invoke;
    window.__TAURI__.core.invoke = async (command, payload) => {
      const result = await invoke(command, payload);
      return command === "read_env_file" ? {
        deployment_mode: "distributed",
        agent_discovery: "registry",
        storage_backend: "postgres",
        orchestra_instance_id: "language-fixture",
        database_url_configured: true,
        kyuubiki_api_token_configured: true,
        kyuubiki_cluster_api_token_configured: false,
        kyuubiki_direct_mesh_token_configured: false,
      } : result;
    };
  });
  await page.locator('.sidebar-tab[data-tab="setup"]').click();
  await page.locator('[data-action="reload-env"]').click();
  await page.waitForFunction(() => window.__kyuubikiInstallerLastCompletedAction === "reload-env");
  await page.locator("#sqlite-path").fill("./tmp/research draft/材料.sqlite3");
  await page.locator("#agent-manifest-path").fill("./deploy/research-draft.json");
  await page.locator("#cluster-api-token").fill("localization-test-only");
  await page.locator("#protect-reads").selectOption("true");
  await page.locator("#cluster-require-fingerprint").selectOption("true");
  await page.locator("#direct-mesh-enabled").selectOption("false");
  await page.locator('.sidebar-tab[data-tab="wizard"]').click();
  await page.locator("#build-mode").selectOption("release-bundle");
  await page.locator('.sidebar-tab[data-tab="release"]').click();
  await page.locator("#release-platform").selectOption("linux");
  await page.locator("#release-target").fill("./dist/research draft");
  await page.locator("#release-build-mode").selectOption("release-no-bundle");
  await page.locator('.sidebar-tab[data-tab="services"]').click();
  await page.locator("#log-service").selectOption("frontend");
  await page.locator('[data-action="load-log"]').click();
  await page.waitForFunction(() => window.__kyuubikiInstallerLastCompletedAction === "load-log");
  await page.locator('.sidebar-tab[data-tab="setup"]').click();
  const form = await installerSetupSnapshot(page);
  const completion = await page.locator("#completion-message").textContent();
  const output = await page.locator("#output").textContent();
  const guide = await page.locator("#completion-guide").textContent();
  const runtimeLog = await page.locator("#runtime-log").textContent();
  for (const language of languages) {
    const copy = await loadInstallerShellTranslation(language);
    await assertLanguageChange(page, language);
    await page.waitForFunction((expected) =>
      document.querySelector('[data-action="service-status"]')?.textContent === expected, copy.actions.serviceStatus);
    assert.deepEqual(await page.locator(".sidebar-tab > span:last-child").allTextContents(), copy.tabs, language);
    assert.equal(await page.locator('[data-action="bootstrap"]').first().textContent(), copy.actions.bootstrap, language);
    assert.equal(await page.locator("#brand-installer-description").textContent(), copy.description, language);
    await assertInstallerSetupCopy(page, copy.setup, language);
    assert.equal(await page.locator('[data-panel="release"] #release-platform-label').textContent(), copy.platform, language);
    assert.deepEqual(await installerSetupSnapshot(page), form, "language changes preserve form values and protocol enums");
    assert.equal(await page.locator("#completion-message").textContent(), completion, "language changes retain diagnostic results");
    assert.equal(await page.locator("#completion-guide").textContent(), guide, "language changes retain completion details");
    assert.equal(await page.locator("#output").textContent(), output, "language changes retain command output");
    assert.equal(await page.locator("#runtime-log").textContent(), runtimeLog, "language changes retain runtime logs");
    assert.equal(new Set(requested).size, languages.indexOf(language) + 1, "locale assets load only as selected");
  }
  for (const language of ["es", "ja", "en", "zh"]) {
    const copy = installerShellCopyFor(language);
    await assertLanguageChange(page, language);
    await page.waitForFunction((expected) =>
      document.querySelector('[data-installer-setup-copy="chooseProfile"]')?.textContent === expected,
    copy.setup.chooseProfile);
    await assertInstallerSetupCopy(page, copy.setup, language);
    assert.equal(await page.locator('[data-panel="release"] #release-platform-label').textContent(), copy.platform, language);
    assert.deepEqual(await installerSetupSnapshot(page), form, language);
    assert.equal(await page.locator("#completion-message").textContent(), completion);
    assert.equal(await page.locator("#output").textContent(), output);
    assert.equal(await page.locator("#runtime-log").textContent(), runtimeLog);
  }
  assert.equal(new Set(requested).size, languages.length, "built-ins require no extra locale module");
  await page.locator('[data-action="reload-env"]').click();
  await page.waitForFunction(() => document.querySelector("#cluster-api-token")?.value === "");
  await assertInstallerSetupCopy(page, installerShellCopyFor("zh").setup, "zh after environment reload");
  await assertInstallerGuidanceLayout(page);
  await assertNoPageErrors(page);
}

async function assertInstallerGuidanceLayout(page) {
  for (const viewport of [{ width: 1180, height: 920 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport);
    for (const language of ["zh", "de", "ar", "ta"]) {
      const copy = language === "zh" ? installerShellCopyFor(language)
        : await loadInstallerShellTranslation(language);
      await assertLanguageChange(page, language);
      await page.waitForFunction((expected) =>
        document.querySelector('[data-installer-setup-copy="welcome"]')?.textContent === expected, copy.setup.welcome);
      for (const panelName of ["setup", "wizard", "release", "services"]) {
        await page.locator(`.sidebar-tab[data-tab="${panelName}"]`).click();
        const overflow = await page.locator(`[data-panel="${panelName}"]`).evaluate((panel) => {
          const boundary = panel.getBoundingClientRect();
          return Array.from(panel.querySelectorAll(".mode-card, .service-card, .completion-card, .wizard-step, [data-installer-setup-copy]"))
            .filter((node) => node.getClientRects().length && node.clientWidth > 0)
            .filter((node) => {
              const rect = node.getBoundingClientRect();
              return node.scrollWidth > node.clientWidth + 2 || rect.right > boundary.right + 2
                || rect.left < boundary.left - 2;
            })
            .map((node) => node.getAttribute("data-installer-setup-copy") || node.className);
        });
        assert.deepEqual(overflow, [], `${language}:${panelName}:${viewport.width}`);
      }
    }
  }
}

async function installerSetupSnapshot(page) {
  return page.locator([
    '[data-panel="setup"] .form-shell input',
    '[data-panel="setup"] .form-shell select',
    "#build-mode", "#release-build-mode", "#release-platform", "#release-target",
    "#log-service", "#log-autorefresh",
  ].join(", ")).evaluateAll((nodes) =>
    nodes.map((node) => ({
      id: node.id,
      value: node.value,
      type: node.type,
      checked: node.checked,
      configured: node.dataset.configured,
      options: node.options ? Array.from(node.options, (option) => option.value) : null,
    })));
}

async function assertInstallerSetupCopy(page, copy, language) {
  const problems = await page.evaluate((expected) => {
    const mismatches = [];
    for (const [attribute, property] of [
      ["data-installer-setup-copy", "textContent"],
      ["data-installer-setup-placeholder", "placeholder"],
    ]) {
      document.querySelectorAll(`[${attribute}]`).forEach((node) => {
        const key = node.getAttribute(attribute);
        const value = property === "placeholder" && node.dataset.configured === "true"
          ? expected.keepConfigured : expected[key];
        if (typeof value !== "string" || node[property] !== value) mismatches.push(key);
      });
    }
    document.querySelectorAll('[data-panel="setup"] .form-shell label.field > span, [data-panel="setup"] .form-shell option').forEach((node) => {
      if (!node.hasAttribute("data-installer-setup-copy")) mismatches.push("unbound: " + node.textContent);
    });
    document.querySelectorAll([
      ".mode-card p", ".mode-card h3", ".mode-card li",
      ".completion-card:nth-child(-n+2) h3",
      ".completion-card:nth-child(-n+2) p:not(#completion-guide)",
      ".completion-list li", "#release-build-mode option",
      ".service-card h3", ".service-card p", ".service-card button",
    ].join(", ")).forEach((node) => {
      if (!node.hasAttribute("data-installer-setup-copy")) mismatches.push("unbound guidance: " + node.textContent);
    });
    const database = document.getElementById("database-url");
    if (database.placeholder !== expected.keepConfigured) mismatches.push("configured database hint");
    return mismatches;
  }, copy);
  assert.deepEqual(problems, [], language);
}
