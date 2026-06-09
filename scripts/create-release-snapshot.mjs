#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const releasesDir = path.join(rootDir, "releases");
const snapshotsDir = path.join(releasesDir, "snapshots");
const indexPath = path.join(releasesDir, "index.json");

function usage() {
  console.log(`Usage:
  node ./scripts/create-release-snapshot.mjs <version> [--status current|staged|archived] [--codename tamamono] [--line 1.x] [--dry-run] [--force]

Examples:
  node ./scripts/create-release-snapshot.mjs 1.4.1 --status staged --dry-run
  node ./scripts/create-release-snapshot.mjs 1.4.1 --status current
`);
}

function parseArgs(argv) {
  const options = {
    version: null,
    status: "staged",
    codename: "tamamono",
    line: "1.x",
    dryRun: false,
    force: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (!value.startsWith("--") && !options.version) {
      options.version = value;
      continue;
    }

    if (value === "--dry-run") {
      options.dryRun = true;
      continue;
    }

    if (value === "--force") {
      options.force = true;
      continue;
    }

    if (value === "--status") {
      options.status = argv[index + 1] ?? options.status;
      index += 1;
      continue;
    }

    if (value === "--codename") {
      options.codename = argv[index + 1] ?? options.codename;
      index += 1;
      continue;
    }

    if (value === "--line") {
      options.line = argv[index + 1] ?? options.line;
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${value}`);
  }

  return options;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync("git", args, { cwd: rootDir, encoding: "utf8" }).trim();
}

function gitStatusLines() {
  const output = git(["status", "--short"]);
  if (!output) {
    return [];
  }

  return output
    .split("\n")
    .map((line) => line.trimEnd())
    .filter(Boolean);
}

function isoDate() {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function packageVersion(relativePath) {
  return readJson(path.join(rootDir, relativePath)).version;
}

function readDesktopBundleVersion(appRelativePath) {
  const infoPlistPath = path.join(rootDir, appRelativePath, "Contents", "Info.plist");
  if (!fs.existsSync(infoPlistPath)) {
    return null;
  }

  try {
    const shortVersion = execFileSync(
      "plutil",
      ["-extract", "CFBundleShortVersionString", "raw", "-o", "-", infoPlistPath],
      { cwd: rootDir, encoding: "utf8" },
    ).trim();
    const buildVersion = execFileSync(
      "plutil",
      ["-extract", "CFBundleVersion", "raw", "-o", "-", infoPlistPath],
      { cwd: rootDir, encoding: "utf8" },
    ).trim();

    return {
      short_version: shortVersion || null,
      build_version: buildVersion || null,
      source: path.relative(rootDir, infoPlistPath),
    };
  } catch {
    return null;
  }
}

function buildSnapshot(version, options) {
  const desktopArtifacts = {
    hub_app: "apps/hub-gui/src-tauri/target/release/bundle/macos/Kyuubiki Hub.app",
    hub_dmg: `apps/hub-gui/src-tauri/target/release/bundle/dmg/Kyuubiki Hub_${version}_aarch64.dmg`,
    workbench_app: "apps/workbench-gui/src-tauri/target/release/bundle/macos/Kyuubiki Workbench.app",
    workbench_dmg: `apps/workbench-gui/src-tauri/target/release/bundle/dmg/Kyuubiki Workbench_${version}_aarch64.dmg`,
    installer_app: "apps/installer-gui/src-tauri/target/release/bundle/macos/Kyuubiki Installer.app",
    installer_dmg: `apps/installer-gui/src-tauri/target/release/bundle/dmg/Kyuubiki Installer_${version}_aarch64.dmg`,
  };

  const collectedDesktopBundleVersions = {
    hub: readDesktopBundleVersion(desktopArtifacts.hub_app),
    workbench: readDesktopBundleVersion(desktopArtifacts.workbench_app),
    installer: readDesktopBundleVersion(desktopArtifacts.installer_app),
  };
  const worktreeStatus = gitStatusLines();

  return {
    version,
    codename: options.codename,
    line: options.line,
    status: options.status,
    date: isoDate(),
    git_commit: git(["rev-parse", "HEAD"]),
    summary: `TODO: summarize ${options.codename} ${version}.`,
    docs: {
      current_line: "docs/current-line.md",
      version_line: "docs/version-line.md",
      packaging: "docs/packaging-and-deployment.md",
      desktop_release_checklist: "docs/desktop-release-checklist.md",
      operator_sdk: "docs/operator-sdk.md",
      workflow_graph: "docs/workflow-graph.md",
      workflow_dataset: "docs/workflow-dataset.md",
    },
    product_surfaces: {
      frontend_workbench: {
        version: packageVersion("apps/frontend/package.json"),
        notes: ["TODO: describe frontend/workbench changes."],
      },
      hub_gui: {
        version: packageVersion("apps/hub-gui/package.json"),
        notes: ["TODO: describe Hub desktop-shell changes."],
      },
      workbench_gui: {
        version: packageVersion("apps/workbench-gui/package.json"),
        notes: ["TODO: describe Workbench desktop-shell changes."],
      },
      installer_gui: {
        version: packageVersion("apps/installer-gui/package.json"),
        notes: ["TODO: describe Installer desktop-shell changes."],
      },
      web_api: {
        version: packageVersion("apps/frontend/package.json"),
        notes: ["TODO: describe API/control-plane changes."],
      },
    },
    workflow_builder: {
      status: "TODO",
      capabilities: [],
    },
    operator_sdk: {
      status: "TODO",
      capabilities: [],
    },
    desktop_artifacts: desktopArtifacts,
    verification: {
      git_worktree: {
        clean: worktreeStatus.length === 0,
        status_short: worktreeStatus,
      },
      desktop_bundle_versions: {
        hub: packageVersion("apps/hub-gui/package.json"),
        workbench: packageVersion("apps/workbench-gui/package.json"),
        installer: packageVersion("apps/installer-gui/package.json"),
      },
      collected_desktop_bundle_versions: collectedDesktopBundleVersions,
      frontend_checks: [],
      web_checks: [],
      rust_checks: [],
      repo_checks: ["git diff --check"],
    },
  };
}

function updateIndex(version, options) {
  const index = fs.existsSync(indexPath)
    ? readJson(indexPath)
    : { line: `${options.codename} ${options.line}`, current_version: version, snapshots: [] };
  const snapshotPath = `snapshots/${version}.json`;
  const entry = {
    version,
    status: options.status,
    date: isoDate(),
    codename: options.codename,
    line: options.line,
    snapshot_path: snapshotPath,
  };

  const nextSnapshots = (index.snapshots ?? []).filter((item) => item.version !== version);
  nextSnapshots.unshift(entry);

  return {
    line: `${options.codename} ${options.line}`,
    current_version: options.status === "current" ? version : index.current_version ?? version,
    snapshots: nextSnapshots,
  };
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (!options.version || options.version === "--help" || options.version === "-h") {
    usage();
    process.exit(options.version ? 0 : 1);
  }

  const snapshotPath = path.join(snapshotsDir, `${options.version}.json`);
  if (fs.existsSync(snapshotPath) && !options.force) {
    throw new Error(`Snapshot already exists at ${path.relative(rootDir, snapshotPath)}. Use --force to overwrite.`);
  }

  const snapshot = buildSnapshot(options.version, options);
  const nextIndex = updateIndex(options.version, options);

  if (options.dryRun) {
    console.log(JSON.stringify({
      snapshot_path: path.relative(rootDir, snapshotPath),
      index_path: path.relative(rootDir, indexPath),
      snapshot,
      index: nextIndex,
    }, null, 2));
    return;
  }

  fs.mkdirSync(snapshotsDir, { recursive: true });
  writeJson(snapshotPath, snapshot);
  writeJson(indexPath, nextIndex);

  console.log(`Created release snapshot scaffold for ${options.version}`);
  console.log(`- ${path.relative(rootDir, snapshotPath)}`);
  console.log(`- ${path.relative(rootDir, indexPath)}`);
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
