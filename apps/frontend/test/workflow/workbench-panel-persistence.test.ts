import assert from "node:assert/strict";
import test from "node:test";
import { createWorkbenchPanelPersistence } from "../../src/components/workbench/workbench-panel-persistence.ts";
import type { PanelLayoutRecord } from "../../../desktop-shared/src/workbench-panel-layout-bridge.ts";

const layout = (sidebar: number): PanelLayoutRecord => ({ version: 1, sizes: { sidebar } });
const settle = async () => { for (let index = 0; index < 10; index++) await Promise.resolve(); };
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((a, b) => { resolve = a; reject = b; });
  return { promise, resolve, reject };
}
function fixture() {
  let cached: string | null = JSON.stringify(layout(200));
  const reads: ReturnType<typeof deferred<PanelLayoutRecord | null>>[] = [];
  const writes: { layout: PanelLayoutRecord; result: ReturnType<typeof deferred<PanelLayoutRecord | null>> }[] = [];
  const statuses: string[] = [];
  const restored: object[] = [];
  const persistence = createWorkbenchPanelPersistence({
    storage: () => ({ getItem: () => cached, setItem: (_key, value) => { cached = value; }, removeItem: () => { cached = null; } }),
    client: {
      read: () => { const read = deferred<PanelLayoutRecord | null>(); reads.push(read); return read.promise; },
      write: value => { const result = deferred<PanelLayoutRecord | null>(); writes.push({ layout: value, result }); return result.promise; },
      dispose: () => {},
    },
    restore: value => { restored.push(value); },
    status: value => { statuses.push(value); },
  });
  return { persistence, reads, writes, statuses, restored, cache: () => cached };
}

test("native committed layout is authoritative over stale WebView cache, including an explicit reset", async () => {
  for (const value of [layout(327), { version: 1, sizes: {} } as PanelLayoutRecord]) {
    const f = fixture();
    assert.deepEqual(f.persistence.initial, { sidebar: 200 });
    f.reads[0].resolve(value);
    await settle();
    assert.deepEqual(f.restored, [value.sizes]);
    assert.equal(f.writes.length, 0, "hydrating preferences must not write native storage");
    assert.equal(f.statuses.at(-1), "native");
    assert.equal(f.cache(), Object.keys(value.sizes).length ? JSON.stringify(value) : null);
    f.persistence.dispose();
  }
});

test("late native load cannot undo an in-flight drag or a newer committed reset", async () => {
  const f = fixture();
  f.persistence.changed();
  f.reads[0].resolve(layout(480));
  await settle();
  assert.deepEqual(f.restored, []);
  assert.equal(f.writes.length, 0);
  const reset = fixture();
  reset.persistence.save({});
  reset.reads[0].resolve(layout(480));
  await settle();
  assert.deepEqual(reset.restored, []);
  assert.deepEqual(reset.writes.map(write => write.layout), [{ version: 1, sizes: {} }]);
});

test("rapid commits are serialized and coalesced without falsely reporting an unfinished save", async () => {
  const f = fixture();
  f.reads[0].resolve(null);
  await settle();
  for (const size of [300, 320, 340]) f.persistence.save({ sidebar: size });
  assert.equal(f.writes.length, 1);
  assert.equal(f.statuses.at(-1), "native-saving");
  f.writes[0].result.resolve(layout(300));
  await settle();
  assert.deepEqual(f.writes.map(write => write.layout), [layout(300), layout(340)]);
  assert.equal(f.statuses.at(-1), "native-saving");
  f.writes[1].result.resolve(layout(340));
  await settle();
  assert.equal(f.statuses.at(-1), "native");
});

test("native failures retain live preferences and a later adjustment can retry", async () => {
  const f = fixture();
  f.reads[0].reject(new Error("unavailable"));
  await settle();
  assert.equal(f.statuses.at(-1), "memory");
  f.persistence.save({ sidebar: 350 });
  f.reads[1].resolve(layout(200));
  await settle();
  assert.deepEqual(f.restored, []);
  f.writes[0].result.reject(new Error("disk full"));
  await settle();
  assert.equal(f.statuses.at(-1), "memory");
  assert.equal(f.cache(), JSON.stringify(layout(350)));
  f.persistence.save({ sidebar: 360 });
  f.writes[1].result.resolve(layout(360));
  await settle();
  assert.equal(f.statuses.at(-1), "native");
});

test("unmount ignores late hydration and browser-only storage failures remain non-fatal", async () => {
  const f = fixture();
  f.persistence.dispose();
  f.reads[0].resolve(layout(320));
  await settle();
  assert.deepEqual(f.restored, []);
  const statuses: string[] = [];
  const browser = createWorkbenchPanelPersistence({ storage: () => { throw new Error("storage denied"); }, client: null,
    restore: () => assert.fail(), status: status => statuses.push(status) });
  browser.save({ sidebar: 280 });
  assert.equal(statuses.at(-1), "memory");
});
