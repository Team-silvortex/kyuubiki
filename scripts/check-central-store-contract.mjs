#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const args = new Set(process.argv.slice(2));

const CONTRACTS = {
  catalog: {
    schemaVersion: "kyuubiki.central-store-catalog/v1",
    schemaPath: "schemas/central-store-catalog.schema.json",
    backendPath: "apps/web/lib/kyuubiki_web/central_store.ex",
    frontendTypesPath: "apps/frontend/src/lib/api/central-store-types.ts",
  },
  session: {
    schemaVersion: "kyuubiki.central-session-policy/v1",
    schemaPath: "schemas/central-session-policy.schema.json",
    backendPath: "apps/web/lib/kyuubiki_web/central_store.ex",
    frontendTypesPath: "apps/frontend/src/lib/api/central-store-types.ts",
  },
  publish: {
    schemaVersion: "kyuubiki.central-publish-policy/v1",
    schemaPath: "schemas/central-publish-policy.schema.json",
    backendPath: "apps/web/lib/kyuubiki_web/central_store.ex",
    frontendTypesPath: "apps/frontend/src/lib/api/central-store-types.ts",
  },
  database: {
    schemaVersion: "kyuubiki.central-database-policy/v1",
    schemaPath: "schemas/central-database-policy.schema.json",
    backendPath: "apps/web/lib/kyuubiki_web/central_store.ex",
    frontendTypesPath: "apps/frontend/src/lib/api/central-store-types.ts",
  },
};

const REQUIRED_FILES = [
  "apps/web/lib/kyuubiki_web/central_store_router.ex",
  "apps/web/lib/kyuubiki_web/storage/central_database.ex",
  "apps/web/test/kyuubiki_web/api/central_store_api_test.exs",
  "apps/web/test/kyuubiki_web/storage/central_database_test.exs",
  "apps/frontend/src/lib/api/central-store-client.ts",
  "apps/frontend/test/workflow/workbench-central-store-api-client.test.ts",
  "scripts/run-central-database-smoke.mjs",
  "scripts/run-remote-central-database-smoke.mjs",
  "docs/central-server-components.md",
];

if (args.has("--self-test")) {
  const report = validate({
    files: {
      "schemas/central-store-catalog.schema.json": JSON.stringify({
        properties: { schema_version: { const: CONTRACTS.catalog.schemaVersion } },
      }),
      "schemas/central-session-policy.schema.json": JSON.stringify({
        properties: { schema_version: { const: CONTRACTS.session.schemaVersion } },
      }),
      "schemas/central-publish-policy.schema.json": JSON.stringify({
        properties: { schema_version: { const: CONTRACTS.publish.schemaVersion } },
      }),
      "schemas/central-database-policy.schema.json": JSON.stringify({
        properties: {
          schema_version: { const: CONTRACTS.database.schemaVersion },
          table_specs: {},
        },
      }),
      "apps/web/lib/kyuubiki_web/central_store.ex": [
        CONTRACTS.catalog.schemaVersion,
        CONTRACTS.session.schemaVersion,
        CONTRACTS.publish.schemaVersion,
        CONTRACTS.database.schemaVersion,
      ].join("\n"),
      "apps/web/lib/kyuubiki_web/storage/central_database.ex":
        "kyuubiki.central-database-contract/v1\ncentral_store_entries\ncentral_artifact_signatures",
      "apps/frontend/src/lib/api/central-store-types.ts": [
        CONTRACTS.catalog.schemaVersion,
        CONTRACTS.session.schemaVersion,
        CONTRACTS.publish.schemaVersion,
        CONTRACTS.database.schemaVersion,
        "kyuubiki.central-database-contract/v1",
        "CentralDatabaseTableSpec",
      ].join("\n"),
      "apps/web/lib/kyuubiki_web/central_store_router.ex": "/catalog\n/session-policy\n/publish-policy\n/database-policy",
      "apps/web/test/kyuubiki_web/storage/central_database_test.exs": "central_store_entries",
      "apps/web/test/kyuubiki_web/api/central_store_api_test.exs": "central_store",
      "apps/frontend/src/lib/api/central-store-client.ts": "/api/v1/central/catalog",
      "apps/frontend/test/workflow/workbench-central-store-api-client.test.ts": "fetchCentralCatalog",
      "scripts/run-central-database-smoke.mjs": "RUN_DB_SMOKE\ncheck-central-database-readiness.mjs",
      "scripts/run-remote-central-database-smoke.mjs": "BatchMode=yes\nDATABASE_URL\nrun-central-database-smoke.mjs",
      "docs/central-server-components.md": [
        "schemas/central-store-catalog.schema.json",
        "schemas/central-publish-policy.schema.json",
        "schemas/central-database-policy.schema.json",
      ].join("\n"),
    },
  });

  if (report.errors.length > 0) {
    report.errors.forEach((error) => console.error(`central-store-contract self-test: ${error}`));
    process.exit(1);
  }

  console.log("central store contract self-test passed");
  process.exit(0);
}

