#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const language = optionValue("--language");
const nextOnly = process.argv.includes("--next");
const jsonOnly = process.argv.includes("--json");

function optionValue(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

execFileSync(process.execPath, ["./scripts/report-full-language-pack-coverage.mjs"], {
  cwd: root,
  stdio: "ignore",
});

const report = JSON.parse(fs.readFileSync(path.join(root, "tmp/language-pack-full-coverage.json"), "utf8"));
const rows = language ? report.rows.filter((row) => row.language === language) : report.rows;
if (language && rows.length === 0) fail(`unknown Workbench language: ${language}`);

const queue = rows
  .flatMap((row) => report.batches.map((batch, order) => {
    const coverage = batch.coverage.find((entry) => entry.language === row.language)?.covered ?? 0;
    return {
      language: row.language,
      batch: batch.id,
      order,
      covered: coverage,
      required: batch.required,
      remaining: batch.required - coverage,
      draft: `tmp/language-pack-translation-drafts/${row.language}-${batch.id}.json`,
      template: `tmp/language-pack-translation-batches/${row.language}-${batch.id}.json`,
    };
  }))
  .filter((entry) => entry.remaining > 0)
  .sort((left, right) => right.remaining - left.remaining || left.language.localeCompare(right.language) || left.order - right.order);

const plan = {
  schema_version: "kyuubiki.language-pack-translation-plan/v1",
  generatedAt: new Date().toISOString(),
  requiredKeys: report.required_keys.length,
  completeLanguages: report.rows.filter((row) => row.covered === row.required).map((row) => row.language),
  incompleteLanguages: report.rows.filter((row) => row.covered !== row.required).map((row) => ({
    language: row.language,
    covered: row.covered,
    required: row.required,
    remaining: row.required - row.covered,
  })),
  queue,
};

const output = path.join(root, "tmp/language-pack-translation-plan.json");
fs.writeFileSync(output, `${JSON.stringify(plan, null, 2)}\n`);

if (nextOnly) {
  const next = queue[0] ?? null;
  if (jsonOnly) console.log(JSON.stringify(next, null, 2));
  else if (next) console.log(`${next.language} ${next.batch}: ${next.covered}/${next.required}; draft ${next.draft}`);
  else console.log("all Workbench language packs are complete");
  process.exit(0);
}

if (jsonOnly) {
  console.log(JSON.stringify(plan, null, 2));
  process.exit(0);
}

console.log(`language translation queue: ${queue.length} incomplete batches across ${plan.incompleteLanguages.length} languages`);
console.log(`complete languages: ${plan.completeLanguages.join(", ") || "none"}`);
for (const entry of queue) console.log(`${entry.language} ${entry.batch}: ${entry.covered}/${entry.required} (${entry.remaining} remaining)`);
