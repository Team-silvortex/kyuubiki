#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const packRoot = path.join(repoRoot, "language-packs");

const PACK_SCHEMA_VERSION = "kyuubiki.language-pack/v1";
const CATALOG_SCHEMA_VERSION = "kyuubiki.language-pack-catalog/v1";
const LOCALE_TARGET_SCHEMA_VERSION = "kyuubiki.localization-mainstream-locales/v1";
const VERSION_LINE = "moxi 2.x";
const TARGET_APP_VERSION = readCurrentReleaseVersion();
const SOURCE_VALUES = new Set(["downloaded", "imported"]);
const SURFACES = new Set(["workbench", "hub"]);
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
const UNSAFE_TEXT_PATTERNS = [
  "<",
  ">",
  "javascript:",
  "data:text/html",
  "onerror=",
  "onclick=",
  "innerhtml",
  "document.cookie",
  "localstorage",
  "eval(",
];

const errors = [];

function fail(message) {
  errors.push(message);
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isSafeRepoRelativePath(relativePath) {
  if (typeof relativePath !== "string" || !relativePath.trim()) return false;
  const normalized = relativePath.replaceAll("\\", "/");
  if (path.isAbsolute(relativePath) || /^[A-Za-z]:\//.test(normalized)) return false;
  return !normalized.split("/").some((part) => part === "..");
}

function collectUnsafeTextIssues(value, label, issues = []) {
  if (typeof value === "string") {
    const normalized = value.toLowerCase();
    const pattern = UNSAFE_TEXT_PATTERNS.find((entry) => normalized.includes(entry));
    if (pattern) issues.push(`${label}: unsafe language pack text contains ${pattern}`);
    return issues;
  }
  if (Array.isArray(value)) {
    value.forEach((entry, index) => collectUnsafeTextIssues(entry, `${label}[${index}]`, issues));
    return issues;
  }
  if (isPlainObject(value)) {
    Object.entries(value).forEach(([key, entry]) => collectUnsafeTextIssues(entry, `${label}.${key}`, issues));
  }
  return issues;
}

function readJson(relativePath) {
  if (!isSafeRepoRelativePath(relativePath)) {
    fail(`${relativePath}: path must be repository-relative and stay inside language pack validation roots`);
    return null;
  }
  const absolutePath = path.join(repoRoot, relativePath);
  try {
    return JSON.parse(fs.readFileSync(absolutePath, "utf8"));
  } catch (error) {
    fail(`${relativePath}: ${error.message}`);
    return null;
  }
}

function readCurrentReleaseVersion() {
  const channelsPath = path.join(repoRoot, "deploy/update-channels.json");
  const channels = JSON.parse(fs.readFileSync(channelsPath, "utf8"));
  if (typeof channels.shipping_version !== "string" || !channels.shipping_version.trim()) {
    throw new Error("deploy/update-channels.json must declare shipping_version for language pack validation");
  }
  return channels.shipping_version;
}

function validateString(pack, field, relativePath) {
  if (typeof pack[field] !== "string" || !pack[field].trim()) {
    fail(`${relativePath}: missing non-empty string field ${field}`);
  }
}

function validateTimestamp(value, relativePath) {
  if (typeof value !== "string" || Number.isNaN(Date.parse(value))) {
    fail(`${relativePath}: updatedAt must be an ISO date-time string`);
  }
}

function valueAtPath(value, dottedPath) {
  return dottedPath.split(".").reduce((current, part) => {
    if (!isPlainObject(current)) return undefined;
    return current[part];
  }, value);
}

function coverageForPack(pack) {
  const requiredPaths = REQUIRED_OVERRIDE_PATHS[pack.targetSurface] ?? [];
  const missing = requiredPaths.filter((entry) => {
    const value = valueAtPath(pack.overrides, entry);
    return typeof value !== "string" || !value.trim();
  });
  return {
    required: requiredPaths.length,
    covered: requiredPaths.length - missing.length,
    missing,
  };
}

function validateLocaleTarget() {
  const target = readJson("config/localization/mainstream-language-pack-locales.json");
  if (!isPlainObject(target)) {
    fail("config/localization/mainstream-language-pack-locales.json: target must be a JSON object");
    return new Map();
  }
  if (target.schema_version !== LOCALE_TARGET_SCHEMA_VERSION) {
    fail(`config/localization/mainstream-language-pack-locales.json: schema_version must be ${LOCALE_TARGET_SCHEMA_VERSION}`);
  }
  if (target.line !== VERSION_LINE) {
    fail(`config/localization/mainstream-language-pack-locales.json: line must be ${VERSION_LINE}`);
  }
  collectUnsafeTextIssues(target, "config/localization/mainstream-language-pack-locales.json").forEach(fail);
  validateTimestamp(target.updatedAt, "config/localization/mainstream-language-pack-locales.json");
  if (!Array.isArray(target.locales)) {
    fail("config/localization/mainstream-language-pack-locales.json: locales must be an array");
    return new Map();
  }
  if (target.locales.length !== target.target_count) {
    fail("config/localization/mainstream-language-pack-locales.json: target_count must match locales length");
  }

  const localeTarget = new Map();
  target.locales.forEach((locale, index) => {
    const label = `config/localization/mainstream-language-pack-locales.json:locales[${index}]`;
    if (!isPlainObject(locale)) {
      fail(`${label}: locale must be an object`);
      return;
    }
    ["language", "englishName", "nativeName"].forEach((field) => validateString(locale, field, label));
    if (localeTarget.has(locale.language)) {
      fail(`${label}: duplicate language ${locale.language}`);
    }
    localeTarget.set(locale.language, locale);
  });

  return localeTarget;
}

function validatePack(relativePath, expectedSurface) {
  if (!isSafeRepoRelativePath(relativePath)) {
    fail(`language-packs/${relativePath}: path must stay inside language pack validation roots`);
    return null;
  }
  const pack = readJson(`language-packs/${relativePath}`);
  if (!isPlainObject(pack)) {
    fail(`language-packs/${relativePath}: pack must be a JSON object`);
    return null;
  }

  [
    "schema_version",
    "id",
    "language",
    "targetSurface",
    "name",
    "version",
    "versionLine",
    "targetAppVersion",
    "source",
    "updatedAt",
  ].forEach((field) => validateString(pack, field, `language-packs/${relativePath}`));

  if (pack.schema_version !== PACK_SCHEMA_VERSION) {
    fail(`language-packs/${relativePath}: schema_version must be ${PACK_SCHEMA_VERSION}`);
  }
  if (pack.targetSurface !== expectedSurface) {
    fail(`language-packs/${relativePath}: targetSurface must be ${expectedSurface}`);
  }
  if (pack.versionLine !== VERSION_LINE) {
    fail(`language-packs/${relativePath}: versionLine must be ${VERSION_LINE}`);
  }
  if (pack.targetAppVersion !== TARGET_APP_VERSION || pack.version !== TARGET_APP_VERSION) {
    fail(`language-packs/${relativePath}: version and targetAppVersion must be ${TARGET_APP_VERSION}`);
  }
  if (!SOURCE_VALUES.has(pack.source)) {
    fail(`language-packs/${relativePath}: source must be imported or downloaded`);
  }
  if (!isPlainObject(pack.overrides)) {
    fail(`language-packs/${relativePath}: overrides must be an object`);
  }
  collectUnsafeTextIssues(pack, `language-packs/${relativePath}`).forEach(fail);
  validateTimestamp(pack.updatedAt, `language-packs/${relativePath}`);

  return pack;
}

function discoverPackFiles(surface) {
  const surfaceDir = path.join(packRoot, surface);
  if (!fs.existsSync(surfaceDir)) {
    fail(`language-packs/${surface}: missing surface directory`);
    return [];
  }
  return fs
    .readdirSync(surfaceDir)
    .filter((entry) => entry.endsWith(".json"))
    .sort()
    .map((entry) => `${surface}/${entry}`);
}

const catalog = readJson("language-packs/catalog.json");
const localeTarget = validateLocaleTarget();
const referencedPaths = new Set();
const seenIds = new Set();
const packsBySurface = new Map([...SURFACES].map((surface) => [surface, []]));
const validatedPacks = [];
const coverageTotals = new Map([...SURFACES].map((surface) => [surface, { covered: 0, required: 0 }]));

if (isPlainObject(catalog)) {
  if (catalog.schema_version !== CATALOG_SCHEMA_VERSION) {
    fail(`language-packs/catalog.json: schema_version must be ${CATALOG_SCHEMA_VERSION}`);
  }
  if (catalog.line !== VERSION_LINE) {
    fail(`language-packs/catalog.json: line must be ${VERSION_LINE}`);
  }
  if (catalog.shipping_version !== TARGET_APP_VERSION) {
    fail(`language-packs/catalog.json: shipping_version must be ${TARGET_APP_VERSION}`);
  }
  validateTimestamp(catalog.updatedAt, "language-packs/catalog.json");
  if (!Array.isArray(catalog.packs)) {
    fail("language-packs/catalog.json: packs must be an array");
  } else {
    collectUnsafeTextIssues(catalog, "language-packs/catalog.json").forEach(fail);
    catalog.packs.forEach((entry, index) => {
      const label = `language-packs/catalog.json:packs[${index}]`;
      if (!isPlainObject(entry)) {
        fail(`${label}: entry must be an object`);
        return;
      }
      ["id", "surface", "language", "name", "path", "status"].forEach((field) =>
        validateString(entry, field, label),
      );
      if (!SURFACES.has(entry.surface)) {
        fail(`${label}: surface must be workbench or hub`);
      }
      if (seenIds.has(entry.id)) {
        fail(`${label}: duplicate id ${entry.id}`);
      }
      seenIds.add(entry.id);
      if (!isSafeRepoRelativePath(entry.path)) {
        fail(`${label}: path must stay inside language pack validation roots`);
        return;
      }
      referencedPaths.add(entry.path);

      const pack = validatePack(entry.path, entry.surface);
      if (!pack) return;
      const coverage = coverageForPack(pack);
      const coverageTotal = coverageTotals.get(entry.surface);
      if (coverageTotal) {
        coverageTotal.covered += coverage.covered;
        coverageTotal.required += coverage.required;
      }
      if (coverage.missing.length > 0) {
        fail(`${label}: ${entry.path} missing override coverage ${coverage.missing.join(", ")}`);
      }
      validatedPacks.push(pack);
      packsBySurface.get(entry.surface)?.push(pack);
      if (pack.id !== entry.id) {
        fail(`${label}: id does not match pack id ${pack.id}`);
      }
      if (pack.language !== entry.language) {
        fail(`${label}: language does not match pack language ${pack.language}`);
      }
      if (pack.name !== entry.name) {
        fail(`${label}: name does not match pack name ${pack.name}`);
      }
    });
  }
} else {
  fail("language-packs/catalog.json: catalog must be a JSON object");
}

const frontendCatalogTest = spawnSync(
  "node",
  [
    "./scripts/test-unit.mjs",
    "workflow/workbench-language-pack-catalog",
  ],
  {
    cwd: path.join(repoRoot, "apps/frontend"),
    encoding: "utf8",
  },
);

const frontendCatalogDataCheck = spawnSync(
  "node",
  ["./scripts/build-workbench-language-pack-catalog.mjs", "--check"],
  { cwd: repoRoot, encoding: "utf8" },
);

if (frontendCatalogDataCheck.status !== 0) {
  fail(
    [
      "apps/frontend/src/components/workbench/workbench-language-pack-catalog-data.ts is stale",
      frontendCatalogDataCheck.stdout.trim(),
      frontendCatalogDataCheck.stderr.trim(),
    ].filter(Boolean).join("\n"),
  );
}

if (frontendCatalogTest.status !== 0) {
  fail(
    [
      "apps/frontend/test/workflow/workbench-language-pack-catalog.test.ts failed",
      frontendCatalogTest.stdout.trim(),
      frontendCatalogTest.stderr.trim(),
    ].filter(Boolean).join("\n"),
  );
}

for (const surface of SURFACES) {
  const packs = packsBySurface.get(surface) ?? [];
  const languages = new Set(packs.map((pack) => pack.language));
  if (packs.length !== localeTarget.size) {
    fail(`language-packs/catalog.json: ${surface} must ship exactly ${localeTarget.size} mainstream language packs`);
  }
  if (languages.size !== packs.length) {
    fail(`language-packs/catalog.json: ${surface} language packs must use unique language tags`);
  }
  for (const [language, locale] of localeTarget) {
    const pack = packs.find((entry) => entry.language === language);
    if (!pack) {
      fail(`language-packs/catalog.json: ${surface} is missing mainstream language ${language}`);
      continue;
    }
    const expectedName = `${locale.englishName} ${surface === "hub" ? "Hub" : "Workbench"} Core`;
    if (pack.name !== expectedName) {
      fail(`language-packs/${surface}/${language}: name must be ${expectedName}`);
    }
  }

  for (const discoveredPath of discoverPackFiles(surface)) {
    if (!referencedPaths.has(discoveredPath)) {
      fail(`language-packs/${discoveredPath}: discovered pack is not listed in catalog.json`);
    }
  }
}

if (errors.length > 0) {
  console.error("Language pack validation failed:");
  errors.forEach((error) => console.error(`- ${error}`));
  process.exit(1);
}

const coverageSummary = [...coverageTotals.entries()]
  .map(([surface, total]) => `${surface} ${total.covered}/${total.required}`)
  .join(", ");

console.log(
  `Validated ${validatedPacks.length} language packs for ${VERSION_LINE} ${TARGET_APP_VERSION}; coverage ${coverageSummary}.`,
);
