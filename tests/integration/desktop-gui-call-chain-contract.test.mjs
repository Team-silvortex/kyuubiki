import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const APPS = [
  { name: "Hub", directory: "hub-gui", router: "hub" },
  { name: "Workbench", directory: "workbench-gui", router: "workbench" },
  { name: "Installer", directory: "installer-gui", router: "installer" },
];

function walkFiles(directory, extension) {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkFiles(entryPath, extension));
    } else if (entry.name.endsWith(extension)) {
      files.push(entryPath);
    }
  }
  return files;
}

function readFiles(files) {
  return files.map((file) => readFileSync(file, "utf8")).join("\n");
}

function valuesFrom(source, pattern) {
  return new Set([...source.matchAll(pattern)].map((match) => match[1]));
}

function appSources(app) {
  const appRoot = path.join(ROOT, "apps", app.directory);
  const tauriRoot = path.join(appRoot, "src-tauri");
  const uiFiles = walkFiles(path.join(appRoot, "ui"), ".js");
  const rustFiles = walkFiles(path.join(tauriRoot, "src"), ".rs");
  const permissionFiles = walkFiles(path.join(tauriRoot, "permissions"), ".toml");
  return {
    appRoot,
    tauriRoot,
    ui: readFiles(uiFiles),
    rust: readFiles(rustFiles),
    permissions: readFiles(permissionFiles),
    capability: JSON.parse(readFileSync(path.join(tauriRoot, "capabilities/main.json"), "utf8")),
  };
}

function registeredCommands(rust) {
  const commands = new Set();
  for (const match of rust.matchAll(/generate_handler!\s*\[([\s\S]*?)\]/gu)) {
    for (const value of match[1].split(",").map((entry) => entry.trim()).filter(Boolean)) {
      commands.add(value);
    }
  }
  return commands;
}

function commandPermissions(permissions) {
  const catalog = new Map();
  for (const block of permissions.split("[[permission]]").slice(1)) {
    const identifier = block.match(/identifier\s*=\s*"([^"]+)"/u)?.[1];
    if (!identifier) continue;
    for (const allow of block.matchAll(/commands\.allow\s*=\s*\[([^\]]*)\]/gu)) {
      for (const command of allow[1].matchAll(/"([^"]+)"/gu)) {
        catalog.set(command[1], identifier);
      }
    }
  }
  return catalog;
}

function directInvocations(ui) {
  return new Set(
    [...valuesFrom(ui, /(?:invoke|invokeTauri)\(\s*["']([^"']+)["']/gu)]
      .filter((command) => !command.includes("/")),
  );
}

function routedActions(app, sources) {
  if (app.router === "hub") {
    return valuesFrom(sources.ui, /case\s+["']([^"']+)["']\s*:/gu);
  }
  if (app.router === "workbench") {
    return new Set([
      ...valuesFrom(sources.ui, /action\s*===\s*["']([^"']+)["']/gu),
      ...valuesFrom(sources.ui, /case\s+["']([^"']+)["']\s*:/gu),
    ]);
  }

  const appJs = readFileSync(path.join(sources.appRoot, "ui/app.js"), "utf8");
  const start = appJs.indexOf("const actionHandlers = {");
  const end = appJs.indexOf("\n  };", start);
  assert.ok(start >= 0 && end > start, "Installer actionHandlers object must remain discoverable");
  return new Set(
    [...appJs.slice(start, end).matchAll(/^\s*(?:["']([^"']+)["']|([A-Za-z][\w-]*))\s*:/gmu)]
      .map((match) => match[1] || match[2]),
  );
}

function requestedGuardedActions(app, sources) {
  const actions = valuesFrom(sources.ui, /invokeGuardedMutation\(\s*["']([^"']+)["']/gu);
  if (app.router !== "hub") return actions;

  for (const match of sources.ui.matchAll(
    /action:\s*["']project ([a-z]+)["'][\s\S]{0,160}?command:\s*["']guarded_mutation_action["']/gu,
  )) {
    actions.add(`project_bundle_${match[1]}`);
  }
  return actions;
}

function acceptedGuardedActions(rust) {
  const start = rust.indexOf("fn guarded_mutation_action");
  assert.ok(start >= 0, "guarded_mutation_action must exist");
  const body = rust.slice(start, rust.indexOf("\nfn main()", start));
  return valuesFrom(body, /"([^"]+)"\s*=>/gu);
}

function missingValues(expected, actual) {
  return [...expected].filter((value) => !actual.has(value)).sort();
}

for (const app of APPS) {
  test(`${app.name} closes every direct UI-to-Tauri IPC chain`, () => {
    const sources = appSources(app);
    const invoked = directInvocations(sources.ui);
    const registered = registeredCommands(sources.rust);
    const permissions = commandPermissions(sources.permissions);
    const enabled = new Set(sources.capability.permissions);

    assert.deepEqual(
      missingValues(invoked, registered),
      [],
      "UI invokes commands that are absent from generate_handler!",
    );
    assert.deepEqual(
      [...invoked].filter((command) => !permissions.has(command)).sort(),
      [],
      "UI invokes commands without a Tauri permission declaration",
    );
    assert.deepEqual(
      [...invoked]
        .filter((command) => permissions.has(command) && !enabled.has(permissions.get(command)))
        .sort(),
      [],
      "UI command permissions are not enabled for the main window",
    );
  });

  test(`${app.name} routes every declarative action and guarded mutation`, () => {
    const sources = appSources(app);
    const actions = valuesFrom(sources.ui, /data-action=["']([^"']+)["']/gu);
    const guarded = requestedGuardedActions(app, sources);

    assert.deepEqual(
      missingValues(actions, routedActions(app, sources)),
      [],
      "declarative UI actions have no router branch",
    );
    assert.deepEqual(
      missingValues(guarded, acceptedGuardedActions(sources.rust)),
      [],
      "frontend guarded mutations are absent from the Rust allowlist",
    );
  });
}

test("delegated secondary action families keep an event owner", () => {
  const requiredOwners = {
    "apps/hub-gui/ui/hub-app-events.js": ["data-mainline-action"],
    "apps/installer-gui/ui/certificate-panel.js": ["data-certificate-action"],
    "apps/installer-gui/ui/remote-node-certificates.js": ["data-certificate-focus"],
    "apps/installer-gui/ui/remote-node-mesh.js": [
      "data-remote-cluster-action",
      "data-remote-mesh-failure-action",
    ],
    "apps/installer-gui/ui/remote-node-panel.js": [
      "data-remote-node-action",
      "data-remote-bulk-action",
    ],
    "apps/installer-gui/ui/remote-node-timeline.js": ["data-recommended-action"],
  };

  for (const [relativePath, attributes] of Object.entries(requiredOwners)) {
    const source = readFileSync(path.join(ROOT, relativePath), "utf8");
    for (const attribute of attributes) {
      assert.match(
        source,
        new RegExp(`(?:closest\\?\\.|closest|querySelectorAll)\\(\\s*["']\\[${attribute}\\]`, "u"),
        `${attribute} has no delegated event owner in ${relativePath}`,
      );
    }
  }
});
