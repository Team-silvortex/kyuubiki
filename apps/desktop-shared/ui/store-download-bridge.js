const protocol = "kyuubiki:store-download/v1";
const frameOrigin = "http://127.0.0.1:3000";
const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
const validSession = (value) => typeof value === "string" && /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/.test(value);
const kinds = ["operator", "workflow_template", "frontend_dsl_template", "language_pack"];
const platforms = ["macos-aarch64", "macos-x86_64", "linux-aarch64", "linux-x86_64", "windows-aarch64", "windows-x86_64", "any"];
const digest = (value) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
const segment = (value) => typeof value === "string" && /^[a-zA-Z0-9_-][a-zA-Z0-9._-]{0,199}$/.test(value)
    && !value.endsWith(".") && !/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(value);
const variantPlatform = (value) => value === "linux-x86_64" || value === "linux-aarch64";
const bounded = (value, max) => typeof value === "string" && value.length <= max && !/[\x00-\x1f\x7f]/.test(value);
export function publicCatalog(value) {
    if (!value || typeof value !== "object")
        return null;
    const page = value;
    try {
        const u = new URL(page.issuer);
        if (u.protocol !== "https:" || u.origin !== page.issuer)
            return null;
    }
    catch {
        return null;
    }
    if (!platforms.includes(page.host_platform) || !bounded(page.catalog_revision, 256) || !page.catalog_revision
        || (page.next_cursor !== null && (!bounded(page.next_cursor, 8192) || !page.next_cursor)) || !Array.isArray(page.items) || page.items.length > 20)
        return null;
    const items = [], ids = new Set();
    for (const item of page.items) {
        if (!item || !kinds.includes(item.kind) || !segment(item.asset_id) || !segment(item.latest_version) || !/^\d/.test(item.latest_version)
            || !bounded(item.title, 512) || !bounded(item.summary, 4096) || !bounded(item.domain, 100)
            || !Array.isArray(item.languages) || item.languages.length > 100 || item.languages.some(l => !bounded(l, 100)))
            return null;
        const id = `${item.kind}/${item.asset_id}`;
        if (ids.has(id))
            return null;
        ids.add(id);
        items.push({ kind: item.kind, asset_id: item.asset_id, latest_version: item.latest_version, title: item.title, summary: item.summary, domain: item.domain, languages: [...item.languages] });
    }
    return { issuer: page.issuer, host_platform: page.host_platform, catalog_revision: page.catalog_revision, items, next_cursor: page.next_cursor };
}
export function validDownloadSelection(value) {
    if (!value || typeof value !== "object")
        return false;
    const s = value;
    return Object.keys(s).length === 6 && Object.keys(s).every(key => ["kind", "asset_id", "version", "platform", "key_id", "key_sha256"].includes(key))
        && kinds.includes(s.kind) && platforms.includes(s.platform) && segment(s.asset_id) && segment(s.version)
        && /^\d/.test(s.version) && segment(s.key_id) && digest(s.key_sha256);
}
function validRequest(value) {
    if (!value || typeof value !== "object")
        return false;
    const r = value;
    if (r.action === "project")
        return validProjectRequest(r);
    if (r.action === "catalog")
        return Object.keys(r).length === 3 && bounded(r.query, 200) && (r.cursor === null || (bounded(r.cursor, 8192) && r.cursor.length > 0));
    if (r.action === "list")
        return Object.keys(r).length === 1;
    if (r.action === "prepare")
        return Object.keys(r).length === 2 && validDownloadSelection(r.selection);
    return ["start", "cancel", "remove", "install", "stage_variant"].includes(r.action) && Object.keys(r).length === 2 && "id" in r && digest(r.id);
}
function pin(value) {
    const p = value;
    return !!p && typeof p === "object" && Object.keys(p).length === 5 && segment(p.package_id) && segment(p.package_version) && /^\d/.test(p.package_version)
        && digest(p.manifest_sha256) && digest(p.entrypoint_sha256) && Array.isArray(p.operators) && p.operators.length > 0 && p.operators.length <= 128
        && new Set(p.operators.map(o => o?.operator_id)).size === p.operators.length
        && p.operators.every(o => o && Object.keys(o).length === 2 && segment(o.operator_id) && ["extract", "transform", "solve"].includes(o.kind));
}
export { pin as validProjectOperatorPin };
function validProjectRequest(r) {
    if (Object.keys(r).length !== (r.platform === undefined ? 4 : 5) || (r.platform !== undefined && !variantPlatform(r.platform))
        || !digest(r.scope) || !bounded(r.project_id, 256) || !r.project_id || r.project_id.trim() !== r.project_id || !r.operation)
        return false;
    const o = r.operation;
    if (o.action === "list")
        return Object.keys(o).length === 1;
    if (!pin(o.package))
        return false;
    if (o.action === "read_deployment")
        return Object.keys(o).length === 3 && Number.isSafeInteger(o.offset) && o.offset >= 0 && o.offset <= 33554432;
    if (o.action === "enable" || o.action === "disable")
        return Object.keys(o).length === 2;
    if (o.action !== "prepare" || Object.keys(o).length !== 6 || !segment(o.operator_id) || !segment(o.task_id))
        return false;
    const record = (v) => !!v && typeof v === "object" && !Array.isArray(v);
    try {
        return record(o.input) && record(o.config) && JSON.stringify([o.input, o.config]).length <= 65536;
    }
    catch {
        return false;
    }
}
export function publicProjectOperators(value) {
    const s = value;
    if (!s || s.schema_version !== "kyuubiki.project-operators/v1" || !digest(s.scope) || !bounded(s.project_id, 256) || !s.project_id || !bounded(s.packages_root, 4096)
        || (s.platform !== undefined && !variantPlatform(s.platform))
        || !Array.isArray(s.installed) || s.installed.length > 64 || s.installed.some(p => !pin(p))
        || !Array.isArray(s.enabled) || s.enabled.length > 64 || s.enabled.some(e => !e || !pin(e.package) || !["ready", "unavailable_or_changed"].includes(e.state))
        || (s.installed_error !== null && safeCode(s.installed_error) !== s.installed_error)
        || (s.task !== null && (typeof s.task !== "object" || s.task.schema_version !== "kyuubiki.operator-task-ir/v1" || JSON.stringify(s.task).length > 131072)))
        return null;
    const d = s.deployment;
    if (d !== undefined && (!d || !pin(d.spec?.package) || !platforms.includes(d.spec.platform) || typeof d.spec.manifest_json !== "string" || d.spec.manifest_json.length > 65536
        || (s.platform !== undefined && d.spec.platform !== s.platform)
        || !Number.isSafeInteger(d.spec.size_bytes) || d.spec.size_bytes <= 0 || d.spec.size_bytes > 33554432 || !Number.isSafeInteger(d.offset) || d.offset < 0
        || !Array.isArray(d.data) || d.data.length > 65536 || d.offset + d.data.length > d.spec.size_bytes || d.data.some(b => !Number.isInteger(b) || b < 0 || b > 255)))
        return null;
    const copy = (p) => ({ package_id: p.package_id, package_version: p.package_version, manifest_sha256: p.manifest_sha256,
        entrypoint_sha256: p.entrypoint_sha256, operators: p.operators.map(o => ({ operator_id: o.operator_id, kind: o.kind })) });
    return { schema_version: s.schema_version, project_id: s.project_id, scope: s.scope, packages_root: s.packages_root, installed: s.installed.map(copy), installed_error: s.installed_error,
        ...(s.platform ? { platform: s.platform } : {}),
        enabled: s.enabled.map(e => ({ package: copy(e.package), state: e.state })), task: s.task,
        ...(d ? { deployment: { spec: { package: copy(d.spec.package), platform: d.spec.platform, manifest_json: d.spec.manifest_json, size_bytes: d.spec.size_bytes }, offset: d.offset, data: [...d.data] } } : {}) };
}
function response(request, value) {
    if (request.action === "project") {
        const s = publicProjectOperators(value);
        return s?.project_id === request.project_id && s.scope === request.scope && s.platform === request.platform ? s : null;
    }
    return request.action === "catalog" ? publicCatalog(value) : publicDownloadViews(value);
}
const states = new Set(["prepared", "authorizing", "downloading", "downloaded", "failed", "cancelled", "cancelling", "interrupted"]);
const safeCode = (value) => typeof value === "string" && /^[a-z_]{1,64}$/.test(value) ? value : "download_operation_failed";
export function publicDownloadViews(value) {
    if (!Array.isArray(value) || value.length > 4)
        return null;
    const result = [];
    const ids = new Set();
    for (const row of value) {
        if (!row || typeof row !== "object")
            return null;
        const v = row;
        const count = (n, max) => typeof n === "number" && Number.isSafeInteger(n) && n >= 0 && n <= max;
        if (!digest(v.id) || ids.has(v.id) || !validDownloadSelection(v.selection) || !states.has(v.state)
            || !count(v.file_count, 32) || v.file_count === 0 || !count(v.total_bytes, 268435456) || !count(v.received_bytes, v.total_bytes)
            || (v.remaining !== null && !count(v.remaining, Number.MAX_SAFE_INTEGER)) || typeof v.metered !== "boolean"
            || typeof v.unlimited !== "boolean" || (v.charged !== null && typeof v.charged !== "boolean")
            || (v.error !== null && safeCode(v.error) !== v.error)
            || (v.installation != null && (!["installing", "installed", "install_failed", "variant_staging", "variant_staged", "variant_stage_failed"].includes(v.installation) || v.state !== "downloaded"))
            || (v.installation?.startsWith("variant_") && (v.selection.kind !== "operator" || !variantPlatform(v.selection.platform)))
            || (v.directory !== null && (typeof v.directory !== "string" || v.directory.length > 4096 || /[\x00-\x1f]/.test(v.directory))))
            return null;
        try {
            const u = new URL(v.issuer);
            if (u.protocol !== "https:" || u.origin !== v.issuer)
                return null;
        }
        catch {
            return null;
        }
        ids.add(v.id);
        // No token, ticket, grant URL or future native field can leak through this projection.
        result.push({ id: v.id, issuer: v.issuer, selection: { ...v.selection }, state: v.state,
            file_count: v.file_count, total_bytes: v.total_bytes, received_bytes: v.received_bytes,
            metered: v.metered, remaining: v.remaining, unlimited: v.unlimited, charged: v.charged, error: v.error, directory: v.directory,
            ...("installation" in v ? { installation: v.installation } : {}) });
    }
    return result;
}
export function installStoreDownloadHost(frame, invoke, host = window) {
    let epoch = 0, disposed = false;
    let session = host.crypto.randomUUID();
    const pending = new Set(), seen = new Set();
    const receive = (event) => {
        if (disposed || frame.isConnected === false || event.origin !== frameOrigin || event.source !== frame.contentWindow)
            return;
        const data = event.data;
        if (!data || data.type !== protocol || typeof data.id !== "string" || !/^[a-zA-Z0-9-]{1,80}$/.test(data.id))
            return;
        const source = event.source;
        if (data.action === "connect" && Object.keys(data).every(key => ["type", "id", "action"].includes(key))) {
            try {
                source.postMessage({ type: `${protocol}:result`, id: data.id, connected: true, session, views: null, error: null }, frameOrigin);
            }
            catch { /* A closing document has no native side effects. */ }
            return;
        }
        if (Object.keys(data).some(key => !["type", "id", "request", "session"].includes(key))
            || !validSession(data.session) || !validRequest(data.request))
            return;
        if (data.session !== session) {
            try {
                source.postMessage({ type: `${protocol}:result`, id: data.id, session: data.session, views: null, error: "download_page_changed" }, frameOrigin);
            }
            catch { /* The old document may already be gone. */ }
            return;
        }
        if (seen.has(data.id))
            return;
        const generation = epoch, capturedSession = session, id = data.id;
        const request = structuredClone(data.request);
        const reply = (views, error) => {
            if (!disposed && generation === epoch && source === frame.contentWindow && frame.isConnected !== false)
                try {
                    source.postMessage({ type: `${protocol}:result`, id, session: capturedSession, views, error }, frameOrigin);
                }
                catch { /* Receipt remains queryable after a page closes. */ }
        };
        if (pending.has(request.action)) {
            reply(null, "download_busy");
            return;
        }
        seen.add(data.id);
        if (seen.size > 128)
            seen.delete(seen.values().next().value);
        pending.add(request.action);
        void Promise.resolve().then(() => {
            if (disposed || generation !== epoch)
                throw "download_page_changed";
            if (request.action === "project")
                return invoke("project_operator", { request: { project_id: request.project_id, scope: request.scope, operation: request.operation, ...(request.platform ? { platform: request.platform } : {}) } });
            return request.action === "catalog" ? invoke("store_catalog", { request: { query: request.query, cursor: request.cursor } }) : invoke("store_download", { request });
        }).then(value => { const views = response(request, value); reply(views, views ? null : "invalid_store_response"); }, error => reply(null, safeCode(error))).finally(() => pending.delete(request.action));
    };
    const invalidate = () => { epoch++; session = host.crypto.randomUUID(); seen.clear(); };
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
export async function requestStoreDownload(request, host = window, timeoutMs = 120000) {
    return await exchange(request, host, timeoutMs);
}
export async function requestStoreCatalog(request, host = window, timeoutMs = 30000) {
    return await exchange(request, host, timeoutMs);
}
export async function requestProjectOperator(project_id, scope, operation, host = window, platform) {
    return await exchange({ action: "project", project_id, scope, operation, ...(platform ? { platform } : {}) }, host, 120000);
}
function exchange(request, host, timeoutMs) {
    return new Promise((resolve, reject) => {
        if (!validRequest(request) || host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1") {
            reject(new Error("desktop_required"));
            return;
        }
        // Metadata-only handshake first; mutations are pinned to that document generation and parent.
        const snapshot = structuredClone(request);
        let id = host.crypto.randomUUID(), session = null, origin = null;
        const cleanup = () => { host.clearTimeout(timer); host.removeEventListener("message", receive); host.removeEventListener("pagehide", hide); };
        const hide = () => { cleanup(); reject(new Error("download_page_changed")); };
        const receive = (event) => {
            if (event.source !== host.parent || !shellOrigins.has(event.origin) || (origin && origin !== event.origin)
                || event.data?.type !== `${protocol}:result` || event.data.id !== id)
                return;
            if (session === null) {
                if (event.data.connected !== true || event.data.error !== null || event.data.views !== null || !validSession(event.data.session)) {
                    cleanup();
                    reject(new Error("download_operation_failed"));
                    return;
                }
                session = event.data.session;
                origin = event.origin;
                id = host.crypto.randomUUID();
                try {
                    host.parent.postMessage({ type: protocol, id, session, request: snapshot }, origin);
                }
                catch {
                    cleanup();
                    reject(new Error("download_operation_failed"));
                }
                return;
            }
            if (event.data.session !== session)
                return;
            cleanup();
            const views = event.data.error === null ? response(snapshot, event.data.views) : null;
            if (views)
                resolve(views);
            else
                reject(new Error(safeCode(event.data.error)));
        };
        const timer = host.setTimeout(() => { cleanup(); reject(new Error("download_timeout_check_status")); }, timeoutMs);
        host.addEventListener("message", receive);
        host.addEventListener("pagehide", hide);
        try {
            host.parent.postMessage({ type: protocol, id, action: "connect" }, "*");
        }
        catch {
            cleanup();
            reject(new Error("download_operation_failed"));
        }
    });
}
