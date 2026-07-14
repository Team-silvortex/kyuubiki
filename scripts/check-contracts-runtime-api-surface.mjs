import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const surfacePath = path.join(
  repoRoot,
  valueAfter("--surface") ?? "config/architecture/contracts-runtime-api-surface.json",
);
const schemaPath = "schemas/contracts-runtime-api-surface.schema.json";

const args = new Set(process.argv.slice(2));

if (args.has("--self-test")) {
  const report = validateSurface({
    $schema: "../../schemas/contracts-runtime-api-surface.schema.json",
    schema_version: "kyuubiki.contracts-runtime-api-surface/v1",
    module_id: "contracts",
    runtime_api: {
      owner: "contracts",
      contract_families: [
        {
          id: "frontend-runtime-api",
          sources: ["scripts/check-contracts-runtime-api-surface.mjs"],
          client_surfaces: ["workbench-shell"],
          stability_contracts: ["typed payloads"],
        },
        {
          id: "protocol-runtime-api",
          sources: ["scripts/check-contracts-runtime-api-surface.mjs"],
          client_surfaces: ["runtime-agent-cli"],
          stability_contracts: ["TaskIR"],
        },
        {
          id: "orchestra-runtime-api",
          sources: ["scripts/check-contracts-runtime-api-surface.mjs"],
          client_surfaces: ["orchestra-control-plane"],
          stability_contracts: ["control-plane surface"],
        },
        {
          id: "central-store-runtime-api",
          sources: [
            "scripts/check-contracts-runtime-api-surface.mjs",
            "config/architecture/central-store-contract.json",
            "schemas/central-store-contract-check.schema.json",
            "apps/web/lib/kyuubiki_web/storage/central_database.ex",
            "schemas/central-database-status.schema.json",
          ],
          client_surfaces: ["workbench-shell"],
          service_surfaces: [
            {
              id: "central-web-service",
              module_id: "orchestra-control-plane",
              kind: "self_host_web",
            },
          ],
          stability_contracts: [
            "central store catalog",
            "database status",
            "self-hosted website service surface",
            "central database table contract",
          ],
        },
      ],
      verification_commands: ["node scripts/check-contracts-runtime-api-surface.mjs"],
    },
  });

  if (report.errors.length > 0) {
    for (const error of report.errors) {
      console.error(`contracts-runtime-api-surface self-test: ${error}`);
    }
    process.exit(1);
  }

  console.log("contracts runtime API surface self-test passed");
  process.exit(0);
}

const surface = JSON.parse(fs.readFileSync(surfacePath, "utf8"));
const report = validateSurface(surface);

if (report.errors.length > 0) {
  for (const error of report.errors) {
    console.error(`contracts-runtime-api-surface: ${error}`);
  }
  process.exit(1);
}

console.log(
  `contracts runtime API surface passed: ${report.familyCount} contract family(s)`,
);

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : null;
}

