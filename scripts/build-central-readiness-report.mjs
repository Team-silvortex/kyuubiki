#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const repoRoot = process.cwd();
const args = new Set(process.argv.slice(2));
const mode = valueAfter("--mode") ?? process.env.MODE ?? "local";
const backend = valueAfter("--backend") ?? process.env.BACKEND ?? "sqlite";
const outPath = valueAfter("--out") ?? process.env.OUT ?? "tmp/central-readiness-report.json";
const markdownPath =
  valueAfter("--markdown-out") ?? process.env.MARKDOWN_OUT ?? siblingMarkdownPath(outPath);

const endpoints = [
  "/api/v1/central/catalog",
  "/api/v1/central/session-policy",
  "/api/v1/central/publish-policy",
  "/api/v1/central/publish-readiness",
  "/api/v1/central/database-policy",
  "/api/v1/central/provenance-policy",
  "/api/v1/central/database-status",
];

const schemaFiles = [
  "schemas/central-store-contract-check.schema.json",
  "schemas/central-store-catalog.schema.json",
  "schemas/central-session-policy.schema.json",
  "schemas/central-publish-policy.schema.json",
  "schemas/central-publish-readiness.schema.json",
  "schemas/central-database-policy.schema.json",
  "schemas/central-provenance-policy.schema.json",
  "schemas/central-database-status.schema.json",
  "schemas/central-readiness-report.schema.json",
];

const configFiles = [
  "config/architecture/central-store-contract.json",
  "config/architecture/module-topology.json",
];

if (args.has("--self-test")) {
  const report = buildReport({
    readiness: {
      schema_version: "kyuubiki.central-database-readiness/v1",
      status: "ok",
      issues: [],
      checks: { static_contract_files: ["a"] },
    },
    files: {
      "apps/web/lib/kyuubiki_web/central_store_router.ex": endpoints.join("\n"),
      "apps/frontend/src/lib/api/central-store-client.ts": endpoints.join("\n"),
      "apps/web/lib/kyuubiki_web/storage/central_database.ex":
        "kyuubiki.central-database-contract/v1 central_store_entries central_artifact_signatures",
      ...Object.fromEntries(schemaFiles.map((file) => [file, "{}"])),
      ...Object.fromEntries(configFiles.map((file) => [file, "kyuubiki.central-store-contract-check/v1"])),
      "config/architecture/module-topology.json":
        "kyuubiki.module-topology/v1 central-web-service self_host_web orchestra-control-plane",
      "docs/central-server-components.md": "central-web-service not a separate top-level module",
    },
  });
  const issues = validateReport(report);
  if (issues.length > 0) fail(`self-test failed: ${issues.join("; ")}`);
  console.log("central readiness report self-test passed");
  process.exit(0);
}

const readiness = runReadiness();
const report = buildReport({
  readiness,
  files: Object.fromEntries(requiredFiles().map((file) => [file, readText(file)])),
});
const issues = validateReport(report);
if (issues.length > 0) fail(`central readiness report invalid: ${issues.join("; ")}`);
writeJson(outPath, report);
writeText(markdownPath, renderMarkdown(report));
console.log(`central readiness report written: ${outPath}`);
console.log(`central readiness summary written: ${markdownPath}`);

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : null;
}

