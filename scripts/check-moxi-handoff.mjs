#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { readJson, rootDir } from "./release-metadata.mjs";

const manifestRelativePath = "docs/moxi-handoff.manifest.json";
const markdownRelativePath = "docs/moxi-handoff.md";
const manifestPath = path.join(rootDir, manifestRelativePath);
const markdownPath = path.join(rootDir, markdownRelativePath);
const docsDir = path.join(rootDir, "docs");
const manifest = readJson(manifestPath);
const markdown = fs.readFileSync(markdownPath, "utf8");
const issues = [];

const expectedStates = ["ready", "active", "watch", "defer_to_2x"];
const expectedGateCount = 7;

if (manifest.schema_version !== "kyuubiki.moxi-handoff/v1") {
  issues.push(`${manifestRelativePath}: unexpected schema_version`);
}

if (manifest.from_line !== "moxi 2.0.0") {
  issues.push(`${manifestRelativePath}: from_line must stay moxi 2.0.0`);
}

if (manifest.to_line !== "moxi 2.x") {
  issues.push(`${manifestRelativePath}: to_line must stay moxi 2.x`);
}

if (!sameStrings(manifest.allowed_gate_states, expectedStates)) {
  issues.push(`${manifestRelativePath}: allowed_gate_states drifted`);
}

if (!normalizeText(markdown).includes(normalizeText(manifest.handoff_statement))) {
  issues.push(`${markdownRelativePath}: missing manifest handoff statement`);
}

if (!markdown.includes("moxi-handoff.manifest.json")) {
  issues.push(`${markdownRelativePath}: missing paired manifest reference`);
}

if (!Array.isArray(manifest.gates) || manifest.gates.length !== expectedGateCount) {
  issues.push(`${manifestRelativePath}: expected ${expectedGateCount} gates`);
}

const gateIds = new Set();
let evidenceCount = 0;

for (const [index, gate] of (manifest.gates ?? []).entries()) {
  const label = `${manifestRelativePath}: gate ${gate?.id ?? index + 1}`;
  requireText(gate?.id, `${label}: missing id`);
  if (gateIds.has(gate.id)) {
    issues.push(`${label}: duplicate id`);
  }
  gateIds.add(gate.id);

  requireText(gate.title, `${label}: missing title`);
  requireText(gate.state, `${label}: missing state`);
  if (!expectedStates.includes(gate.state)) {
    issues.push(`${label}: unsupported state ${gate.state}`);
  }
  requireText(gate.handoff_question, `${label}: missing handoff_question`);
  requireNonEmptyArray(gate.must_close, `${label}: missing must_close items`);
  requireNonEmptyArray(gate.evidence_docs, `${label}: missing evidence docs`);

  if (!markdown.includes(`### ${index + 1}. ${gate.title}`)) {
    issues.push(`${markdownRelativePath}: missing heading for ${gate.title}`);
  }

  for (const item of gate.must_close ?? []) {
    if (!markdown.includes(item)) {
      issues.push(`${markdownRelativePath}: missing must_close item for ${gate.id}`);
    }
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
  console.error("moxi handoff validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log(
  `moxi handoff manifest ok: ${manifest.gates.length} gates, ${evidenceCount} evidence links`,
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
