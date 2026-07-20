#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const endpoint = option("--endpoint") ?? "https://translate.googleapis.com/translate_a/single";
const targetLanguage = option("--language");
const targetBatch = option("--batch");
const maxBatches = Number(option("--max-batches")) || Infinity;
const attempts = Number(option("--attempts")) || 20;
const pauseBetweenEntriesMs = Number(option("--pause-ms")) || 250;
const dryRun = process.argv.includes("--dry-run");
const continueOnError = process.argv.includes("--continue-on-error");

function option(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}

function runCommand(args, failureMessage) {
  const result = spawnSync(process.execPath, args, { stdio: "inherit", cwd: root });
  if (result.status !== 0) {
    throw new Error(failureMessage);
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const reportOutput = path.join(root, "tmp/language-pack-full-coverage.json");
runCommand([
  path.join(root, "scripts/report-full-language-pack-coverage.mjs"),
]);
const report = JSON.parse(fs.readFileSync(reportOutput, "utf8"));

const batchCoverage = report.batches.flatMap((batch) => {
  const coverageByLanguage = new Map(
    batch.coverage.map((entry) => [entry.language, entry]),
  );
  return report.rows.map((row) => {
    const covered = coverageByLanguage.get(row.language)?.covered ?? 0;
    const remaining = batch.required - covered;
    if (remaining <= 0) return null;
    if (targetLanguage && row.language !== targetLanguage) return null;
    if (targetBatch && batch.id !== targetBatch) return null;
    return {
      language: row.language,
      batch: batch.id,
      covered,
      remaining,
      required: batch.required,
    };
  }).filter(Boolean);
});

batchCoverage.sort((a, b) => {
  if (a.remaining !== b.remaining) return b.remaining - a.remaining;
  if (a.language !== b.language) return a.language.localeCompare(b.language);
  return a.batch.localeCompare(b.batch);
});

if (batchCoverage.length === 0) {
  console.log("no missing language packs detected for current coverage filter");
  process.exit(0);
}

if (dryRun) {
  console.log(`dry-run: ${Math.min(maxBatches, batchCoverage.length)} batches pending`);
  for (const item of batchCoverage.slice(0, maxBatches)) {
    console.log(`${item.language} ${item.batch} remaining ${item.remaining}/${item.required}`);
  }
  process.exit(0);
}

let completed = 0;
let failed = 0;
for (const item of batchCoverage.slice(0, maxBatches)) {
  const templatePath = path.join(root, "tmp", `language-pack-translation-${item.language}-${item.batch}.json`);
  try {
    runCommand([
      path.join(root, "scripts/report-full-language-pack-coverage.mjs"),
      "--batch", item.batch,
      "--language", item.language,
      "--template-out", templatePath,
    ], `failed to export template for ${item.language}/${item.batch}`);

    runCommand([
      path.join(root, "scripts/draft-language-pack-machine-translations.mjs"),
      "--in", templatePath,
      "--out", templatePath,
      "--target", item.language,
      "--force",
      "--attempts", String(attempts),
      "--pause-ms", String(pauseBetweenEntriesMs),
      ...(endpoint ? ["--endpoint", endpoint] : []),
    ], `translation failed for ${item.language}/${item.batch}`);

    runCommand([
      path.join(root, "scripts/report-full-language-pack-coverage.mjs"),
      "--apply-from", templatePath,
    ], `failed to apply ${item.language}/${item.batch}`);

    completed += 1;
    console.log(`applied ${item.language} ${item.batch} (${completed}/${Math.min(maxBatches, batchCoverage.length)})`);
  } catch (error) {
    failed += 1;
    console.error(error.message || error);
    if (!continueOnError) throw error;
  }
  await sleep(0);
}

console.log(`translation queue: total=${batchCoverage.length}, applied=${completed}, failed=${failed}`);
if (failed > 0) process.exit(1);
