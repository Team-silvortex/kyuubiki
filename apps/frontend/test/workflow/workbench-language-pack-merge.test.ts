import test from "node:test";
import assert from "node:assert/strict";

import { mergeLanguagePack } from "../../src/lib/workbench/language-pack-merge.ts";

test("mergeLanguagePack preserves existing object branches from invalid leaf overrides", () => {
  const copy = mergeLanguagePack(
    {
      shell: {
        language: "Language",
        idle: "Idle",
      },
      title: "Workbench",
    },
    {
      shell: "broken shell",
      title: { nested: "broken title" },
      custom: { label: "future extension" },
    },
  );

  assert.deepEqual(copy.shell, {
    language: "Language",
    idle: "Idle",
  });
  assert.equal(copy.title, "Workbench");
  assert.deepEqual((copy as Record<string, unknown>).custom, { label: "future extension" });
});

test("mergeLanguagePack still accepts same-shape leaf and nested object overrides", () => {
  const copy = mergeLanguagePack(
    {
      shell: {
        language: "Language",
        shortcuts: ["A", "B"],
      },
      title: "Workbench",
    },
    {
      shell: {
        language: "Idioma",
        shortcuts: ["C"],
      },
      title: "Banco",
    },
  );

  assert.equal(copy.shell.language, "Idioma");
  assert.deepEqual(copy.shell.shortcuts, ["C"]);
  assert.equal(copy.title, "Banco");
});
