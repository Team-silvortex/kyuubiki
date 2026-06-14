import { buildWorkbenchGovernedAuthHeaders } from "@/lib/workbench/governance";

const SETTINGS_KEY = "kyuubiki-workbench-settings";
const SECRETS_KEY = "kyuubiki-workbench-secrets";

function authHeadersFor(url: string) {
  if (typeof window === "undefined") return {};

  try {
    const rawSecrets = window.sessionStorage.getItem(SECRETS_KEY);
    const rawSettings = window.localStorage.getItem(SETTINGS_KEY);
    if (!rawSecrets && !rawSettings) return {};

    const parsedSecrets = rawSecrets
      ? (JSON.parse(rawSecrets) as {
          controlPlaneApiToken?: string;
          clusterApiToken?: string;
          directMeshApiToken?: string;
        })
      : {};
    const parsedLegacySettings = rawSettings
      ? (JSON.parse(rawSettings) as {
          frontendRuntimeMode?: "orchestrated_gui" | "direct_mesh_gui";
          controlPlaneApiToken?: string;
          clusterApiToken?: string;
          directMeshApiToken?: string;
        })
      : {};

    const parsed = {
      controlPlaneApiToken:
        parsedSecrets.controlPlaneApiToken ?? parsedLegacySettings.controlPlaneApiToken,
      clusterApiToken: parsedSecrets.clusterApiToken ?? parsedLegacySettings.clusterApiToken,
      directMeshApiToken:
        parsedSecrets.directMeshApiToken ?? parsedLegacySettings.directMeshApiToken,
    } as {
      controlPlaneApiToken?: string;
      clusterApiToken?: string;
      directMeshApiToken?: string;
    };

    return buildWorkbenchGovernedAuthHeaders({
      url,
      frontendRuntimeMode:
        parsedLegacySettings.frontendRuntimeMode === "direct_mesh_gui"
          ? "direct_mesh_gui"
          : "orchestrated_gui",
      secrets: parsed,
    });
  } catch {
    return {};
  }
}

const DEFAULT_REQUEST_TIMEOUT_MS = 15_000;

function isAbortLikeError(error: unknown): boolean {
  return error instanceof DOMException
    ? error.name === "AbortError"
    : error instanceof Error && error.name === "AbortError";
}

function buildTimeoutMessage(url: string) {
  return `request timed out: ${url}`;
}

async function fetchWithTimeout(url: string, init?: RequestInit, timeoutMs = DEFAULT_REQUEST_TIMEOUT_MS) {
  const controller = new AbortController();
  const timeoutId = window.setTimeout(() => controller.abort(buildTimeoutMessage(url)), timeoutMs);
  const forwardAbort = () => controller.abort(init?.signal?.reason);

  if (init?.signal) {
    if (init.signal.aborted) {
      controller.abort(init.signal.reason);
    } else {
      init.signal.addEventListener("abort", forwardAbort, { once: true });
    }
  }

  try {
    return await fetch(url, {
      ...init,
      signal: controller.signal,
    });
  } catch (error) {
    if (isAbortLikeError(error)) {
      const reason =
        typeof controller.signal.reason === "string" && controller.signal.reason.trim()
          ? controller.signal.reason
          : buildTimeoutMessage(url);
      throw new Error(reason);
    }

    throw error;
  } finally {
    window.clearTimeout(timeoutId);
    init?.signal?.removeEventListener("abort", forwardAbort);
  }
}

async function readResponsePayload(response: Response) {
  const contentType = response.headers.get("content-type") ?? "";

  if (contentType.includes("application/json")) {
    try {
      return await response.json();
    } catch {
      return null;
    }
  }

  const text = await response.text();
  if (!text.trim()) return null;

  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export async function requestJson<T>(url: string, init?: RequestInit, timeoutMs?: number): Promise<T> {
  const headers = new Headers(init?.headers);
  Object.entries(authHeadersFor(url)).forEach(([key, value]) => {
    if (value) headers.set(key, value);
  });

  const response = await fetchWithTimeout(
    url,
    {
      ...init,
      headers,
    },
    timeoutMs,
  );
  const payload = (await readResponsePayload(response)) as (T & { error?: string; message?: string }) | string | null;

  if (!response.ok) {
    if (payload && typeof payload === "object") {
      throw new Error(payload.error ?? payload.message ?? `request failed: ${response.status}`);
    }

    if (typeof payload === "string" && payload.trim()) {
      throw new Error(payload);
    }

    throw new Error(`request failed: ${response.status}`);
  }

  return (payload ?? {}) as T;
}

export async function requestText(url: string, init?: RequestInit, timeoutMs?: number): Promise<string> {
  const headers = new Headers(init?.headers);
  Object.entries(authHeadersFor(url)).forEach(([key, value]) => {
    if (value) headers.set(key, value);
  });

  const response = await fetchWithTimeout(
    url,
    {
      ...init,
      headers,
    },
    timeoutMs,
  );
  const payload = await response.text();

  if (!response.ok) {
    throw new Error(payload || `request failed: ${response.status}`);
  }

  return payload;
}
