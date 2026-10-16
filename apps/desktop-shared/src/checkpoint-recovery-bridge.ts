import { createJournalSessionClient, validJournalSession } from "./journal-session-client.ts";

export type CheckpointIntent = {
  key: string; scope: string; operation: "model" | "version";
  parentId: string; requestId: string; createdAt: number;
};
export type CheckpointReservation = { intent: CheckpointIntent; isNew: boolean };
export type CheckpointJournal = {
  reserve(intent: CheckpointIntent): Promise<CheckpointReservation>;
  list(): Promise<CheckpointIntent[]>;
  // Local guard, never serialized; native mutations cannot be recalled after dispatch.
  remove(key: string, requestId: string, assertCurrent?: () => void): Promise<void>;
};

export function validateCheckpointIntent(value: unknown): CheckpointIntent {
  const entry = value as CheckpointIntent;
  const fields = ["key", "scope", "operation", "parentId", "requestId", "createdAt"];
  if (!entry || typeof entry !== "object" || Object.keys(entry).some(key => !fields.includes(key))
      || !hash(entry.key) || !hash(entry.scope) || !["model", "version"].includes(entry.operation)
      || typeof entry.parentId !== "string" || !entry.parentId || new TextEncoder().encode(entry.parentId).length > 128
      || !requestId(entry.requestId) || !Number.isSafeInteger(entry.createdAt) || entry.createdAt < 0 || entry.createdAt > 8.64e15) {
    throw new Error("checkpoint:journal_corrupt");
  }
  return entry;
}

export function validateCheckpointReservation(value: unknown, requested: CheckpointIntent): CheckpointReservation {
  validateCheckpointIntent(requested);
  const reservation = value as CheckpointReservation;
  if (!reservation || typeof reservation !== "object" || typeof reservation.isNew !== "boolean"
      || Object.keys(reservation).some(key => !["intent", "isNew"].includes(key))) throw new Error("checkpoint:journal_corrupt");
  const entry = { ...validateCheckpointIntent(reservation.intent) };
  // Existing reservations retain their original ID/time; new ones must echo all fields.
  if (entry.key !== requested.key || entry.scope !== requested.scope || entry.operation !== requested.operation
      || entry.parentId !== requested.parentId || (reservation.isNew
        && (entry.requestId !== requested.requestId || entry.createdAt !== requested.createdAt))) {
    throw new Error("checkpoint:journal_corrupt");
  }
  return { intent: entry, isNew: reservation.isNew };
}

const hash = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
const requestId = (value: unknown): value is string => typeof value === "string" && /^[A-Za-z0-9_-]{16,128}$/.test(value);
const protocol = "kyuubiki:checkpoint-recovery/v2";
const frameOrigin = "http://127.0.0.1:3000";
const commands = { list: "list_checkpoint_intents", reserve: "reserve_checkpoint_intent", remove: "remove_checkpoint_intent" };
type Action = keyof typeof commands;
const codes = new Set(["checkpoint:journal_corrupt", "checkpoint:journal_busy", "checkpoint:journal_unsafe_path", "checkpoint:recovery_capacity_reached"]);

function validateResult(action: Action, result: any, intent?: CheckpointIntent) {
  if (action === "list") {
    if (!Array.isArray(result) || result.length > 64) throw new Error("checkpoint:journal_corrupt");
    const entries = result.map(validateCheckpointIntent);
    if (new Set(entries.map(entry => entry.key)).size !== entries.length) throw new Error("checkpoint:journal_corrupt");
  } else if (action === "reserve") {
    if (!intent) throw new Error("checkpoint:journal_corrupt");
    return validateCheckpointReservation(result, intent);
  } else if (result !== null) throw new Error("checkpoint:journal_corrupt");
  return result;
}

