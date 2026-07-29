import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";

import {
  buildWorkbenchPythonPrelude,
  buildWorkbenchFrontendDslFromRecipe,
  CLOSED_LOOP_TRUSS_WORKBENCH_FRONTEND_DSL,
  compileWorkbenchFrontendDslToPython,
  DEFAULT_WORKBENCH_FRONTEND_DSL,
  ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_QUAD_WORKBENCH_FRONTEND_DSL,
  ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_TRIANGLE_WORKBENCH_FRONTEND_DSL,
  HEAT_TO_THERMO_QUAD_RECIPE_ID,
  HEAT_TO_THERMO_QUAD_WORKBENCH_FRONTEND_DSL,
  HEAT_TO_THERMO_TRIANGLE_RECIPE_ID,
  HEAT_TO_THERMO_TRIANGLE_WORKBENCH_FRONTEND_DSL,
  parseWorkbenchFrontendDslDocument,
  serializeWorkbenchFrontendDslDocument,
  WORKBENCH_SCRIPT_ACTIONS,
  WORKBENCH_SCRIPT_MACROS,
  WORKBENCH_SCRIPT_RECIPES,
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

test("workbench Pwdt recipe catalog references only registered GUI script actions", () => {
  const cataloguedActions = new Set(WORKBENCH_SCRIPT_ACTIONS.map((action) => action.id));
  const missing = WORKBENCH_SCRIPT_RECIPES.flatMap((recipe) =>
    recipe.requiredActions
      .filter((action) => !cataloguedActions.has(action))
      .map((action) => `${recipe.id}:${action}`),
  );

  assert.deepEqual(missing, []);
  assert.ok(WORKBENCH_SCRIPT_RECIPES.some((recipe) => recipe.id === "recipe/truss2d/closed-loop"));
  assert.ok(WORKBENCH_SCRIPT_RECIPES.some((recipe) => recipe.id === HEAT_TO_THERMO_QUAD_RECIPE_ID));
  assert.ok(WORKBENCH_SCRIPT_RECIPES.some((recipe) => recipe.id === HEAT_TO_THERMO_TRIANGLE_RECIPE_ID));
  assert.ok(WORKBENCH_SCRIPT_RECIPES.some((recipe) => recipe.id === ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID));
  assert.ok(WORKBENCH_SCRIPT_RECIPES.some((recipe) => recipe.id === ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID));
  assert.deepEqual(WORKBENCH_SCRIPT_RECIPES[0].success?.expectedState, {
    jobStatus: "completed",
    studyKind: "truss_2d",
    systemDataTab: "results",
  });
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

  assert.throws(
    () =>
      parseWorkbenchFrontendDslDocument(
        JSON.stringify({
          dsl_version: "kyuubiki.frontend-dsl/v1",
          name: "bad-recipe",
          steps: [{ kind: "run_recipe", recipeId: "recipe/missing" }],
        }),
      ),
    /unknown Pwdt recipe/,
  );

  assert.throws(
    () =>
      parseWorkbenchFrontendDslDocument(
        JSON.stringify({
          dsl_version: "kyuubiki.frontend-dsl/v1",
          name: "bad-expect-recipe",
          steps: [{ kind: "expect_recipe", recipeId: "recipe/missing" }],
        }),
      ),
    /unknown Pwdt recipe/,
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
        { kind: "expect_recipe", recipeId: WORKBENCH_SCRIPT_RECIPES[0].id },
        { kind: "capture_action_catalog", assign: "model_actions", category: "model" },
        { kind: "capture_recipe_catalog", assign: "study_recipes", category: "study" },
        { kind: "emit_parity_report", assign: "parity" },
      ],
    }),
  );
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.ok(compiled.includes('ky.require_action("job/run")'));
  assert.match(compiled, /ky\.require_macro/);
  assert.match(compiled, /ky\.require_recipe/);
  assert.match(compiled, /ky\.actions_matching\(category="model"\)/);
  assert.match(compiled, /ky\.recipes_matching\(category="study"\)/);
  assert.match(compiled, /ky\.automation_parity_report\(\)/);
});

