export type WorkbenchGuiHostKind = "browser" | "desktop_webview" | "mobile_webview";

export type WorkbenchGuiRuntimeCapability = {
  hostKind: WorkbenchGuiHostKind;
  canUseRemoteBackend: boolean;
  canHostOrchestra: boolean;
  canHostAgent: boolean;
  canInstallRuntime: boolean;
  canUseLocalhostBackend: boolean;
  backendSchemes: readonly ["http:", "https:"];
  posture: "remote_control" | "local_workstation";
};

export type WorkbenchGuiHostProbe = {
  userAgent?: string;
  platform?: string;
  isTauri?: boolean;
};

export type WorkbenchGuiBackendTargetDecision = {
  ok: boolean;
  reason?: "invalid_url" | "unsupported_scheme" | "localhost_forbidden";
};

export type GuiRuntimeCapabilityManifestBinding = {
  binding_id: string;
  target_kind:
    | "orchestra"
    | "agent"
    | "mesh"
    | "direct_runtime"
    | "installer_runtime"
    | "offline_bundle";
  binding_mode:
    | "http_control_plane"
    | "solver_rpc"
    | "direct_mesh"
    | "installer_plan"
    | "local_bundle"
    | "read_only";
  required_capabilities?: readonly string[];
  optional_capabilities?: readonly string[];
  mobile_supported: boolean;
};

export type GuiRuntimeCapabilityManifest = {
  schema_version: "kyuubiki.gui-runtime-capability-manifest/v1";
  surface_kind: "hub" | "workbench" | "installer" | "mobile_webview" | "browser_webview";
  runtime_bindings: readonly GuiRuntimeCapabilityManifestBinding[];
};

export type GuiRuntimeCapabilityBindingQuery = {
  hostKind?: WorkbenchGuiHostKind;
  includeOptional?: boolean;
};

export function resolveWorkbenchGuiRuntimeCapability(
  hostKind: WorkbenchGuiHostKind,
): WorkbenchGuiRuntimeCapability {
  const remoteBase = {
    hostKind,
    canUseRemoteBackend: true,
    backendSchemes: ["http:", "https:"] as const,
  };

  if (hostKind === "mobile_webview") {
    return {
      ...remoteBase,
      canHostOrchestra: false,
      canHostAgent: false,
      canInstallRuntime: false,
      canUseLocalhostBackend: false,
      posture: "remote_control",
    };
  }

  return {
    ...remoteBase,
    canHostOrchestra: hostKind === "desktop_webview",
    canHostAgent: hostKind === "desktop_webview",
    canInstallRuntime: hostKind === "desktop_webview",
    canUseLocalhostBackend: true,
    posture: "local_workstation",
  };
}

export function listGuiRuntimeManifestCapabilities(
  manifest: GuiRuntimeCapabilityManifest,
  options: { includeOptional?: boolean } = {},
) {
  const capabilities = new Set<string>();
  for (const binding of manifest.runtime_bindings) {
    for (const capability of binding.required_capabilities ?? []) capabilities.add(capability);
    if (options.includeOptional === true) {
      for (const capability of binding.optional_capabilities ?? []) capabilities.add(capability);
    }
  }
  return [...capabilities].sort();
}

export function hasGuiRuntimeManifestCapability(
  manifest: GuiRuntimeCapabilityManifest,
  capability: string,
  options: GuiRuntimeCapabilityBindingQuery = {},
) {
  return selectGuiRuntimeManifestBindings(manifest, capability, options).length > 0;
}

export function selectGuiRuntimeManifestBindings(
  manifest: GuiRuntimeCapabilityManifest,
  capability: string,
  options: GuiRuntimeCapabilityBindingQuery = {},
) {
  return manifest.runtime_bindings.filter((binding) => {
    if (options.hostKind === "mobile_webview" && binding.mobile_supported !== true) return false;
    if ((binding.required_capabilities ?? []).includes(capability)) return true;
    return options.includeOptional === true && (binding.optional_capabilities ?? []).includes(capability);
  });
}

