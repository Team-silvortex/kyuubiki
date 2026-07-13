import type { GuiRuntimeCapabilityManifest } from "./gui-runtime-capabilities.ts";

export const INSTALLER_RUNTIME_SURFACE_SCHEMA_VERSION =
  "kyuubiki.installer-runtime-surface/v1";

export type InstallerRuntimeCapability =
  | "install.plan"
  | "integrity.verify"
  | "residue.cleanup"
  | "update.catalog.read"
  | "desktop.package.stage"
  | "runtime.repair"
  | "deployment.plan.submit"
  | "runtime.target.observe"
  | "agent.bootstrap.request"
  | "update.status.read";

export type InstallerRuntimeRoute = {
  id: string;
  bindingId: string;
  targetKind: string;
  bindingMode: string;
  capabilities: InstallerRuntimeCapability[];
  writesLocalState: boolean;
  mobileSupported: boolean;
};

export type InstallerValidationGate = {
  id: string;
  capability: InstallerRuntimeCapability;
  requiredBefore: string;
  failureMode: "block_action" | "diagnostics_only";
};

export type InstallerRuntimeSurface = {
  schemaVersion: typeof INSTALLER_RUNTIME_SURFACE_SCHEMA_VERSION;
  owner: "installer-shell";
  routes: InstallerRuntimeRoute[];
  validationGates: InstallerValidationGate[];
  degradedModes: string[];
};

const INSTALLER_VALIDATION_GATES: InstallerValidationGate[] = [
  {
    id: "verify-component-integrity",
    capability: "integrity.verify",
    requiredBefore: "install.plan",
    failureMode: "block_action",
  },
  {
    id: "preview-residue-cleanup",
    capability: "residue.cleanup",
    requiredBefore: "desktop.package.stage",
    failureMode: "block_action",
  },
  {
    id: "observe-remote-runtime-target",
    capability: "runtime.target.observe",
    requiredBefore: "deployment.plan.submit",
    failureMode: "diagnostics_only",
  },
];

export function buildInstallerRuntimeSurface(
  manifest: GuiRuntimeCapabilityManifest,
): InstallerRuntimeSurface {
  return {
    schemaVersion: INSTALLER_RUNTIME_SURFACE_SCHEMA_VERSION,
    owner: "installer-shell",
    routes: manifest.runtime_bindings.map((binding) => ({
      id: `installer.${binding.binding_id}`,
      bindingId: binding.binding_id,
      targetKind: binding.target_kind,
      bindingMode: binding.binding_mode,
      capabilities: [
        ...(binding.required_capabilities ?? []),
        ...(binding.optional_capabilities ?? []),
      ] as InstallerRuntimeCapability[],
      writesLocalState: binding.target_kind === "installer_runtime",
      mobileSupported: binding.mobile_supported,
    })),
    validationGates: INSTALLER_VALIDATION_GATES,
    degradedModes:
      "degraded_modes" in manifest && Array.isArray(manifest.degraded_modes)
        ? manifest.degraded_modes.map((mode) => String(mode.mode))
        : [],
  };
}

export function listInstallerRuntimeCapabilities(
  surface: InstallerRuntimeSurface,
) {
  return [...new Set(surface.routes.flatMap((route) => route.capabilities))].sort();
}

export function validateInstallerRuntimeSurface(surface: InstallerRuntimeSurface) {
  const capabilities = new Set(listInstallerRuntimeCapabilities(surface));
  const missing = surface.validationGates.filter(
    (gate) =>
      !capabilities.has(gate.capability) || !capabilities.has(gate.requiredBefore),
  );

  return {
    ok: missing.length === 0,
    missingGateIds: missing.map((gate) => gate.id),
  };
}
