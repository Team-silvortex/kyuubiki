export function validateCheckpointIntent(value) {
    const entry = value;
    const fields = ["key", "scope", "operation", "parentId", "requestId", "createdAt"];
    if (!entry || typeof entry !== "object" || Object.keys(entry).some(key => !fields.includes(key))
        || !hash(entry.key) || !hash(entry.scope) || !["model", "version"].includes(entry.operation)
        || typeof entry.parentId !== "string" || !entry.parentId || new TextEncoder().encode(entry.parentId).length > 128
        || !requestId(entry.requestId) || !Number.isSafeInteger(entry.createdAt) || entry.createdAt < 0 || entry.createdAt > 8.64e15) {
        throw new Error("checkpoint:journal_corrupt");
    }
    return entry;
}
const hash = (value) => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
const requestId = (value) => typeof value === "string" && /^[A-Za-z0-9_-]{16,128}$/.test(value);
const protocol = "kyuubiki:checkpoint-recovery/v1";
const frameOrigin = "http://127.0.0.1:3000";
const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
const commands = { list: "list_checkpoint_intents", reserve: "reserve_checkpoint_intent", remove: "remove_checkpoint_intent" };
const codes = new Set(["checkpoint:journal_corrupt", "checkpoint:journal_busy", "checkpoint:journal_unsafe_path", "checkpoint:recovery_capacity_reached"]);
function validateResult(action, result, intent) {
    if (action === "list") {
        if (!Array.isArray(result) || result.length > 64)
            throw new Error("checkpoint:journal_corrupt");
        const entries = result.map(validateCheckpointIntent);
        if (new Set(entries.map(entry => entry.key)).size !== entries.length)
            throw new Error("checkpoint:journal_corrupt");
    }
    else if (action === "reserve") {
        const entry = validateCheckpointIntent(result?.intent);
        if (typeof result?.isNew !== "boolean" || (intent && (entry.key !== intent.key || entry.scope !== intent.scope
            || entry.operation !== intent.operation || entry.parentId !== intent.parentId)))
            throw new Error("checkpoint:journal_corrupt");
    }
    else if (result !== null)
        throw new Error("checkpoint:journal_corrupt");
    return result;
}
/** Dedicated metadata-only IPC, never an arbitrary command or filesystem bridge. */
export function installWorkbenchCheckpointHost(frame, invoke, host = window) {
    let disposed = false;
    let queued = 0;
    let tail = Promise.resolve();
    const receive = (event) => {
        if (disposed || event.source !== frame.contentWindow || event.origin !== frameOrigin)
            return;
        const data = event.data;
        if (data?.type !== protocol || typeof data.id !== "string" || !/^[a-zA-Z0-9-]{1,80}$/.test(data.id)
            || typeof data.action !== "string" || !Object.hasOwn(commands, data.action) || Object.keys(data).some(key => !["type", "id", "action", "intent", "key", "requestId"].includes(key)))
            return;
        const action = data.action;
        try {
            if (action === "reserve")
                validateCheckpointIntent(data.intent);
            if (action === "remove" && (!hash(data.key) || !requestId(data.requestId)))
                return;
            if (action !== "reserve" && data.intent !== undefined)
                return;
            if (action !== "remove" && (data.key !== undefined || data.requestId !== undefined))
                return;
        }
        catch {
            return;
        }
        const source = event.source;
        const reply = (payload) => {
            try {
                if (!disposed && source === frame.contentWindow)
                    source.postMessage({ type: `${protocol}:result`, id: data.id, ...payload }, frameOrigin);
            }
            catch { /* The frame can close while its durable reservation completes. */ }
        };
        if (queued >= 8) {
            reply({ ok: false, error: "checkpoint:journal_busy" });
            return;
        }
        queued++;
        tail = tail.then(async () => {
            if (disposed)
                return;
            try {
                const payload = action === "reserve" ? { payload: data.intent }
                    : action === "remove" ? { key: data.key, requestId: data.requestId } : {};
                const result = validateResult(action, await invoke(commands[action], payload), data.intent);
                reply({ ok: true, result });
            }
            catch (error) {
                const code = String(error).replace(/^Error: /, "");
                reply({ ok: false, error: codes.has(code) ? code : "checkpoint:journal_unavailable" });
            }
        }).finally(() => { queued--; });
    };
    host.addEventListener("message", receive);
    return () => { disposed = true; host.removeEventListener("message", receive); };
}
export function createWorkbenchCheckpointClient(host = window, timeoutMs = 4000) {
    if (host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1")
        return null;
    let origin = null;
    let disposed = false;
    const pending = new Map();
    const receive = (event) => {
        if (event.source !== host.parent || !shellOrigins.has(event.origin) || (origin && event.origin !== origin))
            return;
        const data = event.data;
        if (data?.type !== `${protocol}:result` || typeof data.id !== "string")
            return;
        const entry = pending.get(data.id);
        if (!entry)
            return;
        pending.delete(data.id);
        host.clearTimeout(entry.timer);
        try {
            if (data.ok !== true)
                throw new Error(codes.has(data.error) ? data.error : "checkpoint:journal_unavailable");
            const result = validateResult(entry.action, data.result, entry.intent);
            origin = event.origin;
            entry.resolve(result);
        }
        catch (error) {
            entry.reject(error);
        }
    };
    host.addEventListener("message", receive);
    function request(action, payload = {}) {
        if (disposed || pending.size >= 8 || (action !== "list" && !origin))
            return Promise.reject(new Error("checkpoint:journal_unavailable"));
        return new Promise((resolve, reject) => {
            const id = host.crypto.randomUUID();
            const timer = host.setTimeout(() => { pending.delete(id); reject(new Error("checkpoint:journal_unavailable")); }, timeoutMs);
            pending.set(id, { action, intent: payload.intent, timer, resolve, reject });
            try {
                host.parent.postMessage({ type: protocol, id, action, ...payload }, origin ?? "*");
            }
            catch (error) {
                host.clearTimeout(timer);
                pending.delete(id);
                reject(error);
            }
        });
    }
    return {
        list: () => request("list"),
        async reserve(intent) {
            validateCheckpointIntent(intent);
            // Discovery sends no metadata. Mutations target only the verified parent.
            if (!origin)
                await request("list");
            return request("reserve", { intent });
        },
        async remove(key, id) {
            if (!hash(key) || !requestId(id))
                throw new Error("checkpoint:invalid_recovery_key");
            if (!origin)
                await request("list");
            await request("remove", { key, requestId: id });
        },
        dispose() {
            disposed = true;
            host.removeEventListener("message", receive);
            for (const entry of pending.values()) {
                host.clearTimeout(entry.timer);
                entry.reject(new Error("checkpoint:journal_unavailable"));
            }
            pending.clear();
        },
    };
}
