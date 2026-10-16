const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
export const validJournalSession = (value) => typeof value === "string"
    && /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/.test(value);
/** UI-only document fencing. The session is neither a credential nor a disk identity. */
export function createJournalSessionClient(host, timeoutMs, protocol, namespace, codes, validate) {
    let origin = null, session = null;
    let disposed = false, suspended = false, generation = Symbol(), outstanding = 0;
    let connecting = null;
    const unavailable = () => new Error(`${namespace}:journal_unavailable`);
    const pending = new Map();
    const receive = (event) => {
        if (event.source !== host.parent || !shellOrigins.has(event.origin) || (origin && origin !== event.origin))
            return;
        const data = event.data;
        if (data?.type !== `${protocol}:result` || typeof data.id !== "string")
            return;
        const entry = pending.get(data.id);
        if (!entry || (entry.session !== null && data.session !== entry.session))
            return;
        pending.delete(data.id);
        host.clearTimeout(entry.timer);
        try {
            if (data.ok !== true) {
                if (entry.session && data.expired === true && data.error === `${namespace}:journal_unavailable`
                    && session === entry.session)
                    session = null;
                throw new Error(codes.has(data.error) ? data.error : `${namespace}:journal_unavailable`);
            }
            if (entry.request.action === "connect") {
                if (!validJournalSession(data.session) || data.result !== null)
                    throw new Error(`${namespace}:journal_corrupt`);
                origin = event.origin;
                session = data.session;
                entry.resolve(null);
            }
            else
                entry.resolve(validate(entry.request, data.result));
        }
        catch (error) {
            entry.reject(error);
        }
    };
    const retire = () => {
        generation = Symbol();
        session = null;
        connecting = null;
        for (const entry of pending.values()) {
            host.clearTimeout(entry.timer);
            entry.reject(unavailable());
        }
        pending.clear();
    };
    const hide = () => { suspended = true; retire(); };
    const show = () => { suspended = false; };
    const send = (request, token) => {
        if (disposed || suspended || pending.size >= 8)
            return Promise.reject(unavailable());
        return new Promise((resolve, reject) => {
            const id = host.crypto.randomUUID();
            const timer = host.setTimeout(() => { pending.delete(id); reject(unavailable()); }, timeoutMs);
            pending.set(id, { request, session: token, timer, resolve, reject });
            try {
                host.parent.postMessage({ type: protocol, id, ...request, ...(token ? { session: token } : {}) }, origin ?? "*");
            }
            catch (error) {
                pending.delete(id);
                host.clearTimeout(timer);
                reject(error);
            }
        });
    };
    const connect = () => {
        if (!connecting) {
            const attempt = send({ action: "connect" }, null);
            connecting = attempt;
            const clear = () => { if (connecting === attempt)
                connecting = null; };
            void attempt.then(clear, clear);
        }
        return connecting;
    };
    host.addEventListener("message", receive);
    host.addEventListener("pagehide", hide);
    host.addEventListener("pageshow", show);
    return {
        async request(payload, assertCurrent) {
            if (disposed || suspended || outstanding >= 8)
                throw unavailable();
            const capturedGeneration = generation;
            const check = () => {
                if (disposed || suspended || capturedGeneration !== generation)
                    throw unavailable();
                assertCurrent?.();
            };
            outstanding++;
            try {
                check();
                if (!session)
                    await connect();
                check();
                if (!session)
                    throw unavailable();
                // Session expiry rejects this operation. Only a fresh caller may reconnect.
                const result = await send(payload, session);
                // A reply can resolve before lifecycle/caller changes retire its continuation.
                check();
                return result;
            }
            finally {
                outstanding--;
            }
        },
        dispose() {
            disposed = true;
            retire();
            host.removeEventListener("message", receive);
            host.removeEventListener("pagehide", hide);
            host.removeEventListener("pageshow", show);
        },
    };
}
