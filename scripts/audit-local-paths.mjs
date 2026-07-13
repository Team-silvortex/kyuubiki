#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");

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
  ".service",
  ".sh",
  ".swift",
  ".toml",
  ".ts",
  ".tsx",
  ".yaml",
  ".yml",
  ".zsh",
]);

const IGNORED_PATH_PATTERNS = [
  /(^|\/)node_modules\//,
  /(^|\/)target\//,
  /(^|\/)_build\//,
  /(^|\/)\.next\//,
  /(^|\/)dist\//,
  /(^|\/)build\//,
];

const LOCAL_PATH_PATTERNS = [
  ["", "Users", ""],
  ["", "Users", "Shared", "chroot", "dev", "kyuubiki"],
  ["", "var", "folders", ""],
  ["", "private", "var", ""],
  ["", "Volumes", ""],
  ["", "home", "kyuubiki-dev", ""],
  ["", "opt", "homebrew", ""],
].map((parts) => new RegExp(parts.join("\\/")));

function gitFiles() {
  return execFileSync("git", ["ls-files"], {
    cwd: ROOT,
    encoding: "utf8",
  })
    .split("\n")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function shouldCheck(relativePath) {
  if (IGNORED_PATH_PATTERNS.some((pattern) => pattern.test(relativePath))) {
    return false;
  }
  return CHECKED_EXTENSIONS.has(path.extname(relativePath));
}

const violations = [];

for (const relativePath of gitFiles()) {
  if (!shouldCheck(relativePath)) continue;

  const absolutePath = path.join(ROOT, relativePath);
  if (!existsSync(absolutePath)) continue;

  const contents = readFileSync(absolutePath, "utf8");
  contents.split("\n").forEach((line, index) => {
    const pattern = LOCAL_PATH_PATTERNS.find((candidate) => candidate.test(line));
    if (pattern) {
      violations.push({
        relativePath,
        lineNumber: index + 1,
        pattern: pattern.source,
        line: line.trim(),
      });
    }
  });
}

if (violations.length > 0) {
  const formatted = violations
    .map(
      (violation) =>
        `${violation.relativePath}:${violation.lineNumber}: ${violation.line}`,
    )
    .join("\n");
  console.error(`Local absolute path audit failed:\n${formatted}`);
  process.exit(1);
}

console.log("Local absolute path audit passed.");
