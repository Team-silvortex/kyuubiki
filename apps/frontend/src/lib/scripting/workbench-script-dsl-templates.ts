"use client";

import {
  WORKBENCH_FRONTEND_DSL_REPORT_PREFIX,
  type WorkbenchFrontendDslDocument,
} from "./workbench-script-dsl.ts";

const DSL_VERSION = "kyuubiki.frontend-dsl/v1";

export function buildDefaultWorkbenchFrontendDslDocument(): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: "frontend-layout-report",
    steps: [
      { kind: "log", message: "Starting built-in frontend layout report." },
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
