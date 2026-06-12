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
        key: "controlWindowTab",
        parameter: "mode",
        template: workbenchAutomationSelectors.controlWindowTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${mode}"),
      },
    ],
    rules: [...workbenchAutomationContractRules],
  };
}
