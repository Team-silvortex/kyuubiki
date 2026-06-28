import { HUB_ASSISTANT_AUDIT_KEY, HUB_ASSISTANT_AUDIT_LIMIT, HUB_ASSISTANT_LEGACY_SECRETS_KEY, HUB_ASSISTANT_MODEL_PRESETS, HUB_ASSISTANT_SETTINGS_KEY, HUB_ASSISTANT_TRUSTED_HOSTS_KEY, HUB_DENSITY_DEFAULTS, HUB_DENSITY_SETTINGS_KEY, HUB_HOT_LOG_SETTINGS_KEY, HUB_RECENTS_KEY, HUB_RUNTIME_LOG_SETTINGS_KEY, HUB_WORKLOAD_LIBRARY_KEY, HUB_WORKLOAD_LIBRARY_LIMIT, } from "./hub-app-config.js";
import { loadHubWorkloadLibrary as loadStoredHubWorkloadLibrary, persistHubWorkloadLibrary as persistStoredHubWorkloadLibrary, } from "./hub-workload-library.js";
function parseStoredJson(storage, key, fallback) {
    const raw = storage.getItem(key);
    return raw ? JSON.parse(raw) : fallback;
}
function asRecord(value) {
    return value && typeof value === "object" ? value : {};
}
function asArray(value) {
    return Array.isArray(value) ? value : [];
}
function stringSetFromStorage(storageKey) {
    const parsed = parseStoredJson(window.localStorage, storageKey, []);
    return new Set(asArray(parsed).filter((item) => typeof item === "string"));
}
export function loadHubRecents() {
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
    }
    catch {
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
        const parsed = asRecord(parseStoredJson(window.localStorage, HUB_ASSISTANT_SETTINGS_KEY, {}));
        const modelPreset = String(parsed.modelPreset || "");
        return {
            mode: parsed.mode === "llm" ? "llm" : "local",
            baseUrl: String(parsed.baseUrl || ""),
            modelPreset: HUB_ASSISTANT_MODEL_PRESETS.includes(modelPreset) ? modelPreset : "gpt-5",
            model: String(parsed.model || "gpt-5"),
        };
    }
    catch {
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
        return stringSetFromStorage(storageKey);
    }
    catch {
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
    }
    catch {
        // Ignore cleanup failures for best-effort legacy secret removal.
    }
}
export function loadHubAssistantAudit() {
    try {
        return asArray(parseStoredJson(window.sessionStorage, HUB_ASSISTANT_AUDIT_KEY, []));
    }
    catch {
        return [];
    }
}
export function persistHubAssistantAudit(entries) {
    window.sessionStorage.setItem(HUB_ASSISTANT_AUDIT_KEY, JSON.stringify(entries.slice(0, HUB_ASSISTANT_AUDIT_LIMIT)));
}
export function loadHubHotLogSettings() {
    try {
        const parsed = asRecord(parseStoredJson(window.localStorage, HUB_HOT_LOG_SETTINGS_KEY, {}));
        const interval = String(parsed.interval || "4000");
        return {
            service: String(parsed.service || "hot-stack"),
            autoRefresh: parsed.autoRefresh !== false,
            interval: interval === "2000" || interval === "8000" ? interval : "4000",
        };
    }
    catch {
        return { service: "hot-stack", autoRefresh: true, interval: "4000" };
    }
}
export function persistHubHotLogSettings(settings) {
    window.localStorage.setItem(HUB_HOT_LOG_SETTINGS_KEY, JSON.stringify(settings));
}
export function loadHubRuntimeLogSettings() {
    try {
        const parsed = asRecord(parseStoredJson(window.localStorage, HUB_RUNTIME_LOG_SETTINGS_KEY, {}));
        const service = String(parsed.service || "frontend");
        return {
            service: service === "orchestrator" || service === "agent-5001" || service === "agent-5002"
                ? service
                : "frontend",
            autoRefresh: parsed.autoRefresh !== false,
        };
    }
    catch {
        return { service: "frontend", autoRefresh: true };
    }
}
export function persistHubRuntimeLogSettings(settings) {
    window.localStorage.setItem(HUB_RUNTIME_LOG_SETTINGS_KEY, JSON.stringify(settings));
}
export function loadHubDensitySettings() {
    try {
        const parsed = asRecord(parseStoredJson(window.localStorage, HUB_DENSITY_SETTINGS_KEY, {}));
        return Object.fromEntries(Object.entries(HUB_DENSITY_DEFAULTS).map(([key, defaultExpanded]) => [
            key,
            typeof parsed[key] === "boolean" ? parsed[key] : defaultExpanded,
        ]));
    }
    catch {
        return { ...HUB_DENSITY_DEFAULTS };
    }
}
