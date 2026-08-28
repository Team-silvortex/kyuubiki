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
