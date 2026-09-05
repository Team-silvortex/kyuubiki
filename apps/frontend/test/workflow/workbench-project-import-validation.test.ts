import test from "node:test";
import assert from "node:assert/strict";
import JSZip from "jszip";

import { bindWorkbenchWorkspaceState } from "../../src/components/workbench/workbench-shell-bindings.ts";
import {
  defaultProjectFileManifest,
  parseProjectBundleFile,
  parseProjectBundleJson,
  PROJECT_SCHEMA_VERSION,
} from "../../src/lib/projects/project-format.ts";
import type { ProjectBundle } from "../../src/lib/projects/project-format.ts";

function bundle(): ProjectBundle {
  const record = {
    project_id: "project-a", model_id: "model-a", name: "Model A", kind: "truss_2d",
    model_schema_version: "kyuubiki.model/v1", payload: {}, inserted_at: "", updated_at: "",
  };
  return {
    project_schema_version: PROJECT_SCHEMA_VERSION,
    project: { project_id: "project-a", name: "Project A", inserted_at: "", updated_at: "" },
    models: [record],
    model_versions: [{ ...record, version_id: "version-a", version_number: 1 }],
  };
}

test("workspace binding preserves the import notice state and setter", () => {
  const notice = { id: "import", message: "import failed" };
  const setImportNotice = () => {};
  const bound = bindWorkbenchWorkspaceState({ importNotice: notice, setImportNotice });
  assert.equal(bound.importNotice, notice);
  assert.equal(bound.setImportNotice, setImportNotice);
});

test("valid project records round-trip through section validation", () => {
  const source = bundle();
  const parsed = parseProjectBundleJson(JSON.stringify(source));
  assert.deepEqual(parsed.models, source.models);
  assert.deepEqual(parsed.model_versions, source.model_versions);
});

for (const invalid of [null, [], 1, "bundle"]) {
  test(`project parser rejects non-record root ${JSON.stringify(invalid)}`, () => {
    assert.throws(() => parseProjectBundleJson(JSON.stringify(invalid)), /project bundle must be an object/u);
  });
}

const invalidSections: Array<[string, (source: ProjectBundle) => unknown]> = [
  ["models object", (source) => ({ ...source, models: {} })],
  ["versions object", (source) => ({ ...source, model_versions: {} })],
  ["null model", (source) => ({ ...source, models: [null] })],
  ["duplicate model", (source) => ({ ...source, models: [...source.models, ...source.models] })],
  ["foreign model", (source) => ({ ...source, models: [{ ...source.models[0], project_id: "foreign" }] })],
  ["invalid model payload", (source) => ({ ...source, models: [{ ...source.models[0], payload: [] }] })],
  ["orphan version", (source) => ({ ...source, model_versions: [{ ...source.model_versions[0], model_id: "missing" }] })],
  ["duplicate version", (source) => ({ ...source, model_versions: [...source.model_versions, ...source.model_versions] })],
  ["invalid version number", (source) => ({ ...source, model_versions: [{ ...source.model_versions[0], version_number: 0 }] })],
  ["invalid snapshot", (source) => ({ ...source, workspace_snapshot: [] })],
  ["invalid preset list", (source) => ({ ...source, automation_presets: {} })],
];

for (const [name, corrupt] of invalidSections) {
  test(`project parser rejects ${name} before import`, () => {
    assert.throws(() => parseProjectBundleJson(JSON.stringify(corrupt(bundle()))), /project bundle/u);
  });
}

test("archive sidecar sections are validated after hydration", async () => {
  const source = bundle();
  const paths = defaultProjectFileManifest();
  const zip = new JSZip();
  zip.file("project.json", JSON.stringify(source));
  zip.file(paths.automation_presets_path, JSON.stringify({ invalid: "not an array" }));
  const bytes = await zip.generateAsync({ type: "uint8array" });
  const file = new File([new Uint8Array(bytes).buffer], "invalid.kyuubiki");
  await assert.rejects(parseProjectBundleFile(file), /automation_presets must be an array/u);
});