function requiredFiles() {
  return [
    "apps/web/lib/kyuubiki_web/central_store_router.ex",
    "apps/frontend/src/lib/api/central-store-client.ts",
    "apps/web/lib/kyuubiki_web/storage/central_database.ex",
    "docs/central-server-components.md",
    ...schemaFiles,
    ...configFiles,
  ];
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function runReadiness() {
  const result = spawnSync(
    "node",
    ["./scripts/check-central-database-readiness.mjs", "--mode", mode, "--backend", backend, "--json"],
    {
      cwd: repoRoot,
      encoding: "utf8",
      env: process.env,
    },
  );
  if (result.error) fail(result.error.message);
  if (result.status !== 0) {
    process.stderr.write(result.stderr);
    process.stdout.write(result.stdout);
    process.exit(result.status ?? 1);
  }
  return JSON.parse(result.stdout);
}

function buildReport({ readiness, files }) {
  return {
    schema_version: "kyuubiki.central-readiness-report/v1",
    generated_at: new Date().toISOString(),
    mode,
    backend,
    status: readiness.status === "ok" ? "ok" : "fail",
    readiness,
    api_surface: {
      endpoints: endpoints.map((endpoint) => endpointStatus(endpoint, files)),
    },
    schema_surface: {
      schema_files: schemaFiles.map((file) => ({
        path: file,
        present: typeof files[file] === "string",
      })),
    },
    config_surface: {
      config_files: configFiles.map((file) => ({
        path: file,
        present: typeof files[file] === "string",
        schema_version_present:
          file === "config/architecture/central-store-contract.json"
            ? files[file]?.includes("kyuubiki.central-store-contract-check/v1") === true
            : files[file]?.includes("kyuubiki.module-topology/v1") === true,
      })),
    },
    service_surface: {
      id: "central-web-service",
      module_id: "orchestra-control-plane",
      kind: "self_host_web",
      topology_present: includesAll(files["config/architecture/module-topology.json"], [
        "central-web-service",
        "self_host_web",
        "orchestra-control-plane",
      ]),
      boundary_documented: includesAll(files["docs/central-server-components.md"], [
        "central-web-service",
        "not a separate top-level module",
      ]),
    },
    storage_contract: {
      schema_version: "kyuubiki.central-database-contract/v1",
      table_contract_present: includesAll(files["apps/web/lib/kyuubiki_web/storage/central_database.ex"], [
        "kyuubiki.central-database-contract/v1",
        "central_store_entries",
        "central_artifact_signatures",
      ]),
    },
    runbook: {
      local_readiness: "make check-central-database-readiness MODE=local BACKEND=sqlite",
      remote_dry_run: "make remote-central-database-smoke REMOTE=kyuubiki-lab",
      postgres_smoke: "RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke",
    },
  };
}

function endpointStatus(endpoint, files) {
  const suffix = endpoint.replace("/api/v1/central", "");
  return {
    path: endpoint,
    router_present: files["apps/web/lib/kyuubiki_web/central_store_router.ex"]?.includes(suffix) === true,
    client_present: files["apps/frontend/src/lib/api/central-store-client.ts"]?.includes(endpoint) === true,
  };
}

function includesAll(text, needles) {
  return typeof text === "string" && needles.every((needle) => text.includes(needle));
}

function validateReport(report) {
  const issues = [];
  if (report.schema_version !== "kyuubiki.central-readiness-report/v1") {
    issues.push("unexpected schema_version");
  }
  if (report.status !== "ok") issues.push("readiness status is not ok");
  for (const endpoint of report.api_surface.endpoints) {
    if (!endpoint.router_present) issues.push(`router missing ${endpoint.path}`);
    if (!endpoint.client_present) issues.push(`client missing ${endpoint.path}`);
  }
  if (!report.storage_contract.table_contract_present) {
    issues.push("central database table contract is incomplete");
  }
  for (const schema of report.schema_surface.schema_files) {
    if (!schema.present) issues.push(`schema file missing: ${schema.path}`);
  }
  for (const config of report.config_surface.config_files) {
    if (!config.present) issues.push(`config file missing: ${config.path}`);
    if (!config.schema_version_present) issues.push(`config schema version missing: ${config.path}`);
  }
  if (report.service_surface.topology_present !== true) {
    issues.push("central self-host web service surface missing from topology");
  }
  if (report.service_surface.boundary_documented !== true) {
    issues.push("central self-host web service boundary missing from docs");
  }
  return issues;
}

function writeJson(relativePath, value) {
  const absolute = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, `${JSON.stringify(value, null, 2)}\n`);
}

function writeText(relativePath, value) {
  const absolute = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, value);
}

function siblingMarkdownPath(relativePath) {
  return relativePath.replace(/\.json$/u, ".md");
}

function renderMarkdown(report) {
  const lines = [
    "# Central Readiness Report",
    "",
    `- Schema: \`${report.schema_version}\``,
    `- Status: \`${report.status}\``,
    `- Mode/backend: \`${report.mode}/${report.backend}\``,
    `- Generated: \`${report.generated_at}\``,
    "",
    "## API Surface",
    "",
    "| Endpoint | Router | Client |",
    "| --- | --- | --- |",
  ];
  for (const endpoint of report.api_surface.endpoints) {
    lines.push(
      `| \`${endpoint.path}\` | \`${endpoint.router_present ? "yes" : "no"}\` | \`${endpoint.client_present ? "yes" : "no"}\` |`,
    );
  }
  lines.push("", "## Schemas", "", "| Schema | Present |", "| --- | --- |");
  for (const schema of report.schema_surface.schema_files) {
    lines.push(`| \`${schema.path}\` | \`${schema.present ? "yes" : "no"}\` |`);
  }
  lines.push("", "## Config Surface", "", "| Config | Present | Schema Version |", "| --- | --- | --- |");
  for (const config of report.config_surface.config_files) {
    lines.push(
      `| \`${config.path}\` | \`${config.present ? "yes" : "no"}\` | \`${config.schema_version_present ? "yes" : "no"}\` |`,
    );
  }
  lines.push(
    "",
    "## Service Surface",
    "",
    "| Service | Module | Kind | Topology | Boundary |",
    "| --- | --- | --- | --- | --- |",
    `| \`${report.service_surface.id}\` | \`${report.service_surface.module_id}\` | \`${report.service_surface.kind}\` | \`${report.service_surface.topology_present ? "yes" : "no"}\` | \`${report.service_surface.boundary_documented ? "yes" : "no"}\` |`,
  );
  lines.push(
    "",
    "## Storage Contract",
    "",
    `- Contract: \`${report.storage_contract.schema_version}\``,
    `- Table contract present: \`${report.storage_contract.table_contract_present ? "yes" : "no"}\``,
    "",
    "## Runbook",
    "",
    `- Local readiness: \`${report.runbook.local_readiness}\``,
    `- Remote dry-run: \`${report.runbook.remote_dry_run}\``,
    `- Postgres smoke: \`${report.runbook.postgres_smoke}\``,
    "",
  );
  return lines.join("\n");
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
