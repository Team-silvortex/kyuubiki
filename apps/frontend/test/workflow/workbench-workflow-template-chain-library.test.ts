import test from "node:test";
import assert from "node:assert/strict";

import {
  DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN,
  PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN,
} from "../../src/components/workbench/workflow/workbench-workflow-template-chain-diagnostics-preset.ts";
import {
  listBuiltInWorkflowTemplateChains,
  listStoredWorkflowTemplateChains,
  removeImportedWorkflowTemplateChain,
  saveImportedWorkflowTemplateChain,
  WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY,
} from "../../src/components/workbench/workflow/workbench-workflow-template-chain-library.ts";
import { createWorkflowTopologyActions } from "../../src/components/workbench/workflow/workbench-workflow-topology-actions.ts";
import { isWorkflowOperatorSupportedInRuntime } from "../../src/components/workbench/workflow/workbench-workflow-runtime-support.ts";
import { scoreWorkflowTemplateChainSearch } from "../../src/components/workbench/workflow/workbench-workflow-template-chain-search.ts";
import type { WorkflowGraphDefinition } from "../../src/lib/api/index.ts";

function createMemoryStorage(): Storage {
  const records = new Map<string, string>();
  return {
    get length() {
      return records.size;
    },
    clear: () => records.clear(),
    getItem: (key) => records.get(key) ?? null,
    key: (index) => [...records.keys()][index] ?? null,
    removeItem: (key) => records.delete(key),
    setItem: (key, value) => records.set(key, String(value)),
  };
}

const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: createMemoryStorage() } as unknown as Window,
});

test.after(() => {
  if (originalWindowDescriptor) Object.defineProperty(globalThis, "window", originalWindowDescriptor);
  else Reflect.deleteProperty(globalThis, "window");
});

function branchedChain(id: string, updatedAt?: string) {
  return {
    id,
    label: id,
    version: "1.0.0",
    updatedAt,
    templates: [
      { kind: "input" },
      { kind: "solve", operatorId: "solve.heat_bar_1d" },
      { kind: "solve", operatorId: "solve.bar_1d" },
      { kind: "export", operatorId: "export.summary_json" },
    ],
    connections: [
      { from: 0, to: 1, toPort: "model" },
      { from: 0, to: 2, toPort: "model" },
      { from: 1, to: 3, fromPort: "result", toPort: "thermal" },
      { from: 2, to: 3, fromPort: "result", toPort: "mechanical" },
    ],
  };
}

test("built-in template chains only use runtime-supported operators", () => {
  const unsupported = listBuiltInWorkflowTemplateChains()
    .flatMap((chain) =>
      chain.templates.map((template) => ({
        chainId: chain.id,
        operatorId: template.operatorId,
      })),
    )
    .filter((entry): entry is { chainId: string; operatorId: string } =>
      Boolean(entry.operatorId),
    )
    .filter((entry) => !isWorkflowOperatorSupportedInRuntime(entry.operatorId));

  assert.deepEqual(unsupported, []);
});

test("diagnostics bundle chain stays registered with expected operators and guard defaults", () => {
  const chain = DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN;
  assert.equal(chain.label, "diagnostics -> bundle -> guard -> report");
  assert.equal(chain.templates.length, 7);
  assert.deepEqual(
    chain.templates.map((template) => template.operatorId ?? template.kind),
    [
      "extract.electrostatic_result_diagnostics",
      "extract.thermal_result_diagnostics",
      "extract.thermo_result_diagnostics",
      "transform.compose_diagnostics_bundle",
      "transform.evaluate_diagnostics_bundle_guard",
      "transform.compose_diagnostics_report_payload",
      "export.diagnostics_bundle_markdown",
    ],
  );
  assert.deepEqual(chain.connections?.map((connection) => connection.toPort), [
    "electrostatic",
    "thermal",
    "thermo",
    "bundle",
    "bundle",
    "guard",
    "bundle",
  ]);

  const guardTemplate = chain.templates[4];
  assert.ok(guardTemplate?.config);
  assert.deepEqual(guardTemplate.config?.rules, [
    {
      source: "thermal",
      field: "thermal_temperature_max",
      threshold: 120,
      severity: "warn",
      label: "thermal temperature",
    },
    {
      source: "thermo",
      field: "thermo_peak_stress",
      comparison: "gt",
      threshold: 180,
      severity: "block",
      label: "stress ceiling",
    },
    {
      source: "electrostatic",
      field: "electrostatic_field_peak_magnitude",
      comparison: "gt",
      threshold: 9,
      severity: "warn",
      label: "field ceiling",
    },
  ]);
});

test("peak diagnostics chain stays registered with expected operators and guard defaults", () => {
  const chain = PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN;
  assert.equal(chain.label, "peak extract -> bundle -> report");
  assert.equal(chain.templates.length, 7);
  assert.deepEqual(
    chain.templates.map((template) => template.operatorId ?? template.kind),
    [
      "extract.electrostatic_peak_field",
      "extract.heat_peak_flux",
      "extract.thermo_peak_response",
      "transform.compose_diagnostics_bundle",
      "transform.evaluate_diagnostics_bundle_guard",
      "transform.compose_diagnostics_report_payload",
      "export.diagnostics_bundle_markdown",
    ],
  );
});

