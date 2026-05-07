import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const APPS = ["hub-gui", "installer-gui", "workbench-gui"];

const sharedBridgeSource = path.join(ROOT, "apps/desktop-shared/ui/tauri-bridge.js");
const sharedCssSource = path.join(ROOT, "apps/desktop-shared/ui/desktop-shell.css");
const brandSource = path.join(ROOT, "assets/brand/brand.json");

function ensureDir(target) {
  fs.mkdirSync(target, { recursive: true });
}

function copy(source, target) {
  ensureDir(path.dirname(target));
  fs.copyFileSync(source, target);
}

for (const app of APPS) {
  copy(sharedBridgeSource, path.join(ROOT, "apps", app, "ui/shared/tauri-bridge.js"));
  copy(sharedCssSource, path.join(ROOT, "apps", app, "ui/shared/desktop-shell.css"));
  copy(brandSource, path.join(ROOT, "apps", app, "ui/assets/brand.json"));
}

process.stdout.write(`synced desktop shared assets to ${APPS.join(", ")}\n`);
