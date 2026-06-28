import {
  HUB_ASSISTANT_AUDIT_KEY,
  HUB_ASSISTANT_AUDIT_LIMIT,
  HUB_ASSISTANT_LEGACY_SECRETS_KEY,
  HUB_ASSISTANT_MODEL_PRESETS,
  HUB_ASSISTANT_SETTINGS_KEY,
  HUB_ASSISTANT_TRUSTED_HOSTS_KEY,
  HUB_DENSITY_DEFAULTS,
  HUB_DENSITY_SETTINGS_KEY,
  HUB_HOT_LOG_SETTINGS_KEY,
  HUB_RECENTS_KEY,
  HUB_RUNTIME_LOG_SETTINGS_KEY,
  HUB_WORKLOAD_LIBRARY_KEY,
  HUB_WORKLOAD_LIBRARY_LIMIT,
} from "./hub-app-config.js";
import {
  loadHubWorkloadLibrary as loadStoredHubWorkloadLibrary,
  persistHubWorkloadLibrary as persistStoredHubWorkloadLibrary,
} from "./hub-workload-library.js";

type UnknownRecord = Record<string, unknown>;

export type HubRecents = {
  bundles: unknown[];
  compares: unknown[];
  outputs: unknown[];
  actions?: unknown[];
};

export type HubAssistantSettings = {
  mode: "local" | "llm";
  baseUrl: string;
  modelPreset: string;
  model: string;
};

export type HubHotLogSettings = {
  service: string;
  autoRefresh: boolean;
  interval: "2000" | "4000" | "8000";
};

export type HubRuntimeLogSettings = {
  service: "frontend" | "orchestrator" | "agent-5001" | "agent-5002";
  autoRefresh: boolean;
};

function parseStoredJson(storage: Storage, key: string, fallback: unknown): unknown {
  const raw = storage.getItem(key);
  return raw ? JSON.parse(raw) : fallback;
}

