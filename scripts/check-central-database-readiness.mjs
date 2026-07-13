#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const args = new Set(process.argv.slice(2));
const mode = valueAfter("--mode") ?? process.env.KYUUBIKI_DEPLOYMENT_MODE ?? "local";
const backend = valueAfter("--backend") ?? process.env.KYUUBIKI_STORAGE_BACKEND ?? defaultBackendForMode(mode);

if (args.has("--self-test")) {
  const report = validate({
    mode: "cloud",
    backend: "postgres",
    env: { DATABASE_URL: "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev" },
    files: {
      "apps/web/config/config.exs": "KYUUBIKI_STORAGE_BACKEND DATABASE_URL SQLITE_DATABASE_PATH",
      "apps/web/lib/kyuubiki_web/central_store.ex":
        "kyuubiki.central-database-policy/v1 CentralDatabase.table_specs",
      "apps/web/lib/kyuubiki_web/central_store_router.ex": "/database-policy /database-status",
      "apps/web/lib/kyuubiki_web/storage/central_database.ex":
        "kyuubiki.central-database-contract/v1 central_store_entries central_artifact_signatures",
      "apps/web/lib/kyuubiki_web/storage/schema_setup.ex": "CentralDatabase.create_table_sqls",
      "apps/frontend/src/lib/api/central-store-client.ts":
        "/api/v1/central/database-policy /api/v1/central/database-status",
      "apps/frontend/src/lib/api/central-store-types.ts":
        "CentralDatabaseTableSpec CentralDatabaseStatusPayload kyuubiki.central-database-contract/v1",
      "schemas/central-database-policy.schema.json":
        "kyuubiki.central-database-policy/v1 kyuubiki.central-database-contract/v1",
      "schemas/central-database-status.schema.json": "kyuubiki.central-database-status/v1",
    },
  });

  if (report.issues.length > 0) {
    report.issues.forEach((issue) => console.error(`central-database-readiness self-test: ${issue}`));
    process.exit(1);
  }

  console.log("central database readiness self-test passed");
  process.exit(0);
}

const report = validate({
  mode,
  backend,
  env: process.env,
  files: Object.fromEntries(requiredFiles().map((file) => [file, readText(file)])),
});

if (args.has("--json")) {
  console.log(JSON.stringify(report, null, 2));
}

if (report.issues.length > 0) {
  if (!args.has("--json")) {
    console.error("central database readiness failed:");
    report.issues.forEach((issue) => console.error(`- ${issue}`));
  }
  process.exit(1);
}

if (!args.has("--json")) {
  console.log(`central database readiness ok: mode=${mode}, backend=${backend}`);
}

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : null;
}

function defaultBackendForMode(value) {
  return value === "cloud" || value === "distributed" ? "postgres" : "sqlite";
}

function requiredFiles() {
  return [
    "apps/web/config/config.exs",
    "apps/web/lib/kyuubiki_web/central_store.ex",
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "apps/web/lib/kyuubiki_web/storage/schema_setup.ex",
    "apps/frontend/src/lib/api/central-store-client.ts",
    "apps/frontend/src/lib/api/central-store-types.ts",
    "schemas/central-database-policy.schema.json",
    "schemas/central-database-status.schema.json",
  ];
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function validate({ mode, backend, env, files }) {
  const issues = [];
  const supportedBackends = new Set(["sqlite", "postgres"]);

  if (!supportedBackends.has(backend)) {
    issues.push(`unsupported backend ${backend}; expected sqlite or postgres`);
  }
  if ((mode === "cloud" || mode === "distributed") && backend !== "postgres") {
    issues.push(`${mode} mode must use postgres backend`);
  }
  if (backend === "postgres" && !env.DATABASE_URL) {
    issues.push("DATABASE_URL is required for postgres backend");
  }
  if (backend === "sqlite") {
    const sqlitePath = env.SQLITE_DATABASE_PATH || "./tmp/data/kyuubiki_dev.sqlite3";
    if (!sqlitePath.endsWith(".sqlite3")) {
      issues.push("SQLITE_DATABASE_PATH should point to a .sqlite3 file");
    }
  }

  for (const [file, text] of Object.entries(files)) {
    if (typeof text !== "string") {
      issues.push(`missing required file: ${file}`);
    }
  }

  requireContains(issues, files, "apps/web/config/config.exs", "KYUUBIKI_STORAGE_BACKEND");
  requireContains(issues, files, "apps/web/config/config.exs", "DATABASE_URL");
  requireContains(issues, files, "apps/web/config/config.exs", "SQLITE_DATABASE_PATH");
  requireContains(issues, files, "apps/web/lib/kyuubiki_web/central_store.ex", "kyuubiki.central-database-policy/v1");
  requireContains(issues, files, "apps/web/lib/kyuubiki_web/central_store.ex", "CentralDatabase.table_specs");
  requireContains(issues, files, "apps/web/lib/kyuubiki_web/central_store_router.ex", "/database-policy");
  requireContains(issues, files, "apps/web/lib/kyuubiki_web/central_store_router.ex", "/database-status");
  requireContains(
    issues,
    files,
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "kyuubiki.central-database-contract/v1",
  );
  requireContains(
    issues,
    files,
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "central_store_entries",
  );
  requireContains(
    issues,
    files,
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "central_artifact_signatures",
  );
  requireContains(
    issues,
    files,
    "apps/web/lib/kyuubiki_web/storage/schema_setup.ex",
    "CentralDatabase.create_table_sqls",
  );
  requireContains(issues, files, "apps/frontend/src/lib/api/central-store-client.ts", "/api/v1/central/database-policy");
  requireContains(issues, files, "apps/frontend/src/lib/api/central-store-client.ts", "/api/v1/central/database-status");
  requireContains(issues, files, "apps/frontend/src/lib/api/central-store-types.ts", "CentralDatabaseTableSpec");
  requireContains(issues, files, "apps/frontend/src/lib/api/central-store-types.ts", "CentralDatabaseStatusPayload");
  requireContains(
    issues,
    files,
    "apps/frontend/src/lib/api/central-store-types.ts",
    "kyuubiki.central-database-contract/v1",
  );
  requireContains(issues, files, "schemas/central-database-policy.schema.json", "kyuubiki.central-database-policy/v1");
  requireContains(
    issues,
    files,
    "schemas/central-database-policy.schema.json",
    "kyuubiki.central-database-contract/v1",
  );
  requireContains(
    issues,
    files,
    "schemas/central-database-status.schema.json",
    "kyuubiki.central-database-status/v1",
  );

  return {
    schema_version: "kyuubiki.central-database-readiness/v1",
    mode,
    backend,
    status: issues.length === 0 ? "ok" : "fail",
    checks: {
      static_contract_files: requiredFiles(),
      postgres_requires_database_url: backend === "postgres",
      sqlite_path: env.SQLITE_DATABASE_PATH || "./tmp/data/kyuubiki_dev.sqlite3",
    },
    issues,
  };
}

function requireContains(issues, files, file, needle) {
  if (!files[file]?.includes(needle)) {
    issues.push(`${file} must include ${needle}`);
  }
}
