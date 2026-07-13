import type { GuiRuntimeCapabilityManifest } from "./gui-runtime-capabilities.ts";

export const HUB_RUNTIME_SURFACE_SCHEMA_VERSION =
  "kyuubiki.hub-runtime-surface/v1";

export type HubRuntimeSurfaceCapability =
  | "workload.list"
  | "runtime.target.observe"
  | "job.summary.read"
  | "agent.registry.observe"
  | "log.summary.read"
  | "update.status.read"
  | "workload.catalog.read"
  | "project.recent.read"
  | "diagnostics.export";

export type HubRuntimeSurfaceRoute = {
  id: string;
  bindingId: string;
  targetKind: string;
  bindingMode: string;
  capabilities: HubRuntimeSurfaceCapability[];
  headlessParityRequired: boolean;
  mobileSupported: boolean;
};

export type HubProvenanceChannel = {
  id: string;
  sourceCapability: HubRuntimeSurfaceCapability;
  retention: "session" | "local_cache" | "exportable_bundle";
  editable: boolean;
};

export type HubRuntimeSurface = {
  schemaVersion: typeof HUB_RUNTIME_SURFACE_SCHEMA_VERSION;
  owner: "hub-shell";
  routes: HubRuntimeSurfaceRoute[];
  provenanceChannels: HubProvenanceChannel[];
  degradedModes: string[];
};

const HUB_PROVENANCE_CHANNELS: HubProvenanceChannel[] = [
  {
    id: "recent-project-metadata",
    sourceCapability: "project.recent.read",
    retention: "local_cache",
    editable: false,
  },
  {
    id: "workload-summary-cache",
    sourceCapability: "workload.catalog.read",
    retention: "local_cache",
    editable: false,
  },
  {
    id: "runtime-health-snapshot",
    sourceCapability: "runtime.target.observe",
    retention: "session",
    editable: false,
  },
  {
    id: "hub-diagnostics-export",
    sourceCapability: "diagnostics.export",
    retention: "exportable_bundle",
    editable: false,
  },
];

export function buildHubRuntimeSurface(
  manifest: GuiRuntimeCapabilityManifest,
): HubRuntimeSurface {
  return {
    schemaVersion: HUB_RUNTIME_SURFACE_SCHEMA_VERSION,
    owner: "hub-shell",
    routes: manifest.runtime_bindings.map((binding) => ({
      id: `hub.${binding.binding_id}`,
      bindingId: binding.binding_id,
      targetKind: binding.target_kind,
      bindingMode: binding.binding_mode,
      capabilities: [
        ...(binding.required_capabilities ?? []),
        ...(binding.optional_capabilities ?? []),
      ] as HubRuntimeSurfaceCapability[],
      headlessParityRequired: true,
      mobileSupported: binding.mobile_supported,
    })),
    provenanceChannels: HUB_PROVENANCE_CHANNELS,
    degradedModes:
      "degraded_modes" in manifest && Array.isArray(manifest.degraded_modes)
        ? manifest.degraded_modes.map((mode) => String(mode.mode))
        : [],
  };
}

export function listHubRuntimeCapabilities(surface: HubRuntimeSurface) {
  return [...new Set(surface.routes.flatMap((route) => route.capabilities))].sort();
}

export function listHubPersistentProvenance(surface: HubRuntimeSurface) {
  return surface.provenanceChannels.filter(
    (channel) => channel.retention !== "session",
  );
}
