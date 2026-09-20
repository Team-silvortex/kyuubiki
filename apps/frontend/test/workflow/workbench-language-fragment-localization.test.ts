import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolveWorkbenchBaseCopy } from "../../src/components/workbench/workbench-copy.ts";
import {
  loadBuiltinWorkbenchLanguagePackForLanguage,
  resolveInstalledWorkbenchLanguagePack,
} from "../../src/components/workbench/workbench-language-pack-catalog.ts";

const locales = {
  cs: ["Nástroje", "Vlastnosti", "Uzly"],
  da: ["Værktøjer", "Egenskaber", "Knuder"],
  fi: ["Työkalut", "Ominaisuudet", "Solmut"],
  id: ["Alat", "Properti", "Simpul"],
  ms: ["Alat", "Sifat", "Nod"],
  no: ["Verktøy", "Egenskaper", "Noder"],
  ro: ["Instrumente", "Proprietăți", "Noduri"],
  sw: ["Zana", "Sifa", "Nodi"],
};
const revision = "2026-09-19T02:00:00.000Z";
const foreignPhrases = /Lägg till|Ta bort|Fixera|Genomsnittlig|Förskjutnings|Värmeflöde|Länkläge|Uppdatera|Kör studie|Använd|Styvhetsmatrisen|Sparad|Skjuv|Återställ/u;
const placeholders = (s: string) => [...s.matchAll(/\{[a-zA-Z][a-zA-Z0-9_]*\}/g)].map(([v]) => v).sort();
const units = (s: string) => [...s.matchAll(/\((?:m|m²|GPa|Pa|N)\)/g)].map(([v]) => v).sort();

function flatten(value: unknown, prefix = ""): Record<string, string> {
  if (typeof value === "string") return { [prefix]: value };
  if (!value || typeof value !== "object") return {};
  return Object.assign({}, ...Object.entries(value).map(([key, child]) =>
    flatten(child, prefix ? `${prefix}.${key}` : key)));
}

function readPack(relativePath: string) {
  return JSON.parse(readFileSync(new URL(`../../../../language-packs/workbench/${relativePath}.json`, import.meta.url), "utf8"));
}

function fragments(language: string) {
  const parts = [1, 2, 3, 4].map((part) => readPack(`${language}/extended-0${part}`));
  return { parts, entries: Object.assign({}, ...parts.map((part) => flatten(part.overrides))) as Record<string, string> };
}

const swedish = fragments("sv").entries;
const english = flatten(resolveWorkbenchBaseCopy("en"));

for (const [language, [tools, properties, nodes]] of Object.entries(locales)) {
  test(`${language}: all four extension fragments retain the contract and reject Swedish filler`, async () => {
    const { parts, entries } = fragments(language);
    assert.equal(Object.keys(entries).length, 378);
    assert.deepEqual(Object.keys(entries).sort(), Object.keys(swedish).sort());
    for (const part of parts) {
      assert.equal(part.language, language);
      assert.equal(part.updatedAt, revision);
    }
    const catalog = await loadBuiltinWorkbenchLanguagePackForLanguage(language);
    assert.ok(catalog);
    const generated = flatten(catalog.overrides);
    assert.equal(readPack(language).updatedAt, revision);
    assert.equal(catalog.updatedAt, revision);
    for (const [key, text] of Object.entries(entries)) {
      assert.ok(text.trim(), key);
      assert.doesNotMatch(text, foreignPhrases, key);
      // Shared short words and scientific names are valid in neighboring languages.
      if (swedish[key].length > 25 && /[äö]|\b(?:och|för|till|att|är|från)\b/u.test(swedish[key])) {
        assert.notEqual(text, swedish[key], `${key}: copied Swedish sentence`);
      }
      assert.equal(typeof english[key], "string", `unknown key ${key}`);
      assert.deepEqual(placeholders(text), placeholders(english[key]), key);
      assert.deepEqual(units(text), units(english[key]), key);
      assert.equal(generated[key], text, `${key}: stale generated payload`);
    }
    assert.equal(entries["tabs.tools"], tools);
    assert.equal(entries.properties, properties);
    assert.equal(entries.nodes, nodes);
    assert.equal((catalog.overrides.shortcutLegendRows as string[]).length, 7);
    for (const token of ["Alt", "Shift", "1/2/3/4", "WASD", "BOX"]) {
      assert.ok((catalog.overrides.shortcutLegendRows as string[]).some((row) => row.includes(token)), token);
    }
  });

  test(`${language}: the correction supersedes cached official copy without rewriting imports`, async () => {
    const catalog = await loadBuiltinWorkbenchLanguagePackForLanguage(language);
    assert.ok(catalog);
    const old = { ...catalog, updatedAt: "2026-09-19T01:00:00.000Z", overrides: { properties: "Egenskaper" } };
    assert.equal(resolveInstalledWorkbenchLanguagePack(old, catalog), catalog);
    assert.deepEqual(old.overrides, { properties: "Egenskaper" });
    const imported = { ...old, source: "imported" as const };
    assert.equal(resolveInstalledWorkbenchLanguagePack(imported, catalog), imported);
    const future = { ...old, updatedAt: "2099-01-01T00:00:00.000Z" };
    assert.equal(resolveInstalledWorkbenchLanguagePack(future, catalog), future);
  });
}
