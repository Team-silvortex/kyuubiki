import test from "node:test";
import assert from "node:assert/strict";

import {
  resolveWorkflowTraceContractWarningTone,
  resolveWorkflowTraceLineageSourceLabel,
  resolveWorkflowTraceLineageSourceTone,
  resolveWorkflowTraceProgressStageTone,
} from "../../src/components/workbench/workflow/workbench-workflow-trace-status.ts";

test("workflow progress stages preserve active and terminal status tones", () => {
  for (const stage of ["queued", "preprocessing", "partitioning", "solving", "postprocessing"]) {
    assert.equal(resolveWorkflowTraceProgressStageTone(stage), "watch");
  }
  assert.equal(resolveWorkflowTraceProgressStageTone("completed"), "good");
  assert.equal(resolveWorkflowTraceProgressStageTone("failed"), "risk");
  assert.equal(resolveWorkflowTraceProgressStageTone("cancelled"), "risk");
  assert.equal(resolveWorkflowTraceProgressStageTone("invented-stage"), "risk");
});

test("workflow warning counts reject malformed values", () => {
  assert.equal(resolveWorkflowTraceContractWarningTone(0), "good");
  assert.equal(resolveWorkflowTraceContractWarningTone(3), "watch");
  assert.equal(resolveWorkflowTraceContractWarningTone(4), "risk");
  assert.equal(resolveWorkflowTraceContractWarningTone(-1), "risk");
  assert.equal(resolveWorkflowTraceContractWarningTone(1.5), "risk");
  assert.equal(resolveWorkflowTraceContractWarningTone(Number.NaN), "risk");
});

test("workflow lineage ignores blank source artifact identifiers", () => {
  assert.equal(resolveWorkflowTraceLineageSourceTone(undefined), "watch");
  assert.equal(resolveWorkflowTraceLineageSourceTone(["", "  "]), "watch");
  assert.equal(resolveWorkflowTraceLineageSourceLabel(["", "  "]), "root");
  assert.equal(
    resolveWorkflowTraceLineageSourceTone([null, 42] as unknown as string[]),
    "watch",
  );
  assert.equal(resolveWorkflowTraceLineageSourceTone(["mesh.output"]), "good");
  assert.equal(resolveWorkflowTraceLineageSourceLabel(["mesh.output"]), "derived");
});
