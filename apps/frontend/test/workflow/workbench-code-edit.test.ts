import test from "node:test";
import assert from "node:assert/strict";
import { indentWorkbenchCode, breakWorkbenchCodeLine } from "../../src/components/workbench/workbench-code-edit.ts";
import { getWorkbenchScriptWorkspaceCopy } from "../../src/components/workbench/workbench-script-workspace-copy.ts";

test("PWDT indent follows four-column stops and handles a leading newline", () => {
  assert.deepEqual(indentWorkbenchCode({ value: "a", start: 1, end: 1 }), { value: "a   ", start: 4, end: 4 });
  assert.deepEqual(indentWorkbenchCode({ value: "\na", start: 0, end: 0 }), { value: "    \na", start: 4, end: 4 });
});

test("PWDT block indent excludes a line selected only at its boundary and round-trips", () => {
  const before = { value: "alpha\nbeta\ngamma", start: 0, end: 11 };
  const after = indentWorkbenchCode(before);
  assert.deepEqual(after, { value: "    alpha\n    beta\ngamma", start: 4, end: 19 });
  assert.deepEqual(indentWorkbenchCode(after, true), before);
});

test("PWDT outdent handles partial spaces and tabs without deleting code", () => {
  assert.deepEqual(indentWorkbenchCode({ value: "  a\n\tb", start: 0, end: 6 }, true), { value: "a\nb", start: 0, end: 3 });
  assert.deepEqual(indentWorkbenchCode({ value: "a", start: 0, end: 0 }, true), { value: "a", start: 0, end: 0 });
});

test("PWDT newline preserves indentation and adds a Python block indent only in Python", () => {
  const before = { value: "    if ready:", start: 13, end: 13 };
  assert.deepEqual(breakWorkbenchCodeLine(before, true), { value: "    if ready:\n        ", start: 22, end: 22 });
  assert.deepEqual(breakWorkbenchCodeLine(before, false), { value: "    if ready:\n    ", start: 18, end: 18 });
  assert.deepEqual(breakWorkbenchCodeLine({ value: "\nx", start: 0, end: 1 }, true), { value: "\nx", start: 1, end: 1 });
});

test("PWDT workspace navigation and keyboard help exist for all selectable languages", () => {
  const languages = "en zh ja es ar bn cs da de el fa fi fr he hi id it ko ms nl no pl pt-BR ro ru sv sw ta th tr uk ur vi zh-TW".split(" ");
  const en = getWorkbenchScriptWorkspaceCopy("en");
  for (const language of languages) {
    const copy = getWorkbenchScriptWorkspaceCopy(language);
    assert.deepEqual(Object.keys(copy), Object.keys(en));
    for (const [key, value] of Object.entries(copy)) {
      assert.ok(value.trim(), `${language}:${key}`);
      if (language !== "en") assert.notEqual(copy, en, `${language} must not fall back to English`);
    }
  }
  assert.equal(getWorkbenchScriptWorkspaceCopy("pt-br"), getWorkbenchScriptWorkspaceCopy("pt-BR"));
  assert.equal(getWorkbenchScriptWorkspaceCopy("constructor"), en);
});
