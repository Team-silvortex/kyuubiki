#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
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

const TRACKED_DEBT_LIMITS = new Map([
  ["scripts/kyuubiki-legacy.zsh", 1818],
  ["apps/hub-gui/ui/index.html", 1057],
  ["apps/web/lib/kyuubiki_web/workflow_template_bridge_contract_graphs.ex", 896],
  ["apps/web/lib/kyuubiki_web/playground/agent_pool.ex", 887],
  ["apps/web/lib/kyuubiki_web/playground/agent_registry.ex", 768],
  ["apps/web/lib/kyuubiki_web/workflow_template_electromagnetic_guard_thermo_entries.ex", 727],
  ["apps/web/test/kyuubiki_web/workflow_operator_runtime_test.exs", 722],
  ["apps/web/test/kyuubiki_web/api/workflow_catalog_api_test.exs", 673],
  ["apps/web/test/support/workflow_api_fixtures.exs", 656],
  ["apps/web/test/kyuubiki_web/workflow_template_catalog_test.exs", 601],
]);

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

for (const relativePath of gitFiles()) {
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
