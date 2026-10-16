import type { FrontendRuntimeMode } from "@/lib/api/runtime-types";
import { buildWorkbenchGovernedAuthHeaders, workbenchGovernedAuthSecretKeys } from "@/lib/workbench/governance";
import { readInMemoryWorkbenchSecrets } from "@/lib/workbench/workbench-secrets";
import { resolveWorkbenchApiUrl, trackWorkbenchBackendTargetChange } from "./backend-target";
import { advanceWorkbenchAuthorityRevision, readWorkbenchAuthorityRevision, type WorkbenchAuthorityDimension } from "./authority-revisions";
import { ensureWorkbenchAuthorityStorageTracking } from "./authority-storage-events";
import { captureWorkbenchDocumentAuthority } from "./authority-document-lifecycle";

const SETTINGS_KEY = "kyuubiki-workbench-settings";

type WorkbenchApiAuthSecrets = {
  controlPlaneApiToken?: string;
  clusterApiToken?: string;
  directMeshApiToken?: string;
};

function readStoredControlMode(): FrontendRuntimeMode | null {
  if (typeof window === "undefined") return "orchestrated_gui";

  try {
    const rawSettings = window.localStorage.getItem(SETTINGS_KEY);
    const parsed = rawSettings
      ? (JSON.parse(rawSettings) as { frontendRuntimeMode?: FrontendRuntimeMode })
      : {};
    return parsed.frontendRuntimeMode === "direct_mesh_gui" ? "direct_mesh_gui" : "orchestrated_gui";
  } catch {
    return null;
  }
}

function readFrontendRuntimeModeFromWindow(): FrontendRuntimeMode {
  return readStoredControlMode() ?? "orchestrated_gui";
}

export function trackWorkbenchApiConfigurationChange<T>(write: () => T): T {
  const before = readStoredControlMode();
  return trackWorkbenchBackendTargetChange(() => {
    try { return write(); } finally {
      const after = readStoredControlMode();
      if (before === null || after === null || before !== after) advanceWorkbenchAuthorityRevision("mode");
    }
  });
}

function readWorkbenchApiAuthSecrets(): WorkbenchApiAuthSecrets {
  const parsedSecrets = readInMemoryWorkbenchSecrets();
  return {
    controlPlaneApiToken: parsedSecrets.controlPlaneApiToken,
    clusterApiToken: parsedSecrets.clusterApiToken,
    directMeshApiToken: parsedSecrets.directMeshApiToken,
  };
}

export function buildWorkbenchApiAuthHeaders(url: string) {
  if (typeof window === "undefined") return {};

  const secrets = readWorkbenchApiAuthSecrets();
  if (Object.values(secrets).every((value) => !value)) return {};

  return buildWorkbenchGovernedAuthHeaders({
    url,
    frontendRuntimeMode: readFrontendRuntimeModeFromWindow(),
    secrets,
  });
}

export function captureWorkbenchApiAuthority(...urls: string[]) {
  ensureWorkbenchAuthorityStorageTracking();
  const documentCurrent = captureWorkbenchDocumentAuthority();
  const scope = () => {
    const mode = readFrontendRuntimeModeFromWindow(), secrets = readWorkbenchApiAuthSecrets();
    const dimensions = new Set<WorkbenchAuthorityDimension>(["mode"]);
    for (const url of urls) {
      if (url.startsWith("/")) dimensions.add("backend");
      for (const key of workbenchGovernedAuthSecretKeys(url, mode)) {
        dimensions.add(key);
        if (secrets[key]) break;
      }
    }
    return {
      signature: JSON.stringify([mode, urls.map((url) => [resolveWorkbenchApiUrl(url), buildWorkbenchApiAuthHeaders(url)])]),
      revisions: [...dimensions].map(readWorkbenchAuthorityRevision),
    };
  };
  // Kept only for this request's lifetime; never persisted or included in diagnostics.
  const captured = scope();
  let current = true;
  return () => {
    // Restoring settings cannot revive a request already observed to be stale.
    if (!current) return false;
    try {
      if (!documentCurrent()) { current = false; return false; }
      const next = scope();
      current = captured.signature === next.signature && captured.revisions.length === next.revisions.length
        && captured.revisions.every((revision, index) => revision === next.revisions[index]);
      return current;
    } catch (error) {
      current = false;
      throw error;
    }
  };
}
