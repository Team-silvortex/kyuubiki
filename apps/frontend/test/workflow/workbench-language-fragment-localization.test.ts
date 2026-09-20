import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolveWorkbenchBaseCopy } from "../../src/components/workbench/workbench-copy.ts";
import { loadWorkbenchTranslatedLanguagePackOverrides } from "../../src/components/workbench/workbench-language-pack-catalog-data.ts";
import {
  loadBuiltinWorkbenchLanguagePackForLanguage,
  resolveInstalledWorkbenchLanguagePack,
} from "../../src/components/workbench/workbench-language-pack-catalog.ts";

const locales = {
  ar: ["الأدوات", "الخصائص", "العقد"],
  bn: ["সরঞ্জাম", "বৈশিষ্ট্য", "নোড"],
  cs: ["Nástroje", "Vlastnosti", "Uzly"],
  da: ["Værktøjer", "Egenskaber", "Knuder"],
  fa: ["ابزارها", "ویژگی‌ها", "گره‌ها"],
  fi: ["Työkalut", "Ominaisuudet", "Solmut"],
  he: ["כלים", "מאפיינים", "צמתים"],
  hi: ["उपकरण", "गुण", "नोड"],
  id: ["Alat", "Properti", "Simpul"],
  ms: ["Alat", "Sifat", "Nod"],
  no: ["Verktøy", "Egenskaper", "Noder"],
  ro: ["Instrumente", "Proprietăți", "Noduri"],
  sw: ["Zana", "Sifa", "Nodi"],
};
const revision = "2026-09-20T03:00:00.000Z";
const fragmentRevision = "2026-09-20T01:00:00.000Z";
const scriptPatterns: Record<string, RegExp> = {
  ar: /\p{Script=Arabic}/u, fa: /\p{Script=Arabic}/u,
  bn: /\p{Script=Bengali}/u, he: /\p{Script=Hebrew}/u, hi: /\p{Script=Devanagari}/u,
};
const neutralKeys = new Set(["exportCsv", "exportJson", "jumpMid", "jumpQuarter", "jumpThreeQuarter"]);
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
    const sourceRevision = scriptPatterns[language] ? fragmentRevision : "2026-09-19T02:00:00.000Z";
    for (const part of parts) {
      assert.equal(part.language, language);
      assert.equal(part.updatedAt, sourceRevision);
    }
    const catalog = await loadBuiltinWorkbenchLanguagePackForLanguage(language);
    assert.ok(catalog);
    const generated = flatten(catalog.overrides);
    assert.equal(readPack(language).updatedAt, language === "ar" || language === "fa" ? revision : sourceRevision);
    assert.equal(catalog.updatedAt, revision);
    for (const [key, text] of Object.entries(entries)) {
      assert.ok(text.trim(), key);
      assert.doesNotMatch(text, foreignPhrases, key);
      assert.doesNotMatch(text, /[\u200B\u2060\uFEFF]/u, `${key}: invisible padding`);
      if (language === "ar" || language === "fa") {
        assert.doesNotMatch(text, /[\uFB50-\uFDFF\uFE70-\uFEFF]/u, `${key}: use logical Unicode, not Arabic presentation forms`);
      }
      if (scriptPatterns[language] && !neutralKeys.has(key)) {
        assert.match(text, scriptPatterns[language], `${key}: missing target-language script`);
      }
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
    const old = { ...catalog, updatedAt: fragmentRevision, overrides: { properties: "Egenskaper" } };
    assert.equal(resolveInstalledWorkbenchLanguagePack(old, catalog), catalog);
    assert.deepEqual(old.overrides, { properties: "Egenskaper" });
    const imported = { ...old, source: "imported" as const };
    assert.equal(resolveInstalledWorkbenchLanguagePack(imported, catalog), imported);
    const future = { ...old, updatedAt: "2099-01-01T00:00:00.000Z" };
    assert.equal(resolveInstalledWorkbenchLanguagePack(future, catalog), future);
  });
}

for (const [language, expected] of Object.entries({
  ar: { assistantOpen: "فتح المساعد", assistantClose: "إغلاق المساعد", "kinds.spring_2d": "نابض ثنائي الأبعاد", "kinds.plane_quad_2d": "عنصر رباعي مستو ثنائي الأبعاد", workflowKindLabel: "النوع" },
  fa: { assistantOpen: "باز کردن دستیار", assistantClose: "بستن دستیار", "kinds.spring_2d": "فنر دوبعدی", "kinds.plane_quad_2d": "المان چهارضلعی دوبعدی", workflowPackageInstallRulesReceiptTitle: "رسید اجرا" },
})) {
  test(`${language}: base copy uses logical Unicode and preserves scientific and runtime contracts`, async () => {
    const root = readPack(language);
    const entries = flatten(root.overrides);
    assert.equal(root.updatedAt, revision);
    assert.equal(Object.keys(entries).length, 630);
    const payload = await loadWorkbenchTranslatedLanguagePackOverrides(language);
    assert.ok(payload);
    const generated = flatten(payload);
    const extended = fragments(language).entries;
    for (const [key, text] of Object.entries(entries)) {
      assert.match(text, /\p{Script=Arabic}/u, `${key}: missing localized text`);
      assert.doesNotMatch(text, /[\uFB50-\uFDFF\uFE70-\uFEFF\u200B\u2060]/u, `${key}: presentation forms or invisible padding`);
      assert.equal(typeof english[key], "string", `unknown key ${key}`);
      assert.deepEqual(placeholders(text), placeholders(english[key]), key);
      assert.equal(generated[key], extended[key] ?? text, `${key}: stale generated base payload`);
    }
    for (const [key, value] of Object.entries(expected)) assert.equal(entries[key], value, key);
    for (const [key, tokens] of Object.entries({
      solverAgent: ["Rust"], orchestrator: ["Elixir"],
      controlPlaneTokenHelp: ["x-kyuubiki-token", "/api/v1"],
      directMeshEndpointsHelp: ["host:port"], directMeshTokenHelp: ["/api/direct-mesh"],
      runtimeSecurityFooter: ["/api/health"], mirrorX: ["X"], mirrorY: ["Y"], mirrorZ: ["Z"],
    })) {
      for (const token of tokens) assert.ok(entries[key].includes(token), `${key}: missing ${token}`);
    }
  });
}
