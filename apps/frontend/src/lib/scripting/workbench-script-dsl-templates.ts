"use client";

import {
  WORKBENCH_FRONTEND_DSL_REPORT_PREFIX,
  type WorkbenchFrontendDslDocument,
} from "./workbench-script-dsl.ts";
import {
  CLOSED_LOOP_TRUSS_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID,
  HEAT_TO_THERMO_QUAD_RECIPE_ID,
  HEAT_TO_THERMO_TRIANGLE_RECIPE_ID,
} from "./workbench-script-runtime-recipes.ts";

const DSL_VERSION = "kyuubiki.frontend-dsl/v1";

export function buildDefaultWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "frontend-layout-report",
    steps: [
      { kind: "log", message: "Starting built-in frontend layout report." },
      { kind: "expect_action", action: "nav/setSidebarSection", message: "Navigation action must be scriptable." },
      { kind: "expect_action", action: "job/run", message: "Study submission must be scriptable." },
      { kind: "expect_action", action: "state/replaceFrameModel", message: "Frame import must be scriptable." },
      { kind: "expect_macro", macroId: "macro/openDataResults", message: "Data review macro must be scriptable." },
      { kind: "expect_recipe", recipeId: CLOSED_LOOP_TRUSS_RECIPE_ID, message: "Closed-loop recipe must be registered." },
      {
        kind: "expect_recipe",
        recipeId: HEAT_TO_THERMO_QUAD_RECIPE_ID,
        message: "Heat-to-thermo composite recipe must be registered.",
      },
      {
        kind: "expect_recipe",
        recipeId: HEAT_TO_THERMO_TRIANGLE_RECIPE_ID,
        message: "Triangle heat-to-thermo composite recipe must be registered.",
      },
      {
        kind: "expect_recipe",
        recipeId: ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID,
        message: "Electrostatic-to-heat-to-thermo composite recipe must be registered.",
      },
      {
        kind: "expect_recipe",
        recipeId: ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID,
        message: "Triangle electrostatic-to-heat-to-thermo composite recipe must be registered.",
      },
      {
        kind: "capture_action_catalog",
        assign: "normal_actions",
        risk: "normal",
        message: "Captured normal-risk GUI action count.",
      },
      {
        kind: "capture_recipe_catalog",
        assign: "normal_recipes",
        risk: "normal",
        message: "Captured normal-risk recipe count.",
      },
      {
        kind: "emit_parity_report",
        assign: "pwdt_parity",
        message: "Captured Pwdt GUI parity report.",
      },
      { kind: "capture_now", assign: "layout_report_at", message: "Captured layout report timestamp." },
      { kind: "invoke", action: "nav/setSidebarSection", payload: { section: "workflow" } },
      { kind: "expect_state", key: "sidebarSection", equals: "workflow", message: "Workflow sidebar should be active for layout inspection." },
      {
        kind: "expect_selector_exists_all",
        selectors: [
          { selector: "shell" },
          { selector: "sidebar" },
          { selector: "viewportPanel" },
          { selector: "inspector" },
          { selector: "console" },
        ],
        message: "Primary built-in layout anchors should all be mounted.",
      },
      {
        kind: "capture_state",
        key: "sidebarSection",
        assign: "active_sidebar",
        message: "Captured active sidebar section.",
      },
      {
        kind: "capture_state",
        key: "immersiveViewport",
        assign: "immersive_mode",
        message: "Captured immersive viewport state.",
      },
      {
        kind: "capture_state",
        key: "selectedTruss3dNodeIndices",
        assign: "selected_truss3d_nodes",
        message: "Captured active 3D selection context.",
      },
      {
        kind: "capture_selector_count",
        selector: "runtimeTab",
        assign: "runtime_tab_count",
        message: "Captured runtime tab count even while workflow layout is active.",
      },
      {
        kind: "invoke",
        action: "nav/setSidebarSection",
        payload: { section: "system" },
      },
      { kind: "expect_state", key: "sidebarSection", equals: "system", message: "System sidebar should become active during layout sweep." },
      { kind: "invoke", action: "nav/setTabs", payload: { systemPanelTab: "runtime" } },
      { kind: "assert_selector", selector: "runtimePanel", message: "Runtime panel should be mounted for tab layout checks." },
      { kind: "expect_selector_text", selector: "runtimeTab", value: "control", includes: "Control", message: "Runtime control tab label should remain visible." },
      { kind: "expect_selector_count", selector: "runtimeTab", equals: 7, message: "Runtime panel should expose seven built-in tabs." },
      {
        kind: "capture_selector_text",
        selector: "runtimeTab",
        value: "overview",
        assign: "overview_tab_label",
        message: "Captured runtime overview tab label.",
      },
      {
        kind: "expect_selector_exists_all",
        selectors: [
          { selector: "runtimePanel" },
          { selector: "runtimeTab", value: "control" },
          { selector: "runtimeTab", value: "overview" },
        ],
        message: "Core runtime layout anchors should all be present.",
      },
      { kind: "log", message: "Layout started from sidebar: ${active_sidebar}" },
      { kind: "log", message: "Runtime tab count: ${runtime_tab_count}" },
      { kind: "log", message: "Runtime overview tab label: ${overview_tab_label}" },
      {
        kind: "branch_equals",
        key: "immersiveViewport",
        equals: true,
        then: [{ kind: "log", message: "Immersive viewport layout is currently enabled." }],
        else: [{ kind: "log", message: "Immersive viewport layout is currently disabled." }],
      },
      {
        kind: "foreach_state_list",
        key: "selectedTruss3dNodeIndices",
        item: "node_index",
        steps: [{ kind: "log", message: "Visible 3D node selection entry during layout check: ${node_index}" }],
        else: [{ kind: "log", message: "No active 3D node selection was present during layout capture." }],
      },
      {
        kind: "log",
        message: `${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} parity=\${pwdt_parity} normal_actions=\${normal_actions} normal_recipes=\${normal_recipes}`,
      },
      {
        kind: "log",
        message: `${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} anchors=shell,sidebar,viewportPanel,inspector,console`,
      },
      {
        kind: "log",
        message: `${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} active_sidebar=\${active_sidebar} runtime_tab_count=\${runtime_tab_count}`,
      },
      {
        kind: "log",
        message: `${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} immersive_mode=\${immersive_mode} overview_tab_label=\${overview_tab_label}`,
      },
      {
        kind: "log",
        message: `${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} selected_truss3d_nodes=\${selected_truss3d_nodes}`,
      },
      {
        kind: "log",
        message: `${WORKBENCH_FRONTEND_DSL_REPORT_PREFIX} reported_at=\${layout_report_at} status=passed`,
      },
      { kind: "log", message: "Frontend layout report completed." },
    ],
  };
}

