import test from "node:test";
import assert from "node:assert/strict";

import { validateConditionNodes } from "../../src/components/workbench/workflow/workbench-workflow-validation-condition.ts";

test("validateConditionNodes reports missing comparison value and branch ports", () => {
  const graph = {
    nodes: [
      {
        id: "condition_1",
        kind: "condition",
        config: { predicate: { path: "summary.max", operator: "gt" } },
        inputs: [],
        outputs: [{ id: "if_true", artifact_type: "artifact/json" }],
      },
    ],
  };
  const issues = validateConditionNodes(graph as never);

  assert.equal(issues.length, 2);
  assert.ok(issues.some((issue) => issue.id === "condition:inputs:condition_1"));
  assert.ok(issues.some((issue) => issue.id === "condition:outputs:condition_1"));
});
