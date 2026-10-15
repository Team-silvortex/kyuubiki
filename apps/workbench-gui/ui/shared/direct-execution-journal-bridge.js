const kinds = new Set(["axial_bar_1d", "thermal_bar_1d", "heat_bar_1d", "beam_1d", "thermal_beam_1d", "torsion_1d",
    "spring_1d", "spring_2d", "spring_3d", "truss_2d", "truss_3d", "thermal_truss_2d", "thermal_truss_3d", "frame_2d", "thermal_frame_2d",
    "plane_triangle_2d", "plane_quad_2d", "heat_plane_triangle_2d", "heat_plane_quad_2d", "thermal_plane_triangle_2d",
    "thermal_plane_quad_2d", "electrostatic_plane_triangle_2d", "electrostatic_plane_quad_2d"]);
const fields = ["scope", "endpoint", "study_kind", "execution_id", "created_at", "store_id"];
export function validateDirectExecutionIntent(value) {
    const v = value;
    const endpoint = typeof v?.endpoint === "string" ? /^([A-Za-z0-9][A-Za-z0-9.-]*):([0-9]+)$/.exec(v.endpoint) : null;
    if (!v || typeof v !== "object" || Object.keys(v).some(key => !fields.includes(key))
        || typeof v.scope !== "string" || !/^[a-f0-9]{64}$/.test(v.scope)
        || typeof v.execution_id !== "string" || !/^[a-f0-9]{32}$/.test(v.execution_id)
        || (v.store_id !== undefined && (typeof v.store_id !== "string" || !/^[a-f0-9]{32}$/.test(v.store_id)))
        || !endpoint || v.endpoint.length > 255 || endpoint[1] === "0.0.0.0" || +endpoint[2] < 1 || +endpoint[2] > 65535 || String(+endpoint[2]) !== endpoint[2]
        || !kinds.has(v.study_kind) || !Number.isSafeInteger(v.created_at) || v.created_at < 0 || v.created_at > 8.64e15)
        throw new Error("direct:journal_corrupt");
    return v;
}
export function sameDirectExecutionIntent(a, b) { return fields.every(key => a[key] === b[key]); }
export function validateDirectExecutionList(value) {
    if (!Array.isArray(value) || value.length > 64)
        throw new Error("direct:journal_corrupt");
    const entries = value.map(validateDirectExecutionIntent);
    if (new Set(entries.map(e => `${e.scope}/${e.endpoint}`)).size !== entries.length
        || new Set(entries.map(e => e.execution_id)).size !== entries.length)
        throw new Error("direct:journal_corrupt");
    return entries;
}
const protocol = "kyuubiki:direct-execution-journal/v1";
const frameOrigin = "http://127.0.0.1:3000";
const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
const codes = new Set(["direct:journal_corrupt", "direct:journal_busy", "direct:journal_full", "direct:journal_unsafe_path"]);
function validateResult(request, result) {
    if (request.action === "list")
        validateDirectExecutionList(result);
    else if (request.action === "remove") {
        if (result !== null)
            throw new Error("direct:journal_corrupt");
    }
    else {
        const entry = validateDirectExecutionIntent(result?.intent);
        if (typeof result.is_new !== "boolean" || entry.scope !== request.intent.scope || entry.endpoint !== request.intent.endpoint
            || (result.is_new && !sameDirectExecutionIntent(entry, request.intent)))
            throw new Error("direct:journal_corrupt");
    }
    return result;
}
export function installDirectExecutionJournalHost(frame, invoke, host = window) {
    let disposed = false, queued = 0;
    let tail = Promise.resolve();
    const receive = (event) => {
        const data = event.data;
        if (disposed || event.source !== frame.contentWindow || event.origin !== frameOrigin || data?.type !== protocol
            || typeof data.id !== "string" || !/^[A-Za-z0-9-]{1,80}$/.test(data.id)
            || !["list", "reserve", "remove"].includes(data.action) || Object.keys(data).some(key => !["type", "id", "action", "intent"].includes(key)))
            return;
        let request;
        try {
            if (data.action === "list") {
                if (data.intent !== undefined)
                    return;
                request = { action: "list" };
            }
            else
                request = { action: data.action, intent: { ...validateDirectExecutionIntent(data.intent) } };
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
            catch { /* Closing the frame does not roll back a committed reservation. */ }
        };
        if (queued >= 8) {
            reply({ ok: false, error: "direct:journal_busy" });
            return;
        }
        queued++;
        tail = tail.then(async () => {
            if (disposed)
                return;
            try {
                reply({ ok: true, result: validateResult(request, await invoke("direct_execution_journal", { payload: request })) });
            }
            catch (error) {
                const code = String(error).replace(/^Error: /, "");
                reply({ ok: false, error: codes.has(code) ? code : "direct:journal_unavailable" });
            }
        }).finally(() => { queued--; });
    };
    host.addEventListener("message", receive);
    return () => { disposed = true; host.removeEventListener("message", receive); };
}
export function createDirectExecutionJournalClient(host = window, timeoutMs = 4000) {
    if (host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1")
        return null;
    let origin = null, disposed = false;
    const pending = new Map();
    const receive = (event) => {
        if (event.source !== host.parent || !shellOrigins.has(event.origin) || (origin && origin !== event.origin))
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
                throw new Error(codes.has(data.error) ? data.error : "direct:journal_unavailable");
            const result = validateResult(entry.request, data.result);
            origin = event.origin;
            entry.resolve(result);
        }
        catch (error) {
            entry.reject(error);
        }
    };
    host.addEventListener("message", receive);
    const send = (request) => {
        if (disposed || pending.size >= 8 || (request.action !== "list" && !origin))
            return Promise.reject(new Error("direct:journal_unavailable"));
        return new Promise((resolve, reject) => {
            const id = host.crypto.randomUUID();
            const timer = host.setTimeout(() => { pending.delete(id); reject(new Error("direct:journal_unavailable")); }, timeoutMs);
            pending.set(id, { request, timer, resolve, reject });
            try {
                host.parent.postMessage({ type: protocol, id, ...request }, origin ?? "*");
            }
            catch (error) {
                pending.delete(id);
                host.clearTimeout(timer);
                reject(error);
            }
        });
    };
    const mutate = async (action, value, assertCurrent) => {
        const intent = { ...validateDirectExecutionIntent(value) };
        assertCurrent?.();
        if (!origin)
            await send({ action: "list" });
        assertCurrent?.();
        return send({ action, intent });
    };
    return { list: () => send({ action: "list" }), reserve: value => mutate("reserve", value), remove: (value, assertCurrent) => mutate("remove", value, assertCurrent),
        dispose() { disposed = true; host.removeEventListener("message", receive); for (const entry of pending.values()) {
            host.clearTimeout(entry.timer);
            entry.reject(new Error("direct:journal_unavailable"));
        } pending.clear(); } };
}
