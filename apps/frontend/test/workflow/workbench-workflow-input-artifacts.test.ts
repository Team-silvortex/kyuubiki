import assert from "node:assert/strict";
import test from "node:test";

import type { WorkflowCatalogEntryArtifact } from "../../src/lib/api/workflow-types.ts";
import {
  buildWorkflowInputArtifactTexts,
  parseWorkflowInputArtifactTexts,
} from "../../src/components/workbench/workflow/workbench-workflow-input-artifacts.ts";

function entry(nodeId: string): WorkflowCatalogEntryArtifact {
  return { node_id: nodeId, artifact_type: "artifact/json", description: nodeId };
}

test("draft parsing requires newly declared inputs before their editor is mounted", () => {
  const result = parseWorkflowInputArtifactTexts({ model: "{}" }, [entry("model"), entry("load")]);
  assert.deepEqual(result, { inputArtifacts: { model: {} }, invalidKeys: ["load"] });
});

test("draft parsing ignores removed input text and never submits stale artifacts", () => {
  const texts = { model: "{\"value\":1}", removed: "invalid", old: "{\"secret\":true}" };
  assert.deepEqual(parseWorkflowInputArtifactTexts(texts, [entry("model")]), {
    inputArtifacts: { model: { value: 1 } }, invalidKeys: [],
  });
  assert.equal(texts.removed, "invalid", "checking readiness must not destroy recoverable editor text");
  assert.deepEqual(parseWorkflowInputArtifactTexts(texts, []), { inputArtifacts: {}, invalidKeys: [] });
});

test("draft parsing reports missing, blank and malformed input keys in contract order", () => {
  assert.deepEqual(parseWorkflowInputArtifactTexts(
    { blank: "  \n", malformed: "{", ok: "false" },
    [entry("missing"), entry("blank"), entry("malformed"), entry("ok"), entry("missing")],
  ), { inputArtifacts: { ok: false }, invalidKeys: ["missing", "blank", "malformed"] });
});

test("draft input parsing retains the legacy all-text mode for existing callers", () => {
  assert.deepEqual(parseWorkflowInputArtifactTexts({ a: "0", b: "null", c: "[1,2]", d: "" }), {
    inputArtifacts: { a: 0, b: null, c: [1, 2] }, invalidKeys: ["d"],
  });
});

test("prototype-named input IDs cannot bypass required input checks", () => {
  assert.deepEqual(parseWorkflowInputArtifactTexts({}, [entry("constructor"), entry("__proto__")]), {
    inputArtifacts: {}, invalidKeys: ["constructor", "__proto__"],
  });
});

test("prototype-named input IDs round-trip as own JSON properties", () => {
  const values = JSON.parse('{"__proto__":{"value":1},"constructor":2}') as Record<string, unknown>;
  const entries = [entry("__proto__"), entry("constructor")];
  const result = parseWorkflowInputArtifactTexts(buildWorkflowInputArtifactTexts(entries, values), entries);
  assert.deepEqual(result.invalidKeys, []);
  assert.equal(Object.getPrototypeOf(result.inputArtifacts), Object.prototype);
  assert.deepEqual(JSON.parse(JSON.stringify(result.inputArtifacts)), values);
});

test("correcting a required input clears its blocker without changing the contract", () => {
  const entries = [entry("model"), entry("load")];
  const texts = buildWorkflowInputArtifactTexts(entries, { model: { nodes: [1] } });
  assert.deepEqual(parseWorkflowInputArtifactTexts(texts, entries).invalidKeys, ["load"]);
  assert.deepEqual(parseWorkflowInputArtifactTexts({ ...texts, load: "[2,3]" }, entries), {
    inputArtifacts: { model: { nodes: [1] }, load: [2, 3] }, invalidKeys: [],
  });
  assert.equal(entries.length, 2);
});
