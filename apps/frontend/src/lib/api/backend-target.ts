export const WORKBENCH_API_BASE_URL_STORAGE_KEY = "kyuubiki-workbench-api-base-url";
export const WORKBENCH_API_BASE_URL_QUERY_KEYS = ["kyuubikiApiBaseUrl", "apiBaseUrl"] as const;

type BackendTargetSource = "query" | "local_storage" | "settings" | "environment" | "default";

export type WorkbenchBackendTarget = {
  baseUrl: string;
  source: BackendTargetSource;
};

type WorkbenchSettingsWithBackendTarget = {
  backendApiBaseUrl?: string;
};

export function sanitizeWorkbenchApiBaseUrl(value: string | null | undefined): string {
  const trimmed = typeof value === "string" ? value.trim() : "";
  if (!trimmed) return "";

  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return "";
    parsed.hash = "";
    return parsed.toString().replace(/\/+$/u, "");
  } catch {
    return "";
  }
}

export function resolveWorkbenchBackendTarget(): WorkbenchBackendTarget {
  const queryBaseUrl = readQueryApiBaseUrl();
  if (queryBaseUrl) return { baseUrl: queryBaseUrl, source: "query" };

  const storedBaseUrl = readStoredApiBaseUrl();
  if (storedBaseUrl) return { baseUrl: storedBaseUrl, source: "local_storage" };

  const settingsBaseUrl = readSettingsApiBaseUrl();
  if (settingsBaseUrl) return { baseUrl: settingsBaseUrl, source: "settings" };

  const environmentBaseUrl = sanitizeWorkbenchApiBaseUrl(process.env.NEXT_PUBLIC_KYUUBIKI_API_BASE_URL);
  if (environmentBaseUrl) return { baseUrl: environmentBaseUrl, source: "environment" };

  return { baseUrl: "", source: "default" };
}

export function readPersistedWorkbenchApiBaseUrl(): string {
  const storage = getBrowserLocalStorage();
  if (!storage) return "";
  return sanitizeWorkbenchApiBaseUrl(storage.getItem(WORKBENCH_API_BASE_URL_STORAGE_KEY));
}

export function persistWorkbenchApiBaseUrl(value: string): string {
  const sanitized = sanitizeWorkbenchApiBaseUrl(value);
  const storage = getBrowserLocalStorage();
  if (!storage) return sanitized;

  if (sanitized) {
    storage.setItem(WORKBENCH_API_BASE_URL_STORAGE_KEY, sanitized);
  } else {
    storage.removeItem(WORKBENCH_API_BASE_URL_STORAGE_KEY);
  }
  return sanitized;
}

export function resolveWorkbenchApiUrl(url: string, target = resolveWorkbenchBackendTarget()): string {
  if (!target.baseUrl || isAbsoluteHttpUrl(url) || !url.startsWith("/")) return url;
  return `${target.baseUrl}${url}`;
}

function isAbsoluteHttpUrl(url: string) {
  return /^https?:\/\//iu.test(url);
}

function readQueryApiBaseUrl() {
  const search = typeof window === "undefined" ? "" : window.location?.search ?? "";
  if (!search) return "";

  for (const key of WORKBENCH_API_BASE_URL_QUERY_KEYS) {
    const value = sanitizeWorkbenchApiBaseUrl(new URLSearchParams(search).get(key));
    if (value) return value;
  }

  return "";
}

function readStoredApiBaseUrl() {
  const storage = getBrowserLocalStorage();
  if (!storage) return "";
  return sanitizeWorkbenchApiBaseUrl(storage.getItem(WORKBENCH_API_BASE_URL_STORAGE_KEY));
}

function readSettingsApiBaseUrl() {
  const storage = getBrowserLocalStorage();
  if (!storage) return "";

  try {
    const raw = storage.getItem("kyuubiki-workbench-settings");
    const settings = raw ? (JSON.parse(raw) as WorkbenchSettingsWithBackendTarget) : {};
    return sanitizeWorkbenchApiBaseUrl(settings.backendApiBaseUrl);
  } catch {
    return "";
  }
}

function getBrowserLocalStorage(): Storage | null {
  return typeof window === "undefined" ? null : window.localStorage ?? null;
}
