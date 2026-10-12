import test from "node:test";
import assert from "node:assert/strict";
import { createWorkbenchAdminRecordMutations } from "../../src/components/workbench/workbench-admin-record-mutations.ts";

function gate() {
  let release!: () => void;
  const promise = new Promise<void>((resolve) => { release = resolve; });
  return { promise, release };
}

test("accepted feedback intent is synchronous and duplicate or disposed actions cannot replace it", async () => {
  const held = gate();
  const events: string[] = [];
  const scope = createWorkbenchAdminRecordMutations(() => events.push("reserved"));
  const first = scope.run("job", () => held.promise, () => events.push("accepted"));
  assert.deepEqual(events, ["accepted", "reserved"]);
  assert.equal(scope.run("job", async () => {}, () => assert.fail("duplicate must not replace feedback")), first);
  scope.dispose();
  await scope.run("other", async () => {}, () => assert.fail("disposed intent must not replace feedback"));
  held.release();
  await first;
});

test("same parent identity shares one flight and never queues a conflicting action", async () => {
  const held = gate();
  const events: string[][] = [];
  const scope = createWorkbenchAdminRecordMutations((ids) => events.push(ids));
  let writes = 0;
  const save = scope.run("job", async () => { writes += 1; await held.promise; });
  const remove = scope.run("job", async () => { assert.fail("conflicting deletion must not execute"); });
  assert.equal(save, remove);
  assert.deepEqual(events, [["job"]], "reservation is synchronous, before the first write starts");
  await Promise.resolve();
  assert.equal(writes, 1);
  held.release();
  await save;
  assert.deepEqual(events, [["job"], []]);
});

test("different parent identities execute independently", async () => {
  const held = gate();
  const events: string[][] = [];
  const scope = createWorkbenchAdminRecordMutations((ids) => events.push(ids));
  const first = scope.run("first", () => held.promise);
  await scope.run("other", async () => {});
  assert.deepEqual(events.at(-1), ["first"]);
  held.release();
  await first;
  assert.deepEqual(events.at(-1), []);
});

test("different workspaces do not share mutation reservations", async () => {
  const held = gate();
  const first = createWorkbenchAdminRecordMutations(() => {}).run("same", () => held.promise);
  let writes = 0;
  await createWorkbenchAdminRecordMutations(() => {}).run("same", async () => { writes += 1; });
  assert.equal(writes, 1);
  held.release();
  await first;
});

test("reservation lasts through follow-up reads, not only write confirmation", async () => {
  const write = gate(), read = gate(), reading = gate();
  const events: string[][] = [];
  const scope = createWorkbenchAdminRecordMutations((ids) => events.push(ids));
  const first = scope.run("job", async () => { await write.promise; reading.release(); await read.promise; });
  write.release();
  await reading.promise;
  assert.equal(scope.run("job", async () => { assert.fail("read recovery must not replay a write"); }), first);
  assert.deepEqual(events, [["job"]]);
  read.release();
  await first;
  assert.deepEqual(events.at(-1), []);
});

for (const failure of ["async", "sync"] as const) {
  test(`${failure} failure releases the reservation and retries only on an explicit later action`, async () => {
    const events: string[][] = [];
    const scope = createWorkbenchAdminRecordMutations((ids) => events.push(ids));
    let attempts = 0;
    const fail = () => { attempts += 1; throw new Error("write unavailable"); };
    const failed = scope.run("job", failure === "sync" ? fail : async () => fail());
    assert.equal(scope.run("job", async () => { attempts += 1; }), failed);
    await assert.rejects(failed, /write unavailable/u);
    assert.equal(attempts, 1);
    assert.deepEqual(events, [["job"], []]);
    await scope.run("job", async () => { attempts += 1; });
    assert.equal(attempts, 2);
  });
}

test("disposal suppresses new writes and late state publication without cancelling an owned write", async () => {
  const held = gate();
  const events: string[][] = [];
  const scope = createWorkbenchAdminRecordMutations((ids) => events.push(ids));
  let committed = false;
  const operation = scope.run("job", async () => { await held.promise; committed = true; });
  scope.dispose();
  await scope.run("other", async () => { assert.fail("disposed workspace must not begin a write"); });
  held.release();
  await operation;
  assert.equal(committed, true);
  assert.deepEqual(events, [["job"]]);
});

test("effect remount restores pending identities rather than replacing their flights", async () => {
  const held = gate();
  const events: string[][] = [];
  const scope = createWorkbenchAdminRecordMutations((ids) => events.push(ids));
  const first = scope.run("job", () => held.promise);
  scope.dispose();
  scope.mount();
  assert.equal(scope.run("job", async () => { assert.fail("remount cannot replay a pending write"); }), first);
  assert.deepEqual(events, [["job"], ["job"]]);
  held.release();
  await first;
  assert.deepEqual(events.at(-1), []);
});
