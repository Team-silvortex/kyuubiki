#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DEFAULT_REMOTE_DIR = ".kyuubiki-remote-runs/material-research-example";

function parseArgs(argv) {
  const options = {
    caseFilter: process.env.CASE_FILTER || "",
    host: process.env.KYUUBIKI_LAB_HOST || "kyuubiki-lab",
    matrix: process.env.MATRIX || "compound-core",
    outputSlug: process.env.OUTPUT_SLUG || timestampSlug(),
    profile: process.env.PROFILE || "100k",
    remoteDir: process.env.KYUUBIKI_REMOTE_MATERIAL_DIR || DEFAULT_REMOTE_DIR,
    repeat: process.env.REPEAT || "1",
    solverPreconditioner: process.env.SOLVER_PRECONDITIONER || "auto",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    }
    const next = argv[index + 1];
    if (arg === "--host" && next) {
      options.host = next;
      index += 1;
    } else if (arg === "--case" && next) {
      options.caseFilter = next;
      index += 1;
    } else if (arg === "--remote-dir" && next) {
      options.remoteDir = next;
      index += 1;
    } else if (arg === "--profile" && next) {
      options.profile = next;
      index += 1;
    } else if (arg === "--matrix" && next) {
      options.matrix = next;
      index += 1;
    } else if (arg === "--repeat" && next) {
      options.repeat = next;
      index += 1;
    } else if (arg === "--output-slug" && next) {
      options.outputSlug = next;
      index += 1;
    } else if (arg === "--solver-preconditioner" && next) {
      options.solverPreconditioner = next;
      index += 1;
    } else {
      fail(`unknown or incomplete argument: ${arg}`);
    }
  }
  validateRemoteDir(options.remoteDir);
  return options;
}

function validateRemoteDir(remoteDir) {
  if (!remoteDir || remoteDir.startsWith("/") || remoteDir.includes("..")) {
    fail("remote dir must be a relative path without '..'");
  }
  if (!/^[A-Za-z0-9._/-]+$/.test(remoteDir)) {
    fail("remote dir may only contain letters, numbers, '.', '_', '-', and '/'");
  }
}

function timestampSlug() {
  const stamp = new Date().toISOString().replaceAll(":", "").replace(/\..+$/, "Z");
  return `material-research-remote-${stamp}`;
}

function run(program, args, options = {}) {
  const result = spawnSync(program, args, {
    cwd: options.cwd || repoRoot,
    encoding: "utf8",
    stdio: options.stdio || "inherit",
  });
  if (result.error) fail(`failed to run ${program}: ${result.error.message}`);
  return result.status ?? 1;
}

function requireOk(label, status) {
  if (status !== 0) fail(`${label} failed with exit code ${status}`);
}

function sshArgs(host, command) {
  return ["-o", "BatchMode=yes", "-o", "ConnectTimeout=10", host, command];
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function remoteCommand(options) {
  const profile = shellQuote(options.profile);
  const matrix = shellQuote(options.matrix);
  const repeat = shellQuote(options.repeat);
  const solverPreconditioner = shellQuote(options.solverPreconditioner);
  const caseArgs = options.caseFilter ? ` --case ${shellQuote(options.caseFilter)}` : "";
  return [
    "set -euo pipefail",
    `cd ${shellQuote(options.remoteDir)}`,
    "mkdir -p tmp",
    "make verify-material-research-example OUT=tmp/material-research-example.json",
    "cd workers/rust",
    "cargo test -p kyuubiki-cli material_report",
    "cargo test -p kyuubiki-cli --bin kyuubiki-material-explore",
    `cargo run --release -q -p kyuubiki-benchmark -- --profile ${profile} --matrix ${matrix} --repeat ${repeat} --format json --solver-preconditioner ${solverPreconditioner}${caseArgs} > ../../tmp/remote-material-research-benchmark.json`,
  ].join("; ");
}

function syncSources(options) {
  requireOk(
    "remote mkdir",
    run("ssh", sshArgs(options.host, `mkdir -p ${shellQuote(options.remoteDir)}`)),
  );
  const excludes = [
    ".git/",
    "tmp/",
    "target/",
    "node_modules/",
    "deps/",
    "_build/",
    "dist/",
    ".next/",
    ".DS_Store",
  ];
  const args = [
    "-az",
    "--delete",
    "-e",
    "ssh -o BatchMode=yes -o ConnectTimeout=10",
    ...excludes.flatMap((entry) => [`--exclude=${entry}`]),
    `${repoRoot}/`,
    `${options.host}:${options.remoteDir}/`,
  ];
  requireOk("rsync source sync", run("rsync", args));
}

function pullEvidence(options, localDir) {
  requireOk("local output mkdir", run("mkdir", ["-p", localDir]));
  const artifacts = [
    "tmp/material-research-example.json",
    "tmp/remote-material-research-benchmark.json",
  ];
  for (const artifact of artifacts) {
    const destination = path.join(localDir, path.basename(artifact));
    const source = `${options.host}:${options.remoteDir}/${artifact}`;
    requireOk(`pull ${artifact}`, run("scp", ["-o", "BatchMode=yes", source, destination]));
  }
}

function printUsage() {
  console.log(`Usage:
  node ./scripts/run-remote-material-research-example.mjs [options]

Options:
  --host <ssh-host>       Default: KYUUBIKI_LAB_HOST or kyuubiki-lab
  --case <substring>      Optional benchmark case filter, default: CASE_FILTER
  --remote-dir <path>     Relative remote scratch dir, default: ${DEFAULT_REMOTE_DIR}
  --profile <profile>     Benchmark profile, default: PROFILE or 100k
  --matrix <matrix>       Benchmark matrix, default: MATRIX or compound-core
  --repeat <count>        Benchmark repeat count, default: REPEAT or 1
  --output-slug <slug>    Local tmp output slug
  --solver-preconditioner <name>
                          Default: SOLVER_PRECONDITIONER or auto

This runner never stores credentials. It requires an existing SSH key/config and
uses rsync --delete only inside the selected remote scratch directory.`);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

const options = parseArgs(process.argv.slice(2));
const localDir = path.join(repoRoot, "tmp", "remote-material-research", options.outputSlug);
console.log(`remote host: ${options.host}`);
console.log(`remote dir: ${options.remoteDir}`);
console.log(`benchmark: profile=${options.profile} matrix=${options.matrix} repeat=${options.repeat} solver_preconditioner=${options.solverPreconditioner} case=${options.caseFilter || "all"}`);
syncSources(options);
requireOk("remote material research run", run("ssh", sshArgs(options.host, remoteCommand(options))));
pullEvidence(options, localDir);
requireOk(
  "remote material benchmark summary",
  run("./scripts/kyuubiki", ["build-remote-material-benchmark-summary"]),
);
console.log(`remote material research evidence: ${localDir}`);
