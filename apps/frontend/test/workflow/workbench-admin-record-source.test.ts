import test from "node:test";
import assert from "node:assert/strict";
import { createWorkbenchAdminRecordSources, isWorkbenchAdminRecordSourceCurrent } from "../../src/components/workbench/workbench-admin-record-source.ts";
import { getWorkbenchAdminRecordSourceChangedLabel } from "../../src/components/workbench/workbench-admin-record-source-copy.ts";
import { buildWorkbenchLanguageOptions } from "../../src/components/workbench/workbench-language-options.ts";

function fixture() {
  let authority = "a";
  const sources = createWorkbenchAdminRecordSources(() => {
    const captured = authority;
    return () => captured === authority;
  });
  return { sources, change: (next: string) => { authority = next; } };
}

test("verified collection metadata preserves payload identity and serialized JSON", () => {
  const { sources } = fixture();
  const records = [{ job_id: "same", result: { note: "unchanged" } }];
  const json = JSON.stringify(records);
  sources.mark(records);
  assert.equal(JSON.stringify(records), json);
  assert.deepEqual(Object.keys(records[0]), ["job_id", "result"]);
  assert.equal(sources.sourceOf(records)?.isCurrent(), true);
  assert.equal(sources.sourceOf([records[0]]), null, "a copied collection is not automatically verified");
});

test("same-authority job and result collection reads retain their shared source identity", () => {
  const { sources } = fixture();
  const jobs = [{ job_id: "same" }], results = [{ job_id: "same", result: {} }];
  sources.mark(jobs);
  const original = sources.sourceOf(jobs);
  sources.mark(results);
  sources.mark([{ job_id: "refreshed" }]);
  assert.equal(sources.sourceOf(results), original);
  assert.equal(isWorkbenchAdminRecordSourceCurrent(original, original), true);
});

test("new-authority same-ID records cannot authorize the previous authority's draft", () => {
  const { sources, change } = fixture();
  const a = [{ job_id: "same" }], b = [{ job_id: "same" }];
  sources.mark(a);
  const draft = sources.sourceOf(a);
  change("b");
  assert.equal(isWorkbenchAdminRecordSourceCurrent(draft, draft), false);
  sources.mark(b);
  assert.notEqual(sources.sourceOf(b), draft);
  assert.equal(isWorkbenchAdminRecordSourceCurrent(sources.sourceOf(b), draft), false);
  assert.equal(isWorkbenchAdminRecordSourceCurrent(sources.sourceOf(b), sources.sourceOf(b)), true);
});

test("source-preserving deletion filters never retag an old collection as the current backend", () => {
  const { sources, change } = fixture();
  const records = [{ job_id: "deleted" }, { job_id: "retained" }];
  sources.mark(records);
  const source = sources.sourceOf(records);
  change("b");
  const filtered = sources.filter(records, (record) => record.job_id !== "deleted");
  assert.equal(filtered[0], records[1]);
  assert.equal(sources.sourceOf(filtered), source);
  assert.equal(source?.isCurrent(), false);
  assert.equal(sources.sourceOf(sources.filter([{ job_id: "unknown" }], () => true)), null);
});

test("empty verified reads retain source metadata but do not create a writable record", () => {
  const { sources } = fixture();
  const records: object[] = [];
  sources.mark(records);
  assert.equal(sources.sourceOf(records)?.isCurrent(), true);
  assert.equal(sources.sourceOf(null), null);
  assert.equal(sources.sourceOf(undefined), null);
  assert.equal(isWorkbenchAdminRecordSourceCurrent(null, sources.sourceOf(records)), false);
});

test("source-preserving catalog maps retain the original scope without retagging or changing JSON", () => {
  const { sources, change } = fixture();
  const records = [{ project_id: "known", models: ["removed", "retained"] }];
  sources.mark(records);
  const source = sources.sourceOf(records);
  change("b");
  const next = sources.map(records, (record) => ({ ...record, models: record.models.filter((id) => id !== "removed") }));
  assert.equal(sources.sourceOf(next), source);
  assert.equal(source?.isCurrent(), false);
  assert.deepEqual(records[0].models, ["removed", "retained"]);
  assert.equal(sources.sourceOf(sources.map([{ project_id: "unknown" }], (record) => record)), null);
  assert.deepEqual(Object.keys(next[0]), ["project_id", "models"]);
});

test("failed scope capture cannot mark a collection as verified", () => {
  const sources = createWorkbenchAdminRecordSources(() => () => false);
  const records = [{ job_id: "unknown" }];
  sources.mark(records);
  assert.equal(sources.sourceOf(records), null);
});

test("unreadable record-source checks fail closed without throwing through UI rendering", () => {
  const source = { isCurrent: () => { throw new Error("source cannot be inspected"); } };
  assert.equal(isWorkbenchAdminRecordSourceCurrent(source, source), false);
});

test("verified authority A to B to A reads do not reuse the first draft source", () => {
  const { sources, change } = fixture();
  const first = [{ job_id: "same" }];
  sources.mark(first);
  const old = sources.sourceOf(first);
  change("b");
  sources.mark([]);
  change("a");
  const current = [{ job_id: "same" }];
  sources.mark(current);
  assert.notEqual(sources.sourceOf(current), old);
  assert.equal(isWorkbenchAdminRecordSourceCurrent(sources.sourceOf(current), old), false);
});

for (const { value: language } of buildWorkbenchLanguageOptions({ copy: {}, languagePacks: [], currentLanguage: "en" })) {
  test(`record source recovery has built-in translated guidance for ${language}`, () => {
    const message = getWorkbenchAdminRecordSourceChangedLabel(language);
    assert.ok(message.length > 20);
    if (language !== "en") assert.notEqual(message, getWorkbenchAdminRecordSourceChangedLabel("en"));
    assert.equal(getWorkbenchAdminRecordSourceChangedLabel(language.toUpperCase()), message);
  });
}

test("unknown language uses the explicit English recovery fallback", () => {
  assert.equal(getWorkbenchAdminRecordSourceChangedLabel("unknown"), getWorkbenchAdminRecordSourceChangedLabel("en"));
});
