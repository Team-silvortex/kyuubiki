import assert from "node:assert/strict";
import test from "node:test";

import { handleWorkbenchScriptNavAction } from "../../src/components/workbench/workbench-script-nav-controller.ts";

function createNavigationHarness() {
  const state = {
    sidebarSection: "model",
    workflowPanelTab: "overview",
  };

  const invoke = (action: string, payload: Record<string, unknown>) =>
    handleWorkbenchScriptNavAction({
      action,
      payload,
      studyKind: "truss_2d",
      studyKindResetHandlers: {},
      setStudyKind: () => undefined,
      handleSidebarSectionChange: (section) => {
        state.sidebarSection = section;
      },
      recordHistory: () => undefined,
      changeStudyTypeLabel: "Change study type",
      setStudyTab: () => undefined,
      setModelTab: () => undefined,
      setModelToolsPage: () => undefined,
      setLibraryTab: () => undefined,
      setWorkflowPanelTab: (tab) => {
        state.workflowPanelTab = tab;
      },
      setSystemPanelTab: () => undefined,
      setAssistantWindowOpen: () => undefined,
      setSystemDataTab: () => undefined,
      handleLanguageChange: () => undefined,
      setTheme: () => undefined,
      currentFrontendRuntimeMode: "orchestrated_gui",
      setFrontendRuntimeMode: () => undefined,
      currentDirectMeshEndpointsText: "",
      setDirectMeshEndpointsText: () => undefined,
      setDirectMeshSelectionMode: () => undefined,
      refreshHealth: async () => undefined,
      refreshJobHistory: async () => undefined,
      refreshResults: async () => undefined,
      refreshProjects: async () => undefined,
      refreshSecurityEvents: async () => undefined,
    });

  return { invoke, state };
}

test("script navigation treats Store and Workflow tabs as first-class destinations", async () => {
  const harness = createNavigationHarness();

  const storeResult = await harness.invoke("nav/setSidebarSection", { section: "store" });
  assert.equal(harness.state.sidebarSection, "store");
  assert.equal(storeResult?.section, "store");

  const workflowResult = await harness.invoke("nav/setTabs", { workflowPanelTab: "builder" });
  assert.equal(harness.state.workflowPanelTab, "builder");
  assert.equal((workflowResult?.tabs as Record<string, unknown>).workflowPanelTab, "builder");
});

test("script navigation rejects invalid destinations instead of reporting false success", async () => {
  const harness = createNavigationHarness();

  await assert.rejects(
    () => harness.invoke("nav/setSidebarSection", { section: "missing" }),
    /Invalid section: missing/u,
  );
  await assert.rejects(
    () => harness.invoke("nav/setTabs", { workflowPanelTab: "missing" }),
    /Invalid workflowPanelTab: missing/u,
  );
  await assert.rejects(
    () => harness.invoke("nav/setStudyKind", { studyKind: "missing" }),
    /Invalid studyKind: missing/u,
  );
  await assert.rejects(
    () => harness.invoke("nav/setTabs", {}),
    /requires at least one supported tab value/u,
  );
  assert.deepEqual(harness.state, {
    sidebarSection: "model",
    workflowPanelTab: "overview",
  });
});
