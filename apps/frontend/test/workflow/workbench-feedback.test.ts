import test from "node:test";
import assert from "node:assert/strict";
import { createWorkbenchFeedback, withWorkbenchFeedbackInvalidation } from "../../src/components/workbench/workbench-feedback.ts";

test("a newer feedback owner supersedes the previous operation without cancelling its write", () => {
  const feedback = createWorkbenchFeedback();
  const old = feedback.begin();
  assert.equal(old(), true);
  const current = feedback.begin();
  assert.equal(old(), false);
  assert.equal(current(), true);
});

test("repeated identical messages invalidate the old owner even when React would bail out", () => {
  const feedback = createWorkbenchFeedback();
  const messages: string[] = [];
  const publish = withWorkbenchFeedbackInvalidation(feedback, (value: string) => messages.push(value));
  publish("saved");
  const old = feedback.begin();
  publish("saved");
  assert.equal(old(), false);
  assert.deepEqual(messages, ["saved", "saved"]);
});

test("manual selection ABA cannot revive an old feedback owner", () => {
  const feedback = createWorkbenchFeedback();
  let selection = "a";
  const select = withWorkbenchFeedbackInvalidation(feedback, (value: string) => { selection = value; });
  const old = feedback.begin();
  select("b");
  select("a");
  assert.equal(selection, "a");
  assert.equal(old(), false);
});

test("automatic reconciliation does not need to invalidate feedback", () => {
  const feedback = createWorkbenchFeedback();
  const old = feedback.begin();
  let selection: string | null = "a";
  const rawSelect = (value: string | null) => { selection = value; };
  rawSelect(null);
  assert.equal(selection, null);
  assert.equal(old(), true);
});

test("functional setters are forwarded without executing the updater eagerly", () => {
  const feedback = createWorkbenchFeedback();
  const update = (value: string) => `${value}!`;
  let received: string | typeof update = "";
  const setter = withWorkbenchFeedbackInvalidation(feedback, (next: string | typeof update) => { received = next; });
  const old = feedback.begin();
  setter(update);
  assert.equal(received, update);
  assert.equal(old(), false);
});

test("disposal and remount never restore an old feedback owner", () => {
  const feedback = createWorkbenchFeedback();
  const old = feedback.begin();
  feedback.dispose();
  assert.equal(old(), false);
  feedback.mount();
  assert.equal(old(), false);
  assert.equal(feedback.begin()(), true);
});

test("feedback revisions are independent between workspaces", () => {
  const a = createWorkbenchFeedback();
  const b = createWorkbenchFeedback();
  const ownsA = a.begin();
  const ownsB = b.begin();
  a.invalidate();
  assert.equal(ownsA(), false);
  assert.equal(ownsB(), true);
});
