import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SRC_ROOT = path.join(ROOT, "src");
const MAX_LINES = 600;

const DEBT_LIMITS = new Map([
  ["src/components/workbench/workbench.tsx", 12250],
  ["src/lib/models/model-import.ts", 1445],
  ["src/lib/scripting/workbench-script-runtime.ts", 1040],
  ["src/lib/models/modeler.ts", 820],
]);

function listSourceFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) return listSourceFiles(absolute);
    if (!entry.isFile()) return [];
    if (!absolute.endsWith(".ts") && !absolute.endsWith(".tsx")) return [];
    return [absolute];
  });
}

function countLines(filePath) {
  const contents = readFileSync(filePath, "utf8");
  if (contents.length === 0) return 0;
  return contents.split("\n").length;
}

const violations = [];

for (const filePath of listSourceFiles(SRC_ROOT)) {
  const relative = path.relative(ROOT, filePath).replaceAll(path.sep, "/");
  const lines = countLines(filePath);
  const explicitLimit = DEBT_LIMITS.get(relative);
  const limit = explicitLimit ?? MAX_LINES;

  if (lines > limit) {
    violations.push({ relative, lines, limit, debtTracked: explicitLimit !== undefined });
  }
}

if (violations.length > 0) {
  const formatted = violations
    .sort((left, right) => right.lines - left.lines)
    .map(({ relative, lines, limit, debtTracked }) =>
      `${relative}: ${lines} lines (limit ${limit}${debtTracked ? ", debt guard" : ""})`,
    )
    .join("\n");

  console.error(
    [
      `File line-count guard failed. Default limit is ${MAX_LINES} lines.`,
      "Existing oversized files must not grow past their tracked debt limit until we split them further.",
      formatted,
    ].join("\n\n"),
  );
  process.exit(1);
}

const trackedSummary = [...DEBT_LIMITS.entries()]
  .map(([relative, limit]) => `${relative}<=${limit}`)
  .join(", ");

console.log(`File line-count guard passed. Default limit ${MAX_LINES}; tracked debt: ${trackedSummary}`);
