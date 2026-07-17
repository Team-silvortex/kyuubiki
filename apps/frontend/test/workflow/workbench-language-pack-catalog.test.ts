import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

import {
  buildWorkbenchLanguagePackCatalogRows,
  getBuiltinWorkbenchLanguagePack,
  WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES,
} from "../../src/components/workbench/workbench-language-pack-catalog.ts";
import {
  getPwdtSurfaceDescriptor,
  isPwdtFullConsole,
  PWDT_LONG_NAME,
  PWDT_PRODUCT_NAME,
} from "../../src/lib/scripting/pwdt-surface.ts";
import { getWorkbenchLanguagePackSystemCopy } from "../../src/components/workbench/workbench-language-pack-system-copy.ts";
import {
  getWorkbenchGuardrailCopy,
  getWorkbenchAssistantAuditCopy,
  getWorkbenchMaterialLibraryCopy,
  getWorkbenchMaterialCopy,
  getWorkbenchProtocolAgentCopy,
  getWorkbenchRuntimeAuditEmptyLabel,
  getWorkbenchRuntimeAuditCopy,
  getWorkbenchScriptErrorCopy,
  getWorkbenchScriptInspectCopy,
  getWorkbenchProjectFlowCopy,
} from "../../src/components/workbench/workbench-extended-language-copy.ts";
import {
  getWorkbenchScriptActionSummary,
  getWorkbenchScriptCatalogCopy,
  getWorkbenchScriptMacroSummary,
  getWorkbenchScriptSnippetSummary,
  getWorkbenchScriptSnippetTitle,
} from "../../src/components/workbench/workbench-script-catalog-copy.ts";
import { getWorkbenchScriptDslCopy } from "../../src/components/workbench/workbench-script-dsl-copy.ts";

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

