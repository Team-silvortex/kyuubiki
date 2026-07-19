#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packsDirectory = path.join(root, "language-packs/workbench");
const outputPath = path.join(
  root,
  "apps/frontend/src/components/workbench/workbench-language-pack-catalog-data.ts",
);
const unsafeTextPatterns = ["<", ">", "javascript:", "data:text/html", "onerror=", "onclick=", "innerhtml", "document.cookie", "localstorage", "eval("];

function isPlainObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function mergeOverrides(base, next) {
  for (const [key, value] of Object.entries(next)) {
    if (isPlainObject(value) && isPlainObject(base[key])) mergeOverrides(base[key], value);
    else base[key] = value;
  }
  return base;
}

function safeFragmentPath(relativePath) {
  if (typeof relativePath !== "string" || !relativePath || path.isAbsolute(relativePath)) return false;
  const normalized = relativePath.replaceAll("\\", "/");
  return !normalized.split("/").includes("..");
}

function hasUnsafeText(value) {
  if (typeof value === "string") return unsafeTextPatterns.some((pattern) => value.toLowerCase().includes(pattern));
  if (Array.isArray(value)) return value.some(hasUnsafeText);
  return isPlainObject(value) && Object.values(value).some(hasUnsafeText);
}

function readFragment(pack, fragment, fragmentIds) {
  if (!isPlainObject(fragment) || typeof fragment.batch !== "string" || !safeFragmentPath(fragment.path)) {
    throw new Error(`invalid language-pack fragment declaration for ${pack.language}`);
  }
  if (fragmentIds.has(fragment.batch)) throw new Error(`duplicate ${pack.language} language-pack fragment ${fragment.batch}`);
  fragmentIds.add(fragment.batch);
  const absolutePath = path.resolve(packsDirectory, fragment.path);
  if (!absolutePath.startsWith(`${packsDirectory}${path.sep}`)) throw new Error(`fragment path escapes language packs: ${fragment.path}`);
  const payload = JSON.parse(fs.readFileSync(absolutePath, "utf8"));
  if (!isPlainObject(payload.overrides) || payload.schema_version !== "kyuubiki.language-pack-fragment/v1") {
    throw new Error(`invalid language-pack fragment ${fragment.path}`);
  }
  if (payload.language !== pack.language || payload.targetSurface !== "workbench" || payload.batch !== fragment.batch) {
    throw new Error(`fragment identity mismatch: ${fragment.path}`);
  }
  if (typeof payload.updatedAt !== "string" || Number.isNaN(Date.parse(payload.updatedAt)) || hasUnsafeText(payload)) {
    throw new Error(`unsafe or stale language-pack fragment ${fragment.path}`);
  }
  return payload.overrides;
}

function readWorkbenchPacks() {
  return fs.readdirSync(packsDirectory)
    .filter((name) => name.endsWith(".json"))
    .sort()
    .map((name) => JSON.parse(fs.readFileSync(path.join(packsDirectory, name), "utf8")))
    .map((pack) => {
      if (!isPlainObject(pack.overrides) || (pack.fragments !== undefined && !Array.isArray(pack.fragments))) {
        throw new Error(`invalid language-pack root for ${pack.language ?? "unknown"}`);
      }
      const fragmentIds = new Set();
      const overrides = mergeOverrides({}, pack.overrides ?? {});
      for (const fragment of pack.fragments ?? []) mergeOverrides(overrides, readFragment(pack, fragment, fragmentIds));
      return [pack.language, overrides];
    });
}

function renderCatalog(packs) {
  return [
    "// Generated from language-packs/workbench/*.json. Do not edit by hand.",
    "export const WORKBENCH_TRANSLATED_LANGUAGE_PACK_OVERRIDES: Record<string, Record<string, unknown>> = {",
    ...packs.map(([language, overrides]) => `  ${JSON.stringify(language)}: ${JSON.stringify(overrides)},`),
    "};",
    "",
  ].join("\n");
}

const expected = renderCatalog(readWorkbenchPacks());
if (process.argv.includes("--check")) {
  const actual = fs.readFileSync(outputPath, "utf8");
  if (actual !== expected) {
    throw new Error("Workbench language-pack catalog is stale; run make build-workbench-language-pack-catalog");
  }
  console.log("Workbench language-pack catalog is synchronized.");
} else {
  fs.writeFileSync(outputPath, expected);
  console.log("Built Workbench language-pack catalog from language-packs/workbench.");
}
