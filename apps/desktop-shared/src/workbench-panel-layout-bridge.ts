export type PanelLayoutRecord = { version: 1; sizes: Partial<Record<"sidebar" | "inspector" | "report", number>> };
const protocol = "kyuubiki:workbench-panel-layout/v1";
const resultType = `${protocol}:result`;
const frameOrigin = "http://127.0.0.1:3000";
const shellOrigins = new Set(["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]);
type Invoke = (command: string, payload?: Record<string, unknown>) => Promise<unknown>;

export function isPanelLayoutRecord(value: unknown): value is PanelLayoutRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  if (record.version !== 1 || Object.keys(record).some(key => !["version", "sizes"].includes(key))) return false;
  const sizes = record.sizes;
  if (!sizes || typeof sizes !== "object" || Array.isArray(sizes)) return false;
  return Object.entries(sizes).every(([key, size]) => ["sidebar", "inspector", "report"].includes(key)
    && typeof size === "number" && Number.isFinite(size) && size > 0 && size <= 5000);
}

/** Only the pinned local Workbench frame can access this fixed preference, never arbitrary IPC. */
export function installWorkbenchPanelLayoutHost(frame: HTMLIFrameElement, invoke: Invoke, host: Window = window) {
  let disposed = false;
  let queued = 0;
  let tail = Promise.resolve();
  const receive = (event: MessageEvent) => {
    if (disposed || event.source !== frame.contentWindow || event.origin !== frameOrigin) return;
    const data = event.data;
    if (!data || data.type !== protocol || typeof data.id !== "string" || !/^[a-zA-Z0-9-]{1,80}$/.test(data.id)) return;
    if (!["read", "write"].includes(data.action) || Object.keys(data).some(key => !["type", "id", "action", "layout"].includes(key))) return;
    if (data.action === "write" && !isPanelLayoutRecord(data.layout)) return;
    const source = event.source as Window;
    const reply = (payload: object) => {
      try {
        if (!disposed && source === frame.contentWindow) source.postMessage({ type: resultType, id: data.id, ...payload }, frameOrigin);
      } catch { /* The frame may have closed while native storage was busy. */ }
    };
    if (queued >= 8) { reply({ ok: false }); return; }
    queued++;
    tail = tail.then(async () => {
      if (disposed) return;
      try {
        const layout = await invoke(data.action === "read" ? "get_workbench_panel_layout" : "set_workbench_panel_layout",
          data.action === "write" ? { payload: data.layout } : {});
        if (!isPanelLayoutRecord(layout) && !(data.action === "read" && layout === null)) throw new Error("Invalid layout response");
        reply({ ok: true, layout });
      } catch {
        // Do not expose native paths or arbitrary backend errors to the embedded page.
        reply({ ok: false });
      }
    }).finally(() => { queued--; });
  };
  host.addEventListener("message", receive);
  return () => { disposed = true; host.removeEventListener("message", receive); };
}

export type WorkbenchPanelLayoutClient = {
  read: () => Promise<PanelLayoutRecord | null>;
  write: (layout: PanelLayoutRecord) => Promise<PanelLayoutRecord | null>;
  dispose: () => void;
};

export function createWorkbenchPanelLayoutClient(host: Window = window, timeoutMs = 4000): WorkbenchPanelLayoutClient | null {
  if (host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1") return null;
  let origin: string | null = null;
  let disposed = false;
  const pending = new Map<string, { action: string; timer: number; resolve: (layout: PanelLayoutRecord | null) => void; reject: (error: Error) => void }>();
  const receive = (event: MessageEvent) => {
    if (event.source !== host.parent || !shellOrigins.has(event.origin) || (origin && event.origin !== origin)) return;
    const data = event.data;
    if (data?.type !== resultType || typeof data.id !== "string") return;
    const request = pending.get(data.id);
    if (!request) return;
    pending.delete(data.id);
    host.clearTimeout(request.timer);
    if (data.ok !== true || (!isPanelLayoutRecord(data.layout) && !(request.action === "read" && data.layout === null))) {
      request.reject(new Error("Desktop panel layout storage unavailable"));
      return;
    }
    origin = event.origin;
    request.resolve(data.layout);
  };
  host.addEventListener("message", receive);
  const request = (action: "read" | "write", layout?: PanelLayoutRecord): Promise<PanelLayoutRecord | null> => {
    if (disposed || pending.size >= 8 || (action === "write" && (!origin || !isPanelLayoutRecord(layout)))) {
      return Promise.reject(new Error("Desktop panel layout bridge is not ready"));
    }
    return new Promise((resolve, reject) => {
      const id = host.crypto.randomUUID();
      const timer = host.setTimeout(() => {
        pending.delete(id);
        reject(new Error("Desktop panel layout storage timed out"));
      }, timeoutMs);
      pending.set(id, { action, timer, resolve, reject });
      try {
        // The discovery read contains no data. Writes only target the verified native origin.
        host.parent.postMessage({ type: protocol, id, action, ...(layout ? { layout } : {}) }, origin ?? "*");
      } catch (error) {
        host.clearTimeout(timer);
        pending.delete(id);
        reject(error);
      }
    });
  };
  return {
    read: () => request("read"),
    write: layout => request("write", layout),
    dispose: () => {
      disposed = true;
      host.removeEventListener("message", receive);
      for (const entry of pending.values()) {
        host.clearTimeout(entry.timer);
        entry.reject(new Error("Desktop panel layout bridge closed"));
      }
      pending.clear();
    },
  };
}