test("material decision chain exposes ranking and Pareto operators", () => {
  const chain = listBuiltInWorkflowTemplateChains().find(
    (entry) => entry.id === "material_candidate_rank_pareto_decision",
  );
  assert.ok(chain);
  assert.deepEqual(
    chain.templates.map((template) => template.operatorId ?? template.kind),
    [
      "input",
      "transform.rank_material_candidates",
      "transform.extract_material_pareto_frontier",
      "export.summary_json",
    ],
  );
  assert.deepEqual(chain.connections?.map((connection) => [connection.from, connection.to]), [
    [0, 1],
    [0, 2],
    [2, 3],
  ]);
  assert.ok(scoreWorkflowTemplateChainSearch(chain, "material pareto"));
});

test("imported template chains preserve branched topology across storage reload", () => {
  const chain = branchedChain("imported-branched-topology");
  try {
    const saved = saveImportedWorkflowTemplateChain(chain);
    assert.ok(saved);
    assert.deepEqual(
      listStoredWorkflowTemplateChains().find((entry) => entry.id === chain.id)?.connections,
      chain.connections,
    );
  } finally {
    removeImportedWorkflowTemplateChain(chain.id);
  }
});

test("reloaded branched template chains insert their stored topology into the draft graph", () => {
  const chain = branchedChain("imported-branched-insertion");
  try {
    assert.ok(saveImportedWorkflowTemplateChain(chain));
    const reloaded = listStoredWorkflowTemplateChains().find((entry) => entry.id === chain.id);
    assert.ok(reloaded);

    let draftGraph: WorkflowGraphDefinition | null = {
      schema_version: "kyuubiki.workflow-graph/v1",
      id: "template-chain-insertion",
      nodes: [],
      edges: [],
    };
    const setDraftGraph = (
      action:
        | WorkflowGraphDefinition
        | null
        | ((current: WorkflowGraphDefinition | null) => WorkflowGraphDefinition | null),
    ) => {
      draftGraph = typeof action === "function" ? action(draftGraph) : action;
    };

    createWorkflowTopologyActions(setDraftGraph).insertTemplateChain(reloaded);

    assert.ok(draftGraph);
    assert.deepEqual(
      draftGraph.edges?.map((edge) => [
        edge.from.node,
        edge.from.port,
        edge.to.node,
        edge.to.port,
      ]),
      [
        ["node_1", "value", "node_2", "model"],
        ["node_1", "value", "node_3", "model"],
        ["node_2", "result", "node_4", "thermal"],
        ["node_3", "result", "node_4", "mechanical"],
      ],
    );
  } finally {
    removeImportedWorkflowTemplateChain(chain.id);
  }
});

test("template chain recovery keeps the newest unique valid records within the storage limit", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const storage = createMemoryStorage();
  const records = Array.from({ length: 45 }, (_, index) => ({
    ...branchedChain(
      `imported-recovery-${index}`,
      new Date(Date.UTC(2026, 0, 1, 0, index)).toISOString(),
    ),
    source: "imported",
  }));
  storage.setItem(
    WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY,
    JSON.stringify([
      ...records,
      { ...records[44], label: "Duplicate identity" },
      {
        ...branchedChain("imported-cycle", "2026-01-02T00:00:00.000Z"),
        source: "imported",
        connections: [{ from: 0, to: 1 }, { from: 1, to: 0 }],
      },
    ]),
  );
  browserWindow.localStorage = storage;

  try {
    const recovered = listStoredWorkflowTemplateChains();
    assert.equal(recovered.length, 40);
    assert.equal(new Set(recovered.map((entry) => entry.id)).size, 40);
    assert.equal(recovered[0]?.id, "imported-recovery-44");
    assert.equal(recovered.at(-1)?.id, "imported-recovery-5");
    assert.equal(recovered.some((entry) => entry.id === "imported-cycle"), false);
    const persisted = JSON.parse(
      storage.getItem(WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY) ?? "[]",
    ) as unknown[];
    assert.equal(persisted.length, 40);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("template chain mutations report write failures without throwing", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const chain = {
    ...branchedChain("imported-read-only", "2026-08-28T00:00:00.000Z"),
    source: "imported",
  };
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: (key) => key === WORKFLOW_TEMPLATE_CHAIN_LIBRARY_KEY ? JSON.stringify([chain]) : null,
    setItem: () => {
      throw new Error("quota exceeded");
    },
  } as Storage;

  try {
    assert.equal(saveImportedWorkflowTemplateChain(branchedChain("unsaved-chain")), null);
    assert.equal(removeImportedWorkflowTemplateChain(chain.id), false);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("template chain mutations do not overwrite an unreadable library", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  let writeCount = 0;
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: () => {
      throw new Error("storage unavailable");
    },
    setItem: () => {
      writeCount += 1;
    },
  } as Storage;

  try {
    assert.equal(saveImportedWorkflowTemplateChain(branchedChain("unreadable-save")), null);
    assert.equal(removeImportedWorkflowTemplateChain("unreadable-delete"), false);
    assert.equal(writeCount, 0);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});
