#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { readJson, rootDir } from "./release-metadata.mjs";

const manifestRelativePath = "docs/material-score-contract.manifest.json";
const markdownRelativePath = "docs/material-score-contract.md";
const manifest = readJson(path.join(rootDir, manifestRelativePath));
const markdown = fs.readFileSync(path.join(rootDir, markdownRelativePath), "utf8");
const issues = [];

if (manifest.schema_version !== "kyuubiki.material-score-contract/v1") {
  issues.push(`${manifestRelativePath}: unexpected schema_version`);
}

if (manifest.operator_id !== "transform.score_material_candidates") {
  issues.push(`${manifestRelativePath}: unexpected operator_id`);
}

for (const key of [
  "runtime_paths",
  "test_paths",
  "required_result_fields",
  "ranking_fields",
  "policy_fields",
  "range_fields",
  "stable_error_codes",
]) {
  requireNonEmptyArray(manifest[key], `${manifestRelativePath}: missing ${key}`);
}

const runtimeSources = readReferencedSources(manifest.runtime_paths ?? []);
const testSources = readReferencedSources(manifest.test_paths ?? []);

for (const relativePath of [
  ...(manifest.runtime_paths ?? []),
  ...(manifest.test_paths ?? []),
]) {
  if (!fs.existsSync(path.join(rootDir, relativePath))) {
    issues.push(`${manifestRelativePath}: missing referenced path ${relativePath}`);
  }
}

for (const token of [
  manifest.operator_id,
  ...(manifest.required_result_fields ?? []),
  ...(manifest.ranking_fields ?? []),
  ...(manifest.policy_fields ?? []),
  ...(manifest.range_fields ?? []),
  ...(manifest.stable_error_codes ?? []),
]) {
  if (!markdown.includes(token)) {
    issues.push(`${markdownRelativePath}: missing contract token ${token}`);
  }
}

for (const token of [
  ...(manifest.required_result_fields ?? []),
  ...(manifest.policy_fields ?? []),
  ...(manifest.range_fields ?? []),
  ...(manifest.stable_error_codes ?? []),
]) {
  if (!runtimeSources.includes(token)) {
    issues.push(`runtime sources: missing contract token ${token}`);
  }
}

for (const token of [
  ...(manifest.required_result_fields ?? []),
  ...(manifest.policy_fields ?? []),
  ...(manifest.stable_error_codes ?? []),
]) {
  if (!testSources.includes(token)) {
    issues.push(`test sources: missing contract token ${token}`);
  }
}

if (issues.length > 0) {
  console.error("material score contract validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log(
  `material score contract ok: ${manifest.required_result_fields.length} result fields, ${manifest.stable_error_codes.length} errors`,
);

function requireNonEmptyArray(value, message) {
  if (!Array.isArray(value) || value.length === 0) {
    issues.push(message);
  }
}

function readReferencedSources(relativePaths) {
  return relativePaths
    .map((relativePath) => {
      const absolutePath = path.join(rootDir, relativePath);
      if (!fs.existsSync(absolutePath)) {
        return "";
      }
      return fs.readFileSync(absolutePath, "utf8");
    })
    .join("\n");
}
