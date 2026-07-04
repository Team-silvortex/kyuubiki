import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

import {
  buildWorkbenchLanguagePackCatalogRows,
  getBuiltinWorkbenchLanguagePack,
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

test("workbench language pack catalog localizes readiness labels", () => {
  assert.match(buildWorkbenchLanguagePackCatalogRows("zh")[0]?.status ?? "", /本地导入/);
  assert.match(buildWorkbenchLanguagePackCatalogRows("ja")[0]?.status ?? "", /ローカル取込/);
  assert.match(buildWorkbenchLanguagePackCatalogRows("en")[0]?.status ?? "", /local import/);
});

test("workbench built-in support packs expose installable downloaded payloads", () => {
  const french = getBuiltinWorkbenchLanguagePack("workbench-fr-core-1.15");
  const korean = getBuiltinWorkbenchLanguagePack("workbench-ko-core-1.15");

  assert.equal(french?.source, "downloaded");
  assert.equal(french?.targetSurface, "workbench");
  assert.equal(french?.overrides.workflowCatalogTitle, "Catalogue de workflows");
  assert.equal(korean?.source, "downloaded");
  assert.equal(korean?.targetSurface, "workbench");
  assert.equal(korean?.overrides.workflowCatalogTitle, "워크플로 카탈로그");
  assert.equal(getBuiltinWorkbenchLanguagePack("missing"), null);
});