function validateSurface(surface) {
  const errors = [];

  if (surface.schema_version !== "kyuubiki.contracts-runtime-api-surface/v1") {
    errors.push("unexpected contracts runtime API surface schema_version");
  }

  if (surface.module_id !== "contracts") {
    errors.push("contracts runtime API surface must belong to contracts module");
  }
  if (surface.$schema !== "../../schemas/contracts-runtime-api-surface.schema.json") {
    errors.push("contracts runtime API surface must declare its schema");
  }
  if (surface.runtime_api?.owner !== "contracts") {
    errors.push("contracts runtime API owner must be contracts");
  }
  validateSchemaContract(errors);

  const families = surface.runtime_api?.contract_families ?? [];
  const familyIds = new Set();
  for (const family of families) {
    if (familyIds.has(family.id)) errors.push(`duplicate contract family: ${family.id}`);
    familyIds.add(family.id);
  }
  const requiredFamilies = [
    "frontend-runtime-api",
    "protocol-runtime-api",
    "orchestra-runtime-api",
    "central-store-runtime-api",
  ];

  for (const familyId of requiredFamilies) {
    if (!families.some((family) => family.id === familyId)) {
      errors.push(`missing contract family: ${familyId}`);
    }
  }

  const centralFamily = families.find((family) => family.id === "central-store-runtime-api");
  if (centralFamily) {
    requireIncludes(
      errors,
      centralFamily.sources,
      "config/architecture/central-store-contract.json",
      "central-store-runtime-api source",
    );
    requireIncludes(
      errors,
      centralFamily.sources,
      "schemas/central-store-contract-check.schema.json",
      "central-store-runtime-api source",
    );
    requireIncludes(
      errors,
      centralFamily.sources,
      "apps/web/lib/kyuubiki_web/storage/central_database.ex",
      "central-store-runtime-api source",
    );
    requireIncludes(
      errors,
      centralFamily.sources,
      "schemas/central-database-status.schema.json",
      "central-store-runtime-api source",
    );
    requireIncludes(
      errors,
      centralFamily.stability_contracts,
      "database status",
      "central-store-runtime-api stability contract",
    );
    requireIncludes(
      errors,
      centralFamily.stability_contracts,
      "self-hosted website service surface",
      "central-store-runtime-api stability contract",
    );
    requireServiceSurface(
      errors,
      centralFamily.service_surfaces,
      {
        id: "central-web-service",
        module_id: "orchestra-control-plane",
        kind: "self_host_web",
      },
      "central-store-runtime-api service surface",
    );
    requireIncludes(
      errors,
      centralFamily.stability_contracts,
      "central database table contract",
      "central-store-runtime-api stability contract",
    );
  }

  for (const family of families) {
    if (!Array.isArray(family.sources) || family.sources.length === 0) {
      errors.push(`${family.id} has no source files`);
      continue;
    }

    for (const source of family.sources) {
      if (!isRepoRelativePath(source)) {
        errors.push(`${family.id} source must be repository-relative: ${source}`);
        continue;
      }
      if (!fs.existsSync(path.join(repoRoot, source))) {
        errors.push(`${family.id} source does not exist: ${source}`);
      }
    }

    if (!Array.isArray(family.client_surfaces) || family.client_surfaces.length === 0) {
      errors.push(`${family.id} has no client surfaces`);
    }

    if (
      !Array.isArray(family.stability_contracts) ||
      family.stability_contracts.length === 0
    ) {
      errors.push(`${family.id} has no stability contracts`);
    }
  }
  for (const command of surface.runtime_api?.verification_commands ?? []) {
    if (!command.startsWith("node ") && !command.startsWith("make ")) {
      errors.push(`unsupported verification command prefix: ${command}`);
    }
  }

  return { errors, familyCount: families.length };
}

function validateSchemaContract(errors) {
  const schemaAbsolutePath = path.join(repoRoot, schemaPath);
  if (!fs.existsSync(schemaAbsolutePath)) {
    errors.push(`schema does not exist: ${schemaPath}`);
    return;
  }
  const schemaText = fs.readFileSync(schemaAbsolutePath, "utf8");
  const schema = JSON.parse(schemaText);
  if (schema.properties?.schema_version?.const !== "kyuubiki.contracts-runtime-api-surface/v1") {
    errors.push("contracts runtime API schema version const mismatch");
  }
  if (!schemaText.includes("repoPath") || !schemaText.includes("^(?!/)")) {
    errors.push("contracts runtime API schema must define repo-relative path pattern");
  }
  if (!schemaText.includes("serviceSurface")) {
    errors.push("contracts runtime API schema must define service surface shape");
  }
}

function isRepoRelativePath(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    !path.isAbsolute(value) &&
    !/^[A-Za-z]:/u.test(value) &&
    !value.split(/[\\/]/u).includes("..")
  );
}

function requireIncludes(errors, values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    errors.push(`${label} missing ${expected}`);
  }
}

function requireServiceSurface(errors, surfaces, expected, label) {
  if (!Array.isArray(surfaces)) {
    errors.push(`${label} missing ${expected.id}`);
    return;
  }
  const actual = surfaces.find((surface) => surface.id === expected.id);
  if (!actual) {
    errors.push(`${label} missing ${expected.id}`);
    return;
  }
  for (const [field, value] of Object.entries(expected)) {
    if (actual[field] !== value) {
      errors.push(`${label} ${expected.id} ${field} mismatch`);
    }
  }
}
