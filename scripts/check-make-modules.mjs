#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const rootDir = path.resolve(new URL("..", import.meta.url).pathname);
const makefilePath = path.join(rootDir, "Makefile");
const makeDir = path.join(rootDir, "make");
const makefile = fs.readFileSync(makefilePath, "utf8");
const issues = [];

const includeLines = makefile
  .split("\n")
  .map((line, index) => ({ index: index + 1, line: line.trim() }))
  .filter(({ line }) => line.startsWith("include "));

const includes = includeLines.map(({ line }) => line.replace(/^include\s+/, "").trim());

if (includes[0] !== "make/help.mk") {
  issues.push("Makefile: make/help.mk must be the first include so plain `make` shows help");
}

for (const includePath of includes) {
  if (!includePath.startsWith("make/") || !includePath.endsWith(".mk")) {
    issues.push(`Makefile: include ${includePath} must point at make/*.mk`);
    continue;
  }
  if (!fs.existsSync(path.join(rootDir, includePath))) {
    issues.push(`Makefile: included module does not exist: ${includePath}`);
  }
  if (includePath === "make/targets.mk") {
    issues.push("Makefile: make/targets.mk is retired; use narrow modules instead");
  }
}

for (const { index, line } of makefile.split("\n").map((line, index) => ({ index: index + 1, line }))) {
  const trimmed = line.trim();
  if (trimmed === "" || trimmed.startsWith("#")) continue;
  if (/^[A-Za-z0-9_.-]+\s*[:?+]?=/.test(trimmed)) continue;
  if (trimmed.startsWith("include ")) continue;
  issues.push(`Makefile:${index}: root Makefile should only define shared variables and includes`);
}

const modules = fs
  .readdirSync(makeDir)
  .filter((entry) => entry.endsWith(".mk"))
  .sort();

for (const module of modules) {
  const includePath = `make/${module}`;
  if (!includes.includes(includePath)) {
    issues.push(`${includePath}: module is not included by root Makefile`);
  }
}

if (issues.length > 0) {
  console.error("make module check failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log(`make module check passed: ${includes.length} included module(s)`);
