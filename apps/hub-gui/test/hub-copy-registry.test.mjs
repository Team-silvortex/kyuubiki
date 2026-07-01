import test from "node:test";
import assert from "node:assert/strict";

import {
  HUB_COPY_OVERRIDE_STORAGE_KEY,
  importHubCopyPayload,
  resolveHubCopy,
} from "../ui/hub-copy-registry.js";

class MemoryStorage {
  values = new Map();

  getItem(key) {
    return this.values.get(key) ?? null;
  }

  setItem(key, value) {
    this.values.set(key, value);
  }

  removeItem(key) {
    this.values.delete(key);
  }
}

test("hub language-pack overrides cannot flatten required copy branches", () => {
  const storage = new MemoryStorage();
  importHubCopyPayload(
    {
      schema_version: "kyuubiki.language-pack/v1",
      id: "zh-broken-shape",
      language: "zh",
      targetSurface: "hub",
      name: "Broken shape pack",
      version: "1.14.0",
      source: "imported",
      updatedAt: "2026-06-24T00:00:00.000Z",
      overrides: {
        shell: "broken shell",
        nav: {
          projects: "项目",
        },
        custom: {
          label: "future extension",
        },
      },
    },
    storage,
  );

  const copy = resolveHubCopy(
    {
      en: { shell: { language: "Language" }, nav: { projects: "Projects" } },
      zh: { shell: { language: "语言" }, nav: { projects: "项目旧" } },
    },
    "zh",
    { storage },
  );

  assert.deepEqual(copy.shell, { language: "语言" });
  assert.equal(copy.nav.projects, "项目");
  assert.deepEqual(copy.custom, { label: "future extension" });
});

test("hub registry import recovers from malformed stored override JSON", () => {
  const storage = new MemoryStorage();
  storage.setItem(HUB_COPY_OVERRIDE_STORAGE_KEY, "{not-json");

  const copy = resolveHubCopy(
    {
      en: { shell: { language: "Language" } },
    },
    "en",
    { storage },
  );

  assert.equal(copy.shell.language, "Language");
});

test("hub language-pack import rejects packs targeted at another surface", () => {
  const storage = new MemoryStorage();

  assert.throws(
    () =>
      importHubCopyPayload(
        {
          schema_version: "kyuubiki.language-pack/v1",
          id: "zh-workbench-pack",
          language: "zh",
          targetSurface: "workbench",
          name: "Workbench pack",
          version: "1.14.0",
          source: "imported",
          updatedAt: "2026-06-24T00:00:00.000Z",
          overrides: {
            nav: {
              projects: "项目",
            },
          },
        },
        storage,
      ),
    /invalid-hub-copy-pack/,
  );
});

test("hub pack-only languages use fallback structure with requested language overrides", () => {
  const storage = new MemoryStorage();
  importHubCopyPayload(
    {
      schema_version: "kyuubiki.language-pack/v1",
      id: "fr-hub-pack",
      language: "fr",
      targetSurface: "hub",
      name: "French hub pack",
      version: "1.14.0",
      source: "imported",
      updatedAt: "2026-06-24T00:00:00.000Z",
      overrides: {
        shell: {
          language: "Langue",
        },
      },
    },
    storage,
  );

  const copy = resolveHubCopy(
    {
      en: { shell: { language: "Language", idle: "idle" } },
    },
    "fr",
    { storage },
  );

  assert.deepEqual(copy.shell, { language: "Langue", idle: "idle" });
});
