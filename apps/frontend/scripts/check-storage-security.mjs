import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SRC_ROOT = path.join(ROOT, "src");

const SOURCE_EXTENSIONS = new Set([".ts", ".tsx"]);
const STORAGE_CALL_PATTERN = /\b(?:localStorage|sessionStorage)\.(?:setItem|getItem)\b/;
const SENSITIVE_PATTERN =
  /(?:api[-_]?key|authorization|bearer|credential|password|passwd|secret|session[-_]?key|token)/i;

const ALLOWED_SENSITIVE_STORAGE_LINES = new Set([
  "src/lib/workbench/helpers.ts:const rawSecrets = window.sessionStorage.getItem(WORKBENCH_SECRETS_KEY);",
]);

function listSourceFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) return listSourceFiles(absolute);
    if (!entry.isFile()) return [];
    return SOURCE_EXTENSIONS.has(path.extname(absolute)) ? [absolute] : [];
  });
}

function toRelative(filePath) {
  return path.relative(ROOT, filePath).replaceAll(path.sep, "/");
}

function stableLineKey(relative, line) {
  return `${relative}:${line.trim()}`;
}

function formatViolation(violation) {
  return `${violation.relative}:${violation.lineNumber}: ${violation.line.trim()}`;
}

const violations = [];

for (const filePath of listSourceFiles(SRC_ROOT)) {
  const relative = toRelative(filePath);
  const lines = readFileSync(filePath, "utf8").split("\n");

  lines.forEach((line, index) => {
    if (!STORAGE_CALL_PATTERN.test(line) || !SENSITIVE_PATTERN.test(line)) return;
    if (ALLOWED_SENSITIVE_STORAGE_LINES.has(stableLineKey(relative, line))) return;
    violations.push({ relative, lineNumber: index + 1, line });
  });
}

if (violations.length > 0) {
  console.error(
    [
      "Storage security guard failed.",
      "Do not read or write token, password, credential, API key, or secret-shaped values through browser storage.",
      "Use in-memory workbench secrets or a platform credential vault boundary instead.",
      "",
      violations.map(formatViolation).join("\n"),
    ].join("\n"),
  );
  process.exit(1);
}

console.log("Storage security guard passed. Sensitive browser storage calls were not found.");
