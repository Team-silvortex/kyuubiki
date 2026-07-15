#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { operatorReliabilityPaths } from "./operator-reliability-contracts.mjs";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "releases/qualification-review-decisions/2.0.0/beam-frame-classic-review-decision.json";
const schemaPath = "schemas/operator-qualification-review-decision.schema.json";
const allowedDecisions = new Set(["approve_promotion", "request_changes", "reject_promotion", "block_scope"]);
const decisionToReviewStatus = {
  approve_promotion: "approved",
  request_changes: "pending_signoff",
  reject_promotion: "rejected",
  block_scope: "blocked_scope",
};

function fail(message) {
  console.error(`operator qualification review decision check failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { input: defaultInput };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--in") {
      args.input = argv[index + 1];
      index += 1;
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  return args;
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(repoPath(relativePath), "utf8"));
}

function repoPath(relativePath) {
  const absolute = path.resolve(repoRoot, relativePath);
  const relative = path.relative(repoRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail(`path escapes repository: ${relativePath}`);
  }
  return absolute;
}

function requireString(value, field) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${field} must be a non-empty string`);
  }
}

function checkRequiredFields(decision) {
  const schema = readJson(schemaPath);
  if (schema.properties?.schema_version?.const !== "kyuubiki.operator-qualification-review-decision/v1") {
    fail(`${schemaPath}: schema_version const is stale`);
  }
  for (const field of schema.required ?? []) {
    if (!(field in decision)) fail(`missing required field ${field}`);
  }
}

function checkDecision(decision, inputPath) {
  checkRequiredFields(decision);
  if (decision.schema_version !== "kyuubiki.operator-qualification-review-decision/v1") {
    fail("schema_version is invalid");
  }
  for (const field of ["candidate_id", "release_version", "evidence_path", "review_gate", "decision", "reason", "decided_at"]) {
    requireString(decision[field], field);
  }
  if (!allowedDecisions.has(decision.decision)) fail(`unsupported decision ${decision.decision}`);
  if (!decision.reviewer || typeof decision.reviewer !== "object") fail("reviewer must be an object");
  requireString(decision.reviewer.id, "reviewer.id");
  requireString(decision.reviewer.display_name, "reviewer.display_name");
  if (!Array.isArray(decision.completed_gate_items)) fail("completed_gate_items must be an array");
  if (!Array.isArray(decision.requested_changes)) fail("requested_changes must be an array");
  if (decision.decision === "approve_promotion" && decision.completed_gate_items.length === 0) {
    fail("approve_promotion requires completed_gate_items");
  }
  if (decision.decision === "request_changes" && decision.requested_changes.length === 0) {
    fail("request_changes requires requested_changes");
  }
  repoPath(decision.evidence_path);
  checkAgainstReleaseRecord(decision, inputPath);
}

function checkAgainstReleaseRecord(decision, inputPath) {
  const records = readJson(operatorReliabilityPaths.releaseRecords);
  const record = (records.records ?? []).find((entry) => entry.candidate_id === decision.candidate_id);
  if (!record) fail(`${decision.candidate_id}: no release record`);
  if (records.release_version !== decision.release_version) fail("release_version must match release records");
  if (record.evidence_path !== decision.evidence_path) fail("evidence_path must match release record");
  if (record.review_gate !== decision.review_gate) fail("review_gate must match release record");
  if (record.review_decision_path && record.review_decision_path !== inputPath) {
    fail("input path must match release record review_decision_path");
  }
  if (decision.decision === "approve_promotion" && record.review_status === "blocked_scope") {
    fail("approve_promotion cannot override blocked_scope release record");
  }
  const expectedStatus = decisionToReviewStatus[decision.decision];
  if (decision.decision !== "request_changes" && record.review_status !== expectedStatus) {
    fail(`decision ${decision.decision} requires release record review_status=${expectedStatus}`);
  }
}

const args = parseArgs(process.argv);
checkDecision(readJson(args.input), args.input);
console.log(`operator qualification review decision ok: ${args.input}`);
