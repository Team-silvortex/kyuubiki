import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { ensureInstallerLanguagePack, installerShellCopyFor } from "../ui/installer-shell-copy.js";
import { loadInstallerShellTranslation } from "../ui/installer-shell-translations.js";

const root = new URL("../../../", import.meta.url);
const readJson = (path) => JSON.parse(readFileSync(new URL(path, root), "utf8"));
const locales = readJson("config/localization/mainstream-language-pack-locales.json").locales;

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
