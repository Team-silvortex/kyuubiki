import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

import type { GuiRuntimeCapabilityManifest } from "../../src/lib/api/gui-runtime-capabilities.ts";
import {
  buildHubRuntimeSurface,
  listHubPersistentProvenance,
  listHubRuntimeCapabilities,
} from "../../src/lib/api/hub-runtime-surface.ts";

const repoRoot = path.resolve(new URL("../../../../", import.meta.url).pathname);

function readHubManifest(): GuiRuntimeCapabilityManifest {
  return JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "config/gui-runtime-capabilities/hub.json"),
      "utf8",
    ),
  ) as GuiRuntimeCapabilityManifest;
}

test("hub runtime surface exposes control-plane and offline catalog routes", () => {
  const surface = buildHubRuntimeSurface(readHubManifest());
  const capabilities = listHubRuntimeCapabilities(surface);

  assert.equal(surface.owner, "hub-shell");
  assert.deepEqual(
    surface.routes.map((route) => route.bindingId),
    ["workload-control-plane", "offline-workload-catalog"],
  );
  assert.ok(capabilities.includes("workload.list"));
  assert.ok(capabilities.includes("runtime.target.observe"));
  assert.ok(capabilities.includes("workload.catalog.read"));
  assert.ok(surface.routes.every((route) => route.headlessParityRequired));
});

test("hub provenance surface keeps metadata without making it user-editable", () => {
  const surface = buildHubRuntimeSurface(readHubManifest());
  const persistent = listHubPersistentProvenance(surface);

  assert.deepEqual(
    persistent.map((channel) => channel.id),
    ["recent-project-metadata", "workload-summary-cache", "hub-diagnostics-export"],
  );
  assert.ok(persistent.every((channel) => channel.editable === false));
  assert.ok(surface.degradedModes.includes("catalog_only"));
});
