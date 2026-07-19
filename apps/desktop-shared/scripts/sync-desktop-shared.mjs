import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const APPS = ["hub-gui", "installer-gui", "workbench-gui"];
const CHECK_ONLY = process.argv.includes("--check");
const unknownArgs = process.argv.slice(2).filter((argument) => argument !== "--check");

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
const generatedUiFiles = sharedUiFiles.filter((file) => file.endsWith(".js"));
const installerShellOverride = "\n.desktop-shell-button-primary {\n  background: linear-gradient(180deg, rgba(255, 174, 72, 0.28), rgba(79, 84, 93, 0.96));\n  border-color: rgba(255, 174, 72, 0.34);\n}\n";

if (unknownArgs.length > 0) {
  throw new Error(`unknown argument(s): ${unknownArgs.join(", ")}`);
}

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

function filesInTree(root, relative = "") {
  const current = path.join(root, relative);
  return fs.readdirSync(current, { withFileTypes: true }).flatMap((entry) => {
    const child = path.join(relative, entry.name);
    return entry.isDirectory() ? filesInTree(root, child) : [child];
  });
}

function assertSameFile(source, target) {
  if (!fs.existsSync(target)) {
    throw new Error(`missing synchronized file: ${path.relative(ROOT, target)}`);
  }
  if (!fs.readFileSync(source).equals(fs.readFileSync(target))) {
    throw new Error(`synchronized file differs: ${path.relative(ROOT, target)}`);
  }
}

function assertSameTree(source, target) {
  const sourceFiles = filesInTree(source).sort();
  const targetFiles = filesInTree(target).sort();
  if (sourceFiles.join("\n") !== targetFiles.join("\n")) {
    throw new Error(`synchronized tree file set differs: ${path.relative(ROOT, target)}`);
  }
  for (const relative of sourceFiles) {
    assertSameFile(path.join(source, relative), path.join(target, relative));
  }
}

function assertDirectoryEntries(target, expectedEntries) {
  const actual = fs.readdirSync(target).sort();
  const expected = [...expectedEntries].sort();
  if (actual.join("\n") !== expected.join("\n")) {
    throw new Error(`synchronized directory entries differ: ${path.relative(ROOT, target)}`);
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

function assertLanguagePacks(app) {
  const target = path.join(ROOT, "apps", app, "ui/language-packs");
  assertDirectoryEntries(target, ["catalog.json", "hub", "workbench"]);
  assertSameFile(path.join(languagePackSourceDir, "catalog.json"), path.join(target, "catalog.json"));
  for (const surface of ["hub", "workbench"]) {
    assertSameTree(path.join(languagePackSourceDir, surface), path.join(target, surface));
  }
}

function writeFile(target, contents) {
  ensureDir(path.dirname(target));
  fs.writeFileSync(target, contents);
}

function compileDesktopSharedTypeScript(outDir = sharedUiDir) {
  const tscBin = path.join(ROOT, "apps/frontend/node_modules/.bin/tsc");
  execFileSync(tscBin, ["-p", path.join(desktopSharedDir, "tsconfig.json"), "--outDir", outDir], {
    cwd: desktopSharedDir,
    stdio: "inherit",
  });
  fs.rmSync(path.join(outDir, "runtime-status-types.js"), { force: true });
}

function verifyDesktopSharedAssets() {
  const temporaryUiDir = fs.mkdtempSync(path.join(ROOT, "tmp/desktop-shared-check-"));
  try {
    compileDesktopSharedTypeScript(temporaryUiDir);
    for (const file of generatedUiFiles) {
      const generated = path.join(temporaryUiDir, file);
      const canonical = path.join(sharedUiDir, file);
      assertSameFile(generated, canonical);
    }
    for (const app of APPS) {
      const sharedTargetDir = path.join(ROOT, "apps", app, "ui/shared");
      assertDirectoryEntries(sharedTargetDir, sharedUiFiles);
      for (const file of sharedUiFiles) {
        const canonical = path.join(sharedUiDir, file);
        const target = path.join(sharedTargetDir, file);
        if (app === "installer-gui" && file === "desktop-shell.css") {
          if (fs.readFileSync(target, "utf8") !== `${fs.readFileSync(canonical, "utf8")}${installerShellOverride}`) {
            throw new Error(`synchronized installer stylesheet differs: ${path.relative(ROOT, target)}`);
          }
        } else {
          assertSameFile(canonical, target);
        }
      }
      assertSameFile(brandSource, path.join(ROOT, "apps", app, "ui/assets/brand.json"));
      assertLanguagePacks(app);
    }
  } finally {
    fs.rmSync(temporaryUiDir, { recursive: true, force: true });
  }
}

if (CHECK_ONLY) {
  verifyDesktopSharedAssets();
  process.stdout.write("desktop shared asset synchronization check passed\n");
  process.exit(0);
}

compileDesktopSharedTypeScript();

for (const app of APPS) {
  const sharedTargetDir = path.join(ROOT, "apps", app, "ui/shared");
  for (const file of sharedUiFiles) {
    copy(path.join(sharedUiDir, file), path.join(sharedTargetDir, file));
  }
  if (app === "installer-gui") {
    fs.appendFileSync(path.join(sharedTargetDir, "desktop-shell.css"), installerShellOverride);
  }
  copy(brandSource, path.join(ROOT, "apps", app, "ui/assets/brand.json"));
  syncLanguagePacks(app);
}

process.stdout.write(`synced desktop shared assets to ${APPS.join(", ")}\n`);
