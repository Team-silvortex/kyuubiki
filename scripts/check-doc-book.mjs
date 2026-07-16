#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { readJson, rootDir, updateChannelsPath } from "./release-metadata.mjs";

const shippingVersion = readJson(updateChannelsPath).shipping_version;
const manifestPath = path.join(rootDir, "docs/book-manifest.json");
const manifest = readJson(manifestPath);

const htmlFiles = [
  "docs/book.html",
  "docs/book-ch01-what-is-kyuubiki.html",
  "docs/book-ch02-moxi-line.html",
  "docs/book-ch03-architecture-boundaries.html",
  "docs/book-ch04-runtime-modes.html",
  "docs/book-ch05-workflow-and-operators.html",
  "docs/book-ch06-sdk-surfaces.html",
  "docs/book-ch07-trust-and-safety.html",
  "docs/book-ch08-reading-paths.html",
  "docs/update-catalog.html",
  "docs/installation-integrity-contract.html",
  "apps/hub-gui/ui/docs/index.html",
  "apps/hub-gui/ui/docs/current-line.html",
  "apps/hub-gui/ui/docs/operations.html",
  "apps/hub-gui/ui/docs/installation-integrity.html",
  "apps/hub-gui/ui/docs/troubleshooting.html",
  "apps/hub-gui/ui/docs/update-catalog.html",
];

const versionFiles = [
  "docs/book.html",
  "docs/book-manifest.json",
  "apps/hub-gui/ui/docs/index.html",
  "apps/hub-gui/ui/docs/current-line.html",
  "apps/hub-gui/ui/docs/installation-integrity.html",
  "apps/hub-gui/ui/docs/update-catalog.html",
];

const requiredSnippets = new Map([
  ["docs/book.html", ["Chapter 1: What Kyuubiki is", "Open chapter page"]],
  ["docs/book-ch08-reading-paths.html", ["book-manifest.json", "docs/README.md"]],
  ["apps/hub-gui/ui/docs/index.html", ["Open central book", "Chapter entry", "Quick entry by role"]],
  ["apps/hub-gui/ui/docs/current-line.html", ["Mirror · Chapter 2", "Open central chapter"]],
  ["apps/hub-gui/ui/docs/operations.html", ["Mirror · Chapter 4", "Open central chapter"]],
  [
    "apps/hub-gui/ui/docs/installation-integrity.html",
    ["Mirror · Chapter 7", "Open central chapter", "Open source page"],
  ],
  ["apps/hub-gui/ui/docs/troubleshooting.html", ["Troubleshooting Mirror", "Open reading paths"]],
  ["apps/hub-gui/ui/docs/update-catalog.html", ["Mirror · Chapter 7", "Open central chapter", "Open source page"]],
]);

const forbiddenSnippets = [
  "Kyuubiki Hub Docs",
  "docs home",
  "Back to docs home",
];

const issues = [];

for (const relativePath of htmlFiles) {
  const absolutePath = path.join(rootDir, relativePath);
  if (!fs.existsSync(absolutePath)) {
    issues.push(`missing file: ${relativePath}`);
    continue;
  }

  const text = fs.readFileSync(absolutePath, "utf8");
  for (const forbidden of forbiddenSnippets) {
    if (text.includes(forbidden)) {
      issues.push(`${relativePath}: found forbidden legacy text "${forbidden}"`);
    }
  }

  for (const snippet of requiredSnippets.get(relativePath) ?? []) {
    if (!text.includes(snippet)) {
      issues.push(`${relativePath}: missing required text "${snippet}"`);
    }
  }

  for (const href of extractLocalHrefs(text)) {
    const target = path.resolve(path.dirname(absolutePath), href);
    if (!fs.existsSync(target)) {
      issues.push(`${relativePath}: broken href ${href}`);
    }
  }
}

for (const relativePath of versionFiles) {
  const text = fs.readFileSync(path.join(rootDir, relativePath), "utf8");
  if (!text.includes(shippingVersion)) {
    issues.push(`${relativePath}: missing shipping version ${shippingVersion}`);
  }
}

if ((manifest.chapters ?? []).length !== 8) {
  issues.push(`docs/book-manifest.json: expected 8 chapters, found ${(manifest.chapters ?? []).length}`);
}

for (const chapter of manifest.chapters ?? []) {
  if (!chapter.chapter_page) {
    issues.push(`docs/book-manifest.json: chapter ${chapter.id} missing chapter_page`);
    continue;
  }

  const chapterPath = path.join(rootDir, chapter.chapter_page);
  if (!fs.existsSync(chapterPath)) {
    issues.push(`docs/book-manifest.json: missing chapter page ${chapter.chapter_page}`);
  }
}

const rolePaths = [
  "operator",
  "frontend_engineer",
  "runtime_engineer",
  "sdk_engineer",
  "llm_integrator",
];

for (const role of rolePaths) {
  const entries = manifest.reading_paths?.[role];
  if (!Array.isArray(entries) || entries.length === 0) {
    issues.push(`docs/book-manifest.json: reading path "${role}" is missing or empty`);
  }
}

if (issues.length > 0) {
  console.error("Kyuubiki docs-book check failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exitCode = 1;
} else {
  console.log(`docs-book check passed for version ${shippingVersion}`);
  console.log(`checked ${htmlFiles.length} HTML files and docs/book-manifest.json`);
}

function extractLocalHrefs(text) {
  const matches = text.matchAll(/href="([^"]+)"/g);
  const hrefs = [];
  for (const match of matches) {
    const href = match[1];
    if (!href || href.startsWith("#") || href.startsWith("http://") || href.startsWith("https://")) {
      continue;
    }
    hrefs.push(href);
  }
  return hrefs;
}
