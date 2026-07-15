#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { rootDir } from "./release-metadata.mjs";

const docsRoot = path.join(rootDir, "docs");
const hubDocsRoot = path.join(rootDir, "apps/hub-gui/ui/docs");
const issues = [];

checkDirectoryInventory({
  root: docsRoot,
  indexFile: path.join(docsRoot, "README.md"),
  label: "docs/README.md",
  ignore: new Set(["README.md"]),
});

checkDirectoryInventory({
  root: hubDocsRoot,
  indexFile: path.join(hubDocsRoot, "README.md"),
  label: "apps/hub-gui/ui/docs/README.md",
  ignore: new Set(["README.md"]),
});

if (issues.length > 0) {
  console.error("documentation inventory check failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("documentation inventory ok");

function checkDirectoryInventory({ root, indexFile, label, ignore }) {
  const index = fs.readFileSync(indexFile, "utf8");
  const files = fs
    .readdirSync(root)
    .filter((file) => /\.(md|html|json)$/.test(file))
    .filter((file) => !ignore.has(file))
    .sort();

  for (const file of files) {
    if (!index.includes(file)) {
      issues.push(`${label}: missing inventory entry for ${path.basename(root)}/${file}`);
    }
  }
}
