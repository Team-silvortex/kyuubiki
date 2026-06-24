#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

async function main() {
  const contract = JSON.parse(await readFile(path.join(ROOT, "config/toolchains.json"), "utf8"));
  const env = {
    KYUUBIKI_RUST_TOOLCHAIN: contract.rust.preferred,
    KYUUBIKI_RUST_CHANNEL: contract.rust.channel,
    KYUUBIKI_ELIXIR_CONSTRAINT: contract.elixir.constraint,
    KYUUBIKI_ELIXIR_MINIMUM: contract.elixir.minimum,
    KYUUBIKI_OTP_MINIMUM: contract.elixir.otp_minimum,
    KYUUBIKI_ELIXIR_CONTAINER_BASE: contract.elixir.container_base,
    KYUUBIKI_REMOTE_OTP_VERSION: contract.elixir.lab_otp,
    KYUUBIKI_REMOTE_ELIXIR_VERSION: contract.elixir.lab_elixir,
    KYUUBIKI_NODE_VERSION: contract.node.preferred,
    KYUUBIKI_NODE_ENGINE: contract.node.package_engine,
    KYUUBIKI_HEADLESS_LIVE_TEST_IMAGE: contract.docker.headless_live_test_image,
    KYUUBIKI_DIRECT_MESH_BENCHMARK_IMAGE: contract.docker.direct_mesh_benchmark_image,
    KYUUBIKI_POSTGRES_IMAGE: contract.docker.postgres_image,
  };

  if (process.argv.includes("--json")) {
    console.log(JSON.stringify(env, null, 2));
    return;
  }

  for (const [key, value] of Object.entries(env)) {
    console.log(`export ${key}=${shellQuote(value)}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