test("workbench frontend DSL compiles closed-loop Pwdt recipes into the wasm Python facade", () => {
  const document = parseWorkbenchFrontendDslDocument(
    JSON.stringify({
      dsl_version: "kyuubiki.frontend-dsl/v1",
      name: "recipe-parity",
      steps: [
        {
          kind: "run_recipe",
          recipeId: "recipe/truss2d/closed-loop",
          assign: "study_result",
          payload: { projectName: "DSL Project", modelName: "dsl-truss", bays: 4 },
        },
      ],
    }),
  );
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.match(compiled, /study_result = await ky\.run_recipe/);
  assert.match(compiled, /not isinstance\(study_result, dict\)/);
  assert.match(compiled, /recipe\/truss2d\/closed-loop/);
  assert.match(compiled, /DSL Project/);
  for (const key of WORKBENCH_SCRIPT_RECIPES[0].success?.resultKeys ?? []) {
    assert.match(compiled, new RegExp(JSON.stringify(key)));
  }
});

test("workbench recipe registry can materialize runnable DSL documents", () => {
  for (const recipe of WORKBENCH_SCRIPT_RECIPES) {
    const document = buildWorkbenchFrontendDslFromRecipe(recipe);
    const parsed = parseWorkbenchFrontendDslDocument(serializeWorkbenchFrontendDslDocument(document));
    const compiled = compileWorkbenchFrontendDslToPython(parsed);

    assert.equal(parsed.name, recipe.id.replace(/^recipe\//, "").replaceAll("/", "-"));
    assert.ok(parsed.steps.some((step) => step.kind === "expect_recipe" && step.recipeId === recipe.id));
    assert.ok(parsed.steps.some((step) => step.kind === "capture_recipe_catalog"));
    assert.ok(parsed.steps.some((step) => step.kind === "run_recipe" && step.recipeId === recipe.id));
    for (const [key, equals] of Object.entries(recipe.success?.expectedState ?? {})) {
      assert.ok(parsed.steps.some((step) => step.kind === "expect_state" && step.key === key && step.equals === equals));
    }
    assert.match(compiled, new RegExp(recipe.id.replaceAll("/", "\\/")));
    assert.match(compiled, /ky\.recipes_matching/);
    for (const key of recipe.success?.resultKeys ?? []) {
      assert.match(compiled, new RegExp(JSON.stringify(key)));
    }
    for (const action of recipe.requiredActions) {
      assert.match(compiled, new RegExp(action.replaceAll("/", "\\/")));
    }
  }
});

test("default Pwdt DSL template reports layout and GUI action parity", () => {
  const document = parseWorkbenchFrontendDslDocument(DEFAULT_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "frontend-layout-report");
  assert.ok(compiled.includes("state/replaceFrameModel"));
  assert.ok(compiled.includes("ky.require_recipe"));
  assert.ok(compiled.includes("ky.recipes_matching"));
  assert.match(compiled, /ky\.automation_parity_report\(\)/);
  assert.match(compiled, /\[layout-report\] parity=/);
});

test("closed-loop truss Pwdt DSL template is runnable structured automation", () => {
  const document = parseWorkbenchFrontendDslDocument(CLOSED_LOOP_TRUSS_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "closed-loop-truss-study");
  assert.match(compiled, /ky\.run_recipe/);
  assert.match(compiled, /recipe\/truss2d\/closed-loop/);
  assert.match(compiled, /systemDataTab/);
});

test("heat-to-thermo Pwdt DSL template is runnable structured automation", () => {
  const document = parseWorkbenchFrontendDslDocument(HEAT_TO_THERMO_QUAD_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "heat-to-thermo-quad-study");
  assert.match(compiled, /ky\.run_recipe/);
  assert.match(compiled, /recipe\/heat-thermo\/quad-closed-loop/);
  assert.match(compiled, /state\/projectHeatToThermo/);
  assert.match(compiled, /thermal_plane_quad_2d/);
});

test("heat-to-thermo triangle Pwdt DSL template is runnable structured automation", () => {
  const document = parseWorkbenchFrontendDslDocument(HEAT_TO_THERMO_TRIANGLE_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "heat-to-thermo-triangle-study");
  assert.match(compiled, /ky\.run_recipe/);
  assert.match(compiled, /recipe\/heat-thermo\/triangle-closed-loop/);
  assert.match(compiled, /state\/projectHeatToThermo/);
  assert.match(compiled, /thermal_plane_triangle_2d/);
});

test("electrostatic-to-heat-to-thermo Pwdt DSL template is runnable structured automation", () => {
  const document = parseWorkbenchFrontendDslDocument(ELECTROSTATIC_HEAT_THERMO_QUAD_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "electrostatic-heat-thermo-quad-study");
  assert.match(compiled, /ky\.run_recipe/);
  assert.match(compiled, /recipe\/electrostatic-heat-thermo\/quad-closed-loop/);
  assert.match(compiled, /state\/projectElectrostaticToHeat/);
  assert.match(compiled, /state\/projectHeatToThermo/);
  assert.match(compiled, /thermal_plane_quad_2d/);
});

test("electrostatic-to-heat-to-thermo triangle Pwdt DSL template is runnable structured automation", () => {
  const document = parseWorkbenchFrontendDslDocument(ELECTROSTATIC_HEAT_THERMO_TRIANGLE_WORKBENCH_FRONTEND_DSL);
  const compiled = compileWorkbenchFrontendDslToPython(document);

  assert.equal(document.name, "electrostatic-heat-thermo-triangle-study");
  assert.match(compiled, /ky\.run_recipe/);
  assert.match(compiled, /recipe\/electrostatic-heat-thermo\/triangle-closed-loop/);
  assert.match(compiled, /state\/projectElectrostaticToHeat/);
  assert.match(compiled, /state\/projectHeatToThermo/);
  assert.match(compiled, /thermal_plane_triangle_2d/);
});

test("wasm Python prelude exposes GUI-equivalent capability introspection helpers", () => {
  const prelude = buildWorkbenchPythonPrelude();

  assert.match(prelude, /def require_action/);
  assert.match(prelude, /def actions_by_category/);
  assert.match(prelude, /def require_recipe/);
  assert.match(prelude, /def recipes/);
  assert.match(prelude, /def recipes_matching/);
  assert.match(prelude, /def automation_parity_report/);
  assert.match(prelude, /async def run_recipe/);
  assert.match(prelude, /async def run_closed_loop_truss_study/);
  assert.match(prelude, /async def run_heat_to_thermo_quad_study/);
  assert.match(prelude, /async def run_heat_to_thermo_triangle_study/);
  assert.match(prelude, /async def run_electrostatic_heat_thermo_quad_study/);
  assert.match(prelude, /async def run_electrostatic_heat_thermo_triangle_study/);
  assert.match(prelude, /async def prepare_electrostatic_plane_quad_study/);
  assert.match(prelude, /async def prepare_electrostatic_plane_triangle_study/);
  assert.match(prelude, /async def project_electrostatic_to_heat_quad_study/);
  assert.match(prelude, /async def project_electrostatic_to_heat_triangle_study/);
  assert.match(prelude, /async def project_heat_to_thermo_quad_study/);
  assert.match(prelude, /async def project_heat_to_thermo_triangle_study/);
  assert.match(prelude, /self\.require_action\(action\)/);
  assert.match(prelude, /self\.require_macro\(macro\)/);
  assert.match(prelude, /self\.require_recipe\(recipe_id\)/);
});
