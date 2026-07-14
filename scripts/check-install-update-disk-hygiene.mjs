#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  installationIntegrityContractPath,
  readJson,
  rootDir,
  updateChannelsPath,
} from "./release-metadata.mjs";

const contractRelativePath = "deploy/install-update-disk-hygiene.json";
const contractPath = path.join(rootDir, contractRelativePath);
const expectedSchema = "kyuubiki.install-update-disk-hygiene/v1";
const issues = [];

if (process.argv.includes("--self-test")) {
  runSelfTest();
  process.exit(0);
}

const contract = readJson(contractPath);
const integrity = readJson(installationIntegrityContractPath);
const channels = readJson(updateChannelsPath);
const packaging = readText("docs/packaging-and-deployment.md");
const uploadRunner = readText("workers/rust/crates/script-runner/src/desktop_release_upload_remote.rs");

validateContract(contract, integrity, channels, packaging, uploadRunner);

if (issues.length > 0) {
  console.error("install/update disk hygiene validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log(
  `install/update disk hygiene ok: ${contract.local_retention_policy.removable_roots.length} removable roots`,
);

function validateContract(contractValue, integrityValue, channelsValue, docs, runner) {
  if (contractValue.schema_version !== expectedSchema) {
    fail(`${contractRelativePath}: unexpected schema_version`);
  }

  if (contractValue.shipping_version !== integrityValue.shipping_version) {
    fail(`${contractRelativePath}: shipping_version does not match installation integrity`);
  }

  if (contractValue.shipping_version !== channelsValue.shipping_version) {
    fail(`${contractRelativePath}: shipping_version does not match update channels`);
  }

  requireSourceContract(
    contractValue.source_contracts?.installation_integrity,
    "deploy/installation-integrity-contract.json",
  );
  requireSourceContract(contractValue.source_contracts?.update_channels, "deploy/update-channels.json");
  requireSourceContract(
    contractValue.source_contracts?.packaging_docs,
    "docs/packaging-and-deployment.md",
  );

  validateRemotePolicy(contractValue.remote_artifact_policy, docs, runner);
  validateLocalRetention(contractValue.local_retention_policy, integrityValue, docs, runner);
  validateUpdateVisibility(contractValue.update_visibility_policy, channelsValue);
  requireNonEmptyArray(contractValue.operator_visible_rules, "operator_visible_rules");
}

function validateRemotePolicy(policy, docs, runner) {
  requireText(policy?.authority, "remote_artifact_policy.authority");
  if (policy?.authority !== "remote_download_server") {
    fail("remote_artifact_policy.authority must be remote_download_server");
  }

  requireText(policy?.required_command, "remote_artifact_policy.required_command");
  if (!docs.includes(policy.required_command)) {
    fail("packaging docs must mention the remote upload command");
  }

  for (const key of ["remote_root_env", "remote_host_env", "version_env", "password_env"]) {
    requireText(policy?.[key], `remote_artifact_policy.${key}`);
    if (!docs.includes(policy[key]) || !runner.includes(policy[key])) {
      fail(`${key} must be documented and implemented by the native upload runner`);
    }
  }

  if (policy?.password_policy !== "temporary_compatibility_only") {
    fail("password_policy must keep password uploads temporary-only");
  }
}

function validateLocalRetention(policy, integrityValue, docs, runner) {
  if (policy?.default !== "metadata_and_source_only") {
    fail("local_retention_policy.default must be metadata_and_source_only");
  }

  if (policy?.purge_after_remote_upload_env !== "PURGE_LOCAL") {
    fail("local_retention_policy must use PURGE_LOCAL as the purge switch");
  }

  if (policy?.purge_after_remote_upload_value !== "1") {
    fail("local_retention_policy purge value must be 1");
  }

  if (!docs.includes("PURGE_LOCAL=1") || !runner.includes("PURGE_LOCAL")) {
    fail("PURGE_LOCAL=1 must be documented and implemented by the native upload runner");
  }

  for (const root of policy?.removable_roots ?? []) {
    validateRelativePath(root, "removable_roots");
  }

  for (const root of policy?.protected_roots ?? []) {
    validateRelativePath(root, "protected_roots");
    if (!(integrityValue.protected_paths ?? []).includes(root)) {
      fail(`protected root is not protected by installation integrity: ${root}`);
    }
  }

  if ((policy?.removable_roots ?? []).some((root) => root === "tmp/data")) {
    fail("tmp/data must never be a release-bundle purge target");
  }
}

function validateUpdateVisibility(policy, channelsValue) {
  const channel = (channelsValue.channels ?? []).find((entry) => entry.id === policy?.required_channel);
  if (!channel) {
    fail(`missing update channel: ${policy?.required_channel}`);
    return;
  }

  if (channel.rollout?.cleanup_policy !== policy.required_cleanup_policy) {
    fail("update channel cleanup policy drifted from disk hygiene contract");
  }

  if (channel.rollout?.rollback !== policy.required_rollback) {
    fail("update channel rollback policy drifted from disk hygiene contract");
  }

  if (channel.rollout?.requires_integrity_contract !== policy.requires_integrity_contract) {
    fail("update channel integrity contract link drifted from disk hygiene contract");
  }
}

function validateRelativePath(value, field) {
  if (typeof value !== "string" || value.trim() === "") {
    fail(`${field}: empty path`);
    return;
  }

  if (path.isAbsolute(value) || value.includes("..")) {
    fail(`${field}: path must be repo-relative and non-traversing: ${value}`);
  }
}

function requireSourceContract(value, expected) {
  if (value !== expected) {
    fail(`source contract must point at ${expected}`);
  }

  if (!fs.existsSync(path.join(rootDir, expected))) {
    fail(`source contract is missing on disk: ${expected}`);
  }
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    fail(`${label}: missing text`);
  }
}

