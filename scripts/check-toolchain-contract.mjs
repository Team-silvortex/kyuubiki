#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

async function readText(relativePath) {
  return await readFile(path.join(ROOT, relativePath), "utf8");
}

async function readJson(relativePath) {
  return JSON.parse(await readText(relativePath));
}

function requireContains(issues, file, text, expected, label) {
  if (!text.includes(expected)) {
    issues.push(`${file}: expected ${label} to include ${JSON.stringify(expected)}`);
  }
}

function requirePackageEngine(issues, file, manifest, expected) {
  const actual = manifest.engines?.node;
  if (actual !== expected) {
    issues.push(`${file}: engines.node is ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}

function requirePackageLockEngine(issues, file, lockfile, expected) {
  const actual = lockfile.packages?.[""]?.engines?.node;
  if (actual !== expected) {
    issues.push(`${file}: root engines.node is ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}

async function main() {
  const contract = await readJson("config/toolchains.json");
  const issues = [];

  const rustToolchain = await readText("rust-toolchain.toml");
  requireContains(issues, "rust-toolchain.toml", rustToolchain, `channel = "${contract.rust.channel}"`, "Rust channel");
  requireContains(issues, "rust-toolchain.toml", rustToolchain, `profile = "${contract.rust.profile}"`, "Rust profile");

  for (const component of contract.rust.components) {
    requireContains(issues, "rust-toolchain.toml", rustToolchain, `"${component}"`, `Rust component ${component}`);
  }

  const directMeshDockerfile = await readText("deploy/docker/direct-mesh-benchmark.Dockerfile");
  requireContains(
    issues,
    "deploy/docker/direct-mesh-benchmark.Dockerfile",
    directMeshDockerfile,
    `ARG BASE_IMAGE=${contract.elixir.container_base}`,
    "Elixir container base",
  );
  requireContains(
    issues,
    "deploy/docker/direct-mesh-benchmark.Dockerfile",
    directMeshDockerfile,
    `ARG NODE_VERSION=${contract.node.preferred}`,
    "Node version",
  );
  requireContains(
    issues,
    "deploy/docker/direct-mesh-benchmark.Dockerfile",
    directMeshDockerfile,
    `ARG RUST_TOOLCHAIN=${contract.rust.preferred}`,
    "Rust toolchain",
  );

  const headlessDockerfile = await readText("deploy/docker/headless-live-test.Dockerfile");
  requireContains(
    issues,
    "deploy/docker/headless-live-test.Dockerfile",
    headlessDockerfile,
    `ARG BASE_IMAGE=${contract.elixir.container_base}`,
    "Elixir container base",
  );
  requireContains(
    issues,
    "deploy/docker/headless-live-test.Dockerfile",
    headlessDockerfile,
    `ARG RUST_IMAGE=rust:${contract.rust.preferred}`,
    "Rust base image",
  );

  const webMix = await readText("apps/web/mix.exs");
  requireContains(issues, "apps/web/mix.exs", webMix, `elixir: "${contract.elixir.constraint}"`, "Elixir constraint");

  const sdkMix = await readText("sdks/elixir/mix.exs");
  requireContains(issues, "sdks/elixir/mix.exs", sdkMix, `elixir: "${contract.elixir.constraint}"`, "Elixir SDK constraint");

  const webConfig = await readText("apps/web/config/config.exs");
  for (const envKey of contract.elixir.self_host_required_env) {
    requireContains(issues, "apps/web/config/config.exs", webConfig, envKey, `self-host env ${envKey}`);
  }

  const elixirSelfHostScript = await readText("scripts/check-elixir-self-host.mjs");
  requireContains(issues, "scripts/check-elixir-self-host.mjs", elixirSelfHostScript, "config/toolchains.json", "shared toolchain contract");
  requireContains(issues, "scripts/check-elixir-self-host.mjs", elixirSelfHostScript, "apps/web/mix.exs", "web Mix contract");

  const embeddedRuntime = await readText("workers/rust/crates/installer/src/embedded_runtime.rs");
  requireContains(issues, "workers/rust/crates/installer/src/embedded_runtime.rs", embeddedRuntime, "kyuubiki.embedded-runtimes/v1", "embedded runtime schema");
  requireContains(issues, "workers/rust/crates/installer/src/embedded_runtime.rs", embeddedRuntime, "config/toolchains.json#/elixir", "embedded Elixir source contract");
  requireContains(issues, "workers/rust/crates/installer/src/embedded_runtime.rs", embeddedRuntime, "config/toolchains.json#/node", "embedded Node source contract");

  const runtimeResolver = await readText("scripts/kyuubiki-runtime-resolver.mjs");
  requireContains(issues, "scripts/kyuubiki-runtime-resolver.mjs", runtimeResolver, "embedded-runtimes.json", "embedded runtime manifest lookup");
  requireContains(issues, "scripts/kyuubiki-runtime-resolver.mjs", runtimeResolver, "KYUUBIKI_RUNTIME_STRICT", "strict runtime mode");
  requireContains(issues, "scripts/kyuubiki-runtime-resolver.mjs", runtimeResolver, "host-fallback", "host fallback visibility");

  const remoteMeshRunner = await readText("workers/rust/crates/script-runner/src/workflow_mesh_remote.rs");
  requireContains(issues, "workers/rust/crates/script-runner/src/workflow_mesh_remote.rs", remoteMeshRunner, "scripts/toolchain-env.mjs", "toolchain env loader");
  requireContains(issues, "workers/rust/crates/script-runner/src/workflow_mesh_remote.rs", remoteMeshRunner, "KYUUBIKI_REMOTE_OTP_VERSION", "remote OTP default key");
  requireContains(issues, "workers/rust/crates/script-runner/src/workflow_mesh_remote.rs", remoteMeshRunner, "KYUUBIKI_REMOTE_ELIXIR_VERSION", "remote Elixir default key");

  for (const file of [
    "apps/frontend/package.json",
    "apps/hub-gui/package.json",
    "apps/workbench-gui/package.json",
    "apps/installer-gui/package.json",
  ]) {
    requirePackageEngine(issues, file, await readJson(file), contract.node.package_engine);
    requirePackageLockEngine(issues, file.replace("package.json", "package-lock.json"), await readJson(file.replace("package.json", "package-lock.json")), contract.node.package_engine);
  }

  if (issues.length > 0) {
    console.error("toolchain contract drift detected:");
    for (const issue of issues) {
      console.error(`- ${issue}`);
    }
    process.exit(1);
  }

  console.log("toolchain contract ok");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
