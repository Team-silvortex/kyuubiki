#!/usr/bin/env node

import fs from "node:fs";
import assert from "node:assert/strict";
import path from "node:path";
import {
  desktopArtifactPaths,
  git,
  gitStatusLines,
  installationIntegrityContractPath,
  isoDate,
  packageVersion,
  readDesktopBundleVersion,
  readJson,
  releaseIndexPath,
  releaseLineLabel,
  rootDir,
  snapshotFilePath,
  snapshotRelativePath,
  snapshotsDir,
  syncCurrentReleaseContracts,
  updateChannelsPath,
  writeJson,
} from "./release-metadata.mjs";

const CURRENT_SOURCE_VERSION_FILES = [
  { kind: "package", path: "apps/frontend/package.json", field: "version" },
  { kind: "json", path: "apps/frontend/public/brand.json", field: "releaseVersion" },
  { kind: "package", path: "apps/hub-gui/package.json", field: "version" },
  { kind: "cargo", path: "apps/hub-gui/src-tauri/Cargo.toml", field: "version" },
  { kind: "json", path: "apps/hub-gui/src-tauri/tauri.conf.json", field: "version" },
  { kind: "json", path: "apps/hub-gui/ui/assets/brand.json", field: "releaseVersion" },
  { kind: "package", path: "apps/workbench-gui/package.json", field: "version" },
  { kind: "cargo", path: "apps/workbench-gui/src-tauri/Cargo.toml", field: "version" },
  { kind: "json", path: "apps/workbench-gui/src-tauri/tauri.conf.json", field: "version" },
  { kind: "json", path: "apps/workbench-gui/ui/assets/brand.json", field: "releaseVersion" },
  { kind: "package", path: "apps/installer-gui/package.json", field: "version" },
  { kind: "cargo", path: "apps/installer-gui/src-tauri/Cargo.toml", field: "version" },
  { kind: "json", path: "apps/installer-gui/src-tauri/tauri.conf.json", field: "version" },
  { kind: "json", path: "apps/installer-gui/ui/assets/brand.json", field: "releaseVersion" },
  { kind: "cargo", path: "workers/rust/Cargo.toml", field: "version" },
];

const RELEASE_FRONTEND_CHECKS = [
  "npm run typecheck",
  "npm run build",
  "npm run check:workflow-preflight",
];

const RELEASE_REPO_CHECKS = [
  "git diff --check",
  "make audit-project-organization",
  "make operator-package-preflight",
  "make check-operator-package-dynamic-smoke-contract",
  "make operator-package-dynamic-smoke",
  "make architecture-check",
];

function usage() {
  console.log(`Usage:
  node ./scripts/create-release-snapshot.mjs <version> [--status current|staged|archived] [--codename moxi] [--line 2.x] [--dry-run] [--force]

Examples:
  node ./scripts/create-release-snapshot.mjs 1.6.1 --status staged --dry-run
  node ./scripts/create-release-snapshot.mjs 1.6.1 --status current
`);
}

function parseArgs(argv) {
  const options = {
    version: null,
    status: "staged",
    codename: "moxi",
    line: "2.x",
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

function runSelfTest() {
  assert(RELEASE_FRONTEND_CHECKS.includes("npm run check:workflow-preflight"));
  assert(RELEASE_REPO_CHECKS.includes("make audit-project-organization"));
  assert(RELEASE_REPO_CHECKS.includes("make operator-package-preflight"));
  assert(RELEASE_REPO_CHECKS.includes("make check-operator-package-dynamic-smoke-contract"));
  assert(RELEASE_REPO_CHECKS.includes("make operator-package-dynamic-smoke"));
  assert(RELEASE_REPO_CHECKS.includes("make architecture-check"));
  assert.equal(new Set(RELEASE_REPO_CHECKS).size, RELEASE_REPO_CHECKS.length);
  assert.deepEqual(
    collectSourceVersionIssues("1.20.0", [
      { path: "ok.json", field: "version", actual: "1.20.0" },
      { path: "brand.json", field: "releaseVersion", actual: "1.20.0" },
      { path: "stale.json", field: "version", actual: "1.17.8" },
    ]),
    [{ path: "stale.json", field: "version", actual: "1.17.8", expected: "1.20.0" }],
  );
  console.log("release snapshot self-test passed");
}

function readRepoJson(relativePath) {
  return readJson(path.join(rootDir, relativePath));
}

function readRepoText(relativePath) {
  return fs.readFileSync(path.join(rootDir, relativePath), "utf8");
}

function readCargoVersion(relativePath) {
  return readRepoText(relativePath).match(/^version\s*=\s*"([^"]+)"/mu)?.[1] ?? null;
}

function sourceVersionRows() {
  return CURRENT_SOURCE_VERSION_FILES.map((entry) => {
    const actual =
      entry.kind === "cargo"
        ? readCargoVersion(entry.path)
        : readRepoJson(entry.path)[entry.field] ?? null;

    return {
      path: entry.path,
      field: entry.field,
      actual,
    };
  });
}

function collectSourceVersionIssues(version, rows = sourceVersionRows()) {
  return rows
    .filter((row) => row.actual !== version)
    .map((row) => ({ ...row, expected: version }));
}

function assertCurrentSourceVersions(version, status) {
  if (status !== "current") {
    return;
  }

  const issues = collectSourceVersionIssues(version);
  if (issues.length === 0) {
    return;
  }

  const details = issues
    .map((issue) => `- ${issue.path} :: ${issue.field} expected ${issue.expected} but found ${issue.actual}`)
    .join("\n");
  throw new Error(
    [
      `Cannot create a current release snapshot for ${version} before source versions are aligned.`,
      "Update package, Tauri, brand, and Rust workspace versions first, then rerun the snapshot command.",
      details,
    ].join("\n"),
  );
}

function buildSnapshot(version, options) {
  const desktopArtifacts = desktopArtifactPaths(version);
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
      frontend_checks: RELEASE_FRONTEND_CHECKS,
      web_checks: [],
      rust_checks: [],
      repo_checks: RELEASE_REPO_CHECKS,
    },
  };
}

