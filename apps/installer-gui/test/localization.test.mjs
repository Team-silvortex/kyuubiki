import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { ensureInstallerLanguagePack, installerShellCopyFor } from "../ui/installer-shell-copy.js";
import { loadInstallerShellTranslation } from "../ui/installer-shell-translations.js";
import { builtinInstallerSetupCopy, installerSensitivePlaceholder, renderInstallerSetupCopy } from "../ui/installer-setup-copy.js";

const root = new URL("../../../", import.meta.url);
const readJson = (path) => JSON.parse(readFileSync(new URL(path, root), "utf8"));
const locales = readJson("config/localization/mainstream-language-pack-locales.json").locales;
const dynamicSetupKeys = [
  "keepConfigured", "localStarted", "localRestarted", "cloudStarted", "cloudRestarted",
  "distributedStarted", "allStopped", "localSelected", "cloudSelected", "distributedSelected",
  "logEmpty", "logLoaded", "logAttached", "logPolling",
];

function leaves(value, prefix = "") {
  return Object.entries(value).flatMap(([key, item]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof item === "string" ? [[path, item]] : leaves(item, path);
  });
}

test("all 30 Installer shell translations load locally with complete shell keys", async (t) => {
  t.mock.method(globalThis, "fetch", async (path) => {
    assert.match(path, /^\.\/language-packs\/hub\/[a-z-]+\.json$/);
    return new Response(JSON.stringify(readJson(path.slice(2))), { status: 200 });
  });
  const base = installerShellCopyFor("en");
  const before = JSON.stringify(base);
  const keys = leaves(base).map(([key]) => key).sort();
  for (const { language } of locales) {
    const result = await ensureInstallerLanguagePack(language);
    assert.equal(result.status, "loaded", language);
    const copy = installerShellCopyFor(language);
    const local = await loadInstallerShellTranslation(language);
    assert.deepEqual(Object.keys(local.setup).sort(), Object.keys(base.setup).sort(), language);
    assert.deepEqual(copy.setup, local.setup, "setup uses its own translation, not English fallback");
    assert.deepEqual(leaves(copy).map(([key]) => key).sort(), keys, language);
    for (const [key, text] of leaves(copy)) {
      assert.ok(text.trim(), `${language}:${key}`);
      assert.doesNotMatch(text, /[\u200b\u2060\ufeff]/u, `${language}:${key}`);
    }
    const hub = readJson(`language-packs/hub/${language.toLowerCase()}.json`);
    assert.notEqual(copy.actions.serviceStatus, hub.overrides.shell.actionStatus, language);
    assert.notEqual(copy.actions.bootstrap, hub.overrides.shell.startLocal, language);
    assert.equal(result.message, copy.ready);
  }
  assert.equal(JSON.stringify(base), before, "localization never mutates the English fallback");
  assert.equal(installerShellCopyFor("de").actions.serviceStatus, "Dienste prüfen");
  assert.equal(installerShellCopyFor("fr").actions.bootstrap, "Initialiser l’espace de travail");
  assert.equal(installerShellCopyFor("ko").tabs[3], "업데이트");
  assert.equal(installerShellCopyFor("es").tabs[3], "Actualizaciones");
  assert.equal(await loadInstallerShellTranslation("../en"), null);
});

test("all four built-in Installer setup dictionaries are complete", () => {
  const keys = Object.keys(builtinInstallerSetupCopy.en).sort();
  assert.equal(keys.length, 125);
  for (const [language, copy] of Object.entries(builtinInstallerSetupCopy)) {
    assert.deepEqual(Object.keys(copy).sort(), keys, language);
    for (const [key, text] of Object.entries(copy)) {
      assert.ok(text.trim(), `${language}:${key}`);
      assert.doesNotMatch(text, /[\u200b\u2060\ufeff]/u, `${language}:${key}`);
    }
  }
  assert.equal(builtinInstallerSetupCopy.zh.deploymentMode, "部署模式");
  assert.equal(builtinInstallerSetupCopy.ja.prepareWorkspace, "ワークスペースを準備");
  assert.equal(builtinInstallerSetupCopy.es.protectReads, "Proteger lecturas");
});

