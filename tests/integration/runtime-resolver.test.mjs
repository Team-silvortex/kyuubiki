import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

import { createRuntimeResolver } from "../../scripts/kyuubiki-runtime-resolver.mjs";

const PLATFORM = process.platform === "darwin" ? "macos" : process.platform;

async function writeRuntimeManifest(rootDir) {
  const manifestDir = path.join(rootDir, "dist", PLATFORM, "manifests");
  await mkdir(manifestDir, { recursive: true });
  await writeFile(
    path.join(manifestDir, "embedded-runtimes.json"),
    JSON.stringify(
      {
        schema_version: "kyuubiki.embedded-runtimes/v1",
        platform: PLATFORM,
        policy: { mode: "bundled_first" },
        runtimes: [
          {
            id: "node",
            target_dir: `runtimes/${PLATFORM}/node`,
            bin_dirs: [`dist/${PLATFORM}/runtimes/${PLATFORM}/node/bin`],
            required_for_self_host: true,
          },
        ],
      },
      null,
      2,
    ),
  );
}

test("runtime resolver reports host fallback when embedded payload is absent", async () => {
  const rootDir = await mkdtemp(path.join(tmpdir(), "kyuubiki-runtime-resolver-"));
  await writeRuntimeManifest(rootDir);
  const resolver = createRuntimeResolver({ rootDir });
  const resolved = resolver.resolveRuntimeCommand("node");
  assert.equal(resolved.source, "host-fallback");
  assert.equal(resolved.runtimeId, "node");
});

test("runtime resolver strict mode rejects missing required embedded runtime", async () => {
  const rootDir = await mkdtemp(path.join(tmpdir(), "kyuubiki-runtime-resolver-"));
  await writeRuntimeManifest(rootDir);
  const previous = process.env.KYUUBIKI_RUNTIME_STRICT;
  process.env.KYUUBIKI_RUNTIME_STRICT = "1";
  try {
    const resolver = createRuntimeResolver({ rootDir });
    assert.throws(
      () => resolver.resolveRuntimeCommand("node"),
      /required embedded runtime node is missing command node/u,
    );
  } finally {
    if (previous === undefined) {
      delete process.env.KYUUBIKI_RUNTIME_STRICT;
    } else {
      process.env.KYUUBIKI_RUNTIME_STRICT = previous;
    }
  }
});
