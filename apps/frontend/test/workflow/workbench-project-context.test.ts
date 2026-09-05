import test from "node:test";
import assert from "node:assert/strict";
import { createWorkbenchProjectContext } from "../../src/lib/workbench/project-context.ts";

const original = { projectId: "a", modelId: "model-a", versionId: "version-a" };

test("project context rejects an old request even after navigating away and back", () => {
  const context = createWorkbenchProjectContext(original);
  const current = context.capture();
  context.update({ projectId: "b", modelId: null, versionId: null });
  context.update(original);
  assert.equal(current(), false);
  assert.deepEqual(context.current(), original);
});

test("project context latest operation intent wins without requiring a selection change", () => {
  const context = createWorkbenchProjectContext(original);
  const first = context.begin();
  const second = context.begin();
  assert.equal(first(), false);
  assert.equal(second(), true);
  context.update({ ...original });
  assert.equal(second(), true, "unrelated renders must not invalidate the operation");
});

for (const key of ["projectId", "modelId", "versionId"] as const) {
  test(`project context invalidates pending work when ${key} changes`, () => {
    const context = createWorkbenchProjectContext(original);
    const current = context.capture();
    context.update({ ...original, [key]: "changed" });
    assert.equal(current(), false);
  });
}

test("project context unmount permanently invalidates old tickets and protects its selection value", () => {
  const context = createWorkbenchProjectContext(original);
  const current = context.begin();
  context.current().projectId = "external mutation";
  assert.deepEqual(context.current(), original);
  context.dispose();
  assert.equal(current(), false);
  const whileUnmounted = context.capture();
  assert.equal(whileUnmounted(), false);
  context.mount();
  assert.equal(current(), false);
  assert.equal(whileUnmounted(), false);
  assert.equal(context.begin()(), true);
});

for (const kind of ["projectId", "modelId", "versionId"] as const) {
  test(`project context deletion detaches only the removed ${kind} and its descendants`, () => {
    const context = createWorkbenchProjectContext(original);
    const reading = context.begin();
    assert.equal(context.detachDeleted(kind, "unrelated"), null);
    assert.equal(reading(), true);
    const expected = {
      projectId: kind === "projectId" ? null : original.projectId,
      modelId: kind === "versionId" ? original.modelId : null,
      versionId: null,
    };
    assert.deepEqual(context.detachDeleted(kind, original[kind]), expected);
    assert.deepEqual(context.current(), expected);
    assert.equal(reading(), false, "a pending read cannot reattach a deleted record");
    assert.equal(context.hasModel(original.modelId), kind === "versionId");
    assert.equal(context.detachDeleted(kind, original[kind]), null, "repeated completion is harmless");
  });

  test(`project context does not reconcile deleted ${kind} after unmount`, () => {
    const context = createWorkbenchProjectContext(original);
    context.dispose();
    assert.equal(context.detachDeleted(kind, original[kind]), null);
    assert.equal(context.hasModel(original.modelId), false);
    assert.deepEqual(context.current(), original);
  });
}
