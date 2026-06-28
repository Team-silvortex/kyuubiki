import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const rootDir = path.resolve(appDir, "../..");
const tscBin = path.join(rootDir, "apps/frontend/node_modules/.bin/tsc");

execFileSync(tscBin, ["-p", path.join(appDir, "tsconfig.json")], {
  cwd: appDir,
  stdio: "inherit",
});

for (const generatedTypeOnlyFile of ["hub-i18n-types.js"]) {
  fs.rmSync(path.join(appDir, "ui", generatedTypeOnlyFile), { force: true });
}
