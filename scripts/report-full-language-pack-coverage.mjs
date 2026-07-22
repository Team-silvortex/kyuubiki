#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "../apps/frontend/node_modules/typescript/lib/typescript.js";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sources = [
  "apps/frontend/src/components/workbench/workbench-copy-en-core.ts",
  "apps/frontend/src/components/workbench/workbench-copy-en-extended.ts",
];

function propertyName(name) {
  return ts.isIdentifier(name) || ts.isStringLiteral(name) ? name.text : null;
}

function collectObjectEntries(node, prefix = "", output = []) {
  if (!ts.isObjectLiteralExpression(node)) return output;
  for (const property of node.properties) {
    if (!ts.isPropertyAssignment(property)) continue;
    const name = propertyName(property.name);
    if (!name) continue;
    const key = prefix ? `${prefix}.${name}` : name;
    let initializer = property.initializer;
    while (ts.isAsExpression(initializer) || ts.isParenthesizedExpression(initializer)) {
      initializer = initializer.expression;
    }
    if (ts.isObjectLiteralExpression(initializer)) {
      collectObjectEntries(initializer, key, output);
    } else if (ts.isStringLiteral(initializer) || ts.isNoSubstitutionTemplateLiteral(initializer)) {
      output.push({ key, value: initializer.text });
    } else if (ts.isArrayLiteralExpression(initializer) && initializer.elements.every(
      (element) => ts.isStringLiteral(element) || ts.isNoSubstitutionTemplateLiteral(element),
    )) {
      output.push({ key, value: initializer.elements.map((element) => element.text) });
    }
  }
  return output;
}

function collectSourceKeys(relativePath) {
  const text = fs.readFileSync(path.join(root, relativePath), "utf8");
  const source = ts.createSourceFile(relativePath, text, ts.ScriptTarget.ES2022, true);
  const entries = [];
  source.forEachChild((node) => {
    if (!ts.isVariableStatement(node)) return;
    for (const declaration of node.declarationList.declarations) {
      let initializer = declaration.initializer;
      while (initializer && (ts.isAsExpression(initializer) || ts.isParenthesizedExpression(initializer))) {
        initializer = initializer.expression;
      }
      if (initializer && ts.isObjectLiteralExpression(initializer)) {
        collectObjectEntries(initializer, "", entries);
      }
    }
  });
  const strings = Object.fromEntries(entries.map((entry) => [entry.key, entry.value]));
  return { relativePath, keys: Object.keys(strings).sort(), strings };
}

function valueAtPath(value, dottedPath) {
  return dottedPath.split(".").reduce(
    (current, part) => current && typeof current === "object" ? current[part] : undefined,
    value,
  );
}

function hasTranslation(value, source) {
  if (typeof source === "string") return typeof value === "string" && value.trim().length > 0;
  return Array.isArray(value)
    && value.length === source.length
    && value.every((entry) => typeof entry === "string" && entry.trim().length > 0);
}

function isLocaleInvariantText(text) {
  return typeof text === "string" && /\d/.test(text);
}

function isSourceTranslation(value, source) {
  if (typeof source === "string" && isLocaleInvariantText(source)) return false;
  if (!hasTranslation(value, source)) return false;
  if (typeof source === "string") return value === source;
  return Array.isArray(value) && value.length === source.length && value.every((entry, index) => entry === source[index]);
}

function hasMeaningfulTranslation(value, source) {
  if (typeof source === "string" && /\d/.test(source)) {
    return hasTranslation(value, source);
  }
  return hasTranslation(value, source) && !isSourceTranslation(value, source);
}

function setValueAtPath(value, dottedPath, nextValue) {
  const parts = dottedPath.split(".");
  const last = parts.pop();
  const parent = parts.reduce((current, part) => (current[part] ??= {}), value);
  parent[last] = nextValue;
}

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

