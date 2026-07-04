#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const DEFAULT_MAX_LINES = Number(process.env.MAX_LINES || 600);

const CHECKED_EXTENSIONS = new Set([
  ".css",
  ".ex",
  ".exs",
  ".html",
  ".js",
  ".json",
  ".jsx",
  ".md",
  ".mjs",
  ".py",
  ".rs",
  ".sh",
  ".swift",
  ".ts",
  ".tsx",
  ".zsh",
]);

const IGNORED_PATH_PATTERNS = [
  /(^|\/)gen\/schemas\//,
  /(^|\/)package-lock\.json$/,
  /^assets\/icons\//,
  /^releases\/update-catalog\.json$/,
  /^apps\/frontend\/public\//,
  /^apps\/[^/]+\/ui\/assets\//,
  /^apps\/[^/]+\/src-tauri\/icons\//,
  /^workers\/rust\/benchmarks\/.*\.json$/,
];

const TRACKED_DEBT_LIMITS = new Map([]);

function gitFileList(args) {
  return execFileSync("git", args, {
    cwd: ROOT,
    encoding: "utf8",
  })
    .split("\n")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function projectFiles() {
  return [
    ...new Set([
      ...gitFileList(["ls-files"]),
      ...gitFileList(["ls-files", "--others", "--exclude-standard"]),
    ]),
  ].sort();
}

function shouldCheck(relativePath) {
  const extension = path.extname(relativePath);
  if (!CHECKED_EXTENSIONS.has(extension)) return false;
  return !IGNORED_PATH_PATTERNS.some((pattern) => pattern.test(relativePath));
}

function lineCount(relativePath) {
  const contents = readFileSync(path.join(ROOT, relativePath), "utf8");
  if (contents.length === 0) return 0;
  return contents.split("\n").length;
}

const violations = [];
const debtFilesSeen = new Set();

for (const relativePath of projectFiles()) {
  if (!shouldCheck(relativePath)) continue;

  const lines = lineCount(relativePath);
  const debtLimit = TRACKED_DEBT_LIMITS.get(relativePath);
  if (debtLimit !== undefined) debtFilesSeen.add(relativePath);

  const limit = debtLimit ?? DEFAULT_MAX_LINES;
  if (lines > limit) {
    violations.push({
      relativePath,
      lines,
      limit,
      debtTracked: debtLimit !== undefined,
    });
  }
}

function checkInstallerTestIndex() {
  const relativePath = "workers/rust/crates/installer/src/tests.rs";
  const absolutePath = path.join(ROOT, relativePath);
  if (!existsSync(absolutePath)) return;

  const lines = readFileSync(absolutePath, "utf8").split("\n");
  for (const [index, line] of lines.entries()) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("//")) continue;
    const match = trimmed.match(/^mod ([a-z0-9_]+);$/);
    if (!match) {
      violations.push({
        relativePath,
        lines: index + 1,
        limit: "module-index-only",
        customMessage:
          "installer tests.rs should only declare test modules; put tests in workers/rust/crates/installer/src/tests/",
      });
      return;
    }

    const modulePath = path.join(
      ROOT,
      "workers/rust/crates/installer/src/tests",
      `${match[1]}.rs`,
    );
    if (!existsSync(modulePath)) {
      violations.push({
        relativePath,
        lines: index + 1,
        limit: "module-file-required",
        customMessage: `installer test module ${match[1]} is missing ${path.relative(
          ROOT,
          modulePath,
        )}`,
      });
      return;
    }
  }
}

checkInstallerTestIndex();

for (const relativePath of TRACKED_DEBT_LIMITS.keys()) {
  if (!debtFilesSeen.has(relativePath)) {
    violations.push({
      relativePath,
      lines: 0,
      limit: TRACKED_DEBT_LIMITS.get(relativePath),
      debtTracked: true,
      missingDebtFile: true,
    });
  }
}

if (violations.length > 0) {
  const formatted = violations
    .sort((left, right) => right.lines - left.lines)
    .map((violation) => {
      if (violation.missingDebtFile) {
        return `${violation.relativePath}: debt guard references a missing file`;
      }
      if (violation.customMessage) {
        return `${violation.relativePath}: ${violation.customMessage}`;
      }
      return `${violation.relativePath}: ${violation.lines} lines (limit ${violation.limit}${
        violation.debtTracked ? ", tracked debt" : ""
      })`;
    })
    .join("\n");

  console.error(
    [
      `Project organization audit failed. Default source limit is ${DEFAULT_MAX_LINES} lines.`,
      "Split new oversized files, or lower existing tracked debt after refactoring.",
      formatted,
    ].join("\n\n"),
  );
  process.exit(1);
}

console.log(
  `Project organization audit passed. Default source limit ${DEFAULT_MAX_LINES}; tracked debt ${TRACKED_DEBT_LIMITS.size}.`,
);