/** Dedicated metadata-only IPC, never an arbitrary command or filesystem bridge. */
export function installWorkbenchCheckpointHost(frame: HTMLIFrameElement,
  invoke: (command: string, payload?: Record<string, unknown>) => Promise<unknown>, host: Window = window) {
  let disposed = false, suspended = false, generation = Symbol();
  let session = host.crypto.randomUUID();
  let queued = 0;
  let tail = Promise.resolve();
  const retire = () => { generation = Symbol(); session = host.crypto.randomUUID(); };
  const hide = () => { suspended = true; retire(); };
  const show = () => { suspended = false; };
  const receive = (event: MessageEvent) => {
    if (disposed || suspended || !frame.isConnected || !frame.contentWindow || event.source !== frame.contentWindow || event.origin !== frameOrigin) return;
    const data = event.data;
    if (data?.type !== protocol || typeof data.id !== "string" || !/^[a-zA-Z0-9-]{1,80}$/.test(data.id)) return;
    if (data.action === "connect") {
      if (Object.keys(data).some(key => !["type", "id", "action"].includes(key))) return;
      try { (event.source as Window).postMessage({ type: `${protocol}:result`, id: data.id, ok: true, session, result: null }, frameOrigin); }
      catch { /* A handshake has no native side effects. */ }
      return;
    }
    if (!validJournalSession(data.session) || typeof data.action !== "string" || !Object.hasOwn(commands, data.action)
      || Object.keys(data).some(key => !["type", "id", "action", "intent", "key", "requestId", "session"].includes(key))) return;
    if (data.session !== session) {
      try { (event.source as Window).postMessage({ type: `${protocol}:result`, id: data.id, session: data.session,
        ok: false, error: "checkpoint:journal_unavailable", expired: true }, frameOrigin); }
      catch { /* The old document may already be gone. */ }
      return;
    }
    const action: Action = data.action;
    let intent: CheckpointIntent | undefined;
    try {
      if (action === "reserve") intent = { ...validateCheckpointIntent(data.intent) };
      if (action === "remove" && (!hash(data.key) || !requestId(data.requestId))) return;
      if (action !== "reserve" && data.intent !== undefined) return;
      if (action !== "remove" && (data.key !== undefined || data.requestId !== undefined)) return;
    } catch { return; }
    const source = event.source as Window, capturedGeneration = generation, id = data.id, capturedSession = session;
    const payload = action === "reserve" ? { payload: { ...intent } }
      : action === "remove" ? { key: data.key, requestId: data.requestId } : {};
    // Snapshot document ownership and metadata before waiting behind native work.
    const current = () => !disposed && !suspended && frame.isConnected && source === frame.contentWindow && capturedGeneration === generation;
    const reply = (payload: object) => {
      try {
        if (current()) source.postMessage({ type: `${protocol}:result`, id, session: capturedSession, ...payload }, frameOrigin);
      } catch { /* The frame can close while its durable reservation completes. */ }
    };
    if (queued >= 8) { reply({ ok: false, error: "checkpoint:journal_busy" }); return; }
    queued++;
    tail = tail.then(async () => {
      if (!current()) return;
      try {
        const result = validateResult(action, await invoke(commands[action], payload), intent);
        reply({ ok: true, result });
      } catch (error) {
        const code = String(error).replace(/^Error: /, "");
        reply({ ok: false, error: codes.has(code) ? code : "checkpoint:journal_unavailable" });
      }
    }).finally(() => { queued--; });
  };
  host.addEventListener("message", receive);
  host.addEventListener("pagehide", hide);
  host.addEventListener("pageshow", show);
  frame.addEventListener("load", retire);
  return () => {
    disposed = true; host.removeEventListener("message", receive);
    host.removeEventListener("pagehide", hide); host.removeEventListener("pageshow", show);
    frame.removeEventListener("load", retire);
  };
}

export function createWorkbenchCheckpointClient(host: Window = window, timeoutMs = 4000): (CheckpointJournal & { dispose(): void }) | null {
  if (host.parent === host || new URLSearchParams(host.location.search).get("desktopLayout") !== "1") return null;
  const client = createJournalSessionClient(host, timeoutMs, protocol, "checkpoint", codes,
    (request, result) => validateResult(request.action, result, request.intent));
  return {
    list: () => client.request({ action: "list" }),
    async reserve(value) {
      const intent = { ...validateCheckpointIntent(value) };
      return client.request({ action: "reserve", intent });
    },
    async remove(key, id, assertCurrent) {
      if (!hash(key) || !requestId(id)) throw new Error("checkpoint:invalid_recovery_key");
      await client.request({ action: "remove", key, requestId: id }, assertCurrent);
    },
    dispose: client.dispose,
  };
}