const report = validate({
  files: Object.fromEntries(
    allContractFiles().map((relativePath) => [relativePath, readText(relativePath)]),
  ),
});

if (report.errors.length > 0) {
  report.errors.forEach((error) => console.error(`central-store-contract: ${error}`));
  process.exit(1);
}

console.log(
  `central store contract passed: ${report.schemaCount} schema(s), ${report.requiredFileCount} required file(s)`,
);

function allContractFiles() {
  return [
    ...Object.values(CONTRACTS).flatMap((contract) => [
      contract.schemaPath,
      contract.backendPath,
      contract.frontendTypesPath,
    ]),
    ...REQUIRED_FILES,
  ].filter((value, index, values) => values.indexOf(value) === index);
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function validate({ files }) {
  const errors = [];

  for (const file of allContractFiles()) {
    if (typeof files[file] !== "string") {
      errors.push(`missing required file: ${file}`);
    }
  }

  for (const [name, contract] of Object.entries(CONTRACTS)) {
    validateSchema(name, contract, files, errors);
    validateTextContains(contract.backendPath, contract.schemaVersion, "backend", files, errors);
    validateTextContains(contract.frontendTypesPath, contract.schemaVersion, "frontend types", files, errors);
  }

  validateTextContains(
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "/catalog",
    "router catalog route",
    files,
    errors,
  );
  validateTextContains(
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "/session-policy",
    "router session route",
    files,
    errors,
  );
  validateTextContains(
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "/publish-policy",
    "router publish route",
    files,
    errors,
  );
  validateTextContains(
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "/database-policy",
    "router database route",
    files,
    errors,
  );
  validateTextContains(
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "kyuubiki.central-database-contract/v1",
    "central database contract version",
    files,
    errors,
  );
  validateTextContains(
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "central_artifact_signatures",
    "central database artifact signature table",
    files,
    errors,
  );
  validateTextContains(
    "apps/frontend/src/lib/api/central-store-client.ts",
    "/api/v1/central/catalog",
    "frontend catalog path",
    files,
    errors,
  );
  validateTextContains(
    "apps/frontend/src/lib/api/central-store-types.ts",
    "CentralDatabaseTableSpec",
    "frontend central database table spec type",
    files,
    errors,
  );
  validateTextContains(
    "apps/frontend/src/lib/api/central-store-types.ts",
    "kyuubiki.central-database-contract/v1",
    "frontend central database contract version",
    files,
    errors,
  );
  validateTextContains(
    "schemas/central-database-policy.schema.json",
    "table_specs",
    "central database policy table specs schema",
    files,
    errors,
  );
  validateTextContains(
    "docs/central-server-components.md",
    "schemas/central-store-catalog.schema.json",
    "central docs schema reference",
    files,
    errors,
  );
  validateTextContains(
    "docs/central-server-components.md",
    "schemas/central-publish-policy.schema.json",
    "central docs publish schema reference",
    files,
    errors,
  );
  validateTextContains(
    "docs/central-server-components.md",
    "schemas/central-database-policy.schema.json",
    "central docs database schema reference",
    files,
    errors,
  );
  validateTextContains(
    "scripts/run-central-database-smoke.mjs",
    "check-central-database-readiness.mjs",
    "central database smoke readiness preflight",
    files,
    errors,
  );
  validateTextContains(
    "scripts/run-remote-central-database-smoke.mjs",
    "BatchMode=yes",
    "remote central database smoke ssh key mode",
    files,
    errors,
  );
  validateTextContains(
    "scripts/run-remote-central-database-smoke.mjs",
    "run-central-database-smoke.mjs",
    "remote central database smoke wrapper",
    files,
    errors,
  );

  return { errors, schemaCount: Object.keys(CONTRACTS).length, requiredFileCount: REQUIRED_FILES.length };
}

function validateSchema(name, contract, files, errors) {
  try {
    const schema = JSON.parse(files[contract.schemaPath]);
    const actual = schema.properties?.schema_version?.const;
    if (actual !== contract.schemaVersion) {
      errors.push(`${name} schema const must be ${contract.schemaVersion}`);
    }
  } catch (error) {
    errors.push(`${contract.schemaPath}: ${error.message}`);
  }
}

function validateTextContains(file, needle, label, files, errors) {
  if (!files[file]?.includes(needle)) {
    errors.push(`${label} missing ${needle} in ${file}`);
  }
}
