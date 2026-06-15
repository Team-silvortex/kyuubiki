#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { readJson, rootDir, updateChannelsPath } from "./release-metadata.mjs";

function usage() {
  console.log(`Usage:
  node ./scripts/sync-doc-book-version.mjs
  node ./scripts/sync-doc-book-version.mjs --version 1.7.0 --line "tamamono 1.7.0"

Defaults:
  --version uses deploy/update-channels.json shipping_version
  --line uses "tamamono <version>"
`);
}

function parseArgs(argv) {
  const options = {
    version: null,
    line: null,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];

    if (value === "--help" || value === "-h") {
      usage();
      process.exit(0);
    }

    if (value === "--version") {
      options.version = argv[index + 1] ?? options.version;
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

const args = parseArgs(process.argv.slice(2));
const shippingVersion = args.version ?? readJson(updateChannelsPath).shipping_version;
const versionLine = args.line ?? `tamamono ${shippingVersion}`;

const replacements = [
  {
    file: "docs/book.html",
    rules: [
      [/One book for tamamono [0-9]+\.[0-9]+\.[0-9]+/g, `One book for ${versionLine}`],
      [/Version line: tamamono [0-9]+\.[0-9]+\.[0-9]+/g, `Version line: ${versionLine}`],
    ],
  },
  {
    file: "docs/book-manifest.json",
    rules: [[/"version_line": "tamamono [0-9]+\.[0-9]+\.[0-9]+"/g, `"version_line": "${versionLine}"`]],
  },
  {
    file: "apps/hub-gui/ui/docs/index.html",
    rules: [
      [/Desktop reading entry for tamamono [0-9]+\.[0-9]+\.[0-9]+/g, `Desktop reading entry for ${versionLine}`],
      [/Current line: tamamono [0-9]+\.[0-9]+\.[0-9]+/g, `Current line: ${versionLine}`],
    ],
  },
  {
    file: "apps/hub-gui/ui/docs/current-line.html",
    rules: [[/>tamamono [0-9]+\.[0-9]+\.[0-9]+</g, `>${versionLine}<`]],
  },
];

const updatedFiles = [];

for (const entry of replacements) {
  const absolutePath = path.join(rootDir, entry.file);
  const original = fs.readFileSync(absolutePath, "utf8");
  let next = original;

  for (const [pattern, replacement] of entry.rules) {
    next = next.replace(pattern, replacement);
  }

  if (next !== original) {
    fs.writeFileSync(absolutePath, next);
    updatedFiles.push(entry.file);
  }
}

console.log(`synced docs-book version to ${versionLine}`);
if (updatedFiles.length === 0) {
  console.log("no files changed");
} else {
  for (const file of updatedFiles) {
    console.log(`updated ${file}`);
  }
}