test("pwdt surface registry keeps Workbench full and Hub Installer bounded", () => {
  assert.equal(PWDT_PRODUCT_NAME, "Pwdt");
  assert.equal(PWDT_LONG_NAME, "Python WASM DSL Tooling");
  assert.equal(getPwdtSurfaceDescriptor("workbench").status, "implemented");
  assert.equal(getPwdtSurfaceDescriptor("workbench").canRunPyodide, true);
  assert.equal(getPwdtSurfaceDescriptor("workbench").canEditDsl, true);
  assert.equal(getPwdtSurfaceDescriptor("hub").status, "launcher");
  assert.equal(getPwdtSurfaceDescriptor("hub").canRunPyodide, false);
  assert.equal(getPwdtSurfaceDescriptor("installer").status, "planned-restricted");
  assert.equal(getPwdtSurfaceDescriptor("installer").canInvokeFrontendActions, false);
  assert.equal(isPwdtFullConsole("workbench"), true);
  assert.equal(isPwdtFullConsole("hub"), false);
  assert.equal(isPwdtFullConsole("installer"), false);
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
  const french = getBuiltinWorkbenchLanguagePack("workbench-fr-core-2.0");
  const korean = getBuiltinWorkbenchLanguagePack("workbench-ko-core-2.0");
  const traditionalChinese = getBuiltinWorkbenchLanguagePack("workbench-zh-tw-core-2.0");

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

test("workbench language pack system copy covers mainstream feedback without English fallback", () => {
  const english = getWorkbenchLanguagePackSystemCopy("en");
  for (const locale of WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES) {
    const copy = getWorkbenchLanguagePackSystemCopy(locale.language);
    assert.ok(copy.targetPrefix.trim(), locale.language);
    assert.ok(copy.imported.trim(), locale.language);
    assert.ok(copy.removed.trim(), locale.language);
    assert.ok(copy.invalidJson.trim(), locale.language);
    assert.notEqual(copy.imported, english.imported, locale.language);
    assert.notEqual(copy.removed, english.removed, locale.language);
  }
});

test("workbench script DSL copy covers mainstream languages without button fallback", () => {
  const english = getWorkbenchScriptDslCopy("en");
  for (const locale of WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES) {
    const copy = getWorkbenchScriptDslCopy(locale.language);
    assert.ok(copy.title.trim(), locale.language);
    assert.ok(copy.compile.trim(), locale.language);
    assert.ok(copy.run.trim(), locale.language);
    assert.ok(copy.reset.trim(), locale.language);
    assert.ok(copy.macro.trim(), locale.language);
    assert.notEqual(copy.compile, english.compile, locale.language);
  }
  assert.equal(getWorkbenchScriptDslCopy("zh-TW").compile, "編譯到腳本");
  assert.equal(getWorkbenchScriptDslCopy("pt-BR").compile, "Compilar");
});

test("workbench script catalog copy covers mainstream labels and translated script summaries", () => {
  const english = getWorkbenchScriptCatalogCopy("en");
  const action = {
    id: "ui.click",
    category: "navigation",
    risk: "normal" as const,
    summary: {
      en: "Click a stable UI target.",
      zh: "点击一个稳定 UI 目标。",
    },
  };
  const macro = {
    id: "macro/openDataResults",
    category: "data",
    risk: "normal" as const,
    summary: {
      en: "Open the data results view.",
      zh: "打开数据结果视图。",
    },
    steps: [],
    requiresConfirmation: false,
  };
  const snippet = {
    id: "snippet/runtime/open-control-panel",
    category: "runtime" as const,
    title: {
      en: "Open runtime control panel",
      zh: "打开运行时控制面",
    },
    summary: {
      en: "Open the runtime control panel.",
      zh: "打开运行时控制面。",
    },
    code: "ky.log('ok')",
  };

  for (const locale of WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES) {
    const copy = getWorkbenchScriptCatalogCopy(locale.language);
    const actionSummary = getWorkbenchScriptActionSummary(action, locale.language);
    const macroSummary = getWorkbenchScriptMacroSummary(macro, locale.language);
    const snippetTitle = getWorkbenchScriptSnippetTitle(snippet, locale.language);
    const snippetSummary = getWorkbenchScriptSnippetSummary(snippet, locale.language);
    assert.ok(copy.snippetsMode.trim(), locale.language);
    assert.ok(copy.insertConfigured.trim(), locale.language);
    assert.ok(copy.importPreset.trim(), locale.language);
    assert.ok(actionSummary.trim(), locale.language);
    assert.ok(macroSummary.trim(), locale.language);
    assert.ok(snippetTitle.trim(), locale.language);
    assert.ok(snippetSummary.trim(), locale.language);
    assert.notEqual(copy.insertConfigured, english.insertConfigured, locale.language);
    assert.notEqual(actionSummary, "ui.click · navigation", locale.language);
    assert.notEqual(snippetTitle, snippet.title.en, locale.language);
    assert.notEqual(snippetSummary, snippet.summary.en, locale.language);
  }

  assert.equal(getWorkbenchScriptActionSummary(action, "en"), "Click a stable UI target.");
  assert.equal(getWorkbenchScriptActionSummary(action, "zh"), "点击一个稳定 UI 目标。");
  assert.equal(getWorkbenchScriptActionSummary(action, "fr"), "Action: Navigation — UI clic");
  assert.equal(getWorkbenchScriptSnippetTitle(snippet, "fr"), "runtime ouvrir contrôle panneau");
});

test("workbench extended UI copy covers guardrails, audit, and material labels", () => {
  const englishGuardrail = getWorkbenchGuardrailCopy("en");
  const englishMaterial = getWorkbenchMaterialCopy("en");

  for (const locale of WORKBENCH_MAINSTREAM_LANGUAGE_PACK_LOCALES) {
    const guardrail = getWorkbenchGuardrailCopy(locale.language);
    const audit = getWorkbenchRuntimeAuditCopy(locale.language);
    const material = getWorkbenchMaterialCopy(locale.language);
    const materialLibrary = getWorkbenchMaterialLibraryCopy(locale.language);
    const scriptErrors = getWorkbenchScriptErrorCopy(locale.language);
    const inspect = getWorkbenchScriptInspectCopy(locale.language);
    const protocolAgent = getWorkbenchProtocolAgentCopy(locale.language);
    const auditEmpty = getWorkbenchRuntimeAuditEmptyLabel(locale.language);
    const projectFlow = getWorkbenchProjectFlowCopy(locale.language);
    const assistantAudit = getWorkbenchAssistantAuditCopy(locale.language);
    assert.ok(guardrail.title.trim(), locale.language);
    assert.ok(guardrail.runtime.trim(), locale.language);
    assert.ok(audit.completed.trim(), locale.language);
    assert.ok(audit.script.trim(), locale.language);
    assert.ok(material.steel.trim(), locale.language);
    assert.ok(material.carbonFiber.trim(), locale.language);
    assert.ok(materialLibrary.title.trim(), locale.language);
    assert.ok(materialLibrary.addMaterial.trim(), locale.language);
    assert.ok(scriptErrors.macroMissing.trim(), locale.language);
    assert.ok(inspect.layoutSummary.trim(), locale.language);
    assert.ok(protocolAgent.countLabel(2, 1, 1).trim(), locale.language);
    assert.ok(auditEmpty.trim(), locale.language);
    assert.ok(projectFlow.noJobVersion.trim(), locale.language);
    assert.ok(projectFlow.skippedSensitivePresets(2).trim(), locale.language);
    assert.ok(assistantAudit.manualRecording.trim(), locale.language);
    assert.ok(assistantAudit.governanceDriftDetected("drift").trim(), locale.language);
    assert.notEqual(guardrail.title, englishGuardrail.title, locale.language);
    assert.notEqual(material.steel, englishMaterial.steel, locale.language);
    assert.notEqual(materialLibrary.title, "Material Library", locale.language);
    assert.notEqual(scriptErrors.macroMissing, "Could not find the requested macro.", locale.language);
    assert.notEqual(inspect.layoutSummary, "Layout Summary", locale.language);
    assert.notEqual(auditEmpty, "No security events match the current filters.", locale.language);
    assert.notEqual(projectFlow.noJobVersion, "This job does not have a linked model version.", locale.language);
    assert.notEqual(assistantAudit.planExecuted, "Assistant plan executed.", locale.language);
  }

  assert.equal(getWorkbenchGuardrailCopy("fr").title, "Garde-fous UX");
  assert.equal(getWorkbenchRuntimeAuditCopy("ko").assistant, "도우미");
  assert.equal(getWorkbenchMaterialCopy("zh-TW").carbonFiber, "碳纖維");
  assert.equal(getWorkbenchMaterialLibraryCopy("de").deleteMaterial, "Material löschen");
  assert.equal(getWorkbenchScriptErrorCopy("fr").linkedProjectMissing, "Cet enregistrement n'a pas de projet lié.");
  assert.equal(getWorkbenchScriptInspectCopy("zh-TW").activeSidebar, "目前側欄");
  assert.match(getWorkbenchProtocolAgentCopy("es").countLabel(2, 1, 0), /Agentes alcanzables/);
  assert.equal(getWorkbenchRuntimeAuditEmptyLabel("zh-TW"), "目前篩選下沒有安全事件。");
  assert.equal(getWorkbenchProjectFlowCopy("fr").linkedProjectMissing, "Projet lié introuvable.");
  assert.equal(getWorkbenchAssistantAuditCopy("zh-TW").planExecuted, "助手計畫已執行。");
});
