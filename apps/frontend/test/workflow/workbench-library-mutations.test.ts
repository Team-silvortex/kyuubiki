import test from "node:test";
import assert from "node:assert/strict";
import { createWorkbenchLibraryMutations, getWorkbenchLibraryMutations, resolveWorkbenchLibraryMutationTarget,
  runWorkbenchLibraryTransition, WorkbenchLibraryWritePendingError, type WorkbenchLibraryMutationTarget } from "../../src/components/workbench/workbench-library-mutations.ts";
import { getWorkbenchLibraryMutationPendingLabel } from "../../src/components/workbench/workbench-library-mutation-copy.ts";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";

const label = "Wait for the existing write and read recovery; no action was queued.";
const targets: WorkbenchLibraryMutationTarget[] = [{ kind: "createProject" },
  { kind: "project", projectId: "p" }, { kind: "project", projectId: "q" },
  { kind: "model", projectId: "p", modelId: "a" }, { kind: "model", projectId: "p", modelId: "b" },
  { kind: "model", projectId: "q", modelId: "a" }, { kind: "importProject" }];
const context = () => createWorkbenchProjectContext({ projectId: "p", modelId: "a", versionId: "v" });
function gate() {
  let release!: () => void;
  const promise = new Promise<void>((resolve) => { release = resolve; });
  return { promise, release };
}
const name = (target: WorkbenchLibraryMutationTarget) => JSON.stringify(target);
for (const first of targets) for (const second of targets) {
  test(`library reservation matrix ${name(first)} then ${name(second)}`, async () => {
    const owner = context(), registry = createWorkbenchLibraryMutations(owner, () => () => true), held = gate();
    const pending = registry.run(first, () => held.promise, label), intent = owner.capture();
    let writes = 0;
    const overlaps = first.kind === "importProject" || second.kind === "importProject" ? first.kind === second.kind
      : first.kind === "createProject" || second.kind === "createProject" ? first.kind === second.kind
      : first.projectId === second.projectId && (first.kind === "project" || second.kind === "project" || first.modelId === second.modelId);
    try {
      const next = registry.run(second, async () => { writes += 1; return "second write"; }, label);
      if (overlaps) {
        assert.notEqual(next, pending, "different actions must not borrow the first action's success");
        await assert.rejects(next, /WORKBENCH_LIBRARY_WRITE_PENDING/);
        assert.equal(writes, 0);
      } else { assert.equal(await next, "second write"); assert.equal(writes, 1); }
      assert.equal(intent(), true, "blocked actions cannot advance the existing workspace intent");
    } finally { held.release(); await pending; }
    await registry.run(second, async () => { writes += 1; }, label);
    assert.equal(writes, overlaps ? 1 : 2, "only an explicit action after release may write");
  });
}

for (const mode of ["sync", "async"]) {
  test(`library ${mode} failure releases its reservation without queuing or coalescing a second action`, async () => {
    const registry = createWorkbenchLibraryMutations(context(), () => () => true);
    let attempts = 0;
    const fail = () => { attempts += 1; throw new Error("write rejected"); };
    const first = registry.run(targets[3], mode === "sync" ? fail : async () => fail(), label);
    const failure = assert.rejects(first, /write rejected/);
    await assert.rejects(registry.run(targets[3], async () => { attempts += 1; }, label), /WORKBENCH_LIBRARY_WRITE_PENDING/);
    await failure; assert.equal(attempts, 1);
    await registry.run(targets[3], async () => { attempts += 1; }, label);
    assert.equal(attempts, 2);
  });
}

test("library reservation covers React transition completion and post-write read recovery", async () => {
  const registry = createWorkbenchLibraryMutations(context(), () => () => true);
  const write = gate(), read = gate(), reading = gate();
  let callbackFinished = Promise.resolve();
  const first = registry.run(targets[3], () => runWorkbenchLibraryTransition((callback) => {
    callbackFinished = Promise.resolve(callback());
  }, async () => { await write.promise; reading.release(); await read.promise; return "confirmed"; }), label);
  write.release(); await reading.promise;
  await assert.rejects(registry.run(targets[1], async () => assert.fail("parent deletion cannot interrupt read recovery"), label), /WORKBENCH_LIBRARY_WRITE_PENDING/);
  read.release(); assert.equal(await first, "confirmed"); await callbackFinished;
  await registry.run(targets[1], async () => {}, label);
});

test("library reservations survive effect remount without cancelling an accepted request", async () => {
  const owner = context(), registry = getWorkbenchLibraryMutations(owner), held = gate();
  const first = registry.run(targets[3], async () => { await held.promise; return "committed"; }, label);
  owner.dispose();
  await assert.rejects(registry.run(targets[2], async () => assert.fail("disposed workspace cannot dispatch"), label), /WORKBENCH_CONTEXT_CHANGED/);
  owner.mount(); assert.equal(getWorkbenchLibraryMutations(owner), registry);
  await assert.rejects(registry.run(targets[3], async () => assert.fail("remount cannot replay"), label), /WORKBENCH_LIBRARY_WRITE_PENDING/);
  held.release(); assert.equal(await first, "committed");
});