function loadWorkbenchPack(name) {
  const packPath = path.join(root, "language-packs/workbench", name);
  const pack = JSON.parse(fs.readFileSync(packPath, "utf8"));
  const overrides = mergeOverrides({}, pack.overrides ?? {});
  const fragments = (pack.fragments ?? []).map((fragment) => {
    if (!isPlainObject(fragment) || typeof fragment.batch !== "string" || typeof fragment.path !== "string") {
      throw new Error(`invalid fragment declaration in ${name}`);
    }
    const fragmentPath = path.resolve(path.dirname(packPath), fragment.path);
    if (!fragmentPath.startsWith(`${path.dirname(packPath)}${path.sep}`)) {
      throw new Error(`fragment path escapes language pack directory: ${fragment.path}`);
    }
    const payload = JSON.parse(fs.readFileSync(fragmentPath, "utf8"));
    if (payload.schema_version !== "kyuubiki.language-pack-fragment/v1" || payload.language !== pack.language
      || payload.targetSurface !== "workbench" || payload.batch !== fragment.batch || !isPlainObject(payload.overrides)) {
      throw new Error(`invalid language-pack fragment ${fragment.path}`);
    }
    mergeOverrides(overrides, payload.overrides);
    return { batch: fragment.batch, path: fragmentPath, payload };
  });
  return { pack, packPath, overrides, fragments };
}

const sourceKeySets = sources.map(collectSourceKeys);
const requiredKeys = [...new Set(sourceKeySets.flatMap((source) => source.keys))].sort();
const sourceStrings = Object.assign({}, ...sourceKeySets.map((source) => source.strings));

function optionValue(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}
const packs = fs.readdirSync(path.join(root, "language-packs/workbench"))
  .filter((name) => name.endsWith(".json"))
  .sort()
  .map(loadWorkbenchPack);
const rows = packs
  .map((entry) => {
    const missing = requiredKeys.filter((key) => !hasTranslation(valueAtPath(entry.overrides, key), sourceStrings[key]));
    const meaningful = requiredKeys.filter((key) => hasMeaningfulTranslation(valueAtPath(entry.overrides, key), sourceStrings[key]));
    const sourceMatchedCount = requiredKeys.filter((key) => isSourceTranslation(valueAtPath(entry.overrides, key), sourceStrings[key])).length;
    return {
      language: entry.pack.language,
      covered: meaningful.length,
      meaningful,
      sourceMatchedCount,
      missingOrSourceMatch: requiredKeys.length - meaningful.length,
      required: requiredKeys.length,
      percent: Number(((meaningful.length / requiredKeys.length) * 100).toFixed(1)),
      percentRaw: Number((((requiredKeys.length - missing.length) / requiredKeys.length) * 100).toFixed(1)),
      missing,
    };
  });

const batches = sourceKeySets.flatMap(({ relativePath, keys }) => keys
  .reduce((result, key, index) => {
    const batchIndex = Math.floor(index / 100);
    result[batchIndex] ??= [];
    result[batchIndex].push(key);
    return result;
  }, [])
  .map((batchKeys, index) => ({
    id: `${path.basename(relativePath, ".ts").replace("workbench-copy-en-", "")}-${String(index + 1).padStart(2, "0")}`,
    source: relativePath,
    required: batchKeys.length,
    keys: batchKeys,
    coverage: rows.map((row) => ({
      language: row.language,
      covered: batchKeys.filter((key) => row.meaningful.includes(key)).length,
    })),
  }))); 
