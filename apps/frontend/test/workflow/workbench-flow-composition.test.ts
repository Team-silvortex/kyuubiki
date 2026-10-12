import test from "node:test";
import assert from "node:assert/strict";

import { buildWorkbenchFlowComposition } from "../../src/components/workbench/workbench-flow-composition.ts";
import { createWorkbenchAdminRecordMutations } from "../../src/components/workbench/workbench-admin-record-mutations.ts";
import { createWorkbenchFeedback } from "../../src/components/workbench/workbench-feedback.ts";

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
  const adminRecordMutations = createWorkbenchAdminRecordMutations(() => {});
  const feedback = createWorkbenchFeedback();
  const cancelJob = async () => ({ job: { job_id: "selected", status: "cancelled", progress: 0 } });
  const source = { isCurrent: () => true };
  const composition = buildWorkbenchFlowComposition({
    adminRecordMutations,
    cancelJob,
    adminJobRecordSource: source, adminJobDraftSource: source,
    adminResultRecordSource: source, adminResultDraftSource: source,
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
    workspaceState: { feedback },
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
  assert.equal(composition.adminRecordMutations, adminRecordMutations);
  assert.equal(composition.cancelJob, cancelJob);
  assert.equal(composition.feedback, feedback);
  assert.equal(composition.adminJobRecordSource, source);
  assert.equal(composition.adminJobDraftSource, source);
  assert.equal(composition.adminResultRecordSource, source);
  assert.equal(composition.adminResultDraftSource, source);
});

test("flow composition connects project, model, selection and recovery dependencies to PWDT", () => {
  const workspaceState = {
    projects: [{ project_id: "project-a" }],
    modelVersions: [{ version_id: "version-a", project_id: "project-a", model_id: "model-a" }],
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
