import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const surfacePath = path.join(
  repoRoot,
  "config/architecture/contracts-runtime-api-surface.json",
);

const args = new Set(process.argv.slice(2));

if (args.has("--self-test")) {
  const report = validateSurface({
    schema_version: "kyuubiki.contracts-runtime-api-surface/v1",
    module_id: "contracts",
    runtime_api: {
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
          stability_contracts: [
            "central store catalog",
            "database status",
            "central database table contract",
          ],
        },
      ],
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

function validateSurface(surface) {
  const errors = [];

  if (surface.schema_version !== "kyuubiki.contracts-runtime-api-surface/v1") {
    errors.push("unexpected contracts runtime API surface schema_version");
  }

  if (surface.module_id !== "contracts") {
    errors.push("contracts runtime API surface must belong to contracts module");
  }

  const families = surface.runtime_api?.contract_families ?? [];
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

  return { errors, familyCount: families.length };
}

function requireIncludes(errors, values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    errors.push(`${label} missing ${expected}`);
  }
}
