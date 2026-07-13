#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import process from "node:process";

const args = new Set(process.argv.slice(2));
const mode = valueAfter("--mode") ?? process.env.MODE ?? "cloud";
const backend = valueAfter("--backend") ?? process.env.BACKEND ?? "postgres";
const runSmoke = args.has("--run") || process.env.RUN_DB_SMOKE === "1";

const readiness = run("node", [
  "./scripts/check-central-database-readiness.mjs",
  "--mode",
  mode,
  "--backend",
  backend,
]);

if (readiness.status !== 0) {
  process.exit(readiness.status ?? 1);
}

if (!runSmoke) {
  console.log(
    "central database smoke dry-run ok; set RUN_DB_SMOKE=1 or pass --run to execute Postgres-backed tests",
  );
  process.exit(0);
}

const env = {
  ...process.env,
  KYUUBIKI_DEPLOYMENT_MODE: mode,
  KYUUBIKI_STORAGE_BACKEND: backend,
};

const result = run(
  "mix",
  [
    "test",
    "test/kyuubiki_web/api/central_store_api_test.exs",
    "test/kyuubiki_web/api/asset_store_api_test.exs",
  ],
  { cwd: "apps/web", env },
);

process.exit(result.status ?? 1);

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : null;
}

function run(command, commandArgs, options = {}) {
  const result = spawnSync(command, commandArgs, {
    cwd: options.cwd ?? process.cwd(),
    env: options.env ?? process.env,
    encoding: "utf8",
    stdio: "inherit",
  });

  if (result.error) {
    console.error(result.error.message);
    return { status: 1 };
  }

  return { status: result.status ?? 1 };
}
