import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

import {
  buildWorkbenchLanguagePackCatalogRows,
  getBuiltinWorkbenchLanguagePack,
  WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES,
} from "../../src/components/workbench/workbench-language-pack-catalog.ts";

test("workbench language pack catalog mirrors shipped workbench support packs", () => {
  const catalogPath = path.resolve(process.cwd(), "../../language-packs/catalog.json");
  const catalog = JSON.parse(readFileSync(catalogPath, "utf8")) as {
    packs: Array<{ id: string; language: string; name: string; surface: string }>;
  };
  const shippedWorkbenchPacks = catalog.packs
    .filter((pack) => pack.surface === "workbench")
    .map((pack) => ({ id: pack.id, language: pack.language, name: pack.name }))
    .sort((left, right) => left.id.localeCompare(right.id));
  const rows = buildWorkbenchLanguagePackCatalogRows("en")
    .map((pack) => ({ id: pack.id, language: pack.language, name: pack.name }))
    .sort((left, right) => left.id.localeCompare(right.id));

  assert.deepEqual(rows, shippedWorkbenchPacks);
});

test("workbench language pack catalog covers the mainstream 30 locale target", () => {
  const targetPath = path.resolve(process.cwd(), "../../config/localization/mainstream-language-pack-locales.json");
  const target = JSON.parse(readFileSync(targetPath, "utf8")) as {
    target_count: number;
    locales: Array<{ language: string; englishName: string; nativeName: string }>;
  };
  const rows = buildWorkbenchLanguagePackCatalogRows("en");
  const languages = new Set(rows.map((pack) => pack.language));
  const targetLanguages = target.locales.map((locale) => locale.language).sort();
  const frontendLanguages = WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES.map((locale) => locale.language).sort();

  assert.equal(rows.length, target.target_count);
  assert.equal(WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES.length, target.target_count);
  assert.equal(languages.size, target.target_count);
  assert.deepEqual(frontendLanguages, targetLanguages);
  assert.ok(languages.has("ar"));
  assert.ok(languages.has("pt-BR"));
  assert.ok(languages.has("zh-TW"));
});

test("workbench language pack catalog localizes readiness labels", () => {
  assert.match(buildWorkbenchLanguagePackCatalogRows("zh")[0]?.status ?? "", /本地导入/);
  assert.match(buildWorkbenchLanguagePackCatalogRows("ja")[0]?.status ?? "", /ローカル取込/);
  assert.match(buildWorkbenchLanguagePackCatalogRows("en")[0]?.status ?? "", /local import/);
});

test("workbench built-in support packs expose installable downloaded payloads", () => {
  const french = getBuiltinWorkbenchLanguagePack("workbench-fr-core-1.19");
  const korean = getBuiltinWorkbenchLanguagePack("workbench-ko-core-1.19");
  const traditionalChinese = getBuiltinWorkbenchLanguagePack("workbench-zh-tw-core-1.19");

  assert.equal(french?.source, "downloaded");
  assert.equal(french?.targetSurface, "workbench");
  assert.equal(french?.overrides.workflowCatalogTitle, "Catalogue de workflows");
  assert.equal(korean?.source, "downloaded");
  assert.equal(korean?.targetSurface, "workbench");
  assert.equal(korean?.overrides.workflowCatalogTitle, "워크플로 카탈로그");
  assert.equal(traditionalChinese?.source, "downloaded");
  assert.equal(traditionalChinese?.targetSurface, "workbench");
  assert.equal(traditionalChinese?.language, "zh-TW");
  assert.equal(getBuiltinWorkbenchLanguagePack("missing"), null);
});
