import test from "node:test";
import assert from "node:assert/strict";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { persistWorkbenchApiBaseUrl } from "@/lib/api/backend-target";
import { persistWorkbenchSettings, type WorkbenchSettingsInput } from "@/lib/workbench/helpers";
import { writeInMemoryWorkbenchSecrets } from "@/lib/workbench/workbench-secrets";
import { clearWorkbenchStorageBucket } from "@/components/workbench/system/workbench-system-storage";
import { buildWorkbenchGovernedAuthHeaders } from "@/lib/workbench/governance";

const inspect = "/api/direct-mesh/tasks/inspect";
const settings = (overrides: Partial<WorkbenchSettingsInput> = {}): WorkbenchSettingsInput => ({
  theme: "graphite", language: "en", showShortcutHints: true, immersiveGuardrails: true,
  frontendRuntimeMode: "direct_mesh_gui", directMeshEndpointsText: "127.0.0.1:5001",
  directMeshSelectionMode: "healthiest", controlPlaneApiToken: "", clusterApiToken: "", directMeshApiToken: "",
  assistantMode: "local", assistantApiBaseUrl: "", assistantApiKey: "", assistantModel: "", ...overrides,
});

function fixture(run: (storage: Storage, location: { search: string }) => void) {
  const previousWindow = globalThis.window;
  const entries = new Map<string, string>();
  const storage = {
    getItem: (key: string) => entries.get(key) ?? null,
    setItem: (key: string, value: string) => { entries.set(key, value); },
    removeItem: (key: string) => { entries.delete(key); },
    clear: () => entries.clear(),
    key: (index: number) => [...entries.keys()][index] ?? null,
    get length() { return entries.size; },
  };
  const location = { search: "" };
  globalThis.window = { localStorage: storage, sessionStorage: { removeItem() {} }, location } as unknown as Window & typeof globalThis;
  try {
    writeInMemoryWorkbenchSecrets({});
    assert.equal(persistWorkbenchSettings(settings()), true);
    run(storage, location);
  } finally { writeInMemoryWorkbenchSecrets({}); globalThis.window = previousWindow; }
}

test("managed token-free mode roundtrip retires a request without an intermediate authority check", () => fixture(() => {
  const current = captureWorkbenchApiAuthority(inspect);
  assert.equal(persistWorkbenchSettings(settings({ frontendRuntimeMode: "orchestrated_gui" })), true);
  assert.equal(persistWorkbenchSettings(settings()), true);
  assert.equal(current(), false);
  assert.equal(captureWorkbenchApiAuthority(inspect)(), true);
}));

test("managed gateway roundtrip retires a request without an intermediate authority check", () => fixture(() => {
  persistWorkbenchApiBaseUrl("https://original.invalid");
  const current = captureWorkbenchApiAuthority(inspect);
  persistWorkbenchApiBaseUrl("https://other.invalid");
  persistWorkbenchApiBaseUrl("https://original.invalid");
  assert.equal(current(), false);
  assert.equal(captureWorkbenchApiAuthority(inspect)(), true);
}));

for (const [key, mode, url] of [
  ["directMeshApiToken", "direct_mesh_gui", inspect],
  ["controlPlaneApiToken", "orchestrated_gui", "/api/v1/jobs"],
  ["clusterApiToken", "orchestrated_gui", "/api/v1/agents/register"],
] as const) {
  for (const initial of ["", "original-token"]) {
    test(`managed ${key} roundtrip from ${initial ? "configured" : "empty"} invalidates the old request`, () => fixture(() => {
      persistWorkbenchSettings(settings({ frontendRuntimeMode: mode, [key]: initial }));
      const current = captureWorkbenchApiAuthority(url);
      writeInMemoryWorkbenchSecrets({ [key]: "replacement-token" });
      writeInMemoryWorkbenchSecrets({ [key]: initial });
      assert.equal(current(), false);
      assert.equal(captureWorkbenchApiAuthority(url)(), true);
    }));
  }
}

test("theme, language, equivalent writes and irrelevant credentials do not retire a direct request", () => fixture(storage => {
  persistWorkbenchSettings(settings({ directMeshApiToken: "mesh" }));
  const current = captureWorkbenchApiAuthority(inspect);
  persistWorkbenchSettings(settings({ theme: "linen", language: "zh", directMeshApiToken: " mesh ",
    controlPlaneApiToken: "unused-control", assistantApiKey: "unused-assistant" }));
  assert.equal(current(), true);
  assert.doesNotMatch(JSON.stringify([...Array(storage.length)].map((_, index) => storage.getItem(storage.key(index)!))), /unused-control|unused-assistant|"mesh"/);
}));

