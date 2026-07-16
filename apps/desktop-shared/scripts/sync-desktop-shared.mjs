import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const APPS = ["hub-gui", "installer-gui", "workbench-gui"];

const brandSource = path.join(ROOT, "assets/brand/brand.json");
const languagePackSourceDir = path.join(ROOT, "language-packs");
const sharedUiDir = path.join(ROOT, "apps/desktop-shared/ui");
const desktopSharedDir = path.join(ROOT, "apps/desktop-shared");
const sharedUiFiles = [
  "desktop-shell.css",
  "desktop-shell-runtime-mesh.css",
  "language-pack-loader.js",
  "platform.js",
  "runtime-status-model.js",
  "runtime-status-summary.js",
  "tauri-bridge.js",
];

function ensureDir(target) {
  fs.mkdirSync(target, { recursive: true });
}

function copy(source, target) {
  ensureDir(path.dirname(target));
  fs.copyFileSync(source, target);
}

function copyTree(source, target) {
  ensureDir(target);
  for (const entry of fs.readdirSync(source, { withFileTypes: true })) {
    const sourcePath = path.join(source, entry.name);
    const targetPath = path.join(target, entry.name);
    if (entry.isDirectory()) {
      copyTree(sourcePath, targetPath);
    } else if (entry.isFile()) {
      copy(sourcePath, targetPath);
    }
  }
}

function syncLanguagePacks(app) {
  const target = path.join(ROOT, "apps", app, "ui/language-packs");
  fs.rmSync(target, { recursive: true, force: true });
  copy(path.join(languagePackSourceDir, "catalog.json"), path.join(target, "catalog.json"));
  for (const surface of ["hub", "workbench"]) {
    copyTree(path.join(languagePackSourceDir, surface), path.join(target, surface));
  }
}

function writeFile(target, contents) {
  ensureDir(path.dirname(target));
  fs.writeFileSync(target, contents);
}

function compileDesktopSharedTypeScript() {
  const tscBin = path.join(ROOT, "apps/frontend/node_modules/.bin/tsc");
  execFileSync(tscBin, ["-p", path.join(desktopSharedDir, "tsconfig.json")], {
    cwd: desktopSharedDir,
    stdio: "inherit",
  });
  fs.rmSync(path.join(sharedUiDir, "runtime-status-types.js"), { force: true });
}

compileDesktopSharedTypeScript();

for (const app of APPS) {
  const sharedTargetDir = path.join(ROOT, "apps", app, "ui/shared");
  for (const file of sharedUiFiles) {
    copy(path.join(sharedUiDir, file), path.join(sharedTargetDir, file));
  }
  if (app === "installer-gui") {
    fs.appendFileSync(
      path.join(sharedTargetDir, "desktop-shell.css"),
      "\n.desktop-shell-button-primary {\n  background: linear-gradient(180deg, rgba(255, 174, 72, 0.28), rgba(79, 84, 93, 0.96));\n  border-color: rgba(255, 174, 72, 0.34);\n}\n",
    );
  }
  copy(brandSource, path.join(ROOT, "apps", app, "ui/assets/brand.json"));
  syncLanguagePacks(app);
}

process.stdout.write(`synced desktop shared assets to ${APPS.join(", ")}\n`);
