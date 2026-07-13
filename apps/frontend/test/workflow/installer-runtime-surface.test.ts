import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

import type { GuiRuntimeCapabilityManifest } from "../../src/lib/api/gui-runtime-capabilities.ts";
import {
  buildInstallerRuntimeSurface,
  listInstallerRuntimeCapabilities,
  validateInstallerRuntimeSurface,
} from "../../src/lib/api/installer-runtime-surface.ts";

const repoRoot = path.resolve(new URL("../../../../", import.meta.url).pathname);

function readInstallerManifest(): GuiRuntimeCapabilityManifest {
  return JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "config/gui-runtime-capabilities/installer.json"),
      "utf8",
    ),
  ) as GuiRuntimeCapabilityManifest;
}

test("installer runtime surface separates local install and remote deployment routes", () => {
  const surface = buildInstallerRuntimeSurface(readInstallerManifest());
  const capabilities = listInstallerRuntimeCapabilities(surface);

  assert.equal(surface.owner, "installer-shell");
  assert.deepEqual(
    surface.routes.map((route) => route.bindingId),
    ["local-installation-plan", "remote-deployment-control"],
  );
  assert.equal(surface.routes[0]?.writesLocalState, true);
  assert.equal(surface.routes[1]?.mobileSupported, true);
  assert.ok(capabilities.includes("install.plan"));
  assert.ok(capabilities.includes("deployment.plan.submit"));
});

test("installer validation gates cover integrity, cleanup, and remote target checks", () => {
  const surface = buildInstallerRuntimeSurface(readInstallerManifest());
  const validation = validateInstallerRuntimeSurface(surface);

  assert.equal(validation.ok, true);
  assert.deepEqual(validation.missingGateIds, []);
  assert.deepEqual(
    surface.validationGates.map((gate) => gate.id),
    [
      "verify-component-integrity",
      "preview-residue-cleanup",
      "observe-remote-runtime-target",
    ],
  );
  assert.ok(surface.degradedModes.includes("diagnostics_only"));
});
