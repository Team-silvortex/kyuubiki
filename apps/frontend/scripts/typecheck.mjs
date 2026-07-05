import { existsSync, readdirSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const APP_ROOT = path.join(ROOT, "src", "app");
const TYPE_ROOT = path.join(ROOT, ".next", "types", "app");

const APP_ENTRYPOINTS = new Set(["layout.ts", "layout.tsx", "page.ts", "page.tsx", "route.ts"]);

function listAppEntrypointFiles(dir) {
  if (!existsSync(dir)) return [];

  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) return listAppEntrypointFiles(absolute);
    if (entry.isFile() && APP_ENTRYPOINTS.has(entry.name)) return [absolute];
    return [];
  });
}

function appEntrypointTypePath(entrypointFile) {
  const relative = path.relative(APP_ROOT, entrypointFile);
  const parsed = path.parse(relative);
  const typeRelative = path.join(parsed.dir, `${parsed.name}.ts`);
  return path.join(TYPE_ROOT, typeRelative);
}

function appTypesReady() {
  const entrypointFiles = listAppEntrypointFiles(APP_ROOT);
  if (entrypointFiles.length === 0) return existsSync(TYPE_ROOT);
  return entrypointFiles.every((entrypointFile) => existsSync(appEntrypointTypePath(entrypointFile)));
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: ROOT,
    stdio: "inherit",
    env: process.env,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

if (!appTypesReady()) {
  run("npm", ["run", "build"]);
}

run("npx", ["tsc", "--noEmit", "--incremental", "false"]);
run("node", ["./scripts/check-file-lines.mjs"]);
