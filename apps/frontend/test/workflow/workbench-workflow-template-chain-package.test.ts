import test from "node:test";
import assert from "node:assert/strict";

import {
  asWorkflowTemplateChainPackage,
  buildWorkflowTemplateChainPackage,
} from "../../src/components/workbench/workflow/workbench-workflow-template-chain-package.ts";
import type { WorkflowTemplateChainDefinition } from "../../src/components/workbench/workflow/workbench-workflow-template-chain-library.ts";

function packagePayload() {
  return {
    format: "kyuubiki.workflow-template-chain-package",
    version: 1,
    package_id: "branched-package",
    name: "Branched package",
    exported_at: "2026-08-28T00:00:00.000Z",
    templates: [
      { kind: "input", config: { source: "model" } },
      { kind: "solve", operatorId: "solve.heat_bar_1d" },
      { kind: "solve", operatorId: "solve.bar_1d" },
      { kind: "export", operatorId: "export.summary_json" },
    ],
    connections: [
      { from: 0, to: 1 },
      { from: 0, to: 2 },
      { from: 1, to: 3 },
      { from: 2, to: 3 },
    ],
  };
}

test("template chain package parser preserves valid branched topology", () => {
  const parsed = asWorkflowTemplateChainPackage(packagePayload());
  assert.ok(parsed);
  assert.deepEqual(parsed.connections, packagePayload().connections);
});

test("template chain package parser rejects unsafe topology", () => {
  const cases = [
    [{ from: 0, to: 4 }],
    [{ from: 1, to: 1 }],
    [{ from: 0, to: 1 }, { from: 1, to: 0 }],
    [{ from: 0.5, to: 1 }],
  ];
  for (const connections of cases) {
    assert.equal(asWorkflowTemplateChainPackage({ ...packagePayload(), connections }), null);
  }
});

test("template chain packages own template configuration and topology state", () => {
  const fixture = packagePayload();
  const source: WorkflowTemplateChainDefinition = {
    id: "branched-package",
    label: "Branched package",
    version: "1.0.0",
    templates: structuredClone(fixture.templates),
    connections: structuredClone(fixture.connections),
    source: "imported",
  };
  const built = buildWorkflowTemplateChainPackage(source);
  source.templates[0]!.config = { source: "mutated" };
  source.connections![0]!.to = 3;
  assert.deepEqual(built.templates[0]?.config, { source: "model" });
  assert.deepEqual(built.connections?.[0], { from: 0, to: 1 });

  const raw = packagePayload();
  const parsed = asWorkflowTemplateChainPackage(raw);
  assert.ok(parsed);
  raw.templates[0]!.config = { source: "mutated-again" };
  raw.connections[0]!.to = 3;
  assert.deepEqual(parsed.templates[0]?.config, { source: "model" });
  assert.deepEqual(parsed.connections?.[0], { from: 0, to: 1 });
});
