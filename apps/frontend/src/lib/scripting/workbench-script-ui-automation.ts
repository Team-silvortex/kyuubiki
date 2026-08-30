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
      modelPanel: workbenchAutomationSelectors.modelPanel,
      modelStudyPanel: workbenchAutomationSelectors.modelStudyPanel,
      modelStudyKind: workbenchAutomationSelectors.modelStudyKind,
      modelStudyRun: workbenchAutomationSelectors.modelStudyRun,
      workflowSurface: workbenchAutomationSelectors.workflowSurface,
      workflowCatalogSearch: workbenchAutomationSelectors.workflowCatalogSearch,
      workflowBuilder: workbenchAutomationSelectors.workflowBuilder,
      workflowBuilderSecondaryTools: workbenchAutomationSelectors.workflowBuilderSecondaryTools,
      workflowOperatorSearch: workbenchAutomationSelectors.workflowOperatorSearch,
      workflowTopologyKind: workbenchAutomationSelectors.workflowTopologyKind,
      workflowControlPlane: workbenchAutomationSelectors.workflowControlPlane,
      workflowControlEmptyAction: workbenchAutomationSelectors.workflowControlEmptyAction,
      runtimePanel: workbenchAutomationSelectors.runtimePanel,
      runtimeTabs: workbenchAutomationSelectors.runtimeTabs,
      systemSidebar: workbenchAutomationSelectors.systemSidebar,
      dataAdminPanel: workbenchAutomationSelectors.dataAdminPanel,
      libraryPanel: workbenchAutomationSelectors.libraryPanel,
      storePanel: workbenchAutomationSelectors.storePanel,
      storeSearch: workbenchAutomationSelectors.storeSearch,
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
        key: "uiChunk",
        parameter: "chunkId",
        template: workbenchAutomationSelectors.uiChunk(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${chunkId}"),
      },
      {
        key: "sidebarSection",
        parameter: "section",
        template: workbenchAutomationSelectors.sidebarSection(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${section}"),
      },
      {
        key: "railButton",
        parameter: "section",
        template: workbenchAutomationSelectors.railButton(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${section}"),
      },
      {
        key: "modelTab",
        parameter: "tab",
        template: workbenchAutomationSelectors.modelTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${tab}"),
      },
      {
        key: "modelToolsPage",
        parameter: "page",
        template: workbenchAutomationSelectors.modelToolsPage(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "modelStudyDomain",
        parameter: "domain",
        template: workbenchAutomationSelectors.modelStudyDomain(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${domain}"),
      },
      {
        key: "libraryTab",
        parameter: "tab",
        template: workbenchAutomationSelectors.libraryTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${tab}"),
      },
      {
        key: "librarySamplePage",
        parameter: "page",
        template: workbenchAutomationSelectors.librarySamplePage(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "libraryProjectPage",
        parameter: "page",
        template: workbenchAutomationSelectors.libraryProjectPage(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "libraryModelPage",
        parameter: "page",
        template: workbenchAutomationSelectors.libraryModelPage(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "sampleDomain",
        parameter: "domain",
        template: workbenchAutomationSelectors.sampleDomain(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${domain}"),
      },
      {
        key: "sample",
        parameter: "sampleId",
        template: workbenchAutomationSelectors.sample(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${sampleId}"),
      },
      {
        key: "runtimeTab",
        parameter: "page",
        template: workbenchAutomationSelectors.runtimeTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "systemSurfaceTab",
        parameter: "tab",
        template: workbenchAutomationSelectors.systemSurfaceTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${tab}"),
      },
      {
        key: "systemSettingsPage",
        parameter: "page",
        template: workbenchAutomationSelectors.systemSettingsPage(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "dataTab",
        parameter: "tab",
        template: workbenchAutomationSelectors.dataTab(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${tab}"),
      },
      {
        key: "dataPage",
        parameter: "page",
        template: workbenchAutomationSelectors.dataPage(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${page}"),
      },
      {
        key: "storeKind",
        parameter: "kind",
        template: workbenchAutomationSelectors.storeKind(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${kind}"),
      },
      {
        key: "storeEntry",
        parameter: "entryId",
        template: workbenchAutomationSelectors.storeEntry(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${entryId}"),
      },
      {
        key: "storeEntryAction",
        parameter: "action",
        template: workbenchAutomationSelectors.storeEntryAction(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${action}"),
      },
      {
        key: "storeManifestEntry",
        parameter: "entryId",
        template: workbenchAutomationSelectors.storeManifestEntry(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${entryId}"),
      },
      {
        key: "storeManifestAction",
        parameter: "action",
        template: workbenchAutomationSelectors.storeManifestAction(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${action}"),
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
        key: "workflowTopologyAction",
        parameter: "action",
        template: workbenchAutomationSelectors.workflowTopologyAction(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${action}"),
      },
      {
        key: "workflowControlNode",
        parameter: "nodeId",
        template: workbenchAutomationSelectors.workflowControlNode(SELECTOR_ARGUMENT_TOKEN).replace(SELECTOR_ARGUMENT_TOKEN, "${nodeId}"),
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
