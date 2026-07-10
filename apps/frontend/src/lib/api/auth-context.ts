import type { FrontendRuntimeMode } from "@/lib/api/runtime-types";
import { buildWorkbenchGovernedAuthHeaders } from "@/lib/workbench/governance";
import { readInMemoryWorkbenchSecrets } from "@/lib/workbench/workbench-secrets";

const SETTINGS_KEY = "kyuubiki-workbench-settings";

type WorkbenchApiAuthSecrets = {
  controlPlaneApiToken?: string;
  clusterApiToken?: string;
  directMeshApiToken?: string;
};

function readFrontendRuntimeModeFromWindow(): FrontendRuntimeMode {
  if (typeof window === "undefined") return "orchestrated_gui";

  try {
    const rawSettings = window.localStorage.getItem(SETTINGS_KEY);
    const parsed = rawSettings
      ? (JSON.parse(rawSettings) as { frontendRuntimeMode?: FrontendRuntimeMode })
      : {};
    return parsed.frontendRuntimeMode === "direct_mesh_gui" ? "direct_mesh_gui" : "orchestrated_gui";
  } catch {
    return "orchestrated_gui";
  }
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
