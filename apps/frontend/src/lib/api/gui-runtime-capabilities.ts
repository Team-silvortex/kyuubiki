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
