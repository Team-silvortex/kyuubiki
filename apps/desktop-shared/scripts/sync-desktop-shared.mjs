import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const APPS = ["hub-gui", "installer-gui", "workbench-gui"];

const brandSource = path.join(ROOT, "assets/brand/brand.json");
const sharedUiDir = path.join(ROOT, "apps/desktop-shared/ui");
const sharedUiFiles = [
  "desktop-shell.css",
  "desktop-shell-runtime-mesh.css",
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

function writeFile(target, contents) {
  ensureDir(path.dirname(target));
  fs.writeFileSync(target, contents);
}

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
}

process.stdout.write(`synced desktop shared assets to ${APPS.join(", ")}\n`);
