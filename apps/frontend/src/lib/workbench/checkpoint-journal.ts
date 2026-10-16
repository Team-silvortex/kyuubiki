import { createWorkbenchCheckpointClient, validateCheckpointIntent, validateCheckpointReservation, type CheckpointIntent, type CheckpointJournal } from "../../../../desktop-shared/src/checkpoint-recovery-bridge.ts";
export { validateCheckpointIntent, validateCheckpointReservation, type CheckpointIntent, type CheckpointJournal };

export const CHECKPOINT_JOURNAL_DATABASE = "kyuubiki-checkpoint-recovery";
const STORE = "pending";
const LIMIT = 64;

export function createWorkbenchCheckpointJournal(): CheckpointJournal {
  const browser = createCheckpointJournal();
  let native: ReturnType<typeof createWorkbenchCheckpointClient> | undefined;
  const backend = () => {
    if (native === undefined) native = typeof window === "undefined" ? null : createWorkbenchCheckpointClient();
    // An old or unavailable desktop host must fail closed, not silently fall
    // back to browser storage whose lifetime may end with this WebView.
    return native ?? browser;
  };
  const changed = () => { if (typeof window !== "undefined") window.dispatchEvent(new Event("kyuubiki-checkpoint-journal")); };
  return {
    list: () => backend().list(),
    async reserve(intent) { const value = await backend().reserve(intent); if (native) changed(); return value; },
    async remove(key, requestId, assertCurrent) { await backend().remove(key, requestId, assertCurrent); if (native) changed(); },
  };
}

// IndexedDB read/write transactions allocate one key across multiple WebViews.
// Only small identities are retained, never names, geometry, URLs or credentials.
export function createCheckpointJournal(factory: () => IDBFactory | undefined = () => globalThis.indexedDB): CheckpointJournal {
  async function open(): Promise<IDBDatabase> {
    const idb = factory();
    if (!idb) throw new Error("checkpoint:journal_unavailable");
    return new Promise((resolve, reject) => {
      let settled = false;
      const timer = setTimeout(() => finish(new Error("checkpoint:journal_unavailable")), 3000);
      const finish = (error?: Error, database?: IDBDatabase) => {
        if (settled) { database?.close(); return; }
        settled = true;
        clearTimeout(timer);
        if (error) reject(error); else resolve(database!);
      };
      try {
        const request = idb.open(CHECKPOINT_JOURNAL_DATABASE, 1);
        request.onupgradeneeded = () => {
          if (settled) { request.transaction?.abort(); return; }
          if (!request.result.objectStoreNames.contains(STORE)) request.result.createObjectStore(STORE, { keyPath: "key" });
        };
        request.onsuccess = () => finish(undefined, request.result);
        request.onerror = () => finish(new Error("checkpoint:journal_unavailable", { cause: request.error }));
        request.onblocked = () => finish(new Error("checkpoint:journal_unavailable"));
      } catch (cause) { finish(new Error("checkpoint:journal_unavailable", { cause })); }
    });
  }

  async function access<T>(mode: IDBTransactionMode, work: (store: IDBObjectStore, done: (value: T) => void,
    fail: (error: unknown) => void, checkpoint: () => boolean) => void, assertCurrent?: () => void): Promise<T> {
    assertCurrent?.();
    const db = await open();
    return new Promise<T>((resolve, reject) => {
      let result: T;
      let failure: unknown;
      let tx: IDBTransaction;
      const finishError = () => { db.close(); reject(failure ?? new Error("checkpoint:journal_unavailable")); };
      try {
        assertCurrent?.();
        tx = db.transaction(STORE, mode);
        tx.oncomplete = () => {
          db.close();
          if (mode === "readwrite" && typeof window !== "undefined") window.dispatchEvent(new Event("kyuubiki-checkpoint-journal"));
          resolve(result);
        };
        tx.onabort = finishError;
        tx.onerror = () => { failure ??= new Error("checkpoint:journal_unavailable", { cause: tx.error }); };
        const fail = (error: unknown) => { failure = error; tx.abort(); };
        const checkpoint = () => {
          try { assertCurrent?.(); return true; }
          catch (error) { fail(error); return false; }
        };
        work(tx.objectStore(STORE), (value) => { result = value; }, fail, checkpoint);
      } catch (error) { failure = error; finishError(); }
    });
  }

  function readEntries(store: IDBObjectStore, accept: (entries: CheckpointIntent[]) => void, fail: (error: unknown) => void) {
    const request = store.getAll(undefined, LIMIT + 1);
    request.onsuccess = () => {
      try {
        if (request.result.length > LIMIT) throw new Error("checkpoint:journal_corrupt");
        accept(request.result.map(validateCheckpointIntent));
      } catch (error) { fail(error); }
    };
  }

  return {
    reserve(value) {
      const intent = { ...validateCheckpointIntent(value) };
      return access("readwrite", (store, done, fail) => readEntries(store, (entries) => {
        const existing = entries.find((entry) => entry.key === intent.key);
        if (existing) { done(validateCheckpointReservation({ intent: existing, isNew: false }, intent)); return; }
        if (entries.length >= LIMIT) throw new Error("checkpoint:recovery_capacity_reached");
        store.add(intent);
        done({ intent, isNew: true });
      }, fail));
    },
    list: () => access("readonly", (store, done, fail) => readEntries(store, done, fail)),
    remove: (key, requestId, assertCurrent) => access("readwrite", (store, done, fail, checkpoint) => {
      const request = store.get(key);
      request.onsuccess = () => {
        if (!checkpoint()) return;
        try {
          if (request.result && validateCheckpointIntent(request.result).requestId === requestId) {
            // Delete success is still abortable before this transaction commits.
            store.delete(key).onsuccess = checkpoint;
          }
          done(undefined);
        } catch (error) { fail(error); }
      };
    }, assertCurrent),
  };
}

export async function checkpointDigest(value: string): Promise<string> {
  if (!globalThis.crypto?.subtle) throw new Error("checkpoint:secure_context_required");
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
