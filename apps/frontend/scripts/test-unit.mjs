import { readdirSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TEST_ROOT = path.join(ROOT, "test");
const DOMAIN_FILTER = process.argv[2]?.trim().toLowerCase() ?? "";

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

function filterFiles(files, domainFilter) {
  if (!domainFilter) return files;
  return files.filter((file) =>
    path.relative(TEST_ROOT, file).toLowerCase().includes(domainFilter),
  );
}

if (!statSync(TEST_ROOT, { throwIfNoEntry: false })?.isDirectory()) {
  console.error("frontend unit test root is missing:", TEST_ROOT);
  process.exit(1);
}

const testFiles = filterFiles(listTestFiles(TEST_ROOT), DOMAIN_FILTER);
if (testFiles.length === 0) {
  console.error(
    DOMAIN_FILTER
      ? `no frontend unit tests matched domain filter: ${DOMAIN_FILTER}`
      : "no frontend unit tests found",
  );
  process.exit(1);
}

const result = spawnSync(
  "node",
  ["--test", "--experimental-strip-types", ...testFiles],
  {
    cwd: ROOT,
    stdio: "inherit",
    env: process.env,
  },
);

process.exit(result.status ?? 1);
