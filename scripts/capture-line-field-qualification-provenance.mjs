#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const trackedInputs = [
  "evidence/operator-qualification/line-field-closed-form-baseline.json",
  "evidence/operator-qualification/line-field-closed-form-derivation.md",
  "evidence/operator-qualification/line-field-tolerance-policy.json",
  "workers/rust/crates/solver/tests/accuracy_baselines/line_1d.rs",
  "scripts/check-line-field-closed-form-baseline.mjs",
];

function fail(message) {
  console.error(`line-field qualification provenance capture failed: ${message}`);
  process.exit(1);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    return null;
  }
  return result.stdout.trim();
}

function sha256File(relativePath) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    fail(`tracked input missing: ${relativePath}`);
  }
  return crypto.createHash("sha256").update(fs.readFileSync(absolutePath)).digest("hex");
}

function parseArgs(argv) {
  const args = { out: null };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--out") {
      args.out = argv[index + 1];
      index += 1;
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  return args;
}

function buildProvenance() {
  const gitStatus = run("git", ["status", "--short"]) ?? "";
  return {
    schema_version: "kyuubiki.operator-qualification-provenance/v1",
    version_line: "tamamono 1.15.x",
    candidate_id: "line-field-closed-form",
    generated_at_utc: new Date().toISOString(),
    commands: {
      evidence_check: "node ./scripts/check-line-field-closed-form-baseline.mjs",
      solver_baseline: "cargo test -p kyuubiki-solver --test accuracy_baselines line_1d",
    },
    source_revision: {
      git_commit: run("git", ["rev-parse", "HEAD"]),
      git_branch: run("git", ["rev-parse", "--abbrev-ref", "HEAD"]),
      working_tree_clean: gitStatus.length === 0,
      status_entry_count: gitStatus.length === 0 ? 0 : gitStatus.split("\n").length,
    },
    toolchain: {
      node: process.version,
      rustc: run("rustc", ["--version"]),
      cargo: run("cargo", ["--version"]),
    },
    platform: {
      os: os.platform(),
      arch: os.arch(),
      release: os.release(),
    },
    tracked_inputs: trackedInputs.map((relativePath) => ({
      path: relativePath,
      sha256: sha256File(relativePath),
    })),
    retention_policy: {
      release_artifact: true,
      repo_relative_paths_only: true,
      no_local_absolute_paths: true,
    },
  };
}

function writeOutput(outPath, payload) {
  const text = `${JSON.stringify(payload, null, 2)}\n`;
  if (!outPath) {
    process.stdout.write(text);
    return;
  }
  const absoluteOut = path.resolve(repoRoot, outPath);
  const relativeOut = path.relative(repoRoot, absoluteOut);
  if (relativeOut.startsWith("..") || path.isAbsolute(relativeOut)) {
    fail("--out must stay inside the repository");
  }
  fs.mkdirSync(path.dirname(absoluteOut), { recursive: true });
  fs.writeFileSync(absoluteOut, text);
  console.log(`line-field qualification provenance wrote ${relativeOut}`);
}

const args = parseArgs(process.argv);
writeOutput(args.out, buildProvenance());
