import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

import {
  buildWorkbenchPythonPrelude,
  compileWorkbenchFrontendDslToPython,
  DEFAULT_WORKBENCH_FRONTEND_DSL,
  parseWorkbenchFrontendDslDocument,
  WORKBENCH_SCRIPT_ACTIONS,
  WORKBENCH_SCRIPT_MACROS,
} from "../../src/lib/scripting/workbench-script-runtime.ts";

const CONTROLLER_FILES = [
  "src/components/workbench/workbench-script-nav-controller.ts",
  "src/components/workbench/workbench-script-project-model-controller.ts",
  "src/components/workbench/workbench-script-state-controller.ts",
  "src/components/workbench/workbench-script-macro-data-controller.ts",
];

function readFrontend(relativePath: string) {
  return readFileSync(path.join(process.cwd(), relativePath), "utf8");
}

function implementedControllerActions() {
  const actions = new Set<string>();
  for (const file of CONTROLLER_FILES) {
    const source = readFrontend(file);
    for (const match of source.matchAll(/case "([^"]+)"/g)) {
      actions.add(match[1]);
    }
  }
  return [...actions].sort();
}

test("workbench Pwdt action catalog covers every implemented GUI script action", () => {
  const catalogued = new Set(WORKBENCH_SCRIPT_ACTIONS.map((action) => action.id));
  const missing = implementedControllerActions().filter((action) => !catalogued.has(action));

  assert.deepEqual(missing, []);
  assert.ok(catalogued.has("macro/run"));
  assert.ok(catalogued.has("state/replaceFrameModel"));
  assert.ok(catalogued.has("state/replaceBeamModel"));
});

test("workbench frontend DSL rejects unknown actions and macros before Pyodide execution", () => {
  assert.throws(
    () =>
      parseWorkbenchFrontendDslDocument(
        JSON.stringify({
          dsl_version: "kyuubiki.frontend-dsl/v1",
          name: "bad-action",
          steps: [{ kind: "invoke", action: "missing/action" }],
        }),
      ),
    /unknown Workbench action/,
  );

  assert.throws(
    () =>
      parseWorkbenchFrontendDslDocument(
        JSON.stringify({
          dsl_version: "kyuubiki.frontend-dsl/v1",
          name: "bad-macro",
          steps: [{ kind: "macro", macroId: "macro/missing" }],
        }),
      ),
    /unknown Workbench macro/,
  );
});

test("workbench frontend DSL compiles capability checks into the wasm Python facade", () => {
  const document = parseWorkbenchFrontendDslDocument(
    JSON.stringify({
      dsl_version: "kyuubiki.frontend-dsl/v1",
      name: "capability-parity",
      steps: [
        { kind: "expect_action", action: "job/run" },
        { kind: "expect_macro", macroId: WORKBENCH_SCRIPT_MACROS[0].id },
        { kind: "capture_action_catalog", assign: "model_actions", category: "model" },
        { kind: "emit_parity_report", assign: "parity" },
      ],
    }),
  );
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.ok(compiled.includes('ky.require_action("job/run")'));
  assert.match(compiled, /ky\.require_macro/);
  assert.match(compiled, /ky\.actions_matching\(category="model"\)/);
  assert.match(compiled, /ky\.automation_parity_report\(\)/);
});

test("default Pwdt DSL template reports layout and GUI action parity", () => {
  const document = parseWorkbenchFrontendDslDocument(DEFAULT_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "frontend-layout-report");
  assert.ok(compiled.includes("state/replaceFrameModel"));
  assert.match(compiled, /ky\.automation_parity_report\(\)/);
  assert.match(compiled, /\[layout-report\] parity=/);
});

test("wasm Python prelude exposes GUI-equivalent capability introspection helpers", () => {
  const prelude = buildWorkbenchPythonPrelude();

  assert.match(prelude, /def require_action/);
  assert.match(prelude, /def actions_by_category/);
  assert.match(prelude, /def automation_parity_report/);
  assert.match(prelude, /self\.require_action\(action\)/);
  assert.match(prelude, /self\.require_macro\(macro\)/);
});
