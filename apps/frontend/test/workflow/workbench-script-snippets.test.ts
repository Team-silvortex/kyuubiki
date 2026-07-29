import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkbenchPythonPrelude,
  buildWorkbenchUiAutomationContractSnapshot,
  CLOSED_LOOP_TRUSS_RECIPE_ID,
  HEAT_TO_THERMO_QUAD_RECIPE_ID,
  renderWorkbenchScriptSnippet,
  WORKBENCH_SCRIPT_ACTIONS,
  WORKBENCH_SCRIPT_RECIPES,
  WORKBENCH_SCRIPT_SNIPPETS,
} from "../../src/lib/scripting/workbench-script-runtime.ts";
import {
  getWorkbenchScriptCatalogCopy,
  getWorkbenchScriptRecipeSummary,
} from "../../src/components/workbench/workbench-script-catalog-copy.ts";

function literalMatches(source: string, pattern: RegExp) {
  return [...source.matchAll(pattern)].map((match) => match[1]);
}

test("workbench Pwdt snippets expose a closed-loop study recipe", () => {
  const snippet = WORKBENCH_SCRIPT_SNIPPETS.find((entry) => entry.id === "snippet/workflow/create-run-truss-study");
  assert.ok(snippet);
  assert.equal(snippet.category, "workflow");

  const rendered = renderWorkbenchScriptSnippet(snippet, {
    projectName: "scripted-lab",
    projectDescription: "from test",
    modelName: "scripted-truss",
    activeMaterial: "200",
    bays: 4,
    span: 12,
    height: 2.5,
    loadY: -900,
    timeoutSeconds: 30,
  });

  assert.match(rendered, /await ky\.run_recipe/);
  assert.match(rendered, new RegExp(CLOSED_LOOP_TRUSS_RECIPE_ID.replaceAll("/", "\\/")));
  assert.match(rendered, /scripted-truss/);
});

test("workbench Pwdt snippets expose a heat-to-thermo composite recipe", () => {
  const snippet = WORKBENCH_SCRIPT_SNIPPETS.find((entry) => entry.id === "snippet/workflow/run-heat-to-thermo-quad");
  assert.ok(snippet);
  assert.equal(snippet.category, "workflow");

  const rendered = renderWorkbenchScriptSnippet(snippet, {
    projectName: "scripted-thermal-lab",
    projectDescription: "from test",
    heatModelName: "scripted-heat",
    thermoModelName: "scripted-thermo",
    activeMaterial: "200",
    timeoutSeconds: 30,
  });

  assert.match(rendered, /await ky\.run_recipe/);
  assert.match(rendered, new RegExp(HEAT_TO_THERMO_QUAD_RECIPE_ID.replaceAll("/", "\\/")));
  assert.match(rendered, /scripted-thermo/);
});

test("workbench Pwdt snippets only invoke catalogued actions", () => {
  const actionIds = new Set(WORKBENCH_SCRIPT_ACTIONS.map((action) => action.id));
  const missing = WORKBENCH_SCRIPT_SNIPPETS.flatMap((snippet) =>
    literalMatches(snippet.code, /ky\.invoke\("([^"]+)"/g)
      .filter((actionId) => !actionIds.has(actionId))
      .map((actionId) => `${snippet.id}:${actionId}`),
  );

  assert.deepEqual(missing, []);
});

test("workbench Pwdt snippets only run catalogued recipes", () => {
  const recipeIds = new Set(WORKBENCH_SCRIPT_RECIPES.map((recipe) => recipe.id));
  const missing = WORKBENCH_SCRIPT_SNIPPETS.flatMap((snippet) =>
    literalMatches(snippet.code, /ky\.run_recipe\("([^"]+)"/g)
      .filter((recipeId) => !recipeIds.has(recipeId))
      .map((recipeId) => `${snippet.id}:${recipeId}`),
  );

  assert.deepEqual(missing, []);
});

test("workbench Pwdt catalog copy exposes recipe registry language", () => {
  const copy = getWorkbenchScriptCatalogCopy("zh");
  const recipe = WORKBENCH_SCRIPT_RECIPES[0];

  assert.equal(copy.recipesMode, "工作流配方");
  assert.equal(copy.loadRecipeDsl, "载入 DSL");
  assert.equal(copy.expectedState, "成功状态");
  assert.equal(copy.resultKeys, "结果字段");
  assert.match(getWorkbenchScriptRecipeSummary(recipe, "zh"), /二维桁架|桁架/);
});

test("workbench Pwdt snippets only use selector keys from the UI automation contract", () => {
  const contract = buildWorkbenchUiAutomationContractSnapshot();
  const selectorKeys = new Set([
    ...Object.keys(contract.selectors),
    ...contract.parameterizedSelectors.map((selector) => selector.key),
  ]);
  const missing = WORKBENCH_SCRIPT_SNIPPETS.flatMap((snippet) =>
    [
      ...literalMatches(snippet.code, /ky\.query_selector\("([^"]+)"/g),
      ...literalMatches(snippet.code, /ky\.query_selector_all\("([^"]+)"/g),
      ...literalMatches(snippet.code, /ky\.selector_exists\("([^"]+)"/g),
    ]
      .filter((key) => !selectorKeys.has(key))
      .map((key) => `${snippet.id}:${key}`),
  );

  assert.deepEqual(missing, []);
});

test("wasm Python facade includes GUI-equivalent workflow helpers", () => {
  const prelude = buildWorkbenchPythonPrelude();

  assert.match(prelude, /async def ensure_project/);
  assert.match(prelude, /async def build_parametric_truss_2d/);
  assert.match(prelude, /async def prepare_heat_plane_quad_study/);
  assert.match(prelude, /async def save_model/);
  assert.match(prelude, /async def run_current_study/);
  assert.match(prelude, /async def project_heat_to_thermo_quad_study/);
  assert.match(prelude, /async def open_results/);
  assert.match(prelude, /"project\/create"/);
  assert.match(prelude, /"job\/run"/);
  assert.match(prelude, /"state\/projectHeatToThermo"/);
  assert.match(prelude, /"data\/setFilters"/);
});
