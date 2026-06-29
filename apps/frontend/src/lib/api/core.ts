import { buildWorkbenchGovernedAuthHeaders } from "@/lib/workbench/governance";
import {
  createHttpWorkbenchRequestError,
  normalizeWorkbenchRequestError,
} from "@/lib/api/request-errors";
import { resolveWorkbenchApiUrl } from "@/lib/api/backend-target";
import { readInMemoryWorkbenchSecrets } from "@/lib/workbench/workbench-secrets";

const SETTINGS_KEY = "kyuubiki-workbench-settings";

function authHeadersFor(url: string) {
  if (typeof window === "undefined") return {};

  try {
    const rawSettings = window.localStorage.getItem(SETTINGS_KEY);
    const parsedSecrets = readInMemoryWorkbenchSecrets();
    if (Object.keys(parsedSecrets).length === 0 && !rawSettings) return {};

    const parsedLegacySettings = rawSettings
      ? (JSON.parse(rawSettings) as {
          frontendRuntimeMode?: "orchestrated_gui" | "direct_mesh_gui";
        })
      : {};

    const parsed = {
      controlPlaneApiToken: parsedSecrets.controlPlaneApiToken,
      clusterApiToken: parsedSecrets.clusterApiToken,
      directMeshApiToken: parsedSecrets.directMeshApiToken,
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
  const timeoutId = globalThis.setTimeout(() => controller.abort(buildTimeoutMessage(url)), timeoutMs);
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
    globalThis.clearTimeout(timeoutId);
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
  const requestUrl = resolveWorkbenchApiUrl(url);
  const headers = new Headers(init?.headers);
  Object.entries(authHeadersFor(url)).forEach(([key, value]) => {
    if (value) headers.set(key, value);
  });

  try {
    const response = await fetchWithTimeout(
      requestUrl,
      {
        ...init,
        headers,
      },
      timeoutMs,
    );
    const payload = (await readResponsePayload(response)) as (T & { error?: string; message?: string }) | string | null;

    if (!response.ok) {
      if (payload && typeof payload === "object") {
        throw createHttpWorkbenchRequestError({
          message: payload.error ?? payload.message ?? `request failed: ${response.status}`,
          responseMessage: payload.error ?? payload.message ?? null,
          statusCode: response.status,
          url: requestUrl,
        });
      }

      if (typeof payload === "string" && payload.trim()) {
        throw createHttpWorkbenchRequestError({
          message: payload,
          responseMessage: payload,
          statusCode: response.status,
          url: requestUrl,
        });
      }

      throw createHttpWorkbenchRequestError({
        message: `request failed: ${response.status}`,
        statusCode: response.status,
        url: requestUrl,
      });
    }

    return (payload ?? {}) as T;
  } catch (error) {
    throw normalizeWorkbenchRequestError(error, requestUrl);
  }
}

export async function requestText(url: string, init?: RequestInit, timeoutMs?: number): Promise<string> {
  const requestUrl = resolveWorkbenchApiUrl(url);
  const headers = new Headers(init?.headers);
  Object.entries(authHeadersFor(url)).forEach(([key, value]) => {
    if (value) headers.set(key, value);
  });

  try {
    const response = await fetchWithTimeout(
      requestUrl,
      {
        ...init,
        headers,
      },
      timeoutMs,
    );
    const payload = await response.text();

    if (!response.ok) {
      throw createHttpWorkbenchRequestError({
        message: payload || `request failed: ${response.status}`,
        responseMessage: payload || null,
        statusCode: response.status,
        url: requestUrl,
      });
    }

    return payload;
  } catch (error) {
    throw normalizeWorkbenchRequestError(error, requestUrl);
  }
}
