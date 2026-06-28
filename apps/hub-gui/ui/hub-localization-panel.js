import { importHubCopyPayload, HUB_COPY_OVERRIDE_STORAGE_KEY, clearHubCopyOverrideRegistry, loadHubCopyImportManifest, loadHubCopyOverrideRegistry, } from "./hub-copy-registry.js";
const HUB_LANGUAGE_LABELS = {
    en: "English",
    zh: "中文",
    ja: "日本語",
    es: "Español",
};
function isPlainObject(value) {
    return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
function hasNestedEntries(value) {
    if (!isPlainObject(value)) {
        return false;
    }
    return Object.values(value).some((entry) => {
        if (isPlainObject(entry)) {
            return hasNestedEntries(entry) || Object.keys(entry).length > 0;
        }
        return entry !== undefined;
    });
}
function describeLanguage(language) {
    return HUB_LANGUAGE_LABELS[language] || language;
}
export function buildHubLanguageOptions() {
    const registry = loadHubCopyOverrideRegistry();
    const manifest = loadHubCopyImportManifest();
    const packNames = new Map(manifest.packs.map((pack) => [pack.language, pack.name]));
    const languages = [
        ...Object.keys(HUB_LANGUAGE_LABELS),
        ...Object.keys(registry.languages).filter((language) => hasNestedEntries(registry.languages[language])),
    ];
    return [...new Set(languages)]
        .filter((language) => typeof language === "string" && language.trim())
        .map((language) => ({
        value: language,
        label: HUB_LANGUAGE_LABELS[language] || (packNames.get(language) ? `${packNames.get(language)} (${language.toUpperCase()})` : language.toUpperCase()),
    }));
}
function describeImportMode(mode, copy) {
    if (mode === "pack")
        return copy.guides.localizationImportModePack;
    if (mode === "registry")
        return copy.guides.localizationImportModeRegistry;
    return copy.guides.localizationImportModeNone;
}
function describeLatestAsset(manifest, copy) {
    if (manifest.mode === "pack" && manifest.packs[0]) {
        const pack = manifest.packs[0];
        return `${pack.name} · ${describeLanguage(pack.language)}`;
    }
    if (manifest.mode === "registry" && manifest.registryLabel) {
        return manifest.registryLabel;
    }
    if (manifest.mode === "registry") {
        return copy.guides.localizationImportModeRegistry;
    }
    return copy.guides.localizationLatestAssetNone;
}
function downloadHubJson(filename, payload) {
    const blob = new Blob([`${JSON.stringify(payload, null, 2)}\n`], { type: "application/json" });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    window.URL.revokeObjectURL(url);
}
export function renderHubLocalizationPanel({ elements, copy, activeLanguage, setText }) {
    const registry = loadHubCopyOverrideRegistry();
    const manifest = loadHubCopyImportManifest();
    const languageKeys = Object.keys(registry.languages).filter((language) => hasNestedEntries(registry.languages[language]));
    const activeLanguageOverride = hasNestedEntries(registry.languages[activeLanguage]);
    const defaultLayerEnabled = hasNestedEntries(registry.defaults);
    setText(elements.guidesLocalizationLabel, copy.guides.localizationLabel);
    setText(elements.guidesLocalizationTitle, copy.guides.localizationTitle);
    setText(elements.guidesLocalizationCopy, copy.guides.localizationCopy);
    setText(elements.guidesLocalizationActiveLanguageLabel, copy.guides.localizationActiveLanguageLabel);
    setText(elements.guidesLocalizationInstalledLanguagesLabel, copy.guides.localizationInstalledLanguagesLabel);
    setText(elements.guidesLocalizationDefaultLayerLabel, copy.guides.localizationDefaultLayerLabel);
    setText(elements.guidesLocalizationImportModeLabel, copy.guides.localizationImportModeLabel);
    setText(elements.guidesLocalizationLatestAssetLabel, copy.guides.localizationLatestAssetLabel);
    setText(elements.guidesLocalizationStorageKeyLabel, copy.guides.localizationStorageKeyLabel);
    setText(elements.guidesLocalizationImport, copy.guides.localizationImport);
    setText(elements.guidesLocalizationExport, copy.guides.localizationExport);
    setText(elements.guidesLocalizationClear, copy.guides.localizationClear);
    setText(elements.guidesLocalizationActiveLanguageValue, `${describeLanguage(activeLanguage)} · ${activeLanguageOverride ? copy.guides.localizationCurrentEnabled : copy.guides.localizationCurrentDisabled}`);
    setText(elements.guidesLocalizationInstalledLanguagesValue, languageKeys.length > 0
        ? `${languageKeys.length} · ${languageKeys.map((language) => describeLanguage(language)).join(", ")}`
        : copy.guides.localizationLanguagesNone);
    setText(elements.guidesLocalizationDefaultLayerValue, defaultLayerEnabled ? copy.guides.localizationDefaultEnabled : copy.guides.localizationDefaultDisabled);
    setText(elements.guidesLocalizationImportModeValue, describeImportMode(manifest.mode, copy));
    setText(elements.guidesLocalizationLatestAssetValue, describeLatestAsset(manifest, copy));
    setText(elements.guidesLocalizationStorageKeyValue, HUB_COPY_OVERRIDE_STORAGE_KEY);
    const output = elements.guidesLocalizationOutput;
    if (output && (!output.textContent?.trim() || output.dataset.localizationOutputKind === "idle")) {
        output.textContent =
            languageKeys.length > 0 || defaultLayerEnabled ? copy.guides.localizationOutputReady : copy.guides.localizationOutputEmpty;
        output.dataset.localizationOutputKind = "idle";
    }
    setText(elements.guidesLocalizationHint, copy.guides.localizationOutputHint);
}
function setGuidesLocalizationOutput(elements, setOperationOutput, value, kind = "status") {
    if (elements.guidesLocalizationOutput) {
        elements.guidesLocalizationOutput.textContent = value;
        elements.guidesLocalizationOutput.dataset.localizationOutputKind = kind;
    }
    setOperationOutput(value);
}
export async function importHubLocalizationRegistry({ file, copy, onDidChange, setOutput, }) {
    if (!file) {
        return;
    }
    try {
        const result = importHubCopyPayload(JSON.parse(await file.text()));
        setOutput(result.mode === "pack" ? copy.dynamic.localizationImportedPack : copy.dynamic.localizationImportedRegistry, "status");
        onDidChange();
    }
    catch {
        setOutput(copy.dynamic.localizationInvalid, "error");
    }
}
export function exportHubLocalizationRegistry({ copy, setOutput }) {
    downloadHubJson("kyuubiki-hub-copy-overrides.json", loadHubCopyOverrideRegistry());
    setOutput(copy.dynamic.localizationExported, "status");
}
export function clearHubLocalizationRegistry({ copy, onDidChange, setOutput }) {
    clearHubCopyOverrideRegistry();
    onDidChange?.();
    setOutput(copy.dynamic.localizationCleared, "status");
}
export function bindHubLocalizationPanel(params) {
    const { elements, hubCopy, rerenderLocalizedHubShell, setOperationOutput } = params;
    const setOutput = (value, kind = "status") => setGuidesLocalizationOutput(elements, setOperationOutput, value, kind);
    elements.guidesLocalizationImport?.addEventListener("click", () => {
        elements.guidesLocalizationImportInput?.click();
    });
    elements.guidesLocalizationExport?.addEventListener("click", () => {
        exportHubLocalizationRegistry({
            copy: hubCopy(),
            setOutput,
        });
    });
    elements.guidesLocalizationClear?.addEventListener("click", () => {
        clearHubLocalizationRegistry({
            copy: hubCopy(),
            onDidChange: rerenderLocalizedHubShell,
            setOutput,
        });
    });
    elements.guidesLocalizationImportInput?.addEventListener("change", async (event) => {
        const input = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null;
        const file = input?.files?.[0];
        try {
            await importHubLocalizationRegistry({
                file,
                copy: hubCopy(),
                onDidChange: rerenderLocalizedHubShell,
                setOutput,
            });
        }
        finally {
            if (input) {
                input.value = "";
            }
        }
    });
}
