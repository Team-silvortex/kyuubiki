"use client";

export type WorkbenchScriptRecipeDefinition = {
  id: string;
  category: "study" | "workflow" | "inspection";
  risk: "normal" | "sensitive" | "destructive";
  summary: {
    en: string;
    zh: string;
  };
  payloadExample?: Record<string, unknown>;
  requiredActions: string[];
  success?: {
    expectedState?: Record<string, string | number | boolean | null>;
    resultKeys?: string[];
  };
};

export type WorkbenchScriptRecipeFilters = {
  category?: WorkbenchScriptRecipeDefinition["category"];
  risk?: WorkbenchScriptRecipeDefinition["risk"];
};

export const CLOSED_LOOP_TRUSS_RECIPE_ID = "recipe/truss2d/closed-loop";
export const HEAT_TO_THERMO_QUAD_RECIPE_ID = "recipe/heat-thermo/quad-closed-loop";

export const WORKBENCH_SCRIPT_RECIPES: WorkbenchScriptRecipeDefinition[] = [
  {
    id: CLOSED_LOOP_TRUSS_RECIPE_ID,
    category: "study",
    risk: "normal",
    summary: {
      en: "Create or reuse a project, generate a 2D truss, save it, run the solver, and open results.",
      zh: "创建或复用项目，生成二维桁架，保存模型，提交求解，并打开结果。",
    },
    payloadExample: {
      activeMaterial: "210",
      bays: 6,
      height: 3.5,
      loadY: -1500,
      modelName: "pwdt-truss-study",
      projectName: "Pwdt closed-loop truss",
      span: 18,
      timeoutSeconds: 90,
    },
    requiredActions: [
      "project/create",
      "nav/setStudyKind",
      "nav/setSidebarSection",
      "nav/setTabs",
      "model/setWorkspaceMeta",
      "state/setParametric",
      "model/generateTruss",
      "model/saveAs",
      "job/run",
      "data/setFilters",
    ],
    success: {
      expectedState: {
        jobStatus: "completed",
        studyKind: "truss_2d",
        systemDataTab: "results",
      },
      resultKeys: ["ok", "projectId", "saveResult", "jobStatus", "resultCount"],
    },
  },
  {
    id: HEAT_TO_THERMO_QUAD_RECIPE_ID,
    category: "workflow",
    risk: "normal",
    summary: {
      en: "Run a 2D heat quad study, project the temperature field into a thermo-mechanical quad model, save it, run it, and open results.",
      zh: "运行二维四边形热场研究，将温度场投影为热-力四边形模型，保存后继续求解并打开结果。",
    },
    payloadExample: {
      activeMaterial: "210",
      heatModelName: "pwdt-heat-plane-quad",
      projectName: "Pwdt heat-to-thermo quad",
      thermoModelName: "pwdt-thermal-plane-quad",
      timeoutSeconds: 90,
    },
    requiredActions: [
      "project/create",
      "nav/setStudyKind",
      "nav/setSidebarSection",
      "nav/setTabs",
      "model/setWorkspaceMeta",
      "model/saveAs",
      "job/run",
      "state/projectHeatToThermo",
      "data/setFilters",
    ],
    success: {
      expectedState: {
        jobStatus: "completed",
        studyKind: "thermal_plane_quad_2d",
        systemDataTab: "results",
      },
      resultKeys: [
        "ok",
        "projectId",
        "heatSaveResult",
        "heatJobStatus",
        "thermoProjection",
        "thermoSaveResult",
        "thermoJobStatus",
        "resultCount",
      ],
    },
  },
];

export function getWorkbenchScriptRecipeDefinition(recipeId: string) {
  return WORKBENCH_SCRIPT_RECIPES.find((entry) => entry.id === recipeId) ?? null;
}

export function filterWorkbenchScriptRecipes(filters: WorkbenchScriptRecipeFilters = {}) {
  return WORKBENCH_SCRIPT_RECIPES.filter((recipe) => {
    if (filters.category && recipe.category !== filters.category) return false;
    if (filters.risk && recipe.risk !== filters.risk) return false;
    return true;
  });
}

export function isWorkbenchScriptRecipeId(recipeId: string) {
  return Boolean(getWorkbenchScriptRecipeDefinition(recipeId));
}
