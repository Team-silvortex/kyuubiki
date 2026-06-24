#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { rootDir } from "./release-metadata.mjs";

const DEFAULT_ROOT = "workers/rust/crates";
const DEFAULT_MAX_LINES = 600;
const IGNORED_DIRS = new Set([".git", "target", "node_modules", ".next", "dist", "build"]);

function usage() {
  console.log(`Usage:
  node ./scripts/audit-rust-line-counts.mjs [--root workers/rust/crates] [--max 600] [--json]

Examples:
  node ./scripts/audit-rust-line-counts.mjs
  node ./scripts/audit-rust-line-counts.mjs --max 600 --json
`);
}

function parseArgs(argv) {
  const options = {
    root: DEFAULT_ROOT,
    maxLines: DEFAULT_MAX_LINES,
    json: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];

    if (value === "--help" || value === "-h") {
      usage();
      process.exit(0);
    }

    if (value === "--json") {
      options.json = true;
      continue;
    }

    if (value === "--root") {
      options.root = argv[index + 1] ?? options.root;
      index += 1;
      continue;
    }

    if (value === "--max") {
      const parsed = Number.parseInt(argv[index + 1] ?? "", 10);
      if (!Number.isFinite(parsed) || parsed <= 0) {
        throw new Error(`Invalid --max value: ${argv[index + 1] ?? ""}`);
      }
      options.maxLines = parsed;
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${value}`);
  }

  return options;
}

function walkRustFiles(relativeRoot, results = []) {
  const absoluteRoot = path.join(rootDir, relativeRoot);
  const entries = fs.readdirSync(absoluteRoot, { withFileTypes: true });

  for (const entry of entries) {
    if (IGNORED_DIRS.has(entry.name)) {
      continue;
    }

    const nextRelative = path.join(relativeRoot, entry.name);
    if (entry.isDirectory()) {
      walkRustFiles(nextRelative, results);
      continue;
    }

    if (entry.isFile() && entry.name.endsWith(".rs")) {
      results.push(nextRelative);
    }
  }

  return results;
}

function lineCount(relativePath) {
  const text = fs.readFileSync(path.join(rootDir, relativePath), "utf8");
  if (text.length === 0) {
    return 0;
  }
  return text.endsWith("\n") ? text.split("\n").length - 1 : text.split("\n").length;
}

function audit(options) {
  const files = walkRustFiles(options.root)
    .map((file) => ({ file, lines: lineCount(file) }))
    .sort((left, right) => right.lines - left.lines || left.file.localeCompare(right.file));

  return {
    root: options.root,
    maxLines: options.maxLines,
    checkedFiles: files.length,
    maximum: files[0] ?? null,
    violations: files.filter((entry) => entry.lines > options.maxLines),
  };
}

function printText(result) {
  if (result.violations.length === 0) {
    console.log(
      `Rust line-count audit passed: ${result.checkedFiles} files, max ${result.maximum?.lines ?? 0}/${result.maxLines} lines (${result.maximum?.file ?? "none"}).`,
    );
    return;
  }

  console.error(
    `Rust line-count audit failed: ${result.violations.length} file(s) exceed ${result.maxLines} lines.`,
  );
  for (const violation of result.violations) {
    console.error(`- ${violation.lines} ${violation.file}`);
  }
}

try {
  const options = parseArgs(process.argv.slice(2));
  const result = audit(options);

  if (options.json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    printText(result);
  }

  process.exit(result.violations.length === 0 ? 0 : 1);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
