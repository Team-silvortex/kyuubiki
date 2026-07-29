"use client";

import type { WorkbenchFrontendDslDocument } from "./workbench-script-dsl.ts";
import type { WorkbenchScriptRecipeDefinition } from "./workbench-script-runtime-recipes.ts";

export function buildWorkbenchFrontendDslFromRecipe(
  recipe: WorkbenchScriptRecipeDefinition,
): WorkbenchFrontendDslDocument {
  return {
    dsl_version: "kyuubiki.frontend-dsl/v1",
    name: recipe.id.replace(/^recipe\//, "").replaceAll("/", "-"),
    steps: [
      { kind: "log", message: `Starting Pwdt recipe ${recipe.id}.` },
      {
        kind: "expect_recipe",
        recipeId: recipe.id,
        message: `Recipe ${recipe.id} must be registered.`,
      },
      {
        kind: "capture_recipe_catalog",
        assign: "available_recipes",
        category: recipe.category,
        message: `Captured recipe catalog entries for ${recipe.category}.`,
      },
      ...recipe.requiredActions.map((action) => ({
        kind: "expect_action" as const,
        action,
        message: `Recipe ${recipe.id} requires action ${action}.`,
      })),
      {
        kind: "run_recipe",
        recipeId: recipe.id,
        assign: "recipe_result",
        payload: recipe.payloadExample ?? {},
        message: `Pwdt recipe ${recipe.id} completed.`,
      },
      ...Object.entries(recipe.success?.expectedState ?? {}).map(([key, equals]) => ({
        kind: "expect_state" as const,
        key,
        equals,
        message: `Recipe ${recipe.id} should leave state ${key}=${String(equals)}.`,
      })),
      {
        kind: "emit_parity_report",
        assign: "pwdt_parity",
        message: "Captured Pwdt recipe parity report.",
      },
    ],
  };
}
