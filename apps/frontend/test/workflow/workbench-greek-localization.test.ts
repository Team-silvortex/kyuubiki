import test from "node:test";
import assert from "node:assert/strict";
import { resolveWorkbenchBaseCopy } from "../../src/components/workbench/workbench-copy.ts";
import {
  loadBuiltinWorkbenchLanguagePackForLanguage,
  resolveInstalledWorkbenchLanguagePack,
  WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES,
} from "../../src/components/workbench/workbench-language-pack-catalog.ts";
import { getWorkbenchScriptPanelCopy } from "../../src/components/workbench/workbench-script-panel-copy.ts";
import { getWorkbenchScriptDslCopy } from "../../src/components/workbench/workbench-script-dsl-copy.ts";
import {
  getWorkbenchMaterialLibraryCopy,
  getWorkbenchRuntimeAuditCopy,
  getWorkbenchRuntimeAuditEmptyLabel,
  getWorkbenchScriptInspectCopy,
} from "../../src/components/workbench/workbench-extended-language-copy.ts";

function flatten(value: unknown, prefix = ""): Record<string, string> {
  if (typeof value === "string") return { [prefix]: value };
  if (!value || typeof value !== "object") return {};
  return Object.assign({}, ...Object.entries(value).map(([key, child]) =>
    flatten(child, prefix ? `${prefix}.${key}` : key)));
}

const foreignCopy = /Egenskaper|Verktyg|Träd|Objektträd|Använd objektträdet|Exportera data|manuell studie|Orkestrerad GUI/u;
const placeholders = (value: string) => [...value.matchAll(/\{[a-zA-Z][a-zA-Z0-9_]*\}/g)].map(([token]) => token).sort();

test("Greek catalog has Greek copy throughout its source fragments, not foreign-language filler", async () => {
  const pack = await loadBuiltinWorkbenchLanguagePackForLanguage("el");
  assert.ok(pack);
  const entries = flatten(pack.overrides);
  assert.ok(Object.keys(entries).length > 700, "exercise the full merged pack, not just navigation");
  for (const [key, value] of Object.entries(entries)) {
    assert.doesNotMatch(value, foreignCopy, key);
    if (key === "jumpQuarter" || key === "jumpThreeQuarter") continue;
    assert.match(value, /\p{Script=Greek}/u, `${key}: ${value}`);
  }
  assert.equal(entries["tabs.tools"], "Εργαλεία");
  assert.equal(entries.properties, "Ιδιότητες");
  assert.equal(entries.solverAgent, "Agent επίλυσης Rust");
});

test("Greek translations retain interpolation parameters and shortcut array shape", async () => {
  const pack = await loadBuiltinWorkbenchLanguagePackForLanguage("el");
  assert.ok(pack);
  const source = flatten(resolveWorkbenchBaseCopy("en"));
  const translated = flatten(pack.overrides);
  for (const [key, value] of Object.entries(translated)) {
    if (source[key] !== undefined) assert.deepEqual(placeholders(value), placeholders(source[key]), key);
  }
  assert.equal((pack.overrides.shortcutLegendRows as string[]).length,
    resolveWorkbenchBaseCopy("en").shortcutLegendRows.length);
});

test("Greek PWDT has every English panel key translated, including nested categories", () => {
  const english = flatten(getWorkbenchScriptPanelCopy("en"));
  const greek = flatten(getWorkbenchScriptPanelCopy("el"));
  assert.deepEqual(Object.keys(greek).sort(), Object.keys(english).sort());
  for (const [key, value] of Object.entries(greek)) {
    if (key === "title") continue;
    assert.match(value, /\p{Script=Greek}/u, key);
    assert.notEqual(value, english[key], key);
    assert.deepEqual(placeholders(value), placeholders(english[key]), key);
  }
  assert.equal(getWorkbenchScriptPanelCopy(" EL_gr "), getWorkbenchScriptPanelCopy("el"));
  assert.equal(getWorkbenchScriptPanelCopy("unknown"), getWorkbenchScriptPanelCopy("en"));
  for (const copy of [getWorkbenchScriptDslCopy("el"), getWorkbenchMaterialLibraryCopy("el"), getWorkbenchScriptInspectCopy("el")]) {
    for (const [key, value] of Object.entries(copy)) assert.match(value, /\p{Script=Greek}/u, key);
  }
});

test("same-app-version cached official Greek packs refresh without modifying user imports", async () => {
  const catalog = await loadBuiltinWorkbenchLanguagePackForLanguage("el");
  assert.ok(catalog);
  const old = { ...catalog, updatedAt: "2026-09-19T00:00:00.000Z", overrides: { properties: "Egenskaper" } };
  assert.equal(resolveInstalledWorkbenchLanguagePack(old, catalog), catalog);
  assert.equal(old.overrides.properties, "Egenskaper", "stored data must not be mutated");
  const imported = { ...old, source: "imported" as const };
  assert.equal(resolveInstalledWorkbenchLanguagePack(imported, catalog), imported);
  const future = { ...old, updatedAt: "2099-01-01T00:00:00.000Z" };
  assert.equal(resolveInstalledWorkbenchLanguagePack(future, catalog), future);
});

test("no audit results is a neutral empty state in all supported locales, never a failure", () => {
  for (const language of ["en", "zh", "ja", "es", ...WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES.map((entry) => entry.language)]) {
    const empty = getWorkbenchRuntimeAuditEmptyLabel(language);
    assert.ok(empty.trim(), language);
    assert.notEqual(empty, getWorkbenchRuntimeAuditCopy(language).failed, language);
    if (language !== "en") assert.notEqual(empty, getWorkbenchRuntimeAuditEmptyLabel("en"), language);
  }
  assert.equal(getWorkbenchRuntimeAuditEmptyLabel("unknown"), getWorkbenchRuntimeAuditEmptyLabel("en"));
});
