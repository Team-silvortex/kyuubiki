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
const VERSION_LINE = "tamamono 1.x";
const TARGET_APP_VERSION = "1.14.0";
const SOURCE_VALUES = new Set(["downloaded", "imported"]);
const SURFACES = new Set(["workbench", "hub"]);

const errors = [];

function fail(message) {
  errors.push(message);
}

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function readJson(relativePath) {
  const absolutePath = path.join(repoRoot, relativePath);
  try {
    return JSON.parse(fs.readFileSync(absolutePath, "utf8"));
  } catch (error) {
    fail(`${relativePath}: ${error.message}`);
    return null;
  }
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

function validatePack(relativePath, expectedSurface) {
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
const referencedPaths = new Set();
const seenIds = new Set();
const validatedPacks = [];

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
      referencedPaths.add(entry.path);

      const pack = validatePack(entry.path, entry.surface);
      if (!pack) return;
      validatedPacks.push(pack);
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
    "--import",
    "./test/support/register-alias-loader.mjs",
    "--test",
    "--experimental-strip-types",
    "./test/workflow/workbench-language-pack-catalog.test.ts",
  ],
  {
    cwd: path.join(repoRoot, "apps/frontend"),
    encoding: "utf8",
  },
);

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

console.log(`Validated ${validatedPacks.length} language packs for ${VERSION_LINE} ${TARGET_APP_VERSION}.`);
