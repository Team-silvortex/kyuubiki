#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const defaultRemoteDir = ".kyuubiki-remote-runs/central-database-smoke";
const defaultDryRunDatabaseUrl = "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev";

const options = parseArgs(process.argv.slice(2));
const shouldExecuteSmoke = options.runSmoke;
const databaseUrl = process.env.DATABASE_URL || (!shouldExecuteSmoke ? defaultDryRunDatabaseUrl : "");

console.log(`remote host: ${options.host}`);
console.log(`remote dir: ${options.remoteDir}`);
console.log(`mode/backend: ${options.mode}/${options.backend}`);
console.log(`execute smoke: ${shouldExecuteSmoke ? "yes" : "no (dry-run)"}`);
console.log(`database url: ${databaseUrl ? redactDatabaseUrl(databaseUrl) : "missing"}`);

if (!databaseUrl) {
  fail("DATABASE_URL is required when RUN_DB_SMOKE=1 or --run is used");
}

if (options.planOnly) {
  console.log("plan-only: remote sync and smoke execution skipped");
  process.exit(0);
}

syncSources(options);
const status = run("ssh", sshArgs(options.host, remoteCommand(options, databaseUrl)));
process.exit(status);

function parseArgs(argv) {
  const parsed = {
    backend: process.env.BACKEND || "postgres",
    host: process.env.KYUUBIKI_LAB_HOST || process.env.REMOTE || "kyuubiki-lab",
    mode: process.env.MODE || "cloud",
    planOnly: process.env.PLAN_ONLY === "1",
    remoteDir: process.env.KYUUBIKI_REMOTE_CENTRAL_DB_DIR || defaultRemoteDir,
    runSmoke: process.env.RUN_DB_SMOKE === "1",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    } else if (arg === "--host" && next) {
      parsed.host = next;
      index += 1;
    } else if (arg === "--remote-dir" && next) {
      parsed.remoteDir = next;
      index += 1;
    } else if (arg === "--mode" && next) {
      parsed.mode = next;
      index += 1;
    } else if (arg === "--backend" && next) {
      parsed.backend = next;
      index += 1;
    } else if (arg === "--run") {
      parsed.runSmoke = true;
    } else if (arg === "--plan-only") {
      parsed.planOnly = true;
    } else {
      fail(`unknown or incomplete argument: ${arg}`);
    }
  }

  validateRemoteDir(parsed.remoteDir);
  return parsed;
}

function validateRemoteDir(remoteDir) {
  if (!remoteDir || remoteDir.startsWith("/") || remoteDir.includes("..")) {
    fail("remote dir must be a relative scratch path without '..'");
  }
  if (!/^[A-Za-z0-9._/-]+$/.test(remoteDir)) {
    fail("remote dir may only contain letters, numbers, '.', '_', '-', and '/'");
  }
}

function remoteCommand({ backend, mode, remoteDir, runSmoke }, databaseUrl) {
  const smokeFlag = runSmoke ? "1" : "0";
  return [
    "set -euo pipefail",
    "export PATH=\"$HOME/.kyuubiki-toolchains/node-v20.19.2-linux-x64/bin:$HOME/.elixir-install/installs/elixir/1.20.1-otp-28/bin:$HOME/.elixir-install/installs/otp/28.4/bin:$PATH\"",
    `cd ${shellQuote(remoteDir)}`,
    `export MODE=${shellQuote(mode)}`,
    `export BACKEND=${shellQuote(backend)}`,
    `export RUN_DB_SMOKE=${shellQuote(smokeFlag)}`,
    `export DATABASE_URL=${shellQuote(databaseUrl)}`,
    "node ./scripts/check-central-database-readiness.mjs --mode \"$MODE\" --backend \"$BACKEND\" --json",
    "./scripts/kyuubiki central-database-smoke --mode \"$MODE\" --backend \"$BACKEND\"",
  ].join("; ");
}

function syncSources(options) {
  requireOk("remote mkdir", run("ssh", sshArgs(options.host, `mkdir -p ${shellQuote(options.remoteDir)}`)));
  const excludes = [
    ".git/",
    ".DS_Store",
    "_build/",
    ".next/",
    "deps/",
    "dist/",
    "node_modules/",
    "target/",
    "tmp/",
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

function sshArgs(host, command) {
  return ["-o", "BatchMode=yes", "-o", "ConnectTimeout=10", host, command];
}

function run(program, args) {
  const result = spawnSync(program, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: "inherit",
  });
  if (result.error) fail(`failed to run ${program}: ${result.error.message}`);
  return result.status ?? 1;
}

function requireOk(label, status) {
  if (status !== 0) fail(`${label} failed with exit code ${status}`);
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function redactDatabaseUrl(value) {
  return value.replace(/\/\/([^:/@]+):([^@]+)@/, "//$1:***@");
}

function printUsage() {
  console.log(`Usage:
  node ./scripts/run-remote-central-database-smoke.mjs [options]

Options:
  --host <ssh-host>       Default: KYUUBIKI_LAB_HOST, REMOTE, or kyuubiki-lab
  --remote-dir <path>     Relative remote scratch dir, default: ${defaultRemoteDir}
  --mode <mode>           Default: MODE or cloud
  --backend <backend>     Default: BACKEND or postgres
  --run                   Execute DB-backed tests; otherwise dry-run only
  --plan-only             Print the safe execution plan without SSH/rsync

The runner never stores credentials or server config in the repository. It uses
the existing SSH key/config and forwards DATABASE_URL only through the process
environment for the current run.`);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
