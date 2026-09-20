import test from "node:test";
import assert from "node:assert/strict";
import { getWorkbenchScriptPanelCopy, workbenchScriptPanelCopy } from "../../src/components/workbench/workbench-script-panel-copy.ts";
import { getWorkbenchScriptDslCopy } from "../../src/components/workbench/workbench-script-dsl-copy.ts";
import { getWorkbenchScriptCatalogCopy } from "../../src/components/workbench/workbench-script-catalog-copy.ts";
import { getWorkbenchScriptWorkspaceCopy } from "../../src/components/workbench/workbench-script-workspace-copy.ts";

function flatten(value: object, prefix = ""): Record<string, string> {
  return Object.fromEntries(Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof child === "string" ? [[path, child]] : Object.entries(flatten(child, path));
  }));
}

const locales = ["ar", "fa", "es"] as const;
const sharedSpanishTerms = new Set(["title", "scriptMode", "macrosMode", "timelinePreviewIncludedNo", "error", "riskNormal", "categories.macro"]);
const placeholders = (value: string) => [...value.matchAll(/\{[a-zA-Z][a-zA-Z0-9_]*\}/g)].map(([token]) => token).sort();
const forbidden = /[\u200B\u200E\u200F\u202A-\u202E\u2060-\u2069\uFEFF\uFB50-\uFDFF\uFE70-\uFEFE]/u;

for (const language of locales) {
  for (const [surface, getter, size] of [
    ["panel", getWorkbenchScriptPanelCopy, 148],
    ["DSL", getWorkbenchScriptDslCopy, 8],
    ["catalog", getWorkbenchScriptCatalogCopy, 19],
  ] as const) {
    test(`${language} PWDT ${surface} translates its whole contract rather than copying the English fallback`, () => {
      const english = flatten(getter("en"));
      // Inspect raw panel entries so a future fallback merge cannot disguise missing translations.
      const translated = flatten(surface === "panel" ? workbenchScriptPanelCopy[language] : getter(language));
      assert.equal(Object.keys(english).length, size);
      assert.deepEqual(Object.keys(translated).sort(), Object.keys(english).sort());
      for (const [key, value] of Object.entries(translated)) {
        assert.ok(value.trim(), `${surface}.${key}`);
        assert.doesNotMatch(value, forbidden, `${surface}.${key}`);
        assert.deepEqual(placeholders(value), placeholders(english[key]), `${surface}.${key}`);
        const neutral = surface === "panel" && (language === "es" ? sharedSpanishTerms.has(key) : key === "title");
        if (neutral) continue;
        assert.notEqual(value, english[key], `${surface}.${key}`);
        if (language !== "es") assert.match(value, /\p{Script=Arabic}/u, `${surface}.${key}`);
      }
    });
  }

  test(`${language} PWDT locale aliases resolve consistently across the workspace and its subpanels`, () => {
    for (const getter of [getWorkbenchScriptPanelCopy, getWorkbenchScriptDslCopy, getWorkbenchScriptCatalogCopy, getWorkbenchScriptWorkspaceCopy]) {
      assert.equal(getter(` ${language.toUpperCase()}_XX `), getter(language));
      assert.equal(getter(`${language}-XX`), getter(language));
      for (const invalid of ["", "unknown", "__proto__", "constructor", "toString"]) {
        assert.equal(getter(invalid), getter("en"), invalid);
      }
    }
  });
}

test("PWDT runtime, recording, DSL and catalog terminology retains distinct meanings", () => {
  for (const [language, runtime, editor, truss, snippets] of [
    ["ar", "بيئة التشغيل", "برنامج Python النصي", "تحميل وصفة الجملون", "مقاطع الشيفرة"],
    ["fa", "محیط اجرا", "اسکریپت Python", "بارگذاری دستور خرپا", "قطعه‌کدها"],
    ["es", "Entorno de ejecución", "Script de Python", "Cargar receta de celosía", "Fragmentos"],
  ]) {
    const panel = getWorkbenchScriptPanelCopy(language);
    const catalog = getWorkbenchScriptCatalogCopy(language);
    assert.equal(panel.runtime, runtime);
    assert.equal(panel.editor, editor);
    assert.equal(getWorkbenchScriptDslCopy(language).recipe, truss);
    assert.equal(catalog.snippetsMode, snippets);
    assert.notEqual(catalog.snippetsMode, catalog.recipesMode);
    assert.notEqual(panel.startRecording, panel.stopRecording);
    assert.match(panel.subtitle, /Pyodide/);
    assert.match(panel.headlessSurface, /SDK/);
    assert.match(panel.importMacroJson, /JSON/);
    assert.match(panel.exportPresetJson, /JSON/);
  }
});
