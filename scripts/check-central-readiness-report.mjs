#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const input = valueAfter("--in") ?? process.env.IN ?? "tmp/central-readiness-report.json";
const markdownInput =
  valueAfter("--markdown-in") ?? process.env.MARKDOWN_IN ?? input.replace(/\.json$/u, ".md");
const requiredEndpoints = [
  "/api/v1/central/catalog",
  "/api/v1/central/session-policy",
  "/api/v1/central/publish-policy",
  "/api/v1/central/publish-readiness",
  "/api/v1/central/database-policy",
  "/api/v1/central/provenance-policy",
  "/api/v1/central/database-status",
];
const requiredSchemas = [
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
const requiredConfigs = ["config/architecture/central-store-contract.json"];

if (process.argv.includes("--self-test")) {
  const issues = validateReport({
    schema_version: "kyuubiki.central-readiness-report/v1",
    status: "ok",
    api_surface: {
      endpoints: requiredEndpoints.map((endpoint) => ({
        path: endpoint,
        router_present: true,
        client_present: true,
      })),
    },
    schema_surface: {
      schema_files: requiredSchemas.map((schema) => ({ path: schema, present: true })),
    },
    config_surface: {
      config_files: requiredConfigs.map((config) => ({
        path: config,
        present: true,
        schema_version_present: true,
      })),
    },
    storage_contract: {
      schema_version: "kyuubiki.central-database-contract/v1",
      table_contract_present: true,
    },
    runbook: {
      local_readiness: "make check-central-database-readiness MODE=local BACKEND=sqlite",
      remote_dry_run: "make remote-central-database-smoke REMOTE=kyuubiki-lab",
      postgres_smoke: "RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke",
    },
  });
  issues.push(...validateMarkdown(markdownFixture()));
  if (issues.length > 0) fail(`self-test failed: ${issues.join("; ")}`);
  console.log("central readiness report checker self-test passed");
  process.exit(0);
}

const report = JSON.parse(fs.readFileSync(path.join(repoRoot, input), "utf8"));
const issues = validateReport(report);
if (fs.existsSync(path.join(repoRoot, markdownInput))) {
  issues.push(...validateMarkdown(fs.readFileSync(path.join(repoRoot, markdownInput), "utf8")));
} else {
  issues.push(`missing markdown summary ${markdownInput}`);
}
if (issues.length > 0) fail(`central readiness report failed: ${issues.join("; ")}`);
console.log(`central readiness report passed: ${input}`);

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : null;
}

function validateReport(report) {
  const issues = [];
  if (report.schema_version !== "kyuubiki.central-readiness-report/v1") {
    issues.push("unexpected schema_version");
  }
  if (report.status !== "ok") issues.push("status must be ok");
  const endpoints = report.api_surface?.endpoints ?? [];
  for (const endpoint of requiredEndpoints) {
    const actual = endpoints.find((entry) => entry.path === endpoint);
    if (!actual) issues.push(`missing endpoint ${endpoint}`);
    if (actual && !actual.router_present) issues.push(`router missing ${endpoint}`);
    if (actual && !actual.client_present) issues.push(`client missing ${endpoint}`);
  }
  const schemaFiles = report.schema_surface?.schema_files ?? [];
  for (const schema of requiredSchemas) {
    const actual = schemaFiles.find((entry) => entry.path === schema);
    if (!actual) issues.push(`missing schema ${schema}`);
    if (actual && !actual.present) issues.push(`schema not present ${schema}`);
  }
  const configFiles = report.config_surface?.config_files ?? [];
  for (const config of requiredConfigs) {
    const actual = configFiles.find((entry) => entry.path === config);
    if (!actual) issues.push(`missing config ${config}`);
    if (actual && !actual.present) issues.push(`config not present ${config}`);
    if (actual && !actual.schema_version_present) issues.push(`config schema version missing ${config}`);
  }
  if (report.storage_contract?.schema_version !== "kyuubiki.central-database-contract/v1") {
    issues.push("storage contract schema version mismatch");
  }
  if (report.storage_contract?.table_contract_present !== true) {
    issues.push("storage table contract missing");
  }
  issues.push(...unsafeTextIssues(JSON.stringify(report)));
  return issues;
}

function unsafeTextIssues(text) {
  const patterns = [
    [/Thx\d+/u, "raw lab password-like token"],
    [/DATABASE_URL=.*:\/\/[^:\s]+:[^@\s]+@/u, "inline DATABASE_URL secret"],
    [/ecto:\/\/[^:\s]+:[^@\s]+@/u, "inline ecto credential"],
    [/ssh_pass(word)?/iu, "ssh password field"],
  ];
  return patterns
    .filter(([pattern]) => pattern.test(text))
    .map(([, label]) => `unsafe text detected: ${label}`);
}

function validateMarkdown(markdown) {
  const issues = [];
  const requiredText = [
    "# Central Readiness Report",
    "## API Surface",
    "## Schemas",
    "## Config Surface",
    "## Storage Contract",
    "## Runbook",
    "kyuubiki.central-readiness-report/v1",
    "kyuubiki.central-database-contract/v1",
    "make check-central-database-readiness MODE=local BACKEND=sqlite",
    "make remote-central-database-smoke REMOTE=kyuubiki-lab",
    "RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke",
    ...requiredEndpoints,
    ...requiredSchemas,
    ...requiredConfigs,
  ];
  for (const text of requiredText) {
    if (!markdown.includes(text)) issues.push(`markdown missing ${text}`);
  }
  issues.push(...unsafeTextIssues(markdown));
  return issues;
}

function markdownFixture() {
  return [
    "# Central Readiness Report",
    "",
    "- Schema: `kyuubiki.central-readiness-report/v1`",
    "- Status: `ok`",
    "",
    "## API Surface",
    ...requiredEndpoints.map((endpoint) => `| \`${endpoint}\` | \`yes\` | \`yes\` |`),
    "",
    "## Schemas",
    ...requiredSchemas.map((schema) => `| \`${schema}\` | \`yes\` |`),
    "",
    "## Config Surface",
    ...requiredConfigs.map((config) => `| \`${config}\` | \`yes\` | \`yes\` |`),
    "",
    "## Storage Contract",
    "- Contract: `kyuubiki.central-database-contract/v1`",
    "",
    "## Runbook",
    "- Local readiness: `make check-central-database-readiness MODE=local BACKEND=sqlite`",
    "- Remote dry-run: `make remote-central-database-smoke REMOTE=kyuubiki-lab`",
    "- Postgres smoke: `RUN_DB_SMOKE=1 MODE=cloud BACKEND=postgres make test-central-database-smoke`",
  ].join("\n");
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
