import assert from "node:assert/strict";
import { test } from "node:test";
import { getWorkbenchCompactPanelCopy } from "../../src/components/workbench/workbench-compact-panel-copy";
import { buildWorkbenchLanguageOptions } from "../../src/components/workbench/workbench-language-options";

test("compact panel navigation has explicit copy in all built-in and mainstream languages", () => {
  const languages = buildWorkbenchLanguageOptions({ copy: {}, languagePacks: [], currentLanguage: "en" });
  assert.equal(languages.length, 34);
  const fallback = getWorkbenchCompactPanelCopy("en");
  for (const { value } of languages) {
    const copy = getWorkbenchCompactPanelCopy(value);
    assert.deepEqual(Object.keys(copy), Object.keys(fallback), value);
    assert.ok(Object.values(copy).every((entry) => entry.trim()), value);
    if (value !== "en") assert.notEqual(copy, fallback, value);
  }
  assert.equal(getWorkbenchCompactPanelCopy("pt-BR"), getWorkbenchCompactPanelCopy("pt-br"));
  assert.equal(getWorkbenchCompactPanelCopy("unknown"), fallback);
});
