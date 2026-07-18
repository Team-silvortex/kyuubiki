"use client";

/**
 * Stable automation selectors for the built-in Kyuubiki control plane UI.
 *
 * This contract is intentionally owned by the product shell, not by end users.
 * Users may extend workflows, operators, datasets, and runtime targets, but they
 * must not redefine the core control-plane DOM contract that wasm-python and
 * other automation layers rely on.
 */

export const WORKBENCH_UI_AUTOMATION_CONTRACT_VERSION = 1 as const;

export const workbenchAutomationSelectors = {
  shell: '[data-workbench-shell="root"]',
  sidebar: '[data-workbench-panel="sidebar"]',
  sidebarSection: (section: string) => `[data-workbench-sidebar-section="${section}"]`,
  railButton: (section: string) => `workbench-rail:${section}`,
  inspector: '[data-workbench-panel="inspector"]',
  console: '[data-workbench-panel="console"]',
  viewportPanel: '[data-workbench-panel="viewport"]',
  viewportStage: '[data-workbench-viewport="stage"]',
  loadedModelState: '[data-workbench-state="loaded-model"]',
  libraryTab: (tab: string) => `workbench-library-tab:${tab}`,
  sampleDomain: (domain: string) => `workbench-sample-domain:${domain}`,
  sample: (sampleId: string) => `workbench-sample:${sampleId}`,
  workflowSurface: '[data-workbench-workflow-surface]',
  workflowSurfaceTab: (tab: string) => `[data-workflow-surface-tab="${tab}"]`,
  workflowCatalogSearch: '[data-workflow-catalog-search="query"]',
  workflowCatalogAction: (action: string) => `[data-workflow-catalog-action="${action}"]`,
  workflowBuilder: '[data-workflow-builder-shell="builder"]',
  workflowOperatorSearch: '[data-workflow-operator-search="query"]',
  workflowOperatorAction: (action: string) => `[data-workflow-operator-action="${action}"]`,
  workflowBuilderAction: (action: string) => `[data-workflow-builder-action="${action}"]`,
  runtimePanel: '[data-workbench-runtime="panel"]',
  runtimeTabs: '[data-workbench-runtime="tabs"]',
  runtimeTab: (page: string) => `[data-workbench-runtime-tab="${page}"]`,
  controlWindow: '[data-workbench-control-window="root"]',
  controlWindowTabs: '[data-workbench-control-window="tabs"]',
  controlWindowTab: (mode: string) => `[data-workbench-control-mode-tab="${mode}"]`,
  controlWindowSnapshotMeta: '[data-workbench-control-window="snapshot-meta"]',
  controlWindowMetrics: '[data-workbench-control-window="metrics"]',
  controlWindowActions: '[data-workbench-control-window="actions"]',
  controlWindowExportButton: '[data-workbench-control-action="export-snapshot"]',
  controlWindowImportInput: '[data-workbench-control-action="import-snapshot"]',
  controlWindowResetButton: '[data-workbench-control-action="reset-snapshot-source"]',
} as const;

export type WorkbenchAutomationSelectorKey = keyof typeof workbenchAutomationSelectors;

export const workbenchAutomationContractRules = [
  "The workbench shell structure is product-owned and not user-extensible.",
  "Automation must target stable data-* selectors from this contract instead of visual text.",
  "Accessible labels used for automation are product-owned ids, not localized copy.",
  "Built-in sidebar, inspector, console, viewport, and runtime panels remain product-owned surfaces.",
  "Workflow/operator/data extensions may change content, but not the shell selector contract.",
  "Breaking changes to these selectors require a contract version bump.",
] as const;
