import test from "node:test";
import assert from "node:assert/strict";
import { reconcileWorkbenchAdminDraft } from "../../src/components/workbench/workbench-admin-security-state.ts";

const baseline = (contents: string, jobId: string | null = "selected") => ({ jobId, contents });

for (const [label, original, incoming] of [
  ["message", "running", "completed"], ["project", "project-a", "project-b"],
  ["version", "version-a", "version-b"], ["case", "case-a", "case-b"],
  ["empty field", "", "linked"], ["cleared field", "linked", ""],
  ["result JSON", '{"metric":1}', '{"metric":2}'],
]) {
  test(`clean admin ${label} follows its selected record's refreshed value`, () => {
    assert.equal(reconcileWorkbenchAdminDraft(original, baseline(original), baseline(incoming)), incoming);
  });
  test(`dirty admin ${label} survives its selected record's refreshed value`, () => {
    assert.equal(reconcileWorkbenchAdminDraft("working", baseline(original), baseline(incoming)), "working");
  });
}

test("an intentionally cleared working field remains dirty", () => {
  assert.equal(reconcileWorkbenchAdminDraft("", baseline("linked"), baseline("updated-link")), "");
});

test("first selection loads the verified record rather than retaining an unowned draft", () => {
  assert.equal(reconcileWorkbenchAdminDraft("unowned", baseline("", null), baseline("verified")), "verified");
});

test("explicit selection change loads the other record, even when its id was selected earlier", () => {
  assert.equal(reconcileWorkbenchAdminDraft("working", baseline("old", "first"), baseline("other", "second")), "other");
  assert.equal(reconcileWorkbenchAdminDraft("other-working", baseline("other", "second"), baseline("latest", "first")), "latest");
});

test("confirmed removal clears a dirty draft and null selection never retains orphan editing", () => {
  assert.equal(reconcileWorkbenchAdminDraft("working", baseline("old"), baseline("", null)), "");
  assert.equal(reconcileWorkbenchAdminDraft("orphan", baseline("", null), baseline("", null)), "");
});

test("read baselines advance independently from dirty editing and allow an explicit revert", () => {
  const old = baseline("initial");
  const refreshed = baseline("refreshed");
  const latest = baseline("latest");
  assert.equal(reconcileWorkbenchAdminDraft("working", old, refreshed), "working");
  assert.equal(reconcileWorkbenchAdminDraft("working", refreshed, latest), "working");
  assert.equal(reconcileWorkbenchAdminDraft("latest", latest, baseline("next")), "next");
});

test("confirmed save cleans only the submitted value, never later typing", () => {
  const old = baseline("initial");
  const committed = baseline("submitted");
  assert.equal(reconcileWorkbenchAdminDraft("submitted", old, committed), "submitted");
  assert.equal(reconcileWorkbenchAdminDraft("submitted", committed, baseline("subsequent")), "subsequent");
  assert.equal(reconcileWorkbenchAdminDraft("newer typing", old, committed), "newer typing");
  assert.equal(reconcileWorkbenchAdminDraft("newer typing", committed, baseline("subsequent")), "newer typing");
});
