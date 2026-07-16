#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const targetPath = path.join(repoRoot, "config/localization/mainstream-language-pack-locales.json");
const catalogPath = path.join(repoRoot, "language-packs/catalog.json");

const REQUIRED_OVERRIDE_PATHS = {
  hub: [
    "nav.projects",
    "nav.runtimes",
    "nav.deploy",
    "nav.observe",
    "nav.tools",
    "sections.projects.title",
    "sections.projects.copy",
    "sections.runtimes.title",
    "sections.runtimes.copy",
    "sections.deploy.title",
    "sections.deploy.copy",
    "shell.language",
    "shell.actionStatus",
    "shell.idle",
    "shell.openWorkbench",
    "shell.startLocal",
    "shell.validateEnv",
  ],
  workbench: [
    "title",
    "subtitle",
    "rail.study",
    "rail.model",
    "rail.workflow",
    "rail.store",
    "rail.library",
    "rail.system",
    "sections.study",
    "sections.model",
    "sections.workflow",
    "sections.store",
    "sections.library",
    "sections.system",
    "workflowBuilderPage",
    "workflowRunsPage",
    "workflowCatalogTitle",
    "workflowTemplateChainLibraryLabel",
    "languagePacksTitle",
    "languagePacksHint",
    "languagePacksEmptyLabel",
    "languagePackName",
    "languagePackVersion",
    "languagePackSourceImported",
    "languagePackSourceDownloaded",
    "languagePackDownloadTemplate",
    "languagePackExportInstalled",
    "languagePackImport",
    "languagePackRemove",
    "languagePackCatalogTitle",
    "languagePackCatalogHint",
    "languagePackCatalogAction",
  ],
};

function readJson(absolutePath) {
  return JSON.parse(fs.readFileSync(absolutePath, "utf8"));
}

function valueAtPath(value, dottedPath) {
  return dottedPath.split(".").reduce((current, part) => {
    if (!current || typeof current !== "object" || Array.isArray(current)) return undefined;
    return current[part];
  }, value);
}

function packCoverage(pack) {
  const required = REQUIRED_OVERRIDE_PATHS[pack.targetSurface] ?? [];
  const missing = required.filter((entry) => {
    const value = valueAtPath(pack.overrides, entry);
    return typeof value !== "string" || !value.trim();
  });
  return {
    covered: required.length - missing.length,
    required: required.length,
    ratio: required.length ? (required.length - missing.length) / required.length : 1,
    missing,
  };
}

function formatPercent(ratio) {
  return `${Math.round(ratio * 1000) / 10}%`;
}

function main() {
  const target = readJson(targetPath);
  const catalog = readJson(catalogPath);
  const expectedLanguages = new Set(target.locales.map((locale) => locale.language));
  const rows = [];
  const totals = new Map();

  for (const entry of catalog.packs) {
    const pack = readJson(path.join(repoRoot, "language-packs", entry.path));
    const coverage = packCoverage(pack);
    rows.push({
      surface: pack.targetSurface,
      language: pack.language,
      covered: coverage.covered,
      required: coverage.required,
      percent: formatPercent(coverage.ratio),
      missing: coverage.missing,
    });

    const total = totals.get(pack.targetSurface) ?? { covered: 0, required: 0, languages: new Set() };
    total.covered += coverage.covered;
    total.required += coverage.required;
    total.languages.add(pack.language);
    totals.set(pack.targetSurface, total);
  }

  const summary = [...totals.entries()].map(([surface, total]) => ({
    surface,
    languages: `${total.languages.size}/${expectedLanguages.size}`,
    covered: `${total.covered}/${total.required}`,
    percent: formatPercent(total.required ? total.covered / total.required : 1),
  }));

  console.log(JSON.stringify({ summary, rows }, null, 2));
}

main();
