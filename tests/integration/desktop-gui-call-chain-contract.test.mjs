import assert from "node:assert/strict";
import test from "node:test";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const require = createRequire(new URL("../../apps/frontend/package.json", import.meta.url));
const ts = require("typescript");

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

function reachableUiSources(uiRoot, read = (file) => readFileSync(file, "utf8")) {
  const sources = new Map();
  const visit = (file) => {
    assert.ok(file.startsWith(`${uiRoot}${path.sep}`) && file.endsWith(".js"), `untracked UI module: ${file}`);
    if (sources.has(file)) return;
    const source = read(file);
    sources.set(file, source);
    const syntax = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    assert.equal(syntax.parseDiagnostics.length, 0, `invalid UI module: ${file}`);
    const walk = (node) => {
      let reference;
      if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) reference = node.moduleSpecifier;
      if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
        reference = node.arguments[0];
        assert.ok(reference && (ts.isStringLiteral(reference) || ts.isNoSubstitutionTemplateLiteral(reference)),
          `nonliteral UI import needs explicit coverage: ${file}`);
      }
      if (reference) {
        assert.ok(ts.isStringLiteral(reference) || ts.isNoSubstitutionTemplateLiteral(reference));
        assert.ok(reference.text.startsWith("./") || reference.text.startsWith("../"), `nonlocal UI import: ${file}`);
        visit(path.resolve(path.dirname(file), reference.text));
      }
      ts.forEachChild(node, walk);
    };
    walk(syntax);
  };
  const html = read(path.join(uiRoot, "index.html"));
  const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/giu)];
  assert.ok(scripts.length > 0, "desktop entry must declare its scripts");
  for (const [, attributes, inline] of scripts) {
    const src = attributes.match(/\bsrc\s*=\s*["']([^"']+)["']/iu)?.[1];
    assert.ok(src && !inline.trim(), "inline desktop scripts need explicit coverage");
    visit(path.resolve(uiRoot, src));
  }
  return sources;
}

function appSources(app) {
  const appRoot = path.join(ROOT, "apps", app.directory);
  const tauriRoot = path.join(appRoot, "src-tauri");
  // Shared assets can be shipped but unused; only loaded modules own native calls.
  const uiSources = reachableUiSources(path.join(appRoot, "ui"));
  const rustFiles = walkFiles(path.join(tauriRoot, "src"), ".rs");
  const permissionFiles = walkFiles(path.join(tauriRoot, "permissions"), ".toml");
  return {
    appRoot,
    tauriRoot,
    ui: [...uiSources.values()].join("\n"),
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

test("IPC reachability follows static, re-export and literal dynamic imports, not unused copies", () => {
  const root = path.join(ROOT, "virtual-ui"), files = {
    "index.html": '<script type="module" src="./app.js"></script><script src="./probe.js"></script>',
    "app.js": 'import "./shared.js"; export { value } from "./reexport.js"; import("./lazy.js"); const text = "import fake from unused.js";',
    "shared.js": 'import "./app.js"; invoke("shared_command");',
    "reexport.js": 'export const value = 1; invokeTauri("reexport_command");',
    "lazy.js": 'invoke("lazy_command");',
    "probe.js": 'invoke("probe_command");',
    "unused.js": 'invoke("unused_command");',
  };
  const read = (file) => { const value = files[path.relative(root, file)]; assert.notEqual(value, undefined); return value; };
  const reachable = reachableUiSources(root, read);
  assert.equal(reachable.size, 5);
  assert.deepEqual([...directInvocations([...reachable.values()].join("\n"))].sort(),
    ["lazy_command", "probe_command", "reexport_command", "shared_command"]);
  files["app.js"] += ' import "./unused.js";';
  assert.ok(directInvocations([...reachableUiSources(root, read).values()].join("\n")).has("unused_command"));
});

test("IPC reachability fails closed on unresolved or nonliteral modules", () => {
  const root = path.join(ROOT, "virtual-ui");
  for (const source of ['import(missing);', 'import "./missing.js";', 'import "../outside.js";', 'import "package";']) {
    assert.throws(() => reachableUiSources(root, file => {
      if (file.endsWith("index.html")) return '<script type="module" src="./app.js"></script>';
      if (file.endsWith("app.js")) return source;
      throw new Error("missing module");
    }));
  }
});

test("only Workbench loads the direct execution journal host", () => {
  for (const app of APPS) {
    const sources = reachableUiSources(path.join(ROOT, "apps", app.directory, "ui"));
    assert.equal([...sources.keys()].some(file => file.endsWith("direct-execution-journal-bridge.js")), app.router === "workbench");
  }
});

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
    assert.deepEqual(
      missingValues(acceptedGuardedActions(sources.rust), guarded),
      [],
      "desktop Rust guarded mutations have no frontend owner",
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
