import { readdirSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TEST_ROOT = path.join(ROOT, "test");
const COVERAGE_ENABLED = process.env.KYUUBIKI_FRONTEND_COVERAGE === "1";
const COVERAGE_THRESHOLDS = {
  lines: process.env.KYUUBIKI_FRONTEND_COVERAGE_LINES ?? "50",
  branches: process.env.KYUUBIKI_FRONTEND_COVERAGE_BRANCHES ?? "60",
  functions: process.env.KYUUBIKI_FRONTEND_COVERAGE_FUNCTIONS ?? "55",
};
const DOMAIN_FILTERS = process.argv
  .slice(2)
  .map((filter) => filter.trim().toLowerCase())
  .filter(Boolean);

function listTestFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) return listTestFiles(absolute);
    if (!entry.isFile()) return [];
    if (!absolute.endsWith(".test.ts") && !absolute.endsWith(".test.mjs")) {
      return [];
    }
    return [absolute];
  });
}

function filterFiles(files, domainFilters) {
  if (domainFilters.length === 0) return files;
  return files.filter((file) => {
    const relative = path.relative(TEST_ROOT, file).toLowerCase();
    return domainFilters.some((filter) => relative.includes(filter));
  });
}

if (!statSync(TEST_ROOT, { throwIfNoEntry: false })?.isDirectory()) {
  console.error("frontend unit test root is missing:", TEST_ROOT);
  process.exit(1);
}

const testFiles = filterFiles(listTestFiles(TEST_ROOT), DOMAIN_FILTERS);
if (testFiles.length === 0) {
  console.error(
    DOMAIN_FILTERS.length > 0
      ? `no frontend unit tests matched domain filter(s): ${DOMAIN_FILTERS.join(", ")}`
      : "no frontend unit tests found",
  );
  process.exit(1);
}

const nodeArgs = [
  "--import",
  "./test/support/register-alias-loader.mjs",
  ...(COVERAGE_ENABLED
    ? [
        "--experimental-test-coverage",
        "--test-coverage-include=src/**/*.ts",
        "--test-coverage-include=src/**/*.tsx",
        "--test-coverage-exclude=src/**/*.d.ts",
        `--test-coverage-lines=${COVERAGE_THRESHOLDS.lines}`,
        `--test-coverage-branches=${COVERAGE_THRESHOLDS.branches}`,
        `--test-coverage-functions=${COVERAGE_THRESHOLDS.functions}`,
      ]
    : []),
  "--test",
  ...testFiles,
];

const result = spawnSync(
  "node",
  nodeArgs,
  {
    cwd: ROOT,
    stdio: "inherit",
    env: process.env,
  },
);

process.exit(result.status ?? 1);
