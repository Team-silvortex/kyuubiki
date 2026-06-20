#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_MATRICES = ["mechanical-core", "thermal-core", "compound-core"];
const DEFAULT_REPORTS_DIR = path.resolve("workers/rust/benchmarks/reports");

function parseArgs(argv) {
  const options = {
    profile: "10k",
    reportsDir: DEFAULT_REPORTS_DIR,
    output: "",
    matrices: [...DEFAULT_MATRICES],
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];

    if (arg === "--profile" && next) {
      options.profile = next;
      index += 1;
      continue;
    }

    if (arg === "--reports-dir" && next) {
      options.reportsDir = path.resolve(next);
      index += 1;
      continue;
    }

    if (arg === "--output" && next) {
      options.output = path.resolve(next);
      index += 1;
      continue;
    }

    if (arg === "--matrices" && next) {
      options.matrices = next
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);
      index += 1;
      continue;
    }
  }

  if (!options.output) {
    options.output = path.resolve(
      options.reportsDir,
      `standard-${options.profile}-compare.md`,
    );
  }

  return options;
}

async function loadReport(reportPath) {
  const content = await readFile(reportPath, "utf8");
  const lines = content.split("\n");
  const bodyStart = lines.findIndex((line) => line.startsWith("- Profile:"));
  const body = bodyStart >= 0 ? lines.slice(bodyStart).join("\n").trim() : content.trim();
  return body;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const sections = [];

  for (const matrix of options.matrices) {
    const reportPath = path.resolve(
      options.reportsDir,
      `${matrix}-${options.profile}-compare.md`,
    );
    const body = await loadReport(reportPath);
    sections.push({
      matrix,
      reportPath,
      body,
    });
  }

  const lines = [
    "# Kyuubiki Standard Benchmark Comparison",
    "",
    `- Profile: \`${options.profile}\``,
    `- Matrices: ${options.matrices.map((matrix) => `\`${matrix}\``).join(", ")}`,
    `- Reports directory: \`${path.relative(process.cwd(), options.reportsDir) || "."}\``,
    "",
    "## Included reports",
    "",
    ...sections.map(
      (section) =>
        `- \`${section.matrix}\`: \`${path.relative(process.cwd(), section.reportPath)}\``,
    ),
    "",
  ];

  for (const section of sections) {
    lines.push(`## ${section.matrix}`);
    lines.push("");
    lines.push(section.body);
    lines.push("");
  }

  await writeFile(options.output, `${lines.join("\n").trimEnd()}\n`);
  process.stdout.write(`${path.relative(process.cwd(), options.output)}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
