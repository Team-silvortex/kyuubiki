import test from "node:test";
import assert from "node:assert/strict";

import { validateRuntimeSupport } from "../../src/components/workbench/workflow/workbench-workflow-validation-runtime.ts";

test("validateRuntimeSupport flags unsupported operator ids", () => {
  const graph = {
    nodes: [
      {
        id: "transform_1",
        kind: "transform",
        operator_id: "transform.not_supported_yet",
      },
    ],
  };
  const issues = validateRuntimeSupport(graph as never);

  assert.equal(issues.length, 1);
  assert.equal(issues[0]?.id, "runtime:unsupported:transform_1");
});
