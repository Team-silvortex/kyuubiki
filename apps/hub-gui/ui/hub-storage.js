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

export function loadHubRecents() {
  try {
    const raw = window.localStorage.getItem(HUB_RECENTS_KEY);
    if (!raw) {
      return { bundles: [], compares: [], outputs: [] };
    }

    const parsed = JSON.parse(raw);
    return {
      bundles: Array.isArray(parsed?.bundles) ? parsed.bundles : [],
      compares: Array.isArray(parsed?.compares) ? parsed.compares : [],
      outputs: Array.isArray(parsed?.outputs) ? parsed.outputs : [],
      actions: Array.isArray(parsed?.actions) ? parsed.actions : [],
    };
  } catch {
    return { bundles: [], compares: [], outputs: [], actions: [] };
  }
}

export function persistHubRecents(recents) {
  window.localStorage.setItem(HUB_RECENTS_KEY, JSON.stringify(recents));
}

export function loadHubWorkloadLibrary() {
  return loadStoredHubWorkloadLibrary(HUB_WORKLOAD_LIBRARY_KEY);
}

export function persistHubWorkloadLibrary(entries) {
  persistStoredHubWorkloadLibrary(HUB_WORKLOAD_LIBRARY_KEY, entries, HUB_WORKLOAD_LIBRARY_LIMIT);
}

export function loadHubAssistantSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_ASSISTANT_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return {
      mode: parsed?.mode === "llm" ? "llm" : "local",
      baseUrl: String(parsed?.baseUrl || ""),
      modelPreset: HUB_ASSISTANT_MODEL_PRESETS.includes(String(parsed?.modelPreset || "")) ? parsed.modelPreset : "gpt-5",
      model: String(parsed?.model || "gpt-5"),
    };
  } catch {
    return { mode: "local", baseUrl: "", modelPreset: "gpt-5", model: "gpt-5" };
  }
}

export function persistHubAssistantSettings(settings) {
  window.localStorage.setItem(HUB_ASSISTANT_SETTINGS_KEY, JSON.stringify(settings));
}

export function loadHubAssistantTrustedHosts() {
  return loadHubTrustedHosts(HUB_ASSISTANT_TRUSTED_HOSTS_KEY);
}

export function loadHubTrustedHosts(storageKey) {
  try {
    const raw = window.localStorage.getItem(storageKey);
    const parsed = raw ? JSON.parse(raw) : [];
    return new Set(Array.isArray(parsed) ? parsed.filter((item) => typeof item === "string") : []);
  } catch {
    return new Set();
  }
}

export function persistHubAssistantTrustedHosts(hosts) {
  persistHubTrustedHosts(HUB_ASSISTANT_TRUSTED_HOSTS_KEY, hosts);
}

export function persistHubTrustedHosts(storageKey, hosts) {
  window.localStorage.setItem(storageKey, JSON.stringify(Array.from(hosts)));
}

export function clearLegacyHubAssistantSecrets() {
  try {
    window.sessionStorage.removeItem(HUB_ASSISTANT_LEGACY_SECRETS_KEY);
  } catch {
    // Ignore cleanup failures for best-effort legacy secret removal.
  }
}

export function loadHubAssistantAudit() {
  try {
    const raw = window.sessionStorage.getItem(HUB_ASSISTANT_AUDIT_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function persistHubAssistantAudit(entries) {
  window.sessionStorage.setItem(HUB_ASSISTANT_AUDIT_KEY, JSON.stringify(entries.slice(0, HUB_ASSISTANT_AUDIT_LIMIT)));
}

export function loadHubHotLogSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_HOT_LOG_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const interval = String(parsed?.interval || "4000");
    return {
      service: String(parsed?.service || "hot-stack"),
      autoRefresh: parsed?.autoRefresh !== false,
      interval: ["2000", "4000", "8000"].includes(interval) ? interval : "4000",
    };
  } catch {
    return { service: "hot-stack", autoRefresh: true, interval: "4000" };
  }
}

export function persistHubHotLogSettings(settings) {
  window.localStorage.setItem(HUB_HOT_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

export function loadHubRuntimeLogSettings() {
  try {
    const raw = window.localStorage.getItem(HUB_RUNTIME_LOG_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const service = String(parsed?.service || "frontend");
    return {
      service: ["frontend", "orchestrator", "agent-5001", "agent-5002"].includes(service) ? service : "frontend",
      autoRefresh: parsed?.autoRefresh !== false,
    };
  } catch {
    return { service: "frontend", autoRefresh: true };
  }
}

export function persistHubRuntimeLogSettings(settings) {
  window.localStorage.setItem(HUB_RUNTIME_LOG_SETTINGS_KEY, JSON.stringify(settings));
}

export function loadHubDensitySettings() {
  try {
    const raw = window.localStorage.getItem(HUB_DENSITY_SETTINGS_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return Object.fromEntries(
      Object.entries(HUB_DENSITY_DEFAULTS).map(([key, defaultExpanded]) => [
        key,
        typeof parsed?.[key] === "boolean" ? parsed[key] : defaultExpanded,
      ]),
    );
  } catch {
    return { ...HUB_DENSITY_DEFAULTS };
  }
}