test("Installer setup markup binds every static key, with dynamic results handled by controllers", () => {
  const html = readFileSync(new URL("../ui/index.html", import.meta.url), "utf8");
  const boundKeys = new Set([...html.matchAll(/data-installer-setup-(?:copy|placeholder|ready-copy)="([^"]+)"/gu)]
    .map((match) => match[1]));
  const expected = Object.keys(builtinInstallerSetupCopy.en).filter((key) => !dynamicSetupKeys.includes(key));
  assert.deepEqual([...boundKeys].sort(), expected.sort());
});

test("Installer service results keep log placeholders and technical terms in all 34 languages", async () => {
  for (const language of ["en", "zh", "ja", "es", ...locales.map((locale) => locale.language)]) {
    const copy = builtinInstallerSetupCopy[language] || (await loadInstallerShellTranslation(language)).setup;
    for (const key of ["logEmpty", "logLoaded", "logAttached", "logPolling"]) {
      assert.deepEqual(copy[key].match(/\{[^{}]+\}/gu), ["{service}"], `${language}:${key}`);
    }
    assert.match(copy.localServiceHelp, /SQLite/u, language);
    assert.match(copy.cloudServiceHelp, /PostgreSQL/u, language);
    assert.match(copy.stopServiceHelp, /Rust/u, language);
    for (const key of dynamicSetupKeys) {
      if (language !== "en") assert.notEqual(copy[key], builtinInstallerSetupCopy.en[key], `${language}:${key}`);
    }
  }
});

test("configured credentials keep an explicit localized preservation hint in all locales", async () => {
  for (const language of ["en", "zh", "ja", "es", ...locales.map((locale) => locale.language)]) {
    const copy = builtinInstallerSetupCopy[language] || (await loadInstallerShellTranslation(language)).setup;
    for (const id of ["database-url", "api-token", "cluster-api-token", "direct-mesh-token"]) {
      assert.equal(installerSensitivePlaceholder(id, true, copy), copy.keepConfigured, language);
    }
    assert.equal(installerSensitivePlaceholder("api-token", false, copy), copy.apiTokenHint);
    assert.equal(installerSensitivePlaceholder("cluster-api-token", false, copy), copy.clusterTokenHint);
    assert.equal(installerSensitivePlaceholder("direct-mesh-token", false, copy), copy.directMeshTokenHint);
    assert.equal(installerSensitivePlaceholder("database-url", false, copy),
      "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev");
    assert.match(copy.environmentHelp, /\.env\.local/u, language);
    assert.match(copy.initEnv, /\.env\.local/u, language);
  }
});

test("Installer guidance retains technical identifiers across all 34 languages", async () => {
  for (const language of ["en", "zh", "ja", "es", ...locales.map((locale) => locale.language)]) {
    const copy = builtinInstallerSetupCopy[language] || (await loadInstallerShellTranslation(language)).setup;
    assert.match(copy.localFile, /\.sqlite3/u, language);
    assert.match(copy.cloudDatabaseUrl, /DATABASE_URL/u, language);
    assert.match(copy.distributedHelp, /Rust/u, language);
    assert.match(copy.distributedSsh, /SSH/u, language);
    for (const key of ["welcomeHelp", "afterSetupHelp", "localHelp", "cloudHelp", "distributedHelp"]) {
      if (language !== "en") assert.notEqual(copy[key], builtinInstallerSetupCopy.en[key], `${language}:${key}`);
    }
  }
  assert.equal((await loadInstallerShellTranslation("de")).setup.fastPath, "Schnellstart");
  assert.equal((await loadInstallerShellTranslation("fr")).setup.logFrontend, "Interface");
});

test("Installer ready guidance translates before results and never replaces a result", () => {
  const welcome = { textContent: "Welcome", getAttribute: () => "welcome" };
  const guide = { textContent: "initial", getAttribute: () => "completionGuide" };
  const root = {
    querySelectorAll: (selector) => ({
      "[data-installer-setup-copy]": [welcome],
      "[data-installer-setup-placeholder]": [],
      "[data-installer-setup-ready-copy]": [guide],
    })[selector],
    querySelector: () => null,
  };
  renderInstallerSetupCopy(root, builtinInstallerSetupCopy.zh);
  assert.equal(welcome.textContent, "欢迎使用");
  assert.equal(guide.textContent, builtinInstallerSetupCopy.zh.completionGuide);
  guide.textContent = "Diagnostic result: retry required";
  renderInstallerSetupCopy(root, builtinInstallerSetupCopy.es, { hasCompletionResult: true });
  assert.equal(welcome.textContent, "Bienvenido");
  assert.equal(guide.textContent, "Diagnostic result: retry required");
});
