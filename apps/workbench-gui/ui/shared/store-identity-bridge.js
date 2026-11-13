import { storeAccountPortalUrl } from "./store-account-bridge.js";
const protocol = "kyuubiki:store-identity/v1";
const frameOrigin = "http://127.0.0.1:3000";
const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
const states = new Set(["signed_out", "preparing", "awaiting_browser", "exchanging", "signed_in", "expired", "refreshing", "error"]);
export function validIdentityConfig(value) {
    if (!value || typeof value !== "object")
        return false;
    const config = value;
    if (Object.keys(config).some(key => !["issuer", "client_id", "callback_port", "ca_pem", "ca_sha256"].includes(key)))
        return false;
    try {
        if (new URL(storeAccountPortalUrl(config.issuer, "en")).origin !== config.issuer)
            return false;
    }
    catch {
        return false;
    }
    return typeof config.client_id === "string" && /^[\x21-\x7e]{1,256}$/.test(config.client_id)
        && Number.isInteger(config.callback_port) && config.callback_port >= 1024 && config.callback_port <= 65535
        && typeof config.ca_pem === "string" && config.ca_pem.length <= 16384
        && typeof config.ca_sha256 === "string" && ((!config.ca_pem && !config.ca_sha256)
        || (!!config.ca_pem && /^[0-9a-f]{64}$/i.test(config.ca_sha256) && !config.ca_pem.includes("PRIVATE KEY")));
}
function validRequest(value) {
    if (!value || typeof value !== "object")
        return false;
    const request = value;
    return request.action === "begin" ? Object.keys(request).length === 2 && validIdentityConfig(request.config)
        : Object.keys(request).length === 1 && ["status", "cancel", "logout", "refresh"].includes(request.action);
}
function publicStatus(value) {
    if (!value || typeof value !== "object")
        return null;
    const view = value;
    const text = (v, max) => v === null || (typeof v === "string" && v.length <= max && !/[\x00-\x1f]/.test(v));
    if (!states.has(view.status) || !text(view.issuer, 2048) || !text(view.subject, 256) || !text(view.display_name, 512)
        || !text(view.error, 64) || typeof view.can_refresh !== "boolean"
        || (view.expires_at !== null && (!Number.isSafeInteger(view.expires_at) || view.expires_at < 0)))
        return null;
    // Explicit projection: future native fields can never accidentally forward credentials.
    return { status: view.status, issuer: view.issuer, subject: view.subject, display_name: view.display_name,
        expires_at: view.expires_at, can_refresh: view.can_refresh, error: view.error };
}
export function installStoreIdentityHost(frame, invoke, host = window) {
    let epoch = 0, disposed = false;
    const pending = new Set();
    const seen = new Set();
    const receive = (event) => {
        if (disposed || frame.isConnected === false || event.origin !== frameOrigin || event.source !== frame.contentWindow)
            return;
        const data = event.data;
        if (!data || data.type !== protocol || typeof data.id !== "string" || !/^[a-zA-Z0-9-]{1,80}$/.test(data.id)
            || Object.keys(data).some(key => !["type", "id", "request"].includes(key)) || !validRequest(data.request)
            || seen.has(data.id))
            return;
        const generation = epoch, source = event.source;
        const reply = (status) => {
            if (!disposed && epoch === generation && source === frame.contentWindow && frame.isConnected !== false) {
                source.postMessage({ type: `${protocol}:result`, id: data.id, ok: !!status, status }, frameOrigin);
            }
        };
        if (pending.has(data.request.action)) {
            reply(null);
            return;
        }
        seen.add(data.id);
        if (seen.size > 128)
            seen.delete(seen.values().next().value);
        pending.add(data.request.action);
        void Promise.resolve().then(() => {
            if (disposed || epoch !== generation)
                throw new Error("Page changed");
            return invoke("store_identity", { request: data.request });
        }).then(value => reply(publicStatus(value)), () => reply(null)).finally(() => pending.delete(data.request.action));
    };
    const invalidate = () => {
        epoch++;
        seen.clear();
        // Cancel a browser transaction, not an already authenticated native session.
        void invoke("store_identity", { request: { action: "cancel" } }).catch(() => { });
    };
    const stop = () => {
        if (disposed)
            return;
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
export function requestStoreIdentity(request, host = window, timeoutMs = 45000) {
    return new Promise((resolve, reject) => {
        if (!validRequest(request) || host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1") {
            reject(new Error("Desktop account connection unavailable"));
            return;
        }
        const id = host.crypto.randomUUID();
        const cleanup = () => { host.clearTimeout(timer); host.removeEventListener("message", receive); };
        const receive = (event) => {
            if (event.source !== host.parent || !shellOrigins.has(event.origin)
                || event.data?.type !== `${protocol}:result` || event.data.id !== id)
                return;
            cleanup();
            const status = event.data.ok === true ? publicStatus(event.data.status) : null;
            if (status)
                resolve(status);
            else
                reject(new Error("Account operation failed"));
        };
        const timer = host.setTimeout(() => { cleanup(); reject(new Error("Account operation timed out; check session status before retrying")); }, timeoutMs);
        host.addEventListener("message", receive);
        try {
            host.parent.postMessage({ type: protocol, id, request }, "*");
        }
        catch {
            cleanup();
            reject(new Error("Account operation unavailable"));
        }
    });
}
