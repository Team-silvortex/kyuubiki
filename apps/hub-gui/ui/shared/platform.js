export const DESKTOP_PLATFORMS = ["macos", "linux", "windows"];
export const DESKTOP_PLATFORM_LABELS = {
    all: "All platforms",
    macos: "macOS",
    linux: "Linux",
    windows: "Windows",
};
export function detectDesktopPlatform() {
    const navigatorWithUaData = globalThis.navigator;
    const platformText = [
        navigatorWithUaData?.userAgentData?.platform,
        navigatorWithUaData?.platform,
        navigatorWithUaData?.userAgent,
    ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
    if (platformText.includes("mac"))
        return "macos";
    if (platformText.includes("win"))
        return "windows";
    return "linux";
}
export function normalizeDesktopPlatform(value, fallback = detectDesktopPlatform(), options = {}) {
    if (options.allowAll && value === "all")
        return "all";
    return isDesktopPlatform(value) ? value : fallback;
}
export function desktopPlatformLabel(value) {
    return (DESKTOP_PLATFORM_LABELS[normalizeDesktopPlatform(value, "linux", { allowAll: true })] ||
        DESKTOP_PLATFORM_LABELS.linux);
}
export function desktopReleaseRoot(value) {
    return `dist/${normalizeDesktopPlatform(value)}`;
}
export function desktopReleaseRootPattern() {
    return "dist/{platform}";
}
export function desktopPlatformContextLabel(label, value, context = "host") {
    const suffix = String(context || "").trim();
    return suffix
        ? `${label} · ${desktopPlatformLabel(value)} ${suffix}`
        : `${label} · ${desktopPlatformLabel(value)}`;
}
export function desktopPlatformOptions(options = {}) {
    const entries = options.includeAll
        ? ["all", ...DESKTOP_PLATFORMS]
        : [...DESKTOP_PLATFORMS];
    return entries.map((value) => ({
        value,
        label: DESKTOP_PLATFORM_LABELS[value],
    }));
}
export function populateDesktopPlatformSelect(select, options = {}) {
    if (!select)
        return;
    const currentValue = String(select.value || "").trim();
    const normalizedCurrent = normalizeDesktopPlatform(currentValue, options.fallback || detectDesktopPlatform(), { allowAll: options.includeAll === true });
    select.innerHTML = "";
    for (const option of desktopPlatformOptions({ includeAll: options.includeAll === true })) {
        const element = document.createElement("option");
        element.value = option.value;
        element.textContent = option.label;
        if (option.value === normalizedCurrent) {
            element.selected = true;
        }
        select.appendChild(element);
    }
}
export function syncDesktopReleaseTargetInput(input, platform) {
    if (!input)
        return;
    const nextValue = desktopReleaseRoot(platform);
    const currentValue = String(input.value || "").trim();
    const placeholder = String(input.placeholder || "").trim();
    input.placeholder = nextValue;
    if (!currentValue ||
        currentValue === placeholder ||
        currentValue.startsWith("dist/")) {
        input.value = nextValue;
    }
}
function isDesktopPlatform(value) {
    return DESKTOP_PLATFORMS.includes(value);
}
