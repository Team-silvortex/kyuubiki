import test from "node:test";
import assert from "node:assert/strict";
import { bundleCreationTarget } from "../ui/hub-bundle-creation.js";
import { BUNDLE_CREATION_TRANSLATIONS, bundleCreationCopy } from "../ui/hub-bundle-creation-copy.js";
import { DESKTOP_LANGUAGE_LABELS } from "../../desktop-shared/ui/language-pack-loader.js";
import { read } from "./smoke-fixtures.mjs";
import { runProjectBundleAction } from "../ui/hub-project-bundles.js";

test("bundle creation previews POSIX, Windows, UNC, Unicode and extension-safe targets", () => {
  for (const [directory, name, path] of [
    ["/tmp", "Research", "/tmp/Research.kyuubiki"],
    ["/", "Research", "/Research.kyuubiki"],
    ["/tmp/", "材料研究", "/tmp/材料研究.kyuubiki"],
    ["/tmp/Research ", "Study", "/tmp/Research /Study.kyuubiki"],
    ["/tmp/with\\backslash", "Study", "/tmp/with\\backslash/Study.kyuubiki"],
    ["/tmp/ends\\", "Study", "/tmp/ends\\/Study.kyuubiki"],
    ["/tmp", " sample.KYUUBIKI ", "/tmp/sample.kyuubiki"],
    ["C:\\", "Research", "C:\\Research.kyuubiki"],
    ["C:\\Users\\Research", "Thermal", "C:\\Users\\Research\\Thermal.kyuubiki"],
    ["C:/Research/", "Heat", "C:/Research/Heat.kyuubiki"],
    ["\\\\server\\share\\", "Study", "\\\\server\\share\\Study.kyuubiki"],
  ]) {
    const result = bundleCreationTarget(directory, name);
    assert.equal(result.valid, true, `${directory} / ${name}`);
    assert.equal(result.payload.path, path);
  }
});

test("invalid names and relative paths cannot enable a bundle creation request", () => {
  for (const name of ["", ".", "..", "../escape", "sub/name", "sub\\name", "bad:name", "CON", "con.txt", "NUL", "COM1", "LPT9",
    "bad?", "bad*", 'bad"', "bad<", "bad>", "bad|", "bad\nname", "bad\0name", "trailing.", "space .kyuubiki", ".kyuubiki", "x".repeat(201), "研".repeat(67)]) {
    assert.deepEqual(bundleCreationTarget("/tmp", name), { valid: false, field: "name" }, name);
  }
  for (const directory of ["", "relative/path", "~/Desktop", "C:relative", "https://example.com", "\\single", "\\\\server", "/tmp\0"]) {
    assert.deepEqual(bundleCreationTarget(directory, "Research"), { valid: false, field: "directory" }, directory);
  }
});

test("new bundle copy covers every selectable desktop language", () => {
  for (const language of Object.keys(DESKTOP_LANGUAGE_LABELS)) {
    assert.ok(BUNDLE_CREATION_TRANSLATIONS[language], language);
    assert.equal(Object.values(bundleCreationCopy(language)).length, 13);
    assert.ok(Object.values(bundleCreationCopy(language)).every((value) => typeof value === "string" && value.trim()));
  }
  assert.equal(bundleCreationCopy("zh").folder, "选择文件夹...");
});

test("the native chooser is narrowly scoped and the boot fallback cannot silently create a bundle", () => {
  const permissions = JSON.parse(read("src-tauri/capabilities/main.json")).permissions;
  assert.ok(permissions.includes("allow-project-bundle-pick-path"));
  assert.ok(!permissions.some((entry) => entry.startsWith("fs:") || entry.startsWith("dialog:")));
  assert.match(read("src-tauri/src/hub_path_picker.rs"), /async fn project_bundle_pick_path/);
  assert.match(read("src-tauri/src/hub_path_picker.rs"), /spawn_blocking/);
  assert.doesNotMatch(read("ui/hub-boot-probe.js"), /project_bundle_create/);
});

test("creation history records the attempted target on failure and the native target on success", async () => {
  const entries = [];
  const elements = { projectBundlePath: { value: "/tmp/previous.kyuubiki" } };
  const options = {
    action: "project create", command: "guarded_mutation_action",
    payload: { path: "/tmp/next" }, elements,
    projectActionLabels: { "project create": "create" },
    saveProjectBundleRecents: (entry) => entries.push(entry),
    outputTarget: () => {}, setBusy: () => {},
    successOutput: (result) => {
      elements.projectBundlePath.value = JSON.parse(result).path;
      return result;
    },
  };
  await assert.rejects(runProjectBundleAction({ ...options,
    invokeTauri: async () => { throw new Error("permission denied"); },
  }), /permission denied/);
  assert.equal(elements.projectBundlePath.value, "/tmp/previous.kyuubiki");
  assert.equal(entries[0].bundlePath, "/tmp/next");
  await runProjectBundleAction({ ...options,
    invokeTauri: async () => JSON.stringify({ path: "/tmp/next.kyuubiki" }),
  });
  assert.equal(entries[1].bundlePath, "/tmp/next.kyuubiki");
});
