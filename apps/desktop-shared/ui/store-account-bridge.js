const protocol = "kyuubiki:store-account/v1";
const frameOrigin = "http://127.0.0.1:3000";
const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
export function storeAccountPortalUrl(origin, language) {
    if (typeof origin !== "string" || origin.length > 2048 || !/^https:\/\/[a-zA-Z0-9.:[\]-]+\/?$/.test(origin)) {
        throw new Error("Use a bare HTTPS account origin without credentials, path, query or fragment");
    }
    const parsed = new URL(origin);
    if (parsed.protocol !== "https:" || !parsed.hostname || parsed.username || parsed.password || parsed.search || parsed.hash
        || /:\/?$/.test(origin) || parsed.pathname !== "/" || parsed.port === "0") {
        throw new Error("Use a bare HTTPS account origin without credentials, path, query or fragment");
    }
    return `${parsed.origin}/account?lang=${language.startsWith("zh") ? "zh" : "en"}`;
}
/** A narrow browser-open capability, not arbitrary native IPC or an authentication result. */
export function installStoreAccountHost(frame, invoke, host = window) {
    let disposed = false;
    let busy = false;
    let lastId = "";
    let generation = 0;
    const invalidate = () => { generation++; lastId = ""; };
    const receive = (event) => {
        if (disposed || frame.isConnected === false || event.source !== frame.contentWindow || event.origin !== frameOrigin)
            return;
        const data = event.data;
        if (!data || data.type !== protocol || typeof data.id !== "string" || !/^[a-zA-Z0-9-]{1,80}$/.test(data.id)
            || typeof data.language !== "string" || data.language.length > 32
            || Object.keys(data).some(key => !["type", "id", "origin", "language"].includes(key)))
            return;
        let url;
        try {
            url = new URL(storeAccountPortalUrl(data.origin, data.language));
        }
        catch {
            return;
        }
        if (lastId === data.id)
            return;
        const id = data.id;
        const epoch = generation;
        const source = event.source;
        const reply = (ok) => {
            try {
                if (!disposed && epoch === generation && frame.isConnected !== false && source === frame.contentWindow) {
                    source.postMessage({ type: `${protocol}:result`, id, ok }, frameOrigin);
                }
            }
            catch { /* The embedded page may have closed. */ }
        };
        if (busy) {
            reply(false);
            return;
        }
        busy = true;
        lastId = id;
        void Promise.resolve().then(() => {
            if (disposed || epoch !== generation || frame.isConnected === false || source !== frame.contentWindow)
                throw new Error("Account bridge closed");
            return invoke("open_store_account_portal", { origin: url.origin, language: url.searchParams.get("lang") });
        }).then(() => reply(true), () => reply(false)).finally(() => { busy = false; });
    };
    const stop = () => {
        disposed = true;
        invalidate();
        host.removeEventListener("message", receive);
        host.removeEventListener("pagehide", stop);
        frame.removeEventListener("load", invalidate);
    };
    host.addEventListener("message", receive);
    host.addEventListener("pagehide", stop);
    frame.addEventListener("load", invalidate);
    return stop;
}
export function requestStoreAccountPortal(origin, language, host = window, timeoutMs = 8000) {
    return new Promise((resolve, reject) => {
        const url = new URL(storeAccountPortalUrl(origin, language));
        if (host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1") {
            reject(new Error("Account opener requires the desktop shell"));
            return;
        }
        const id = host.crypto.randomUUID();
        const cleanup = () => { host.clearTimeout(timer); host.removeEventListener("message", receive); };
        const receive = (event) => {
            if (event.source !== host.parent || !shellOrigins.has(event.origin)
                || event.data?.type !== `${protocol}:result` || event.data.id !== id)
                return;
            cleanup();
            if (event.data.ok === true)
                resolve();
            else
                reject(new Error("System browser could not be opened"));
        };
        const timer = host.setTimeout(() => { cleanup(); reject(new Error("Account browser request timed out")); }, timeoutMs);
        host.addEventListener("message", receive);
        try {
            // This message contains only a public origin and language, never credentials.
            host.parent.postMessage({ type: protocol, id, origin: url.origin, language: url.searchParams.get("lang") }, "*");
        }
        catch {
            cleanup();
            reject(new Error("Account browser request failed"));
        }
    });
}
