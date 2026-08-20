"use client";

import {
  WORKBENCH_UI_AUTOMATION_CONTRACT_VERSION,
  workbenchAutomationContractRules,
  workbenchAutomationSelectors,
} from "@/components/workbench/workbench-ui-automation-contract";

const SELECTOR_ARGUMENT_TOKEN = "__WORKBENCH_SELECTOR_ARG__";

export type WorkbenchUiAutomationParameterizedSelector = {
  key: string;
  parameter: string;
  template: string;
};

export type WorkbenchUiAutomationContractSnapshot = {
  contractVersion: number;
  shellExtensible: false;
  selectors: Record<string, string>;
  parameterizedSelectors: WorkbenchUiAutomationParameterizedSelector[];
  rules: string[];
};

export function buildWorkbenchUiAutomationContractSnapshot(): WorkbenchUiAutomationContractSnapshot {
  return {
    contractVersion: WORKBENCH_UI_AUTOMATION_CONTRACT_VERSION,
    shellExtensible: false,
    selectors: {
      shell: workbenchAutomationSelectors.shell,
      sidebar: workbenchAutomationSelectors.sidebar,
      inspector: workbenchAutomationSelectors.inspector,
      console: workbenchAutomationSelectors.console,
      viewportPanel: workbenchAutomationSelectors.viewportPanel,
      viewportStage: workbenchAutomationSelectors.viewportStage,
      loadedModelState: workbenchAutomationSelectors.loadedModelState,
      workflowSurface: workbenchAutomationSelectors.workflowSurface,
      workflowCatalogSearch: workbenchAutomationSelectors.workflowCatalogSearch,
      workflowBuilder: workbenchAutomationSelectors.workflowBuilder,
      workflowOperatorSearch: workbenchAutomationSelectors.workflowOperatorSearch,
      workflowTopologyKind: workbenchAutomationSelectors.workflowTopologyKind,
      runtimePanel: workbenchAutomationSelectors.runtimePanel,
      runtimeTabs: workbenchAutomationSelectors.runtimeTabs,
      controlWindow: workbenchAutomationSelectors.controlWindow,
      controlWindowTabs: workbenchAutomationSelectors.controlWindowTabs,
      controlWindowSnapshotMeta: workbenchAutomationSelectors.controlWindowSnapshotMeta,
      controlWindowMetrics: workbenchAutomationSelectors.controlWindowMetrics,
      controlWindowActions: workbenchAutomationSelectors.controlWindowActions,
      controlWindowExportButton: workbenchAutomationSelectors.controlWindowExportButton,
      controlWindowImportInput: workbenchAutomationSelectors.controlWindowImportInput,
      controlWindowResetButton: workbenchAutomationSelectors.controlWindowResetButton,
    },
    parameterizedSelectors: [
      {
        key: "sidebarSection",
        parameter: "section",
        template: workbenchAutomationSelectors.sidebarSection(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${section}"),
      },
      {
        key: "runtimeTab",
        parameter: "page",
        template: workbenchAutomationSelectors.runtimeTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "workflowSurfaceTab",
        parameter: "tab",
        template: workbenchAutomationSelectors.workflowSurfaceTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${tab}"),
      },
      {
        key: "workflowCatalogEntry",
        parameter: "workflowId",
        template: workbenchAutomationSelectors.workflowCatalogEntry(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${workflowId}"),
      },
      {
        key: "workflowCatalogAction",
        parameter: "action",
        template: workbenchAutomationSelectors.workflowCatalogAction(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${action}"),
      },
      {
        key: "workflowOperatorAction",
        parameter: "action",
        template: workbenchAutomationSelectors.workflowOperatorAction(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${action}"),
      },
      {
        key: "workflowBuilderAction",
        parameter: "action",
        template: workbenchAutomationSelectors.workflowBuilderAction(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${action}"),
      },
      {
        key: "workflowInputArtifact",
        parameter: "nodeId",
        template: workbenchAutomationSelectors.workflowInputArtifact(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${nodeId}"),
      },
      {
        key: "workflowRun",
        parameter: "jobId",
        template: workbenchAutomationSelectors.workflowRun(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${jobId}"),
      },
      {
        key: "workflowRunStatus",
        parameter: "status",
        template: workbenchAutomationSelectors.workflowRunStatus(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${status}"),
      },
      {
        key: "workflowRunWorkflow",
        parameter: "workflowId",
        template: workbenchAutomationSelectors.workflowRunWorkflow(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${workflowId}"),
      },
      {
        key: "controlWindowTab",
        parameter: "mode",
        template: workbenchAutomationSelectors.controlWindowTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${mode}"),
      },
    ],
    rules: [...workbenchAutomationContractRules],
  };
}
