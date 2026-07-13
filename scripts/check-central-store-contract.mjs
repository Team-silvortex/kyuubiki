#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const args = new Set(process.argv.slice(2));
const configPath =
  valueAfter("--config") ??
  process.env.CONFIG ??
  "config/architecture/central-store-contract.json";
const config = JSON.parse(readText(configPath));
const configSchemaPath = "schemas/central-store-contract-check.schema.json";

if (args.has("--self-test")) {
  const report = validate({ files: selfTestFiles() });
  if (report.errors.length > 0) {
    report.errors.forEach((error) => console.error(`central-store-contract self-test: ${error}`));
    process.exit(1);
  }

  console.log("central store contract self-test passed");
  process.exit(0);
}

const report = validate({
  files: readRuntimeFiles(),
});

if (report.errors.length > 0) {
  report.errors.forEach((error) => console.error(`central-store-contract: ${error}`));
  process.exit(1);
}

console.log(
  `central store contract passed: ${report.schemaCount} schema(s), ${report.requiredFileCount} required file(s), ${report.textCheckCount} text check(s)`,
);

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : null;
}

function allContractFiles() {
  const contracts = Array.isArray(config.contracts) ? config.contracts : [];
  const requiredFiles = Array.isArray(config.required_files) ? config.required_files : [];
  const textChecks = Array.isArray(config.text_checks) ? config.text_checks : [];

  return unique([
    configPath,
    configSchemaPath,
    ...contracts.flatMap((contract) => [
      contract.schema_path,
      contract.backend_path,
      contract.frontend_types_path,
    ]),
    ...requiredFiles,
    ...textChecks.map((check) => check.file),
  ].filter((value) => typeof value === "string" && value.trim() !== ""));
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function readRuntimeFiles() {
  const bootstrapFiles = {
    [configPath]: JSON.stringify(config),
    [configSchemaPath]: readText(configSchemaPath),
  };
  const bootstrapReport = validateConfig({ files: bootstrapFiles });
  if (bootstrapReport.errors.length > 0) return bootstrapFiles;

  return Object.fromEntries(allContractFiles().map((relativePath) => [relativePath, readText(relativePath)]));
}

function selfTestFiles() {
  const files = Object.fromEntries(allContractFiles().map((file) => [file, ""]));
  files[configPath] = JSON.stringify(config);
  files[configSchemaPath] = JSON.stringify({
    properties: { schema_version: { const: "kyuubiki.central-store-contract-check/v1" } },
    $defs: {
      repoPath: { pattern: repoPathPattern() },
    },
  });

  for (const contract of config.contracts) {
    files[contract.schema_path] = JSON.stringify({
      properties: { schema_version: { const: contract.schema_version } },
    });
    files[contract.backend_path] += `\n${contract.schema_version}`;
    files[contract.frontend_types_path] += `\n${contract.schema_version}`;
  }

  for (const check of config.text_checks) {
    files[check.file] = appendFixtureText(files[check.file], check.text, check.file);
  }

  return files;
}

function validate({ files }) {
  const errors = [];
  errors.push(...validateConfig({ files }).errors);
  if (errors.length > 0) return buildReport(errors);

  for (const file of allContractFiles()) {
    if (typeof files[file] !== "string") {
      errors.push(`missing required file: ${file}`);
    }
  }

  const contracts = Array.isArray(config.contracts) ? config.contracts : [];
  const textChecks = Array.isArray(config.text_checks) ? config.text_checks : [];

  for (const contract of contracts) {
    validateSchema(contract, files, errors);
    validateTextContains(contract.backend_path, contract.schema_version, "backend", files, errors);
    validateTextContains(contract.frontend_types_path, contract.schema_version, "frontend types", files, errors);
  }

  for (const check of textChecks) {
    validateTextContains(check.file, check.text, check.label, files, errors);
  }

  return buildReport(errors);
}

function buildReport(errors) {
  const contracts = Array.isArray(config.contracts) ? config.contracts : [];
  const requiredFiles = Array.isArray(config.required_files) ? config.required_files : [];
  const textChecks = Array.isArray(config.text_checks) ? config.text_checks : [];

  return {
    errors,
    schemaCount: contracts.length,
    requiredFileCount: requiredFiles.length,
    textCheckCount: textChecks.length,
  };
}

function validateConfig({ files }) {
  const errors = [];

  if (config.schema_version !== "kyuubiki.central-store-contract-check/v1") {
    errors.push("unexpected central store contract config schema_version");
  }
  validateConfigShape(errors);
  validateConfigInvariants(errors);
  validateConfigSchema(files, errors);

  return { errors };
}

function validateSchema(contract, files, errors) {
  try {
    const schema = JSON.parse(files[contract.schema_path]);
    const actual = schema.properties?.schema_version?.const;
    if (actual !== contract.schema_version) {
      errors.push(`${contract.id} schema const must be ${contract.schema_version}`);
    }
  } catch (error) {
    errors.push(`${contract.schema_path}: ${error.message}`);
  }
}

function validateConfigSchema(files, errors) {
  try {
    const schema = JSON.parse(files[configSchemaPath]);
    const actual = schema.properties?.schema_version?.const;
    if (actual !== "kyuubiki.central-store-contract-check/v1") {
      errors.push("central store contract config schema const mismatch");
    }
    if (schema.$defs?.repoPath?.pattern !== repoPathPattern()) {
      errors.push("central store contract config schema must define repo-relative path pattern");
    }
  } catch (error) {
    errors.push(`${configSchemaPath}: ${error.message}`);
  }
}

function validateConfigShape(errors) {
  requireArray(config.contracts, "contracts", errors);
  requireArray(config.required_files, "required_files", errors);
  requireArray(config.text_checks, "text_checks", errors);

  const contracts = Array.isArray(config.contracts) ? config.contracts : [];
  const textChecks = Array.isArray(config.text_checks) ? config.text_checks : [];

  for (const contract of contracts) {
    for (const field of ["id", "schema_version", "schema_path", "backend_path", "frontend_types_path"]) {
      requireString(contract?.[field], `contract.${field}`, errors);
    }
  }

  for (const check of textChecks) {
    for (const field of ["file", "text", "label"]) {
      requireString(check?.[field], `text_check.${field}`, errors);
    }
  }
}

function validateConfigInvariants(errors) {
  const contracts = Array.isArray(config.contracts) ? config.contracts : [];
  const requiredFiles = Array.isArray(config.required_files) ? config.required_files : [];
  const textChecks = Array.isArray(config.text_checks) ? config.text_checks : [];

  reportDuplicates(contracts.map((contract) => contract.id), "contract id", errors);
  reportDuplicates(contracts.map((contract) => contract.schema_path), "contract schema path", errors);
  reportDuplicates(textChecks.map((check) => `${check.file}::${check.text}`), "text check", errors);

  const paths = [
    configPath,
    configSchemaPath,
    ...contracts.flatMap((contract) => [
      contract.schema_path,
      contract.backend_path,
      contract.frontend_types_path,
    ]),
    ...requiredFiles,
    ...textChecks.map((check) => check.file),
  ];

  for (const value of paths) {
    if (typeof value === "string") validateRepoRelativePath(value, errors);
  }

  const serialized = JSON.stringify(config);
  for (const [pattern, label] of unsafePatterns()) {
    if (pattern.test(serialized)) errors.push(`unsafe central store contract config text: ${label}`);
  }
}

function reportDuplicates(values, label, errors) {
  const seen = new Set();
  const duplicates = new Set();

  for (const value of values) {
    if (typeof value !== "string" || value.trim() === "") continue;
    if (seen.has(value)) duplicates.add(value);
    seen.add(value);
  }

  for (const value of duplicates) {
    errors.push(`duplicate ${label}: ${value}`);
  }
}

function validateRepoRelativePath(value, errors) {
  if (path.isAbsolute(value) || /^[A-Za-z]:[\\/]/u.test(value)) {
    errors.push(`path must be repository-relative: ${value}`);
  }

  if (value.split(/[\\/]/u).includes("..")) {
    errors.push(`path must not traverse parent directories: ${value}`);
  }
}

function unsafePatterns() {
  return [
    [/Thx\d+/u, "raw lab password-like token"],
    [/DATABASE_URL=.*:\/\/[^:\s]+:[^@\s]+@/u, "inline DATABASE_URL secret"],
    [/ecto:\/\/[^:\s]+:[^@\s]+@/u, "inline ecto credential"],
    [/ssh_pass(word)?/iu, "ssh password field"],
  ];
}

function repoPathPattern() {
  return "^(?!/)(?![A-Za-z]:)(?!.*(^|/)\\.\\.(/|$)).+";
}

function requireArray(value, label, errors) {
  if (!Array.isArray(value) || value.length === 0) errors.push(`${label} must be a non-empty array`);
}

function requireString(value, label, errors) {
  if (typeof value !== "string" || value.trim() === "") {
    errors.push(`${label} must be a non-empty string`);
  }
}

function validateTextContains(file, needle, label, files, errors) {
  if (!files[file]?.includes(needle)) {
    errors.push(`${label} missing ${needle} in ${file}`);
  }
}

function appendFixtureText(current, text, file) {
  if (!file.endsWith(".json")) return `${current}\n${text}`;

  try {
    const parsed = current.trim() ? JSON.parse(current) : {};
    parsed._self_test_text = [parsed._self_test_text, text].filter(Boolean).join("\n");
    return JSON.stringify(parsed);
  } catch (_error) {
    return `${current}\n${text}`;
  }
}

function unique(values) {
  return values.filter((value, index) => values.indexOf(value) === index);
}