for (const target of [targets[3], targets[6]]) {
test(`reservations on another backend are independent while an active return still conflicts: ${name(target)}`, async () => {
  let backend = "a";
  const registry = createWorkbenchLibraryMutations(context(), () => { const captured = backend; return () => backend === captured; });
  const a = gate(), b = gate();
  const first = registry.run(target, () => a.promise, label);
  backend = "b"; const second = registry.run(target, () => b.promise, label);
  backend = "a";
  await assert.rejects(registry.run(target, async () => assert.fail("active original backend cannot be bypassed"), label), /WORKBENCH_LIBRARY_WRITE_PENDING/);
  b.release(); await second; a.release(); await first;
  await registry.run(targets[1], async () => {}, label);
});
}

test("different workspace owners never share reservations", async () => {
  const firstOwner = context(), secondOwner = context(), held = gate();
  const first = getWorkbenchLibraryMutations(firstOwner).run(targets[3], () => held.promise, label);
  await getWorkbenchLibraryMutations(secondOwner).run(targets[3], async () => {}, label);
  held.release(); await first;
});

test("unreadable authority is an awaitable rejection, not a synchronous UI exception", async () => {
  const registry = createWorkbenchLibraryMutations(context(), () => { throw new Error("authority unreadable"); });
  let pending!: Promise<void>;
  assert.doesNotThrow(() => { pending = registry.run(targets[3], async () => assert.fail("unreadable authority cannot dispatch"), label); });
  await assert.rejects(pending, /authority unreadable/);
});

test("caller edits cannot move an already reserved target to another model", async () => {
  const registry = createWorkbenchLibraryMutations(context(), () => () => true), held = gate();
  const target = { kind: "model" as const, projectId: "p", modelId: "a" };
  const first = registry.run(target, () => held.promise, label); target.modelId = "b";
  await assert.rejects(registry.run(targets[3], async () => {}, label), /WORKBENCH_LIBRARY_WRITE_PENDING/);
  await registry.run(targets[4], async () => {}, label); held.release(); await first;
});

test("late busy feedback loses authority after a backend switch", async () => {
  let backend = "a";
  const registry = createWorkbenchLibraryMutations(context(), () => { const captured = backend; return () => backend === captured; }), held = gate();
  const first = registry.run(targets[3], () => held.promise, label);
  await assert.rejects(registry.run(targets[1], async () => {}, label), (error) => {
    assert.ok(error instanceof WorkbenchLibraryWritePendingError);
    assert.equal(error.isCurrent(), true); backend = "b"; assert.equal(error.isCurrent(), false); return true;
  });
  held.release(); await first;
});

test("write target mapping groups versions under the model and leaves navigation and explicit import admission outside action mapping", () => {
  for (const action of ["model/save", "model/deleteSelected", "model/renameSelectedVersion", "model/deleteSelectedVersion"]) {
    assert.deepEqual(resolveWorkbenchLibraryMutationTarget(action, "p", "a"), targets[3]);
  }
  for (const action of ["model/saveAs", "project/updateSelected", "project/deleteSelected"]) {
    assert.deepEqual(resolveWorkbenchLibraryMutationTarget(action, "p", "a"), targets[1]);
  }
  assert.deepEqual(resolveWorkbenchLibraryMutationTarget("model/save", "p", null), targets[1]);
  assert.deepEqual(resolveWorkbenchLibraryMutationTarget("project/create", null, null), targets[0]);
  for (const action of ["project/select", "model/openRecoveredSave", "project/import", "model/setWorkspaceMeta"]) assert.equal(resolveWorkbenchLibraryMutationTarget(action, "p", "a"), null);
});

test("library conflict guidance covers all 34 supported locales without English fallback", () => {
  const locales = ["en", "zh", "ja", "es", "ar", "bn", "cs", "da", "de", "el", "fa", "fi", "fr", "he", "hi", "id", "it", "ko", "ms", "nl", "no", "pl", "pt-br", "ro", "ru", "sv", "sw", "ta", "th", "tr", "uk", "ur", "vi", "zh-tw"];
  for (const locale of locales) {
    const copy = getWorkbenchLibraryMutationPendingLabel(locale); assert.ok(copy);
    if (locale !== "en") assert.notEqual(copy, getWorkbenchLibraryMutationPendingLabel("en"));
    assert.equal(copy, getWorkbenchLibraryMutationPendingLabel(locale.toUpperCase()));
  }
});