function asRecord(value: unknown): UnknownRecord {
  return value && typeof value === "object" ? (value as UnknownRecord) : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function stringSetFromStorage(storageKey: string): Set<string> {
  const parsed = parseStoredJson(window.localStorage, storageKey, []);
  return new Set(asArray(parsed).filter((item): item is string => typeof item === "string"));
}

export function loadHubRecents(): HubRecents {
  try {
    const raw = window.localStorage.getItem(HUB_RECENTS_KEY);
    if (!raw) {
      return { bundles: [], compares: [], outputs: [] };
    }

    const parsed = asRecord(JSON.parse(raw));
    return {
      bundles: asArray(parsed.bundles),
      compares: asArray(parsed.compares),
      outputs: asArray(parsed.outputs),
      actions: asArray(parsed.actions),
    };
  } catch {
    return { bundles: [], compares: [], outputs: [], actions: [] };
  }
}

export function persistHubRecents(recents: HubRecents): void {
  window.localStorage.setItem(HUB_RECENTS_KEY, JSON.stringify(recents));
}

export function loadHubWorkloadLibrary(): unknown[] {
  return loadStoredHubWorkloadLibrary(HUB_WORKLOAD_LIBRARY_KEY);
}

export function persistHubWorkloadLibrary(entries: unknown[]): void {
  persistStoredHubWorkloadLibrary(HUB_WORKLOAD_LIBRARY_KEY, entries, HUB_WORKLOAD_LIBRARY_LIMIT);
}

export function loadHubAssistantSettings(): HubAssistantSettings {
  try {
    const parsed = asRecord(parseStoredJson(window.localStorage, HUB_ASSISTANT_SETTINGS_KEY, {}));
    const modelPreset = String(parsed.modelPreset || "");
    return {
      mode: parsed.mode === "llm" ? "llm" : "local",
      baseUrl: String(parsed.baseUrl || ""),
      modelPreset: HUB_ASSISTANT_MODEL_PRESETS.includes(modelPreset as never) ? modelPreset : "gpt-5",
      model: String(parsed.model || "gpt-5"),
    };
  } catch {
    return { mode: "local", baseUrl: "", modelPreset: "gpt-5", model: "gpt-5" };
  }
}

export function persistHubAssistantSettings(settings: HubAssistantSettings): void {
  window.localStorage.setItem(HUB_ASSISTANT_SETTINGS_KEY, JSON.stringify(settings));
}

export function loadHubAssistantTrustedHosts(): Set<string> {
  return loadHubTrustedHosts(HUB_ASSISTANT_TRUSTED_HOSTS_KEY);
}

export function loadHubTrustedHosts(storageKey: string): Set<string> {
  try {
    return stringSetFromStorage(storageKey);
  } catch {
    return new Set();
  }
}

export function persistHubAssistantTrustedHosts(hosts: Set<string>): void {
  persistHubTrustedHosts(HUB_ASSISTANT_TRUSTED_HOSTS_KEY, hosts);
}

export function persistHubTrustedHosts(storageKey: string, hosts: Set<string>): void {
  window.localStorage.setItem(storageKey, JSON.stringify(Array.from(hosts)));
}

export function clearLegacyHubAssistantSecrets(): void {
  try {
    window.sessionStorage.removeItem(HUB_ASSISTANT_LEGACY_SECRETS_KEY);
  } catch {
    // Ignore cleanup failures for best-effort legacy secret removal.
  }
}

export function loadHubAssistantAudit(): unknown[] {
  try {
    return asArray(parseStoredJson(window.sessionStorage, HUB_ASSISTANT_AUDIT_KEY, []));
  } catch {
    return [];
  }
}

export function persistHubAssistantAudit(entries: unknown[]): void {
  window.sessionStorage.setItem(
    HUB_ASSISTANT_AUDIT_KEY,
    JSON.stringify(entries.slice(0, HUB_ASSISTANT_AUDIT_LIMIT)),
  );
}

export function loadHubHotLogSettings(): HubHotLogSettings {
  try {
    const parsed = asRecord(parseStoredJson(window.localStorage, HUB_HOT_LOG_SETTINGS_KEY, {}));
    const interval = String(parsed.interval || "4000");
    return {
      service: String(parsed.service || "hot-stack"),
      autoRefresh: parsed.autoRefresh !== false,
      interval: interval === "2000" || interval === "8000" ? interval : "4000",
    };
  } catch {
    return { service: "hot-stack", autoRefresh: true, interval: "4000" };
  }
}

export function persistHubHotLogSettings(settings: HubHotLogSettings): void {
  window.localStorage.setItem(HUB_HOT_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

export function loadHubRuntimeLogSettings(): HubRuntimeLogSettings {
  try {
    const parsed = asRecord(parseStoredJson(window.localStorage, HUB_RUNTIME_LOG_SETTINGS_KEY, {}));
    const service = String(parsed.service || "frontend");
    return {
      service:
        service === "orchestrator" || service === "agent-5001" || service === "agent-5002"
          ? service
          : "frontend",
      autoRefresh: parsed.autoRefresh !== false,
    };
  } catch {
    return { service: "frontend", autoRefresh: true };
  }
}

export function persistHubRuntimeLogSettings(settings: HubRuntimeLogSettings): void {
  window.localStorage.setItem(HUB_RUNTIME_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

export function loadHubDensitySettings(): Record<string, boolean> {
  try {
    const parsed = asRecord(parseStoredJson(window.localStorage, HUB_DENSITY_SETTINGS_KEY, {}));
    return Object.fromEntries(
      Object.entries(HUB_DENSITY_DEFAULTS).map(([key, defaultExpanded]) => [
        key,
        typeof parsed[key] === "boolean" ? parsed[key] : defaultExpanded,
      ]),
    );
  } catch {
    return { ...HUB_DENSITY_DEFAULTS };
  }
}
