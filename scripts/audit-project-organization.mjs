#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
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
  ".mk",
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

const AUDIT_LOCKFILE_CONTRACT = loadAuditLockfileContract();
const REQUIRED_AUDIT_LOCKFILES = [
  ...AUDIT_LOCKFILE_CONTRACT.npm.map((auditDir) => `${auditDir}/package-lock.json`),
  ...AUDIT_LOCKFILE_CONTRACT.cargo.map((auditDir) => `${auditDir}/Cargo.lock`),
];

if (process.argv.includes("--self-test")) {
  runSelfTest();
  process.exit(0);
}

function gitFileList(args) {
  return execFileSync("git", args, {
    cwd: ROOT,
    encoding: "utf8",
  })
    .split("\n")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function loadAuditLockfileContract() {
  const contract = JSON.parse(
    readFileSync(path.join(ROOT, "config/dependency-audit-lockfiles.json"), "utf8"),
  );
  assert.equal(contract.schema, "kyuubiki.dependency-audit-lockfiles/v1");
  assert.ok(Array.isArray(contract.npm));
  assert.ok(Array.isArray(contract.cargo));
  return contract;
}

function gitCheckIgnored(relativePath) {
  const result = spawnSync("git", ["check-ignore", "-q", relativePath], {
    cwd: ROOT,
    encoding: "utf8",
    stdio: "pipe",
  });
  if (result.status === 0) return true;
  if (result.status === 1) return false;
  const detail = [result.stderr, result.stdout].filter(Boolean).join("\n");
  throw new Error(`git check-ignore failed for ${relativePath}${detail ? `: ${detail}` : ""}`);
}

function requiredAuditLockfileViolations(paths, options = {}) {
  const localViolations = [];
  const exists = options.exists ?? ((relativePath) => existsSync(path.join(ROOT, relativePath)));
  const ignored = options.ignored ?? gitCheckIgnored;

  for (const relativePath of paths) {
    if (!exists(relativePath)) {
      localViolations.push({
        relativePath,
        lines: 0,
        limit: "dependency-audit-lockfile-required",
        customMessage: "dependency audit lockfile is required for reproducible security checks",
      });
      continue;
    }
    if (ignored(relativePath)) {
      localViolations.push({
        relativePath,
        lines: 0,
        limit: "dependency-audit-lockfile-not-ignored",
        customMessage: "dependency audit lockfile must not be ignored by git",
      });
    }
  }
  return localViolations;
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

function installerTestIndexViolations(relativePath, contents, moduleExists) {
  const localViolations = [];
  const lines = contents.split("\n");
  for (const [index, line] of lines.entries()) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("//")) continue;
    const match = trimmed.match(/^mod ([a-z0-9_]+);$/);
    if (!match) {
      localViolations.push({
        relativePath,
        lines: index + 1,
        limit: "module-index-only",
        customMessage:
          "installer tests.rs should only declare test modules; put tests in workers/rust/crates/installer/src/tests/",
      });
      return localViolations;
    }

    const moduleRelativePath = `workers/rust/crates/installer/src/tests/${match[1]}.rs`;
    if (!moduleExists(moduleRelativePath)) {
      localViolations.push({
        relativePath,
        lines: index + 1,
        limit: "module-file-required",
        customMessage: `installer test module ${match[1]} is missing ${moduleRelativePath}`,
      });
      return localViolations;
    }
  }
  return localViolations;
}

function checkInstallerTestIndex() {
  const relativePath = "workers/rust/crates/installer/src/tests.rs";
  const absolutePath = path.join(ROOT, relativePath);
  if (!existsSync(absolutePath)) return;

  violations.push(
    ...installerTestIndexViolations(
      relativePath,
      readFileSync(absolutePath, "utf8"),
      (moduleRelativePath) => existsSync(path.join(ROOT, moduleRelativePath)),
    ),
  );
}

checkInstallerTestIndex();

function checkRequiredAuditLockfiles() {
  violations.push(...requiredAuditLockfileViolations(REQUIRED_AUDIT_LOCKFILES));
}

checkRequiredAuditLockfiles();

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

function runSelfTest() {
  const relativePath = "workers/rust/crates/installer/src/tests.rs";
  const existingModules = new Set([
    "workers/rust/crates/installer/src/tests/control_update.rs",
    "workers/rust/crates/installer/src/tests/security_integrity.rs",
  ]);
  const moduleExists = (moduleRelativePath) => existingModules.has(moduleRelativePath);

  assert.deepEqual(
    installerTestIndexViolations(
      relativePath,
      "mod control_update;\nmod security_integrity;\n",
      moduleExists,
    ),
    [],
  );
  assert.equal(
    installerTestIndexViolations(relativePath, "#[test]\nfn inline_test() {}\n", moduleExists)[0]
      .limit,
    "module-index-only",
  );
  assert.equal(
    installerTestIndexViolations(relativePath, "mod missing_module;\n", moduleExists)[0].limit,
    "module-file-required",
  );
  assert.deepEqual(REQUIRED_AUDIT_LOCKFILES, [
    "apps/frontend/package-lock.json",
    "apps/hub-gui/package-lock.json",
    "apps/installer-gui/package-lock.json",
    "apps/workbench-gui/package-lock.json",
    "workers/rust/Cargo.lock",
    "sdks/rust/Cargo.lock",
    "apps/hub-gui/src-tauri/Cargo.lock",
    "apps/installer-gui/src-tauri/Cargo.lock",
    "apps/workbench-gui/src-tauri/Cargo.lock",
  ]);
  assert.equal(
    requiredAuditLockfileViolations(["missing.lock"], {
      exists: () => false,
      ignored: () => false,
    })[0].limit,
    "dependency-audit-lockfile-required",
  );
  assert.equal(
    requiredAuditLockfileViolations(["ignored.lock"], {
      exists: () => true,
      ignored: () => true,
    })[0].limit,
    "dependency-audit-lockfile-not-ignored",
  );
  assert.deepEqual(
    requiredAuditLockfileViolations(["ok.lock"], {
      exists: () => true,
      ignored: () => false,
    }),
    [],
  );
  console.log("project organization audit self-test passed");
}
