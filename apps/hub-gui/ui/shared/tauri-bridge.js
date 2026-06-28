export async function invokeTauri(command, payload = {}) {
    const tauri = window.__TAURI__;
    if (!tauri?.core?.invoke) {
        throw new Error("Tauri runtime is not available. Run this UI inside the desktop shell.");
    }
    return tauri.core.invoke(command, payload);
}
export async function listenTauri(eventName, handler) {
    const tauri = window.__TAURI__;
    if (!tauri?.event?.listen) {
        throw new Error("Tauri event API is not available.");
    }
    return tauri.event.listen(eventName, handler);
}
export async function loadDesktopBrand() {
    try {
        const response = await fetch("./assets/brand.json");
        if (!response.ok) {
            return null;
        }
        return response.json();
    }
    catch (_error) {
        return null;
    }
}
export function normalizeDesktopLanguage(value) {
    return typeof value === "string" && value.trim() ? value.trim() : "en";
}
function readLanguagePayload(payload) {
    return normalizeDesktopLanguage(payload?.language);
}
export async function loadDesktopLanguagePreference() {
    try {
        const payload = await invokeTauri("get_global_language_preference");
        return readLanguagePayload(payload);
    }
    catch (_error) {
        return "en";
    }
}
export async function saveDesktopLanguagePreference(language) {
    const payload = await invokeTauri("set_global_language_preference", {
        payload: { language: normalizeDesktopLanguage(language) },
    });
    return readLanguagePayload(payload);
}
export function watchDesktopLanguagePreference({ getCurrentLanguage, onChange, intervalMs = 2000, } = {}) {
    if (typeof onChange !== "function") {
        return () => { };
    }
    let disposed = false;
    let pending = false;
    const readCurrentLanguage = typeof getCurrentLanguage === "function" ? getCurrentLanguage : () => undefined;
    const checkLanguage = async () => {
        if (disposed || pending) {
            return;
        }
        pending = true;
        try {
            const nextLanguage = await loadDesktopLanguagePreference();
            if (nextLanguage && nextLanguage !== normalizeDesktopLanguage(readCurrentLanguage())) {
                await onChange(nextLanguage);
            }
        }
        finally {
            pending = false;
        }
    };
    const onVisibilityChange = () => {
        if (document.visibilityState !== "hidden") {
            void checkLanguage();
        }
    };
    window.addEventListener("focus", checkLanguage);
    window.addEventListener("pageshow", checkLanguage);
    document.addEventListener("visibilitychange", onVisibilityChange);
    const timer = window.setInterval(() => {
        if (document.visibilityState !== "hidden") {
            void checkLanguage();
        }
    }, Math.max(500, intervalMs));
    return () => {
        disposed = true;
        window.clearInterval(timer);
        window.removeEventListener("focus", checkLanguage);
        window.removeEventListener("pageshow", checkLanguage);
        document.removeEventListener("visibilitychange", onVisibilityChange);
    };
}
export function setText(target, value) {
    const element = typeof target === "string" ? document.getElementById(target) : target;
    if (element && value !== undefined && value !== null) {
        element.textContent = String(value);
    }
}
const DESKTOP_STATE_VARIANTS = [
    "desktop-shell-state--active",
    "desktop-shell-state--healthy",
    "desktop-shell-state--warning",
    "desktop-shell-state--danger",
    "desktop-shell-state--idle",
];
const DESKTOP_STATE_KEYWORDS = {
    danger: [
        "error",
        "failed",
        "failure",
        "down",
        "stopped",
        "unhealthy",
        "invalid",
        "denied",
        "panic",
        "refused",
        "crash",
        "dead",
    ],
    warning: [
        "warning",
        "missing",
        "pending",
        "starting",
        "restarting",
        "degraded",
        "partial",
        "timeout",
        "attention",
        "queued",
        "recent",
    ],
    healthy: ["ready", "healthy", "running", "stable", "connected", "ok", "passed", "online"],
    active: [
        "active",
        "enabled",
        "selected",
        "tauri",
        "local",
        "cloud",
        "distributed",
        "direct mesh",
        "direct-mesh",
        "orchestrated_gui",
        "sqlite",
        "postgres",
        "frontend",
    ],
    idle: ["idle", "unknown", "none", "--", "n/a"],
};
function includesAnyKeyword(text, keywords) {
    return keywords.some((keyword) => text.includes(keyword));
}
export function classifyDesktopState(value, options = {}) {
    const fallback = options.fallback || "idle";
    const kind = options.kind || "auto";
    const text = String(value || "")
        .trim()
        .toLowerCase();
    if (!text) {
        return fallback;
    }
    if (kind === "activity") {
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.danger))
            return "danger";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.warning))
            return "warning";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.active))
            return "active";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.healthy))
            return "healthy";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.idle))
            return "idle";
        return fallback;
    }
    if (kind === "health") {
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.danger))
            return "danger";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.warning))
            return "warning";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.healthy))
            return "healthy";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.active))
            return "active";
        if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.idle))
            return "idle";
        return fallback;
    }
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.danger))
        return "danger";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.warning))
        return "warning";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.healthy))
        return "healthy";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.active))
        return "active";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.idle))
        return "idle";
    return fallback;
}
export function applyDesktopState(element, value, options = {}) {
    if (!element) {
        return fallbackStateResult(value, options);
    }
    const resolvedValue = value ?? element.textContent ?? "";
    const state = classifyDesktopState(resolvedValue, options);
    element.classList.add("desktop-shell-state");
    DESKTOP_STATE_VARIANTS.forEach((className) => {
        element.classList.remove(className);
    });
    element.classList.add(`desktop-shell-state--${state}`);
    if (value !== undefined) {
        element.textContent = String(value);
    }
    if (element instanceof HTMLElement) {
        element.dataset.desktopStateResolved = state;
    }
    return state;
}
function fallbackStateResult(value, options) {
    return classifyDesktopState(value, options);
}
export function syncDesktopStates(root = document) {
    root.querySelectorAll("[data-desktop-state]").forEach((element) => {
        const kind = element instanceof HTMLElement ? element.dataset.desktopState || "auto" : "auto";
        const value = element instanceof HTMLElement
            ? element.dataset.stateValue || element.textContent || ""
            : element.textContent || "";
        applyDesktopState(element, value, { kind: kind });
    });
}
