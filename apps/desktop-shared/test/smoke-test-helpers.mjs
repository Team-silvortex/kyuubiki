import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export function createFixtureRoot(importMetaUrl) {
  return path.resolve(path.dirname(fileURLToPath(importMetaUrl)), "..");
}

export function createFixtureReader(root) {
  return function read(relativePath) {
    return fs.readFileSync(path.join(root, relativePath), "utf8");
  };
}

export function assertMatches(content, patterns) {
  for (const pattern of patterns) {
    assert.match(content, pattern);
  }
}