export function buildClosedLoopTrussWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "closed-loop-truss-study",
    steps: [
      { kind: "log", message: "Starting Pwdt closed-loop truss recipe." },
      { kind: "expect_action", action: "project/create", message: "Project creation must be scriptable." },
      { kind: "expect_action", action: "model/saveAs", message: "Model save-as must be scriptable." },
      { kind: "expect_action", action: "job/run", message: "Study submission must be scriptable." },
      {
        kind: "run_recipe",
        recipeId: CLOSED_LOOP_TRUSS_RECIPE_ID,
        assign: "closed_loop",
        payload: {
          activeMaterial: "210",
          bays: 6,
          height: 3.5,
          loadY: -1500,
          modelName: "pwdt-truss-study",
          projectDescription: "Created from Pwdt frontend DSL.",
          projectName: "Pwdt closed-loop truss",
          span: 18,
          timeoutSeconds: 90,
        },
        message: "Pwdt closed-loop truss recipe completed.",
      },
      { kind: "expect_state", key: "systemDataTab", equals: "results", message: "Results data tab should be active after the recipe." },
      { kind: "emit_parity_report", assign: "pwdt_parity", message: "Captured Pwdt recipe parity report." },
    ],
  };
}

export function buildHeatToThermoQuadWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "heat-to-thermo-quad-study",
    steps: [
      { kind: "log", message: "Starting Pwdt heat-to-thermo quad recipe." },
      { kind: "expect_action", action: "project/create", message: "Project creation must be scriptable." },
      { kind: "expect_action", action: "state/projectHeatToThermo", message: "Heat result projection must be scriptable." },
      { kind: "expect_action", action: "job/run", message: "Study submission must be scriptable." },
      {
        kind: "run_recipe",
        recipeId: HEAT_TO_THERMO_QUAD_RECIPE_ID,
        assign: "heat_to_thermo",
        payload: {
          activeMaterial: "210",
          heatModelName: "pwdt-heat-plane-quad",
          projectDescription: "Created from Pwdt frontend DSL.",
          projectName: "Pwdt heat-to-thermo quad",
          thermoModelName: "pwdt-thermal-plane-quad",
          timeoutSeconds: 90,
        },
        message: "Pwdt heat-to-thermo quad recipe completed.",
      },
      { kind: "expect_state", key: "studyKind", equals: "thermal_plane_quad_2d", message: "Thermo-mechanical study should be active after projection." },
      { kind: "expect_state", key: "systemDataTab", equals: "results", message: "Results data tab should be active after the recipe." },
      { kind: "emit_parity_report", assign: "pwdt_parity", message: "Captured Pwdt recipe parity report." },
    ],
  };
}

export function buildHeatToThermoTriangleWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "heat-to-thermo-triangle-study",
    steps: [
      { kind: "log", message: "Starting Pwdt heat-to-thermo triangle recipe." },
      { kind: "expect_action", action: "project/create", message: "Project creation must be scriptable." },
      { kind: "expect_action", action: "state/projectHeatToThermo", message: "Heat result projection must be scriptable." },
      { kind: "expect_action", action: "job/run", message: "Study submission must be scriptable." },
      {
        kind: "run_recipe",
        recipeId: HEAT_TO_THERMO_TRIANGLE_RECIPE_ID,
        assign: "heat_to_thermo_triangle",
        payload: {
          activeMaterial: "210",
          heatModelName: "pwdt-heat-plane-triangle",
          projectDescription: "Created from Pwdt frontend DSL.",
          projectName: "Pwdt heat-to-thermo triangle",
          thermoModelName: "pwdt-thermal-plane-triangle",
          timeoutSeconds: 90,
        },
        message: "Pwdt heat-to-thermo triangle recipe completed.",
      },
      { kind: "expect_state", key: "studyKind", equals: "thermal_plane_triangle_2d", message: "Triangle thermo-mechanical study should be active after projection." },
      { kind: "expect_state", key: "systemDataTab", equals: "results", message: "Results data tab should be active after the recipe." },
      { kind: "emit_parity_report", assign: "pwdt_parity", message: "Captured Pwdt recipe parity report." },
    ],
  };
}

export function buildElectrostaticHeatThermoQuadWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "electrostatic-heat-thermo-quad-study",
    steps: [
      { kind: "log", message: "Starting Pwdt electrostatic-to-heat-to-thermo quad recipe." },
      { kind: "expect_action", action: "project/create", message: "Project creation must be scriptable." },
      {
        kind: "expect_action",
        action: "state/projectElectrostaticToHeat",
        message: "Electrostatic result projection must be scriptable.",
      },
      { kind: "expect_action", action: "state/projectHeatToThermo", message: "Heat result projection must be scriptable." },
      { kind: "expect_action", action: "job/run", message: "Study submission must be scriptable." },
      {
        kind: "run_recipe",
        recipeId: ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID,
        assign: "electrostatic_heat_thermo",
        payload: {
          activeMaterial: "210",
          electrostaticModelName: "pwdt-electrostatic-plane-quad",
          heatModelName: "pwdt-joule-heat-plane-quad",
          projectDescription: "Created from Pwdt frontend DSL.",
          projectName: "Pwdt electrostatic-heat-thermo quad",
          thermoModelName: "pwdt-joule-thermal-plane-quad",
          timeoutSeconds: 90,
        },
        message: "Pwdt electrostatic-to-heat-to-thermo quad recipe completed.",
      },
      { kind: "expect_state", key: "studyKind", equals: "thermal_plane_quad_2d", message: "Thermo-mechanical study should be active after the full projection chain." },
      { kind: "expect_state", key: "systemDataTab", equals: "results", message: "Results data tab should be active after the recipe." },
      { kind: "emit_parity_report", assign: "pwdt_parity", message: "Captured Pwdt recipe parity report." },
    ],
  };
}

export function buildElectrostaticHeatThermoTriangleWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "electrostatic-heat-thermo-triangle-study",
    steps: [
      { kind: "log", message: "Starting Pwdt electrostatic-to-heat-to-thermo triangle recipe." },
      { kind: "expect_action", action: "project/create", message: "Project creation must be scriptable." },
      {
        kind: "expect_action",
        action: "state/projectElectrostaticToHeat",
        message: "Electrostatic result projection must be scriptable.",
      },
      { kind: "expect_action", action: "state/projectHeatToThermo", message: "Heat result projection must be scriptable." },
      { kind: "expect_action", action: "job/run", message: "Study submission must be scriptable." },
      {
        kind: "run_recipe",
        recipeId: ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID,
        assign: "electrostatic_heat_thermo_triangle",
        payload: {
          activeMaterial: "210",
          electrostaticModelName: "pwdt-electrostatic-plane-triangle",
          heatModelName: "pwdt-joule-heat-plane-triangle",
          projectDescription: "Created from Pwdt frontend DSL.",
          projectName: "Pwdt electrostatic-heat-thermo triangle",
          thermoModelName: "pwdt-joule-thermal-plane-triangle",
          timeoutSeconds: 90,
        },
        message: "Pwdt electrostatic-to-heat-to-thermo triangle recipe completed.",
      },
      { kind: "expect_state", key: "studyKind", equals: "thermal_plane_triangle_2d", message: "Triangle thermo-mechanical study should be active after the full projection chain." },
      { kind: "expect_state", key: "systemDataTab", equals: "results", message: "Results data tab should be active after the recipe." },
      { kind: "emit_parity_report", assign: "pwdt_parity", message: "Captured Pwdt recipe parity report." },
    ],
  };
}
