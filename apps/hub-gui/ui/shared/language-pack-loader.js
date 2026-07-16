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
    for (const path of candidatePackPaths(surface, normalized)) {
        const payload = await fetchJson(path);
        const pack = validatePack(surface, normalized, payload);
        if (!pack)
            continue;
        const result = {
            status: "loaded",
            language: normalized,
            path,
            pack,
            message: `${pack.name || describeDesktopLanguage(normalized)} loaded lazily from ${path}.`,
        };
        packCache.set(cacheKey, result);
        return result;
    }
    const result = {
        status: "missing",
        language: normalized,
        message: `${describeDesktopLanguage(normalized)} language pack is not bundled; falling back to English until the pack is installed.`,
    };
    packCache.set(cacheKey, result);
    return result;
}
