import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { loadBuiltinWorkbenchLanguagePackForLanguage, WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES } from "../../src/components/workbench/workbench-language-pack-catalog.ts";
import { copyEsPrimary } from "../../src/components/workbench/workbench-copy-es-primary.ts";

const primaryKeys = [
  "rail.study",
  "rail.model",
  "rail.workflow",
  "rail.store",
  "rail.library",
  "rail.system",
  "sections.study",
  "sections.workflow",
  "sections.store",
  "sections.library",
  "roleLabel",
  "overview",
  "result",
  "actions",
  "details",
  "controls",
  "report",
  "messages",
  "viewport",
  "settings",
  "scripts",
  "assistant",
  "config",
  "runtime",
  "data",
  "packs",
  "routing",
  "access",
  "stack",
  "security",
  "agents",
  "audit",
  "modelMaterialsPage",
  "modelGeneratePage",
  "createProject",
  "projectNameField",
  "projectDescriptionField",
  "importProject",
  "exportProject",
  "save",
  "saveAs",
  "deleteProject",
  "workflowBuilderPage",
  "workflowRunsPage",
  "workflowCatalogTitle",
  "workflowRunDraftLabel",
  "renderDiagnosticsTitle",
  "renderStrategyAuto",
  "renderStrategyFull",
  "renderStrategyProgressive",
  "renderStrategyFocus",
  "sections.model",
  "sections.system",
  "workspace",
  "modelStudyPage",
  "modelStudioPage",
  "modelOverviewPage",
  "controlsSetupPage",
  "controlsReviewPage",
  "tabs.summary",
  "tabs.controls",
  "tabs.results",
  "tabs.models"
];

function leaf(object: unknown, path: string): unknown {
  return path.split(".").reduce<unknown>((value, key) => (value as Record<string, unknown>)?.[key], object);
}

test("primary navigation, projects and rendering have clean localized text in all 30 packs", async () => {
  for (const { language } of WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES) {
    const pack = await loadBuiltinWorkbenchLanguagePackForLanguage(language);
    assert.ok(pack, language);
    for (const key of primaryKeys) {
      const text = leaf(pack.overrides, key);
      assert.equal(typeof text, "string", `${language}:${key}`);
      assert.ok((text as string).trim(), `${language}:${key}`);
      assert.doesNotMatch(text as string, /[\u200b\u2060\ufeff]/u, `${language}:${key}`);
    }
  }
});

test("store labels describe the store rather than a save action or a construction worker", async () => {
  assert.equal(leaf((await loadBuiltinWorkbenchLanguagePackForLanguage("de"))?.overrides, "rail.store"), "Shop");
  assert.equal(leaf((await loadBuiltinWorkbenchLanguagePackForLanguage("de"))?.overrides, "workflowBuilderPage"), "Editor");
  assert.equal(leaf((await loadBuiltinWorkbenchLanguagePackForLanguage("vi"))?.overrides, "rail.study"), "Nghiên cứu");
  assert.equal(leaf((await loadBuiltinWorkbenchLanguagePackForLanguage("ur"))?.overrides, "report"), "رپورٹ");
  assert.equal(copyEsPrimary.sections.store, "Tienda del proyecto");
  assert.equal(copyEsPrimary.renderDiagnosticsTitle, "Diagnóstico de renderizado");
});

test("Hub bundle operations have local overrides in all 30 shipped language packs", () => {
  const fields = ["introTitle","introCopy","bundlePath","comparePath","outputPath","inspect","validate","normalize","unpack","pack","diff","openWorkbench","desktopTools","recentBundles","recentCompare","recentOutputs","recentActions","favorites","ready","all","failed","keepFailed","import","export","clear","recent"];
  for (const { language } of WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES) {
    const path = new URL(`../../../../language-packs/hub/${language.toLowerCase()}.json`, import.meta.url);
    const pack = JSON.parse(readFileSync(path, "utf8"));
    for (const key of fields) {
      assert.equal(typeof pack.overrides.bundles[key], "string", `${language}:bundles.${key}`);
      assert.ok(pack.overrides.bundles[key].trim(), `${language}:bundles.${key}`);
      assert.doesNotMatch(pack.overrides.bundles[key], /[\u200b\u2060\ufeff]/u);
    }
  }
});
