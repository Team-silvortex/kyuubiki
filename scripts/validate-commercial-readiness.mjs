#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { readJson, rootDir } from "./release-metadata.mjs";

const docsDir = path.join(rootDir, "docs");
const manifestRelativePath = "docs/commercial-readiness-2.0.manifest.json";
const markdownRelativePath = "docs/commercial-readiness-2.0.md";
const manifestPath = path.join(rootDir, manifestRelativePath);
const markdownPath = path.join(rootDir, markdownRelativePath);
const manifest = readJson(manifestPath);
const markdown = fs.readFileSync(markdownPath, "utf8");
const issues = [];

const expectedClassifications = [
  "required",
  "acceptable limitation",
  "defer to 2.x",
  "blocker",
];

if (manifest.schema_version !== "kyuubiki.commercial-readiness/v1") {
  issues.push(`${manifestRelativePath}: unexpected schema_version`);
}

if (manifest.release_target !== "2.0") {
  issues.push(`${manifestRelativePath}: release_target must be 2.0`);
}

if (!sameStrings(manifest.classification_values, expectedClassifications)) {
  issues.push(`${manifestRelativePath}: classification_values drifted`);
}

if (!Array.isArray(manifest.gates) || manifest.gates.length !== 8) {
  issues.push(`${manifestRelativePath}: expected 8 commercial readiness gates`);
}

if (!markdown.includes("commercial-readiness-2.0.manifest.json")) {
  issues.push(`${markdownRelativePath}: missing paired manifest reference`);
}

if (!normalizeText(markdown).includes(normalizeText(manifest.exit_statement))) {
  issues.push(`${markdownRelativePath}: missing manifest exit statement`);
}

const gateIds = new Set();
let evidenceCount = 0;

for (const [index, gate] of (manifest.gates ?? []).entries()) {
  const label = `${manifestRelativePath}: gate ${gate?.id ?? index + 1}`;
  if (!gate?.id || gateIds.has(gate.id)) {
    issues.push(`${label}: missing or duplicate id`);
  }
  gateIds.add(gate?.id);

  requireText(gate.title, `${label}: missing title`);
  requireNonEmptyArray(gate.required, `${label}: missing required items`);
  requireNonEmptyArray(
    gate.acceptable_limitations,
    `${label}: missing acceptable limitations`,
  );
  requireNonEmptyArray(gate.blockers, `${label}: missing blocker items`);
  requireNonEmptyArray(gate.evidence_docs, `${label}: missing evidence docs`);

  if (!markdown.includes(`## ${index + 1}. ${gate.title}`)) {
    issues.push(`${markdownRelativePath}: missing heading for ${gate.title}`);
  }

  for (const evidenceDoc of gate.evidence_docs ?? []) {
    evidenceCount += 1;
    if (!fs.existsSync(path.join(docsDir, evidenceDoc))) {
      issues.push(`${label}: missing evidence doc ${evidenceDoc}`);
    }
    if (!markdown.includes(`](${evidenceDoc})`)) {
      issues.push(`${markdownRelativePath}: missing evidence link ${evidenceDoc}`);
    }
  }
}

if (issues.length > 0) {
  console.error("commercial readiness validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log(
  `commercial readiness manifest ok: ${manifest.gates.length} gates, ${evidenceCount} evidence links`,
);

function requireText(value, message) {
  if (typeof value !== "string" || value.trim() === "") {
    issues.push(message);
  }
}

function requireNonEmptyArray(value, message) {
  if (!Array.isArray(value) || value.length === 0) {
    issues.push(message);
  }
}

function sameStrings(left, right) {
  return (
    Array.isArray(left) &&
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function normalizeText(value) {
  return String(value).replace(/\s+/g, " ").trim();
}