function requireNonEmptyArray(value, label) {
  if (!Array.isArray(value) || value.length === 0) {
    fail(`${label}: missing non-empty array`);
  }
}

function readText(relativePath) {
  return fs.readFileSync(path.join(rootDir, relativePath), "utf8");
}

function fail(message) {
  issues.push(message);
}

function runSelfTest() {
  issues.length = 0;
  const sample = {
    schema_version: expectedSchema,
    shipping_version: "1.20.0",
    source_contracts: {
      installation_integrity: "deploy/installation-integrity-contract.json",
      update_channels: "deploy/update-channels.json",
      packaging_docs: "docs/packaging-and-deployment.md",
    },
    remote_artifact_policy: {
      authority: "remote_download_server",
      required_command: "./scripts/kyuubiki desktop-upload-remote",
      remote_root_env: "KYUUBIKI_RELEASE_REMOTE_DIR",
      remote_host_env: "KYUUBIKI_RELEASE_REMOTE_HOST",
      version_env: "KYUUBIKI_RELEASE_VERSION",
      password_env: "KYUUBIKI_RELEASE_REMOTE_PASSWORD",
      password_policy: "temporary_compatibility_only",
    },
    local_retention_policy: {
      default: "metadata_and_source_only",
      purge_after_remote_upload_env: "PURGE_LOCAL",
      purge_after_remote_upload_value: "1",
      removable_roots: ["dist/macos"],
      protected_roots: ["tmp/data"],
    },
    update_visibility_policy: {
      required_channel: "stable",
      required_cleanup_policy: "allowlisted",
      required_rollback: "same-channel reinstall",
      requires_integrity_contract: "deploy/installation-integrity-contract.json",
    },
    operator_visible_rules: ["visible cleanup"],
  };

  validateContract(
    sample,
    { shipping_version: "1.20.0", protected_paths: ["tmp/data"] },
    {
      shipping_version: "1.20.0",
      channels: [
        {
          id: "stable",
          rollout: {
            cleanup_policy: "allowlisted",
            rollback: "same-channel reinstall",
            requires_integrity_contract: "deploy/installation-integrity-contract.json",
          },
        },
      ],
    },
    "PURGE_LOCAL=1 ./scripts/kyuubiki desktop-upload-remote KYUUBIKI_RELEASE_REMOTE_DIR KYUUBIKI_RELEASE_REMOTE_HOST KYUUBIKI_RELEASE_VERSION KYUUBIKI_RELEASE_REMOTE_PASSWORD",
    "PURGE_LOCAL KYUUBIKI_RELEASE_REMOTE_DIR KYUUBIKI_RELEASE_REMOTE_HOST KYUUBIKI_RELEASE_VERSION KYUUBIKI_RELEASE_REMOTE_PASSWORD",
  );

  if (issues.length > 0) {
    throw new Error(`self-test unexpectedly failed: ${issues.join("; ")}`);
  }

  issues.length = 0;
  sample.local_retention_policy.removable_roots = ["/tmp/unsafe"];
  validateContract(sample, { shipping_version: "1.20.0", protected_paths: ["tmp/data"] }, {
    shipping_version: "1.20.0",
    channels: [],
  }, "", "");

  if (!issues.some((issue) => issue.includes("repo-relative"))) {
    throw new Error("self-test did not reject absolute removable root");
  }

  console.log("install/update disk hygiene self-test passed");
}
