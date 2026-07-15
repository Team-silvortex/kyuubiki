#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const artifactPath = "evidence/operator-qualification/line-field-closed-form-baseline.json";
const requiredOperators = new Set([
  "solve.bar_1d",
  "solve.thermal_bar_1d",
  "solve.heat_bar_1d",
  "solve.electrostatic_bar_1d",
]);

function fail(message) {
  console.error(`line-field closed-form baseline check failed: ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function isFiniteNumber(value) {
  return typeof value === "number" && Number.isFinite(value);
}

function findFieldOverride(policy, operatorId, field) {
  return (policy.field_overrides ?? []).find((override) => {
    if (override.field !== field) {
      return false;
    }
    return !override.operator_id || override.operator_id === operatorId;
  });
}

function validateTolerancePolicy(policy) {
  if (policy.schema_version !== "kyuubiki.operator-qualification-tolerance-policy/v1") {
    fail("tolerance_policy: unexpected schema_version");
  }
  if (policy.candidate_id !== "line-field-closed-form") {
    fail("tolerance_policy: candidate_id must be line-field-closed-form");
  }
  for (const kind of ["absolute", "relative", "sign"]) {
    if (!policy.policy?.[kind]) {
      fail(`tolerance_policy: missing ${kind} policy`);
    }
  }
  if (!Array.isArray(policy.scope?.not_allowed) || policy.scope.not_allowed.length === 0) {
    fail("tolerance_policy: scope.not_allowed must document what this policy cannot claim");
  }
}

function validateNumericToleranceAgainstPolicy(expectation, policy, operatorId, context) {
  const tolerance = expectation.tolerance;
  const override = findFieldOverride(policy, operatorId, expectation.field);
  if (override && override.allowed_kind !== tolerance.kind) {
    fail(`${context}: tolerance kind must match field override ${override.allowed_kind}`);
  }
  const maxValue = override?.max_value ?? policy.policy[tolerance.kind]?.max_value;
  if (!isFiniteNumber(maxValue) || tolerance.value > maxValue) {
    fail(`${context}: tolerance ${tolerance.value} exceeds policy max ${maxValue}`);
  }
}

function validateTolerance(expectation, policy, operatorId, context) {
  const tolerance = expectation.tolerance;
  if (!tolerance || typeof tolerance.kind !== "string") {
    fail(`${context}: tolerance.kind is required`);
  }
  if (tolerance.kind === "absolute" || tolerance.kind === "relative") {
    if (!isFiniteNumber(tolerance.value) || tolerance.value <= 0) {
      fail(`${context}: numeric tolerance must be finite and positive`);
    }
    validateNumericToleranceAgainstPolicy(expectation, policy, operatorId, context);
    return;
  }
  if (tolerance.kind === "sign") {
    if (typeof tolerance.value !== "string" || tolerance.value.length === 0) {
      fail(`${context}: sign tolerance must carry a non-empty label`);
    }
    if (typeof expectation.value !== "string" || expectation.value.length === 0) {
      fail(`${context}: sign expectation must carry a non-empty value`);
    }
    const allowed = policy.policy.sign?.allowed_values ?? [];
    if (!allowed.includes(tolerance.value)) {
      fail(`${context}: sign tolerance value ${tolerance.value} is not policy-approved`);
    }
    return;
  }
  fail(`${context}: unsupported tolerance kind ${tolerance.kind}`);
}

function validateExpectation(expectation, policy, operatorId, context) {
  if (!expectation || typeof expectation.field !== "string" || expectation.field.length === 0) {
    fail(`${context}: expectation.field is required`);
  }
  if (typeof expectation.value !== "string" && !isFiniteNumber(expectation.value)) {
    fail(`${context}: expectation.value must be a finite number or sign label`);
  }
  validateTolerance(expectation, policy, operatorId, `${context}:${expectation.field}`);
}

function validateBaseline(baseline, source, policy, seenOperators) {
  const context = baseline?.operator_id ?? "unknown operator";
  if (!requiredOperators.has(context)) {
    fail(`${context}: not part of line-field-closed-form candidate`);
  }
  if (seenOperators.has(context)) {
    fail(`${context}: duplicate baseline entry`);
  }
  seenOperators.add(context);
  if (typeof baseline.test_name !== "string" || !source.includes(baseline.test_name)) {
    fail(`${context}: source test does not contain ${baseline.test_name}`);
  }
  if (typeof baseline.case_id !== "string" || baseline.case_id.length === 0) {
    fail(`${context}: case_id is required`);
  }
  if (!baseline.closed_form || typeof baseline.closed_form.summary !== "string") {
    fail(`${context}: closed_form.summary is required`);
  }
  if (!Array.isArray(baseline.closed_form.formulae) || baseline.closed_form.formulae.length === 0) {
    fail(`${context}: closed_form.formulae must be non-empty`);
  }
  if (!baseline.inputs || typeof baseline.inputs !== "object") {
    fail(`${context}: inputs must be present`);
  }
  if (!Array.isArray(baseline.expectations) || baseline.expectations.length === 0) {
    fail(`${context}: expectations must be non-empty`);
  }
  for (const expectation of baseline.expectations) {
    validateExpectation(expectation, policy, context, context);
  }
}

function validateDerivationNote(artifact, source) {
  if (typeof artifact.derivation_note !== "string") {
    fail("derivation_note is required");
  }
  const notePath = path.join(repoRoot, artifact.derivation_note);
  if (!fs.existsSync(notePath)) {
    fail(`derivation_note does not exist: ${artifact.derivation_note}`);
  }
  const note = fs.readFileSync(notePath, "utf8");
  for (const operatorId of requiredOperators) {
    if (!note.includes(operatorId)) {
      fail(`derivation_note does not mention ${operatorId}`);
    }
  }
  for (const baseline of artifact.baselines ?? []) {
    if (typeof baseline.test_name === "string" && !source.includes(baseline.test_name)) {
      fail(`${baseline.operator_id}: missing source test ${baseline.test_name}`);
    }
    if (typeof baseline.case_id === "string" && !note.includes(baseline.case_id)) {
      fail(`${baseline.operator_id}: derivation_note does not mention ${baseline.case_id}`);
    }
  }
}

function loadTolerancePolicy(artifact) {
  if (typeof artifact.tolerance_policy !== "string") {
    fail("tolerance_policy is required");
  }
  const policyPath = path.join(repoRoot, artifact.tolerance_policy);
  if (!fs.existsSync(policyPath)) {
    fail(`tolerance_policy does not exist: ${artifact.tolerance_policy}`);
  }
  const policy = readJson(artifact.tolerance_policy);
  validateTolerancePolicy(policy);
  return policy;
}

function validate() {
  const artifact = readJson(artifactPath);
  if (artifact.schema_version !== "kyuubiki.operator-qualification-baseline/v1") {
    fail("unexpected schema_version");
  }
  if (artifact.version_line !== "moxi 2.0.x") {
    fail("version_line must match moxi 2.0.x");
  }
  if (artifact.candidate_id !== "line-field-closed-form") {
    fail("candidate_id must be line-field-closed-form");
  }
  if (artifact.status !== "collecting") {
    fail("status must be collecting until qualification is granted");
  }
  if (typeof artifact.source_test !== "string") {
    fail("source_test is required");
  }
  const sourcePath = path.join(repoRoot, artifact.source_test);
  if (!fs.existsSync(sourcePath)) {
    fail(`source_test does not exist: ${artifact.source_test}`);
  }
  const source = fs.readFileSync(sourcePath, "utf8");
  const policy = loadTolerancePolicy(artifact);
  validateDerivationNote(artifact, source);
  if (!Array.isArray(artifact.baselines) || artifact.baselines.length !== requiredOperators.size) {
    fail(`baselines must contain exactly ${requiredOperators.size} entries`);
  }
  const seenOperators = new Set();
  for (const baseline of artifact.baselines) {
    validateBaseline(baseline, source, policy, seenOperators);
  }
  for (const operatorId of requiredOperators) {
    if (!seenOperators.has(operatorId)) {
      fail(`missing baseline for ${operatorId}`);
    }
  }
  console.log(`line-field closed-form baseline ok: ${artifact.baselines.length} operators`);
}

validate();
