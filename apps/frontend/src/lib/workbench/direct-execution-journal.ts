import { createDirectExecutionJournalClient, validateDirectExecutionIntent, validateDirectExecutionList, sameDirectExecutionIntent,
  type DirectExecutionJournal, type DirectExecutionIntent } from "../../../../desktop-shared/src/direct-execution-journal-bridge.ts";
export { type DirectExecutionJournal, type DirectExecutionIntent };
const DATABASE = "kyuubiki-direct-execution";
const STORE = "pending";
export function createBrowserDirectExecutionJournal(factory = () => globalThis.indexedDB): DirectExecutionJournal {
  async function access<T>(mode: IDBTransactionMode, work: (entries: DirectExecutionIntent[], store: IDBObjectStore) => T): Promise<T> {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      let settled = false;
      const timer = setTimeout(() => fail(), 3000);
      function fail() { if (!settled) { settled = true; clearTimeout(timer); reject(new Error("direct:journal_unavailable")); } }
      try {
        const request = factory().open(DATABASE, 1);
        request.onerror = fail; request.onblocked = fail;
        request.onupgradeneeded = () => {
          if (settled) { request.transaction?.abort(); return; }
          request.result.createObjectStore(STORE, { keyPath: ["scope", "endpoint"] });
        };
        request.onsuccess = () => { if (settled) { request.result.close(); return; } settled = true; clearTimeout(timer); resolve(request.result); };
      } catch { fail(); }
    });
    return new Promise<T>((resolve, reject) => {
      let result: T, failure: unknown;
      try {
        // Strict durability is requested where the browser supports IndexedDB's option.
        const tx = db.transaction(STORE, mode, mode === "readwrite" ? { durability: "strict" } : undefined);
        tx.oncomplete = () => { db.close(); resolve(result); };
        tx.onabort = () => { db.close(); reject(failure ?? new Error("direct:journal_unavailable")); };
        tx.onerror = () => { failure ??= new Error("direct:journal_unavailable"); };
        const store = tx.objectStore(STORE), read = store.getAll(undefined, 65);
        read.onsuccess = () => {
          try { result = work(validateDirectExecutionList(read.result), store); }
          catch (error) { failure = error; tx.abort(); }
        };
      } catch (error) { db.close(); reject(error); }
    });
  }
  return {
    list: () => access("readonly", entries => entries),
    reserve(value) {
      const intent = { ...validateDirectExecutionIntent(value) };
      return access("readwrite", (entries, store) => {
        const existing = entries.find(e => e.scope === intent.scope && e.endpoint === intent.endpoint);
        if (existing) return { intent: existing, is_new: false };
        if (entries.some(e => e.execution_id === intent.execution_id)) throw new Error("direct:journal_corrupt");
        if (entries.length >= 64) throw new Error("direct:journal_full");
        store.add(intent);
        return { intent, is_new: true };
      });
    },
    remove(value) {
      const intent = { ...validateDirectExecutionIntent(value) };
      return access("readwrite", (entries, store) => {
        if (entries.some(e => sameDirectExecutionIntent(e, intent))) store.delete([intent.scope, intent.endpoint]);
      });
    },
  };
}
export function createWorkbenchDirectExecutionJournal(): DirectExecutionJournal {
  let journal: DirectExecutionJournal | undefined;
  const backend = () => journal ??= (typeof window === "undefined" ? null : createDirectExecutionJournalClient()) ?? createBrowserDirectExecutionJournal();
  return { list: () => backend().list(), reserve: intent => backend().reserve(intent), remove: intent => backend().remove(intent) };
}
export const workbenchDirectExecutionJournal = createWorkbenchDirectExecutionJournal();
