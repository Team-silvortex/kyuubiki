#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const AUDIT_LOCKFILE_CONTRACT = loadAuditLockfileContract();
const NPM_AUDIT_DIRS = AUDIT_LOCKFILE_CONTRACT.npm;
const CARGO_AUDIT_DIRS = AUDIT_LOCKFILE_CONTRACT.cargo;

const NPM_AUDIT_ARGS = ["audit", "--omit=dev", "--package-lock-only", "--json"];
const CARGO_AUDIT_ARGS = ["audit"];

if (process.argv.includes("--self-test")) {
  runSelfTest();
  process.exit(0);
}

function loadAuditLockfileContract() {
  const contractPath = path.join(ROOT, "config/dependency-audit-lockfiles.json");
  const contract = JSON.parse(readFileSync(contractPath, "utf8"));
  assert.equal(contract.schema, "kyuubiki.dependency-audit-lockfiles/v1");
  assert.ok(Array.isArray(contract.npm));
  assert.ok(Array.isArray(contract.cargo));
  return contract;
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd: path.join(ROOT, cwd),
    encoding: "utf8",
    stdio: "pipe",
  });

  return {
    command: [command, ...args].join(" "),
    cwd,
    status: result.status ?? 1,
    stdout: result.stdout.trim(),
    stderr: result.stderr.trim(),
  };
}

function summarizeNpmAudit(output) {
  try {
    const parsed = JSON.parse(output);
    const total = parsed.metadata?.vulnerabilities?.total ?? "unknown";
    return `${total} vulnerability(s)`;
  } catch {
    return "unable to parse npm audit JSON";
  }
}

function formatNpmAuditFailure(output) {
  try {
    const parsed = JSON.parse(output);
    const vulnerabilities = Object.values(parsed.vulnerabilities ?? {});
    if (vulnerabilities.length === 0) return output;
    return vulnerabilities
      .map((vulnerability) => {
        const via = Array.isArray(vulnerability.via)
          ? vulnerability.via
              .map((entry) => {
                if (typeof entry === "string") return entry;
                return [entry.title, entry.url].filter(Boolean).join(" ");
              })
              .join("; ")
          : String(vulnerability.via ?? "unknown");
        const direct = vulnerability.isDirect ? "direct" : "transitive";
        return `- ${vulnerability.name} (${vulnerability.severity}, ${direct}): ${via}`;
      })
      .join("\n");
  } catch {
    return output;
  }
}

function auditNpm() {
  return NPM_AUDIT_DIRS.map((cwd) => {
    const result = run("npm", NPM_AUDIT_ARGS, cwd);
    const summary = summarizeNpmAudit(result.stdout);
    return { ...result, summary };
  });
}

function auditCargo() {
  return CARGO_AUDIT_DIRS.map((cwd) => {
    const result = run("cargo", CARGO_AUDIT_ARGS, cwd);
    const warningSummary = result.stdout
      .split("\n")
      .find((line) => line.includes("allowed warnings found"));
    return {
      ...result,
      summary: warningSummary?.trim() || "0 vulnerability(s)",
    };
  });
}

function runSelfTest() {
  assert.deepEqual(NPM_AUDIT_DIRS, [
    "apps/frontend",
    "apps/hub-gui",
    "apps/installer-gui",
    "apps/workbench-gui",
  ]);
  assert.deepEqual(CARGO_AUDIT_DIRS, [
    "workers/rust",
    "sdks/rust",
    "apps/hub-gui/src-tauri",
    "apps/installer-gui/src-tauri",
    "apps/workbench-gui/src-tauri",
  ]);
  assert.deepEqual(
    NPM_AUDIT_DIRS.map((auditDir) => `${auditDir}/package-lock.json`),
    [
      "apps/frontend/package-lock.json",
      "apps/hub-gui/package-lock.json",
      "apps/installer-gui/package-lock.json",
      "apps/workbench-gui/package-lock.json",
    ],
  );
  assert.deepEqual(
    CARGO_AUDIT_DIRS.map((auditDir) => `${auditDir}/Cargo.lock`),
    [
      "workers/rust/Cargo.lock",
      "sdks/rust/Cargo.lock",
      "apps/hub-gui/src-tauri/Cargo.lock",
      "apps/installer-gui/src-tauri/Cargo.lock",
      "apps/workbench-gui/src-tauri/Cargo.lock",
    ],
  );
  assert.deepEqual(NPM_AUDIT_ARGS, ["audit", "--omit=dev", "--package-lock-only", "--json"]);
  assert.deepEqual(CARGO_AUDIT_ARGS, ["audit"]);
  assert.equal(
    summarizeNpmAudit('{"metadata":{"vulnerabilities":{"total":0}}}'),
    "0 vulnerability(s)",
  );
  assert.equal(summarizeNpmAudit("not json"), "unable to parse npm audit JSON");
  assert.equal(
    formatNpmAuditFailure(
      JSON.stringify({
        vulnerabilities: {
          next: {
            name: "next",
            severity: "critical",
            isDirect: true,
            via: [{ title: "Middleware bypass", url: "https://example.test/advisory" }, "postcss"],
          },
        },
      }),
    ),
    "- next (critical, direct): Middleware bypass https://example.test/advisory; postcss",
  );
  console.log("dependency audit self-test passed");
}

function printResult(result) {
  const marker = result.status === 0 ? "ok" : "failed";
  console.log(`[${marker}] ${result.cwd}: ${result.command}`);
  console.log(`      ${result.summary}`);
  if (result.status !== 0) {
    const stdout = result.command.startsWith("npm audit")
      ? formatNpmAuditFailure(result.stdout)
      : result.stdout;
    const detail = [result.stderr, stdout].filter(Boolean).join("\n");
    console.error(detail);
  }
}

const results = [...auditNpm(), ...auditCargo()];
for (const result of results) {
  printResult(result);
}

const failures = results.filter((result) => result.status !== 0);
if (failures.length > 0) {
  console.error(`dependency audit failed: ${failures.length} lane(s) failed`);
  process.exit(1);
}

console.log("dependency audit passed");