const complete = rows.every((row) => row.covered === row.required);
const report = {
  schema_version: "kyuubiki.language-pack-full-coverage/v1",
  sources,
  required_keys: requiredKeys,
  complete,
  batches,
  rows,
};
const output = path.join(root, "tmp/language-pack-full-coverage.json");
const markdownOutput = path.join(root, "tmp/language-pack-full-coverage.md");
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
const markdown = [
  "# Full Language-Pack Coverage",
  "",
  `Status: **${complete ? "complete" : "incomplete"}**. Each shipped Workbench language pack must provide a real override for all ${requiredKeys.length} visible-copy keys before it can claim full coverage.`,
  "",
  "## Language Coverage",
  "",
  "| Language | Covered | Source-matches | Required | Coverage |",
  "| --- | ---: | ---: | ---: | ---: |",
  ...rows.map((row) => `| ${row.language} | ${row.covered} | ${row.sourceMatchedCount} | ${row.required} | ${row.percent}% |`),
  "",
  "## Delivery Batches",
  "",
  "| Batch | Source | Keys | Lowest coverage |",
  "| --- | --- | ---: | ---: |",
  ...batches.map((batch) => {
    const minimum = Math.min(...batch.coverage.map((entry) => entry.covered));
    return `| ${batch.id} | ${path.basename(batch.source)} | ${batch.required} | ${minimum}/${batch.required} |`;
  }),
  "",
].join("\n");
fs.writeFileSync(markdownOutput, markdown);
console.log(`full Workbench language-pack coverage: ${requiredKeys.length} keys; ${complete ? "complete" : "incomplete"}`);
console.log("reports: tmp/language-pack-full-coverage.json, tmp/language-pack-full-coverage.md");
const batchId = optionValue("--batch");
const language = optionValue("--language");
const templateOutput = optionValue("--template-out");
const templateOptionsRequested = ["--batch", "--language", "--template-out"].some((option) => process.argv.includes(option));
if (templateOptionsRequested && ![batchId, language, templateOutput].every((value) => value)) {
  throw new Error("--batch, --language, and --template-out must be provided together");
}
if (batchId && language && templateOutput) {
  const batch = batches.find((entry) => entry.id === batchId);
  const row = rows.find((entry) => entry.language === language);
  if (!batch) throw new Error(`unknown translation batch: ${batchId}`);
  if (!row) throw new Error(`unknown Workbench language: ${language}`);
  const pack = packs.find((entry) => entry.pack.language === language);
  const template = {
    schema_version: "kyuubiki.language-pack-translation-batch/v1",
    language,
    batch: batch.id,
    source: batch.source,
    strings: batch.keys.map((key) => ({
      key,
      source: sourceStrings[key],
      translation: valueAtPath(pack.overrides, key) ?? (Array.isArray(sourceStrings[key]) ? [] : ""),
    })),
  };
  const absoluteTemplateOutput = path.resolve(root, templateOutput);
  fs.mkdirSync(path.dirname(absoluteTemplateOutput), { recursive: true });
  fs.writeFileSync(absoluteTemplateOutput, `${JSON.stringify(template, null, 2)}\n`);
  console.log(`translation batch template: ${path.relative(root, absoluteTemplateOutput)}`);
}
const applyFrom = optionValue("--apply-from");
if (process.argv.includes("--apply-from") && !applyFrom) {
  throw new Error("--apply-from requires a translation batch path");
}
if (applyFrom) {
  const input = JSON.parse(fs.readFileSync(path.resolve(root, applyFrom), "utf8"));
  const batch = batches.find((entry) => entry.id === input.batch);
  const row = rows.find((entry) => entry.language === input.language);
  if (input.schema_version !== "kyuubiki.language-pack-translation-batch/v1") {
    throw new Error("translation batch schema is not supported");
  }
  if (!batch || !row || input.source !== batch.source || !Array.isArray(input.strings)) {
    throw new Error("translation batch does not match a shipped Workbench pack");
  }
  const inputByKey = new Map(input.strings.map((entry) => [entry.key, entry]));
  if (inputByKey.size !== batch.keys.length || batch.keys.some((key) => !inputByKey.has(key))) {
    throw new Error("translation batch keys must exactly match its declared batch");
  }
  for (const key of batch.keys) {
    const entry = inputByKey.get(key);
    if (JSON.stringify(entry.source) !== JSON.stringify(sourceStrings[key])) {
      throw new Error(`translation batch source drift for ${key}`);
    }
    if (!hasMeaningfulTranslation(entry.translation, sourceStrings[key])) {
      throw new Error(`translation is missing or has the wrong shape for ${key}`);
    }
  }
  const entry = packs.find((candidate) => candidate.pack.language === input.language);
  const fragment = entry.fragments.find((candidate) => candidate.batch === batch.id);
  const target = fragment?.payload ?? entry.pack;
  const targetPath = fragment?.path ?? entry.packPath;
  for (const key of batch.keys) setValueAtPath(target.overrides, key, inputByKey.get(key).translation);
  target.updatedAt = new Date().toISOString();
  fs.writeFileSync(targetPath, `${JSON.stringify(target, null, 2)}\n`);
  console.log(`applied ${batch.id} translations to ${path.relative(root, targetPath)}`);
}
const strictLanguage = optionValue("--strict-language");
if (process.argv.includes("--strict-language") && !strictLanguage) {
  throw new Error("--strict-language requires a Workbench language code");
}
const languageComplete = strictLanguage && rows.find((row) => row.language === strictLanguage)?.covered === requiredKeys.length;
if (strictLanguage && !languageComplete) {
  console.error(`full language-pack coverage is incomplete for ${strictLanguage}`);
  process.exitCode = 1;
}
if (process.argv.includes("--strict") && !complete) {
  console.error("full language-pack coverage is incomplete");
  process.exitCode = 1;
}
