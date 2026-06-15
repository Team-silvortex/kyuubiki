import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const APPS = ["hub-gui", "installer-gui", "workbench-gui"];

const sharedBridgeSource = path.join(ROOT, "apps/desktop-shared/ui/tauri-bridge.js");
const brandSource = path.join(ROOT, "assets/brand/brand.json");

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
  writeFile(
    path.join(ROOT, "apps", app, "ui/shared/tauri-bridge.js"),
    'export * from "../../../desktop-shared/ui/tauri-bridge.js";\n',
  );
  writeFile(
    path.join(ROOT, "apps", app, "ui/shared/runtime-status-summary.js"),
    'export {\n  formatRuntimeStatusReport,\n  renderRuntimeStatusPlane,\n} from "../../../desktop-shared/ui/runtime-status-summary.js";\n',
  );
  if (app === "installer-gui") {
    writeFile(
      path.join(ROOT, "apps", app, "ui/shared/desktop-shell.css"),
      '@import "../../../desktop-shared/ui/desktop-shell.css";\n\n.desktop-shell-button-primary {\n  background: linear-gradient(180deg, rgba(255, 174, 72, 0.28), rgba(79, 84, 93, 0.96));\n  border-color: rgba(255, 174, 72, 0.34);\n}\n',
    );
  } else {
    writeFile(path.join(ROOT, "apps", app, "ui/shared/desktop-shell.css"), '@import "../../../desktop-shared/ui/desktop-shell.css";\n');
  }
  copy(brandSource, path.join(ROOT, "apps", app, "ui/assets/brand.json"));
}

process.stdout.write(`synced desktop shared assets to ${APPS.join(", ")}\n`);