export function resolveWorkbenchGuiRuntimeCapabilityFromManifest(
  hostKind: WorkbenchGuiHostKind,
  manifest: GuiRuntimeCapabilityManifest,
): WorkbenchGuiRuntimeCapability {
  const base = resolveWorkbenchGuiRuntimeCapability(hostKind);
  const localBackendTargets = new Set(["agent", "mesh", "direct_runtime", "installer_runtime"]);
  const hasLocalBackendBinding = manifest.runtime_bindings.some((binding) =>
    localBackendTargets.has(binding.target_kind),
  );
  const hasInstallerBinding = manifest.runtime_bindings.some(
    (binding) => binding.target_kind === "installer_runtime" || binding.binding_mode === "installer_plan",
  );
  const capabilities = new Set(
    manifest.runtime_bindings.flatMap((binding) => [
      ...(binding.required_capabilities ?? []),
      ...(binding.optional_capabilities ?? []),
    ]),
  );

  if (hostKind === "mobile_webview" || manifest.surface_kind === "mobile_webview") {
    return {
      ...base,
      canHostOrchestra: false,
      canHostAgent: false,
      canInstallRuntime: false,
      canUseLocalhostBackend: false,
      posture: "remote_control",
    };
  }

  return {
    ...base,
    canHostOrchestra: base.canHostOrchestra && capabilities.has("orchestra.host"),
    canHostAgent: base.canHostAgent && capabilities.has("agent.host"),
    canInstallRuntime: base.canInstallRuntime && hasInstallerBinding,
    canUseLocalhostBackend: base.canUseLocalhostBackend && hasLocalBackendBinding,
  };
}

export function inferWorkbenchGuiHostKind(probe: WorkbenchGuiHostProbe = {}): WorkbenchGuiHostKind {
  if (probe.isTauri) return "desktop_webview";

  const userAgent = (probe.userAgent ?? "").toLowerCase();
  const platform = (probe.platform ?? "").toLowerCase();
  const looksMobile =
    /android|iphone|ipad|ipod|mobile/u.test(userAgent) ||
    /iphone|ipad|android/u.test(platform);

  return looksMobile ? "mobile_webview" : "browser";
}

export function explainWorkbenchGuiRuntimeBoundary(capability: WorkbenchGuiRuntimeCapability) {
  if (capability.hostKind === "mobile_webview") {
    return {
      title: "Mobile GUI remote-control boundary",
      summary:
        "Mobile WebView can operate the GUI and call remote backend APIs, but it cannot deploy or host orchestra, agents, or installer-managed runtimes.",
      forbidden: ["host_orchestra", "host_agent", "install_runtime", "localhost_runtime_assumption"],
      allowed: ["remote_backend_control", "workflow_authoring", "result_observation", "remote_job_submission"],
    };
  }

  return {
    title: "Workstation GUI runtime boundary",
    summary:
      "Desktop-capable GUI hosts may bind to local or remote backends while keeping runtime execution behind stable service contracts.",
    forbidden: ["ui_coupled_runtime_internals"],
    allowed: ["local_backend_control", "remote_backend_control", "installer_runtime_management"],
  };
}

export function isWorkbenchBackendTargetAllowedForGuiCapability(
  baseUrl: string,
  capability: WorkbenchGuiRuntimeCapability,
): WorkbenchGuiBackendTargetDecision {
  let parsed: URL;
  try {
    parsed = new URL(baseUrl);
  } catch {
    return { ok: false, reason: "invalid_url" };
  }

  if (!capability.backendSchemes.includes(parsed.protocol as "http:" | "https:")) {
    return { ok: false, reason: "unsupported_scheme" };
  }

  if (!capability.canUseLocalhostBackend && isLocalhostName(parsed.hostname)) {
    return { ok: false, reason: "localhost_forbidden" };
  }

  return { ok: true };
}

function isLocalhostName(hostname: string) {
  const normalized = hostname.toLowerCase().replace(/^\[/u, "").replace(/\]$/u, "");
  return normalized === "localhost" || normalized.startsWith("127.") || normalized === "::1";
}
