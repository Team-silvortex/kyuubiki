export const DESKTOP_LANGUAGE_LABELS = {
    en: "English",
    zh: "中文",
    ja: "日本語",
    es: "Español",
    ar: "العربية · Arabic",
    bn: "বাংলা · Bengali",
    cs: "Čeština · Czech",
    da: "Dansk · Danish",
    de: "Deutsch · German",
    el: "Ελληνικά · Greek",
    fa: "فارسی · Persian",
    fi: "Suomi · Finnish",
    fr: "Français · French",
    he: "עברית · Hebrew",
    hi: "हिन्दी · Hindi",
    id: "Bahasa Indonesia · Indonesian",
    it: "Italiano · Italian",
    ko: "한국어 · Korean",
    ms: "Bahasa Melayu · Malay",
    nl: "Nederlands · Dutch",
    no: "Norsk · Norwegian",
    pl: "Polski · Polish",
    "pt-BR": "Português (Brasil)",
    ro: "Română · Romanian",
    ru: "Русский · Russian",
    sv: "Svenska · Swedish",
    sw: "Kiswahili · Swahili",
    ta: "தமிழ் · Tamil",
    th: "ไทย · Thai",
    tr: "Türkçe · Turkish",
    uk: "Українська · Ukrainian",
    ur: "اردو · Urdu",
    vi: "Tiếng Việt · Vietnamese",
    "zh-TW": "繁體中文 · Traditional Chinese",
};
const BUILTIN_DESKTOP_LANGUAGES = new Set(["en", "zh", "ja", "es"]);
const MAX_LANGUAGE_PACK_FRAGMENTS = 32;
const packCache = new Map();
function languageSlug(language) {
    return language.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}
function isPlainObject(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
function candidatePackPaths(surface, language) {
    const slug = languageSlug(language);
    return [
        `./language-packs/${surface}/${slug}.json`,
        `../../../../language-packs/${surface}/${slug}.json`,
        `../../../language-packs/${surface}/${slug}.json`,
    ];
}
async function fetchJson(path) {
    try {
        const response = await fetch(path, { cache: "no-store" });
        if (!response.ok)
            return null;
        return response.json();
    }
    catch (_error) {
        return null;
    }
}
function isSafeRelativePath(path) {
    if (typeof path !== "string" || !path || path.startsWith("/") || path.includes("\\"))
        return false;
    return !path.split("/").some((segment) => !segment || segment === "." || segment === "..");
}
function mergeOverrides(target, source) {
    for (const [key, value] of Object.entries(source)) {
        const current = target[key];
        if (isPlainObject(current) && isPlainObject(value))
            mergeOverrides(current, value);
        else
            target[key] = value;
    }
}
function fragmentPath(rootPath, relativePath) {
    return `${rootPath.slice(0, rootPath.lastIndexOf("/") + 1)}${relativePath}`;
}
async function loadPackFragments(surface, language, rootPath, pack) {
    if (pack.fragments === undefined)
        return pack;
    if (!Array.isArray(pack.fragments) || pack.fragments.length > MAX_LANGUAGE_PACK_FRAGMENTS)
        return null;
    const batches = new Set();
    const paths = new Set();
    const references = [];
    for (const reference of pack.fragments) {
        if (!isPlainObject(reference) || typeof reference.batch !== "string" || !reference.batch)
            return null;
        if (!isSafeRelativePath(reference.path) ||
            !reference.path.startsWith(`${languageSlug(language)}/`) ||
            !reference.path.endsWith(".json") ||
            batches.has(reference.batch) ||
            paths.has(reference.path)) {
            return null;
        }
        batches.add(reference.batch);
        paths.add(reference.path);
        references.push({ batch: reference.batch, path: reference.path });
    }
    const payloads = await Promise.all(references.map((reference) => fetchJson(fragmentPath(rootPath, reference.path))));
    const overrides = {};
    if (isPlainObject(pack.overrides))
        mergeOverrides(overrides, pack.overrides);
    for (let index = 0; index < references.length; index += 1) {
        const reference = references[index];
        const payload = payloads[index];
        if (!isPlainObject(payload) ||
            payload.schema_version !== "kyuubiki.language-pack-fragment/v1" ||
            payload.language !== language ||
            payload.targetSurface !== surface ||
            payload.batch !== reference.batch ||
            !isPlainObject(payload.overrides)) {
            return null;
        }
        mergeOverrides(overrides, payload.overrides);
    }
    return { ...pack, overrides };
}
function validatePack(surface, language, value) {
    if (!isPlainObject(value) || !isPlainObject(value.overrides))
        return null;
    if (value.targetSurface !== surface || value.language !== language)
        return null;
    return value;
}
export function isBuiltinDesktopLanguage(language) {
    return BUILTIN_DESKTOP_LANGUAGES.has(language);
}
export function describeDesktopLanguage(language) {
    return DESKTOP_LANGUAGE_LABELS[language] || language;
}
export function buildDesktopLanguageOptions() {
    return Object.entries(DESKTOP_LANGUAGE_LABELS).map(([value, label]) => ({ value, label }));
}
export async function loadDesktopLanguagePack(surface, language) {
    const normalized = typeof language === "string" && language.trim() ? language.trim() : "en";
    if (isBuiltinDesktopLanguage(normalized)) {
        return {
            status: "builtin",
            language: normalized,
            message: `${describeDesktopLanguage(normalized)} is built in.`,
        };
    }
    const cacheKey = `${surface}:${normalized}`;
    const cached = packCache.get(cacheKey);
    if (cached)
        return cached;
    let invalidBundle = false;
    for (const path of candidatePackPaths(surface, normalized)) {
        const payload = await fetchJson(path);
        const pack = validatePack(surface, normalized, payload);
        if (!pack)
            continue;
        const completePack = await loadPackFragments(surface, normalized, path, pack);
        if (!completePack) {
            invalidBundle = true;
            continue;
        }
        const result = {
            status: "loaded",
            language: normalized,
            path,
            pack: completePack,
            message: `${completePack.name || describeDesktopLanguage(normalized)} loaded lazily from ${path}.`,
        };
        packCache.set(cacheKey, result);
        return result;
    }
    const result = {
        status: invalidBundle ? "invalid" : "missing",
        language: normalized,
        message: invalidBundle
            ? `${describeDesktopLanguage(normalized)} language pack is incomplete or invalid; falling back to English.`
            : `${describeDesktopLanguage(normalized)} language pack is not bundled; falling back to English until the pack is installed.`,
    };
    packCache.set(cacheKey, result);
    return result;
}
