export const HUB_COPY_OVERRIDE_STORAGE_KEY = "hub.copy-overrides.v1";
export const HUB_COPY_IMPORT_MANIFEST_STORAGE_KEY = "hub.copy-import-manifest.v1";
export const HUB_LANGUAGE_PACK_SCHEMA_VERSION = "kyuubiki.language-pack/v1";
let cachedRegistryRaw = null;
let cachedRegistryValue = createEmptyRegistry();
let cachedManifestRaw = null;
let cachedManifestValue = createEmptyImportManifest();
function createEmptyRegistry() {
    return {
        defaults: {},
        languages: {},
    };
}
function createEmptyImportManifest() {
    return {
        mode: "none",
        importedAt: "",
        registryLabel: "",
        packs: [],
    };
}
function isPlainObject(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
function cloneValue(value) {
    if (Array.isArray(value)) {
        return value.map((entry) => cloneValue(entry));
    }
    if (!isPlainObject(value)) {
        return value;
    }
    const result = {};
    Object.entries(value).forEach(([key, entry]) => {
        result[key] = cloneValue(entry);
    });
    return result;
}
function mergeCopyBranch(base, override, options = {}) {
    if (!isPlainObject(base)) {
        if (override === undefined) {
            return cloneValue(base);
        }
        if (options.preserveShape && base !== undefined && isPlainObject(override)) {
            return cloneValue(base);
        }
        if (options.preserveShape && Array.isArray(base) !== Array.isArray(override) && base !== undefined) {
            return cloneValue(base);
        }
        return cloneValue(override);
    }
    const result = cloneValue(base);
    if (!isPlainObject(override)) {
        return override === undefined || options.preserveShape ? result : cloneValue(override);
    }
    Object.entries(override).forEach(([key, entry]) => {
        if (isPlainObject(result[key]) && isPlainObject(entry)) {
            result[key] = mergeCopyBranch(result[key], entry, options);
            return;
        }
        if (options.preserveShape && result[key] !== undefined) {
            const leftIsObject = isPlainObject(result[key]);
            const rightIsObject = isPlainObject(entry);
            const leftIsArray = Array.isArray(result[key]);
            const rightIsArray = Array.isArray(entry);
            if (leftIsObject !== rightIsObject || leftIsArray !== rightIsArray) {
                return;
            }
        }
        result[key] = cloneValue(entry);
    });
    return result;
}
function appendLanguageOverrides(target, languages) {
    if (!isPlainObject(languages)) {
        return;
    }
    Object.entries(languages).forEach(([language, entry]) => {
        if (!isPlainObject(entry)) {
            return;
        }
        target.languages[language] = mergeCopyBranch(target.languages[language] || {}, entry);
    });
}
function normalizeHubCopyOverrideRegistry(payload) {
    const normalized = createEmptyRegistry();
    if (!isPlainObject(payload)) {
        return normalized;
    }
    if (isPlainObject(payload.defaults)) {
        normalized.defaults = mergeCopyBranch(normalized.defaults, payload.defaults);
    }
    if (isPlainObject(payload.languages)) {
        appendLanguageOverrides(normalized, payload.languages);
    }
    if (typeof payload.language === "string" && isPlainObject(payload.overrides)) {
        appendLanguageOverrides(normalized, {
            [payload.language]: payload.overrides,
        });
    }
    else if (isPlainObject(payload.overrides)) {
        normalized.defaults = mergeCopyBranch(normalized.defaults, payload.overrides);
    }
    ["en", "zh", "ja", "es"].forEach((language) => {
        if (isPlainObject(payload[language])) {
            appendLanguageOverrides(normalized, {
                [language]: payload[language],
            });
        }
    });
    return normalized;
}
function normalizePackDescriptor(entry) {
    if (!isPlainObject(entry)) {
        return null;
    }
    const language = typeof entry.language === "string" ? entry.language.trim() : "";
    const id = typeof entry.id === "string" ? entry.id.trim() : "";
    const name = typeof entry.name === "string" ? entry.name.trim() : "";
    if (!language || !id || !name) {
        return null;
    }
    return {
        id,
        language,
        name,
        version: typeof entry.version === "string" ? entry.version.trim() : "",
        versionLine: typeof entry.versionLine === "string" ? entry.versionLine.trim() : "",
        targetAppVersion: typeof entry.targetAppVersion === "string" ? entry.targetAppVersion.trim() : "",
        source: typeof entry.source === "string" ? entry.source.trim() : "",
        updatedAt: typeof entry.updatedAt === "string" ? entry.updatedAt.trim() : "",
        description: typeof entry.description === "string" ? entry.description.trim() : "",
        schemaVersion: typeof entry.schemaVersion === "string" ? entry.schemaVersion.trim() : "",
        kind: typeof entry.kind === "string" ? entry.kind.trim() : "language-pack",
    };
}
function normalizeHubCopyImportManifest(payload) {
    const normalized = createEmptyImportManifest();
    if (!isPlainObject(payload)) {
        return normalized;
    }
    const mode = typeof payload.mode === "string" ? payload.mode.trim() : "";
    normalized.mode = mode === "pack" || mode === "registry" ? mode : "none";
    normalized.importedAt = typeof payload.importedAt === "string" ? payload.importedAt.trim() : "";
    normalized.registryLabel = typeof payload.registryLabel === "string" ? payload.registryLabel.trim() : "";
    normalized.packs = Array.isArray(payload.packs)
        ? payload.packs.map((entry) => normalizePackDescriptor(entry)).filter((entry) => Boolean(entry))
        : [];
    return normalized;
}
function buildLanguagePackDescriptor(payload) {
    const language = typeof payload.language === "string" ? payload.language.trim() : "";
    const name = typeof payload.name === "string" ? payload.name.trim() : "";
    const targetSurface = typeof payload.targetSurface === "string" ? payload.targetSurface.trim() : "";
    if (!language || !name || !isPlainObject(payload.overrides) || (targetSurface && targetSurface !== "hub")) {
        return null;
    }
    return normalizePackDescriptor({
        id: typeof payload.id === "string" && payload.id.trim() ? payload.id.trim() : `${language}-hub-pack`,
        language,
        name,
        version: typeof payload.version === "string" ? payload.version : "",
        versionLine: typeof payload.versionLine === "string" ? payload.versionLine : "",
        targetAppVersion: typeof payload.targetAppVersion === "string" ? payload.targetAppVersion : "",
        source: typeof payload.source === "string" ? payload.source : "imported",
        updatedAt: typeof payload.updatedAt === "string" && payload.updatedAt.trim() ? payload.updatedAt : new Date().toISOString(),
        description: typeof payload.description === "string" ? payload.description : "",
        schemaVersion: typeof payload.schema_version === "string" ? payload.schema_version : "",
        kind: "language-pack",
    });
}
function loadManifestRaw(storage = window.localStorage) {
    if (!storage?.getItem) {
        return createEmptyImportManifest();
    }
    let raw = null;
    try {
        raw = storage.getItem(HUB_COPY_IMPORT_MANIFEST_STORAGE_KEY);
    }
    catch (_error) {
        return createEmptyImportManifest();
    }
    if (!raw) {
        cachedManifestRaw = null;
        cachedManifestValue = createEmptyImportManifest();
        return cachedManifestValue;
    }
    if (raw === cachedManifestRaw) {
        return cachedManifestValue;
    }
    try {
        cachedManifestValue = normalizeHubCopyImportManifest(JSON.parse(raw));
        cachedManifestRaw = raw;
        return cachedManifestValue;
    }
    catch (_error) {
        cachedManifestRaw = raw;
        cachedManifestValue = createEmptyImportManifest();
        return cachedManifestValue;
    }
}
function loadHubCopyOverrideRegistryRaw(storage = window.localStorage) {
    if (!storage?.getItem) {
        return createEmptyRegistry();
    }
    let raw = null;
    try {
        raw = storage.getItem(HUB_COPY_OVERRIDE_STORAGE_KEY);
    }
    catch (_error) {
        return createEmptyRegistry();
    }
    if (!raw) {
        cachedRegistryRaw = null;
        cachedRegistryValue = createEmptyRegistry();
        return cachedRegistryValue;
    }
    if (raw === cachedRegistryRaw) {
        return cachedRegistryValue;
    }
    try {
        cachedRegistryValue = normalizeHubCopyOverrideRegistry(JSON.parse(raw));
        cachedRegistryRaw = raw;
        return cachedRegistryValue;
    }
    catch (_error) {
        cachedRegistryRaw = raw;
        cachedRegistryValue = createEmptyRegistry();
        return cachedRegistryValue;
    }
}
export function loadHubCopyOverrideRegistry(storage = window.localStorage) {
    return cloneValue(loadHubCopyOverrideRegistryRaw(storage));
}
export function saveHubCopyOverrideRegistry(payload, storage = window.localStorage) {
    if (!storage?.setItem) {
        return createEmptyRegistry();
    }
    const normalized = normalizeHubCopyOverrideRegistry(payload);
    const serialized = JSON.stringify(normalized);
    storage.setItem(HUB_COPY_OVERRIDE_STORAGE_KEY, serialized);
    cachedRegistryRaw = serialized;
    cachedRegistryValue = normalized;
    return cloneValue(normalized);
}
export function loadHubCopyImportManifest(storage = window.localStorage) {
    return cloneValue(loadManifestRaw(storage));
}
export function saveHubCopyImportManifest(payload, storage = window.localStorage) {
    if (!storage?.setItem) {
        return createEmptyImportManifest();
    }
    const normalized = normalizeHubCopyImportManifest(payload);
    const serialized = JSON.stringify(normalized);
    storage.setItem(HUB_COPY_IMPORT_MANIFEST_STORAGE_KEY, serialized);
    cachedManifestRaw = serialized;
    cachedManifestValue = normalized;
    return cloneValue(normalized);
}
export function clearHubCopyOverrideRegistry(storage = window.localStorage) {
    if (storage?.removeItem) {
        storage.removeItem(HUB_COPY_OVERRIDE_STORAGE_KEY);
        storage.removeItem(HUB_COPY_IMPORT_MANIFEST_STORAGE_KEY);
    }
    cachedRegistryRaw = null;
    cachedRegistryValue = createEmptyRegistry();
    cachedManifestRaw = null;
    cachedManifestValue = createEmptyImportManifest();
}
export function importHubCopyPayload(payload, storage = window.localStorage) {
    const currentRegistry = loadHubCopyOverrideRegistry(storage);
    const currentManifest = loadHubCopyImportManifest(storage);
    const importedAt = new Date().toISOString();
    const packPayload = isPlainObject(payload) ? payload : {};
    if (packPayload.schema_version === HUB_LANGUAGE_PACK_SCHEMA_VERSION ||
        (typeof packPayload.language === "string" && isPlainObject(packPayload.overrides))) {
        const descriptor = buildLanguagePackDescriptor(packPayload);
        if (!descriptor) {
            throw new Error("invalid-hub-copy-pack");
        }
        const nextRegistry = normalizeHubCopyOverrideRegistry({
            ...currentRegistry,
            languages: {
                ...currentRegistry.languages,
                [descriptor.language]: packPayload.overrides,
            },
        });
        const nextPacks = currentManifest.packs.filter((entry) => !(entry.id === descriptor.id && entry.language === descriptor.language));
        nextPacks.unshift(descriptor);
        saveHubCopyOverrideRegistry(nextRegistry, storage);
        saveHubCopyImportManifest({
            mode: "pack",
            importedAt,
            registryLabel: "",
            packs: nextPacks.slice(0, 24),
        }, storage);
        return { mode: "pack", registry: nextRegistry, manifest: loadHubCopyImportManifest(storage), descriptor };
    }
    const nextRegistry = normalizeHubCopyOverrideRegistry(payload);
    saveHubCopyOverrideRegistry(nextRegistry, storage);
    saveHubCopyImportManifest({
        mode: "registry",
        importedAt,
        registryLabel: typeof packPayload.name === "string" ? packPayload.name : "",
        packs: [],
    }, storage);
    return { mode: "registry", registry: nextRegistry, manifest: loadHubCopyImportManifest(storage), descriptor: null };
}
export function resolveHubCopy(baseI18n, language, options = {}) {
    const fallbackLanguage = options.fallbackLanguage || "en";
    const requestedLanguage = typeof language === "string" && language.trim() ? language.trim() : fallbackLanguage;
    const baseLanguage = isPlainObject(baseI18n?.[requestedLanguage]) ? requestedLanguage : fallbackLanguage;
    const baseCopy = baseI18n?.[baseLanguage] || baseI18n?.[fallbackLanguage] || {};
    const registry = loadHubCopyOverrideRegistryRaw(options.storage);
    const mergedDefaults = mergeCopyBranch(baseCopy, registry.defaults, { preserveShape: true });
    const languageOverrides = registry.languages[requestedLanguage] || registry.languages[baseLanguage] || {};
    return mergeCopyBranch(mergedDefaults, languageOverrides, { preserveShape: true });
}