test("query-pinned and absolute requests ignore changes to a masked persisted gateway", () => fixture((_storage, location) => {
  location.search = "?apiBaseUrl=https%3A%2F%2Fquery.invalid";
  const query = captureWorkbenchApiAuthority(inspect);
  persistWorkbenchApiBaseUrl("https://unused.invalid");
  persistWorkbenchApiBaseUrl("");
  assert.equal(query(), true);
  location.search = "";
  const absolute = captureWorkbenchApiAuthority("https://fixed.invalid/api/v1/jobs");
  persistWorkbenchApiBaseUrl("https://other.invalid");
  assert.equal(absolute(), true);
}));

test("a configured cluster token masks control-plane rotation but its own removal still invalidates", () => fixture(() => {
  persistWorkbenchSettings(settings({ frontendRuntimeMode: "orchestrated_gui", clusterApiToken: "cluster", controlPlaneApiToken: "control" }));
  const current = captureWorkbenchApiAuthority("/api/v1/agents/register");
  writeInMemoryWorkbenchSecrets({ clusterApiToken: "cluster", controlPlaneApiToken: "unused-rotation" });
  assert.equal(current(), true);
  writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "unused-rotation" });
  writeInMemoryWorkbenchSecrets({ clusterApiToken: "cluster", controlPlaneApiToken: "unused-rotation" });
  assert.equal(current(), false);
}));

test("clearing managed settings then restoring them cannot revive an old direct request", () => fixture(() => {
  const current = captureWorkbenchApiAuthority(inspect);
  clearWorkbenchStorageBucket("settings");
  persistWorkbenchSettings(settings());
  assert.equal(current(), false);
}));

for (const partial of [false, true]) {
  test(`settings write failure with partial=${partial} tracks only the actually changed authority`, () => fixture(storage => {
    const current = captureWorkbenchApiAuthority(inspect);
    const set = storage.setItem;
    storage.setItem = (key, value) => {
      if (partial) set(key, value);
      throw new Error("injected storage failure");
    };
    assert.equal(persistWorkbenchSettings(settings({ frontendRuntimeMode: "orchestrated_gui" })), false);
    storage.setItem = set;
    persistWorkbenchSettings(settings());
    assert.equal(current(), !partial);
  }));

  test(`gateway write failure with partial=${partial} tracks only the actually changed authority`, () => fixture(storage => {
    const current = captureWorkbenchApiAuthority(inspect);
    const set = storage.setItem;
    storage.setItem = (key, value) => {
      if (partial) set(key, value);
      throw new Error("injected storage failure");
    };
    assert.throws(() => persistWorkbenchApiBaseUrl("https://other.invalid"), /injected storage failure/);
    storage.setItem = set;
    persistWorkbenchApiBaseUrl("");
    assert.equal(current(), !partial);
  }));
}

test("shared authority dependencies preserve governed header routing and cluster fallback", () => {
  const routes = [
    [inspect, "direct"], ["/api/health", "control"], ["/api/v1/jobs", "control"],
    ["/api/playground/run", "control"], ["/api/v1/agents/register", "cluster"],
    ["/api/v1/agents/a/heartbeat", "cluster"], ["/api/v1/agents/a", "cluster"],
    ["/api/unrelated", "none"], ["https://other.invalid/api/v1/jobs", "none"],
  ];
  for (const frontendRuntimeMode of ["direct_mesh_gui", "orchestrated_gui"] as const) {
    for (const clusterApiToken of ["cluster", ""]) {
      for (const [url, kind] of routes) {
        const token = kind === "direct" ? (frontendRuntimeMode === "direct_mesh_gui" ? "mesh" : "")
          : frontendRuntimeMode === "orchestrated_gui" ? (kind === "cluster" ? clusterApiToken || "control" : kind === "control" ? "control" : "") : "";
        assert.deepEqual(buildWorkbenchGovernedAuthHeaders({ url, frontendRuntimeMode,
          secrets: { clusterApiToken, controlPlaneApiToken: " control ", directMeshApiToken: " mesh " },
        }), token ? { "x-kyuubiki-token": token } : {}, `${frontendRuntimeMode} ${url}`);
      }
    }
  }
});
