import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const CONTRACT_PATH = "config/architecture/desktop-capability-closure.json";
const contract = JSON.parse(readFileSync(path.join(ROOT, CONTRACT_PATH), "utf8"));

function walkFiles(directory, extension) {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...walkFiles(entryPath, extension));
    else if (entry.name.endsWith(extension)) files.push(entryPath);
  }
  return files;
}

function read(relativePath) {
  return readFileSync(path.join(ROOT, relativePath), "utf8");
}

function appSources(app) {
  const root = path.join(ROOT, "apps", app);
  return {
    ui: walkFiles(path.join(root, "ui"), ".js")
      .concat(walkFiles(path.join(root, "ui"), ".html"))
      .map((file) => readFileSync(file, "utf8"))
      .join("\n"),
    rust: walkFiles(path.join(root, "src-tauri/src"), ".rs")
      .map((file) => readFileSync(file, "utf8"))
      .join("\n"),
  };
}

function expectTokens(source, tokens, label) {
  for (const token of tokens || []) {
    assert.ok(source.includes(token), `${label} is missing evidence token: ${token}`);
  }
}

function hasExecutableAssistantRoute(source, action) {
  const escaped = action.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  return new RegExp(`(?:case\\s+|^\\s*)["']${escaped}["']\\s*:`, "mu").test(source);
}

test("desktop capability closure contract has unique, complete coordinates", () => {
  assert.equal(contract.schema_version, "kyuubiki.desktop-capability-closure/v1");
  assert.ok(contract.capabilities.length >= 20, "major desktop capability set must not shrink silently");
  const ids = contract.capabilities.map((capability) => capability.id);
  assert.equal(new Set(ids).size, ids.length, "capability ids must be unique");

  for (const capability of contract.capabilities) {
    assert.ok(capability.app, `${capability.id} must declare an app`);
    assert.ok(capability.ui_action, `${capability.id} must declare a UI action`);
    assert.ok(capability.route_file, `${capability.id} must declare route evidence`);
    assert.ok(capability.native_command, `${capability.id} must declare a native command`);
    assert.ok(capability.native_file, `${capability.id} must declare native evidence`);
    assert.ok(capability.route_tokens?.length, `${capability.id} must declare observable route/outcome evidence`);
    assert.ok(capability.native_tokens?.length, `${capability.id} must declare implementation evidence`);
  }
});

test("every required desktop capability closes UI, route, native, and result layers", () => {
  const sourcesByApp = new Map();

  for (const capability of contract.capabilities) {
    const sources = sourcesByApp.get(capability.app) || appSources(capability.app);
    sourcesByApp.set(capability.app, sources);
    assert.ok(
      sources.ui.includes(`data-action="${capability.ui_action}"`),
      `${capability.id} has no visible/declarative UI entry`,
    );

    const routeSource = read(capability.route_file);
    expectTokens(routeSource, capability.route_tokens, `${capability.id} route`);
    assert.ok(
      sources.rust.includes(capability.native_command),
      `${capability.id} native command is absent from the app backend`,
    );
    if (capability.native_action) {
      assert.ok(
        sources.rust.includes(`"${capability.native_action}"`),
        `${capability.id} guarded native operation is absent`,
      );
    }
    expectTokens(read(capability.native_file), capability.native_tokens, `${capability.id} native`);
  }
});

test("capabilities that promise PWDT parity are catalogued and executable", () => {
  const catalog = read("apps/hub-gui/ui/hub-app-config.js");
  const executor = read("apps/hub-gui/ui/hub-assistant-engine.js");

  for (const capability of contract.capabilities.filter((entry) => entry.automation_action)) {
    assert.ok(
      catalog.includes(`"${capability.automation_action}"`),
      `${capability.id} is missing from the PWDT action catalog`,
    );
    assert.ok(
      hasExecutableAssistantRoute(executor, capability.automation_action),
      `${capability.id} has no PWDT execution branch or route entry`,
    );
  }
});
