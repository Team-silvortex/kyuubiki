import { existsSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const APP_ROOT = path.join(ROOT, "src", "app");
const TYPE_ROOT = path.join(ROOT, ".next", "types", "app");

function listRouteFiles(dir) {
  if (!existsSync(dir)) return [];

  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) return listRouteFiles(absolute);
    if (entry.isFile() && entry.name === "route.ts") return [absolute];
    return [];
  });
}

function relativeRouteTypePath(routeFile) {
  const relative = path.relative(APP_ROOT, routeFile);
  return path.join(TYPE_ROOT, relative);
}

function routeTypesReady() {
  const routeFiles = listRouteFiles(APP_ROOT);
  if (routeFiles.length === 0) return existsSync(TYPE_ROOT);
  return routeFiles.every((routeFile) => existsSync(relativeRouteTypePath(routeFile)));
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

if (!routeTypesReady()) {
  run("npm", ["run", "build"]);
}

run("npx", ["tsc", "--noEmit", "--incremental", "false"]);
