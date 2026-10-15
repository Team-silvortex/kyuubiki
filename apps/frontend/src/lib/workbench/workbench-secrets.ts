import { advanceWorkbenchAuthorityRevision } from "@/lib/api/authority-revisions";

export const WORKBENCH_SECRETS_KEY = "kyuubiki-workbench-secrets";

export type StoredWorkbenchSecrets = {
  controlPlaneApiToken?: string;
  clusterApiToken?: string;
  directMeshApiToken?: string;
  assistantApiKey?: string;
};

let inMemoryWorkbenchSecrets: StoredWorkbenchSecrets = {};

export function readInMemoryWorkbenchSecrets(): StoredWorkbenchSecrets {
  return { ...inMemoryWorkbenchSecrets };
}

export function writeInMemoryWorkbenchSecrets(input: StoredWorkbenchSecrets) {
  const next: StoredWorkbenchSecrets = {
    ...(input.controlPlaneApiToken?.trim()
      ? { controlPlaneApiToken: input.controlPlaneApiToken.trim() }
      : {}),
    ...(input.clusterApiToken?.trim() ? { clusterApiToken: input.clusterApiToken.trim() } : {}),
    ...(input.directMeshApiToken?.trim()
      ? { directMeshApiToken: input.directMeshApiToken.trim() }
      : {}),
    ...(input.assistantApiKey?.trim() ? { assistantApiKey: input.assistantApiKey.trim() } : {}),
  };
  for (const key of ["controlPlaneApiToken", "clusterApiToken", "directMeshApiToken"] as const) {
    if (inMemoryWorkbenchSecrets[key] !== next[key]) advanceWorkbenchAuthorityRevision(key);
  }
  inMemoryWorkbenchSecrets = next;
}

export function scrubPersistedWorkbenchSecrets() {
  if (typeof window === "undefined") return;
  window.sessionStorage.removeItem(WORKBENCH_SECRETS_KEY);
}
