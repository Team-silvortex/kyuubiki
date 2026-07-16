import test from "node:test";
import assert from "node:assert/strict";
import type { SetStateAction } from "react";

import { installWorkbenchLanguagePackPayload } from "../../src/components/workbench/workbench-language-pack-controller.ts";
import type { WorkbenchLanguagePack } from "../../src/lib/workbench/helpers.ts";

function buildInstallHarness() {
  let packs: WorkbenchLanguagePack[] = [];
  let message = "";
  return {
    get message() {
      return message;
    },
    get packs() {
      return packs;
    },
    setLanguagePacks(updater: SetStateAction<WorkbenchLanguagePack[]>) {
      packs = typeof updater === "function" ? updater(packs) : updater;
    },
    setMessage(value: string) {
      message = value;
    },
  };
}

test("workbench language pack import rejects unsafe UI text before state install", () => {
  for (const payload of [
    { overrides: { title: "<script>alert(1)</script>" } },
    { overrides: { title: "javascript:alert(1)" } },
    { overrides: { title: "onclick=steal()" } },
    { overrides: { title: "localStorage.token" } },
  ]) {
    const harness = buildInstallHarness();
    installWorkbenchLanguagePackPayload({
      raw: {
        id: "hostile-pack",
        language: "en",
        name: "Hostile pack",
        targetSurface: "workbench",
        ...payload,
      },
      language: "en",
      setLanguagePacks: harness.setLanguagePacks,
      setMessage: harness.setMessage,
    });

    assert.deepEqual(harness.packs, []);
    assert.match(harness.message, /unsafe UI text/);
  }
});

test("workbench language pack import still installs safe overrides", () => {
  const harness = buildInstallHarness();
  installWorkbenchLanguagePackPayload({
    raw: {
      id: "safe-pack",
      language: "en",
      name: "Safe pack",
      targetSurface: "workbench",
      overrides: { workflowCatalogTitle: "Workflow catalog" },
    },
    language: "en",
    setLanguagePacks: harness.setLanguagePacks,
    setMessage: harness.setMessage,
  });

  assert.equal(harness.packs.length, 1);
  assert.equal(harness.packs[0]?.id, "safe-pack");
  assert.match(harness.message, /Language pack imported/);
});