function updateIndex(version, options) {
  const index = fs.existsSync(releaseIndexPath)
    ? readJson(releaseIndexPath)
    : { line: releaseLineLabel(options.codename, options.line), current_version: version, snapshots: [] };
  const entry = {
    version,
    status: options.status,
    date: isoDate(),
    codename: options.codename,
    line: options.line,
    snapshot_path: snapshotRelativePath(version),
  };

  const nextSnapshots = (index.snapshots ?? [])
    .filter((item) => item.version !== version)
    .map((item) =>
      options.status === "current" && item.status === "current"
        ? { ...item, status: "archived" }
        : item,
    );
  nextSnapshots.unshift(entry);

  return {
    line: releaseLineLabel(options.codename, options.line),
    current_version: options.status === "current" ? version : index.current_version ?? version,
    snapshots: nextSnapshots,
  };
}

function archivePreviousCurrentSnapshots(nextIndex, options) {
  if (options.status !== "current") {
    return;
  }

  for (const entry of nextIndex.snapshots ?? []) {
    if (entry.version === options.version || entry.status !== "archived") {
      continue;
    }

    const filePath = path.join(rootDir, "releases", entry.snapshot_path);
    if (!fs.existsSync(filePath)) {
      continue;
    }

    const snapshot = readJson(filePath);
    if (snapshot.status !== "current") {
      continue;
    }

    writeJson(filePath, { ...snapshot, status: "archived" });
  }
}

function main() {
  if (process.argv.includes("--self-test")) {
    runSelfTest();
    return;
  }

  const options = parseArgs(process.argv.slice(2));
  if (!options.version || options.version === "--help" || options.version === "-h") {
    usage();
    process.exit(options.version ? 0 : 1);
  }

  const snapshotPath = snapshotFilePath(options.version);
  if (fs.existsSync(snapshotPath) && !options.force) {
    throw new Error(`Snapshot already exists at ${path.relative(rootDir, snapshotPath)}. Use --force to overwrite.`);
  }

  assertCurrentSourceVersions(options.version, options.status);

  const snapshot = buildSnapshot(options.version, options);
  const nextIndex = updateIndex(options.version, options);

  if (options.dryRun) {
    console.log(JSON.stringify({
      snapshot_path: path.relative(rootDir, snapshotPath),
      index_path: path.relative(rootDir, releaseIndexPath),
      snapshot,
      index: nextIndex,
    }, null, 2));
    return;
  }

  fs.mkdirSync(snapshotsDir, { recursive: true });
  writeJson(snapshotPath, snapshot);
  writeJson(releaseIndexPath, nextIndex);
  archivePreviousCurrentSnapshots(nextIndex, options);

  if (options.status === "current") {
    syncCurrentReleaseContracts({
      version: options.version,
      codename: options.codename,
      line: options.line,
    });
  }

  console.log(`Created release snapshot scaffold for ${options.version}`);
  console.log(`- ${path.relative(rootDir, snapshotPath)}`);
  console.log(`- ${path.relative(rootDir, releaseIndexPath)}`);
  if (options.status === "current") {
    console.log(`- ${path.relative(rootDir, updateChannelsPath)}`);
    console.log(`- ${path.relative(rootDir, installationIntegrityContractPath)}`);
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
