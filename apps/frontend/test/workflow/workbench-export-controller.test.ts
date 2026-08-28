import test from "node:test";
import assert from "node:assert/strict";

import {
  downloadWorkbenchProjectBundleJson,
  downloadWorkbenchProjectBundleZip,
} from "../../src/components/workbench/workbench-export-controller.ts";

const labels = {
  initialFailed: "fallback failure",
  projectExported: "exported",
  projectExportedPartial: "partial",
};

test("project download helpers preserve build failures as explicit outcomes", async () => {
  for (const download of [downloadWorkbenchProjectBundleJson, downloadWorkbenchProjectBundleZip]) {
    const failure = new Error("bundle build failed");
    const messages: string[] = [];
    const result = await download({
      buildBundle: async () => { throw failure; },
      labels,
      selectedProject: null,
      setMessage: (message) => messages.push(message),
    });

    assert.equal(result.ok, false);
    if (result.ok) assert.fail("download failure must not report success");
    assert.equal(result.error, failure);
    assert.deepEqual(messages, [failure.message]);
  }
});
