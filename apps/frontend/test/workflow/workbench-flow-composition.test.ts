import test from "node:test";
import assert from "node:assert/strict";

import { buildWorkbenchFlowComposition } from "../../src/components/workbench/workbench-flow-composition.ts";

test("flow composition wires project exports from the project storage controller", () => {
  const downloadProjectBundleJson = async () => {};
  const downloadProjectBundleZip = async () => {};
  const composition = buildWorkbenchFlowComposition({
    interactionControllers: {
      assistantAudit: {},
      topLevelActions: {},
      uiActionController: {},
      toggleImmersiveViewport: () => {},
    },
    projectFlows: {
      adminDataEffects: {},
      projectStorageController: {
        downloadProjectBundleJson,
        downloadProjectBundleZip,
        openModelVersionById: async () => ({ ok: true }),
      },
    },
    shellState: { languagePacks: [] },
    studyResultDerived: {},
    workflowController: {},
    workspaceState: {},
  });

  assert.equal(composition.downloadProjectBundleJson, downloadProjectBundleJson);
  assert.equal(composition.downloadProjectBundleZip, downloadProjectBundleZip);
});

test("flow composition preserves selected admin records and editing drafts", () => {
  const composition = buildWorkbenchFlowComposition({
    interactionControllers: {
      assistantAudit: {},
      topLevelActions: {},
      uiActionController: {},
      toggleImmersiveViewport: () => {},
    },
    projectFlows: {
      adminDataEffects: {},
      projectStorageController: {},
    },
    shellState: { languagePacks: [] },
    studyResultDerived: {},
    workflowController: {},
    workspaceState: {},
    selectedAdminJobId: "job-qualification",
    adminJobMessage: "job message",
    adminJobProjectId: "project-qualification",
    adminJobModelVersionId: "version-qualification",
    adminJobCaseId: "case-qualification",
    selectedAdminResultJobId: "result-qualification",
    adminResultDraft: '{"metric":42}',
  });

  assert.equal(composition.selectedAdminJobId, "job-qualification");
  assert.equal(composition.adminJobMessage, "job message");
  assert.equal(composition.adminJobProjectId, "project-qualification");
  assert.equal(composition.adminJobModelVersionId, "version-qualification");
  assert.equal(composition.adminJobCaseId, "case-qualification");
  assert.equal(composition.selectedAdminResultJobId, "result-qualification");
  assert.equal(composition.adminResultDraft, '{"metric":42}');
});

test("flow composition connects project, model, selection and recovery dependencies to PWDT", () => {
  const workspaceState = {
    projects: [{ project_id: "project-a" }],
    selectedModelId: "model-a",
    setSelectedProjectId: () => {},
    setSelectedTruss3dNodes: () => {},
    setSystemAlerts: () => {},
  };
  const services = {
    projectLibraryBackendService: { createProject: async () => ({}) },
    resetActiveResult: () => {},
    openWorkspaceStudy: () => {},
  };
  const composition = buildWorkbenchFlowComposition({
    ...services,
    workspaceState,
    interactionControllers: { assistantAudit: {}, topLevelActions: {}, uiActionController: {} },
    projectFlows: { adminDataEffects: {}, projectStorageController: {} },
    shellState: { languagePacks: [] }, studyResultDerived: {}, workflowController: {},
  });
  for (const [name, value] of Object.entries({ ...services, ...workspaceState })) {
    assert.equal(composition[name as keyof typeof composition], value, `${name} must reach the action controller`);
  }
});
