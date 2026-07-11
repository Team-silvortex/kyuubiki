#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { rootDir } from "./release-metadata.mjs";
import { markdownFactChecks } from "./version-line-markdown-facts.mjs";

function usage() {
  console.log(`Usage:
  node ./scripts/audit-version-line.mjs [--expected 1.6.0] [--next 1.7.0] [--codename tamamono] [--json]

Examples:
  node ./scripts/audit-version-line.mjs
  node ./scripts/audit-version-line.mjs --expected 1.6.0 --next 1.7.0
  node ./scripts/audit-version-line.mjs --expected 1.6.0 --json
`);
}

function parseArgs(argv) {
  const options = {
    expected: null,
    next: null,
    codename: "tamamono",
    json: false,
    selfTest: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];

    if (value === "--help" || value === "-h") {
      usage();
      process.exit(0);
    }

    if (value === "--json") {
      options.json = true;
      continue;
    }

    if (value === "--self-test") {
      options.selfTest = true;
      continue;
    }

    if (value === "--expected") {
      options.expected = argv[index + 1] ?? options.expected;
      index += 1;
      continue;
    }

    if (value === "--next") {
      options.next = argv[index + 1] ?? options.next;
      index += 1;
      continue;
    }

    if (value === "--codename") {
      options.codename = argv[index + 1] ?? options.codename;
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${value}`);
  }

  return options;
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(rootDir, relativePath), "utf8"));
}

function readText(relativePath) {
  return fs.readFileSync(path.join(rootDir, relativePath), "utf8");
}

function currentReleaseVersion() {
  return readJson("releases/index.json").current_version;
}

function versionMinorLine(version) {
  const parts = String(version).split(".");
  if (parts.length < 2) {
    return `${version}.x`;
  }

  return `${parts[0]}.${parts[1]}.x`;
}

function versionDisplay(codename, version) {
  return `${codename} ${version}`;
}

function walk(relativePath, results = []) {
  const absolutePath = path.join(rootDir, relativePath);
  const entries = fs.readdirSync(absolutePath, { withFileTypes: true });

  for (const entry of entries) {
    if (
      entry.name === ".git" ||
      entry.name === "node_modules" ||
      entry.name === "target" ||
      entry.name === ".next" ||
      entry.name === "dist" ||
      entry.name === "build"
    ) {
      continue;
    }

    const nextRelativePath = path.join(relativePath, entry.name);
    if (entry.isDirectory()) {
      walk(nextRelativePath, results);
      continue;
    }

    results.push(nextRelativePath);
  }

  return results;
}

function exactChecks(expectedVersion) {
  const expectedMinorLine = versionMinorLine(expectedVersion);
  const expectedDisplayVersion = versionDisplay("tamamono", expectedVersion);
  const expectedDisplayMinorLine = versionDisplay("tamamono", expectedMinorLine);
  const files = [
    "apps/frontend/package.json",
    "apps/hub-gui/package.json",
    "apps/hub-gui/src-tauri/Cargo.toml",
    "apps/hub-gui/src-tauri/tauri.conf.json",
    "apps/hub-gui/ui/assets/brand.json",
    "apps/workbench-gui/package.json",
    "apps/workbench-gui/src-tauri/Cargo.toml",
    "apps/workbench-gui/src-tauri/tauri.conf.json",
    "apps/workbench-gui/ui/assets/brand.json",
    "apps/installer-gui/package.json",
    "apps/installer-gui/src-tauri/Cargo.toml",
    "apps/installer-gui/src-tauri/tauri.conf.json",
    "apps/installer-gui/ui/assets/brand.json",
    "workers/rust/Cargo.toml",
    "docs/ui-automation-contract.json",
  ];
  const checks = [];

  for (const relativePath of files) {
    if (relativePath.endsWith("package.json")) {
      const json = readJson(relativePath);
      checks.push({
        kind: "version",
        file: relativePath,
        field: "version",
        expected: expectedVersion,
        actual: json.version ?? null,
      });
      continue;
    }

    if (relativePath.endsWith("tauri.conf.json")) {
      const json = readJson(relativePath);
      checks.push({
        kind: "version",
        file: relativePath,
        field: "version",
        expected: expectedVersion,
        actual: json.version ?? null,
      });
      continue;
    }

    if (relativePath.endsWith("brand.json")) {
      const json = readJson(relativePath);
      checks.push({
        kind: "releaseVersion",
        file: relativePath,
        field: "releaseVersion",
        expected: expectedVersion,
        actual: json.releaseVersion ?? null,
      });
      continue;
    }

    if (relativePath.endsWith("Cargo.toml")) {
      const match = readText(relativePath).match(/^version\s*=\s*"([^"]+)"/mu);
      checks.push({
        kind: "version",
        file: relativePath,
        field: "version",
        expected: expectedVersion,
        actual: match?.[1] ?? null,
      });
      continue;
    }

    if (relativePath.endsWith("ui-automation-contract.json")) {
      const json = readJson(relativePath);
      checks.push({
        kind: "minor_line",
        file: relativePath,
        field: "version",
        expected: expectedMinorLine,
        actual: json.version ?? null,
      });
    }
  }

  const channels = readJson("deploy/update-channels.json");
  const contract = readJson("deploy/installation-integrity-contract.json");
  const releaseIndex = readJson("releases/index.json");

  checks.push({
    kind: "shipping_version",
    file: "deploy/update-channels.json",
    field: "shipping_version",
    expected: expectedVersion,
    actual: channels.shipping_version ?? null,
  });
  checks.push({
    kind: "stable_channel_version",
    file: "deploy/update-channels.json",
    field: "channels[stable].version",
    expected: expectedVersion,
    actual: (channels.channels ?? []).find((channel) => channel.id === (channels.default_channel ?? "stable"))?.version ?? null,
  });
  checks.push({
    kind: "shipping_version",
    file: "deploy/installation-integrity-contract.json",
    field: "shipping_version",
    expected: expectedVersion,
    actual: contract.shipping_version ?? null,
  });
  const requiredVersionRule = (contract.visible_rules ?? []).find((rule) =>
    rule.label === "required development version" || rule.label === "required shipping version"
  );
  checks.push({
    kind: "required_version",
    file: "deploy/installation-integrity-contract.json",
    field: "visible_rules[required development version].value",
    expected: expectedVersion,
    actual: requiredVersionRule?.value ?? null,
  });
  checks.push({
    kind: "current_version",
    file: "releases/index.json",
    field: "current_version",
    expected: expectedVersion,
    actual: releaseIndex.current_version ?? null,
  });
  checks.push({
    kind: "shipping_version",
    file: "releases/update-catalog.json",
    field: "shipping_version",
    expected: expectedVersion,
    actual: readJson("releases/update-catalog.json").shipping_version ?? null,
  });

  const languagePackCatalog = readJson("language-packs/catalog.json");
  checks.push({
    kind: "language_pack_catalog_shipping_version",
    file: "language-packs/catalog.json",
    field: "shipping_version",
    expected: expectedVersion,
    actual: languagePackCatalog.shipping_version ?? null,
  });

  for (const entry of languagePackCatalog.packs ?? []) {
    const packPath = `language-packs/${entry.path}`;
    const pack = readJson(packPath);
    checks.push({
      kind: "language_pack_version",
      file: packPath,
      field: "version",
      expected: expectedVersion,
      actual: pack.version ?? null,
    });
    checks.push({
      kind: "language_pack_target_app_version",
      file: packPath,
      field: "targetAppVersion",
      expected: expectedVersion,
      actual: pack.targetAppVersion ?? null,
    });
  }

  checks.push(...markdownFactChecks(expectedVersion, "tamamono", readText));

  const currentSnapshots = (releaseIndex.snapshots ?? [])
    .filter((snapshot) => snapshot.status === "current")
    .map((snapshot) => snapshot.version);
  checks.push({
    kind: "release_current_snapshot",
    file: "releases/index.json",
    field: "snapshots[status=current]",
    expected: expectedVersion,
    actual: currentSnapshots.length === 1 ? currentSnapshots[0] : currentSnapshots.join(","),
  });

  const catalog = readJson("releases/update-catalog.json");
  const currentCatalogVersions = (catalog.versions ?? [])
    .filter((version) => version.status === "current")
    .map((version) => version.version);
  checks.push({
    kind: "catalog_current_version",
    file: "releases/update-catalog.json",
    field: "versions[status=current]",
    expected: expectedVersion,
    actual: currentCatalogVersions.length === 1 ? currentCatalogVersions[0] : currentCatalogVersions.join(","),
  });

  return checks.map((check) => ({
    ...check,
    ok: check.actual === check.expected,
  }));
}

function searchInventory(expectedVersion, codename) {
  const minorLine = versionMinorLine(expectedVersion);
  const displayVersion = versionDisplay(codename, expectedVersion);
  const displayMinorLine = versionDisplay(codename, minorLine);
  const scanRoots = ["README.md", "docs", "apps", "deploy", "releases", "workers", "sdks", "scripts"];
  const files = [];

  for (const scanRoot of scanRoots) {
    const absolutePath = path.join(rootDir, scanRoot);
    if (!fs.existsSync(absolutePath)) {
      continue;
    }

    if (fs.statSync(absolutePath).isDirectory()) {
      files.push(...walk(scanRoot));
    } else {
      files.push(scanRoot);
    }
  }

  const allowedExtensions = new Set([
    ".md",
    ".html",
    ".json",
    ".js",
    ".mjs",
    ".ts",
    ".tsx",
    ".zsh",
    ".cmd",
    ".toml",
    ".ex",
    ".exs",
  ]);

  const patterns = [
    { label: "exact_version", value: expectedVersion },
    { label: "minor_line", value: minorLine },
    { label: "display_version", value: displayVersion },
    { label: "display_minor_line", value: displayMinorLine },
  ];

  const inventory = [];

  for (const relativePath of files.sort()) {
    if (relativePath.endsWith("package-lock.json")) {
      continue;
    }

    const extension = path.extname(relativePath);
    if (!allowedExtensions.has(extension) && !/README$/u.test(relativePath)) {
      continue;
    }

    const contents = readText(relativePath);
    const hits = patterns
      .map((pattern) => ({
        label: pattern.label,
        value: pattern.value,
        count: contents.split(pattern.value).length - 1,
      }))
      .filter((pattern) => pattern.count > 0);

    if (hits.length === 0) {
      continue;
    }

    inventory.push({
      file: relativePath,
      hits,
    });
  }

  return inventory;
}

function nextVersionCandidates(expectedVersion, nextVersion, codename) {
  if (!nextVersion) {
    return [];
  }

  const currentDisplay = versionDisplay(codename, expectedVersion);
  const currentMinorLine = versionMinorLine(expectedVersion);
  const nextMinorLine = versionMinorLine(nextVersion);

  return searchInventory(expectedVersion, codename)
    .filter((entry) => !entry.file.startsWith(`releases/snapshots/${expectedVersion}.json`))
    .map((entry) => ({
      ...entry,
      suggested_replacements: [
        { from: expectedVersion, to: nextVersion },
        { from: currentDisplay, to: versionDisplay(codename, nextVersion) },
        { from: currentMinorLine, to: nextMinorLine },
      ],
    }));
}

function printHumanReport(report) {
  console.log(`Version line audit for ${report.codename} ${report.expected}`);
  console.log("");

  const failedChecks = report.exact_checks.filter((check) => !check.ok);
  console.log(`Exact contract checks: ${report.exact_checks.length} total, ${failedChecks.length} mismatched`);
  for (const check of failedChecks) {
    console.log(`- mismatch: ${check.file} :: ${check.field} expected ${check.expected} but found ${check.actual}`);
  }

  if (failedChecks.length === 0) {
    console.log("- all exact version contracts match the expected development version");
  }

  console.log("");
  console.log(`Textual version references: ${report.reference_inventory.length} files`);
  for (const entry of report.reference_inventory.slice(0, 20)) {
    const summary = entry.hits.map((hit) => `${hit.label}=${hit.count}`).join(", ");
    console.log(`- ${entry.file} :: ${summary}`);
  }

  if (report.reference_inventory.length > 20) {
    console.log(`- ... ${report.reference_inventory.length - 20} more files`);
  }

  if (report.next_version) {
    console.log("");
    console.log(`1.7 prep candidates for ${report.next_version}: ${report.next_candidates.length} files`);
    for (const entry of report.next_candidates.slice(0, 20)) {
      console.log(`- ${entry.file}`);
    }

    if (report.next_candidates.length > 20) {
      console.log(`- ... ${report.next_candidates.length - 20} more files`);
    }
  }
}

function runSelfTest() {
  const checks = markdownFactChecks("1.18.0", "tamamono", (file) => {
    if (file === "docs/version-line.md") {
      return "current development point: `tamamono 1.15.0`\ncurrent documentation target: `tamamono 1.15.x` pre-`moxi` line";
    }
    if (file === "docs/current-line.md") {
      return "The current development point in this line is `tamamono 1.15.0`.";
    }
    if (file === "docs/installer-remote-control.md") {
      return "remote runtime control surface in the `tamamono 1.15.x` preparation line.";
    }
    if (file === "docs/desktop-release-checklist.md") {
      return "Examples for the current `1.15.0` workspace-prep line:";
    }
    return "";
  });
  const failed = checks.filter((check) => check.actual !== check.expected);
  if (failed.length !== checks.length) {
    console.error("Version line audit self-test failed: stale Markdown facts were not rejected");
    process.exit(1);
  }
  console.log("version line audit self-test passed");
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runSelfTest();
    return;
  }
  const expectedVersion = options.expected ?? currentReleaseVersion();
  const report = {
    codename: options.codename,
    expected: expectedVersion,
    next_version: options.next,
    exact_checks: exactChecks(expectedVersion),
    reference_inventory: searchInventory(expectedVersion, options.codename),
    next_candidates: nextVersionCandidates(expectedVersion, options.next, options.codename),
  };

  if (options.json) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }

  printHumanReport(report);
}

main();
